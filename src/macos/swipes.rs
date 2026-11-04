use std::{cell::RefCell, ffi::c_void, ptr::NonNull};

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSEvent, NSEventType, NSTouchPhase};
use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopCommonModes,
};
use objc2_core_graphics::{
    CGEvent, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy,
    CGEventType,
};

use crate::swipe::{Swipe, SwipeTracker};

/// Listens for three-finger trackpad swipes and reports each with the pointer's
/// position. The tap only listens, so touches still reach every app, and it needs
/// only Accessibility.
pub struct SwipeMonitor {
    tap: CFRetained<CFMachPort>,
    source: CFRetained<CFRunLoopSource>,
    _context: Box<Context>,
}

struct Context {
    report: Box<dyn Fn(Swipe, (i32, i32))>,
    tracker: RefCell<SwipeTracker>,
    tap: RefCell<Option<CFRetained<CFMachPort>>>,
}

impl SwipeMonitor {
    /// Installs on the main run loop. Fails without Accessibility.
    pub fn install(report: impl Fn(Swipe, (i32, i32)) + 'static) -> Result<Self, String> {
        let context = Box::new(Context {
            report: Box::new(report),
            tracker: RefCell::default(),
            tap: RefCell::default(),
        });
        let gesture = NSEventType::Gesture.0 as u32;
        // SAFETY: `tap_callback` matches `CGEventTapCallBack`, and `context` stays
        // heap-pinned in the monitor until `Drop` invalidates the tap.
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::SessionEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                1 << gesture,
                Some(tap_callback),
                (&raw const *context).cast_mut().cast(),
            )
        }
        .ok_or("trackpad swipes need Accessibility")?;
        let source = CFMachPort::new_run_loop_source(None, Some(&tap), 0)
            .ok_or("could not listen for trackpad swipes")?;
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

impl Drop for SwipeMonitor {
    fn drop(&mut self) {
        if let Some(main) = CFRunLoop::main() {
            // SAFETY: `kCFRunLoopCommonModes` is a static Core Foundation mode constant.
            main.remove_source(Some(&self.source), unsafe { kCFRunLoopCommonModes });
        }
        self.tap.invalidate();
    }
}

unsafe extern "C-unwind" fn tap_callback(
    _: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    // SAFETY: `user_info` points to the `Context` boxed in `SwipeMonitor::install`, which
    // outlives the tap, and the tap calls back only on the main thread.
    let context = unsafe { &*user_info.cast::<Context>() };
    // SAFETY: the tap passes a live event for the duration of the callback.
    let cg_event = unsafe { event.as_ref() };
    if matches!(
        event_type,
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
    ) {
        if let Some(tap) = context.tap.borrow().as_ref() {
            CGEvent::tap_enable(tap, true);
        }
        return event.as_ptr();
    }
    autoreleasepool(|_| {
        let Some(ns_event) = NSEvent::eventWithCGEvent(cg_event) else {
            return;
        };
        let touches: Vec<_> = ns_event
            .allTouches()
            .iter()
            .filter(|touch| touch.phase().intersects(NSTouchPhase::Touching))
            .map(|touch| {
                let point = touch.normalizedPosition();
                (point.x, point.y)
            })
            .collect();
        let swipe = context
            .tracker
            .borrow_mut()
            .frame(ns_event.timestamp(), &touches);
        if let Some(swipe) = swipe {
            let point = CGEvent::location(Some(cg_event));
            (context.report)(swipe, (point.x.round() as i32, point.y.round() as i32));
        }
    });
    event.as_ptr()
}
