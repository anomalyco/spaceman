use std::collections::HashMap;

use crate::{
    AppInstanceId, AppObservation, Axis, BindingState, Direction, Gaps, NativeObservation,
    ObservedApplication, ObservedWindow, PhysicalWindowId, Rect, ResizeDirection, Settings, Slot,
    SlotId,
};

use super::Engine;

/// A laptop's usable frame: odd extents below the menu bar.
const LAPTOP: Rect = Rect {
    x: 0,
    y: 33,
    width: 1511,
    height: 949,
};

const DESK: Rect = Rect {
    x: 0,
    y: 0,
    width: 1000,
    height: 800,
};

const EIGHT: Gaps = Gaps { inner: 8, outer: 8 };

fn window(id: u32) -> PhysicalWindowId {
    PhysicalWindowId {
        app: AppInstanceId {
            pid: 1,
            launched_at: 1,
        },
        window_id: id,
    }
}

/// `count` tiles side by side, as windows `1..=count`.
fn row(count: u32, gaps: Gaps) -> Engine {
    let mut engine = Engine::default();
    for id in 1..=count {
        engine.insert_slot(Slot {
            id: SlotId(u64::from(id)),
            weight: 1,
            binding: BindingState::Bound(window(id)),
        });
    }
    engine.set_gaps(gaps);
    engine
}

/// `[1 | (2 over (3 | 4)) | 5]`: both axes, nested twice.
fn nested(gaps: Gaps) -> Engine {
    let mut engine = row(5, gaps);
    assert!(engine.move_window_relative(&window(3), &window(2), Direction::Down));
    assert!(engine.move_window_relative(&window(4), &window(3), Direction::Right));
    engine
}

fn tiles(engine: &Engine, output: Rect) -> Vec<Rect> {
    engine
        .placements(&Settings::for_output(output))
        .into_iter()
        .map(|placement| placement.frame)
        .collect()
}

fn span(rect: Rect, axis: Axis) -> (i32, i32) {
    match axis {
        Axis::Horizontal => (rect.x, rect.x + rect.width),
        Axis::Vertical => (rect.y, rect.y + rect.height),
    }
}

/// How far apart two rectangles are along `axis`; negative while they share
/// part of it.
fn separation(a: Rect, b: Rect, axis: Axis) -> i32 {
    let (a, b) = (span(a, axis), span(b, axis));
    (b.0 - a.1).max(a.0 - b.1)
}

/// Every tile lies inside `output` less its outer gap, no two tiles come
/// closer than the inner gap, and each side of each tile is either on the
/// edge of that area or exactly one inner gap from a tile it faces.
fn assert_gapped(tiles: &[Rect], output: Rect, gaps: Gaps) {
    let area = Rect {
        x: output.x + gaps.outer,
        y: output.y + gaps.outer,
        width: output.width - 2 * gaps.outer,
        height: output.height - 2 * gaps.outer,
    };
    for (index, tile) in tiles.iter().enumerate() {
        assert!(tile.width > 0 && tile.height > 0, "{tile:?}");
        for (axis, cross) in [
            (Axis::Horizontal, Axis::Vertical),
            (Axis::Vertical, Axis::Horizontal),
        ] {
            let (start, end) = span(*tile, axis);
            let (area_start, area_end) = span(area, axis);
            let faced = |edge: fn(Rect, Axis) -> i32, at: i32| {
                tiles
                    .iter()
                    .any(|other| separation(*tile, *other, cross) < 0 && edge(*other, axis) == at)
            };
            assert!(
                start == area_start || faced(|other, axis| span(other, axis).1, start - gaps.inner),
                "{tile:?} leading {axis:?} edge in {tiles:?}"
            );
            assert!(
                end == area_end || faced(|other, axis| span(other, axis).0, end + gaps.inner),
                "{tile:?} trailing {axis:?} edge in {tiles:?}"
            );
        }
        for other in &tiles[index + 1..] {
            assert!(
                separation(*tile, *other, Axis::Horizontal) >= gaps.inner
                    || separation(*tile, *other, Axis::Vertical) >= gaps.inner,
                "{tile:?} and {other:?} are closer than the gap"
            );
        }
    }
}

#[test]
fn two_tiles_leave_the_configured_gaps() {
    assert_eq!(
        tiles(&row(2, EIGHT), DESK),
        [
            Rect {
                x: 8,
                y: 8,
                width: 488,
                height: 784,
            },
            Rect {
                x: 504,
                y: 8,
                width: 488,
                height: 784,
            },
        ]
    );
}

#[test]
fn nested_mixed_splits_keep_every_gap_exact_for_odd_and_even_sizes() {
    for output in [DESK, LAPTOP] {
        for (inner, outer) in [(8, 8), (7, 3), (1, 0), (0, 5), (9, 12), (33, 64)] {
            let gaps = Gaps { inner, outer };
            let engine = nested(gaps);
            let tiles = tiles(&engine, output);
            assert_eq!(tiles.len(), 5);
            assert_gapped(&tiles, output, gaps);
        }
    }
}

#[test]
fn without_gaps_tiles_are_exactly_the_layout_frames() {
    let mut engine = nested(Gaps::default());
    assert!(engine.resize_focused_tile(&window(1), ResizeDirection::Grow, 50, LAPTOP));
    let plain: Vec<_> = engine
        .layout
        .placements(LAPTOP)
        .into_iter()
        .map(|(_, frame)| frame)
        .collect();
    assert_eq!(tiles(&engine, LAPTOP), plain);
    assert_eq!(tiles(&Engine::default(), LAPTOP), []);

    // Gaps are not sticky: turning them off restores the same frames.
    engine.set_gaps(EIGHT);
    assert_ne!(tiles(&engine, LAPTOP), plain);
    engine.set_gaps(Gaps::default());
    assert_eq!(tiles(&engine, LAPTOP), plain);
}

#[test]
fn keyboard_resize_moves_a_gapped_boundary_by_exactly_the_increment() {
    let mut engine = row(2, EIGHT);
    let before = tiles(&engine, DESK);

    assert!(engine.resize_focused_tile(&window(1), ResizeDirection::Grow, 50, DESK));
    let grown = tiles(&engine, DESK);
    assert_eq!(grown[0].width, before[0].width + 50);
    assert_eq!(grown[1].width, before[1].width - 50);
    assert_gapped(&grown, DESK, EIGHT);

    assert!(engine.resize_focused_tile(&window(1), ResizeDirection::Shrink, 50, DESK));
    assert_eq!(tiles(&engine, DESK), before);
}

#[test]
fn dragging_either_side_of_a_gap_moves_the_boundary_without_shifting_by_the_gap() {
    for gaps in [EIGHT, Gaps { inner: 7, outer: 3 }] {
        // The left tile's trailing edge.
        let mut engine = row(2, gaps);
        let before = tiles(&engine, LAPTOP);
        let dragged = Rect {
            width: before[0].width + 100,
            ..before[0]
        };
        assert!(engine.resize_window(&window(1), dragged, LAPTOP));
        let after = tiles(&engine, LAPTOP);
        assert_eq!(after[0], dragged);
        assert_eq!(after[1].x, dragged.x + dragged.width + gaps.inner);
        assert_eq!(after[1].x + after[1].width, before[1].x + before[1].width);

        // The right tile's leading edge.
        let mut engine = row(2, gaps);
        let dragged = Rect {
            x: before[1].x - 100,
            width: before[1].width + 100,
            ..before[1]
        };
        assert!(engine.resize_window(&window(2), dragged, LAPTOP));
        let after = tiles(&engine, LAPTOP);
        assert_eq!(after[1], dragged);
        assert_eq!(after[0].x + after[0].width, dragged.x - gaps.inner);
        assert_eq!(after[0].x, before[0].x);
    }
}

#[test]
fn a_tile_left_where_the_layout_put_it_changes_no_weight() {
    let mut engine = nested(EIGHT);
    let layout = engine.layout.clone();
    let placed = tiles(&engine, LAPTOP);

    for (id, frame) in (1..).zip(&placed) {
        assert!(!engine.resize_window(&window(id), *frame, LAPTOP));
        // Readback noise within the resize tolerance is not a drag either.
        let noisy = Rect {
            x: frame.x + 1,
            width: frame.width - 2,
            height: frame.height + 2,
            ..*frame
        };
        assert!(!engine.resize_window(&window(id), noisy, LAPTOP));
    }

    assert_eq!(engine.layout, layout);
    assert_eq!(tiles(&engine, LAPTOP), placed);
}

#[test]
fn a_corner_drag_moves_both_gapped_boundaries() {
    let mut engine = row(3, EIGHT);
    assert!(engine.move_window_relative(&window(1), &window(2), Direction::Up));
    let before = tiles(&engine, DESK);
    let dragged = Rect {
        width: before[0].width + 60,
        height: before[0].height + 40,
        ..before[0]
    };

    assert!(engine.resize_window(&window(1), dragged, DESK));

    let after = tiles(&engine, DESK);
    assert_eq!(after[0], dragged);
    assert_eq!(after[1].y, dragged.y + dragged.height + 8);
    assert_eq!(after[2].x, dragged.x + dragged.width + 8);
    assert_gapped(&after, DESK, EIGHT);
}

#[test]
fn balance_equalizes_gapped_tiles_and_then_has_nothing_left_to_do() {
    let gaps = Gaps { inner: 7, outer: 3 };
    let mut engine = row(3, gaps);
    for _ in 0..3 {
        assert!(engine.resize_focused_tile(&window(1), ResizeDirection::Grow, 50, LAPTOP));
    }

    assert!(engine.balance_focused_parent(&window(1), LAPTOP));

    let balanced = tiles(&engine, LAPTOP);
    let widths: Vec<_> = balanced.iter().map(|tile| tile.width).collect();
    assert!(widths.iter().max().unwrap() - widths.iter().min().unwrap() <= 1);
    assert_eq!(
        widths.iter().sum::<i32>() + 2 * gaps.inner,
        LAPTOP.width - 2 * gaps.outer
    );
    assert_gapped(&balanced, LAPTOP, gaps);
    assert!(!engine.balance_focused_parent(&window(1), LAPTOP));
}

#[test]
fn a_minimum_size_gets_exactly_its_tile_and_the_gaps_stay() {
    let mut engine = nested(EIGHT);
    let minimums = HashMap::from([(window(3), (420, 500))]);

    assert!(engine.fit_minimums(DESK, &minimums));

    let fitted = tiles(&engine, DESK);
    assert_eq!((fitted[2].width, fitted[2].height), (420, 500));
    assert_gapped(&fitted, DESK, EIGHT);
    // Nothing changed, so the next pass has nothing to refit.
    assert!(!engine.fit_minimums(DESK, &minimums));
    assert_eq!(tiles(&engine, DESK), fitted);

    // New gaps discard the fit; the same minimum is then met again.
    let wider = Gaps {
        inner: 8,
        outer: 40,
    };
    engine.set_gaps(wider);
    engine.fit_minimums(DESK, &minimums);
    let refitted = tiles(&engine, DESK);
    assert_eq!((refitted[2].width, refitted[2].height), (420, 500));
    assert_gapped(&refitted, DESK, wider);
}

#[test]
fn balancing_a_cramped_split_shares_exactly_what_the_gaps_leave() {
    // `[(1 | 2) | 3]`: the nested pair cannot shrink below two minimum tiles,
    // so the third tile gets precisely the rest of the gapped width.
    let mut engine = row(3, EIGHT);
    assert!(engine.create_implicit_split(&window(3), Direction::Right));
    let output = Rect {
        width: 188,
        height: 400,
        ..Rect::default()
    };

    assert!(engine.balance_focused_parent(&window(3), output));

    let balanced = tiles(&engine, output);
    let widths: Vec<_> = balanced.iter().map(|tile| tile.width).collect();
    assert_eq!(widths, [42, 42, 72]);
    assert_gapped(&balanced, output, EIGHT);
    assert!(!engine.balance_focused_parent(&window(3), output));
}

#[test]
fn an_accepted_cycle_reports_and_remembers_the_gapped_frames() {
    let app = AppInstanceId {
        pid: 1,
        launched_at: 1,
    };
    let observed = |window_id| ObservedWindow {
        id: window(window_id),
        frame: DESK,
        resizable: true,
        title: None,
    };
    let mut engine = Engine::default();
    engine.set_gaps(EIGHT);
    let settings = Settings::for_output(DESK);
    let ticket = engine.begin_cycle();

    let accepted = engine
        .accept(
            ticket,
            &NativeObservation {
                cycle: ticket.cycle,
                mouse_down: false,
                applications: vec![AppObservation::Complete {
                    app: ObservedApplication {
                        id: app,
                        bundle_id: "com.example.Editor".into(),
                    },
                    focused: None,
                    windows: vec![observed(1), observed(2)],
                }],
            },
            &settings,
        )
        .unwrap();

    assert_eq!(accepted.placements, engine.placements(&settings));
    assert_eq!(accepted.placements[1].frame.x, 504);
    for placement in &accepted.placements {
        assert_eq!(
            engine.evidence.previous_placements[&placement.slot],
            placement.frame
        );
    }
}

#[test]
fn a_row_that_fits_only_without_its_gaps_stacks_a_pair() {
    let minimums = HashMap::from([(window(1), (495, 0)), (window(2), (495, 0))]);

    // 495 + 495 fits 1000 points of width.
    let mut touching = row(2, Gaps::default());
    assert!(!touching.fit_minimums(DESK, &minimums));
    assert_eq!(tiles(&touching, DESK)[1].x, 500);

    // With an 8 point gap and 8 points at each edge it needs 1014.
    let mut gapped = row(2, EIGHT);
    assert!(gapped.fit_minimums(DESK, &minimums));
    let stacked = tiles(&gapped, DESK);
    assert_eq!((stacked[0].width, stacked[1].width), (984, 984));
    assert_eq!(stacked[1].y, stacked[0].y + stacked[0].height + 8);
    assert_gapped(&stacked, DESK, EIGHT);

    // Stacking needs both heights and the gap between them.
    let tall = HashMap::from([(window(1), (495, 390)), (window(2), (495, 390))]);
    let mut gapped = row(2, EIGHT);
    gapped.fit_minimums(DESK, &tall);
    assert_eq!(tiles(&gapped, DESK), tiles(&row(2, EIGHT), DESK));
}

#[test]
fn previews_fill_their_rect_whatever_the_gaps() {
    let preview = Rect {
        width: 1500,
        height: 1100,
        ..Rect::default()
    };
    let plain = nested(Gaps::default());
    let gapped = nested(Gaps {
        inner: 40,
        outer: 60,
    });

    assert_eq!(gapped.preview(preview), plain.preview(preview));
    assert_eq!(
        gapped.preview_placements(preview),
        plain.preview_placements(preview)
    );
}

#[test]
fn tiles_squeezed_out_by_their_gaps_keep_a_point_inside_the_area() {
    // Six cells of 55 points cannot each give up a 64 point gap.
    let gaps = Gaps {
        inner: 64,
        outer: 16,
    };
    let output = Rect {
        x: -40,
        y: 10,
        width: 300,
        height: 200,
    };
    let squeezed = tiles(&row(6, gaps), output);
    for (index, tile) in squeezed.iter().enumerate() {
        assert_eq!((tile.width, tile.y, tile.height), (1, 26, 168));
        assert!(tile.x >= -24 && tile.x < 244, "{squeezed:?}");
        for other in &squeezed[index + 1..] {
            assert!(separation(*tile, *other, Axis::Horizontal) >= 0);
        }
    }
    // One tile fewer and every gap is whole again.
    assert_gapped(&tiles(&row(5, gaps), output), output, gaps);
}

#[test]
fn outputs_smaller_than_their_gaps_never_produce_empty_or_outside_tiles() {
    let gaps = Gaps {
        inner: 64,
        outer: 200,
    };
    for output in [
        Rect {
            x: -40,
            y: 10,
            width: 90,
            height: 70,
        },
        Rect {
            width: 9,
            height: 7,
            ..Rect::default()
        },
    ] {
        for tile in tiles(&nested(gaps), output) {
            assert!(tile.width > 0 && tile.height > 0, "{tile:?}");
            assert!(tile.x >= output.x && tile.x + tile.width <= output.x + output.width);
            assert!(tile.y >= output.y && tile.y + tile.height <= output.y + output.height);
        }
    }
    // Nothing to divide is not a reason to panic.
    assert_eq!(tiles(&row(2, gaps), Rect::default()).len(), 2);
}

/// Random nestings, resizes, balances and drags on random displays: whatever
/// the layout became, its tiles keep exact gaps and a second look at an
/// untouched tile changes nothing.
#[test]
fn random_layouts_and_edits_keep_every_gap_exact() {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move |bound: u32| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % u64::from(bound)) as i32
    };
    let directions = [
        Direction::Left,
        Direction::Right,
        Direction::Up,
        Direction::Down,
    ];
    let mut exact = 0;
    for _ in 0..500 {
        let count = 1 + next(7).unsigned_abs();
        let gaps = Gaps {
            inner: next(40),
            outer: next(60),
        };
        let output = Rect {
            x: next(3000) - 1500,
            y: next(200),
            width: 700 + next(2000),
            height: 500 + next(1200),
        };
        let mut engine = row(count, gaps);
        for _ in 0..next(12) {
            let (a, b) = (1 + next(count), 1 + next(count));
            let (a, b) = (window(a.unsigned_abs()), window(b.unsigned_abs()));
            let direction = directions[next(4).unsigned_abs() as usize];
            let _changed = match next(6) {
                0 => engine.move_window_relative(&a, &b, direction),
                1 => engine.create_implicit_split(&a, direction),
                2 => engine.resize_focused_tile(&a, ResizeDirection::Grow, 50, output),
                3 => engine.resize_focused_tile(&a, ResizeDirection::Shrink, 50, output),
                4 => engine.balance_focused_parent(&a, output),
                _ => {
                    let frame = engine
                        .placements(&Settings::for_output(output))
                        .into_iter()
                        .find(|placement| placement.window.as_ref() == Some(&a))
                        .unwrap()
                        .frame;
                    let dragged = Rect {
                        x: frame.x + next(81) - 40,
                        y: frame.y + next(81) - 40,
                        width: frame.width + next(161) - 80,
                        height: frame.height + next(161) - 80,
                    };
                    engine.resize_window(&a, dragged, output)
                }
            };
        }
        let placed = tiles(&engine, output);
        assert_eq!(placed.len(), count as usize);
        // A tile whose share is no larger than the gap has collapsed to a point.
        if placed
            .iter()
            .any(|tile| tile.width == 1 || tile.height == 1)
        {
            continue;
        }
        assert_gapped(&placed, output, gaps);
        for (window, frame) in engine.bound_windows().iter().zip(&placed) {
            assert!(!engine.resize_window(window, *frame, output));
        }
        assert_eq!(tiles(&engine, output), placed);
        exact += 1;
    }
    assert!(exact > 450, "only {exact} layouts were checked");
}
