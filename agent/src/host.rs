//! Single-process macOS Rune host: runs Rune's AppKit event loop on the main
//! thread alongside the native status item, Sparkle updater, winit termination
//! gate, and a 25 ms `NSTimer` pump in `NSRunLoopCommonModes`.

#![allow(unsafe_code)]

use std::{cell::RefCell, ptr::NonNull};

use block2::RcBlock;
use objc2::rc::{Retained, autoreleasepool};
use objc2_foundation::{NSRunLoop, NSRunLoopCommonModes, NSTimer};
use spaceman::app::{HostCallbacks, MainThreadAgent, PreparedAgent, TICK_INTERVAL, TickOutcome};

use crate::Ui;

struct Launch {
    prepared: PreparedAgent,
    window: rune::AppHandle<Ui>,
}

struct PumpState {
    agent: MainThreadAgent,
    window: rune::AppHandle<Ui>,
    timer: Retained<NSTimer>,
}

thread_local! {
    static LAUNCH: RefCell<Option<Launch>> = const { RefCell::new(None) };
    static PUMP: RefCell<Option<PumpState>> = const { RefCell::new(None) };
    static STARTUP_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn bring_window_forward(window: &rune::AppHandle<Ui>) {
    window.set_dock_icon(true);
    window.open();
}

pub fn run() -> Result<(), String> {
    redirect_stderr_to_log();
    let (prepared, bridge) = spaceman::app::prepare()?;
    let accessibility = spaceman::macos::accessibility_trusted();
    let initial_tour = crate::onboarding::launch_step(accessibility);
    let open_at_launch = initial_tour.is_some() || !accessibility;
    let mut app = crate::live_app(bridge, initial_tour, accessibility)
        .args(Vec::new())
        .accessory();
    app = if open_at_launch {
        app.keep_running()
    } else {
        app.windowless()
    };
    let window = app.handle();
    if open_at_launch {
        window.set_dock_icon(true);
    }
    LAUNCH.with(|slot| {
        *slot.borrow_mut() = Some(Launch {
            prepared,
            window: window.clone(),
        });
    });
    window.update_if(|_, _| {
        start_main_thread();
        false
    });
    let run_result = app.try_run();
    if let Some(state) = PUMP.with(|slot| slot.borrow_mut().take()) {
        state.timer.invalidate();
    }
    if let Some(error) = STARTUP_ERROR.with(|slot| slot.borrow_mut().take()) {
        return Err(error);
    }
    run_result
}

fn start_main_thread() {
    let Some(Launch { prepared, window }) = LAUNCH.with(|slot| slot.borrow_mut().take()) else {
        return;
    };
    let settings_window = window.clone();
    let setup_window = window.clone();
    let callbacks = HostCallbacks {
        open_settings: Box::new(move || {
            bring_window_forward(&settings_window);
            Ok(())
        }),
        open_setup: Box::new(move || {
            setup_window.update_if(|ui, _| {
                ui.start_tour();
                true
            });
            bring_window_forward(&setup_window);
            Ok(())
        }),
        save_hud: Box::new(|enabled| {
            let mut doc = crate::rules::Doc::load();
            doc.set_hud(enabled);
            doc.save()
        }),
    };
    let agent = match prepared.start_main_thread(callbacks) {
        Ok(agent) => agent,
        Err(error) => {
            STARTUP_ERROR.with(|slot| *slot.borrow_mut() = Some(error));
            spaceman::app::allow_immediate_termination();
            window.quit();
            return;
        }
    };
    let block = RcBlock::new(|_: NonNull<NSTimer>| {
        autoreleasepool(|_| tick_pump());
    });
    // SAFETY: Scheduled on the main thread's run loop; the block touches only main-thread state.
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_repeats_block(
            TICK_INTERVAL.as_secs_f64(),
            true,
            &block,
        )
    };
    // Keep pumping while the status-bar menu tracks pointer input.
    // SAFETY: `mainRunLoop` and `NSRunLoopCommonModes` are valid on the main thread.
    unsafe {
        NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes);
    }
    PUMP.with(|slot| {
        *slot.borrow_mut() = Some(PumpState {
            agent,
            window,
            timer,
        });
    });
}

fn tick_pump() {
    let outcome = PUMP.with(|slot| slot.borrow_mut().as_mut().map(|state| state.agent.tick()));
    match outcome {
        Some(TickOutcome::Continue) | None => {}
        Some(TickOutcome::Quit) => {
            if let Some(state) = PUMP.with(|slot| slot.borrow_mut().take()) {
                state.timer.invalidate();
                state.window.quit();
            }
        }
        Some(TickOutcome::TerminateAppKit) => {
            if let Some(state) = PUMP.with(|slot| slot.borrow_mut().take()) {
                state.timer.invalidate();
                drop(state);
            }
            spaceman::app::finish_appkit_termination();
        }
    }
}

/// When launched as an app bundle, stderr goes nowhere. Redirect it to a
/// size-capped, owner-only log file so runtime diagnostics survive. Terminal
/// launches keep stderr on the terminal.
fn redirect_stderr_to_log() {
    use std::{
        io::IsTerminal,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };
    if std::io::stderr().is_terminal() {
        return;
    }
    let Some(path) = spaceman::config::application_support_path("agent.log") else {
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
    // SAFETY: `file` is an open file descriptor and `STDERR_FILENO` is valid;
    // duplicating onto stderr atomically replaces it before worker threads start.
    unsafe {
        use std::os::fd::AsRawFd;
        libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO);
    }
}
