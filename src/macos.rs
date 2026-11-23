mod apply;
mod ax;
mod events;
mod keys;
mod swipes;
#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    ptr::NonNull,
};

use objc2::{MainThreadMarker, rc::autoreleasepool};
use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSScreen, NSWorkspace};
use objc2_application_services::{AXError, AXIsProcessTrusted, AXUIElement};
use objc2_core_foundation::{CFArray, CFRetained, CFString, CFUUID};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventType, CGMouseButton,
    CGWindowID, CGWindowListOption,
};
use objc2_foundation::{NSNumber, ns_string};

use crate::{
    AppInstanceId, AppObservation, DisplaySelector, NativeObservation, ObservedApplication,
    ObservedWindow, PhysicalWindowId, Rect, Settings,
};

pub use apply::{
    ApplyOutcome, ApplyResult, WindowAccessError, apply, apply_managed, apply_while, focus_window,
    frame, raise_window, set_position_only,
};
pub use ax::{
    onscreen_window_frames, onscreen_window_order, window_at_point, window_server_frame,
    window_server_frames,
};
pub use events::WindowEventMonitor;
pub use keys::KeyMonitor;
pub use swipes::SwipeMonitor;

use apply::{cache_managed_window, evict_managed_app, prune_managed_app};
use ax::{
    application_element, attribute, button_is_enabled, cf_array_items, is_settable,
    onscreen_windows, optional_bool_attribute, physical_window_id, set_ax_timeout, window_frame,
    window_server_windows,
};

#[derive(Debug)]
pub enum SnapshotError {
    AccessibilityPermissionRequired,
    MainThreadRequired,
    MainScreenUnavailable,
    WindowListUnavailable,
    TargetWindowUnavailable,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessibilityPermissionRequired => {
                formatter.write_str("Accessibility permission is required for live snapshots")
            }
            Self::MainThreadRequired => formatter.write_str("snapshot must run on the main thread"),
            Self::MainScreenUnavailable => formatter.write_str("the main display is unavailable"),
            Self::WindowListUnavailable => {
                formatter.write_str("the on-screen window list is unavailable")
            }
            Self::TargetWindowUnavailable => {
                formatter.write_str("the target window is unavailable on attached displays")
            }
        }
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *const CFUUID;
}

pub fn snapshot(cycle: u64) -> Result<(NativeObservation, Settings), SnapshotError> {
    if !accessibility_trusted() {
        return Err(SnapshotError::AccessibilityPermissionRequired);
    }
    snapshot_in_output(cycle, main_output()?)
}

/// Pane's own windows (Settings, Onboarding, HUD) are never managed, even when the CLI
/// observes `Pane.app` from a second process.
pub(super) fn is_own_application(application: &NSRunningApplication) -> bool {
    application.bundleIdentifier().is_some_and(|id| {
        &*id == ns_string!("anomaly.pane") || &*id == ns_string!("com.kitlangton.spaceman")
    })
}

#[must_use]
pub fn accessibility_trusted() -> bool {
    // SAFETY: `AXIsProcessTrusted` queries TCC state without taking caller pointers.
    unsafe { AXIsProcessTrusted() }
}

fn main_output() -> Result<Rect, SnapshotError> {
    Ok(outputs()?.main().frame)
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct DisplayOutput {
    pub id: String,
    pub name: String,
    pub frame: Rect,
    pub is_main: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayOutputs {
    displays: Vec<DisplayOutput>,
    main_index: usize,
}

impl DisplayOutputs {
    pub fn new(displays: Vec<DisplayOutput>) -> Result<Self, SnapshotError> {
        let Some(main_index) = displays.iter().position(|display| display.is_main) else {
            return Err(SnapshotError::MainScreenUnavailable);
        };
        Ok(Self {
            displays,
            main_index,
        })
    }

    #[must_use]
    pub fn displays(&self) -> &[DisplayOutput] {
        &self.displays
    }

    #[must_use]
    pub fn main(&self) -> &DisplayOutput {
        &self.displays[self.main_index]
    }

    #[must_use]
    pub fn secondary(&self) -> Option<&DisplayOutput> {
        self.displays.iter().find(|display| !display.is_main)
    }

    #[must_use]
    pub fn resolve_exact(&self, selector: &DisplaySelector) -> Option<&DisplayOutput> {
        match selector {
            DisplaySelector::Main => Some(self.main()),
            DisplaySelector::Secondary => self.secondary(),
            DisplaySelector::Preferred(preferred) => self
                .displays
                .iter()
                .find(|display| display.id == *preferred || display.name == *preferred),
        }
    }

    #[must_use]
    pub fn resolve(&self, selector: &DisplaySelector) -> &DisplayOutput {
        self.resolve_exact(selector).unwrap_or_else(|| self.main())
    }

    pub fn frames(&self) -> impl Iterator<Item = Rect> + '_ {
        self.displays.iter().map(|display| display.frame)
    }

    #[must_use]
    pub fn union(&self) -> Rect {
        self.frames().fold(self.main().frame, union_rect)
    }
}

pub fn snapshot_in_outputs(
    cycle: u64,
    outputs: &DisplayOutputs,
) -> Result<NativeObservation, SnapshotError> {
    snapshot_in_output(cycle, outputs.union()).map(|(observation, _)| observation)
}

pub fn outputs() -> Result<DisplayOutputs, SnapshotError> {
    autoreleasepool(|_| {
        let marker = MainThreadMarker::new().ok_or(SnapshotError::MainThreadRequired)?;
        let screens = NSScreen::screens(marker);
        let primary_height = (0..screens.count())
            .map(|index| screens.objectAtIndex(index).frame())
            .find(|frame| frame.origin.x == 0.0 && frame.origin.y == 0.0)
            .map(|frame| frame.size.height)
            .ok_or(SnapshotError::MainScreenUnavailable)?;
        let mut displays = Vec::with_capacity(screens.count());
        for index in 0..screens.count() {
            let screen = screens.objectAtIndex(index);
            let frame = screen.frame();
            let display_id =
                screen_display_id(&screen).ok_or(SnapshotError::MainScreenUnavailable)?;
            displays.push(DisplayOutput {
                id: display_uuid(display_id).unwrap_or_else(|| display_id.to_string()),
                name: screen.localizedName().to_string(),
                frame: screen_output(&screen, primary_height),
                is_main: frame.origin.x == 0.0 && frame.origin.y == 0.0,
            });
        }
        DisplayOutputs::new(displays)
    })
}

/// Each connected display's Core Graphics ID, keyed by the ID `outputs` gives it. Main thread
/// only; empty elsewhere.
#[must_use]
pub fn display_ids() -> HashMap<String, u32> {
    autoreleasepool(|_| {
        let Some(marker) = MainThreadMarker::new() else {
            return HashMap::new();
        };
        let screens = NSScreen::screens(marker);
        (0..screens.count())
            .filter_map(|index| screen_display_id(&screens.objectAtIndex(index)))
            .map(|display_id| {
                let id = display_uuid(display_id).unwrap_or_else(|| display_id.to_string());
                (id, display_id)
            })
            .collect()
    })
}

fn screen_display_id(screen: &NSScreen) -> Option<u32> {
    let description = screen.deviceDescription();
    let number = description.objectForKey(ns_string!("NSScreenNumber"))?;
    number
        .downcast_ref::<NSNumber>()
        .map(NSNumber::unsignedIntValue)
}

fn display_uuid(display_id: u32) -> Option<String> {
    // SAFETY: `CGDisplayCreateUUIDFromDisplayID` returns null or a +1 retained `CFUUIDRef`.
    let raw = unsafe { CGDisplayCreateUUIDFromDisplayID(display_id) };
    // SAFETY: `raw` is non-null and +1 retained under the Core Foundation Create rule.
    let uuid = unsafe { CFRetained::from_raw(NonNull::new(raw.cast_mut())?) };
    CFUUID::new_string(None, Some(&uuid)).map(|uuid| uuid.to_string())
}

pub fn snapshot_for_window(
    cycle: u64,
    window_id: u32,
) -> Result<(NativeObservation, Settings), SnapshotError> {
    if !accessibility_trusted() {
        return Err(SnapshotError::AccessibilityPermissionRequired);
    }
    let outputs = outputs()?;
    let (observation, _) = snapshot_in_output(cycle, outputs.union())?;
    let frame = observation
        .applications
        .iter()
        .find_map(|app| match app {
            AppObservation::Complete { windows, .. } => windows
                .iter()
                .find(|window| window.id.window_id == window_id)
                .map(|window| window.frame),
            _ => None,
        })
        .ok_or(SnapshotError::TargetWindowUnavailable)?;
    let output = outputs
        .frames()
        .find(|output| output.contains_center(frame))
        .ok_or(SnapshotError::TargetWindowUnavailable)?;
    snapshot_in_output(cycle, output)
}

pub fn snapshot_in_output(
    cycle: u64,
    output: Rect,
) -> Result<(NativeObservation, Settings), SnapshotError> {
    autoreleasepool(|_| {
        if !accessibility_trusted() {
            return Err(SnapshotError::AccessibilityPermissionRequired);
        }
        let onscreen = onscreen_windows().ok_or(SnapshotError::WindowListUnavailable)?;

        let running = NSWorkspace::sharedWorkspace().runningApplications();
        let screen_locked = is_screen_locked();
        let mut observed_pids = HashSet::new();
        let mut applications: Vec<_> = (0..running.count())
            .filter_map(|index| {
                let application = running.objectAtIndex(index);
                let pid = application.processIdentifier();
                let windows = onscreen.get(&pid)?;
                observed_pids.insert(pid);
                observe_application(&application, pid, windows, output, screen_locked)
            })
            .collect();
        let mut extra_pids: Vec<_> = onscreen
            .keys()
            .copied()
            .filter(|pid| *pid > 0 && !observed_pids.contains(pid))
            .collect();
        extra_pids.sort_unstable();
        for pid in extra_pids {
            if let Some(application) =
                NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
                && let Some(windows) = onscreen.get(&pid)
                && let Some(observed) =
                    observe_application(&application, pid, windows, output, screen_locked)
            {
                applications.push(observed);
            }
        }

        Ok((
            NativeObservation {
                cycle,
                mouse_down: mouse_down(),
                applications,
            },
            Settings::for_output(output),
        ))
    })
}

fn screen_output(screen: &NSScreen, primary_height: f64) -> Rect {
    let visible = screen.visibleFrame();
    Rect {
        x: visible.origin.x.round() as i32,
        y: (primary_height - visible.origin.y - visible.size.height).round() as i32,
        width: visible.size.width.round() as i32,
        height: visible.size.height.round() as i32,
    }
}

fn union_rect(left: Rect, right: Rect) -> Rect {
    let x = left.x.min(right.x);
    let y = left.y.min(right.y);
    Rect {
        x,
        y,
        width: (left.x + left.width).max(right.x + right.width) - x,
        height: (left.y + left.height).max(right.y + right.height) - y,
    }
}

/// When launched as an app bundle, stderr goes nowhere. Redirect it to a
/// size-capped, owner-only log file so runtime diagnostics survive. Terminal
/// launches keep stderr on the terminal.
pub(crate) fn redirect_stderr_to_log() {
    use std::{
        io::IsTerminal,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };
    if std::io::stderr().is_terminal() {
        return;
    }
    let Some(path) = crate::config::application_support_path("agent.log") else {
        return;
    };
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
    if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 5 * 1024 * 1024) {
        // Keep one previous generation: the history before a restart is
        // usually what a bug report needs.
        let previous = path.with_extension("log.prev");
        drop(std::fs::rename(&path, &previous));
        drop(std::fs::set_permissions(
            &previous,
            std::fs::Permissions::from_mode(0o600),
        ));
    }
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&path)
    else {
        return;
    };
    drop(std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(0o600),
    ));
    redirect_stderr_to(&file);
}

fn redirect_stderr_to(file: &std::fs::File) {
    let fd = std::os::unix::io::AsRawFd::as_raw_fd(file);
    // SAFETY: dup2 atomically replaces descriptor 2 with a duplicate of a
    // descriptor we own; the original File may close independently afterwards.
    let _: i32 = unsafe { libc::dup2(fd, libc::STDERR_FILENO) };
}

#[must_use]
pub fn focused_window() -> Option<PhysicalWindowId> {
    let id = frontmost_application()?;
    let element = application_element(id.pid);
    let focused = attribute::<AXUIElement>(&element, "AXFocusedWindow").ok()?;
    physical_window_id(&id, &focused)
}

#[must_use]
pub fn frontmost_application() -> Option<AppInstanceId> {
    autoreleasepool(|_| {
        let application = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        if is_lock_screen(&application) {
            return None;
        }
        let pid = resolve_application_pid(&application)?;
        Some(app_instance_with_pid(&application, pid))
    })
}

const LOCK_SCREEN_BUNDLE_ID: &str = "com.apple.loginwindow";

fn is_lock_screen(application: &NSRunningApplication) -> bool {
    application
        .bundleIdentifier()
        .is_some_and(|bundle| &*bundle == ns_string!("com.apple.loginwindow"))
}

fn is_screen_locked() -> bool {
    autoreleasepool(|_| {
        NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .is_some_and(|app| is_lock_screen(&app))
    })
}

#[must_use]
pub fn mouse_down() -> bool {
    CGEventSource::button_state(CGEventSourceStateID::HIDSystemState, CGMouseButton::Left)
}

/// How long ago the user last typed, clicked, or changed modifiers, and
/// whether Command is held. Tells Command-Tab apart from an app activating
/// itself in focus-follow logs, without recording which keys were pressed.
#[derive(Clone, Copy, Debug)]
pub struct RecentInput {
    pub key_down: f64,
    pub mouse_down: f64,
    pub modifiers_changed: f64,
    pub command_held: bool,
}

impl std::fmt::Display for RecentInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "key {:.2}s ago, click {:.2}s ago, modifiers {:.2}s ago, command {}",
            self.key_down,
            self.mouse_down,
            self.modifiers_changed,
            if self.command_held { "held" } else { "up" }
        )
    }
}

#[must_use]
pub fn recent_input() -> RecentInput {
    let state = CGEventSourceStateID::HIDSystemState;
    let since = |event| CGEventSource::seconds_since_last_event_type(state, event);
    RecentInput {
        key_down: since(CGEventType::KeyDown),
        mouse_down: since(CGEventType::LeftMouseDown).min(since(CGEventType::RightMouseDown)),
        modifiers_changed: since(CGEventType::FlagsChanged),
        command_held: CGEventSource::flags_state(state).contains(CGEventFlags::MaskCommand),
    }
}

#[must_use]
pub fn mouse_position() -> Option<(i32, i32)> {
    let event = CGEvent::new(None)?;
    let point = CGEvent::location(Some(&event));
    Some((point.x.round() as i32, point.y.round() as i32))
}

pub(super) fn running_application(
    id: &AppInstanceId,
) -> Option<objc2::rc::Retained<NSRunningApplication>> {
    autoreleasepool(|_| {
        NSRunningApplication::runningApplicationWithProcessIdentifier(id.pid)
            .filter(|application| launch_time(application, id.pid) == id.launched_at)
    })
}

pub(super) fn process_incarnation_matches(id: &AppInstanceId) -> bool {
    autoreleasepool(|_| running_application(id).is_some())
}

#[must_use]
pub fn is_process_incarnation_running(id: &AppInstanceId) -> bool {
    let running = process_incarnation_matches(id);
    if !running {
        evict_managed_app(id);
    }
    running
}

pub(super) fn app_instance_with_pid(application: &NSRunningApplication, pid: i32) -> AppInstanceId {
    AppInstanceId {
        pid,
        launched_at: launch_time(application, pid),
    }
}

/// Trampoline-launched bundles (such as Xcode's `DeviceHub.app`, whose
/// `CFBundleExecutable` is `DevicesTrampoline`) can register an `LSASN` whose
/// `NSRunningApplication.processIdentifier()` reports `-1` even while the real
/// process owns windows in the window server. Match against window-server
/// owner PIDs to recover the real process identifier.
fn resolve_application_pid(application: &NSRunningApplication) -> Option<i32> {
    let pid = application.processIdentifier();
    if pid > 0 {
        return Some(pid);
    }
    let windows = window_server_windows(CGWindowListOption::OptionAll)?;
    let mut checked = HashSet::new();
    for window in windows {
        if window.pid <= 0 || !checked.insert(window.pid) {
            continue;
        }
        if NSRunningApplication::runningApplicationWithProcessIdentifier(window.pid)
            .is_some_and(|candidate| *candidate == *application)
        {
            return Some(window.pid);
        }
    }
    None
}

/// Processes started outside LaunchServices (a bare executable run from a
/// terminal) have no `launchDate`; their kernel start time identifies the
/// process incarnation instead.
pub(super) fn launch_time(application: &NSRunningApplication, pid: i32) -> u64 {
    application.launchDate().map_or_else(
        || process_start_time(pid).unwrap_or(0),
        |date| (date.timeIntervalSince1970() * 1000.0).round() as u64,
    )
}

pub(super) fn process_start_time(pid: i32) -> Option<u64> {
    if pid <= 0 {
        return None;
    }
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: `info` is a writable buffer of exactly `size` bytes, and
    // `PROC_PIDTBSDINFO` writes at most one `proc_bsdinfo` into it.
    let written = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if written != size {
        return None;
    }
    // SAFETY: the kernel filled the whole struct.
    let info = unsafe { info.assume_init() };
    let millis = info.pbi_start_tvsec * 1000 + info.pbi_start_tvusec / 1000;
    (millis > 0).then_some(millis)
}

fn observe_application(
    application: &NSRunningApplication,
    pid: i32,
    onscreen: &HashMap<CGWindowID, i64>,
    output: Rect,
    screen_locked: bool,
) -> Option<AppObservation> {
    if pid <= 0
        || pid == std::process::id() as i32
        || is_own_application(application)
        || application.isHidden()
    {
        return None;
    }

    let id = app_instance_with_pid(application, pid);
    let accessory = application.activationPolicy() == NSApplicationActivationPolicy::Accessory;
    let bundle_id = application
        .bundleIdentifier()
        .map_or_else(|| "unknown".into(), |value| value.to_string());
    if bundle_id == LOCK_SCREEN_BUNDLE_ID {
        return None;
    }
    if screen_locked {
        return Some(AppObservation::Unavailable {
            app: id,
            error: "screen is locked".into(),
        });
    }
    let app_element = application_element(pid);

    let windows = match attribute::<CFArray>(&app_element, "AXWindows") {
        Ok(windows) => windows,
        Err(error) => {
            return Some(AppObservation::Unavailable {
                app: id,
                error: format!("AXWindows: {error:?}"),
            });
        }
    };

    let focused_element = match attribute::<AXUIElement>(&app_element, "AXFocusedWindow") {
        Ok(window) => Some(window),
        Err(AXError::NoValue | AXError::AttributeUnsupported) => None,
        Err(error) => {
            return Some(AppObservation::Unavailable {
                app: id,
                error: format!("AXFocusedWindow: {error:?}"),
            });
        }
    };
    let focused_id = focused_element
        .as_ref()
        .and_then(|window| physical_window_id(&id, window));
    let focused = focused_id
        .clone()
        .filter(|window| onscreen.contains_key(&window.window_id));
    let mut observed = Vec::new();
    let mut current_windows = HashSet::new();
    let elements = focused_element
        .as_deref()
        .into_iter()
        .map(|window| (window, focused_id.clone()))
        .chain(cf_array_items::<AXUIElement>(&windows).map(|window| (window, None)));
    for (window, known_id) in elements {
        let Some(window_id) = known_id.or_else(|| physical_window_id(&id, window)) else {
            return Some(AppObservation::Unavailable {
                app: id,
                error: "physical window ID unavailable".into(),
            });
        };
        if !current_windows.insert(window_id.clone()) {
            continue;
        }
        set_ax_timeout(window);
        match observe_window(window_id, accessory, window, onscreen, output) {
            Ok(WindowObservation::Observed(observation)) => {
                cache_managed_window(&observation.id, window);
                observed.push(observation);
            }
            Ok(WindowObservation::Ignored) => {}
            Err(error) => return Some(AppObservation::Unavailable { app: id, error }),
        }
    }
    prune_managed_app(&id, &current_windows);

    if id.launched_at == 0 && !observed.is_empty() {
        return Some(AppObservation::Unavailable {
            app: id,
            error: "application launch date is unavailable".into(),
        });
    }

    Some(AppObservation::Complete {
        app: ObservedApplication { id, bundle_id },
        focused,
        windows: observed,
    })
}

pub(super) enum WindowObservation {
    Observed(ObservedWindow),
    Ignored,
}

const MIN_MANAGEABLE_WINDOW_DIMENSION: i32 = 64;

pub(super) fn observe_window(
    id: PhysicalWindowId,
    accessory: bool,
    window: &AXUIElement,
    onscreen: &HashMap<CGWindowID, i64>,
    output: Rect,
) -> Result<WindowObservation, String> {
    let Some(&layer) = onscreen.get(&id.window_id) else {
        return Ok(WindowObservation::Ignored);
    };
    let role =
        attribute::<CFString>(window, "AXRole").map_err(|error| format!("AXRole: {error:?}"))?;
    if *role != *CFString::from_static_str("AXWindow") {
        return Ok(WindowObservation::Ignored);
    }
    let subrole = match attribute::<CFString>(window, "AXSubrole") {
        Ok(subrole) => subrole,
        Err(AXError::NoValue | AXError::AttributeUnsupported) => {
            return Ok(WindowObservation::Ignored);
        }
        Err(error) => return Err(format!("AXSubrole: {error:?}")),
    };
    if *subrole != *CFString::from_static_str("AXStandardWindow") {
        return Ok(WindowObservation::Ignored);
    }

    if optional_bool_attribute(window, "AXMinimized")
        .map_err(|error| format!("AXMinimized: {error:?}"))?
        || optional_bool_attribute(window, "AXFullScreen")
            .map_err(|error| format!("AXFullScreen: {error:?}"))?
        || !is_settable(window, "AXPosition")
            .map_err(|error| format!("AXPosition settable: {error:?}"))?
    {
        return Ok(WindowObservation::Ignored);
    }

    let frame = window_frame(window).ok_or("AX frame was unavailable")?;
    if frame.width < MIN_MANAGEABLE_WINDOW_DIMENSION
        || frame.height < MIN_MANAGEABLE_WINDOW_DIMENSION
        || !output.contains_center(frame)
    {
        return Ok(WindowObservation::Ignored);
    }
    let fullscreen_enabled = button_is_enabled(window, "AXFullScreenButton");
    let has_close_button = attribute::<AXUIElement>(window, "AXCloseButton").is_ok();
    if (layer != 0 || accessory) && !fullscreen_enabled && !has_close_button {
        return Ok(WindowObservation::Ignored);
    }
    let modal = optional_bool_attribute(window, "AXModal").unwrap_or(false);
    let dialog = modal || (!fullscreen_enabled && (has_close_button || accessory));
    let resizable = !dialog
        && is_settable(window, "AXSize").map_err(|error| format!("AXSize settable: {error:?}"))?;
    let title = match attribute::<CFString>(window, "AXTitle") {
        Ok(title) => Some(std::sync::Arc::from(title.to_string())),
        Err(_) => None,
    };
    Ok(WindowObservation::Observed(ObservedWindow {
        id,
        frame,
        resizable,
        title,
    }))
}
