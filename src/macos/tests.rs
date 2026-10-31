use std::thread;

use crate::{AppInstanceId, DisplaySelector, PhysicalWindowId, Rect};

use super::{
    DisplayOutput, DisplayOutputs,
    apply::{group_placements_by_app, position_matches},
    ax::{WindowServerWindow, top_window_at_point},
    events::running_application_instances,
    frontmost_application, is_process_incarnation_running, process_start_time,
};

#[test]
fn processes_without_a_launch_date_are_identified_by_their_kernel_start_time() {
    let now = crate::config::now().as_millis() as u64;
    let started = process_start_time(std::process::id() as i32).unwrap();
    assert!(started <= now && now - started < 3_600_000);
    assert_eq!(process_start_time(std::process::id() as i32), Some(started));
    assert_eq!(process_start_time(0), None);
}

#[test]
fn pointer_hit_testing_uses_front_to_back_geometry_without_ax_focus() {
    let front = WindowServerWindow {
        pid: 1,
        id: 10,
        frame: Rect {
            x: -1000,
            y: -200,
            width: 400,
            height: 300,
        },
    };
    let behind = WindowServerWindow {
        pid: 2,
        id: 20,
        frame: Rect {
            width: 800,
            ..front.frame
        },
    };
    let windows = [front, behind];
    assert_eq!(top_window_at_point(&windows, (-900, -100)).unwrap().id, 10);
    assert_eq!(top_window_at_point(&windows, (-500, -100)).unwrap().id, 20);
    assert!(top_window_at_point(&windows, (100, 100)).is_none());
}

#[test]
fn position_only_validation_ignores_window_size() {
    let target = Rect {
        x: 100,
        y: 200,
        width: 800,
        height: 600,
    };
    let differently_sized = Rect {
        width: 320,
        height: 240,
        ..target
    };

    assert!(position_matches(differently_sized, target));
}

#[test]
fn managed_layout_reads_each_app_window_list_once() {
    let app = AppInstanceId {
        pid: 1,
        launched_at: 1,
    };
    let windows: Vec<_> = (1..=256)
        .map(|window_id| PhysicalWindowId {
            app: app.clone(),
            window_id,
        })
        .collect();
    let placements: Vec<_> = windows
        .iter()
        .map(|window| (window, Rect::default()))
        .collect();

    let groups = group_placements_by_app(&placements);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].0, app);
    assert_eq!(groups[0].1.len(), 256);
}

#[test]
fn arbitrary_display_topology_resolves_preferences_and_fallbacks() {
    let outputs = DisplayOutputs::new(vec![
        DisplayOutput {
            id: "main-id".into(),
            name: "Built-in".into(),
            frame: Rect::default(),
            is_main: true,
        },
        DisplayOutput {
            id: "second-id".into(),
            name: "Studio Display".into(),
            frame: Rect {
                x: 100,
                ..Rect::default()
            },
            is_main: false,
        },
        DisplayOutput {
            id: "third-id".into(),
            name: "Projector".into(),
            frame: Rect {
                x: 200,
                ..Rect::default()
            },
            is_main: false,
        },
    ])
    .unwrap();

    assert_eq!(outputs.displays().len(), 3);
    assert_eq!(
        outputs
            .resolve(&DisplaySelector::Preferred("third-id".into()))
            .id,
        "third-id"
    );
    assert_eq!(
        outputs
            .resolve(&DisplaySelector::Preferred("missing".into()))
            .id,
        "main-id"
    );
}

fn current_phys_footprint() -> u64 {
    // SAFETY: `rusage_info_v4` is a plain C struct of integer counters valid when zero-initialized.
    let mut info = unsafe { std::mem::zeroed::<libc::rusage_info_v4>() };
    // SAFETY: `info` is writable stack storage matching `RUSAGE_INFO_V4`.
    let rc = unsafe {
        libc::proc_pid_rusage(
            libc::getpid(),
            libc::RUSAGE_INFO_V4,
            (&raw mut info).cast::<libc::rusage_info_t>(),
        )
    };
    assert_eq!(rc, 0, "proc_pid_rusage failed");
    info.ri_phys_footprint
}

#[test]
fn worker_thread_queries_drain_objective_c_autorelease_pool() {
    let (baseline, after) = thread::spawn(|| {
        for _ in 0..10 {
            let apps = running_application_instances();
            drop(frontmost_application());
            if let Some(first) = apps.first() {
                std::hint::black_box(is_process_incarnation_running(first));
            }
        }
        let baseline = current_phys_footprint();
        for _ in 0..400 {
            let apps = running_application_instances();
            drop(frontmost_application());
            for app in &apps {
                std::hint::black_box(is_process_incarnation_running(app));
            }
        }
        let after = current_phys_footprint();
        (baseline, after)
    })
    .join()
    .expect("worker thread panicked");

    let growth = after.saturating_sub(baseline);
    assert!(
        growth < 2 * 1024 * 1024,
        "worker thread leaked {growth} bytes of phys_footprint across 400 cycles (baseline={baseline}, after={after})"
    );
}
