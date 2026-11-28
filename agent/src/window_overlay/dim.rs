//! The dim: a dark wash over the one window the overlay acts on, out to that window's own
//! corners, so its card stands clear of whatever the window shows. Nothing is outlined, and no
//! other window is touched. Where a float covers the window the wash stops, round that float's
//! corners, as the window itself does. It never takes the pointer: a click on it reaches the
//! window below.
//!
//! Every window also gets a `sense`: its frame, drawn as nothing, where the pointer is heard.

use std::f32::consts::FRAC_PI_2;

use rune::{El, Shape, SpringPair, Transition, el};
use spaceman::{Rect, app::UiWindowOverlayWindow};

use super::{Wiring, look};

/// A window's corner, as macOS rounds it.
pub const RADIUS: f32 = 12.0;
/// The most windows in front whose shape is cut out of the wash.
const MAX_COVERS: usize = 8;
/// Points per corner arc of a window in front.
const CORNER_STEPS: usize = 6;

/// A rectangle as its left, top, right and bottom.
type Edges = [f32; 4];
type Point = [f32; 2];

fn key(part: &str, window: &UiWindowOverlayWindow) -> String {
    format!(
        "{part}-{}-{}",
        window.window.app.pid, window.window.window_id
    )
}

/// Whether `cover` hides any of a window `w` by `h`.
fn reaches(cover: &Edges, w: f32, h: f32) -> bool {
    cover[0] < w && cover[1] < h && cover[2] > 0.0 && cover[3] > 0.0
}

/// The parts of a window `w` by `h` that no rectangle in `covers` hides: the grid the covers'
/// edges rule, row by row, each row's showing cells joined into runs. No two overlap.
fn showing(w: f32, h: f32, covers: &[Edges]) -> Vec<Edges> {
    let cuts = |axis: usize, length: f32| {
        let mut at: Vec<f32> = covers
            .iter()
            .filter(|cover| reaches(cover, w, h))
            .flat_map(|cover| [cover[axis], cover[axis + 2]])
            .map(|edge| edge.clamp(0.0, length))
            .chain([0.0, length])
            .collect();
        at.sort_by(f32::total_cmp);
        at.dedup();
        at
    };
    let (columns, rows) = (cuts(0, w), cuts(1, h));
    let hidden = |x: f32, y: f32| {
        covers
            .iter()
            .any(|cover| x > cover[0] && x < cover[2] && y > cover[1] && y < cover[3])
    };
    let mut parts = Vec::new();
    for row in rows.windows(2) {
        let mut run: Option<Edges> = None;
        for column in columns.windows(2) {
            if hidden((column[0] + column[1]) / 2.0, (row[0] + row[1]) / 2.0) {
                parts.extend(run.take());
            } else {
                let left = run.map_or(column[0], |run| run[0]);
                run = Some([left, row[0], column[1], row[1]]);
            }
        }
        parts.extend(run);
    }
    parts
}

/// What a window in front leaves showing at its own rounded corners: for each corner of each
/// rectangle in `covers` that lies inside the window `w` by `h` and under no other cover, the
/// corner's square less its quarter disc, as a polygon from the corner round the arc.
fn slivers(w: f32, h: f32, covers: &[Edges]) -> Vec<Vec<Point>> {
    let mut out = Vec::new();
    for (index, cover) in covers.iter().enumerate() {
        let radius = RADIUS
            .min((cover[2] - cover[0]) / 2.0)
            .min((cover[3] - cover[1]) / 2.0);
        // Each corner, and which way the cover lies from it.
        let corners = [
            (cover[0], cover[1], 1.0, 1.0),
            (cover[2], cover[1], -1.0, 1.0),
            (cover[2], cover[3], -1.0, -1.0),
            (cover[0], cover[3], 1.0, -1.0),
        ];
        for (x, y, dx, dy) in corners {
            let (far_x, far_y): (f32, f32) = (x + dx * radius, y + dy * radius);
            let square = [x.min(far_x), y.min(far_y), x.max(far_x), y.max(far_y)];
            let inside = square[0] >= 0.0 && square[1] >= 0.0 && square[2] <= w && square[3] <= h;
            let clear = covers.iter().enumerate().all(|(other, cover)| {
                other == index
                    || square[2] <= cover[0]
                    || square[0] >= cover[2]
                    || square[3] <= cover[1]
                    || square[1] >= cover[3]
            });
            if !inside || !clear {
                continue;
            }
            // From the arc's end on the cover's horizontal edge round to its vertical one.
            let arc = (0..=CORNER_STEPS).map(|step| {
                let angle = FRAC_PI_2 * step as f32 / CORNER_STEPS as f32;
                [
                    far_x - dx * radius * angle.sin(),
                    far_y - dy * radius * angle.cos(),
                ]
            });
            out.push(std::iter::once([x, y]).chain(arc).collect());
        }
    }
    out
}

/// `window`'s wash on a display whose bounds are `screen` (global points). `fronts`: the
/// windows in front of it.
pub fn dim(window: &UiWindowOverlayWindow, screen: Rect, fronts: &[Rect]) -> El {
    let frame = window.frame;
    let (w, h) = (frame.width as f32, frame.height as f32);
    // The windows in front that reach this one, in its own points.
    let covers: Vec<Edges> = fronts
        .iter()
        .map(|front| {
            let (left, top) = ((front.x - frame.x) as f32, (front.y - frame.y) as f32);
            [
                left,
                top,
                left + front.width as f32,
                top + front.height as f32,
            ]
        })
        .filter(|cover| reaches(cover, w, h))
        .take(MAX_COVERS)
        .collect();
    let wash = el()
        .key(key("dim", window))
        .absolute((frame.x - screen.x) as f32, (frame.y - screen.y) as f32)
        .size(w, h)
        // One layer, so the pieces of a cut wash darken the window once where they meet.
        .layer_opacity(look::DIM)
        .motion(look::TRAVEL)
        .transition(Transition::opacity().springs(SpringPair::new(look::ENTER, look::EXIT)))
        .a11y_hidden();
    if covers.is_empty() {
        return wash.radius(RADIUS).fill(look::obsidian());
    }
    let parts =
        showing(w, h, &covers)
            .into_iter()
            .zip(0..)
            .map(|([left, top, right, bottom], index)| {
                el().key(format!("part-{index}"))
                    .absolute(left, top)
                    .size(right - left, bottom - top)
                    .fill(look::obsidian())
            });
    let corners = slivers(w, h, &covers)
        .into_iter()
        .fold(Shape::new(), |shape, sliver| {
            shape.fill(sliver, look::obsidian())
        });
    wash.clip_rounded(RADIUS).children(parts).child(
        el().key("corners")
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
            .shape(corners),
    )
}

/// Where the pointer is heard for `window`: its frame, drawn as nothing.
pub fn sense(window: &UiWindowOverlayWindow, screen: Rect, wiring: Wiring) -> El {
    let frame = window.frame;
    let area = el()
        .key(key("sense", window))
        .absolute((frame.x - screen.x) as f32, (frame.y - screen.y) as f32)
        .size(frame.width as f32, frame.height as f32)
        .a11y_hidden();
    wiring.sense(area, &window.window)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(parts: &[Edges]) -> f32 {
        parts
            .iter()
            .map(|[left, top, right, bottom]| (right - left) * (bottom - top))
            .sum()
    }

    /// No two parts share any area.
    fn assert_apart(parts: &[Edges]) {
        for (index, a) in parts.iter().enumerate() {
            for b in &parts[index + 1..] {
                let apart = a[2] <= b[0] || b[2] <= a[0] || a[3] <= b[1] || b[3] <= a[1];
                assert!(apart, "{a:?} and {b:?} overlap");
            }
        }
    }

    #[test]
    fn an_uncovered_window_is_washed_whole() {
        assert_eq!(showing(800.0, 600.0, &[]), [[0.0, 0.0, 800.0, 600.0]]);
        // A window in front that misses it changes nothing.
        let beside = [[900.0, 0.0, 1200.0, 300.0]];
        assert_eq!(showing(800.0, 600.0, &beside), [[0.0, 0.0, 800.0, 600.0]]);
    }

    #[test]
    fn a_window_in_front_is_cut_out_of_the_wash() {
        // A float over the middle: the rows above and below it whole, and a run either side.
        let parts = showing(800.0, 600.0, &[[300.0, 200.0, 500.0, 400.0]]);
        assert_eq!(
            parts,
            [
                [0.0, 0.0, 800.0, 200.0],
                [0.0, 200.0, 300.0, 400.0],
                [500.0, 200.0, 800.0, 400.0],
                [0.0, 400.0, 800.0, 600.0],
            ]
        );
        // A float over the top-right corner, hanging off both edges.
        let parts = showing(800.0, 600.0, &[[600.0, -50.0, 900.0, 150.0]]);
        assert_eq!(
            parts,
            [[0.0, 0.0, 600.0, 150.0], [0.0, 150.0, 800.0, 600.0]]
        );
        // Two floats that overlap each other are cut out once.
        let covers = [[100.0, 100.0, 400.0, 300.0], [300.0, 200.0, 600.0, 500.0]];
        let parts = showing(800.0, 600.0, &covers);
        assert_apart(&parts);
        let hidden = 300.0 * 200.0 + 300.0 * 300.0 - 100.0 * 100.0;
        assert_eq!(area(&parts), 800.0 * 600.0 - hidden);
        // Covered whole: nothing to wash.
        assert!(showing(800.0, 600.0, &[[-5.0, -5.0, 805.0, 605.0]]).is_empty());
    }

    #[test]
    fn the_wash_follows_a_float_round_its_corners() {
        // A float in the middle: a sliver at each of its four corners, each starting at the
        // corner and ending on the float's side a radius away.
        let cover = [300.0, 200.0, 500.0, 400.0];
        let found = slivers(800.0, 600.0, &[cover]);
        assert_eq!(found.len(), 4);
        assert_eq!(found[0][0], [300.0, 200.0]);
        assert_eq!(found[0][1], [300.0 + RADIUS, 200.0]);
        let last = found[0].last().unwrap();
        assert!((last[0] - 300.0).abs() < 0.01 && (last[1] - (200.0 + RADIUS)).abs() < 0.01);
        // Every point of a sliver stays in its corner's square, outside the float's curve.
        for sliver in &found {
            let [x, y] = sliver[0];
            assert!(
                sliver
                    .iter()
                    .all(|point| (point[0] - x).abs() <= RADIUS + 0.01
                        && (point[1] - y).abs() <= RADIUS + 0.01)
            );
        }
        // A corner off the window, or under another float, has none.
        let hanging = [600.0, -50.0, 900.0, 150.0];
        let found = slivers(800.0, 600.0, &[hanging]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0][0], [600.0, 150.0]);
        let over = [450.0, 350.0, 700.0, 500.0];
        assert_eq!(slivers(800.0, 600.0, &[cover, over]).len(), 3 + 3);
    }
}
