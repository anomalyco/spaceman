//! Rune Playground (`⇧⌘D` and `--playground`) for auditing Spaceman's scenes
//! on isolated fixture state without touching `config.toml` or live windows.

use std::sync::OnceLock;

use rune::{App, Playground};

use crate::{BASE_SCENES, Pane, Scene, Ui, WINDOW_SIZE, blank_fixture};

pub fn attach(app: App<Ui>) -> App<Ui> {
    let playground = Playground::new()
        .enabled(true)
        .bar(false)
        .scenes(|name| name.contains('/'))
        .sizes(&[("Spaceman window", WINDOW_SIZE)]);
    app.playground(playground)
}

pub fn scenes() -> &'static [Scene] {
    static ALL: OnceLock<Vec<Scene>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut out = BASE_SCENES.to_vec();
        let mut grouped: Vec<(usize, Scene)> = BASE_SCENES
            .iter()
            .map(|&(name, apply)| {
                let mut ui = blank_fixture();
                apply(&mut ui);
                let (rank, group) = group(&ui);
                let path: &'static str = Box::leak(format!("{group}/{name}").into_boxed_str());
                (rank, (path, apply))
            })
            .collect();
        grouped.sort_by_key(|(rank, _)| *rank);
        out.extend(grouped.into_iter().map(|(_, scene)| scene));
        out
    })
}

fn group(ui: &Ui) -> (usize, &'static str) {
    let rank = Pane::ALL
        .iter()
        .position(|pane| *pane == ui.pane)
        .unwrap_or(0);
    (rank, ui.pane.title())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_every_base_scene_under_its_pane() {
        let grouped: Vec<&str> = scenes()
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| name.contains('/'))
            .collect();
        assert_eq!(grouped.len(), BASE_SCENES.len());
        for expected in [
            "General/general",
            "General/general-setup",
            "Rules/rules",
            "Rules/rules-drag",
            "Rules/rules-add",
            "Workspaces/workspaces",
            "Shortcuts/shortcuts",
        ] {
            assert!(
                grouped.contains(&expected),
                "{expected} missing from {grouped:?}"
            );
        }
    }
}
