//! A card's workspace chips: the workspaces worth sending the window to, not all of them. Each
//! display's are one group in a recessed track: the workspace it shows, every workspace that
//! holds windows, and the window's own, which is the bright chip. This display's group ends
//! with a plus for its next empty workspace; another display's opens with that display's name.

use rune::{El, col, el, row, text};
use spaceman::{
    WorkspaceId,
    app::{UiWindowOverlayDisplay, UiWindowOverlayWindow, WindowOverlayCommand},
};

use super::{Control, Wiring, glyphs, look};

pub const CHIP: f32 = 24.0;
const CHIP_RADIUS: f32 = 6.0;
/// A track's inset: its corner is a chip's plus this, so the two curves stay concentric.
const TRACK_PAD: f32 = 2.0;
const _: () = assert!(CHIP_RADIUS + TRACK_PAD == look::CONTROL_RADIUS);
/// A track is as tall as a button, so one line holds both.
pub const TRACK_H: f32 = CHIP + 2.0 * TRACK_PAD;
/// Before another display's name, and between it and that display's chips.
const LABEL_PAD: (f32, f32) = (6.0, 4.0);

/// What a chip is to the window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// The workspace that owns the window.
    Current,
    /// A workspace to send it to.
    Send,
    /// This display's next empty workspace, drawn as a plus.
    Empty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chip {
    pub workspace: WorkspaceId,
    pub kind: Kind,
}

/// One display's chips.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    /// The display's number when it is another display than the card's.
    pub display: Option<usize>,
    pub chips: Vec<Chip>,
}

/// `window`'s chips: this display's group, then one per other display with something to show,
/// in the displays' order.
#[must_use]
pub fn groups(window: &UiWindowOverlayWindow, display: &UiWindowOverlayDisplay) -> Vec<Group> {
    let listed = |entry: &&spaceman::app::UiWindowOverlayWorkspace| {
        entry.workspace == window.workspace || entry.visible || entry.occupied()
    };
    let chip = |entry: &spaceman::app::UiWindowOverlayWorkspace| Chip {
        workspace: entry.workspace,
        kind: if entry.workspace == window.workspace {
            Kind::Current
        } else {
            Kind::Send
        },
    };
    let here = display
        .workspaces
        .iter()
        .filter(|entry| entry.on_this_display);
    let mut chips: Vec<Chip> = here.clone().filter(listed).map(chip).collect();
    chips.extend(here.clone().find(|entry| !listed(entry)).map(|entry| Chip {
        workspace: entry.workspace,
        kind: Kind::Empty,
    }));
    let mut groups = vec![Group {
        display: None,
        chips,
    }];
    let mut others: Vec<usize> = display
        .workspaces
        .iter()
        .filter(|entry| !entry.on_this_display)
        .map(|entry| entry.display)
        .collect();
    others.sort_unstable();
    others.dedup();
    for number in others {
        let chips: Vec<Chip> = display
            .workspaces
            .iter()
            .filter(|entry| !entry.on_this_display && entry.display == number)
            .filter(listed)
            .map(chip)
            .collect();
        groups.push(Group {
            display: Some(number),
            chips,
        });
    }
    groups.retain(|group| !group.chips.is_empty());
    groups
}

impl Group {
    fn label(&self) -> Option<String> {
        self.display.map(|number| format!("Display {number}"))
    }

    /// The room the display's name takes at the track's head.
    fn label_width(&self) -> f32 {
        self.label().map_or(0.0, |label| {
            let name = rune::text::advance(&label, look::label_style()).ceil();
            LABEL_PAD.0 + name + LABEL_PAD.1
        })
    }

    /// Chips per row in a track at most `limit` wide: all of them when they fit.
    fn per_row(&self, limit: f32) -> usize {
        let room = limit - 2.0 * TRACK_PAD - self.label_width();
        ((room / CHIP).floor().max(1.0) as usize).min(self.chips.len().max(1))
    }

    /// The track's size when it may be at most `limit` wide: one row when its chips fit,
    /// more rows under the first when they don't.
    #[must_use]
    pub fn size(&self, limit: f32) -> (f32, f32) {
        let per_row = self.per_row(limit);
        let rows = self.chips.len().div_ceil(per_row).max(1);
        (
            2.0 * TRACK_PAD + self.label_width() + per_row as f32 * CHIP,
            2.0 * TRACK_PAD + rows as f32 * CHIP,
        )
    }

    /// The track: the display's name when it is another display, then the chips. `hot` is the
    /// control under the pointer.
    pub fn track(
        &self,
        window: &UiWindowOverlayWindow,
        limit: f32,
        hot: Option<Control>,
        wiring: Wiring,
    ) -> El {
        let per_row = self.per_row(limit);
        let rows = self.chips.chunks(per_row).zip(0..).map(|(chips, index)| {
            row()
                .key(format!("row-{index}"))
                .h(CHIP)
                .children(chips.iter().map(|chip| {
                    let lit = hot == Some(Control::Send(chip.workspace));
                    chip.face(window, lit, wiring)
                }))
        });
        let (width, height) = self.size(limit);
        let mut track = row()
            .key(format!("track-{}", self.display.unwrap_or_default()))
            .role("group")
            .size(width, height)
            .padding(TRACK_PAD)
            .radius(look::CONTROL_RADIUS)
            .fill(look::track());
        if let Some(label) = self.label() {
            track = track.label(label.clone()).child(
                row()
                    .key("display")
                    .h(CHIP)
                    .pl(LABEL_PAD.0)
                    .pr(LABEL_PAD.1)
                    .center_y()
                    .a11y_hidden()
                    .child(
                        text(label)
                            .key("name")
                            .font(look::label_style())
                            .color(look::faint()),
                    ),
            );
        }
        track.child(col().key("chips").children(rows))
    }
}

impl Chip {
    fn face(self, window: &UiWindowOverlayWindow, hot: bool, wiring: Wiring) -> El {
        let workspace = self.workspace;
        let current = self.kind == Kind::Current;
        let (fill, ink) = look::chip(current, hot, self.kind == Kind::Empty);
        let label = match self.kind {
            Kind::Current => format!("On workspace {workspace}"),
            Kind::Send => format!("Send to workspace {workspace}"),
            Kind::Empty => format!("Send to empty workspace {workspace}"),
        };
        let chip = el()
            .key(format!("ws-{workspace}"))
            .role("button")
            .label(label)
            .size(CHIP, CHIP)
            .radius(CHIP_RADIUS)
            .fill(fill)
            .center()
            .child(if self.kind == Kind::Empty {
                glyphs::plus(ink)
            } else {
                text(workspace.to_string())
                    .key("number")
                    .font(look::chip_style(current))
                    .color(ink)
            });
        if current {
            // Lifted a hair off its track, as the one thing on the card that is bright.
            return chip.shadow(0.5, 1.5, look::black(0.4));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use spaceman::{PhysicalWindowId, Rect, app::UiWindowOverlayWorkspace};

    fn workspace(number: u8) -> WorkspaceId {
        WorkspaceId::new(number).unwrap()
    }

    fn window(on: u8) -> UiWindowOverlayWindow {
        UiWindowOverlayWindow {
            window: PhysicalWindowId {
                app: spaceman::AppInstanceId {
                    pid: 7,
                    launched_at: 1,
                },
                window_id: 70,
            },
            bundle_id: None,
            frame: Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
            workspace: workspace(on),
            floating: false,
            pinned: false,
            focused: false,
            swaps: Vec::new(),
        }
    }

    /// Workspaces 1-10: 1-5 on this display (display 1), which shows 1; 6-8 on display 2, which
    /// shows 6; 9 and 10 on display 3, which shows 9. `occupied` hold a window each.
    fn display(occupied: &[u8]) -> UiWindowOverlayDisplay {
        UiWindowOverlayDisplay {
            id: "main".into(),
            screen: Rect {
                x: 0,
                y: 0,
                width: 1600,
                height: 1000,
            },
            workspace: WorkspaceId::ONE,
            fullscreen: false,
            workspaces: (1..=10)
                .map(|number| UiWindowOverlayWorkspace {
                    workspace: workspace(number),
                    windows: usize::from(occupied.contains(&number)),
                    apps: Vec::new(),
                    on_this_display: number <= 5,
                    display: match number {
                        ..=5 => 1,
                        6..=8 => 2,
                        _ => 3,
                    },
                    visible: [1, 6, 9].contains(&number),
                })
                .collect(),
            windows: Vec::new(),
        }
    }

    fn numbers(group: &Group) -> Vec<(String, Kind)> {
        group
            .chips
            .iter()
            .map(|chip| (chip.workspace.to_string(), chip.kind))
            .collect()
    }

    fn chip(number: &str, kind: Kind) -> (String, Kind) {
        (number.to_owned(), kind)
    }

    #[test]
    fn only_occupied_workspaces_get_a_chip_then_one_plus_for_the_next_empty() {
        let groups = groups(&window(1), &display(&[1, 3]));
        assert_eq!(groups[0].display, None);
        assert_eq!(
            numbers(&groups[0]),
            [
                chip("1", Kind::Current),
                chip("3", Kind::Send),
                // 2 is the first empty workspace here; 4 and 5 get nothing.
                chip("2", Kind::Empty),
            ]
        );
        // Every workspace here in use: no plus.
        let full = super::groups(&window(1), &display(&[1, 2, 3, 4, 5]));
        assert_eq!(full[0].chips.len(), 5);
        assert!(full[0].chips.iter().all(|chip| chip.kind != Kind::Empty));
        // A pinned window shows on workspace 1 but lives on 4: 4 is its bright chip, and the
        // workspace on show keeps a chip though nothing lives there.
        let pinned = super::groups(&window(4), &display(&[4]));
        assert_eq!(
            numbers(&pinned[0]),
            [
                chip("1", Kind::Send),
                chip("4", Kind::Current),
                chip("2", Kind::Empty),
            ]
        );
    }

    #[test]
    fn each_other_display_is_its_own_named_group() {
        let groups = groups(&window(1), &display(&[1, 8]));
        assert_eq!(groups.len(), 3);
        // Display 2 shows 6 (empty, but where a window sent to that display would land) and
        // holds 8; display 3 only shows 9. Neither gets a plus.
        assert_eq!(groups[1].label().as_deref(), Some("Display 2"));
        assert_eq!(
            numbers(&groups[1]),
            [chip("6", Kind::Send), chip("8", Kind::Send)]
        );
        assert_eq!(groups[2].label().as_deref(), Some("Display 3"));
        assert_eq!(numbers(&groups[2]), [chip("9", Kind::Send)]);
        // With one display there is one group and no name.
        let mut alone = display(&[1]);
        alone.workspaces.truncate(5);
        let groups = super::groups(&window(1), &alone);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].label(), None);
    }

    #[test]
    fn a_track_holds_its_chips_in_one_row_and_wraps_them_when_it_must() {
        let groups = groups(&window(1), &display(&[1, 2, 3, 4, 5, 8]));
        let here = &groups[0];
        assert_eq!(here.chips.len(), 5);
        assert_eq!(here.size(400.0), (5.0 * CHIP + 2.0 * TRACK_PAD, TRACK_H));
        // Room for three: two rows, three chips wide.
        let narrow = 3.0 * CHIP + 2.0 * TRACK_PAD + 5.0;
        assert_eq!(
            here.size(narrow),
            (3.0 * CHIP + 2.0 * TRACK_PAD, 2.0 * CHIP + 2.0 * TRACK_PAD)
        );
        // No room at all still holds one chip a row.
        assert_eq!(here.size(0.0).0, CHIP + 2.0 * TRACK_PAD);
        // Another display's name widens its track.
        let other = &groups[1];
        assert_eq!(
            other.size(400.0).0,
            2.0 * TRACK_PAD + other.label_width() + 2.0 * CHIP
        );
        assert!(other.label_width() > LABEL_PAD.0 + LABEL_PAD.1);
    }
}
