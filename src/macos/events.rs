use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    ptr,
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Sender, channel},
    },
    thread::{self, JoinHandle},
};

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_application_services::{AXError, AXObserver, AXUIElement};
use objc2_core_foundation::{
    CFArray, CFRetained, CFRunLoop, CFRunLoopSource, CFString, kCFRunLoopDefaultMode,
};
use objc2_core_graphics::CGWindowListOption;

use crate::AppInstanceId;

use super::{
    app_instance_with_pid,
    ax::{application_element, attribute, cf_array_items, window_server_windows},
    is_own_application,
};

pub struct WindowEventMonitor {
    invalidation: Arc<EventInvalidation>,
    applications: Sender<Vec<AppInstanceId>>,
    stopped: Arc<AtomicBool>,
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
        let stopped = Arc::new(AtomicBool::new(false));
        let (applications, receiver) = channel::<Vec<AppInstanceId>>();
        let thread_invalidation = Arc::clone(&invalidation);
        let thread_stopped = Arc::clone(&stopped);
        let thread = thread::Builder::new()
            .name("spaceman-ax-events".into())
            .spawn(move || {
                let Some(run_loop) = CFRunLoop::current() else {
                    return;
                };
                let mut registrations = HashMap::<AppInstanceId, EventRegistration>::new();
                let mut known_applications = HashSet::new();
                while !thread_stopped.load(Ordering::Acquire) {
                    autoreleasepool(|_| {
                        let mut latest = None;
                        while let Ok(pids) = receiver.try_recv() {
                            latest = Some(pids);
                        }
                        if let Some(pids) = latest {
                            let desired: HashSet<_> = pids.into_iter().collect();
                            if desired != known_applications {
                                thread_invalidation.dirty.store(true, Ordering::Release);
                                known_applications.clone_from(&desired);
                            }
                            registrations.retain(|app, registration| {
                                if desired.contains(app) {
                                    true
                                } else {
                                    // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
                                    run_loop.remove_source(Some(&registration.source), unsafe {
                                        kCFRunLoopDefaultMode
                                    });
                                    false
                                }
                            });
                            for app in desired {
                                if !registrations.contains_key(&app)
                                    && let Some(registration) = EventRegistration::new(
                                        app.pid,
                                        &thread_invalidation,
                                        &run_loop,
                                    )
                                {
                                    registrations.insert(app, registration);
                                }
                            }
                        }
                        // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
                        CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, 0.1, true);
                    });
                }
                autoreleasepool(|_| {
                    for registration in registrations.into_values() {
                        // SAFETY: `kCFRunLoopDefaultMode` is a static Core Foundation mode constant.
                        run_loop.remove_source(Some(&registration.source), unsafe {
                            kCFRunLoopDefaultMode
                        });
                    }
                });
            })
            .ok();
        let monitor = Self {
            invalidation,
            applications,
            stopped,
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
        drop(self.applications.send(running_application_instances()));
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
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
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
