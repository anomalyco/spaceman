//! One window's card, top to bottom: what it is (the app, never a title, and where it lives),
//! where it can go (a chip per workspace), and how it sits (Float, Pin, Fullscreen, and an
//! arrow toward each neighbouring tile it can swap with). The line under the app's name says
//! where the window is, and while the pointer is on a control, what a click would do.

use rune::{El, SpringPair, TextStyle, Transition, col, el, row, text};
use spaceman::{
    Direction,
    app::{UiWindowOverlayDisplay, UiWindowOverlayWindow, WindowOverlayCommand},
};

use super::{Control, Wiring, chips, glyphs, look, place};
use crate::hud::Icons;

const ICON: f32 = 28.0;
const ICON_GAP: f32 = 10.0;
const HEADER_H: f32 = 32.0;
const NAME_H: f32 = 16.0;
const STATUS_H: f32 = 14.0;
/// The focus dot after the app's name.
const FOCUS_DOT: f32 = 6.0;
/// Room the header's text always has, so the status line can say what a click would do.
const HEADER_TEXT: (f32, f32) = (176.0, 118.0);
/// An app icon in the status line: the apps a workspace already holds.
const STATUS_ICON: f32 = 12.0;
const STATUS_GAP: f32 = 4.0;
/// At most this many of them.
const STATUS_APPS: usize = 4;
/// Header to chips, and chips to the buttons.
const HEADER_GAP: f32 = 10.0;
const ROW_GAP: f32 = 8.0;
const ACTION_H: f32 = 28.0;
const ACTION_GAP: f32 = 4.0;
/// A labelled button: 8, glyph, 6, label, 10. The glyph's side is trimmed, as its box
/// carries its own air.
const ACTION_PAD: (f32, f32, f32) = (8.0, 6.0, 10.0);
/// Between the mode buttons and the swap arrows, at least.
const SWAP_GAP: f32 = 12.0;

/// How a card is drawn for its window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    /// The window is too narrow for the full card: chips wrap at five and buttons lose
    /// their labels.
    pub compact: bool,
    pub size: (f32, f32),
}

/// The card for `window`: the full one when the window is wide enough to hold it.
#[must_use]
pub fn plan(window: &UiWindowOverlayWindow, display: &UiWindowOverlayDisplay) -> Plan {
    let full = size(window, display, false);
    let compact = (window.frame.width as f32) < full.0 + 2.0 * place::CARD_GAP;
    Plan {
        compact,
        size: if compact {
            size(window, display, true)
        } else {
            full
        },
    }
}

/// The mode buttons a window gets: a float has no Fullscreen Layout to join.
fn modes(window: &UiWindowOverlayWindow) -> &'static [Control] {
    if window.floating {
        &[Control::Float, Control::Pin]
    } else {
        &[Control::Float, Control::Pin, Control::Fullscreen]
    }
}

const fn label(control: Control) -> &'static str {
    match control {
        Control::Float => "Float",
        Control::Pin => "Pin",
        Control::Fullscreen => "Fullscreen",
        Control::Swap(_) | Control::Send(_) => "",
    }
}

fn action_width(control: Control, compact: bool) -> f32 {
    if compact || label(control).is_empty() {
        return ACTION_H;
    }
    let label = rune::text::advance(label(control), look::label_style());
    (ACTION_PAD.0 + glyphs::SIZE + ACTION_PAD.1 + label + ACTION_PAD.2).ceil()
}

fn run(widths: impl Iterator<Item = f32>) -> f32 {
    let (count, sum) = widths.fold((0, 0.0), |(count, sum), width| (count + 1, sum + width));
    sum + (count as f32 - 1.0).max(0.0) * ACTION_GAP
}

fn actions_width(window: &UiWindowOverlayWindow, compact: bool) -> f32 {
    let modes = run(modes(window)
        .iter()
        .map(|control| action_width(*control, compact)));
    if window.swaps.is_empty() {
        modes
    } else {
        modes + SWAP_GAP + run(window.swaps.iter().map(|_| ACTION_H))
    }
}

/// The card's size: its widest row and the card's inset, and its three rows.
#[must_use]
pub fn size(
    window: &UiWindowOverlayWindow,
    display: &UiWindowOverlayDisplay,
    compact: bool,
) -> (f32, f32) {
    let chips = chips::size(&display.workspaces, chips::per_row(compact));
    let header = ICON
        + ICON_GAP
        + if compact {
            HEADER_TEXT.1
        } else {
            HEADER_TEXT.0
        };
    let inner = chips.0.max(actions_width(window, compact)).max(header);
    (
        (inner + 2.0 * look::PAD).ceil(),
        2.0 * look::PAD + HEADER_H + HEADER_GAP + chips.1 + ROW_GAP + ACTION_H,
    )
}

/// What the line under the app's name says.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    /// What it always says in full.
    pub text: String,
    /// What it adds while there is room, each after a dot.
    pub notes: Vec<&'static str>,
    /// Apps to show after the text while there is room, by bundle ID: what a workspace
    /// already holds.
    pub apps: Vec<String>,
    /// It says what a click would do, not where the window is.
    pub preview: bool,
}

/// Where `window` is, or with the pointer on `control`, what a click there would do.
#[must_use]
pub fn status(
    window: &UiWindowOverlayWindow,
    display: &UiWindowOverlayDisplay,
    control: Option<Control>,
) -> Status {
    let preview = |text: &str| Status {
        text: text.to_owned(),
        notes: Vec::new(),
        apps: Vec::new(),
        preview: true,
    };
    match control {
        None => Status {
            text: if window.pinned {
                "On every workspace".to_owned()
            } else {
                format!("Workspace {}", window.workspace)
            },
            notes: Vec::new(),
            apps: Vec::new(),
            preview: false,
        },
        Some(Control::Float) if window.floating => preview("Tile it again"),
        Some(Control::Float) => preview("Float above the tiles"),
        Some(Control::Pin) if window.pinned => preview("Unpin from every workspace"),
        Some(Control::Pin) => preview("Pin to every workspace"),
        Some(Control::Fullscreen) if display.fullscreen => preview("Show every tile again"),
        Some(Control::Fullscreen) => preview("Show one window at a time"),
        Some(Control::Swap(side)) => preview(match side {
            Direction::Left => "Swap with the tile on the left",
            Direction::Right => "Swap with the tile on the right",
            Direction::Up => "Swap with the tile above",
            Direction::Down => "Swap with the tile below",
        }),
        Some(Control::Send(workspace)) => {
            let entry = display
                .workspaces
                .iter()
                .find(|entry| entry.workspace == workspace);
            let mut notes = Vec::new();
            if entry.is_some_and(|entry| !entry.on_this_display) {
                notes.push("other display");
            }
            if entry.is_none_or(|entry| !entry.occupied()) {
                notes.push("empty");
            }
            Status {
                text: format!("Send to {workspace}"),
                notes,
                apps: entry.map(|entry| entry.apps.clone()).unwrap_or_default(),
                preview: true,
            }
        }
    }
}

/// What a card needs beside its window.
#[derive(Clone, Copy)]
pub struct Parts<'a> {
    pub display: &'a UiWindowOverlayDisplay,
    /// The app's name.
    pub name: &'a str,
    pub icons: &'a Icons,
    /// The control of this card the pointer is on.
    pub hot: Option<Control>,
    pub plan: Plan,
    pub origin: (f32, f32),
    /// Reduce Motion: the card fades where it rests.
    pub still: bool,
    pub wiring: Wiring,
}

/// `window`'s card.
pub fn card(window: &UiWindowOverlayWindow, parts: Parts<'_>) -> El {
    let Parts {
        display,
        plan,
        wiring,
        ..
    } = parts;
    let springs = SpringPair::new(look::ENTER, look::EXIT);
    let presence = if parts.still {
        Transition::opacity().springs(springs)
    } else {
        Transition::scale(look::REST_SCALE).springs(springs)
    };
    let inner = plan.size.0 - 2.0 * look::PAD;
    let card = col()
        .key(format!(
            "card-{}-{}",
            window.window.app.pid, window.window.window_id
        ))
        .role("group")
        .label(parts.name.to_owned())
        .absolute(parts.origin.0, parts.origin.1)
        .w(plan.size.0)
        .h(plan.size.1)
        .padding(look::PAD)
        .motion(look::TRAVEL)
        .transition(presence)
        .child(header(window, &parts, inner))
        .child(chips::chips(window, display, plan.compact, parts.hot, wiring).mt(HEADER_GAP))
        .child(actions(window, &parts).mt(ROW_GAP));
    wiring.card(look::glass(card, wiring.hair), &window.window)
}

fn header(window: &UiWindowOverlayWindow, parts: &Parts<'_>, inner: f32) -> El {
    let text_w = inner - ICON - ICON_GAP;
    let dot = if window.focused { FOCUS_DOT + 6.0 } else { 0.0 };
    let mut name = row().key("name").h(NAME_H).gap(6.0).center_y().child(
        text(parts.name.to_owned())
            .key("app")
            .font(look::name_style())
            .color(look::text())
            .max_w(text_w - dot)
            .truncate_fade(),
    );
    if window.focused {
        name = name.child(
            el().key("focused")
                .role("img")
                .label("Has keyboard focus")
                .size(FOCUS_DOT, FOCUS_DOT)
                .radius(FOCUS_DOT / 2.0)
                .fill(look::accent()),
        );
    }
    row()
        .key("header")
        .h(HEADER_H)
        .gap(ICON_GAP)
        .center_y()
        .child(app_icon(
            parts.icons,
            window.bundle_id.as_deref(),
            parts.name,
            ICON,
        ))
        .child(col().key("id").w(text_w).child(name).child(status_line(
            &status(window, parts.display, parts.hot),
            text_w,
            parts.icons,
        )))
}

/// What the status line shows in `width`: its text whole, then each note that still fits,
/// then as many of its apps as the room left holds. A narrow card says less, never half of it.
fn fit(status: &Status, width: f32) -> (String, usize) {
    let advance = |says: &str| rune::text::advance(says, look::status_style());
    let mut says = status.text.clone();
    for note in &status.notes {
        let longer = format!("{says} · {note}");
        if advance(&longer) <= width {
            says = longer;
        }
    }
    let room = (width - advance(&says)) / (STATUS_ICON + STATUS_GAP);
    let apps = (room.max(0.0) as usize).min(STATUS_APPS);
    (says, apps.min(status.apps.len()))
}

/// The status line: each thing it says cross-fades with the next, in place, so nothing moves.
fn status_line(status: &Status, width: f32, icons: &Icons) -> El {
    let (says, apps) = fit(status, width);
    let ink = if status.preview {
        look::text()
    } else {
        look::quiet()
    };
    let mut line = row()
        .key(format!("says-{says}-{}", status.apps.join(",")))
        .absolute(0.0, 0.0)
        .h(STATUS_H)
        .gap(STATUS_GAP)
        .center_y()
        .transition(Transition::opacity().springs(look::STATUS))
        .child(
            text(says)
                .key("text")
                .font(look::status_style())
                .color(ink)
                .max_w(width)
                .truncate_fade(),
        );
    for bundle in status.apps.iter().take(apps) {
        line = line.child(app_icon(
            icons,
            Some(bundle),
            &crate::apps::fallback_name(bundle),
            STATUS_ICON,
        ));
    }
    el().key("status").w(width).h(STATUS_H).child(line)
}

/// An app's icon at `size`, or its initial in a rounded square when it has none here.
fn app_icon(icons: &Icons, bundle: Option<&str>, name: &str, size: f32) -> El {
    let base = el()
        .key(format!("icon-{}", bundle.unwrap_or_default()))
        .size(size, size)
        .a11y_hidden();
    match bundle.and_then(|bundle| icons.get(bundle)) {
        Some(pixels) => base.pixels(pixels),
        None => base
            .radius(size * 0.25)
            .fill(look::white(0.14))
            .center()
            .child(
                text(name.chars().next().map(String::from).unwrap_or_default())
                    .font(TextStyle::new((size * 0.5).max(7.0), 600.0))
                    .color(look::quiet()),
            ),
    }
}

/// The mode buttons, then the swap arrows at the row's far end.
fn actions(window: &UiWindowOverlayWindow, parts: &Parts<'_>) -> El {
    let target = &window.window;
    let mut line = row().key("actions").h(ACTION_H).gap(ACTION_GAP).center_y();
    for control in modes(window) {
        let (on, command, hint) = match control {
            Control::Float => (
                window.floating,
                WindowOverlayCommand::ToggleFloat(target.clone()),
                if window.floating { "Tile" } else { "Float" },
            ),
            Control::Pin => (
                window.pinned,
                WindowOverlayCommand::TogglePin(target.clone()),
                if window.pinned { "Unpin" } else { "Pin" },
            ),
            _ => (
                parts.display.fullscreen,
                WindowOverlayCommand::ToggleFullscreen(target.clone()),
                "Fullscreen Layout",
            ),
        };
        line = line.child(action(window, parts, *control, on, hint, command));
    }
    if !window.swaps.is_empty() {
        line = line.child(el().key("gap").grow(1.0).min_w(SWAP_GAP - 2.0 * ACTION_GAP));
        for side in &window.swaps {
            let command = WindowOverlayCommand::Swap(target.clone(), *side);
            let hint = match side {
                Direction::Left => "Swap left",
                Direction::Right => "Swap right",
                Direction::Up => "Swap up",
                Direction::Down => "Swap down",
            };
            line = line.child(action(
                window,
                parts,
                Control::Swap(*side),
                false,
                hint,
                command,
            ));
        }
    }
    line
}

/// One button: its glyph, and on a full card a mode's label. `on` lights it amber.
fn action(
    window: &UiWindowOverlayWindow,
    parts: &Parts<'_>,
    control: Control,
    on: bool,
    hint: &'static str,
    command: WindowOverlayCommand,
) -> El {
    let hot = parts.hot == Some(control);
    let (fill, rim, ink) = look::control(on, hot);
    let glyph = match control {
        Control::Float => glyphs::float(ink, look::faint()),
        Control::Pin => glyphs::pin(ink),
        Control::Fullscreen => glyphs::fullscreen(ink),
        Control::Swap(side) => glyphs::arrow(side, ink),
        Control::Send(_) => el(),
    };
    let key = match control {
        Control::Swap(side) => format!("swap-{side}"),
        other => label(other).to_lowercase(),
    };
    let mut button = row()
        .key(key)
        .role("button")
        .label(hint)
        .h(ACTION_H)
        .w(action_width(control, parts.plan.compact))
        .radius(look::CONTROL_RADIUS)
        .fill(fill)
        .border(parts.wiring.hair, rim)
        .center_y();
    button = if parts.plan.compact || label(control).is_empty() {
        button.center().child(glyph)
    } else {
        button
            .pl(ACTION_PAD.0)
            .pr(ACTION_PAD.2)
            .gap(ACTION_PAD.1)
            .child(glyph)
            .child(
                text(label(control))
                    .key("label")
                    .font(look::label_style())
                    .color(ink),
            )
    };
    parts
        .wiring
        .control(button, &window.window, control, command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spaceman::{PhysicalWindowId, Rect, WorkspaceId, app::UiWindowOverlayWorkspace};

    fn window(floating: bool, swaps: Vec<Direction>) -> UiWindowOverlayWindow {
        UiWindowOverlayWindow {
            window: PhysicalWindowId {
                app: spaceman::AppInstanceId {
                    pid: 7,
                    launched_at: 1,
                },
                window_id: 70,
            },
            bundle_id: Some("com.example.Editor".into()),
            frame: Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
            workspace: WorkspaceId::ONE,
            floating,
            pinned: false,
            focused: false,
            swaps,
        }
    }

    /// A display listing the first `workspaces` workspaces (1-10, then A-Z): 2 holds two apps,
    /// and 6 onward are on another display.
    fn display(workspaces: usize) -> UiWindowOverlayDisplay {
        let named = ('A'..='Z').filter_map(|letter| WorkspaceId::named(letter).ok());
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
            workspaces: WorkspaceId::numbered()
                .chain(named)
                .take(workspaces)
                .zip(1..)
                .map(|(workspace, number)| UiWindowOverlayWorkspace {
                    workspace,
                    windows: usize::from(number == 2) * 2,
                    apps: if number == 2 {
                        vec!["com.example.Mail".into(), "com.example.Chat".into()]
                    } else {
                        Vec::new()
                    },
                    on_this_display: number <= 5,
                })
                .collect(),
            windows: Vec::new(),
        }
    }

    #[test]
    fn a_float_offers_float_and_pin_and_a_tile_adds_fullscreen() {
        assert_eq!(
            modes(&window(true, Vec::new())),
            [Control::Float, Control::Pin]
        );
        assert_eq!(
            modes(&window(false, Vec::new())),
            [Control::Float, Control::Pin, Control::Fullscreen]
        );
    }

    #[test]
    fn the_card_is_as_wide_as_its_widest_row_and_three_rows_tall() {
        let tile = window(false, vec![Direction::Left, Direction::Right]);
        let ten = size(&tile, &display(10), false);
        let chips = chips::size(&display(10).workspaces, 10);
        assert_eq!(ten.0, (chips.0 + 2.0 * look::PAD).ceil());
        assert_eq!(
            ten.1,
            2.0 * look::PAD + HEADER_H + HEADER_GAP + chips::CHIP + ROW_GAP + ACTION_H
        );
        // A second row of chips makes it taller, not wider.
        let more = size(&tile, &display(14), false);
        assert_eq!(more.0, ten.0);
        assert_eq!(more.1 - ten.1, chips::CHIP + chips::GAP);
        // With few workspaces the buttons set the width, and swap arrows widen them.
        let few = size(&tile, &display(3), false);
        assert_eq!(
            few.0,
            (actions_width(&tile, false) + 2.0 * look::PAD).ceil()
        );
        assert!(size(&window(false, Vec::new()), &display(3), false).0 < few.0);
    }

    #[test]
    fn a_window_narrower_than_its_card_gets_the_compact_one() {
        let display = display(10);
        let wide = window(false, vec![Direction::Right]);
        assert!(!plan(&wide, &display).compact);
        let mut narrow = wide.clone();
        narrow.frame.width = 300;
        let compact = plan(&narrow, &display);
        assert!(compact.compact);
        assert!(compact.size.0 < plan(&wide, &display).size.0);
        // Five chips a row: a second row, and buttons without their labels.
        assert_eq!(
            compact.size.1 - plan(&wide, &display).size.1,
            chips::CHIP + chips::GAP
        );
        assert_eq!(action_width(Control::Fullscreen, true), ACTION_H);
        assert!(action_width(Control::Fullscreen, false) > ACTION_H);
    }

    #[test]
    fn the_status_line_says_where_the_window_is_then_what_a_click_would_do() {
        let display = display(10);
        let tile = window(false, vec![Direction::Left]);
        let rest = status(&tile, &display, None);
        assert_eq!((rest.text.as_str(), rest.preview), ("Workspace 1", false));
        let mut pinned = window(true, Vec::new());
        pinned.pinned = true;
        assert_eq!(status(&pinned, &display, None).text, "On every workspace");

        let text = |window: &UiWindowOverlayWindow, control| {
            let status = status(window, &display, Some(control));
            assert!(status.preview);
            status.text
        };
        assert_eq!(text(&tile, Control::Float), "Float above the tiles");
        assert_eq!(
            text(&window(true, Vec::new()), Control::Float),
            "Tile it again"
        );
        assert_eq!(text(&tile, Control::Pin), "Pin to every workspace");
        assert_eq!(text(&pinned, Control::Pin), "Unpin from every workspace");
        assert_eq!(
            text(&tile, Control::Fullscreen),
            "Show one window at a time"
        );
        assert_eq!(
            text(&tile, Control::Swap(Direction::Left)),
            "Swap with the tile on the left"
        );

        // An occupied workspace names its apps (as icons); an empty one says so; one on
        // another display says that first.
        let two = WorkspaceId::new(2).unwrap();
        let send = status(&tile, &display, Some(Control::Send(two)));
        assert_eq!(send.text, "Send to 2");
        assert!(send.notes.is_empty());
        assert_eq!(send.apps, ["com.example.Mail", "com.example.Chat"]);
        let three = status(
            &tile,
            &display,
            Some(Control::Send(WorkspaceId::new(3).unwrap())),
        );
        assert_eq!(
            (three.text.as_str(), &three.notes[..]),
            ("Send to 3", &["empty"][..])
        );
        let seven = status(
            &tile,
            &display,
            Some(Control::Send(WorkspaceId::new(7).unwrap())),
        );
        assert_eq!(seven.notes, ["other display", "empty"]);
        assert!(seven.apps.is_empty());
    }

    #[test]
    fn a_narrow_status_line_keeps_its_text_whole_and_drops_notes_then_apps() {
        let status = Status {
            text: "Send to 6".to_owned(),
            notes: vec!["other display", "empty"],
            apps: (0..6)
                .map(|index| format!("com.example.App{index}"))
                .collect(),
            preview: true,
        };
        // Room for everything: both notes, and no more apps than the line ever shows.
        assert_eq!(
            fit(&status, 600.0),
            ("Send to 6 · other display · empty".to_owned(), STATUS_APPS)
        );
        // Room for the text and the short note: the long one goes, and the apps with it.
        let short = rune::text::advance("Send to 6 · empty", look::status_style());
        assert_eq!(
            fit(&status, short + 1.0),
            ("Send to 6 · empty".to_owned(), 0)
        );
        // Room for the text and two icons.
        let text = rune::text::advance("Send to 6", look::status_style());
        let two = text + 2.0 * (STATUS_ICON + STATUS_GAP) + 1.0;
        assert_eq!(fit(&status, two), ("Send to 6".to_owned(), 2));
        // No room at all: the text still, to fade at the card's edge.
        assert_eq!(fit(&status, 10.0), ("Send to 6".to_owned(), 0));
    }
}
