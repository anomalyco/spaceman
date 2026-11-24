use std::{collections::HashMap, ffi::c_void, ptr, ptr::NonNull, sync::OnceLock};

use objc2::rc::autoreleasepool;
use objc2_app_kit::NSRunningApplication;
use objc2_application_services::{AXError, AXUIElement, AXValue, AXValueType};
use objc2_core_foundation::{
    CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType, CGPoint, CGRect,
    CGSize, ConcreteType,
};
use objc2_core_graphics::{
    CGRectMakeWithDictionaryRepresentation, CGWindowID, CGWindowListCopyWindowInfo,
    CGWindowListOption, kCGWindowBounds, kCGWindowLayer, kCGWindowNumber, kCGWindowOwnerPID,
};

use crate::{AppInstanceId, PhysicalWindowId, Rect};

use super::launch_time;

type WindowIdFunction = unsafe extern "C" fn(&AXUIElement, *mut CGWindowID) -> AXError;

const AX_MESSAGING_TIMEOUT: f32 = 0.5;

pub(super) struct WindowServerWindow {
    pub(super) pid: i32,
    pub(super) id: u32,
    pub(super) frame: Rect,
}

pub(super) fn set_position(window: &AXUIElement, frame: Rect) -> AXError {
    let Some(value) = ax_value(
        AXValueType::CGPoint,
        CGPoint {
            x: f64::from(frame.x),
            y: f64::from(frame.y),
        },
    ) else {
        return AXError::Failure;
    };
    set_attribute(window, "AXPosition", &value)
}

pub(super) fn set_size(window: &AXUIElement, frame: Rect) -> AXError {
    let Some(value) = ax_value(
        AXValueType::CGSize,
        CGSize {
            width: f64::from(frame.width),
            height: f64::from(frame.height),
        },
    ) else {
        return AXError::Failure;
    };
    set_attribute(window, "AXSize", &value)
}

pub(super) fn window_frame(window: &AXUIElement) -> Option<Rect> {
    let position = ax_point(window, "AXPosition")?;
    let size = ax_size(window, "AXSize")?;
    Some(Rect {
        x: position.x.round() as i32,
        y: position.y.round() as i32,
        width: size.width.round() as i32,
        height: size.height.round() as i32,
    })
}

pub(super) fn onscreen_windows() -> Option<HashMap<i32, HashMap<CGWindowID, i64>>> {
    let list = CGWindowListCopyWindowInfo(
        CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements,
        0,
    )?;
    let mut windows = HashMap::<i32, HashMap<CGWindowID, i64>>::new();
    for entry in cf_array_items::<CFDictionary>(&list) {
        if let Some((pid, id, layer)) = window_server_identity(entry) {
            windows.entry(pid).or_default().insert(id, layer);
        }
    }
    Some(windows)
}

/// The window server's authoritative frame for a window. Accessibility frame
/// reads can lag behind reality for apps with asynchronous accessibility
/// servers (Electron); Core Graphics reflects the real position immediately.
/// The full window list is queried because the single-window variant
/// (`kCGWindowListOptionIncludingWindow`) silently returns nothing for some
/// applications' windows.
pub fn window_server_frame(id: &PhysicalWindowId) -> Option<Rect> {
    window_server_windows(CGWindowListOption::OptionAll)?
        .into_iter()
        .find(|window| window.id == id.window_id && window.pid == id.app.pid)
        .map(|window| window.frame)
}

/// How far outside its frame the pointer can still grab a window's edge to
/// resize it. macOS puts much of a window's resize border outside the frame,
/// so an edge drag often starts in a gap or over the neighboring window.
const RESIZE_BORDER_REACH: i32 = 10;

/// Every window the pointer may be acting on, front to back with its current
/// frame: the window under the point and any whose resize border reaches it.
/// Hit-tests window-server geometry, without synchronous AX calls or native
/// handles crossing the main-thread/worker boundary.
pub fn windows_at_point(point: (i32, i32)) -> Vec<(PhysicalWindowId, Rect)> {
    autoreleasepool(|_| {
        let Some(windows) = window_server_windows(CGWindowListOption::OptionOnScreenOnly) else {
            return Vec::new();
        };
        windows_reaching_point(&windows, point)
            .filter_map(|window| {
                let application =
                    NSRunningApplication::runningApplicationWithProcessIdentifier(window.pid)?;
                let launched_at = launch_time(&application, window.pid);
                (launched_at != 0).then_some((
                    PhysicalWindowId {
                        app: AppInstanceId {
                            pid: window.pid,
                            launched_at,
                        },
                        window_id: window.id,
                    },
                    window.frame,
                ))
            })
            .collect()
    })
}

pub(super) fn windows_reaching_point(
    windows: &[WindowServerWindow],
    point: (i32, i32),
) -> impl Iterator<Item = &WindowServerWindow> {
    windows.iter().filter(move |window| {
        let reach = Rect {
            x: window.frame.x - RESIZE_BORDER_REACH,
            y: window.frame.y - RESIZE_BORDER_REACH,
            width: window.frame.width + 2 * RESIZE_BORDER_REACH,
            height: window.frame.height + 2 * RESIZE_BORDER_REACH,
        };
        reach.contains(point.0, point.1)
    })
}

/// Window-server frames for every ordinary window in one list copy. Use this
/// instead of [`window_server_frame`] when checking more than one window.
pub fn window_server_frames() -> Option<HashMap<CGWindowID, Rect>> {
    window_server_frame_map(CGWindowListOption::OptionAll)
}

/// On-screen window-server IDs, front-most first.
pub fn onscreen_window_order() -> Option<Vec<CGWindowID>> {
    Some(
        window_server_windows(CGWindowListOption::OptionOnScreenOnly)?
            .into_iter()
            .map(|window| window.id)
            .collect(),
    )
}

/// Window-server frames for every ordinary window currently on screen. A
/// window listed here exists and is visible even when an app's Accessibility
/// snapshot momentarily omits it.
pub fn onscreen_window_frames() -> Option<HashMap<CGWindowID, Rect>> {
    window_server_frame_map(CGWindowListOption::OptionOnScreenOnly)
}

pub(super) fn window_server_windows(
    options: CGWindowListOption,
) -> Option<Vec<WindowServerWindow>> {
    let mut windows = Vec::new();
    let list = CGWindowListCopyWindowInfo(options | CGWindowListOption::ExcludeDesktopElements, 0)?;
    for entry in cf_array_items::<CFDictionary>(&list) {
        let Some((pid, id, _)) = window_server_identity(entry) else {
            continue;
        };
        let Some(frame) = window_server_bounds(entry) else {
            continue;
        };
        windows.push(WindowServerWindow { pid, id, frame });
    }
    Some(windows)
}

fn window_server_frame_map(options: CGWindowListOption) -> Option<HashMap<CGWindowID, Rect>> {
    Some(
        window_server_windows(options)?
            .into_iter()
            .map(|window| (window.id, window.frame))
            .collect(),
    )
}

fn window_server_identity(entry: &CFDictionary) -> Option<(i32, u32, i64)> {
    // SAFETY: CoreGraphics window-info keys are process-lifetime static `CFString`s.
    let layer = dictionary_number(entry, unsafe { kCGWindowLayer })?;
    if !matches!(layer, 0 | 3) {
        return None;
    }
    // SAFETY: CoreGraphics window-info keys are process-lifetime static `CFString`s.
    let id = dictionary_number(entry, unsafe { kCGWindowNumber })
        .and_then(|id| u32::try_from(id).ok())?;
    // SAFETY: CoreGraphics window-info keys are process-lifetime static `CFString`s.
    let pid = dictionary_number(entry, unsafe { kCGWindowOwnerPID })
        .and_then(|pid| i32::try_from(pid).ok())?;
    Some((pid, id, layer))
}

fn window_server_bounds(entry: &CFDictionary) -> Option<Rect> {
    // SAFETY: `kCGWindowBounds` is a process-lifetime static `CFString`.
    let bounds = dictionary_value::<CFDictionary>(entry, unsafe { kCGWindowBounds })?;
    let mut frame = CGRect::ZERO;
    // SAFETY: `bounds` is a valid CoreGraphics rect dictionary and `frame` is writable stack storage.
    unsafe { CGRectMakeWithDictionaryRepresentation(Some(bounds), ptr::from_mut(&mut frame)) }
        .then_some(Rect {
            x: frame.origin.x.round() as i32,
            y: frame.origin.y.round() as i32,
            width: frame.size.width.round() as i32,
            height: frame.size.height.round() as i32,
        })
}

fn dictionary_value<'a, T: ConcreteType>(
    dictionary: &'a CFDictionary,
    key: &CFString,
) -> Option<&'a T> {
    // SAFETY: `dictionary` retains its Core Foundation values for the lifetime `'a`, and `key`
    // is a valid `CFStringRef`.
    let raw = unsafe { dictionary.value((key as *const CFString).cast::<c_void>()) };
    // SAFETY: `raw` is either null or a Core Foundation object owned by `dictionary`.
    unsafe { raw.cast::<CFType>().as_ref() }?.downcast_ref::<T>()
}

fn dictionary_number(dictionary: &CFDictionary, key: &CFString) -> Option<i64> {
    dictionary_value::<CFNumber>(dictionary, key)?.as_i64()
}

pub(super) fn physical_window_id(
    app: &AppInstanceId,
    element: &AXUIElement,
) -> Option<PhysicalWindowId> {
    static FUNCTION: OnceLock<Option<WindowIdFunction>> = OnceLock::new();
    let function = FUNCTION
        .get_or_init(|| {
            // SAFETY: Looks up the private HIServices symbol in the current process image.
            let symbol =
                unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"_AXUIElementGetWindow".as_ptr()) };
            // SAFETY: `_AXUIElementGetWindow` has C signature `(AXUIElementRef, CGWindowID*) -> AXError`.
            (!symbol.is_null()).then(|| unsafe { std::mem::transmute(symbol) })
        })
        .as_ref()?;
    let mut window_id = 0;
    // SAFETY: `element` is a live `AXUIElement` and `window_id` is a valid out-pointer.
    let error = unsafe { function(element, &mut window_id) };
    (error == AXError::Success && window_id != 0).then(|| PhysicalWindowId {
        app: app.clone(),
        window_id,
    })
}

pub(super) fn should_ignore_missing_window_id(role: Option<&str>, subrole: Option<&str>) -> bool {
    match role {
        Some("AXWindow") => subrole.is_some_and(|subrole| subrole != "AXStandardWindow"),
        Some(_) => true,
        None => false,
    }
}

/// Returns true when an Accessibility entry in `AXWindows` / `AXFocusedWindow` that has no
/// `CGWindowID` is demonstrably not a standard window (for example Finder's `AXScrollArea` /
/// `AXDesktop` entry), so callers can skip it instead of marking the entire app unavailable.
pub(super) fn is_non_standard_window_element(element: &AXUIElement) -> bool {
    let role = attribute::<CFString>(element, "AXRole")
        .ok()
        .map(|value| value.to_string());
    if role.as_deref().is_some_and(|value| value != "AXWindow") {
        return true;
    }
    let subrole = attribute::<CFString>(element, "AXSubrole")
        .ok()
        .map(|value| value.to_string());
    should_ignore_missing_window_id(role.as_deref(), subrole.as_deref())
}

pub(super) fn is_settable(element: &AXUIElement, name: &'static str) -> Result<bool, AXError> {
    let attribute = CFString::from_static_str(name);
    let mut settable = 0;
    // SAFETY: `attribute` is valid and `settable` is writable stack storage.
    let error = unsafe { element.is_attribute_settable(&attribute, NonNull::from(&mut settable)) };
    (error == AXError::Success)
        .then_some(settable != 0)
        .ok_or(error)
}

pub(super) fn optional_bool_attribute(
    element: &AXUIElement,
    name: &'static str,
) -> Result<bool, AXError> {
    match attribute::<CFBoolean>(element, name).map(|value| value.value()) {
        Ok(value) => Ok(value),
        Err(AXError::NoValue | AXError::AttributeUnsupported) => Ok(false),
        Err(error) => Err(error),
    }
}

pub(super) fn attribute<T: ConcreteType>(
    element: &AXUIElement,
    name: &'static str,
) -> Result<CFRetained<T>, AXError> {
    let name = CFString::from_static_str(name);
    let mut raw: *const CFType = ptr::null();
    // SAFETY: `copy_attribute_value` follows the Core Foundation Copy rule into `raw`.
    let error = unsafe { element.copy_attribute_value(&name, NonNull::from(&mut raw)) };
    if error != AXError::Success {
        return Err(error);
    }
    let Some(raw) = NonNull::new(raw.cast_mut()) else {
        return Err(AXError::NoValue);
    };
    // SAFETY: `raw` is a +1 retained `CFTypeRef` returned on `AXError::Success`.
    unsafe { CFRetained::from_raw(raw) }
        .downcast()
        .map_err(|_| AXError::IllegalArgument)
}

pub(super) fn cf_array_items<'a, T: ConcreteType + 'a>(
    array: &'a CFArray,
) -> impl Iterator<Item = &'a T> + 'a {
    (0..array.count()).filter_map(move |index| {
        // SAFETY: `index` is within `0..array.count()`.
        let raw = unsafe { array.value_at_index(index) };
        // SAFETY: `raw` is null or a Core Foundation object retained by `array` for `'a`.
        unsafe { raw.cast::<CFType>().as_ref() }?.downcast_ref::<T>()
    })
}

pub(super) fn application_element(pid: i32) -> CFRetained<AXUIElement> {
    // SAFETY: `AXUIElementCreateApplication` safely creates a wrapper for any PID.
    let application = unsafe { AXUIElement::new_application(pid) };
    set_ax_timeout(&application);
    application
}

pub(super) fn set_ax_timeout(element: &AXUIElement) {
    // SAFETY: Setting a messaging timeout on a live `AXUIElement` has no pointer preconditions.
    let _: AXError = unsafe { element.set_messaging_timeout(AX_MESSAGING_TIMEOUT) };
}

pub(super) fn set_attribute(element: &AXUIElement, name: &'static str, value: &CFType) -> AXError {
    // SAFETY: Both `CFString` and `value` are valid Core Foundation objects for the call.
    unsafe { element.set_attribute_value(&CFString::from_static_str(name), value) }
}

fn ax_value<T>(kind: AXValueType, mut value: T) -> Option<CFRetained<AXValue>> {
    // SAFETY: `value` matches the caller's `AXValueType` (`CGPoint` or `CGSize`) on the stack.
    unsafe { AXValue::new(kind, NonNull::from(&mut value).cast::<c_void>()) }
}

fn ax_value_get<T: Copy>(
    element: &AXUIElement,
    name: &'static str,
    kind: AXValueType,
    mut zero: T,
) -> Option<T> {
    let value = attribute::<AXValue>(element, name).ok()?;
    // SAFETY: `zero` matches the layout expected by `kind` (`CGPoint` or `CGSize`).
    unsafe {
        value
            .value(kind, NonNull::from(&mut zero).cast::<c_void>())
            .then_some(zero)
    }
}

fn ax_point(element: &AXUIElement, name: &'static str) -> Option<CGPoint> {
    ax_value_get(element, name, AXValueType::CGPoint, CGPoint::ZERO)
}

fn ax_size(element: &AXUIElement, name: &'static str) -> Option<CGSize> {
    ax_value_get(element, name, AXValueType::CGSize, CGSize::ZERO)
}
