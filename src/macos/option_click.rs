//! Active `CGEventTap` that strips `Option` from cross-app `LeftMouseDown`
//! clicks so macOS focuses the clicked app without hiding the current one.
//!
//! Only listens to `LeftMouseDown` and early-returns immediately whenever
//! `MaskAlternate` is not set. Makes zero Accessibility calls: hit-testing uses
//! Core Graphics's on-screen window list and AppKit display/frontmost metadata.

use std::{cell::RefCell, ffi::c_void, ptr::NonNull};

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_core_foundation::{
    CFDictionary, CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopCommonModes,
};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType, CGWindowListCopyWindowInfo, CGWindowListOption,
};

use crate::{
    Rect,
    option_click::{DOCK_LAYER, HitWindow, OptionClickContext, should_strip_option},
};

use super::ax::{
    cf_array_items, window_server_alpha, window_server_bounds, window_server_entry,
    window_server_owner_name,
};

/// Active event tap guarding cross-app Option-clicks while enabled and tiling.
/// Dropping the guard removes the run-loop source and invalidates the Mach port.
pub struct OptionClickGuard {
    tap: CFRetained<CFMachPort>,
    source: CFRetained<CFRunLoopSource>,
    _context: Box<Context>,
}

struct Context {
    tap: RefCell<Option<CFRetained<CFMachPort>>>,
    self_pid: i32,
}

impl OptionClickGuard {
    /// Installs on the main run loop. Fails without Accessibility permission.
    pub fn install() -> Result<Self, String> {
        let self_pid = i32::try_from(std::process::id()).unwrap_or(0);
        let context = Box::new(Context {
            tap: RefCell::default(),
            self_pid,
        });
        // SAFETY: `tap_callback` matches `CGEventTapCallBack`, and `context` stays
        // heap-pinned in the guard until `Drop` removes the source and invalidates the tap.
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::SessionEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                1u64 << CGEventType::LeftMouseDown.0,
                Some(tap_callback),
                (&raw const *context).cast_mut().cast(),
            )
        }
        .ok_or("Option-click guard needs Accessibility")?;
        let source = CFMachPort::new_run_loop_source(None, Some(&tap), 0)
            .ok_or("could not create Option-click run loop source")?;
        let main = CFRunLoop::main().ok_or("the main run loop is unavailable")?;
        // SAFETY: `kCFRunLoopCommonModes` is a static Core Foundation mode constant.
        main.add_source(Some(&source), unsafe { kCFRunLoopCommonModes });
        context.tap.replace(Some(tap.clone()));
        CGEvent::tap_enable(&tap, true);
        Ok(Self {
            tap,
            source,
            _context: context,
        })
    }
}

impl Drop for OptionClickGuard {
    fn drop(&mut self) {
        if let Some(main) = CFRunLoop::main() {
            // SAFETY: `kCFRunLoopCommonModes` is a static Core Foundation mode constant.
            main.remove_source(Some(&self.source), unsafe { kCFRunLoopCommonModes });
        }
        self.tap.invalidate();
    }
}

/// Clears only `MaskAlternate`, preserving Command, Shift, Control, and all other flags.
#[must_use]
const fn strip_option_flag(flags: CGEventFlags) -> CGEventFlags {
    CGEventFlags(flags.0 & !CGEventFlags::MaskAlternate.0)
}

fn frontmost_pid() -> Option<i32> {
    let application = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    if super::is_lock_screen(&application) {
        return None;
    }
    let pid = application.processIdentifier();
    (pid > 0).then_some(pid)
}

struct OwnedHitWindow {
    owner_pid: i32,
    layer: i64,
    alpha: f64,
    frame: Rect,
    owner_name: Option<String>,
    bundle_id: Option<String>,
}

fn windows_containing(point: (i32, i32), self_pid: i32) -> Option<Vec<OwnedHitWindow>> {
    let list = CGWindowListCopyWindowInfo(
        CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements,
        0,
    )?;
    let mut windows = Vec::new();
    for entry in cf_array_items::<CFDictionary>(&list) {
        let Some(frame) = window_server_bounds(entry) else {
            continue;
        };
        if !frame.contains(point.0, point.1) {
            continue;
        }
        let alpha = window_server_alpha(entry);
        if alpha.is_nan() || alpha <= 0.0 {
            continue;
        }
        let Some((owner_pid, _, layer)) = window_server_entry(entry) else {
            continue;
        };
        let owner_name = window_server_owner_name(entry);
        let bundle_id =
            if owner_pid > 0 && owner_pid != self_pid && (0..DOCK_LAYER).contains(&layer) {
                NSRunningApplication::runningApplicationWithProcessIdentifier(owner_pid)
                    .and_then(|app| app.bundleIdentifier().map(|id| id.to_string()))
            } else {
                None
            };
        windows.push(OwnedHitWindow {
            owner_pid,
            layer,
            alpha,
            frame,
            owner_name,
            bundle_id,
        });
    }
    Some(windows)
}

unsafe extern "C-unwind" fn tap_callback(
    _: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    // SAFETY: `user_info` points to the `Context` boxed in `OptionClickGuard::install`,
    // which outlives the tap, and the tap calls back only on the main thread.
    let context = unsafe { &*user_info.cast::<Context>() };
    if matches!(
        event_type,
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
    ) {
        if let Some(tap) = context.tap.borrow().as_ref() {
            CGEvent::tap_enable(tap, true);
        }
        return event.as_ptr();
    }
    if event_type != CGEventType::LeftMouseDown {
        return event.as_ptr();
    }
    // SAFETY: the tap passes a live event for the duration of the callback.
    let cg_event = unsafe { event.as_ref() };
    let flags = CGEvent::flags(Some(cg_event));
    if !flags.contains(CGEventFlags::MaskAlternate) {
        return event.as_ptr();
    }

    autoreleasepool(|_| {
        let raw_point = CGEvent::location(Some(cg_event));
        let point = (raw_point.x.round() as i32, raw_point.y.round() as i32);
        let menu_bar_strips = super::displays::outputs()
            .map(|outputs| outputs.menu_bar_strips())
            .unwrap_or_default();
        if menu_bar_strips
            .iter()
            .any(|strip| strip.contains(point.0, point.1))
        {
            return;
        }
        let frontmost = frontmost_pid();
        let Some(owned) = windows_containing(point, context.self_pid) else {
            return;
        };
        let borrowed: Vec<HitWindow<'_>> = owned
            .iter()
            .map(|window| HitWindow {
                owner_pid: window.owner_pid,
                layer: window.layer,
                alpha: window.alpha,
                frame: window.frame,
                owner_name: window.owner_name.as_deref(),
                bundle_id: window.bundle_id.as_deref(),
            })
            .collect();
        let decision = should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point,
            frontmost_pid: frontmost,
            self_pid: context.self_pid,
            menu_bar_strips: &menu_bar_strips,
            windows: &borrowed,
        });
        if decision {
            CGEvent::set_flags(Some(cg_event), strip_option_flag(flags));
        }
    });

    event.as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_option_flag_clears_only_alternate_and_preserves_command_shift_control() {
        let option_only = CGEventFlags::MaskAlternate;
        assert!(!strip_option_flag(option_only).contains(CGEventFlags::MaskAlternate));

        let option_cmd = CGEventFlags::MaskAlternate | CGEventFlags::MaskCommand;
        let stripped = strip_option_flag(option_cmd);
        assert!(!stripped.contains(CGEventFlags::MaskAlternate));
        assert!(stripped.contains(CGEventFlags::MaskCommand));

        let all = CGEventFlags::MaskAlternate
            | CGEventFlags::MaskCommand
            | CGEventFlags::MaskShift
            | CGEventFlags::MaskControl;
        let stripped_all = strip_option_flag(all);
        assert!(!stripped_all.contains(CGEventFlags::MaskAlternate));
        assert!(stripped_all.contains(CGEventFlags::MaskCommand));
        assert!(stripped_all.contains(CGEventFlags::MaskShift));
        assert!(stripped_all.contains(CGEventFlags::MaskControl));
    }
}
