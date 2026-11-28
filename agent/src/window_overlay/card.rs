//! The window's card, two rows. The first says what the window is: its app's icon and name,
//! never a title, and beside the name, only while the pointer is on a control, what a click
//! there would do. The second is where it can go and how it sits: the workspace chips in their
//! tracks, then Float, Pin and Fullscreen, then an arrow toward each neighbouring tile it can
//! swap with. On a window too narrow for that row it wraps onto more lines.

use rune::{El, SpringPair, TextStyle, Transition, col, el, row, text};
use spaceman::{
    Direction,
    app::{UiWindowOverlayDisplay, UiWindowOverlayWindow, WindowOverlayCommand},
};

use super::{Control, Wiring, chips, glyphs, look, place};
use crate::hud::Icons;

const ICON: f32 = 28.0;
const ICON_GAP: f32 = 9.0;
const HEADER_H: f32 = 28.0;
/// Between the app's name and the hint.
const NAME_GAP: f32 = 8.0;
const HINT_H: f32 = 14.0;
/// The hint sits this far below centre, so its smaller type shares the name's baseline.
const HINT_DROP: f32 = 1.0;
/// Less room than this beside the name and the hint stays away.
const HINT_MIN: f32 = 44.0;
/// Room the header's text has on a card whose row is shorter than this, so a hint fits.
const HEADER_TEXT: f32 = 150.0;
/// An app icon in the hint: the apps a workspace already holds.
const HINT_ICON: f32 = 12.0;
const HINT_GAP: f32 = 4.0;
/// At most this many of them.
const HINT_APPS: usize = 4;
/// Header to the row, and between the row's lines when it wraps.
const ROW_GAP: f32 = 8.0;
/// Between two tracks. Twice this parts the tracks from the buttons and the modes from the
/// swap arrows.
const ITEM_GAP: f32 = 6.0;
const BUTTON: f32 = 28.0;
const _: () = assert!(BUTTON == chips::TRACK_H);
const BUTTON_GAP: f32 = 2.0;
/// The narrowest a card's row may be held to, however narrow its window: four swap arrows.
const MIN_ROW: f32 = 4.0 * BUTTON + 3.0 * BUTTON_GAP;

/// One thing in the card's second row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Item {
    /// A display's chips, by its place in the card's groups.
    Track(usize),
    /// Float, Pin and Fullscreen.
    Modes,
    /// The swap arrows.
    Swaps,
}

/// How a card is drawn for its window.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub size: (f32, f32),
    /// The widest a line of the row, or a track in it, may be.
    limit: f32,
    groups: Vec<chips::Group>,
    /// The row's items, line by line: one line when the window is wide enough.
    lines: Vec<Vec<Item>>,
}

/// The mode buttons a window gets: a float has no Fullscreen Layout to join.
fn modes(window: &UiWindowOverlayWindow) -> &'static [Control] {
    if window.floating {
        &[Control::Float, Control::Pin]
    } else {
        &[Control::Float, Control::Pin, Control::Fullscreen]
    }
}

/// A run of `count` buttons.
fn run(count: usize) -> f32 {
    count as f32 * BUTTON + count.saturating_sub(1) as f32 * BUTTON_GAP
}

fn item_size(
    item: Item,
    window: &UiWindowOverlayWindow,
    groups: &[chips::Group],
    limit: f32,
) -> (f32, f32) {
    match item {
        Item::Track(index) => groups[index].size(limit),
        Item::Modes => (run(modes(window).len()), BUTTON),
        Item::Swaps => (run(window.swaps.len()), BUTTON),
    }
}

/// The room between two neighbours on a line.
const fn gap(before: Item, after: Item) -> f32 {
    match (before, after) {
        (Item::Track(_), Item::Track(_)) => ITEM_GAP,
        _ => 2.0 * ITEM_GAP,
    }
}

/// How wide `line` is, given each item's width.
fn line_width(line: &[Item], width: impl Fn(Item) -> f32) -> f32 {
    let gaps: f32 = line.windows(2).map(|pair| gap(pair[0], pair[1])).sum();
    line.iter().map(|item| width(*item)).sum::<f32>() + gaps
}

/// `items` in order on as few lines as hold them, none wider than `limit` unless one item is.
fn flow(items: &[Item], limit: f32, width: impl Fn(Item) -> f32) -> Vec<Vec<Item>> {
    let mut lines: Vec<Vec<Item>> = Vec::new();
    for item in items {
        match lines.last_mut() {
            Some(line) => {
                line.push(*item);
                if line_width(line, &width) > limit {
                    line.pop();
                    lines.push(vec![*item]);
                }
            }
            None => lines.push(vec![*item]),
        }
    }
    lines
}

/// The card for `window`: its row on one line when the window is wide enough to hold it,
/// wrapped to the window's width when not.
#[must_use]
pub fn plan(window: &UiWindowOverlayWindow, display: &UiWindowOverlayDisplay) -> Plan {
    let on_screen = display.screen.width as f32 - 2.0 * place::EDGE_MARGIN;
    let outer = (window.frame.width as f32 - 2.0 * place::CARD_GAP).min(on_screen);
    let limit = (outer - 2.0 * look::PAD).max(MIN_ROW);
    let groups = chips::groups(window, display);
    let mut items: Vec<Item> = (0..groups.len()).map(Item::Track).collect();
    items.push(Item::Modes);
    if !window.swaps.is_empty() {
        items.push(Item::Swaps);
    }
    let size = |item| item_size(item, window, &groups, limit);
    let lines = flow(&items, limit, |item| size(item).0);
    let widest = lines
        .iter()
        .map(|line| line_width(line, |item| size(item).0))
        .fold(0.0, f32::max);
    let rows: f32 = lines
        .iter()
        .map(|line| ROW_GAP + line.iter().map(|item| size(*item).1).fold(0.0, f32::max))
        .sum();
    let inner = widest.max((ICON + ICON_GAP + HEADER_TEXT).min(limit));
    Plan {
        size: (
            (inner + 2.0 * look::PAD).ceil(),
            2.0 * look::PAD + HEADER_H + rows,
        ),
        limit,
        groups,
        lines,
    }
}

/// What the hint beside the app's name says.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    /// What it always says in full.
    pub text: String,
    /// What it adds while there is room, each after a dot.
    pub notes: Vec<String>,
    /// Apps to show after the text while there is room, by bundle ID: what a workspace
    /// already holds.
    pub apps: Vec<String>,
}

/// What a click on `control` would do to `window`. With the pointer on no control there is
/// nothing to say: the card never repeats where the window already is.
#[must_use]
pub fn status(
    window: &UiWindowOverlayWindow,
    display: &UiWindowOverlayDisplay,
    control: Option<Control>,
) -> Option<Status> {
    let says = |text: &str| Status {
        text: text.to_owned(),
        notes: Vec::new(),
        apps: Vec::new(),
    };
    Some(match control? {
        Control::Float if window.floating => says("Tile it again"),
        Control::Float => says("Float above the tiles"),
        Control::Pin if window.pinned => says("Unpin"),
        Control::Pin => says("Pin to every workspace"),
        Control::Fullscreen if display.fullscreen => says("Show every tile again"),
        Control::Fullscreen => says("Show one window at a time"),
        Control::Swap(side) => says(match side {
            Direction::Left => "Swap left",
            Direction::Right => "Swap right",
            Direction::Up => "Swap up",
            Direction::Down => "Swap down",
        }),
        Control::Send(workspace) => {
            let entry = display
                .workspaces
                .iter()
                .find(|entry| entry.workspace == workspace);
            let mut notes = Vec::new();
            if let Some(entry) = entry.filter(|entry| !entry.on_this_display) {
                notes.push(format!("Display {}", entry.display));
            }
            if entry.is_none_or(|entry| !entry.occupied()) {
                notes.push("empty".to_owned());
            }
            Status {
                text: format!("Send to {workspace}"),
                notes,
                apps: entry.map(|entry| entry.apps.clone()).unwrap_or_default(),
            }
        }
    })
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
    pub plan: &'a Plan,
    pub origin: (f32, f32),
    /// Reduce Motion: the card fades where it rests.
    pub still: bool,
    pub wiring: Wiring,
}

/// `window`'s card.
pub fn card(window: &UiWindowOverlayWindow, parts: Parts<'_>) -> El {
    let Parts { plan, wiring, .. } = parts;
    let springs = SpringPair::new(look::ENTER, look::EXIT);
    let presence = if parts.still {
        Transition::opacity().springs(springs)
    } else {
        Transition::scale(look::REST_SCALE).springs(springs)
    };
    let inner = plan.size.0 - 2.0 * look::PAD;
    let lines = plan
        .lines
        .iter()
        .zip(0..)
        .map(|(line, index)| self::line(window, &parts, line, index));
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
        .gap(ROW_GAP)
        .motion(look::TRAVEL)
        .transition(presence)
        .child(header(window, &parts, inner))
        .children(lines);
    wiring.card(look::glass(card, wiring.hair), &window.window)
}

fn header(window: &UiWindowOverlayWindow, parts: &Parts<'_>, inner: f32) -> El {
    let text_w = inner - ICON - ICON_GAP;
    let name_w = rune::text::advance(parts.name, look::name_style())
        .ceil()
        .min(text_w);
    let room = text_w - name_w - NAME_GAP;
    let mut id = row()
        .key("id")
        .w(text_w)
        .h(HEADER_H)
        .gap(NAME_GAP)
        .center_y()
        .child(
            text(parts.name.to_owned())
                .key("app")
                .font(look::name_style())
                .color(look::text())
                .max_w(text_w)
                .truncate_fade(),
        );
    if room >= HINT_MIN {
        let status = status(window, parts.display, parts.hot);
        id = id.child(hint(status.as_ref(), room, parts.icons));
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
        .child(id)
}

/// What the hint shows in `width`: its text whole, then each note that still fits, then as
/// many of its apps as the room left holds. A narrow card says less, never half of it.
fn fit(status: &Status, width: f32) -> (String, usize) {
    let advance = |says: &str| rune::text::advance(says, look::hint_style());
    let mut says = status.text.clone();
    for note in &status.notes {
        let longer = format!("{says} · {note}");
        if advance(&longer) <= width {
            says = longer;
        }
    }
    let room = (width - advance(&says)) / (HINT_ICON + HINT_GAP);
    let apps = (room.max(0.0) as usize).min(HINT_APPS);
    (says, apps.min(status.apps.len()))
}

/// The hint: there only while the pointer is on a control. Each thing it says cross-fades
/// with the next, in place, so nothing on the card moves.
fn hint(status: Option<&Status>, width: f32, icons: &Icons) -> El {
    let slot = el().key("hint").w(width).h(HINT_H).mt(HINT_DROP);
    let Some(status) = status else {
        return slot;
    };
    let (says, apps) = fit(status, width);
    let mut line = row()
        .key(format!("says-{says}-{}", status.apps.join(",")))
        .absolute(0.0, 0.0)
        .h(HINT_H)
        .gap(HINT_GAP)
        .center_y()
        .transition(Transition::opacity().springs(look::HINT))
        .child(
            text(says)
                .key("text")
                .font(look::hint_style())
                .color(look::quiet())
                .max_w(width)
                .truncate_fade(),
        );
    for bundle in status.apps.iter().take(apps) {
        line = line.child(app_icon(
            icons,
            Some(bundle),
            &crate::apps::fallback_name(bundle),
            HINT_ICON,
        ));
    }
    slot.child(line)
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

/// One line of the row: its tracks from the left, its buttons at the far end.
fn line(window: &UiWindowOverlayWindow, parts: &Parts<'_>, items: &[Item], index: i32) -> El {
    let plan = parts.plan;
    let height = items
        .iter()
        .map(|item| item_size(*item, window, &plan.groups, plan.limit).1)
        .fold(0.0, f32::max);
    let mut line = row()
        .key(format!("line-{index}"))
        .h(height)
        .gap(ITEM_GAP)
        .center_y();
    for (item, place) in items.iter().zip(0..) {
        // The buttons sit at the line's far end; on a line of their own, the modes keep the
        // near end and the arrows take the far one.
        let spring = match (item, place) {
            (Item::Track(_), _) | (Item::Modes, 0) => false,
            (Item::Modes, _) => true,
            (Item::Swaps, _) => !matches!(items.first(), Some(Item::Track(_))),
        };
        if spring && place > 0 {
            line = line.child(el().key(format!("spring-{place}")).grow(1.0));
        }
        line = line.child(match item {
            Item::Track(group) => {
                plan.groups[*group].track(window, plan.limit, parts.hot, parts.wiring)
            }
            Item::Modes => mode_buttons(window, parts),
            // After the modes on a line that opens with tracks: the same room a spring leaves.
            Item::Swaps if place > 0 && !spring => swap_buttons(window, parts).ml(ITEM_GAP),
            Item::Swaps => swap_buttons(window, parts),
        });
    }
    line
}

fn mode_buttons(window: &UiWindowOverlayWindow, parts: &Parts<'_>) -> El {
    let target = &window.window;
    let buttons = modes(window).iter().map(|control| {
        let (on, command, label) = match control {
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
        button(window, parts, *control, on, label, command)
    });
    row()
        .key("modes")
        .h(BUTTON)
        .gap(BUTTON_GAP)
        .children(buttons)
}

fn swap_buttons(window: &UiWindowOverlayWindow, parts: &Parts<'_>) -> El {
    let buttons = window.swaps.iter().map(|side| {
        let command = WindowOverlayCommand::Swap(window.window.clone(), *side);
        let label = match side {
            Direction::Left => "Swap left",
            Direction::Right => "Swap right",
            Direction::Up => "Swap up",
            Direction::Down => "Swap down",
        };
        button(window, parts, Control::Swap(*side), false, label, command)
    });
    row()
        .key("swaps")
        .h(BUTTON)
        .gap(BUTTON_GAP)
        .children(buttons)
}

/// One button: its glyph alone. `on` holds it bright.
fn button(
    window: &UiWindowOverlayWindow,
    parts: &Parts<'_>,
    control: Control,
    on: bool,
    label: &'static str,
    command: WindowOverlayCommand,
) -> El {
    let hot = parts.hot == Some(control);
    let (fill, rim, ink) = look::button(on, hot);
    let (key, glyph) = match control {
        Control::Float => ("float".to_owned(), glyphs::float(ink, look::faint())),
        Control::Pin => ("pin".to_owned(), glyphs::pin(ink)),
        Control::Fullscreen => ("fullscreen".to_owned(), glyphs::fullscreen(ink)),
        Control::Swap(side) => (format!("swap-{side}"), glyphs::arrow(side, ink)),
        Control::Send(_) => (String::new(), el()),
    };
    let face = el()
        .key(key)
        .role("button")
        .label(label)
        .size(BUTTON, BUTTON)
        .radius(look::CONTROL_RADIUS)
        .fill(fill)
        .border(parts.wiring.hair, rim)
        .center()
        .child(glyph);
    parts.wiring.control(face, &window.window, control, command)
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

    /// A display listing workspaces 1-10: this one shows 1 and holds 1-5, display 2 shows 6
    /// and holds the rest. 2 holds two apps and 7 one.
    fn display() -> UiWindowOverlayDisplay {
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
                .zip(1..)
                .map(|(workspace, number)| UiWindowOverlayWorkspace {
                    workspace,
                    windows: match number {
                        1 | 7 => 1,
                        2 => 2,
                        _ => 0,
                    },
                    apps: if number == 2 {
                        vec!["com.example.Mail".into(), "com.example.Chat".into()]
                    } else {
                        Vec::new()
                    },
                    on_this_display: number <= 5,
                    display: if number <= 5 { 1 } else { 2 },
                    visible: number == 1 || number == 6,
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
    fn the_card_is_two_rows_and_as_wide_as_its_second() {
        let display = display();
        let tile = window(false, vec![Direction::Left, Direction::Right]);
        let plan = plan(&tile, &display);
        // This display's track, display 2's, the modes, the arrows: one line.
        assert_eq!(
            plan.lines,
            [[Item::Track(0), Item::Track(1), Item::Modes, Item::Swaps]]
        );
        let tracks: f32 = plan
            .groups
            .iter()
            .map(|group| group.size(plan.limit).0)
            .sum();
        let row = tracks + ITEM_GAP + 2.0 * ITEM_GAP + run(3) + 2.0 * ITEM_GAP + run(2);
        assert_eq!(plan.size.0, (row + 2.0 * look::PAD).ceil());
        assert_eq!(plan.size.1, 2.0 * look::PAD + HEADER_H + ROW_GAP + BUTTON);
        // Without neighbours to swap with the row ends at the modes.
        let alone = super::plan(&window(false, Vec::new()), &display);
        assert_eq!(alone.lines[0].last(), Some(&Item::Modes));
        assert!(alone.size.0 < plan.size.0);
        // A short row still leaves the header room for a hint.
        let mut one = display.clone();
        one.workspaces.truncate(1);
        let short = super::plan(&window(true, Vec::new()), &one);
        assert_eq!(
            short.size.0,
            ICON + ICON_GAP + HEADER_TEXT + 2.0 * look::PAD
        );
    }

    #[test]
    fn a_window_narrower_than_the_row_wraps_it_and_never_widens_the_card() {
        let display = display();
        let wide = window(false, vec![Direction::Left, Direction::Right]);
        let mut narrow = wide.clone();
        narrow.frame.width = 280;
        let plan = plan(&narrow, &display);
        // The tracks keep the first line; the buttons take the next.
        assert_eq!(
            plan.lines,
            [
                vec![Item::Track(0), Item::Track(1)],
                vec![Item::Modes, Item::Swaps]
            ]
        );
        assert!(plan.size.0 <= 280.0 - 2.0 * place::CARD_GAP);
        assert_eq!(
            plan.size.1,
            2.0 * look::PAD + HEADER_H + 2.0 * (ROW_GAP + BUTTON)
        );
        // Narrower than anything: every item on its own line, and the card only as wide as
        // its widest, the four arrows.
        narrow.frame.width = 60;
        narrow.swaps = vec![
            Direction::Left,
            Direction::Up,
            Direction::Down,
            Direction::Right,
        ];
        let plan = super::plan(&narrow, &display);
        assert_eq!(plan.lines.len(), 4);
        assert_eq!(plan.size.0, MIN_ROW + 2.0 * look::PAD);
    }

    #[test]
    fn the_hint_is_silent_at_rest_and_says_what_a_click_would_do() {
        let display = display();
        let tile = window(false, vec![Direction::Left]);
        // No "Workspace 1": the bright chip already says where the window is.
        assert_eq!(status(&tile, &display, None), None);

        let text = |window: &UiWindowOverlayWindow, control| {
            status(window, &display, Some(control)).unwrap().text
        };
        let mut pinned = window(true, Vec::new());
        pinned.pinned = true;
        assert_eq!(text(&tile, Control::Float), "Float above the tiles");
        assert_eq!(
            text(&window(true, Vec::new()), Control::Float),
            "Tile it again"
        );
        assert_eq!(text(&tile, Control::Pin), "Pin to every workspace");
        assert_eq!(text(&pinned, Control::Pin), "Unpin");
        assert_eq!(
            text(&tile, Control::Fullscreen),
            "Show one window at a time"
        );
        assert_eq!(text(&tile, Control::Swap(Direction::Left)), "Swap left");

        // An occupied workspace names its apps (as icons); an empty one says so; one on
        // another display names that display first.
        let send = |number| {
            let workspace = WorkspaceId::new(number).unwrap();
            status(&tile, &display, Some(Control::Send(workspace))).unwrap()
        };
        let two = send(2);
        assert_eq!(two.text, "Send to 2");
        assert!(two.notes.is_empty());
        assert_eq!(two.apps, ["com.example.Mail", "com.example.Chat"]);
        let three = send(3);
        assert_eq!(
            (three.text.as_str(), &three.notes[..]),
            ("Send to 3", &["empty".to_owned()][..])
        );
        assert_eq!(send(7).notes, ["Display 2"]);
        assert_eq!(send(6).notes, ["Display 2", "empty"]);
    }

    #[test]
    fn a_narrow_hint_keeps_its_text_whole_and_drops_notes_then_apps() {
        let status = Status {
            text: "Send to 6".to_owned(),
            notes: vec!["Display 2".to_owned(), "empty".to_owned()],
            apps: (0..6)
                .map(|index| format!("com.example.App{index}"))
                .collect(),
        };
        // Room for everything: both notes, and no more apps than the hint ever shows.
        assert_eq!(
            fit(&status, 600.0),
            ("Send to 6 · Display 2 · empty".to_owned(), HINT_APPS)
        );
        // Room for the text and the short note: the long one goes, and the apps with it.
        let short = rune::text::advance("Send to 6 · empty", look::hint_style());
        assert_eq!(
            fit(&status, short + 1.0),
            ("Send to 6 · empty".to_owned(), 0)
        );
        // Room for the text and two icons.
        let text = rune::text::advance("Send to 6", look::hint_style());
        let two = text + 2.0 * (HINT_ICON + HINT_GAP) + 1.0;
        assert_eq!(fit(&status, two), ("Send to 6".to_owned(), 2));
        // No room at all: the text still, to fade at the card's edge.
        assert_eq!(fit(&status, 10.0), ("Send to 6".to_owned(), 0));
    }
}
