//! Spaceman's single-process macOS agent (`spaceman-agent`): runs the window
//! manager, native status bar item, Sparkle updater, Settings window, and Rune
//! Playground (`⇧⌘D` / `--playground`) on one AppKit + Rune event loop.

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: procwatch::alloc::Counting = procwatch::alloc::Counting::system();

mod activity;
mod apps;
mod controls;
#[cfg(target_os = "macos")]
mod host;
mod hud;
mod map;
mod onboarding;
mod panes;
mod playground;
mod rules;
mod view;

use std::sync::Arc;

use rules::{Doc, Place};
use rune::{AppSpec, KeyEvent, PointerPress, Tx, Waker};
use spaceman::{
    WorkspaceId,
    app::{UiAction, UiAgentSnapshot, UiBridge, UiHudEvent, UiLaunchAtLogin, UiUpdateStatus},
};

/// One fixed size: tiling window managers float such a window, and every pane fits it.
pub const WINDOW_SIZE: (f32, f32) = (860.0, 580.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    General,
    Rules,
    Workspaces,
    Shortcuts,
    Activity,
}

impl Pane {
    pub const ALL: [Pane; 5] = [
        Pane::General,
        Pane::Rules,
        Pane::Workspaces,
        Pane::Shortcuts,
        Pane::Activity,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Pane::General => "General",
            Pane::Rules => "Rules",
            Pane::Workspaces => "Workspaces",
            Pane::Shortcuts => "Shortcuts",
            Pane::Activity => "Activity",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Pane::General => "general",
            Pane::Rules => "rules",
            Pane::Workspaces => "workspaces",
            Pane::Shortcuts => "shortcuts",
            Pane::Activity => "activity",
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
    /// `None` until the runtime publishes its first snapshot.
    pub agent: Option<UiAgentSnapshot>,
    pub activity: activity::ActivityPane,
    bridge: Option<UiBridge>,
    window: Option<rune::AppHandle<Ui>>,
    hud_driver: Option<hud::HudDriver>,
    pub hud_preview: Option<UiHudEvent>,
    waker: Option<Waker>,
    frame_remembered: bool,
    pub tour: Option<onboarding::Tour>,
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
    fn load_live(
        bridge: UiBridge,
        initial_tour: Option<onboarding::Step>,
        accessibility: bool,
    ) -> Ui {
        let mut ui = Ui {
            pane: Pane::Rules,
            navigated: false,
            doc: Doc::load(),
            apps: apps::Catalog::default(),
            agent: None,
            activity: activity::ActivityPane::default(),
            bridge: Some(bridge),
            window: None,
            hud_driver: None,
            hud_preview: None,
            waker: None,
            frame_remembered: false,
            tour: None,
            drag: None,
            menu: None,
            running: Vec::new(),
            error: None,
            fixture: false,
        };
        ui.look_up_apps();
        if let Some(step) = initial_tour {
            ui.tour = Some(onboarding::Tour::from_launch(step, accessibility, &mut ui));
        }
        ui
    }

    pub fn start_tour(&mut self) {
        self.menu = None;
        self.drag = None;
        self.tour = Some(onboarding::Tour::manual(self));
    }

    pub fn select_pane(&mut self, pane: Pane) {
        self.pane = pane;
        self.navigated = true;
        self.menu = None;
        self.drag = None;
    }

    fn send_action(&self, action: UiAction) {
        if !self.fixture
            && let Some(bridge) = &self.bridge
        {
            bridge.send(action);
        }
    }

    pub fn toggle_automatic(&self) {
        self.send_action(UiAction::ToggleAutomatic);
    }

    pub fn open_accessibility(&self) {
        self.send_action(UiAction::OpenAccessibility);
    }

    pub fn toggle_launch_at_login(&self) {
        self.send_action(UiAction::ToggleLaunchAtLogin);
    }

    pub fn check_for_updates(&self) {
        self.send_action(UiAction::CheckForUpdates);
    }

    pub fn toggle_hud(&mut self) {
        let next = !self.doc.hud();
        self.edit(|doc| doc.set_hud(next));
        if self.error.is_none() {
            if let Some(driver) = &self.hud_driver {
                driver.set_enabled(next);
            }
            self.send_action(UiAction::SetHud(next));
        }
    }

    pub fn mark_wrong(&self) {
        self.send_action(UiAction::MarkWrong);
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
    ui.waker = Some(waker.clone());
    if !ui.fixture
        && let Some(bridge) = &ui.bridge
    {
        bridge.set_waker(Arc::new(move || waker()));
    }
}

fn poll(ui: &mut Ui, _clock: f64, _tx: &mut Tx) -> bool {
    if !ui.fixture && !ui.frame_remembered {
        ui.frame_remembered = rune_mac::remember_frame("Spaceman");
    }
    let mut changed = onboarding::poll(ui);
    changed |= ui.activity.poll(
        ui.pane == Pane::Activity && ui.tour.is_none() && ui.hud_preview.is_none(),
        ui.fixture,
    );
    let Some(bridge) = ui.bridge.clone() else {
        return changed;
    };
    let news = bridge.take_news();
    if let Some(snapshot) = news.snapshot {
        if let Some(driver) = &ui.hud_driver {
            driver.set_enabled(snapshot.hud);
        }
        if !ui.navigated && ui.agent.is_none() && !snapshot.accessibility && ui.tour.is_none() {
            ui.pane = Pane::General;
        }
        ui.agent = Some(snapshot);
        changed = true;
        if ui.tour.is_some() {
            changed |= onboarding::poll(ui);
        }
    }
    if let Some(hud_event) = news.hud
        && let Some(driver) = &ui.hud_driver
    {
        driver.trigger(hud_event);
    }
    if news.config_changed && ui.drag.is_none() {
        ui.doc = Doc::load();
        ui.look_up_apps();
        if let Some(driver) = &ui.hud_driver {
            driver.set_enabled(ui.doc.hud());
        }
        changed = true;
    }
    changed
}

fn on_key(ui: &mut Ui, event: &KeyEvent, _tx: &mut Tx) -> bool {
    if event.key == rune::Key::Escape {
        if ui.drag.is_some() || ui.menu.is_some() {
            ui.drag = None;
            ui.menu = None;
            return true;
        }
        if ui.tour.as_ref().is_some_and(|tour| tour.flow.manual()) {
            onboarding::input(ui, onboarding::Input::Dismiss);
            return true;
        }
    }
    if ui.tour.is_none() && ui.hud_preview.is_none() && !event.mods.shift && !event.mods.alt {
        let pane = match event.key {
            rune::Key::Command('1') => Some(Pane::General),
            rune::Key::Command('2') => Some(Pane::Rules),
            rune::Key::Command('3') => Some(Pane::Workspaces),
            rune::Key::Command('4') => Some(Pane::Shortcuts),
            rune::Key::Command('5') => Some(Pane::Activity),
            _ => None,
        };
        if let Some(pane) = pane {
            ui.select_pane(pane);
            return true;
        }
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
            .dev_stats(activity::process_vitals)
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
pub(crate) fn live_app(
    bridge: spaceman::app::UiBridge,
    initial_tour: Option<onboarding::Step>,
    accessibility: bool,
) -> rune::App<Ui> {
    let initial_hud = Doc::load().hud();
    let mut app = base_app()
        .init(move || Ui::load_live(bridge, initial_tour, accessibility))
        .on_reopen(on_reopen);
    let hud_driver = hud::install(&mut app, initial_hud);
    app.handle().update_if(move |ui, _| {
        ui.hud_driver = Some(hud_driver);
        false
    });
    app
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
    ("onboarding-welcome", |ui| {
        fixture(ui, Pane::Rules);
        let tour =
            onboarding::Tour::fixture(onboarding::Step::Welcome, false, false, Vec::new(), ui);
        onboarding::apply_fixture(ui, tour);
    }),
    ("onboarding-conflict", |ui| {
        fixture(ui, Pane::Rules);
        let tour = onboarding::Tour::fixture(
            onboarding::Step::Welcome,
            false,
            false,
            vec!["AeroSpace".into()],
            ui,
        );
        onboarding::apply_fixture(ui, tour);
    }),
    ("onboarding-setup", |ui| {
        fixture(ui, Pane::Rules);
        let tour = onboarding::Tour::fixture(onboarding::Step::Setup, false, false, Vec::new(), ui);
        onboarding::apply_fixture(ui, tour);
    }),
    ("onboarding-setup-waiting", |ui| {
        fixture(ui, Pane::Rules);
        let tour = onboarding::Tour::fixture(onboarding::Step::Setup, false, true, Vec::new(), ui);
        onboarding::apply_fixture(ui, tour);
    }),
    ("onboarding-setup-ready", |ui| {
        fixture(ui, Pane::Rules);
        let tour = onboarding::Tour::fixture(onboarding::Step::Setup, true, false, Vec::new(), ui);
        onboarding::apply_fixture(ui, tour);
    }),
    ("onboarding-rules", |ui| {
        fixture(ui, Pane::Rules);
        ui.doc = Doc::parse(
            "version = 1\n\n[[rules]]\nbundle_id = \"com.apple.Safari\"\naction = \"route\"\nworkspace = 1\n\n[[rules]]\nbundle_id = \"com.apple.MobileSMS\"\naction = \"route\"\nworkspace = 2\n\n[[rules]]\nbundle_id = \"com.apple.systempreferences\"\naction = \"float\"\n",
        );
        ui.look_up_apps();
        let tour = onboarding::Tour::fixture(onboarding::Step::Rules, true, false, Vec::new(), ui);
        onboarding::apply_fixture(ui, tour);
    }),
    ("onboarding-done", |ui| {
        fixture(ui, Pane::Rules);
        let tour = onboarding::Tour::fixture(onboarding::Step::Done, true, false, Vec::new(), ui);
        onboarding::apply_fixture(ui, tour);
    }),
    ("general", |ui| fixture(ui, Pane::General)),
    ("general-setup", |ui| {
        fixture(ui, Pane::General);
        if let Some(agent) = &mut ui.agent {
            agent.accessibility = false;
            agent.automatic = false;
            agent.launch_at_login = UiLaunchAtLogin::Off;
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
    ("activity", |ui| {
        fixture(ui, Pane::Activity);
        ui.activity = activity::ActivityPane::fixture_healthy();
    }),
    ("hud-switch", |ui| {
        fixture(ui, Pane::Rules);
        ui.hud_preview = Some(spaceman::app::UiHudEvent {
            workspace: spaceman::WorkspaceId::TWO,
            kind: spaceman::app::UiHudKind::Workspace,
            preview: spaceman::app::UiHudLayout::Split(vec![
                spaceman::Rect {
                    x: 0,
                    y: 0,
                    width: 860,
                    height: 1100,
                },
                spaceman::Rect {
                    x: 860,
                    y: 0,
                    width: 640,
                    height: 550,
                },
                spaceman::Rect {
                    x: 860,
                    y: 550,
                    width: 640,
                    height: 550,
                },
            ]),
            windows: 3,
        });
    }),
    ("hud-moved", |ui| {
        fixture(ui, Pane::Rules);
        ui.hud_preview = Some(spaceman::app::UiHudEvent {
            workspace: spaceman::WorkspaceId::FOUR,
            kind: spaceman::app::UiHudKind::Moved,
            preview: spaceman::app::UiHudLayout::Split(vec![
                spaceman::Rect {
                    x: 0,
                    y: 0,
                    width: 750,
                    height: 1100,
                },
                spaceman::Rect {
                    x: 750,
                    y: 0,
                    width: 750,
                    height: 1100,
                },
            ]),
            windows: 2,
        });
    }),
    ("hud-fullscreen", |ui| {
        fixture(ui, Pane::Rules);
        ui.hud_preview = Some(spaceman::app::UiHudEvent {
            workspace: spaceman::WorkspaceId::ONE,
            kind: spaceman::app::UiHudKind::Fullscreen { enabled: true },
            preview: spaceman::app::UiHudLayout::Fullscreen,
            windows: 2,
        });
    }),
];

pub(crate) fn blank_fixture() -> Ui {
    let mut ui = Ui {
        pane: Pane::Rules,
        navigated: false,
        doc: Doc::parse(include_str!("../fixtures/config.toml")),
        apps: apps::Catalog::default(),
        agent: None,
        activity: activity::ActivityPane::default(),
        bridge: None,
        window: None,
        hud_driver: None,
        hud_preview: None,
        waker: None,
        frame_remembered: true,
        tour: None,
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
    ui.tour = None;
    ui.hud_preview = None;
    ui.doc = Doc::parse(include_str!("../fixtures/config.toml"));
    ui.look_up_apps();
    ui.agent = Some(UiAgentSnapshot {
        accessibility: true,
        automatic: true,
        hud: true,
        launch_at_login: UiLaunchAtLogin::On,
        update: UiUpdateStatus::Idle,
        active_workspaces: vec![
            WorkspaceId::THREE,
            WorkspaceId::named('M').unwrap_or(WorkspaceId::SIX),
        ],
        last_error: None,
    });
}
