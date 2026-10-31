//! Spaceman's settings window, a Rune app the agent starts from the menu bar (see the agent's
//! `settings.rs`). It edits `config.toml` directly; the agent reloads it on its next tick.

mod agent;
mod apps;
mod controls;
mod map;
mod panes;
mod rules;
mod view;

use rune::{AppSpec, KeyEvent, PointerPress, Tx, Waker};

use rules::{Doc, Place};

/// One fixed size: tiling window managers float such a window, and every pane fits it.
pub const WINDOW_SIZE: (f32, f32) = (860.0, 580.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    General,
    Rules,
    Workspaces,
    Shortcuts,
}

impl Pane {
    pub const ALL: [Pane; 4] = [
        Pane::General,
        Pane::Rules,
        Pane::Workspaces,
        Pane::Shortcuts,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Pane::General => "General",
            Pane::Rules => "Rules",
            Pane::Workspaces => "Workspaces",
            Pane::Shortcuts => "Shortcuts",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Pane::General => "general",
            Pane::Rules => "rules",
            Pane::Workspaces => "workspaces",
            Pane::Shortcuts => "shortcuts",
        }
    }
}

/// The one anchored menu that is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Menu {
    /// Pick a running app to send to this place.
    Add(Place),
    /// A rule's context menu, at the secondary press.
    Rule(usize, PointerPress),
}

pub struct Ui {
    pub pane: Pane,
    /// Someone picked a pane, so the agent's first report doesn't move them.
    pub navigated: bool,
    pub doc: Doc,
    pub apps: apps::Catalog,
    /// `None` until the agent first answers; `Some(None)` while it isn't running.
    pub agent: Option<Option<agent::Agent>>,
    inbox: Option<agent::Shared>,
    pub drag: Option<map::Drag>,
    pub menu: Option<Menu>,
    pub running: Vec<apps::Running>,
    /// The last save that failed.
    pub error: Option<String>,
    /// Scenes render fixed data and ignore the live agent and file.
    fixture: bool,
}

impl Ui {
    fn load() -> Ui {
        let mut ui = Ui {
            pane: Pane::Rules,
            navigated: false,
            doc: Doc::load(),
            apps: apps::Catalog::default(),
            agent: None,
            inbox: None,
            drag: None,
            menu: None,
            running: Vec::new(),
            error: None,
            fixture: false,
        };
        ui.look_up_apps();
        ui
    }

    pub fn select_pane(&mut self, pane: Pane) {
        self.pane = pane;
        self.navigated = true;
        self.menu = None;
        self.drag = None;
    }

    /// Names and icons for every app the rules mention, before the view needs them.
    fn look_up_apps(&mut self) {
        for rule in self.doc.rules() {
            if let Some(bundle_id) = &rule.bundle_id {
                self.apps.get(bundle_id);
            }
        }
    }

    /// Applies `edit` and saves; a save that fails puts the rules back as they were.
    pub fn edit(&mut self, edit: impl FnOnce(&mut Doc)) {
        let before = self.doc.clone();
        edit(&mut self.doc);
        if !self.fixture {
            match self.doc.save() {
                Ok(()) => self.error = None,
                Err(error) => {
                    self.error = Some(error);
                    self.doc = before;
                }
            }
        }
        self.look_up_apps();
    }

    pub fn open_add_menu(&mut self, place: Place) {
        let ruled: Vec<String> = self
            .doc
            .rules()
            .into_iter()
            .filter(|rule| rule.title.is_none())
            .filter_map(|rule| rule.bundle_id)
            .collect();
        self.running = apps::running()
            .into_iter()
            .filter(|app| !ruled.contains(&app.bundle_id))
            .collect();
        for app in &self.running {
            self.apps.get(&app.bundle_id);
        }
        self.menu = Some(Menu::Add(place));
    }
}

fn setup(ui: &mut Ui, waker: Waker) {
    if !ui.fixture {
        ui.inbox = Some(agent::start(waker));
    }
}

fn poll(ui: &mut Ui, _clock: f64, _tx: &mut Tx) -> bool {
    let Some(inbox) = &ui.inbox else {
        return false;
    };
    let news = std::mem::take(
        &mut *inbox
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    let mut changed = false;
    if let Some(agent) = news.agent {
        if !ui.navigated
            && ui.agent.is_none()
            && agent.as_ref().is_some_and(|agent| !agent.accessibility)
        {
            ui.pane = Pane::General;
        }
        ui.agent = Some(agent);
        changed = true;
    }
    if news.config_changed && ui.drag.is_none() {
        ui.doc = Doc::load();
        ui.look_up_apps();
        changed = true;
    }
    changed
}

fn on_key(ui: &mut Ui, event: &KeyEvent, _tx: &mut Tx) -> bool {
    if event.key == rune::Key::Escape && (ui.drag.is_some() || ui.menu.is_some()) {
        ui.drag = None;
        ui.menu = None;
        return true;
    }
    false
}

fn spec() -> AppSpec<Ui> {
    AppSpec {
        title: "Spaceman",
        size: WINDOW_SIZE,
        min_size: Some(WINDOW_SIZE),
        init: Ui::load,
        view: view::view,
        scenes: SCENES,
        fonts: rune_theme_opencode::FONTS,
        families: rune_theme_opencode::FAMILIES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
    }
}

fn main() {
    rune::App::new(spec())
        .chrome(rune::Chrome {
            full_size_content: true,
            traffic_lights: Some((18.0, 18.0)),
            drag_regions: true,
        })
        .window_options(rune::WindowOptions {
            min_size: Some(WINDOW_SIZE),
            max_size: Some(WINDOW_SIZE),
            resizable: false,
            ..Default::default()
        })
        .standard_menus()
        .run();
}

// ---------------------------------------------------------------------------
// Scenes for headless stills: `spaceman-settings --snap out.png --scene rules`

type Scene = (&'static str, fn(&mut Ui));

const SCENES: &[Scene] = &[
    ("general", |ui| fixture(ui, Pane::General)),
    ("general-setup", |ui| {
        fixture(ui, Pane::General);
        if let Some(Some(agent)) = &mut ui.agent {
            agent.accessibility = false;
            agent.launch_at_login = agent::LaunchAtLogin::Off;
        }
    }),
    ("rules", |ui| fixture(ui, Pane::Rules)),
    ("rules-drag", |ui| {
        fixture(ui, Pane::Rules);
        ui.drag = map::Drag::fixture(&ui.doc);
    }),
    ("rules-add", |ui| {
        fixture(ui, Pane::Rules);
        ui.open_add_menu(Place::Float);
    }),
    ("workspaces", |ui| fixture(ui, Pane::Workspaces)),
    ("shortcuts", |ui| fixture(ui, Pane::Shortcuts)),
];

fn fixture(ui: &mut Ui, pane: Pane) {
    ui.fixture = true;
    ui.pane = pane;
    ui.doc = Doc::parse(include_str!("../fixtures/config.toml"));
    ui.look_up_apps();
    ui.agent = Some(Some(agent::Agent {
        accessibility: true,
        automatic: true,
        launch_at_login: agent::LaunchAtLogin::On,
        update: agent::Update::Idle,
        active: vec!["3".into(), "M".into()],
    }));
}
