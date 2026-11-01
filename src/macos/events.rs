use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    io::{self, PipeReader, PipeWriter, Read, Write},
    os::fd::{AsRawFd, RawFd},
    ptr::{self, NonNull},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};
use objc2_application_services::{AXError, AXObserver, AXUIElement};
use objc2_core_foundation::{
    CFArray, CFFileDescriptor, CFOptionFlags, CFRetained, CFRunLoop, CFRunLoopSource, CFString,
    kCFFileDescriptorReadCallBack, kCFRunLoopDefaultMode,
};
use objc2_core_graphics::CGWindowListOption;

use crate::AppInstanceId;

use super::{
    app_instance_with_pid,
    ax::{application_element, attribute, cf_array_items, set_ax_timeout, window_server_windows},
    is_own_application,
};

/// Park the AX run loop until a window notification or wake pipe fires.
const IDLE_RUN_LOOP_SECONDS: f64 = 86_400.0;
const FALLBACK_POLL_SECONDS: f64 = 0.1;

pub struct WindowEventMonitor {
    invalidation: Arc<EventInvalidation>,
    applications: Arc<Mutex<Option<Vec<AppInstanceId>>>>,
    stopped: Arc<AtomicBool>,
    wake: Option<PipeWriter>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct EventInvalidation {
    dirty: AtomicBool,
    window_created: AtomicBool,
}

impl WindowEventMonitor {
    #[must_use]
    pub fn new() -> Self {
        let invalidation = Arc::new(EventInvalidation {
            dirty: AtomicBool::new(true),
            window_created: AtomicBool::new(false),
        });
        let applications = Arc::new(Mutex::new(None));
        let stopped = Arc::new(AtomicBool::new(false));
        let (reader, wake) = match nonblocking_pipe() {
            Some((reader, writer)) => (Some(reader), Some(writer)),
            None => (None, None),
        };
        let thread_invalidation = Arc::clone(&invalidation);
        let thread_applications = Arc::clone(&applications);
        let thread_stopped = Arc::clone(&stopped);
        let thread = thread::Builder::new()
            .name("spaceman-ax-events".into())
            .spawn(move || {
                run_event_loop(
                    reader,
                    thread_invalidation,
                    thread_applications,
                    thread_stopped,
                );
            })
            .ok();
        let monitor = Self {
            invalidation,
            applications,
            stopped,
            wake,
            thread,
        };
        monitor.refresh_applications();
        monitor
    }

    pub fn take_dirty(&self) -> bool {
        self.invalidation.dirty.swap(false, Ordering::AcqRel)
    }

    pub fn take_window_created(&self) -> bool {
        self.invalidation
            .window_created
            .swap(false, Ordering::AcqRel)
    }

    pub fn invalidate(&self) {
        self.invalidation.dirty.store(true, Ordering::Release);
    }

    pub fn refresh_applications(&self) {
        if let Ok(mut pending) = self.applications.lock() {
            *pending = Some(running_application_instances());
        }
        if let Some(mut wake) = self.wake.as_ref() {
            // `WouldBlock` means an unread wake byte is already queued in the pipe buffer.
            drop(wake.write(&[1]));
        }
    }
}

impl Default for WindowEventMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WindowEventMonitor {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        // Closing the write end wakes `CFFileDescriptor` immediately with EOF.
        self.wake = None;
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

fn run_event_loop(
    reader: Option<PipeReader>,
    invalidation: Arc<EventInvalidation>,
    applications: Arc<Mutex<Option<Vec<AppInstanceId>>>>,
    stopped: Arc<AtomicBool>,
) {
    let Some(run_loop) = CFRunLoop::current() else {
        return;
    };
    let mut wake = reader.and_then(|reader| EventWake::new(reader, &run_loop));
    let timeout = if wake.is_some() {
        IDLE_RUN_LOOP_SECONDS
    } else {
        FALLBACK_POLL_SECONDS
    };
    let mut registrations = HashMap::<AppInstanceId, EventRegistration>::new();
    let mut known_applications = HashSet::new();
    while !stopped.load(Ordering::Acquire) {
        let keep_running = autoreleasepool(|_| {
            if let Some(wake) = &mut wake
                && !wake.drain_and_arm()
            {
                return false;
            }
            if stopped.load(Ordering::Acquire) {
                return false;
            }
            let latest = applications
                .lock()
                .ok()
                .and_then(|mut pending| pending.take());
            if let Some(pids) = latest {
                sync_registrations(
                    pids,
                    &mut known_applications,
                    &mut registrations,
                    &invalidation,
                    &stopped,
                    &run_loop,
                );
            }
            // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
            CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, timeout, true);
            true
        });
        if !keep_running {
            break;
        }
    }
    autoreleasepool(|_| {
        if let Some(wake) = wake {
            // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
            run_loop.remove_source(Some(&wake.source), unsafe { kCFRunLoopDefaultMode });
            wake.descriptor.invalidate();
        }
        for registration in registrations.into_values() {
            // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
            run_loop.remove_source(Some(&registration.source), unsafe { kCFRunLoopDefaultMode });
        }
    });
}

fn sync_registrations(
    pids: Vec<AppInstanceId>,
    known_applications: &mut HashSet<AppInstanceId>,
    registrations: &mut HashMap<AppInstanceId, EventRegistration>,
    invalidation: &Arc<EventInvalidation>,
    stopped: &AtomicBool,
    run_loop: &CFRunLoop,
) {
    let desired: HashSet<_> = pids.into_iter().collect();
    if desired != *known_applications {
        invalidation.dirty.store(true, Ordering::Release);
        known_applications.clone_from(&desired);
    }
    registrations.retain(|app, registration| {
        if desired.contains(app) {
            true
        } else {
            // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
            run_loop.remove_source(Some(&registration.source), unsafe { kCFRunLoopDefaultMode });
            false
        }
    });
    for app in desired {
        if stopped.load(Ordering::Acquire) {
            break;
        }
        if !registrations.contains_key(&app)
            && let Some(registration) = EventRegistration::new(app.pid, invalidation, run_loop)
        {
            registrations.insert(app, registration);
        }
    }
}

struct EventWake {
    reader: PipeReader,
    descriptor: CFRetained<CFFileDescriptor>,
    source: CFRetained<CFRunLoopSource>,
}

impl EventWake {
    fn new(reader: PipeReader, run_loop: &CFRunLoop) -> Option<Self> {
        // SAFETY: `reader` stays open inside `Self` until after `descriptor.invalidate()` runs
        // (`close_on_invalidate` is `false`), `wake_pipe_callback` is a valid C callback, and
        // `context` is null.
        let descriptor = unsafe {
            CFFileDescriptor::new(
                None,
                reader.as_raw_fd(),
                false,
                Some(wake_pipe_callback),
                ptr::null(),
            )
        }?;
        let source = CFFileDescriptor::new_run_loop_source(None, Some(&descriptor), 0)?;
        // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
        run_loop.add_source(Some(&source), unsafe { kCFRunLoopDefaultMode });
        Some(Self {
            reader,
            descriptor,
            source,
        })
    }

    fn drain_and_arm(&mut self) -> bool {
        let mut buf = [0_u8; 64];
        loop {
            match self.reader.read(&mut buf) {
                Ok(0) => return false,
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        self.descriptor
            .enable_call_backs(kCFFileDescriptorReadCallBack);
        true
    }
}

unsafe extern "C-unwind" fn wake_pipe_callback(
    _descriptor: *mut CFFileDescriptor,
    _callback_types: CFOptionFlags,
    _info: *mut c_void,
) {
}

fn nonblocking_pipe() -> Option<(PipeReader, PipeWriter)> {
    let (reader, writer) = io::pipe().ok()?;
    if !set_nonblocking(reader.as_raw_fd()) || !set_nonblocking(writer.as_raw_fd()) {
        return None;
    }
    Some((reader, writer))
}

fn set_nonblocking(fd: RawFd) -> bool {
    // SAFETY: `fd` is an open pipe descriptor owned by the caller.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    // SAFETY: `flags` came from `F_GETFL` on the same valid pipe descriptor.
    flags >= 0 && unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == 0
}

struct EventRegistration {
    _observer: CFRetained<AXObserver>,
    _application: CFRetained<AXUIElement>,
    source: CFRetained<CFRunLoopSource>,
}

impl EventRegistration {
    fn new(pid: i32, invalidation: &Arc<EventInvalidation>, run_loop: &CFRunLoop) -> Option<Self> {
        let application = application_element(pid);
        let mut raw_observer = ptr::null_mut();
        // SAFETY: `AXObserverCreate` writes a +1 retained observer into `raw_observer` on success.
        let result = unsafe {
            AXObserver::create(
                pid,
                Some(window_event_callback),
                NonNull::from(&mut raw_observer),
            )
        };
        if result != AXError::Success {
            return None;
        }
        // SAFETY: `raw_observer` is non-null and +1 retained after `AXError::Success`.
        let observer = unsafe { CFRetained::from_raw(NonNull::new(raw_observer)?) };
        let refcon = Arc::as_ptr(invalidation).cast_mut().cast::<c_void>();
        // SAFETY: `invalidation` outlives all observers because `WindowEventMonitor::drop` joins
        // the event thread and removes every run-loop source before dropping `self.invalidation`.
        let window_created = unsafe {
            observer.add_notification(
                &application,
                &CFString::from_static_str("AXWindowCreated"),
                refcon,
            )
        };
        if !matches!(
            window_created,
            AXError::Success | AXError::NotificationAlreadyRegistered
        ) {
            return None;
        }
        // SAFETY: Same lifetime invariant as `AXWindowCreated` above.
        let _: AXError = unsafe {
            observer.add_notification(
                &application,
                &CFString::from_static_str("AXFocusedWindowChanged"),
                refcon,
            )
        };
        subscribe_window_notifications(&observer, &application, refcon);
        // SAFETY: `observer` is a valid live `AXObserver`.
        let source = unsafe { observer.run_loop_source() };
        // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
        run_loop.add_source(Some(&source), unsafe { kCFRunLoopDefaultMode });
        Some(Self {
            _observer: observer,
            _application: application,
            source,
        })
    }
}

unsafe extern "C-unwind" fn window_event_callback(
    observer: NonNull<AXObserver>,
    element: NonNull<AXUIElement>,
    notification: NonNull<CFString>,
    refcon: *mut c_void,
) {
    // SAFETY: Core Foundation guarantees `notification` is a valid `CFString` for the callback.
    let window_created =
        unsafe { notification.as_ref() } == &*CFString::from_static_str("AXWindowCreated");
    // SAFETY: `refcon` was created from `Arc::as_ptr(&invalidation)` and stays valid until the
    // event thread removes all sources before `WindowEventMonitor` drops the `Arc`.
    if let Some(invalidation) = unsafe { refcon.cast::<EventInvalidation>().as_ref() } {
        if window_created {
            invalidation.window_created.store(true, Ordering::Release);
        }
        invalidation.dirty.store(true, Ordering::Release);
    }
    if window_created {
        subscribe_one_window(
            // SAFETY: `observer` and `element` are non-null handles supplied by HIServices.
            unsafe { observer.as_ref() },
            // SAFETY: `element` is the newly created window `AXUIElement` supplied by HIServices.
            unsafe { element.as_ref() },
            refcon,
        );
    }
}

fn subscribe_window_notifications(
    observer: &AXObserver,
    application: &AXUIElement,
    refcon: *mut c_void,
) {
    let Ok(windows) = attribute::<CFArray>(application, "AXWindows") else {
        return;
    };
    for window in cf_array_items::<AXUIElement>(&windows) {
        subscribe_one_window(observer, window, refcon);
    }
}

fn subscribe_one_window(observer: &AXObserver, window: &AXUIElement, refcon: *mut c_void) {
    set_ax_timeout(window);
    for notification in [
        "AXUIElementDestroyed",
        "AXMoved",
        "AXResized",
        "AXWindowMiniaturized",
        "AXWindowDeminiaturized",
        "AXTitleChanged",
    ] {
        // SAFETY: `observer` and `window` are live AX objects and `refcon` outlives the observer.
        let _: AXError = unsafe {
            observer.add_notification(window, &CFString::from_static_str(notification), refcon)
        };
    }
}

pub(super) fn running_application_instances() -> Vec<AppInstanceId> {
    autoreleasepool(|_| {
        let self_pid = std::process::id() as i32;
        let applications = NSWorkspace::sharedWorkspace().runningApplications();
        let mut seen_pids = HashSet::new();
        let mut instances: Vec<_> = (0..applications.count())
            .filter_map(|index| {
                let application = applications.objectAtIndex(index);
                let pid = application.processIdentifier();
                (pid > 0
                    && pid != self_pid
                    && application.activationPolicy() != NSApplicationActivationPolicy::Prohibited
                    && !is_own_application(&application)
                    && seen_pids.insert(pid))
                .then(|| app_instance_with_pid(&application, pid))
            })
            .collect();
        if let Some(windows) = window_server_windows(CGWindowListOption::OptionAll) {
            for window in windows {
                if window.pid > 0
                    && window.pid != self_pid
                    && seen_pids.insert(window.pid)
                    && let Some(application) =
                        NSRunningApplication::runningApplicationWithProcessIdentifier(window.pid)
                    && !is_own_application(&application)
                {
                    instances.push(app_instance_with_pid(&application, window.pid));
                }
            }
        }
        instances
    })
}
