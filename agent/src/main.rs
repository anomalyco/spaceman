//! Spaceman's single-process macOS agent (`spaceman-agent`): runs the window
//! manager, native status bar item, Sparkle updater, Settings window, and Rune
//! Playground (`⇧⌘D` / `--playground`) on one AppKit + Rune event loop.

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: procwatch::alloc::Counting = procwatch::alloc::Counting::system();

mod agent;
mod apps;
mod controls;
#[cfg(target_os = "macos")]
mod host;
mod map;
mod panes;
mod playground;
mod rules;
mod view;

use std::sync::Arc;

use rules::{Doc, Place};
use rune::{AppSpec, KeyEvent, PointerPress, Tx, Waker};

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
    /// `None` until the agent first answers; `Some(None)` in disconnected fixtures.
    pub agent: Option<Option<agent::Agent>>,
    #[cfg(target_os = "macos")]
    bridge: Option<spaceman::app::UiBridge>,
    window: Option<rune::AppHandle<Ui>>,
    frame_remembered: bool,
    pub drag: Option<map::Drag>,
    pub menu: Option<Menu>,
    pub running: Vec<apps::Running>,
    /// The last save that failed.
    pub error: Option<String>,
    /// Scenes render fixed data and ignore the live agent and file.
    fixture: bool,
}

impl Ui {
    #[cfg(target_os = "macos")]
    fn load_live(bridge: spaceman::app::UiBridge) -> Ui {
        let mut ui = Ui {
            pane: Pane::Rules,
            navigated: false,
            doc: Doc::load(),
            apps: apps::Catalog::default(),
            agent: None,
            bridge: Some(bridge),
            window: None,
            frame_remembered: false,
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

    pub fn toggle_automatic(&self) {
        #[cfg(target_os = "macos")]
        if !self.fixture
            && let Some(bridge) = &self.bridge
        {
            bridge.send(spaceman::app::UiAction::ToggleAutomatic);
        }
    }

    pub fn open_accessibility(&self) {
        #[cfg(target_os = "macos")]
        if !self.fixture
            && let Some(bridge) = &self.bridge
        {
            bridge.send(spaceman::app::UiAction::OpenAccessibility);
        }
    }

    pub fn toggle_launch_at_login(&self) {
        #[cfg(target_os = "macos")]
        if !self.fixture
            && let Some(bridge) = &self.bridge
        {
            bridge.send(spaceman::app::UiAction::ToggleLaunchAtLogin);
        }
    }

    pub fn check_for_updates(&self) {
        #[cfg(target_os = "macos")]
        if !self.fixture
            && let Some(bridge) = &self.bridge
        {
            bridge.send(spaceman::app::UiAction::CheckForUpdates);
        }
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
    #[cfg(target_os = "macos")]
    if !ui.fixture
        && let Some(bridge) = &ui.bridge
    {
        bridge.set_waker(Arc::new(move || waker()));
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (ui, waker);
}

fn poll(ui: &mut Ui, _clock: f64, _tx: &mut Tx) -> bool {
    if !ui.fixture && !ui.frame_remembered {
        ui.frame_remembered = rune_mac::remember_frame("Spaceman");
    }
    #[cfg(target_os = "macos")]
    {
        let Some(bridge) = &ui.bridge else {
            return false;
        };
        let news = bridge.take_news();
        let mut changed = false;
        if let Some(snapshot) = news.snapshot {
            let agent = agent::Agent::from_snapshot(snapshot);
            if !ui.navigated && ui.agent.is_none() && !agent.accessibility {
                ui.pane = Pane::General;
            }
            ui.agent = Some(Some(agent));
            changed = true;
        }
        if news.config_changed && ui.drag.is_none() {
            ui.doc = Doc::load();
            ui.look_up_apps();
            changed = true;
        }
        changed
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

fn on_key(ui: &mut Ui, event: &KeyEvent, _tx: &mut Tx) -> bool {
    if event.key == rune::Key::Escape && (ui.drag.is_some() || ui.menu.is_some()) {
        ui.drag = None;
        ui.menu = None;
        return true;
    }
    false
}

fn on_close(ui: &mut Ui, _tx: &mut Tx) {
    ui.drag = None;
    ui.menu = None;
    if let Some(window) = &ui.window {
        window.set_dock_icon(false);
    }
}

#[cfg(target_os = "macos")]
fn on_reopen(ui: &mut Ui, _has_visible_windows: bool, _tx: &mut Tx) -> bool {
    if let Some(window) = &ui.window {
        host::bring_window_forward(window);
    }
    true
}

fn dev_stats() -> String {
    let usage = procwatch::usage();
    match (usage.footprint, usage.fds) {
        (Some(footprint), Some(fds)) => format!("{} MB, {fds} fds", footprint >> 20),
        (Some(footprint), None) => format!("{} MB", footprint >> 20),
        _ => String::new(),
    }
}

fn spec() -> AppSpec<Ui> {
    AppSpec {
        title: "Spaceman",
        size: WINDOW_SIZE,
        min_size: Some(WINDOW_SIZE),
        init: blank_fixture,
        view: view::view,
        scenes: playground::scenes(),
        fonts: rune_theme_opencode::FONTS,
        families: rune_theme_opencode::FAMILIES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
    }
}

fn base_app() -> rune::App<Ui> {
    let app = playground::attach(
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
            .on_close(on_close)
            .dev_stats(dev_stats)
            .standard_menus(),
    );
    let window = app.handle();
    window.update_if({
        let window = window.clone();
        move |ui, _| {
            ui.window = Some(window);
            false
        }
    });
    app
}

#[cfg(target_os = "macos")]
pub(crate) fn live_app(bridge: spaceman::app::UiBridge) -> rune::App<Ui> {
    base_app()
        .init(move || Ui::load_live(bridge))
        .on_reopen(on_reopen)
}

fn preview_mode() -> bool {
    rune::tooling::is_headless()
        || std::env::args()
            .skip(1)
            .any(|arg| matches!(arg.as_str(), "--playground" | "--help" | "-h"))
}

fn main() {
    if preview_mode() {
        base_app().run();
        return;
    }
    #[cfg(target_os = "macos")]
    if let Err(error) = host::run() {
        drop(std::io::Write::write_all(
            &mut std::io::stderr(),
            format!("Spaceman failed: {error}\n").as_bytes(),
        ));
        std::process::exit(1);
    }
    #[cfg(not(target_os = "macos"))]
    base_app().run();
}

// ---------------------------------------------------------------------------
// Scenes for headless stills (`--snap out.png --scene rules`) and Playground

pub(crate) type Scene = (&'static str, fn(&mut Ui));

pub(crate) const BASE_SCENES: &[Scene] = &[
    ("general", |ui| fixture(ui, Pane::General)),
    ("general-setup", |ui| {
        fixture(ui, Pane::General);
        if let Some(Some(agent)) = &mut ui.agent {
            agent.accessibility = false;
            agent.automatic = false;
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

pub(crate) fn blank_fixture() -> Ui {
    let mut ui = Ui {
        pane: Pane::Rules,
        navigated: false,
        doc: Doc::parse(include_str!("../fixtures/config.toml")),
        apps: apps::Catalog::default(),
        agent: None,
        #[cfg(target_os = "macos")]
        bridge: None,
        window: None,
        frame_remembered: true,
        drag: None,
        menu: None,
        running: Vec::new(),
        error: None,
        fixture: true,
    };
    fixture(&mut ui, Pane::Rules);
    ui
}

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
        last_error: None,
    }));
}
