use super::*;
use crate::{
    AppInstanceId,
    motion::{SLIDE_CENTER_MARGIN, SharedEdge, SpringParams},
};

const AREA: Rect = Rect {
    x: 0,
    y: 25,
    width: 1200,
    height: 875,
};

fn columns(widths: &[StripWidth]) -> Vec<StripColumn> {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| StripColumn {
            slot: SlotId(index as u64 + 1),
            width: *width,
        })
        .collect()
}

fn free() -> ScrollingDisplayBounds {
    ScrollingDisplayBounds::new(AREA, [])
}

/// A second monitor to the right of `AREA`.
fn seam_right() -> ScrollingDisplayBounds {
    ScrollingDisplayBounds::new(
        AREA,
        [Rect {
            x: AREA.width,
            ..AREA
        }],
    )
}

fn frame(x: i32, width: i32) -> Rect {
    Rect { x, width, ..AREA }
}

fn window(id: u32) -> PhysicalWindowId {
    PhysicalWindowId {
        app: AppInstanceId {
            pid: 1,
            launched_at: 1,
        },
        window_id: id,
    }
}

#[test]
fn presets_step_through_a_third_to_full_and_stop_at_the_ends() {
    assert_eq!(StripWidth::default(), StripWidth::Half);
    let percents: Vec<_> = StripWidth::PRESETS.iter().map(|w| w.percent()).collect();
    assert_eq!(percents, [33, 50, 66, 80, 100]);
    let mut width = StripWidth::Third;
    for expected in &StripWidth::PRESETS[1..] {
        width = width.wider();
        assert_eq!(width, *expected);
    }
    assert_eq!(StripWidth::Full.wider(), StripWidth::Full);
    assert_eq!(StripWidth::Full.narrower(), StripWidth::FourFifths);
    assert_eq!(StripWidth::Third.narrower(), StripWidth::Third);
    assert_eq!(StripWidth::Third.share(1200), 400);
    assert_eq!(StripWidth::TwoThirds.share(1200), 800);
    assert_eq!(StripWidth::FourFifths.share(1200), 960);
}

#[test]
fn the_ribbon_lays_columns_end_to_end() {
    let strip = ribbon(
        &columns(&[StripWidth::Half, StripWidth::TwoThirds, StripWidth::Third]),
        1200,
    );
    let placed: Vec<_> = strip
        .iter()
        .map(|column| (column.x, column.width))
        .collect();
    assert_eq!(placed, [(0, 600), (600, 800), (1400, 400)]);
    assert_eq!(ribbon_width(&strip), 1800);
    assert_eq!(ribbon_width(&[]), 0);
}

#[test]
fn minimal_scroll_only_moves_when_the_column_leaves_the_view() {
    let strip = ribbon(&columns(&[StripWidth::Half; 4]), 1200);
    let total = ribbon_width(&strip);
    // Already in view: the camera stays put.
    assert_eq!(minimal_scroll(0.0, 1200, strip[1], total), 0.0);
    // Past the right edge: just far enough to show its right edge.
    assert_eq!(minimal_scroll(0.0, 1200, strip[2], total), 600.0);
    assert_eq!(minimal_scroll(0.0, 1200, strip[3], total), 1200.0);
    // Past the left edge: its left edge.
    assert_eq!(minimal_scroll(1200.0, 1200, strip[1], total), 600.0);
    // Mid-glide the same rule applies from wherever the camera is aimed.
    assert_eq!(minimal_scroll(300.0, 1200, strip[2], total), 600.0);
    // A column as wide as the view aligns its left edge.
    let wide = ribbon(&columns(&[StripWidth::Half, StripWidth::Full]), 1200);
    assert_eq!(minimal_scroll(0.0, 1200, wide[1], 1800), 600.0);
    // The camera never leaves the ribbon.
    assert_eq!(clamp_camera(-50.0, 1200, total), 0.0);
    assert_eq!(clamp_camera(5000.0, 1200, total), 1200.0);
    assert_eq!(clamp_camera(10.0, 1200, 600), 0.0);
}

#[test]
fn a_restored_anchor_puts_its_column_at_the_left_edge() {
    let strip = ribbon(&columns(&[StripWidth::Half; 4]), 1200);
    assert_eq!(anchor_camera(&strip, Some(SlotId(2)), 1200), 600.0);
    assert_eq!(anchor_camera(&strip, Some(SlotId(4)), 1200), 1200.0);
    assert_eq!(anchor_camera(&strip, Some(SlotId(99)), 1200), 0.0);
    assert_eq!(anchor_camera(&strip, None, 1200), 0.0);
}

#[test]
fn off_screen_columns_peek_sixteen_points_past_free_edges() {
    let strip = ribbon(&columns(&[StripWidth::Half; 5]), 1200);
    let layout = rest_layout(AREA, &free(), &strip, 1200.0);
    assert_eq!(
        layout.columns,
        vec![
            (SlotId(1), ColumnRest::Peeking(frame(-600 + PEEK_PX, 600))),
            (SlotId(2), ColumnRest::Peeking(frame(-600 + PEEK_PX, 600))),
            (SlotId(3), ColumnRest::Visible(frame(0, 600))),
            (SlotId(4), ColumnRest::Visible(frame(600, 600))),
            (SlotId(5), ColumnRest::Peeking(frame(1200 - PEEK_PX, 600))),
        ]
    );
    assert_eq!(layout.anchor, Some(SlotId(3)));
    assert_eq!(
        layout.hidden().collect::<Vec<_>>(),
        [SlotId(1), SlotId(2), SlotId(5)]
    );
}

#[test]
fn a_partly_visible_column_runs_off_a_free_edge() {
    let strip = ribbon(&columns(&[StripWidth::TwoThirds; 3]), 1200);
    let layout = rest_layout(AREA, &free(), &strip, 400.0);
    assert_eq!(
        layout.visible().collect::<Vec<_>>(),
        vec![(SlotId(1), frame(-400, 800)), (SlotId(2), frame(400, 800))]
    );
    assert_eq!(layout.anchor, Some(SlotId(1)));
}

#[test]
fn columns_never_cross_an_edge_shared_with_another_display() {
    let strip = ribbon(&columns(&[StripWidth::TwoThirds; 3]), 1200);
    let layout = rest_layout(AREA, &seam_right(), &strip, 0.0);
    assert_eq!(
        layout.columns,
        vec![
            (SlotId(1), ColumnRest::Visible(frame(0, 800))),
            // Partly in view: kept inside the display rather than shown on the next one.
            (SlotId(2), ColumnRest::Visible(frame(400, 800))),
            (SlotId(3), ColumnRest::Parked),
        ]
    );
    // The free left edge still peeks.
    let layout = rest_layout(AREA, &seam_right(), &strip, 1200.0);
    assert_eq!(
        layout.rest(SlotId(1)),
        Some(ColumnRest::Peeking(frame(-800 + PEEK_PX, 800)))
    );
}

fn glide(from: f64, to: f64, start: Instant, bounds: ScrollingDisplayBounds) -> StripMotion {
    let strip = ribbon(&columns(&[StripWidth::Half; 4]), 1200);
    let mut camera = CameraSpring::at_rest(SpringParams::default(), from, start);
    camera.retarget(to, start);
    StripMotion {
        workspace: WorkspaceId::ONE,
        display_id: "main".into(),
        output: AREA,
        bounds,
        camera,
        issued: start,
        columns: strip
            .iter()
            .map(|column| MotionColumn {
                window: window(column.slot.0 as u32),
                x: column.x,
                width: column.width,
                shift: None,
            })
            .collect(),
    }
}

fn x_of(positions: &[(PhysicalWindowId, (i32, i32))], id: u32) -> i32 {
    positions
        .iter()
        .find(|(window, _)| window.window_id == id)
        .map(|(_, (x, _))| *x)
        .unwrap()
}

#[test]
fn a_glide_slides_through_many_intermediate_frames_at_120_hz() {
    let start = Instant::now();
    let motion = glide(0.0, 600.0, start, free());
    let frame = Duration::from_micros(8_333);
    let settled = motion.settles_at(start);
    let frames = (settled - start).as_micros() / frame.as_micros();
    assert!((20..90).contains(&frames), "{frames} frames");
    let mut previous = x_of(&motion.positions(start), 3);
    assert_eq!(previous, 1200 - PEEK_PX);
    let mut distinct = 0;
    for index in 1..=frames as u32 {
        let x = x_of(&motion.positions(start + frame * index), 3);
        assert!(x <= previous, "column 3 moves only left");
        distinct += usize::from(x != previous);
        previous = x;
    }
    assert!(distinct >= 15, "{distinct} distinct positions");
    assert_eq!(x_of(&motion.final_positions(), 3), 600);
    assert_eq!(x_of(&motion.final_positions(), 1), -600 + PEEK_PX);
    assert!(motion.is_settled(settled));
    assert!(!motion.is_settled(start + frame));
}

#[test]
fn retargeting_mid_glide_keeps_position_and_velocity() {
    let start = Instant::now();
    let mut motion = glide(0.0, 600.0, start, free());
    let mid = start + Duration::from_millis(60);
    let before = motion.camera.sample(mid);
    motion.camera.retarget(1200.0, mid);
    let after = motion.camera.sample(mid);
    assert!((before.position - after.position).abs() < 1e-9);
    assert!((before.velocity - after.velocity).abs() < 1e-9);
    assert!(before.position > 0.0 && before.position < 600.0);
    assert_eq!(motion.camera.target(), 1200.0);
}

#[test]
fn nothing_slides_onto_the_neighbouring_display() {
    let start = Instant::now();
    let motion = glide(1200.0, 0.0, start, seam_right());
    let mut at = start;
    while at < motion.settles_at(start) {
        for (_, (x, _)) in motion.positions(at) {
            assert!(x + 600 <= AREA.width, "x {x} crosses the seam");
        }
        at += Duration::from_millis(4);
    }
}

#[test]
fn a_shifted_column_springs_back_to_its_place() {
    let start = Instant::now();
    let mut motion = glide(0.0, 0.0, start, free());
    let mut shift = CameraSpring::at_rest(SpringParams::default(), 200.0, start);
    shift.retarget(0.0, start);
    motion.columns[1].shift = Some(shift);
    assert_eq!(x_of(&motion.positions(start), 2), 800);
    assert!(!motion.is_settled(start + Duration::from_millis(30)));
    let settled = motion.settles_at(start);
    // Within a point of its place once settled; the final frame snaps it there.
    assert!((x_of(&motion.positions(settled), 2) - 600).abs() <= 1);
    assert_eq!(x_of(&motion.final_positions(), 2), 600);
}

#[test]
fn unsized_columns_follow_the_last_width_picked() {
    let mut state = StripState::default();
    assert_eq!(state.width(SlotId(1)), StripWidth::Half);
    state.set_width(SlotId(2), StripWidth::Full);
    assert_eq!(state.width(SlotId(2)), StripWidth::Full);
    assert_eq!(state.width(SlotId(3)), StripWidth::Full);
    state.set_width(SlotId(1), StripWidth::Third);
    assert_eq!(state.width(SlotId(2)), StripWidth::Full);
    assert_eq!(state.width(SlotId(3)), StripWidth::Third);
    state.anchor = Some(SlotId(2));
    state.retain_slots(|slot| slot != SlotId(2));
    assert_eq!(state.widths, vec![(SlotId(1), StripWidth::Third)]);
    assert_eq!(state.anchor, None);
}

#[test]
fn state_serializes_only_what_differs_from_the_default() {
    assert_eq!(
        serde_json::to_value(StripState::default()).unwrap(),
        serde_json::json!({})
    );
    let mut state = StripState {
        enabled: true,
        anchor: Some(SlotId(3)),
        ..StripState::default()
    };
    state.set_width(SlotId(3), StripWidth::FourFifths);
    let encoded = serde_json::to_value(&state).unwrap();
    assert_eq!(
        encoded,
        serde_json::json!({
            "enabled": true,
            "default_width": "four_fifths",
            "widths": [[3, "four_fifths"]],
            "anchor": 3
        })
    );
    assert_eq!(
        serde_json::from_value::<StripState>(encoded).unwrap(),
        state
    );
}

fn ids(windows: &[PhysicalWindowId]) -> Vec<u32> {
    windows.iter().map(|window| window.window_id).collect()
}

/// Two monitors side by side, as on a desk with a second display to the right: the column
/// arriving across the seam waits at its rest under its neighbor, which slides off it like a
/// curtain; the column leaving waits at the seam while its neighbor slides over it.
#[test]
fn a_column_crossing_a_shared_edge_stays_under_the_neighbor_sliding_across_it() {
    let start = Instant::now();
    let arriving = glide(0.0, 600.0, start, seam_right()).curtain(start, None);
    assert_eq!(ids(&arriving.tucked), [3, 4]);
    assert_eq!(
        ids(&arriving.over),
        [2],
        "the first column never reaches the seam"
    );

    let leaving = glide(600.0, 0.0, start, seam_right()).curtain(start, None);
    assert_eq!(ids(&leaving.tucked), [3, 4]);
    assert_eq!(ids(&leaving.over), [2]);
}

#[test]
fn free_edges_tuck_nothing() {
    let start = Instant::now();
    for (from, to) in [(0.0, 600.0), (600.0, 0.0)] {
        assert_eq!(
            glide(from, to, start, free()).curtain(start, None),
            Curtain::default()
        );
    }
}

/// Past a free edge an arriving column starts at its sliver and moves on every frame of the
/// way in, never jumping ahead to its rest.
#[test]
fn a_column_arriving_past_a_free_edge_slides_in_from_its_sliver() {
    let start = Instant::now();
    let motion = glide(0.0, 600.0, start, free());
    assert_eq!(x_of(&motion.positions(start), 3), AREA.width - PEEK_PX);
    let mut last = AREA.width - PEEK_PX;
    let mut moving_frames = 0;
    for frame in 1..=60 {
        let x = x_of(
            &motion.positions(start + Duration::from_micros(8_333 * frame)),
            3,
        );
        assert!(x <= last + 1, "frame {frame}: {x} after {last}");
        if x < last {
            moving_frames += 1;
        }
        last = x;
    }
    assert_eq!(last, 600);
    // A 180 ms glide spans about 21 frames at 120 Hz: it moves on nearly every one.
    assert!(moving_frames >= 18, "moved on {moving_frames} frames");
}

/// `seam_right` with columns sliding past the seam as far as their center allows.
fn slide_right() -> ScrollingDisplayBounds {
    seam_right().with_shared_edge(SharedEdge::Slide)
}

/// In Slide, a column arriving across the seam starts half past it and slides in on every
/// frame of its second half, while its center never leaves this display.
#[test]
fn sliding_past_a_shared_edge_keeps_each_column_centered_on_its_own_display() {
    let start = Instant::now();
    let motion = glide(0.0, 600.0, start, slide_right());
    let farthest = AREA.width - 300 - SLIDE_CENTER_MARGIN;
    assert_eq!(x_of(&motion.positions(start), 3), farthest);
    let mut at = start;
    let mut seen = std::collections::BTreeSet::new();
    while at < motion.settles_at(start) {
        for (window, (x, _)) in motion.positions(at) {
            assert!(
                x + 300 < AREA.width,
                "{window:?} at {x}: its center crosses the seam"
            );
            if window.window_id == 3 {
                seen.insert(x);
            }
        }
        at += Duration::from_millis(4);
    }
    assert!(seen.len() > 20, "column 3 took {} positions", seen.len());
    assert_eq!(x_of(&motion.final_positions(), 3), 600);
}

/// Slide still keeps the columns that rest out of view parked rather than half on screen, and
/// the neighbor still slides over the one waiting at the seam.
#[test]
fn sliding_parks_at_rest_and_still_curtains_the_rest_of_the_way() {
    let strip = ribbon(&columns(&[StripWidth::TwoThirds; 3]), 1200);
    let layout = rest_layout(AREA, &slide_right(), &strip, 0.0);
    assert_eq!(
        layout.columns,
        vec![
            (SlotId(1), ColumnRest::Visible(frame(0, 800))),
            // Partly in view: it overhangs the seam by a little under half its width.
            (
                SlotId(2),
                ColumnRest::Visible(frame(400 + 400 - SLIDE_CENTER_MARGIN, 800))
            ),
            (SlotId(3), ColumnRest::Parked),
        ]
    );
    let start = Instant::now();
    let arriving = glide(0.0, 600.0, start, slide_right()).curtain(start, None);
    assert_eq!(ids(&arriving.tucked), [3, 4]);
    assert_eq!(ids(&arriving.over), [2]);
    assert_eq!(
        glide(600.0, 0.0, start, slide_right())
            .curtain(start, None)
            .tucked
            .len(),
        2
    );
}

/// The column taking focus is never tucked or covered: focus raises it as the key lands and it
/// glides in on top, while the leaving column it crosses stays curtained under the rest.
#[test]
fn the_column_taking_focus_glides_in_on_top() {
    let start = Instant::now();
    for bounds in [seam_right(), slide_right()] {
        let motion = glide(0.0, 600.0, start, bounds);
        let curtain = motion.curtain(start, Some(&window(3)));
        assert_eq!(ids(&curtain.tucked), [4]);
        assert_eq!(
            ids(&curtain.over),
            [2],
            "nothing is raised for the focused column"
        );
    }
}
