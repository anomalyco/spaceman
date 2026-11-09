//! Frame-by-frame checks of the HUD's motion: the overlay's own view and flow, stepped at
//! 120 Hz on a virtual clock and read back as presented rects, opacity and scale.
//!
//! `cargo test -p spaceman-agent hud::motion_tests::trace -- --ignored --nocapture` prints the
//! capsule's pose for every frame of a show, a burst of switches and a hide.

use rune::{
    Env, Runtime,
    motion_gate::{Run, Snap},
};
use spaceman::{
    Axis, Rect, WorkspaceId,
    app::{UiHudEvent, UiHudKind, UiHudLayout},
};

use super::{HOLD_MS, HudOverlay, HudTick, LEAVE_MS, OVERLAY_SIZE, overlay_view, pill};

const HZ: f64 = 120.0;
/// The most a spring here moves opacity in one 120 Hz frame; a pop is several times this.
const OPACITY_STEP: f32 = 0.15;
/// One 8-bit step of alpha: below it nothing is drawn.
const UNSEEN: f32 = 1.0 / 255.0;

struct Hud {
    run: Run<HudOverlay>,
    frame: u64,
}

impl Hud {
    /// Up and at rest on `event`, as the panel's first frame draws it.
    fn showing(event: UiHudEvent) -> Self {
        Self::start(event, false)
    }

    fn start(event: UiHudEvent, reduce_motion: bool) -> Self {
        let mut overlay = HudOverlay::new(true);
        overlay.reduce_motion = reduce_motion;
        overlay.flow.trigger(event, 0);
        let rt = Runtime::new(overlay, overlay_view, OVERLAY_SIZE, Env::new(true, 2.0));
        let mut run = Run::new(rt, HZ);
        // The labels roll, so a count changing in place would be a swap.
        run.rec.counts_roll = true;
        run.step();
        Self { run, frame: 0 }
    }

    fn now_ms(&self) -> u64 {
        (self.frame as f64 * 1_000.0 / HZ).round() as u64
    }

    /// One frame: the flow's deadlines first, as the overlay's poll runs them.
    fn step(&mut self) -> HudTick {
        self.frame += 1;
        let now_ms = self.now_ms();
        let mut tick = HudTick::Idle;
        self.run.rt.update_if(|overlay, _| {
            tick = overlay.flow.tick(now_ms);
            tick == HudTick::Leave
        });
        self.run.step();
        tick
    }

    fn play(&mut self, ms: u64) {
        let until = self.now_ms() + ms;
        while self.now_ms() < until {
            self.step();
        }
    }

    /// Plays until the flow reports `tick`, and returns the recording's frame it landed on.
    fn play_until(&mut self, tick: HudTick) -> usize {
        for _ in 0..(HZ as u64 * 5) {
            if self.step() == tick {
                return self.run.rec.frames - 1;
            }
        }
        panic!("the flow never reached {tick:?}");
    }

    /// A switch landing on the next frame; returns that frame.
    fn switch(&mut self, event: UiHudEvent) -> usize {
        let now_ms = self.now_ms();
        self.run.rec.input("switch");
        self.run.rt.update(move |overlay, _| {
            overlay.flow.trigger(event, now_ms);
        });
        self.step();
        self.run.rec.frames - 1
    }

    /// The presented state of the element whose path ends in `path`, from frame `from` on.
    fn snaps(&self, path: &str, from: usize) -> Vec<Snap> {
        let suffix = format!("/{path}");
        let track = self
            .run
            .rec
            .tracks
            .iter()
            .find(|track| track.path == path || track.path.ends_with(&suffix))
            .unwrap_or_else(|| panic!("nothing drawn at {path}"));
        track
            .frames
            .iter()
            .filter(|(frame, _)| *frame >= from)
            .map(|(_, snap)| *snap)
            .collect()
    }

    /// How far below its resting place the glyph that `label` gained on frame `at` started.
    fn rolled_in_from(&self, label: &str, at: usize) -> f32 {
        let inside = format!("/{label}/");
        let glyph = self
            .run
            .rec
            .tracks
            .iter()
            .filter(|track| track.path.contains(&inside))
            .find(|track| track.frames.first().is_some_and(|(frame, _)| *frame == at))
            .unwrap_or_else(|| panic!("no glyph of {label} entered on frame {at}"));
        let y = |index: usize| glyph.frames[index].1.rect[1];
        y(0) - y(glyph.frames.len() - 1)
    }

    fn findings(&self) -> Vec<String> {
        self.run.finish().iter().map(ToString::to_string).collect()
    }
}

fn split(columns: i32) -> UiHudLayout {
    let width = 1500 / columns.max(1);
    UiHudLayout::Split(
        (0..columns)
            .map(|column| Rect {
                x: column * width,
                y: 0,
                width,
                height: 1100,
            })
            .collect(),
    )
}

fn workspace(number: u8, windows: i32) -> UiHudEvent {
    UiHudEvent {
        workspace: WorkspaceId::new(number).unwrap(),
        kind: UiHudKind::Workspace,
        preview: split(windows),
        windows: windows as usize,
    }
}

/// Every step is small and in one direction; `rising` says which.
fn assert_one_way(values: &[f32], rising: bool, step: f32, what: &str) {
    for pair in values.windows(2) {
        let delta = if rising {
            pair[1] - pair[0]
        } else {
            pair[0] - pair[1]
        };
        assert!(
            (-0.001..=step).contains(&delta),
            "{what}: {} then {} in one frame",
            pair[0],
            pair[1]
        );
    }
}

fn opacity(snaps: &[Snap]) -> Vec<f32> {
    snaps.iter().map(|snap| snap.op).collect()
}

#[test]
fn a_burst_of_switches_a_hide_and_a_show_pass_the_motion_gate() {
    let mut hud = Hud::showing(workspace(1, 3));
    hud.play(300);
    // 1, 2, 3, then back to 1: each lands while the last is still moving.
    hud.switch(workspace(2, 1));
    hud.play(160);
    hud.switch(workspace(3, 2));
    hud.play(160);
    hud.switch(workspace(1, 3));
    hud.play_until(HudTick::Gone);
    hud.play(100);
    hud.switch(workspace(4, 0));
    hud.play(700);
    // Pops, jumps, swapped text, cut fades, overshoot, flicker, motion that never settles.
    assert_eq!(hud.findings(), Vec::<String>::new());
}

#[test]
fn the_capsule_leaves_and_comes_back_along_one_path() {
    let mut hud = Hud::showing(workspace(1, 2));
    let rest = hud.snaps("hud-slot", 0)[0];
    assert_eq!(
        (rest.op, rest.scale),
        (1.0, 1.0),
        "the first frame is at rest"
    );

    let left = hud.play_until(HudTick::Leave);
    let gone = hud.play_until(HudTick::Gone);
    let exit = hud.snaps("hud-slot", left - 1);
    let exit = &exit[..=gone - left];
    assert_one_way(&opacity(exit), false, OPACITY_STEP, "exit opacity");
    assert_one_way(
        &exit.iter().map(|snap| snap.rect[1]).collect::<Vec<_>>(),
        true,
        1.0,
        "exit drop",
    );
    let last = exit[exit.len() - 1];
    assert!(
        last.op < UNSEEN,
        "ordering the panel out after {LEAVE_MS} ms cuts a capsule still at {}",
        last.op
    );
    assert!(
        (last.scale - 0.96).abs() < 0.001,
        "it shrank to {}",
        last.scale
    );

    // Hidden, then another workspace: it enters from the pose it left in, content at rest.
    hud.play(400);
    let shown = hud.switch(workspace(3, 1));
    hud.play(LEAVE_MS + 200);
    let enter = hud.snaps("hud-slot", shown);
    assert!(enter[0].op < 0.1, "it popped in at {}", enter[0].op);
    assert_one_way(&opacity(&enter), true, OPACITY_STEP, "enter opacity");
    assert_one_way(
        &enter.iter().map(|snap| snap.rect[1]).collect::<Vec<_>>(),
        false,
        1.0,
        "enter rise",
    );
    let settled = enter[enter.len() - 1];
    assert_eq!(
        (settled.op, settled.scale, settled.rect),
        (1.0, 1.0, rest.rect)
    );
    let in_200_ms = enter[(0.2 * HZ) as usize];
    assert!(in_200_ms.op > 0.97, "{} after 200 ms", in_200_ms.op);

    // Nothing inside rolled or resized on the way in: the capsule is its final width, scaled.
    let capsule = hud.snaps("hud-pill", shown);
    let width = capsule[capsule.len() - 1].rect[2];
    for snap in &capsule {
        assert!(
            (snap.rect[2] / snap.scale - width).abs() < 0.1,
            "the capsule was {} wide at scale {} on its way to {width}",
            snap.rect[2],
            snap.scale
        );
    }
    assert_eq!(hud.findings(), Vec::<String>::new());
}

#[test]
fn a_switch_during_the_exit_turns_the_capsule_around_where_it_is() {
    let mut hud = Hud::showing(workspace(1, 2));
    let left = hud.play_until(HudTick::Leave);
    hud.play(50);
    let before = *hud.snaps("hud-slot", left).last().unwrap();
    assert!(
        (0.2..0.8).contains(&before.op),
        "mid-exit, at {}",
        before.op
    );

    let back = hud.switch(workspace(2, 2));
    hud.play(400);
    let after = hud.snaps("hud-slot", back);
    // It carries on from where it was: no restart from nothing, no snap to full.
    assert!((after[0].op - before.op).abs() < OPACITY_STEP);
    assert!((after[0].scale - before.scale).abs() < 0.01);
    assert!((after[0].rect[1] - before.rect[1]).abs() < 1.0);
    let lowest = opacity(&after).into_iter().fold(1.0, f32::min);
    assert!(lowest > before.op - OPACITY_STEP, "it dipped to {lowest}");
    for pair in after.windows(2) {
        assert!((pair[1].op - pair[0].op).abs() <= OPACITY_STEP);
    }
    assert_eq!(after[after.len() - 1].op, 1.0);
    // The gate reads a capsule that dims and comes back as a flicker; here a switch asked for it.
    let findings = hud.findings();
    assert!(
        findings
            .iter()
            .all(|finding| finding.starts_with("flicker")),
        "{findings:?}"
    );
}

#[test]
fn tiles_slide_and_labels_roll_the_way_of_travel() {
    let mut hud = Hud::showing(workspace(2, 2));
    hud.play(300);
    let screen = hud.snaps("mini-screen", 0)[0].rect;

    // Forward: the next workspace's tiles come in from the right as the old ones leave left,
    // and its number rolls up from below.
    let forward = hud.switch(workspace(3, 2));
    hud.play(60);
    let entering = hud.snaps("ws3", forward);
    assert!(entering[0].rect[0] > screen[0] + 5.0, "from the right");
    assert!(entering[0].op < 0.1);
    let leaving = hud.snaps("ws2", forward);
    assert!(
        leaving[leaving.len() - 1].rect[0] < screen[0] - 1.0,
        "out to the left"
    );
    hud.play(500);
    assert!(
        hud.rolled_in_from("hud-workspace", forward) > 5.0,
        "up from below"
    );
    let settled = hud.snaps("ws3", forward);
    assert_one_way(
        &settled.iter().map(|snap| snap.rect[0]).collect::<Vec<_>>(),
        false,
        2.0,
        "tiles sliding in",
    );
    assert_eq!(settled[settled.len() - 1].rect[0], screen[0]);

    // Back: the other way.
    let back = hud.switch(workspace(1, 2));
    hud.play(60);
    assert!(
        hud.snaps("ws1", back)[0].rect[0] < screen[0] - 5.0,
        "from the left"
    );
    hud.play(500);
    assert!(
        hud.rolled_in_from("hud-workspace", back) < -5.0,
        "down from above"
    );
    assert_eq!(hud.findings(), Vec::<String>::new());
}

#[test]
fn another_kind_of_message_replaces_the_whole_phrase() {
    let moved = UiHudEvent {
        kind: UiHudKind::Moved,
        ..workspace(4, 2)
    };
    let mut hud = Hud::showing(moved);
    hud.play(300);
    let moved_x = hud.snaps("hud-moved", 0)[0].rect[0];

    // "Moved to Workspace 4" to "Workspace 4": the old phrase fades out as the new one fades in
    // where it rests, instead of "Workspace 4" sliding left across the letters rolling out.
    let switched = hud.switch(workspace(4, 2));
    hud.play(600);
    let leaving = hud.snaps("hud-moved", switched);
    assert!(
        leaving.len() < (0.4 * HZ) as usize,
        "the old phrase is gone"
    );
    assert!(leaving[leaving.len() - 1].op < 0.05);
    let arriving = hud.snaps("hud-workspace", switched);
    assert!(arriving[0].op < 0.1);
    let rest = arriving[arriving.len() - 1].rect;
    assert!(
        (arriving[0].rect[1] - rest[1]).abs() < 1.0,
        "in place, not rolled"
    );
    assert!(
        (rest[0] - moved_x).abs() > 5.0,
        "the capsule narrowed around it"
    );
    // The count is the same phrase in both, so it never left.
    assert_eq!(hud.snaps("hud-count", 0).len(), hud.run.rec.frames);

    hud.switch(UiHudEvent {
        kind: UiHudKind::Fullscreen { enabled: true },
        preview: UiHudLayout::Fullscreen,
        ..workspace(4, 2)
    });
    hud.play(600);
    assert_eq!(hud.findings(), Vec::<String>::new());
}

#[test]
fn a_layout_change_on_the_same_workspace_moves_the_tiles_it_has() {
    let mut hud = Hud::showing(workspace(1, 2));
    hud.play(300);
    let before = hud.snaps("ws1/tile1", 0)[0].rect;

    let changed = hud.switch(UiHudEvent {
        kind: UiHudKind::Layout {
            axis: Axis::Vertical,
        },
        preview: UiHudLayout::Split(vec![
            Rect {
                x: 0,
                y: 0,
                width: 1500,
                height: 550,
            },
            Rect {
                x: 0,
                y: 550,
                width: 1500,
                height: 550,
            },
        ]),
        ..workspace(1, 2)
    });
    hud.play(500);
    // The same tile, from where it was to where it goes, with no slide of the whole set.
    let tile = hud.snaps("ws1/tile1", changed - 1);
    assert_eq!(tile[0].rect, before);
    for pair in tile.windows(2) {
        for edge in 0..4 {
            assert!((pair[1].rect[edge] - pair[0].rect[edge]).abs() < 2.0);
        }
        assert_eq!(pair[1].op, 1.0);
    }
    let after = tile[tile.len() - 1].rect;
    assert!(after[0] < before[0] - 5.0 && after[1] > before[1] + 5.0);
    assert_eq!(hud.findings(), Vec::<String>::new());
}

#[test]
fn reduce_motion_fades_and_nothing_travels() {
    let mut hud = Hud::start(workspace(2, 2), true);
    hud.play(300);
    let rest = hud.snaps("hud-slot", 0)[0];
    let screen = hud.snaps("mini-screen", 0)[0].rect;

    let switched = hud.switch(workspace(3, 2));
    hud.play(400);
    for snap in hud.snaps("ws3", switched) {
        assert_eq!(snap.rect[0], screen[0], "tiles fade in place");
    }
    assert_eq!(hud.rolled_in_from("hud-workspace", switched), 0.0);

    let left = hud.play_until(HudTick::Leave);
    let gone = hud.play_until(HudTick::Gone);
    let exit = hud.snaps("hud-slot", left);
    let exit = &exit[..=gone - left];
    for snap in exit {
        assert_eq!(
            (snap.scale, snap.rect),
            (1.0, rest.rect),
            "no scale, no drop"
        );
    }
    assert!(
        exit.iter().any(|snap| (0.2..0.8).contains(&snap.op)),
        "a fade, not a cut"
    );
    assert_one_way(&opacity(exit), false, OPACITY_STEP, "reduced exit");
    assert!(exit[exit.len() - 1].op < UNSEEN);
    hud.play(600);
    assert_eq!(hud.findings(), Vec::<String>::new());
}

#[test]
fn the_widest_capsule_and_its_shadow_fit_the_panel() {
    let ten = WorkspaceId::new(10).unwrap();
    let widest = [
        UiHudEvent {
            workspace: ten,
            kind: UiHudKind::Moved,
            preview: split(3),
            windows: 99,
        },
        UiHudEvent {
            workspace: ten,
            kind: UiHudKind::Workspace,
            preview: UiHudLayout::Fullscreen,
            windows: 99,
        },
    ];
    for event in widest {
        let hud = Hud::showing(event);
        let [x, y, width, height] = hud.snaps("hud-pill", 0)[0].rect;
        // The shadow's widest layer blurs 32 pt; its lowest reaches `SHADOW_REACH` below.
        assert!(
            x >= 32.0 && x + width <= OVERLAY_SIZE.0 - 32.0,
            "{x} + {width}"
        );
        assert!(y + height + pill::SHADOW_REACH <= OVERLAY_SIZE.1);
    }
}

#[test]
#[ignore = "prints the capsule's pose per frame for tuning; asserts nothing"]
fn trace() {
    let mut hud = Hud::showing(workspace(1, 3));
    hud.play(100);
    hud.switch(workspace(2, 1));
    hud.play(160);
    hud.switch(workspace(3, 2));
    hud.play_until(HudTick::Gone);
    hud.switch(workspace(1, 3));
    hud.play(300);
    println!("frame,ms,opacity,scale,y,capsule_x,capsule_width");
    let (slot, capsule) = (hud.snaps("hud-slot", 0), hud.snaps("hud-pill", 0));
    for (frame, (slot, capsule)) in slot.iter().zip(&capsule).enumerate() {
        println!(
            "{frame},{:.0},{:.3},{:.3},{:.2},{:.2},{:.2}",
            frame as f64 * 1_000.0 / HZ,
            slot.op,
            slot.scale,
            slot.rect[1],
            capsule.rect[0],
            capsule.rect[2]
        );
    }
    println!("hold {HOLD_MS} ms, leave {LEAVE_MS} ms");
}
