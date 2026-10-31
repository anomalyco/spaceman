use std::{
    cell::RefCell,
    path::PathBuf,
    ptr::NonNull,
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc::Sender,
    },
};

use block2::{Block, RcBlock};
use libloading::Library;
use objc2::runtime::{AnyClass, AnyObject, Bool, NSObjectProtocol, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, msg_send, rc::Retained};
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSInteger, NSNotification, NSNotificationCenter, NSObject, NSString};

use crate::app::Action;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum UpdateStatus {
    Unavailable,
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable,
}

struct DelegateIvars {
    actions: Sender<Action>,
}

objc2::define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateIvars]
    struct SparkleDelegate;

    unsafe impl NSObjectProtocol for SparkleDelegate {}

    impl SparkleDelegate {
        #[unsafe(method(updater:didFinishUpdateCycleForUpdateCheck:error:))]
        fn did_finish_update_cycle(
            &self,
            _updater: &AnyObject,
            _update_check: NSInteger,
            _error: Option<&AnyObject>,
        ) {
            if status() == UpdateStatus::Checking {
                set_status(UpdateStatus::Idle);
            }
        }

        #[unsafe(method(updater:shouldPostponeRelaunchForUpdate:untilInvokingBlock:))]
        fn should_postpone_relaunch(
            &self,
            _updater: &AnyObject,
            _item: &AnyObject,
            install_handler: &Block<dyn Fn()>,
        ) -> Bool {
            let handler = install_handler.copy();
            let accepted = PENDING_INSTALL.with(|pending| {
                let mut pending = pending.borrow_mut();
                if pending.is_some() {
                    false
                } else {
                    *pending = Some(handler);
                    true
                }
            });
            if accepted {
                drop(self.ivars().actions.send(Action::InstallUpdate));
            }
            Bool::YES
        }
    }
);

impl SparkleDelegate {
    fn new(actions: Sender<Action>, marker: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(marker).set_ivars(DelegateIvars { actions });
        // SAFETY: `NSObject`'s `-init` takes no arguments and returns an initialized `Self`.
        unsafe { msg_send![super(this), init] }
    }
}

struct SparkleUpdater {
    controller: Retained<AnyObject>,
    _delegate: Retained<SparkleDelegate>,
    observers: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
    _framework: Library,
}

static UPDATE_STATUS: AtomicU8 = AtomicU8::new(UpdateStatus::Unavailable as u8);

impl Drop for SparkleUpdater {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in &self.observers {
            // SAFETY: Each observer token was returned by `defaultCenter` in `start`.
            unsafe { center.removeObserver(observer.as_ref()) };
        }
    }
}

thread_local! {
    static UPDATER: RefCell<Option<SparkleUpdater>> = const { RefCell::new(None) };
    static PENDING_INSTALL: RefCell<Option<RcBlock<dyn Fn()>>> = const { RefCell::new(None) };
}

pub(crate) fn start(actions: Sender<Action>) -> Result<(), String> {
    let marker = MainThreadMarker::new().ok_or("Sparkle must start on the main thread")?;
    let Some(framework_path) = framework_path()? else {
        return Ok(());
    };
    // SAFETY: Loads the signed Sparkle framework embedded inside Spaceman's own app bundle.
    let framework = unsafe { Library::new(&framework_path) }
        .map_err(|error| format!("could not load {}: {error}", framework_path.display()))?;
    let center = NSNotificationCenter::defaultCenter();
    let observers = [
        (
            "SUUpdaterDidFindValidUpdateNotification",
            UpdateStatus::UpdateAvailable,
        ),
        (
            "SUUpdaterDidNotFindUpdateNotification",
            UpdateStatus::UpToDate,
        ),
    ]
    .into_iter()
    .map(|(name, status)| {
        let name = NSString::from_str(name);
        let block = RcBlock::new(move |_: NonNull<NSNotification>| set_status(status));
        // SAFETY: `block` only writes an atomic `u8` and owns no borrowed stack references.
        unsafe {
            center.addObserverForName_object_queue_usingBlock(Some(&name), None, None, &block)
        }
    })
    .collect();
    let class = AnyClass::get(c"SPUStandardUpdaterController")
        .ok_or("Sparkle updater class is unavailable")?;
    let delegate = SparkleDelegate::new(actions, marker);
    // SAFETY: `SPUStandardUpdaterController` is allocated and initialized on the main thread
    // with a retained `delegate` kept alive alongside `controller` in `SparkleUpdater`.
    let controller: Retained<AnyObject> = unsafe {
        let allocated: *mut AnyObject = msg_send![class, alloc];
        let controller: *mut AnyObject = msg_send![
            allocated,
            initWithStartingUpdater: Bool::YES,
            updaterDelegate: &*delegate,
            userDriverDelegate: std::ptr::null_mut::<AnyObject>()
        ];
        Retained::from_raw(controller).ok_or("Sparkle updater initialization returned null")?
    };
    set_status(UpdateStatus::Idle);
    UPDATER.with(|updater| {
        *updater.borrow_mut() = Some(SparkleUpdater {
            controller,
            _delegate: delegate,
            observers,
            _framework: framework,
        });
    });
    Ok(())
}

pub(crate) fn check_for_updates() {
    let Some(marker) = MainThreadMarker::new() else {
        return;
    };
    UPDATER.with(|updater| {
        let updater = updater.borrow();
        let Some(updater) = updater.as_ref() else {
            set_status(UpdateStatus::Unavailable);
            return;
        };
        set_status(UpdateStatus::Checking);
        // SAFETY: Invokes standard `SPUStandardUpdaterController` selectors on the main thread.
        unsafe {
            #[allow(deprecated)]
            NSApplication::sharedApplication(marker).activateIgnoringOtherApps(true);
            let _: () =
                msg_send![&*updater.controller, checkForUpdates: std::ptr::null_mut::<AnyObject>()];
            let user_driver: *mut AnyObject = msg_send![&*updater.controller, userDriver];
            let _: () = msg_send![user_driver, showUpdateInFocus];
        }
    });
}

pub(crate) fn status() -> UpdateStatus {
    match UPDATE_STATUS.load(Ordering::Acquire) {
        value if value == UpdateStatus::Idle as u8 => UpdateStatus::Idle,
        value if value == UpdateStatus::Checking as u8 => UpdateStatus::Checking,
        value if value == UpdateStatus::UpToDate as u8 => UpdateStatus::UpToDate,
        value if value == UpdateStatus::UpdateAvailable as u8 => UpdateStatus::UpdateAvailable,
        _ => UpdateStatus::Unavailable,
    }
}

pub(crate) fn continue_install() -> Result<(), String> {
    let handler = PENDING_INSTALL.with(|pending| pending.borrow_mut().take());
    let handler = handler.ok_or("Sparkle install continuation is missing")?;
    handler.call(());
    Ok(())
}

fn set_status(status: UpdateStatus) {
    UPDATE_STATUS.store(status as u8, Ordering::Release);
}

fn framework_path() -> Result<Option<PathBuf>, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let Some(macos_dir) = executable.parent() else {
        return Ok(None);
    };
    let path = macos_dir.join("../Frameworks/Sparkle.framework/Versions/B/Sparkle");
    Ok(path.exists().then_some(path))
}
