//! A window's outline: a line just inside each visible window, so the eye pairs every card with
//! the window it controls. Quiet on every window, amber on the one with keyboard focus, bright
//! amber with a faint tint on the one under the pointer, dashed on a float. Where a float
//! covers a window, that window's outline stops, as the window itself does. It never takes the
//! pointer: a click on it reaches the window below.
//!
//! The line is a light stroke with a thinner dark one hugging its outer side, so one of the two
//! reads whatever the window shows: the dark on a white page, the light on a black one.

use rune::{Color, El, LineCap, Shape, SpringPair, Transition, el, oklch};
use spaceman::{Rect, app::UiWindowOverlayWindow};

use super::{Wiring, look};

/// How far inside its window the outline sits: clear of the window's own corners whatever
/// their radius, and of a neighbour's outline when tiles touch.
pub const INSET: f32 = 3.0;
const RADIUS: f32 = 12.0;
/// The dark stroke along the outline's outer side, then the line itself inside it.
const EDGE: f32 = 1.0;
const LINE: f32 = 1.5;
/// A float's dashes: on, off.
const DASH: (f32, f32) = (9.0, 6.0);
/// Points per corner arc.
const CORNER_STEPS: usize = 6;
/// A resting outline, and the focused window's, as opacity of the whole line.
const REST: f32 = 0.5;
const FOCUSED: f32 = 0.9;

type Point = [f32; 2];

/// A rounded rectangle `inset` inside a box `w` by `h`, as a closed run of points clockwise
/// from the top-right corner. Two insets give concentric curves, point for point.
fn rounded(w: f32, h: f32, inset: f32) -> Vec<Point> {
    let r = RADIUS.min(w / 2.0).min(h / 2.0) - inset;
    let (left, top, right, bottom) = (inset, inset, w - inset, h - inset);
    let corners = [
        ([right - r, top + r], 270.0),
        ([right - r, bottom - r], 0.0),
        ([left + r, bottom - r], 90.0),
        ([left + r, top + r], 180.0),
    ];
    corners
        .into_iter()
        .flat_map(|(centre, from): (Point, f32)| {
            (0..=CORNER_STEPS).map(move |step| {
                let angle = (from + 90.0 * step as f32 / CORNER_STEPS as f32).to_radians();
                [centre[0] + r * angle.cos(), centre[1] + r * angle.sin()]
            })
        })
        .collect()
}

/// The stretch of the segment `a` to `b` inside `cover` (x, y, right, bottom), as the
/// fractions along it where it enters and leaves.
fn inside(a: Point, b: Point, cover: [f32; 4]) -> Option<(f32, f32)> {
    let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
    for axis in 0..2 {
        let (from, delta) = (a[axis], b[axis] - a[axis]);
        let (low, high) = (cover[axis], cover[axis + 2]);
        if delta.abs() < f32::EPSILON {
            if from <= low || from >= high {
                return None;
            }
        } else {
            let (first, second) = ((low - from) / delta, (high - from) / delta);
            enter = enter.max(first.min(second));
            leave = leave.min(first.max(second));
        }
    }
    (enter < leave).then_some((enter, leave))
}

/// The parts of the closed outline `points` that no rectangle in `covers` hides, each an open
/// run of points. With nothing hidden, the whole outline as one run that ends where it began.
fn showing(points: &[Point], covers: &[[f32; 4]]) -> Vec<Vec<Point>> {
    // The ends exactly, so a run that comes back to its start closes on the same point.
    let at = |a: Point, b: Point, t: f32| match t {
        t if t <= 0.0 => a,
        t if t >= 1.0 => b,
        t => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t],
    };
    let mut runs: Vec<Vec<Point>> = Vec::new();
    // Whether the run being built is still open at the end of the last segment walked.
    let mut open = false;
    for index in 0..points.len() {
        let (a, b) = (points[index], points[(index + 1) % points.len()]);
        let mut hidden: Vec<(f32, f32)> = covers
            .iter()
            .filter_map(|cover| inside(a, b, *cover))
            .collect();
        hidden.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut from = 0.0;
        let mut shown: Vec<(f32, f32)> = Vec::new();
        for (enter, leave) in hidden {
            if enter > from {
                shown.push((from, enter));
            }
            from = from.max(leave);
        }
        if from < 1.0 {
            shown.push((from, 1.0));
        }
        let mut reaches_end = false;
        for (start, end) in shown {
            match runs.last_mut() {
                Some(run) if open && start == 0.0 => run.push(at(a, b, end)),
                _ => runs.push(vec![at(a, b, start), at(a, b, end)]),
            }
            reaches_end = end == 1.0;
        }
        open = reaches_end;
    }
    // The walk began mid-run: the last run carries on into the first.
    if runs.len() > 1
        && open
        && runs.first().and_then(|run| run.first()) == points.first()
        && let Some(mut last) = runs.pop()
    {
        last.extend(runs[0].iter().skip(1));
        runs[0] = last;
    }
    runs
}

fn length(run: &[Point]) -> f32 {
    run.windows(2)
        .map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]))
        .sum()
}

/// `run` cut into dashes: `dash` points on, `gap` off, the period stretched a little so a
/// whole number fits. An open run ends on a dash's end; one that closes on its start ends on
/// a gap, so the pattern meets itself. The number of dashes is the one a run `like` points
/// long would get, so a line and the slightly longer edge around it break in step.
fn dashes(run: &[Point], like: f32, (dash, gap): (f32, f32)) -> Vec<Vec<Point>> {
    let total = length(run);
    let closed = run.len() > 2 && run.first() == run.last();
    let pattern = if closed {
        (like / (dash + gap)).round().max(1.0) * (dash + gap)
    } else {
        let count = ((like + gap) / (dash + gap)).round().max(1.0);
        count * dash + (count - 1.0) * gap
    };
    let scale = total / pattern;
    let (dash, period) = (dash * scale, (dash + gap) * scale);
    let mut out: Vec<Vec<Point>> = Vec::new();
    let mut walked = 0.0;
    for pair in run.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let span = (b[0] - a[0]).hypot(b[1] - a[1]);
        if span <= 0.0 {
            continue;
        }
        let at = |distance: f32| {
            let t = ((distance - walked) / span).clamp(0.0, 1.0);
            [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
        };
        // Every dash that touches this segment.
        let mut index = (walked / period).floor();
        while index * period < walked + span {
            let (start, end) = (index * period, index * period + dash);
            if end > walked {
                let begins_here = start >= walked;
                let (from, to) = (at(start.max(walked)), at(end.min(walked + span)));
                match out.last_mut() {
                    Some(last) if !begins_here => last.push(to),
                    _ => out.push(vec![from, to]),
                }
            }
            index += 1.0;
        }
        walked += span;
    }
    out
}

/// The outline's strokes for a window `w` by `h`: the dark edge outside, the line inside it.
fn strokes(w: f32, h: f32, covers: &[[f32; 4]], dashed: bool, ink: Color) -> Shape {
    let mut line = showing(&rounded(w, h, EDGE + LINE / 2.0), covers);
    let mut edge = showing(&rounded(w, h, EDGE / 2.0), covers);
    if dashed {
        let lengths: Vec<f32> = line.iter().map(|run| length(run)).collect();
        // A float in front may cut the two into different runs; then each dashes for itself.
        let in_step = lengths.len() == edge.len();
        edge = edge
            .iter()
            .zip(lengths.iter().chain(std::iter::repeat(&0.0)))
            .flat_map(|(run, like)| {
                let like = if in_step { *like } else { length(run) };
                dashes(run, like, DASH)
            })
            .collect();
        line = line
            .iter()
            .flat_map(|run| dashes(run, length(run), DASH))
            .collect();
    }
    let mut shape = Shape::new().view_box(w, h);
    for run in edge {
        shape = shape.stroke_with(run, false, EDGE, LineCap::Round, oklch(0.16, 0.01, 262.0));
    }
    for run in line {
        shape = shape.stroke_with(run, false, LINE, LineCap::Round, ink);
    }
    shape
}

/// `window`'s outline on a display whose bounds are `screen` (global points). `covers`: the
/// windows in front of it. `hot`: the pointer is on the window or its card.
pub fn outline(
    window: &UiWindowOverlayWindow,
    screen: Rect,
    covers: &[Rect],
    hot: bool,
    wiring: Wiring,
) -> El {
    let frame = window.frame;
    let (x, y) = (frame.x as f32 + INSET, frame.y as f32 + INSET);
    let w = (frame.width as f32 - 2.0 * INSET).max(2.0 * RADIUS);
    let h = (frame.height as f32 - 2.0 * INSET).max(2.0 * RADIUS);
    // The windows in front, in this outline's own points.
    let covers: Vec<[f32; 4]> = covers
        .iter()
        .map(|cover| {
            let (left, top) = (cover.x as f32 - x, cover.y as f32 - y);
            [
                left,
                top,
                left + cover.width as f32,
                top + cover.height as f32,
            ]
        })
        .collect();
    let (ink, strength) = if window.focused {
        (look::accent(), FOCUSED)
    } else {
        (look::text(), REST)
    };
    let layer = |key: &'static str, ink: Color| {
        el().key(key)
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
            .shape(strokes(w, h, &covers, window.floating, ink))
    };
    let outline = el()
        .key(format!(
            "frame-{}-{}",
            window.window.app.pid, window.window.window_id
        ))
        .absolute(x - screen.x as f32, y - screen.y as f32)
        .size(w, h)
        .motion(look::TRAVEL)
        .transition(Transition::opacity().springs(SpringPair::new(look::ENTER, look::EXIT)))
        .a11y_hidden()
        .child(layer("line", ink).opacity(strength))
        .child(
            // The pointer's window: the same line in full amber over a faint tint, faded in.
            layer("hot", look::accent())
                .radius(RADIUS)
                .fill(look::accent_a(0.05))
                .opacity(if hot { 1.0 } else { 0.0 })
                .opacity_motion(look::HOVER),
        );
    wiring.frame(outline, &window.window)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain square outline, clockwise from its top-left corner.
    const SQUARE: [Point; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

    /// The run's ends, to a hundredth of a point.
    fn ends(run: &[Point]) -> [[i32; 2]; 2] {
        let hundredths = |point: &Point| point.map(|value| (value * 100.0).round() as i32);
        [
            run.first().map_or([0; 2], hundredths),
            run.last().map_or([0; 2], hundredths),
        ]
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn an_uncovered_outline_is_one_run_that_closes() {
        let runs = showing(&SQUARE, &[]);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].first(), runs[0].last());
        assert_eq!(length(&runs[0]), 400.0);
        // A window in front that misses the outline changes nothing.
        assert_eq!(showing(&SQUARE, &[[20.0, 20.0, 80.0, 80.0]]), runs);
        // The real outline closes too, and stays inside its box.
        let rounded = rounded(300.0, 200.0, EDGE / 2.0);
        let run = &showing(&rounded, &[])[0];
        assert_eq!(run.first(), run.last());
        assert!(
            run.iter()
                .all(|[x, y]| { (0.0..=300.0).contains(x) && (0.0..=200.0).contains(y) })
        );
    }

    #[test]
    fn a_window_in_front_cuts_the_outline_where_it_covers_it() {
        // A float over the right edge's middle, as wide as the edge and beyond.
        let runs = showing(&SQUARE, &[[60.0, 30.0, 140.0, 70.0]]);
        assert_eq!(
            runs.len(),
            1,
            "one run, from below the float round to above it"
        );
        assert_eq!(ends(&runs[0]), [[10000, 7000], [10000, 3000]]);
        assert!(near(length(&runs[0]), 400.0 - 40.0));

        // Over a corner: the cut takes part of two edges.
        let runs = showing(&SQUARE, &[[70.0, -10.0, 130.0, 40.0]]);
        assert_eq!(runs.len(), 1);
        assert_eq!(ends(&runs[0]), [[10000, 4000], [7000, 0]]);

        // Two floats on one edge leave the piece between them as its own run.
        let covers = [[-10.0, 10.0, 10.0, 30.0], [-10.0, 60.0, 10.0, 80.0]];
        let runs = showing(&SQUARE, &covers);
        assert_eq!(runs.len(), 2);
        let between = runs.iter().find(|run| near(length(run), 30.0)).unwrap();
        assert_eq!(ends(between), [[0, 6000], [0, 3000]]);

        // Covered whole: nothing to draw.
        assert!(showing(&SQUARE, &[[-5.0, -5.0, 105.0, 105.0]]).is_empty());
    }

    #[test]
    fn dashes_fill_a_run_evenly_and_bend_round_its_corners() {
        // 100 points, 9 on and 6 off: seven dashes, stretched to end on a dash.
        let line = [[0.0, 0.0], [100.0, 0.0]];
        let cut = dashes(&line, 100.0, DASH);
        assert_eq!(cut.len(), 7);
        assert_eq!(cut[0][0], [0.0, 0.0]);
        let end = cut[6].last().unwrap();
        assert!((end[0] - 100.0).abs() < 0.01, "{end:?}");
        let on = length(&cut[0]);
        assert!(cut.iter().all(|dash| (length(dash) - on).abs() < 0.01));
        assert!((on - 9.0).abs() < 0.5, "{on}");

        // A dash that reaches a corner carries on round it as one piece.
        let corner = [[0.0, 0.0], [5.0, 0.0], [5.0, 20.0]];
        let cut = dashes(&corner, 25.0, (10.0, 5.0));
        assert_eq!(cut[0].len(), 3);
        assert_eq!(ends(&cut[0]), [[0, 0], [500, 500]]);
        assert_eq!(cut.len(), 2);
        // Shorter than one dash: the run itself.
        assert_eq!(dashes(&[[0.0, 0.0], [4.0, 0.0]], 4.0, DASH).len(), 1);
        // Round a closed outline the pattern ends on a gap, so no two dashes meet at the seam.
        let mut square = SQUARE.to_vec();
        square.push(SQUARE[0]);
        let cut = dashes(&square, 400.0, (10.0, 10.0));
        assert_eq!(cut.len(), 20);
        assert_eq!(ends(&cut[19])[1], [0, 1000]);
        // An edge a little longer than its line takes the line's count, not its own.
        let line = [[0.0, 0.0], [110.0, 0.0]];
        assert_eq!(dashes(&line, 100.0, DASH).len(), 7);
        assert_eq!(dashes(&line, 110.0, DASH).len(), 8);
    }
}
