//! Spaceman's single-process macOS agent (`spaceman-agent`): runs the window
//! manager, native status bar item, updater, Settings window, and Rune
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
mod updates;
mod view;

use std::sync::Arc;

pub use panes::Pane;
use rules::{Doc, Place};
use rune::{AppSpec, KeyEvent, PointerPress, Tx, Waker};
use spaceman::app::{UiAction, UiAgentSnapshot, UiBridge};

/// One fixed size: tiling window managers float such a window, and every pane fits it.
pub const WINDOW_SIZE: (f32, f32) = (860.0, 580.0);

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
    pub hud_preview: Option<hud::HudPreview>,
    frame_remembered: bool,
    pub tour: Option<onboarding::Tour>,
    pub drag: Option<map::Drag>,
    pub menu: Option<Menu>,
    pub running: Vec<apps::Running>,
    /// Every connected display, as Settings › Workspaces offers them.
    pub(crate) displays: Vec<panes::workspaces::ConnectedDisplay>,
    /// When the display list was last read, on Rune's clock.
    pub(crate) displays_at: Option<f64>,
    /// The last save that failed.
    pub error: Option<String>,
    /// Settings › Layout while a slider is held: the preview follows it, the file waits for
    /// the release.
    pub(crate) spacing_draft: Option<spaceman::spacing::Spacing>,
    /// Whether the four edge sliders show, once someone opened or closed them.
    pub(crate) per_edge: Option<bool>,
    /// Scenes render fixed data and ignore the live agent and file.
    pub(crate) fixture: bool,
}

impl Ui {
    #[cfg(target_os = "macos")]
    fn load_live(
        doc: Doc,
        bridge: UiBridge,
        initial_tour: Option<onboarding::Step>,
        accessibility: bool,
    ) -> Ui {
        let mut ui = Ui {
            pane: Pane::Rules,
            navigated: false,
            doc,
            apps: apps::Catalog::default(),
            agent: None,
            activity: activity::ActivityPane::default(),
            bridge: Some(bridge),
            window: None,
            hud_driver: None,
            hud_preview: None,
            frame_remembered: false,
            tour: None,
            drag: None,
            menu: None,
            running: Vec::new(),
            displays: Vec::new(),
            displays_at: None,
            error: None,
            spacing_draft: None,
            per_edge: None,
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
        // Re-running setup from Settings replaces a first-run tour, and that
        // tour's hold on automatic tiling ends with it.
        if self.tour.as_ref().is_some_and(|tour| !tour.flow.manual()) {
            onboarding::dismiss(self);
        }
        self.tour = Some(onboarding::Tour::manual(self));
    }

    pub fn select_pane(&mut self, pane: Pane) {
        self.pane = pane;
        self.navigated = true;
        self.menu = None;
        self.drag = None;
        if pane == Pane::Workspaces {
            // Read the displays before the next frame paints them.
            self.displays_at = None;
        }
    }

    /// Releases the first-run setup tour's hold on automatic tiling.
    pub fn finish_setup(&self) {
        self.send_action(UiAction::FinishSetup);
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

    /// Restores every window, then installs the ready update and relaunches.
    pub fn install_update(&self) {
        self.send_action(UiAction::InstallUpdate);
    }

    /// Switches to the update channel's release on offer.
    pub fn confirm_update_switch(&self) {
        self.send_action(UiAction::ConfirmUpdateSwitch);
    }

    /// Opens the release's download, for a copy that can't update itself.
    pub fn download_update(&self) {
        self.send_action(UiAction::DownloadUpdate);
    }

    /// Saves the channel, then checks it at once.
    pub fn set_update_channel(&mut self, channel: spaceman::config::UpdateChannel) {
        self.edit(|doc| doc.set_update_channel(channel));
        if self.error.is_none() {
            self.check_for_updates();
        }
    }

    /// Sets the Workspace HUD off (`None`) or on with `style` (`Some(style)`).
    pub fn set_hud(&mut self, mode: Option<spaceman::config::HudStyle>) {
        let enabled = mode.is_some();
        self.edit(|doc| {
            doc.set_hud(enabled);
            if let Some(style) = mode {
                doc.set_hud_style(style);
            }
        });
        if self.error.is_none() {
            if let Some(driver) = &self.hud_driver {
                driver.set_enabled(enabled);
                if let Some(style) = mode {
                    driver.set_style(style);
                }
            }
            self.send_action(UiAction::SetHud(enabled));
        }
    }

    /// Saves gaps, edge padding and the single-window width, then retiles so every display
    /// takes them at once.
    pub fn set_spacing(&mut self, spacing: spaceman::spacing::Spacing) {
        self.spacing_draft = None;
        if self.doc.spacing() == spacing {
            return;
        }
        self.edit(|doc| doc.set_spacing(spacing));
        if self.error.is_none() {
            self.send_action(UiAction::Retile);
        }
    }

    pub fn mark_wrong(&self) {
        self.send_action(UiAction::MarkWrong);
    }

    /// Names and icons for every app the rules mention, before the view needs them.
    pub(crate) fn look_up_apps(&mut self) {
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
    if !ui.fixture
        && let Some(bridge) = &ui.bridge
    {
        bridge.set_waker(Arc::new(move || waker()));
    }
}

fn poll(ui: &mut Ui, clock: f64, tx: &mut Tx) -> bool {
    if !ui.fixture && !ui.frame_remembered {
        ui.frame_remembered = rune_mac::remember_frame("Spaceman");
    }
    let mut changed = onboarding::poll(ui, tx);
    changed |= panes::workspaces::poll_displays(ui, clock);
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
            changed |= onboarding::poll(ui, tx);
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
            driver.set_style(ui.doc.hud_style());
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
        if ui.tour.is_some() {
            onboarding::dismiss(ui);
            return true;
        }
    }
    if ui.tour.is_none()
        && ui.hud_preview.is_none()
        && !event.mods.shift
        && !event.mods.alt
        && let rune::Key::Command(digit) = event.key
        && let Some(pane) = Pane::from_shortcut(digit)
    {
        ui.select_pane(pane);
        return true;
    }
    false
}

fn on_close(ui: &mut Ui, _tx: &mut Tx) {
    ui.drag = None;
    ui.menu = None;
    // Closing the setup window dismisses the tour, so a first-run tour starts
    // tiling now rather than holding it behind a window nobody can see.
    onboarding::dismiss(ui);
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
        init: playground::blank_fixture,
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
    let doc = Doc::load();
    let initial_hud = doc.hud();
    let initial_style = doc.hud_style();
    let hud_bridge = bridge.clone();
    let mut app = base_app()
        .init(move || Ui::load_live(doc, bridge, initial_tour, accessibility))
        .on_reopen(on_reopen);
    let hud_sink = Arc::new(hud::LiveHudSink::new(hud_bridge, app.handle()));
    let hud_driver = hud::install(&mut app, initial_hud, initial_style, hud_sink);
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
