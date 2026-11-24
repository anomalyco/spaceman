use std::time::Duration;

use super::*;
use crate::{
    labs::LabFeature,
    strip::{PEEK_PX, StripWidth},
};

const LAB_ON: &str = "version = 1\n[labs]\nscrolling_strip = true\n";

fn column(x: i32, width: i32) -> crate::Rect {
    crate::Rect {
        x,
        y: 0,
        width,
        height: 800,
    }
}

fn press(
    runtime: &mut WorkspaceRuntime<FakeBackend>,
    action: WorkerAction,
) -> Result<CycleStatus, String> {
    press_on(runtime, action, outputs())
}

fn press_on(
    runtime: &mut WorkspaceRuntime<FakeBackend>,
    action: WorkerAction,
    outputs: macos::DisplayOutputs,
) -> Result<CycleStatus, String> {
    match runtime.execute(
        WorkerCommand::Action(action, outputs),
        &AtomicBool::new(false),
    ) {
        Some(WorkerResult::Status { result, .. }) => result,
        _ => panic!("an action returns status"),
    }
}

/// Four tiled windows on workspace 1 (think Ghostty, Zed, Safari and Linear), focus on the first.
/// The test's main display has a second one to its right: its left edge is free, its right edge
/// a seam.
fn four_windows(config: &str) -> (WorkspaceRuntime<FakeBackend>, Vec<crate::PhysicalWindowId>) {
    let windows: Vec<_> = (1..=4).map(|index| window(index * 10)).collect();
    let frames: Vec<_> = windows
        .iter()
        .zip(0..)
        .map(|(window, index)| (window.clone(), frame(index * 200)))
        .collect();
    let backend = FakeBackend {
        frames: frames.iter().cloned().collect(),
        focused: Some(windows[0].clone()),
        frontmost: Some(windows[0].app.clone()),
        observation: Some(observation_for_windows(
            "com.example.Test",
            &windows[0],
            &frames,
        )),
        ..FakeBackend::default()
    };
    let mut runtime = runtime_with_config(backend, Config::parse(config).unwrap());
    for window in &windows {
        bind(&mut runtime, WorkspaceId::ONE, window);
    }
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    (runtime, windows)
}

#[derive(Clone, Copy, Debug)]
enum At {
    /// On screen at this frame, not parked.
    Shown(crate::Rect),
    /// Peeking past the left edge, its on-screen frame recorded durably.
    PeekLeft(i32),
    /// Parked off every display, its frame recorded durably.
    Parked,
}

const fn peek_left(width: i32) -> At {
    At::PeekLeft(width)
}

fn assert_strip(
    runtime: &WorkspaceRuntime<FakeBackend>,
    windows: &[crate::PhysicalWindowId],
    expected: &[At],
) {
    for (index, (window, at)) in windows.iter().zip(expected).enumerate() {
        let frame = runtime.backend.frames[window];
        match at {
            At::Shown(expected) => {
                assert_eq!(frame, *expected, "window {index}");
                assert!(
                    !runtime.parked.contains_key(window),
                    "window {index} parked"
                );
                assert!(!runtime.backend.persisted.contains_key(window));
            }
            At::PeekLeft(width) => {
                assert_eq!(
                    (frame.x, frame.y, frame.width),
                    (-width + PEEK_PX, 0, *width),
                    "window {index}"
                );
                assert!(runtime.parked.contains_key(window), "window {index}");
                assert!(runtime.backend.persisted.contains_key(window));
            }
            At::Parked => {
                assert!(runtime.parked.contains_key(window), "window {index}");
                assert!(runtime.backend.persisted.contains_key(window));
                assert!(
                    !outputs()
                        .frames()
                        .any(|display| display.contains_center(frame)),
                    "window {index} at {frame:?}"
                );
            }
        }
    }
}

#[test]
fn lab_off_refuses_the_strip_and_tiles_as_usual_until_it_is_turned_on() {
    let (mut runtime, windows) = four_windows("version = 1\n");
    // A workspace switch left on while the Lab was on stays inert while it is off.
    runtime.workspaces.set_strip_layout(WorkspaceId::ONE, true);

    assert_eq!(
        press(&mut runtime, WorkerAction::ToggleStrip).err(),
        Some(LabFeature::ScrollingStrip.off_message())
    );
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[1].clone()));
    let split = [
        At::Shown(column(0, 250)),
        At::Shown(column(250, 250)),
        At::Shown(column(500, 250)),
        At::Shown(column(750, 250)),
    ];
    assert_strip(&runtime, &windows, &split);

    runtime.replace_config(Config::parse(LAB_ON).unwrap());
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    let strip = [
        At::Shown(column(0, 500)),
        At::Shown(column(500, 500)),
        At::Parked,
        At::Parked,
    ];
    assert_strip(&runtime, &windows, &strip);
}

/// Settings, a float or another unmanaged app having focus still toggles the strip: the press
/// falls back to the workspace showing on the pointer's display.
#[test]
fn toggle_without_a_managed_focus_acts_on_the_workspace_under_the_pointer() {
    let (mut runtime, _) = four_windows(LAB_ON);
    let settings = window(99);
    runtime.backend.focused = Some(settings.clone());
    runtime.backend.frontmost = Some(settings.app.clone());
    runtime.backend.pointer = Some((400, 300));

    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    assert!(runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    assert!(!runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));

    // Pointer on the second display: its own workspace toggles, workspace 1 stays as it is.
    let secondary = runtime
        .visible_workspaces(&outputs())
        .into_iter()
        .find(|resolved| resolved.display_id == "secondary-id")
        .map(|resolved| resolved.workspace)
        .expect("the second display shows a workspace");
    assert_ne!(secondary, WorkspaceId::ONE);
    runtime.backend.pointer = Some((1400, 300));
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    assert!(runtime.workspaces.uses_strip_layout(secondary));
    assert!(!runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
}

/// No focused window and no pointer reading (an empty desktop): the main display's workspace.
#[test]
fn toggle_with_nothing_focused_acts_on_the_main_display() {
    let (mut runtime, _) = four_windows(LAB_ON);
    runtime.backend.focused = None;
    runtime.backend.frontmost = None;
    runtime.backend.pointer = None;

    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    assert!(runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
}

#[test]
fn stepping_along_the_strip_peeks_parks_and_reveals_columns_durably() {
    let (mut runtime, windows) = four_windows(LAB_ON);

    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    assert!(runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
    let start = [
        At::Shown(column(0, 500)),
        At::Shown(column(500, 500)),
        At::Parked,
        At::Parked,
    ];
    assert_strip(&runtime, &windows, &start);

    // ⌥L onto a column already on screen: minimal scroll leaves the camera alone.
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[1].clone()));
    assert_strip(&runtime, &windows, &start);

    // ⌥L past the seam: Safari comes in, Ghostty peeks past the free left edge with its
    // last on-screen frame recorded.
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[2].clone()));
    let scrolled = [
        peek_left(500),
        At::Shown(column(0, 500)),
        At::Shown(column(500, 500)),
        At::Parked,
    ];
    assert_strip(&runtime, &windows, &scrolled);
    assert_eq!(runtime.backend.persisted[&windows[0]], column(0, 500));

    // ⌥H stays inside the view; ⌥H again brings Ghostty back and its recovery frame goes.
    press(&mut runtime, WorkerAction::Focus(Direction::Left)).unwrap();
    assert_strip(&runtime, &windows, &scrolled);
    press(&mut runtime, WorkerAction::Focus(Direction::Left)).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[0].clone()));
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert_strip(&runtime, &windows, &start);

    // Observation cycles keep every slot and the viewport.
    for _ in 0..3 {
        runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    }
    assert_eq!(
        runtime.workspaces.engine(WorkspaceId::ONE).bound_windows(),
        windows
    );
    assert_strip(&runtime, &windows, &start);
    let state = runtime.backend.workspace_state.clone().unwrap();
    let mut restarted = Workspaces::default();
    restarted.restore(state);
    assert!(restarted.uses_strip_layout(WorkspaceId::ONE));
}

#[test]
fn focus_from_outside_spaceman_scrolls_to_a_parked_column() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();

    // Later, ⌘Tab or the Dock focuses Linear while it is parked.
    runtime.recently_parked.clear();
    runtime.backend.focused = Some(windows[3].clone());
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_strip(
        &runtime,
        &windows,
        &[
            peek_left(500),
            peek_left(500),
            At::Shown(column(0, 500)),
            At::Shown(column(500, 500)),
        ],
    );
}

#[test]
fn pause_and_quit_restore_every_peeking_and_parked_column() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    let recovery = runtime.parked.clone();
    assert_eq!(recovery.len(), 2);

    let result = runtime.execute(
        WorkerCommand::RestoreAll(Some(outputs())),
        &AtomicBool::new(false),
    );

    assert!(matches!(result, Some(WorkerResult::Restored { failed: 0 })));
    assert!(runtime.parked.is_empty());
    assert!(runtime.backend.persisted.is_empty());
    for window in &windows {
        let frame = runtime.backend.frames[window];
        assert!(outputs().main().frame.contains_center(frame));
        if let Some(original) = recovery.get(window) {
            assert_eq!((frame.x, frame.y), (original.x, original.y));
        }
    }
}

#[test]
fn switching_away_parks_every_column_and_back_reveals_only_the_viewport() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();

    press(
        &mut runtime,
        WorkerAction::SwitchWorkspace(WorkspaceId::TWO),
    )
    .unwrap();
    assert_strip(&runtime, &windows, &[At::Parked; 4]);

    press(
        &mut runtime,
        WorkerAction::SwitchWorkspace(WorkspaceId::ONE),
    )
    .unwrap();
    assert_strip(
        &runtime,
        &windows,
        &[
            At::Shown(column(0, 500)),
            At::Shown(column(500, 500)),
            At::Parked,
            At::Parked,
        ],
    );
}

#[test]
fn reordering_and_width_presets_follow_the_focused_column() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();

    // ⌥⇧L carries Ghostty one column right; it is still in view.
    press(&mut runtime, WorkerAction::Reorder(Direction::Right)).unwrap();
    let order = runtime.workspaces.engine(WorkspaceId::ONE).bound_windows();
    assert_eq!(
        order,
        vec![
            windows[1].clone(),
            windows[0].clone(),
            windows[2].clone(),
            windows[3].clone()
        ]
    );
    assert_strip(
        &runtime,
        &[windows[1].clone(), windows[0].clone()],
        &[At::Shown(column(0, 500)), At::Shown(column(500, 500))],
    );

    // ⌥= ⌥= ⌥=: 66%, 80%, then 100%, the view following Ghostty's right edge. Unsized
    // columns follow the last width picked.
    press(&mut runtime, WorkerAction::Resize(ResizeDirection::Grow)).unwrap();
    assert_eq!(runtime.backend.frames[&windows[0]], column(334, 666));
    assert_eq!(runtime.backend.frames[&windows[1]], column(-332, 666));
    press(&mut runtime, WorkerAction::Resize(ResizeDirection::Grow)).unwrap();
    assert_eq!(runtime.backend.frames[&windows[0]], column(200, 800));
    press(&mut runtime, WorkerAction::Resize(ResizeDirection::Grow)).unwrap();
    assert_eq!(runtime.backend.frames[&windows[0]], outputs().main().frame);
    assert_strip(&runtime, &[windows[1].clone()], &[peek_left(1000)]);
    assert_eq!(
        runtime.workspaces.strip(WorkspaceId::ONE).width(SlotId(10)),
        StripWidth::Full
    );

    // ⌥L and back: one full-width column at a time.
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert_eq!(runtime.backend.frames[&windows[2]], outputs().main().frame);
    press(&mut runtime, WorkerAction::Focus(Direction::Left)).unwrap();
    assert_eq!(runtime.backend.frames[&windows[0]], outputs().main().frame);

    // ⌥- four times, down to a third, and no further.
    for _ in 0..5 {
        press(&mut runtime, WorkerAction::Resize(ResizeDirection::Shrink)).unwrap();
    }
    assert_eq!(
        runtime.workspaces.strip(WorkspaceId::ONE).width(SlotId(10)),
        StripWidth::Third
    );
    assert_eq!(runtime.backend.frames[&windows[0]].width, 333);

    // ⌥` cycles to the next column, wrapping at the end.
    press(&mut runtime, WorkerAction::CycleFocus).unwrap();
    assert_eq!(runtime.backend.focused, Some(windows[2].clone()));
}

#[test]
fn turning_the_strip_off_reveals_every_column_in_the_split_layout() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();

    assert!(!runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
    assert_strip(
        &runtime,
        &windows,
        &[
            At::Shown(column(0, 250)),
            At::Shown(column(250, 250)),
            At::Shown(column(500, 250)),
            At::Shown(column(750, 250)),
        ],
    );

    // Fullscreen Layout and the strip exclude each other.
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::ToggleFullscreenLayout).unwrap();
    assert!(runtime.workspaces.uses_fullscreen_layout(WorkspaceId::ONE));
    assert!(!runtime.workspaces.uses_strip_layout(WorkspaceId::ONE));
    assert!(runtime.parked.is_empty());
}

#[test]
fn ghostty_native_tab_replacement_keeps_the_strip_column_and_width() {
    let previous = window(10);
    let other = window(20);
    let replacement = window(11);
    let windows = [(previous.clone(), frame(50)), (other.clone(), frame(500))];
    let backend = FakeBackend {
        frames: windows.iter().cloned().collect(),
        focused: Some(previous.clone()),
        frontmost: Some(previous.app.clone()),
        observation: Some(observation_for_windows(
            "com.mitchellh.ghostty",
            &previous,
            &windows,
        )),
        ..FakeBackend::default()
    };
    let mut runtime = runtime_with_config(backend, Config::parse(LAB_ON).unwrap());
    bind(&mut runtime, WorkspaceId::ONE, &previous);
    bind(&mut runtime, WorkspaceId::ONE, &other);
    let cancelled = AtomicBool::new(false);
    runtime.cycle(outputs(), &cancelled).unwrap();
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    for _ in 0..3 {
        press(&mut runtime, WorkerAction::Resize(ResizeDirection::Grow)).unwrap();
    }
    runtime.cycle(outputs(), &cancelled).unwrap();
    let full = outputs().main().frame;
    assert_eq!(runtime.backend.frames[&previous], full);

    runtime.backend.frames.remove(&previous);
    runtime.backend.frames.insert(replacement.clone(), full);
    runtime.backend.focused = Some(replacement.clone());
    runtime.backend.observation = Some(observation_for_windows(
        "com.mitchellh.ghostty",
        &replacement,
        &[(replacement.clone(), full), (other.clone(), frame(500))],
    ));
    runtime.cycle(outputs(), &cancelled).unwrap();

    let engine = runtime.workspaces.engine(WorkspaceId::ONE);
    assert_eq!(engine.slot_for_window(&replacement), Some(SlotId(10)));
    assert_eq!(
        engine.bound_windows(),
        vec![replacement.clone(), other.clone()]
    );
    assert_eq!(
        runtime.workspaces.strip(WorkspaceId::ONE).width(SlotId(10)),
        StripWidth::Full
    );
    assert_eq!(runtime.backend.frames[&replacement], full);
    assert!(runtime.parked.contains_key(&other));
}

/// An app that drags an out-of-view column back on screen in the background is pushed back:
/// past the seam to its parking spot, past the free edge to its sliver.
#[test]
fn a_strip_column_that_drifts_back_on_screen_is_put_back() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    let recovery = runtime.parked.clone();
    for window in [&windows[0], &windows[3]] {
        runtime.backend.frames.insert(window.clone(), frame(40));
    }
    runtime.backend.frontmost = None;
    runtime.recently_parked.clear();

    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();

    assert_strip(
        &runtime,
        &windows,
        &[
            peek_left(500),
            At::Shown(column(0, 500)),
            At::Shown(column(500, 500)),
            At::Parked,
        ],
    );
    assert_eq!(runtime.parked, recovery);
    assert_eq!(
        runtime.workspaces.workspace_of(&windows[3]),
        Some(WorkspaceId::ONE)
    );
}

/// On a lone laptop display both edges are free, so columns out of view peek. Docking a display
/// to the right turns that edge into a seam: the columns past it are parked against the new
/// displays at once, and focus follow cannot re-home them meanwhile.
#[test]
fn docking_a_display_parks_columns_that_peeked_past_the_new_seam() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    let laptop = macos::DisplayOutputs::new(vec![outputs().displays()[0].clone()]).unwrap();
    let cancelled = AtomicBool::new(false);
    runtime.cycle(laptop.clone(), &cancelled).unwrap();
    press_on(&mut runtime, WorkerAction::ToggleStrip, laptop).unwrap();
    for window in &windows[2..] {
        let at = runtime.backend.frames[window];
        assert_eq!((at.x, at.width), (1000 - PEEK_PX, 500), "{window:?}");
        assert!(runtime.parked.contains_key(window));
    }
    runtime.backend.frontmost = None;

    runtime.cycle(outputs(), &cancelled).unwrap();

    assert_strip(
        &runtime,
        &windows,
        &[
            At::Shown(column(0, 500)),
            At::Shown(column(500, 500)),
            At::Parked,
            At::Parked,
        ],
    );
    for window in &windows[2..] {
        assert_eq!(
            runtime.workspaces.workspace_of(window),
            Some(WorkspaceId::ONE)
        );
    }
}

fn glides(runtime: &mut WorkspaceRuntime<FakeBackend>) -> Vec<crate::strip::StripMotion> {
    runtime
        .strip_views
        .outbox
        .drain(..)
        .filter_map(|command| match command {
            crate::app::strip::StripCommand::Glide(motion) => Some(*motion),
            crate::app::strip::StripCommand::Halt(_) => None,
        })
        .collect()
}

/// With a main thread presenting glides, ⌥L hands the camera's spring to it instead of writing
/// frames, after recording the frame of every column the glide takes out of view. Once the
/// spring settles the worker lays the strip out for good.
#[test]
fn a_focus_step_glides_and_settles_into_the_rest_layout() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    runtime.backend.animate_strip = true;
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    assert!(
        glides(&mut runtime).is_empty(),
        "turning the strip on doesn't glide"
    );
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert!(
        glides(&mut runtime).is_empty(),
        "a column in view doesn't move the camera"
    );

    let before = windows
        .iter()
        .map(|window| runtime.backend.frames[window])
        .collect::<Vec<_>>();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();

    let motion = glides(&mut runtime).pop().expect("a glide");
    assert_eq!(motion.workspace, WorkspaceId::ONE);
    assert_eq!(motion.camera.target(), 500.0);
    assert!(runtime.strip_in_motion(WorkspaceId::ONE));
    // Nothing moves while the glide runs (Safari is only sized for it), but Ghostty's frame
    // is already on disk.
    for (window, frame) in windows.iter().zip(&before) {
        let now = runtime.backend.frames[window];
        assert_eq!((now.x, now.y), (frame.x, frame.y));
    }
    assert_eq!(runtime.backend.frames[&windows[2]].width, 500);
    assert_eq!(runtime.backend.persisted[&windows[0]], column(0, 500));
    let riders: Vec<_> = motion.columns.iter().map(|c| c.window.clone()).collect();
    assert_eq!(
        riders,
        windows[..3].to_vec(),
        "Linear stays parked past the seam"
    );
    // A cycle mid-glide leaves the windows to the main thread.
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert_eq!(runtime.backend.frames[&windows[1]], before[1]);
    assert_eq!(runtime.backend.frames[&windows[0]], before[0]);

    let deadline = runtime.strip_views.next_deadline().expect("a deadline");
    assert!(deadline - Instant::now() < Duration::from_secs(1));
    runtime.settle_strip_glides(deadline);

    assert!(!runtime.strip_in_motion(WorkspaceId::ONE));
    assert!(runtime.strip_views.next_deadline().is_none());
    assert_strip(
        &runtime,
        &windows,
        &[
            peek_left(500),
            At::Shown(column(0, 500)),
            At::Shown(column(500, 500)),
            At::Parked,
        ],
    );
}

/// A cycle mid-glide can catch a wide column crossing the screen's edge centered off screen in
/// the Accessibility snapshot and the on-screen list read after it, while the window-server
/// snapshot read just before still had it centered on screen (seen live: Safari at 80% lost its
/// slot gliding back to the first column). The glide vouches for every column the window server
/// lists, so none loses its slot.
#[test]
fn a_cycle_mid_glide_keeps_a_column_caught_off_screen() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    runtime.backend.animate_strip = true;
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert!(runtime.strip_in_motion(WorkspaceId::ONE));
    let safari = &windows[2];
    runtime
        .backend
        .frames
        .insert(safari.clone(), column(2100, 500));
    runtime
        .backend
        .server_frames
        .insert(safari.clone(), column(700, 500));
    runtime.backend.ordered_out.insert(safari.window_id);
    // In view before the glide and partly in view after it, so nothing recorded its frame.
    runtime.parked.remove(safari);

    for _ in 0..3 {
        runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    }

    assert_eq!(
        runtime.workspaces.workspace_of(safari),
        Some(WorkspaceId::ONE)
    );
    assert_eq!(
        runtime.workspaces.engine(WorkspaceId::ONE).bound_windows(),
        windows
    );
}

/// Taps faster than the glide bend the one spring toward the newest target.
#[test]
fn rapid_taps_retarget_one_glide() {
    let (mut runtime, _) = four_windows(LAB_ON);
    runtime.backend.animate_strip = true;
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    for _ in 0..3 {
        press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    }
    // The second step starts the glide and the third bends it; only the newest is queued.
    let motions = glides(&mut runtime);
    assert_eq!(motions.len(), 1);
    assert_eq!(motions[0].camera.target(), 1000.0);
    let position = motions[0].camera.position(Instant::now());
    assert!(position < 500.0, "still on its way: {position}");
    assert!(runtime.strip_in_motion(WorkspaceId::ONE));
}

/// Switching workspaces mid-glide stops the glide before the windows are parked.
#[test]
fn switching_workspaces_halts_a_running_glide() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    runtime.backend.animate_strip = true;
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    glides(&mut runtime);

    press(
        &mut runtime,
        WorkerAction::SwitchWorkspace(WorkspaceId::TWO),
    )
    .unwrap();

    assert!(runtime.strip_views.outbox.iter().any(|command| matches!(
        command,
        crate::app::strip::StripCommand::Halt(WorkspaceId::ONE)
    )));
    assert!(!runtime.strip_in_motion(WorkspaceId::ONE));
    assert_strip(&runtime, &windows, &[At::Parked; 4]);
}

/// Plays the main thread's part: every window of the queued glide lands where it ends.
fn present(runtime: &mut WorkspaceRuntime<FakeBackend>) -> crate::strip::StripMotion {
    let motion = glides(runtime).pop().expect("a glide");
    for (window, (x, y)) in motion.final_positions() {
        let frame = runtime.backend.frames.get_mut(&window).unwrap();
        frame.x = x;
        frame.y = y;
    }
    motion
}

fn settle(runtime: &mut WorkspaceRuntime<FakeBackend>) {
    let deadline = runtime
        .strip_views
        .next_deadline()
        .expect("a glide to settle");
    runtime.settle_strip_glides(deadline);
}

fn focus_calls(runtime: &WorkspaceRuntime<FakeBackend>) -> Vec<crate::PhysicalWindowId> {
    runtime
        .backend
        .calls
        .iter()
        .filter_map(|call| match call {
            BackendCall::Focus(window) => Some(window.clone()),
            _ => None,
        })
        .collect()
}

/// Seen live on a desk with a second display to the right, every window one app's (Brave):
/// gliding back to the first column, the third waited at the seam under the second, the glide
/// settled with it still on screen, and heal took it for the app un-parking itself. The second
/// time it exempted the window from parking, leaving it over the right half of the display for
/// good. The settling layout parks it now, and heal leaves strip columns to the strip.
#[test]
fn gliding_across_a_shared_edge_and_back_never_exempts_a_column_from_parking() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    runtime.backend.animate_strip = true;
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    let start = [
        At::Shown(column(0, 500)),
        At::Shown(column(500, 500)),
        At::Parked,
        At::Parked,
    ];
    let scrolled = [
        peek_left(500),
        At::Shown(column(0, 500)),
        At::Shown(column(500, 500)),
        At::Parked,
    ];
    for round in 0..3 {
        // ⌥L: the third column comes in across the seam (camera 0 -> 500).
        press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
        let motion = present(&mut runtime);
        assert_eq!(motion.camera.target(), 500.0, "round {round}");
        settle(&mut runtime);
        assert_strip(&runtime, &windows, &scrolled);
        assert_eq!(runtime.backend.focused, Some(windows[2].clone()));

        // ⌥H twice: back to the first column, the third leaving across the seam (500 -> 0).
        press(&mut runtime, WorkerAction::Focus(Direction::Left)).unwrap();
        press(&mut runtime, WorkerAction::Focus(Direction::Left)).unwrap();
        let motion = present(&mut runtime);
        assert_eq!(motion.camera.target(), 0.0, "round {round}");
        assert_eq!(
            runtime.backend.frames[&windows[2]].x, 500,
            "held at the seam until the glide ends"
        );
        settle(&mut runtime);
        assert_strip(&runtime, &windows, &start);

        // Long after, with the app frontmost: nothing un-parks or exempts it.
        for _ in 0..3 {
            runtime.recently_parked.clear();
            runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
        }
        assert!(runtime.parking_exempt.is_empty(), "round {round}");
        assert!(runtime.recent_heals.is_empty(), "round {round}");
        assert_strip(&runtime, &windows, &start);
        press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    }
}

/// The column arriving across the seam takes focus as the key lands, so typing reaches it at
/// once, and glides in on top: the neighbor sliding off it is never raised over it, and nothing
/// focuses again when the glide settles.
#[test]
fn a_column_arriving_across_a_shared_edge_takes_focus_at_once_on_top() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    runtime.backend.animate_strip = true;
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    // The arriving column is at the back, where the old curtain raised its neighbor over it.
    runtime.backend.stacking = Some(
        [1, 0, 3, 2]
            .map(|index: usize| windows[index].window_id)
            .to_vec(),
    );
    runtime.backend.calls.clear();

    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();

    assert!(runtime.strip_in_motion(WorkspaceId::ONE));
    assert_eq!(
        focus_calls(&runtime),
        [windows[2].clone()],
        "focused mid-glide"
    );
    assert_eq!(runtime.backend.focused, Some(windows[2].clone()));
    assert!(
        !runtime
            .backend
            .calls
            .contains(&BackendCall::Raise(windows[1].clone())),
        "{:?}",
        runtime.backend.calls
    );
    runtime.cycle(outputs(), &AtomicBool::new(false)).unwrap();
    assert!(runtime.strip_in_motion(WorkspaceId::ONE));

    present(&mut runtime);
    settle(&mut runtime);
    assert_eq!(focus_calls(&runtime), [windows[2].clone()]);
    assert_eq!(runtime.backend.focused, Some(windows[2].clone()));
}

/// A tap faster than the glide steps on from the column it aimed at, not the one Accessibility
/// still reports, and a focus the app refuses mid-glide is retried when the glide settles
/// instead of failing the key.
#[test]
fn rapid_taps_step_from_the_aimed_column_and_retry_a_refused_one() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    runtime.backend.animate_strip = true;
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    assert_eq!(runtime.current_focus(), Some(windows[3].clone()));

    // ⌥H back onto a column the glide doesn't tuck, while the app refuses focus.
    runtime.backend.focus_outcome = Some(false);
    press(&mut runtime, WorkerAction::Focus(Direction::Left)).unwrap();
    runtime.backend.focus_outcome = None;
    present(&mut runtime);
    settle(&mut runtime);
    assert_eq!(runtime.backend.focused, Some(windows[2].clone()));
}

/// A click elsewhere while a refused focus waits for the glide to settle wins: the glide
/// doesn't take focus back when it settles.
#[test]
fn a_deferred_focus_yields_to_focus_moved_elsewhere_mid_glide() {
    let (mut runtime, windows) = four_windows(LAB_ON);
    runtime.backend.animate_strip = true;
    press(&mut runtime, WorkerAction::ToggleStrip).unwrap();
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    runtime.backend.focus_outcome = Some(false);
    press(&mut runtime, WorkerAction::Focus(Direction::Right)).unwrap();
    runtime.backend.focus_outcome = None;
    assert!(runtime.strip_in_motion(WorkspaceId::ONE));
    assert_ne!(runtime.backend.focused, Some(windows[2].clone()));
    let elsewhere = window(99);
    runtime.backend.focused = Some(elsewhere.clone());
    runtime.backend.calls.clear();

    present(&mut runtime);
    settle(&mut runtime);

    assert!(focus_calls(&runtime).is_empty());
    assert_eq!(runtime.backend.focused, Some(elsewhere));
}
