//! Setup tour orchestration, starter app discovery, and live polling.

mod flow;
#[cfg(test)]
mod tests;
mod view;

use std::{
    thread,
    time::{Duration, Instant},
};

pub use flow::{Effect, Flow, Input, SetupState, Step, launch_step};
pub use view::{apply_fixture, overlay};

use crate::{Ui, agent::LaunchAtLogin, apps};

const ADVANCE_BEAT: Duration = Duration::from_millis(700);
const STARTER_LIMIT: usize = 4;

const FALLBACK_STARTERS: &[(&str, &str)] = &[
    ("com.apple.Safari", "Safari"),
    ("com.apple.MobileSMS", "Messages"),
    ("com.apple.Terminal", "Terminal"),
    ("com.apple.systempreferences", "System Settings"),
];

#[derive(Clone, Debug)]
pub struct StarterApp {
    pub bundle_id: String,
    pub name: String,
}

pub struct Tour {
    pub flow: Flow,
    pub conflicts: Vec<String>,
    pub starter_apps: Vec<StarterApp>,
    advance_at: Option<Instant>,
    completed: bool,
}

impl Tour {
    pub fn from_launch(step: Step, accessibility: bool, ui: &mut Ui) -> Self {
        let status = SetupState {
            accessibility,
            launch_at_login: ui_launch_at_login(ui),
        };
        let starter_apps = live_starters(ui);
        Self {
            flow: Flow::new(step, status, false),
            conflicts: apps::conflicting_window_managers(),
            starter_apps,
            advance_at: None,
            completed: false,
        }
    }

    pub fn manual(ui: &mut Ui) -> Self {
        let status = ui_setup_state(ui);
        let starter_apps = live_starters(ui);
        Self {
            flow: Flow::new(Step::Welcome, status, true),
            conflicts: apps::conflicting_window_managers(),
            starter_apps,
            advance_at: None,
            completed: true,
        }
    }

    pub fn fixture(
        step: Step,
        accessibility: bool,
        asked: bool,
        conflicts: Vec<String>,
        ui: &mut Ui,
    ) -> Self {
        if let Some(Some(agent)) = &mut ui.agent {
            agent.accessibility = accessibility;
            agent.automatic = accessibility;
        }
        let status = SetupState {
            accessibility,
            launch_at_login: ui_launch_at_login(ui),
        };
        let mut flow = Flow::new(Step::Welcome, status, false);
        while flow.step() != step {
            if flow.step() == Step::Setup && !accessibility {
                // Force fixture into later step only when accessibility is true.
                break;
            }
            flow.on(Input::Continue);
        }
        if step == Step::Setup && asked && !accessibility {
            flow.on(Input::GrantAccessibility);
        }
        let starter_apps = FALLBACK_STARTERS
            .iter()
            .map(|&(bundle_id, name)| {
                let cached = ui.apps.get(bundle_id);
                StarterApp {
                    bundle_id: bundle_id.to_owned(),
                    name: if cached.icon.is_some() {
                        cached.name
                    } else {
                        name.to_owned()
                    },
                }
            })
            .collect();
        Self {
            flow,
            conflicts,
            starter_apps,
            advance_at: None,
            completed: false,
        }
    }
}

pub fn poll(ui: &mut Ui) -> bool {
    let status = ui_setup_state(ui);
    let live = !ui.fixture;
    let Some(tour) = &mut ui.tour else {
        return false;
    };
    let before = tour.flow.view();
    let step_before = before.step;
    let mut effect = tour.flow.on(Input::Status(status));
    if tour
        .advance_at
        .is_some_and(|at| at.elapsed() >= ADVANCE_BEAT)
    {
        tour.advance_at = None;
        if tour.flow.step() == Step::Setup {
            effect = tour.flow.on(Input::Continue);
        }
    }
    if status.accessibility && !tour.completed {
        tour.completed = true;
        if live {
            flow::record_completion();
        }
    }
    apply(ui, effect);
    remember_step(ui, step_before);
    let after = ui.tour.as_ref().map(|tour| tour.flow.view());
    after.as_ref() != Some(&before)
}

pub fn input(ui: &mut Ui, input: Input) {
    let Some(tour) = &mut ui.tour else {
        return;
    };
    let step_before = tour.flow.step();
    let effect = tour.flow.on(input);
    apply(ui, effect);
    remember_step(ui, step_before);
}

fn remember_step(ui: &Ui, before: Step) {
    if !ui.fixture
        && let Some(tour) = &ui.tour
        && !tour.flow.manual()
        && !tour.completed
        && tour.flow.step() != before
    {
        flow::record_step(tour.flow.step());
    }
}

fn apply(ui: &mut Ui, effect: Effect) {
    match effect {
        Effect::None => {}
        Effect::OpenAccessibility => ui.open_accessibility(),
        Effect::ToggleLaunchAtLogin => {
            if ui.fixture {
                if let Some(Some(agent)) = &mut ui.agent {
                    agent.launch_at_login = match agent.launch_at_login {
                        LaunchAtLogin::On | LaunchAtLogin::NeedsApproval => LaunchAtLogin::Off,
                        LaunchAtLogin::Off | LaunchAtLogin::Failed(_) => LaunchAtLogin::On,
                    };
                }
                let status = ui_setup_state(ui);
                if let Some(tour) = &mut ui.tour {
                    tour.flow.on(Input::Status(status));
                }
            } else {
                ui.toggle_launch_at_login();
            }
        }
        Effect::AdvanceSoon => {
            if let Some(tour) = &mut ui.tour {
                tour.advance_at = Some(Instant::now());
            }
            if let Some(waker) = ui.waker.clone() {
                thread::spawn(move || {
                    thread::sleep(ADVANCE_BEAT + Duration::from_millis(15));
                    waker();
                });
            }
        }
        Effect::Finish => {
            if !ui.fixture {
                flow::record_completion();
            }
            ui.tour = None;
        }
    }
}

fn ui_setup_state(ui: &Ui) -> SetupState {
    let accessibility = ui
        .agent
        .clone()
        .flatten()
        .is_some_and(|agent| agent.accessibility);
    SetupState {
        accessibility,
        launch_at_login: ui_launch_at_login(ui),
    }
}

fn ui_launch_at_login(ui: &Ui) -> bool {
    ui.agent.clone().flatten().is_some_and(|agent| {
        matches!(
            agent.launch_at_login,
            LaunchAtLogin::On | LaunchAtLogin::NeedsApproval
        )
    })
}

fn live_starters(ui: &mut Ui) -> Vec<StarterApp> {
    let mut seen = Vec::new();
    let mut out = Vec::new();

    for app in apps::running() {
        if out.len() >= STARTER_LIMIT {
            break;
        }
        if apps::window_manager_name(Some(&app.bundle_id), None).is_some() {
            continue;
        }
        seen.push(app.bundle_id.clone());
        ui.apps.get(&app.bundle_id);
        out.push(StarterApp {
            bundle_id: app.bundle_id,
            name: app.name,
        });
    }

    for rule in ui.doc.rules() {
        if out.len() >= STARTER_LIMIT {
            break;
        }
        let Some(bundle_id) = rule.bundle_id else {
            continue;
        };
        if rule.title.is_some() || seen.contains(&bundle_id) {
            continue;
        }
        let cached = ui.apps.get(&bundle_id);
        seen.push(bundle_id.clone());
        out.push(StarterApp {
            bundle_id,
            name: cached.name,
        });
    }

    for &(bundle_id, fallback_name) in FALLBACK_STARTERS {
        if out.len() >= STARTER_LIMIT {
            break;
        }
        if seen.iter().any(|existing| existing == bundle_id) {
            continue;
        }
        let cached = ui.apps.get(bundle_id);
        seen.push(bundle_id.to_owned());
        out.push(StarterApp {
            bundle_id: bundle_id.to_owned(),
            name: if cached.icon.is_some() {
                cached.name
            } else {
                fallback_name.to_owned()
            },
        });
    }

    out
}
