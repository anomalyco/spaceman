use super::*;

const AREA: Rect = Rect {
    x: 0,
    y: 25,
    width: 1440,
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

fn four() -> Vec<StripColumn> {
    columns(&[StripWidth::TwoThirds; 4])
}

fn frame(x: i32, width: i32) -> Rect {
    Rect { x, width, ..AREA }
}

#[test]
fn empty_and_single_column_strips_fill_the_area() {
    assert_eq!(
        compute_strip_layout(AREA, &[], None, None),
        StripLayout::default()
    );
    let single = columns(&[StripWidth::Half]);
    let layout = compute_strip_layout(AREA, &single, Some(SlotId(1)), None);
    assert_eq!(layout.visible, vec![(SlotId(1), AREA)]);
    assert!(layout.hidden.is_empty());
    assert_eq!(layout.anchor, Some(SlotId(1)));
}

#[test]
fn two_thirds_column_shares_the_display_with_a_peeking_neighbor() {
    let layout = compute_strip_layout(AREA, &four(), Some(SlotId(1)), None);
    assert_eq!(
        layout.visible,
        vec![(SlotId(1), frame(0, 950)), (SlotId(2), frame(950, 490))]
    );
    assert_eq!(layout.hidden, vec![SlotId(3), SlotId(4)]);
    assert_eq!(layout.anchor, Some(SlotId(1)));
}

#[test]
fn half_columns_split_the_display_evenly() {
    let strip = columns(&[StripWidth::Half; 3]);
    let layout = compute_strip_layout(AREA, &strip, Some(SlotId(2)), Some(SlotId(1)));
    assert_eq!(
        layout.visible,
        vec![(SlotId(1), frame(0, 720)), (SlotId(2), frame(720, 720))]
    );
    assert_eq!(layout.hidden, vec![SlotId(3)]);
}

#[test]
fn full_column_shows_alone() {
    let mut strip = four();
    strip[1].width = StripWidth::Full;
    let layout = compute_strip_layout(AREA, &strip, Some(SlotId(2)), Some(SlotId(1)));
    assert_eq!(layout.visible, vec![(SlotId(2), AREA)]);
    assert_eq!(layout.hidden, vec![SlotId(1), SlotId(3), SlotId(4)]);
    assert_eq!(layout.anchor, Some(SlotId(2)));
}

#[test]
fn focusing_a_column_already_on_screen_never_moves_the_viewport() {
    let layout = compute_strip_layout(AREA, &four(), Some(SlotId(2)), Some(SlotId(1)));
    assert_eq!(
        layout.visible,
        vec![(SlotId(1), frame(0, 490)), (SlotId(2), frame(490, 950))]
    );
    assert_eq!(layout.anchor, Some(SlotId(1)));
}

#[test]
fn stepping_past_the_viewport_slides_it_by_one_column() {
    // Right: the incoming column enters on the right.
    let right = compute_strip_layout(AREA, &four(), Some(SlotId(3)), Some(SlotId(1)));
    assert_eq!(
        right.visible,
        vec![(SlotId(2), frame(0, 490)), (SlotId(3), frame(490, 950))]
    );
    assert_eq!(right.hidden, vec![SlotId(1), SlotId(4)]);
    assert_eq!(right.anchor, Some(SlotId(2)));
    // Left: it enters on the left.
    let left = compute_strip_layout(AREA, &four(), Some(SlotId(2)), Some(SlotId(3)));
    assert_eq!(
        left.visible,
        vec![(SlotId(2), frame(0, 950)), (SlotId(3), frame(950, 490))]
    );
    assert_eq!(left.anchor, Some(SlotId(2)));
}

#[test]
fn viewport_clamps_to_the_ends_of_the_strip() {
    let last = compute_strip_layout(AREA, &four(), Some(SlotId(4)), None);
    assert_eq!(
        last.visible,
        vec![(SlotId(3), frame(0, 490)), (SlotId(4), frame(490, 950))]
    );
    // An anchor on the last column (left there by a full-width view) still shows two.
    let anchored_last = compute_strip_layout(AREA, &four(), Some(SlotId(4)), Some(SlotId(4)));
    assert_eq!(anchored_last.visible, last.visible);
    assert_eq!(anchored_last.anchor, Some(SlotId(3)));
}

#[test]
fn unknown_active_column_falls_back_to_the_anchor_then_the_first() {
    let anchored = compute_strip_layout(AREA, &four(), Some(SlotId(99)), Some(SlotId(3)));
    assert_eq!(anchored.anchor, Some(SlotId(3)));
    let first = compute_strip_layout(AREA, &four(), None, None);
    assert_eq!(first.anchor, Some(SlotId(1)));
}

#[test]
fn every_viewport_fills_the_area_with_columns_wholly_inside_it() {
    let widths = [StripWidth::Half, StripWidth::TwoThirds, StripWidth::Full];
    for count in 1..=5 {
        for width in widths {
            let strip = columns(&vec![width; count]);
            for active in 0..=count as u64 {
                for anchor in 0..=count as u64 {
                    let layout = compute_strip_layout(
                        AREA,
                        &strip,
                        Some(SlotId(active)),
                        Some(SlotId(anchor)),
                    );
                    let total: i32 = layout.visible.iter().map(|(_, frame)| frame.width).sum();
                    assert_eq!(total, AREA.width);
                    assert_eq!(layout.visible.len() + layout.hidden.len(), count);
                    for (_, frame) in &layout.visible {
                        assert!(frame.x >= AREA.x && frame.x + frame.width <= AREA.x + AREA.width);
                        assert!(frame.width * 100 >= AREA.width * 34 || count == 1);
                        assert_eq!((frame.y, frame.height), (AREA.y, AREA.height));
                    }
                    if (1..=count as u64).contains(&active) {
                        assert!(
                            layout
                                .visible
                                .iter()
                                .any(|(slot, _)| *slot == SlotId(active))
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn widths_step_between_half_two_thirds_and_full() {
    assert_eq!(StripWidth::default(), StripWidth::TwoThirds);
    assert_eq!(StripWidth::TwoThirds.wider(), StripWidth::Full);
    assert_eq!(StripWidth::Full.wider(), StripWidth::Full);
    assert_eq!(StripWidth::Full.narrower(), StripWidth::TwoThirds);
    assert_eq!(StripWidth::TwoThirds.narrower(), StripWidth::Half);
    assert_eq!(StripWidth::Half.narrower(), StripWidth::Half);
}

#[test]
fn unsized_columns_follow_the_last_width_picked() {
    let mut state = StripState::default();
    assert_eq!(state.width(SlotId(1)), StripWidth::TwoThirds);
    state.set_width(SlotId(2), StripWidth::Full);
    assert_eq!(state.width(SlotId(2)), StripWidth::Full);
    assert_eq!(state.width(SlotId(3)), StripWidth::Full);
    state.set_width(SlotId(1), StripWidth::Half);
    assert_eq!(state.width(SlotId(2)), StripWidth::Full);
    assert_eq!(state.width(SlotId(3)), StripWidth::Half);
    state.anchor = Some(SlotId(2));
    state.retain_slots(|slot| slot != SlotId(2));
    assert_eq!(state.widths, vec![(SlotId(1), StripWidth::Half)]);
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
    state.set_width(SlotId(3), StripWidth::Full);
    let encoded = serde_json::to_value(&state).unwrap();
    assert_eq!(
        encoded,
        serde_json::json!({
            "enabled": true,
            "default_width": "full",
            "widths": [[3, "full"]],
            "anchor": 3
        })
    );
    assert_eq!(
        serde_json::from_value::<StripState>(encoded).unwrap(),
        state
    );
}
