use std::time::{Duration, Instant};

use super::*;

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn first_settled(spring: &CameraSpring, start: Instant, tolerance: f64) -> Duration {
    (0..2_000)
        .map(ms)
        .find(|elapsed| spring.is_settled(start + *elapsed, tolerance))
        .expect("spring settles within two seconds")
}

#[test]
fn default_spring_is_critically_damped_and_settles_a_column_move_in_180_ms() {
    let params = SpringParams::default();
    assert_eq!(params.damping_ratio, 1.0);
    assert_eq!(params, SpringParams::for_duration(180, 1.0));
    let start = Instant::now();
    for (duration, expected) in [(120, ms(120)), (180, ms(180)), (400, ms(400))] {
        let mut spring =
            CameraSpring::at_rest(SpringParams::for_duration(duration, 1.0), 0.0, start);
        spring.retarget(CALIBRATION_MOVE_PT, start);
        let settled = first_settled(&spring, start, SETTLE_TOLERANCE_PT);
        assert!(
            settled.abs_diff(expected) <= ms(1),
            "{duration} ms spring settled after {settled:?}"
        );
    }
}

#[test]
fn critically_damped_spring_matches_the_closed_form() {
    let start = Instant::now();
    let (x0, v0, target) = (-640.0, 1_200.0, 900.0);
    let spring = CameraSpring::with_state(SpringParams::default(), target + x0, v0, target, start);
    let w = SpringParams::default().angular_frequency();
    for elapsed in [0, 8, 16, 50, 120, 300] {
        let t = ms(elapsed).as_secs_f64();
        let decay = (-w * t).exp();
        let expected_x = target + decay * (x0 + (w * x0 + v0) * t);
        let expected_v = decay * (v0 - w * (w * x0 + v0) * t);
        let sample = spring.sample(start + ms(elapsed));
        assert!(
            (sample.position - expected_x).abs() < 1e-9,
            "x at {elapsed} ms"
        );
        assert!(
            (sample.velocity - expected_v).abs() < 1e-9,
            "v at {elapsed} ms"
        );
    }
}

#[test]
fn velocity_is_the_derivative_of_position_in_every_damping_regime() {
    let start = Instant::now();
    for damping_ratio in [0.5, 1.0, 1.8] {
        let params = SpringParams {
            damping_ratio,
            ..SpringParams::default()
        };
        let spring = CameraSpring::with_state(params, 0.0, -300.0, 1_000.0, start);
        assert!((spring.position(start) - 0.0).abs() < 1e-9);
        assert!((spring.velocity(start) + 300.0).abs() < 1e-9);
        for elapsed in [5, 40, 90, 200] {
            let now = start + ms(elapsed);
            let step = Duration::from_micros(10);
            let numeric = (spring.position(now + step) - spring.position(now - step))
                / (2.0 * step.as_secs_f64());
            let analytic = spring.velocity(now);
            assert!(
                (numeric - analytic).abs() < 1e-3 * analytic.abs().max(1.0),
                "ζ={damping_ratio} at {elapsed} ms: numeric {numeric} vs analytic {analytic}"
            );
        }
    }
}

#[test]
fn a_short_move_settles_sooner_without_overshoot() {
    let start = Instant::now();
    let mut spring = CameraSpring::at_rest(SpringParams::default(), 0.0, start);
    spring.retarget(100.0, start);
    let settled = first_settled(&spring, start, SETTLE_TOLERANCE_PT);
    assert!(
        settled > ms(100) && settled < ms(180),
        "settled after {settled:?}"
    );
    let max = (0..400)
        .map(|elapsed| spring.position(start + ms(elapsed)))
        .fold(f64::MIN, f64::max);
    assert!(max <= 100.0 + 1e-9, "overshot to {max}");
}

#[test]
fn retarget_keeps_position_and_velocity_continuous() {
    let start = Instant::now();
    let mut spring = CameraSpring::at_rest(SpringParams::default(), 0.0, start);
    spring.retarget(800.0, start);
    let mid = start + ms(60);
    let before = spring.sample(mid);
    spring.retarget(1_600.0, mid);
    let after = spring.sample(mid);
    assert!((before.position - after.position).abs() < 1e-9);
    assert!((before.velocity - after.velocity).abs() < 1e-9);
    assert!(after.velocity > 0.0);
}

#[test]
fn rapid_retargets_glide_in_one_accelerating_arc() {
    let start = Instant::now();
    let mut spring = CameraSpring::at_rest(SpringParams::default(), 0.0, start);
    let mut previous = 0.0;
    for tap in 1..=4_u32 {
        let now = start + ms(u64::from(tap - 1) * 60);
        spring.retarget(f64::from(tap) * 500.0, now);
        for frame in 0..60 {
            let position = spring.position(now + ms(frame));
            assert!(position >= previous - 1e-9, "moved backwards at tap {tap}");
            previous = position;
        }
    }
    let end = start + ms(180);
    let settled = first_settled(&spring, end, SETTLE_TOLERANCE_PT);
    assert!((spring.position(end + settled) - 2_000.0).abs() <= SETTLE_TOLERANCE_PT);
}

#[test]
fn a_fast_pass_through_the_target_is_not_settled() {
    let start = Instant::now();
    let spring = CameraSpring::with_state(SpringParams::default(), 500.0, 4_000.0, 500.0, start);
    assert!(!spring.is_settled(start, SETTLE_TOLERANCE_PT));
    let resting = CameraSpring::at_rest(SpringParams::default(), 500.0, start);
    assert!(resting.is_settled(start, SETTLE_TOLERANCE_PT));
}

#[test]
fn sampling_before_the_spring_was_aimed_reads_its_start() {
    let start = Instant::now() + ms(100);
    let spring = CameraSpring::with_state(SpringParams::default(), 10.0, 0.0, 20.0, start);
    assert!((spring.position(start - ms(50)) - 10.0).abs() < 1e-9);
}

#[test]
fn history_measures_steady_velocity() {
    let start = Instant::now();
    let mut history = MotionHistory::new();
    for frame in 0..30 {
        history.push(8.0, start + ms(frame * 8));
    }
    assert!((history.position() - 240.0).abs() < 1e-9);
    assert!((history.velocity() - 1_000.0).abs() < 80.0);
}

#[test]
fn history_forgets_samples_older_than_the_window() {
    let start = Instant::now();
    let mut history = MotionHistory::new();
    for frame in 0..20 {
        history.push(40.0, start + ms(frame * 10));
    }
    for frame in 20..60 {
        history.push(2.0, start + ms(frame * 10));
    }
    assert!(history.samples.len() <= 16);
    assert!((history.velocity() - 200.0).abs() < 30.0);
}

#[test]
fn resting_zero_deltas_collapse_into_one_anchor_and_decay_velocity() {
    let start = Instant::now();
    let mut history = MotionHistory::new();
    for frame in 0..10 {
        history.push(10.0, start + ms(frame * 10));
    }
    for frame in 10..40 {
        history.push(0.0, start + ms(frame * 10));
    }
    let zeros = history
        .samples
        .iter()
        .filter(|sample| sample.delta == 0.0)
        .count();
    assert_eq!(zeros, 1);
    assert_eq!(history.velocity(), 0.0);
    assert_eq!(history.projected_end(), history.position());
}

#[test]
fn projection_coasts_by_velocity_over_deceleration() {
    let projected = project_end(100.0, 1_000.0);
    let expected = 100.0 + 1_000.0 / (1000.0 * -(0.997_f64.ln()));
    assert!((projected - expected).abs() < 1e-9);
    assert!(projected > 400.0 && projected < 450.0);
    assert!(project_end(100.0, -1_000.0) < 100.0);
}

#[test]
fn reset_clears_position_and_velocity() {
    let start = Instant::now();
    let mut history = MotionHistory::new();
    history.push(5.0, start);
    history.push(5.0, start + ms(10));
    history.reset();
    assert_eq!(history.position(), 0.0);
    assert_eq!(history.velocity(), 0.0);
}

fn side_by_side() -> [(&'static str, Rect); 2] {
    [
        ("left", rect(0, 25, 1512, 957)),
        ("right", rect(1512, 0, 2560, 1415)),
    ]
}

#[test]
fn neighbouring_displays_share_their_seam() {
    let left = ScrollingDisplayBounds::for_display(side_by_side(), "left").unwrap();
    let right = ScrollingDisplayBounds::for_display(side_by_side(), "right").unwrap();
    assert!(!left.adjacent_left && left.adjacent_right);
    assert!(right.adjacent_left && !right.adjacent_right);
}

#[test]
fn a_shared_edge_keeps_columns_inside_the_seam() {
    let left = ScrollingDisplayBounds::for_display(side_by_side(), "left").unwrap();
    assert_eq!(left.clamp_column_x(1_400, 800, 12), 1512 - 800);
    let right = ScrollingDisplayBounds::for_display(side_by_side(), "right").unwrap();
    assert_eq!(right.clamp_column_x(900, 800, 12), 1512);
}

#[test]
fn a_free_edge_lets_columns_slide_off_leaving_a_peek() {
    let left = ScrollingDisplayBounds::for_display(side_by_side(), "left").unwrap();
    assert_eq!(left.clamp_column_x(-2_000, 800, 12), -788);
    assert_eq!(left.clamp_column_x(-300, 800, 12), -300);
    let right = ScrollingDisplayBounds::for_display(side_by_side(), "right").unwrap();
    assert_eq!(right.clamp_column_x(9_000, 800, 24), 1512 + 2560 - 24);
    assert_eq!(right.column_x_range(800, 0).1, 1512 + 2560 - 1);
}

#[test]
fn a_dock_gap_still_counts_as_a_shared_seam() {
    let display = rect(1_600, 0, 1_500, 1_000);
    let bounds = ScrollingDisplayBounds::new(display, [rect(0, 0, 1_520, 1_000)]);
    assert!(bounds.adjacent_left);
}

#[test]
fn displays_above_or_below_leave_both_side_edges_free() {
    let display = rect(0, 0, 1_500, 1_000);
    let bounds = ScrollingDisplayBounds::new(display, [rect(-200, -1_200, 1_900, 1_200)]);
    assert!(!bounds.adjacent_left && !bounds.adjacent_right);
}

#[test]
fn a_column_wider_than_a_display_between_two_seams_pins_to_the_left_edge() {
    let display = rect(1_000, 0, 1_000, 800);
    let bounds = ScrollingDisplayBounds::new(
        display,
        [rect(0, 0, 1_000, 800), rect(2_000, 0, 1_000, 800)],
    );
    assert_eq!(bounds.column_x_range(1_400, 12), (1_000, 1_000));
    assert_eq!(bounds.clamp_column_x(400, 1_400, 12), 1_000);
}

#[test]
fn an_unknown_display_has_no_bounds() {
    assert!(ScrollingDisplayBounds::for_display(side_by_side(), "gone").is_none());
}

/// Kit's desk: two 2560-point Studio Displays side by side.
#[test]
fn sliding_lets_a_column_overhang_a_shared_edge_up_to_its_center() {
    let displays = [
        ("left", rect(0, 0, 2_560, 1_415)),
        ("right", rect(2_560, 0, 2_560, 1_415)),
    ];
    let tuck = ScrollingDisplayBounds::for_display(displays, "left").unwrap();
    assert_eq!(tuck.shared_edge, SharedEdge::Tuck);
    assert_eq!(tuck.clamp_column_x(2_560, 1_280, 16), 2_560 - 1_280);
    let slide = tuck.with_shared_edge(SharedEdge::Slide);
    let farthest = slide.clamp_column_x(2_560, 1_280, 16);
    assert_eq!(farthest, 2_560 - 640 - SLIDE_CENTER_MARGIN);
    assert!(
        farthest + 640 < 2_560,
        "the center stays on the left display"
    );
    assert!(slide.holds_at_shared_edge(2_560, 1_280, 16));
    assert!(!slide.holds_at_shared_edge(farthest, 1_280, 16));
    // The free left edge peeks exactly as before.
    assert_eq!(slide.clamp_column_x(-5_000, 1_280, 16), -1_280 + 16);
    assert!(!slide.holds_at_shared_edge(-5_000, 1_280, 16));
    // The right display overhangs its left seam the same way.
    let right = ScrollingDisplayBounds::for_display(displays, "right")
        .unwrap()
        .with_shared_edge(SharedEdge::Slide);
    assert_eq!(
        right.clamp_column_x(0, 1_280, 16),
        2_560 - 640 + SLIDE_CENTER_MARGIN
    );
    // Too narrow to overhang without its center crossing: it tucks.
    assert_eq!(SharedEdge::Slide.overhang(10), 0);
}

#[test]
fn retuning_a_spring_keeps_its_motion_continuous() {
    let start = Instant::now();
    let spring = CameraSpring::with_state(SpringParams::default(), 0.0, 0.0, 600.0, start);
    let mid = start + ms(60);
    let before = spring.sample(mid);
    let faster = spring.retuned(SpringParams::for_duration(150, 0.78), mid);
    let after = faster.sample(mid);
    assert!((before.position - after.position).abs() < 1e-9);
    assert!((before.velocity - after.velocity).abs() < 1e-9);
    assert_eq!(faster.target(), 600.0);
    assert_eq!(faster.params().damping_ratio, 0.78);
}
