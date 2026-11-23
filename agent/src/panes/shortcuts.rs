//! Shortcuts pane: Workspaces and Windows & Layout keyboard and trackpad reference.

use rune::{El, col};

use crate::{
    controls::{self, Paint},
    view::scroll_body,
};

const WORKSPACE_SHORTCUTS: &[(&str, &[&[&str]])] = &[
    ("Switch to workspace 1–10", &[&["⌥", "1…9, 0"]]),
    ("Move window to workspace 1–10", &[&["⌥", "⇧", "1…9, 0"]]),
    ("Switch to a named workspace", &[&["⌥", "A…Z"]]),
    ("Move window to a named workspace", &[&["⌥", "⇧", "A…Z"]]),
    ("Previous workspace", &[&["⌥", "Tab"]]),
    ("Move workspace to next display", &[&["⌥", "⇧", "Tab"]]),
    ("Switch adjacent workspace", &[&["3-finger swipe"]]),
];

const LAYOUT_SHORTCUTS: &[(&str, &[&[&str]])] = &[
    ("Next window", &[&["⌥", "`"]]),
    ("Focus tile", &[&["⌥", "H J K L"]]),
    ("Move tile", &[&["⌥", "⇧", "H J K L"]]),
    ("Flip split", &[&["⌥", "/"]]),
    ("Resize tile", &[&["⌥", "-"], &["⌥", "="]]),
    ("Toggle floating window", &[&["⌥", "⇧", "Space"]]),
    ("Fullscreen layout", &[&["⌥", "⇧", "F"]]),
    ("Scrolling Strip (Labs)", &[&["⌥", "⇧", "S"]]),
];

pub fn pane(p: Paint) -> El {
    scroll_body(
        "shortcuts-body",
        col()
            .key("sections")
            .gap(24.0)
            .child(controls::section(
                p,
                "workspaces",
                "Workspaces",
                controls::shortcut_rows(p, "workspace-shortcut", WORKSPACE_SHORTCUTS),
            ))
            .child(controls::section(
                p,
                "windows-layout",
                "Windows & Layout",
                controls::shortcut_rows(p, "layout-shortcut", LAYOUT_SHORTCUTS),
            )),
    )
}
