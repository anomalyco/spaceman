//! A card's workspace chips: one per workspace the window can be sent to. The chip of the
//! window's own workspace is bright, an occupied one carries a dot per window (up to three), an
//! empty one recedes, and the one under the pointer lights amber. A hairline rules off the
//! workspaces of another display.

use rune::{El, el, row, text};
use spaceman::app::{
    UiWindowOverlayDisplay, UiWindowOverlayWindow, UiWindowOverlayWorkspace, WindowOverlayCommand,
};

use super::{Control, Wiring, look};

pub const CHIP: f32 = 28.0;
pub const GAP: f32 = 4.0;
/// The rule between one display's workspaces and the next's.
const RULE: (f32, f32) = (1.0, 14.0);
const DOT: f32 = 3.0;
const DOT_GAP: f32 = 2.0;
const MAX_DOTS: usize = 3;
/// Every number sits this far above centre, clear of the dots, so the row keeps one baseline.
const NUMBER_LIFT: f32 = 3.0;

/// Chips per row before they wrap: ten on a card with room, five on a narrow one.
#[must_use]
pub const fn per_row(compact: bool) -> usize {
    if compact { 5 } else { 10 }
}

/// Whether a rule goes before chip `index` of a row that starts at `start`: the chip before
/// it, in the same row, is on another display.
fn ruled(workspaces: &[UiWindowOverlayWorkspace], start: usize, index: usize) -> bool {
    index > start && workspaces[index].on_this_display != workspaces[index - 1].on_this_display
}

/// The chip rows' size: the widest row, and every row with the gaps between.
#[must_use]
pub fn size(workspaces: &[UiWindowOverlayWorkspace], per_row: usize) -> (f32, f32) {
    let per_row = per_row.max(1);
    let width = (0..workspaces.len())
        .step_by(per_row)
        .map(|start| {
            let end = (start + per_row).min(workspaces.len());
            let rules = (start..end)
                .filter(|&index| ruled(workspaces, start, index))
                .count();
            (end - start) as f32 * CHIP
                + (end - start - 1) as f32 * GAP
                + rules as f32 * (RULE.0 + GAP)
        })
        .fold(0.0, f32::max);
    let rows = workspaces.len().div_ceil(per_row).max(1);
    (width, rows as f32 * CHIP + (rows - 1) as f32 * GAP)
}

/// `window`'s chips, `hot` the one under the pointer.
pub fn chips(
    window: &UiWindowOverlayWindow,
    display: &UiWindowOverlayDisplay,
    compact: bool,
    hot: Option<Control>,
    wiring: Wiring,
) -> El {
    let per_row = per_row(compact);
    let workspaces = &display.workspaces;
    let rows = (0..workspaces.len()).step_by(per_row).map(|start| {
        let end = (start + per_row).min(workspaces.len());
        let mut line = row()
            .key(format!("chips-{start}"))
            .h(CHIP)
            .gap(GAP)
            .center_y();
        for index in start..end {
            let entry = &workspaces[index];
            if ruled(workspaces, start, index) {
                line = line.child(
                    el().key(format!("rule-{index}"))
                        .size(RULE.0, RULE.1)
                        .fill(look::white(0.14))
                        .a11y_hidden(),
                );
            }
            let lit = hot == Some(Control::Send(entry.workspace));
            line = line.child(chip(window, entry, lit, wiring));
        }
        line
    });
    rune::col().key("chips").gap(GAP).children(rows)
}

fn chip(
    window: &UiWindowOverlayWindow,
    entry: &UiWindowOverlayWorkspace,
    hot: bool,
    wiring: Wiring,
) -> El {
    let workspace = entry.workspace;
    let current = workspace == window.workspace;
    let (fill, rim, ink, dot) = if current {
        (
            look::text(),
            look::white(0.0),
            look::ink_on_bright(),
            look::ink_on_bright().alpha(0.5),
        )
    } else if hot {
        (
            look::accent_a(0.18),
            look::accent_a(0.60),
            look::accent(),
            look::accent_a(0.75),
        )
    } else if entry.occupied() {
        (
            look::white(0.075),
            look::white(0.08),
            look::text(),
            look::white(0.5),
        )
    } else {
        (
            look::white(0.03),
            look::white(0.0),
            look::faint(),
            look::white(0.0),
        )
    };
    let label = if current {
        format!("On workspace {workspace}")
    } else {
        format!("Send to workspace {workspace}")
    };
    let mut chip = el()
        .key(format!("ws-{workspace}"))
        .role("button")
        .label(label)
        .size(CHIP, CHIP)
        .radius(look::CONTROL_RADIUS)
        .fill(fill)
        .border(wiring.hair, rim)
        .pb(NUMBER_LIFT)
        .center()
        .child(
            text(workspace.to_string())
                .key("number")
                .font(look::chip_style())
                .color(ink),
        );
    let dots = entry.windows.min(MAX_DOTS);
    let run = dots as f32 * DOT + dots.saturating_sub(1) as f32 * DOT_GAP;
    for index in 0..dots {
        chip = chip.child(
            el().key(format!("dot-{index}"))
                .absolute(
                    (CHIP - run) / 2.0 + index as f32 * (DOT + DOT_GAP),
                    CHIP - DOT - 4.0,
                )
                .size(DOT, DOT)
                .radius(DOT / 2.0)
                .fill(dot),
        );
    }
    if current {
        return chip;
    }
    wiring.control(
        chip,
        &window.window,
        Control::Send(workspace),
        WindowOverlayCommand::MoveTo {
            window: window.window.clone(),
            workspace,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use spaceman::WorkspaceId;

    /// The first `count` workspaces (1-10, then A-Z), the first `here` of them on this display.
    fn workspaces(count: usize, here: usize) -> Vec<UiWindowOverlayWorkspace> {
        let named = ('A'..='Z').filter_map(|letter| WorkspaceId::named(letter).ok());
        WorkspaceId::numbered()
            .chain(named)
            .take(count)
            .enumerate()
            .map(|(index, workspace)| UiWindowOverlayWorkspace {
                workspace,
                windows: 0,
                apps: Vec::new(),
                on_this_display: index < here,
            })
            .collect()
    }

    #[test]
    fn chips_wrap_at_ten_or_at_five_on_a_narrow_card() {
        let ten = workspaces(10, 10);
        assert_eq!(size(&ten, per_row(false)), (10.0 * CHIP + 9.0 * GAP, CHIP));
        assert_eq!(
            size(&ten, per_row(true)),
            (5.0 * CHIP + 4.0 * GAP, 2.0 * CHIP + GAP)
        );
        // An eleventh starts a second row; the first row still sets the width.
        let eleven = workspaces(11, 11);
        assert_eq!(
            size(&eleven, 10),
            (10.0 * CHIP + 9.0 * GAP, 2.0 * CHIP + GAP)
        );
        assert_eq!(size(&workspaces(3, 3), 10).0, 3.0 * CHIP + 2.0 * GAP);
        // No workspaces at all still leaves a row's height.
        assert_eq!(size(&[], 10), (0.0, CHIP));
    }

    #[test]
    fn a_rule_separates_another_displays_workspaces_and_widens_the_row() {
        // 1-5 here, 6-10 on the other display: one rule, between 5 and 6.
        let split = workspaces(10, 5);
        assert!(ruled(&split, 0, 5));
        assert!(!ruled(&split, 0, 4) && !ruled(&split, 0, 6));
        assert_eq!(size(&split, 10).0, 10.0 * CHIP + 9.0 * GAP + RULE.0 + GAP);
        // On a narrow card 6 starts the second row: a row never opens with a rule.
        assert!(!ruled(&split, 5, 5));
        assert_eq!(size(&split, 5).0, 5.0 * CHIP + 4.0 * GAP);
    }
}
