//! Pure state machine and owner-only persistence for Spaceman's 3-step setup tour.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use crate::controls::PermissionControl;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    Welcome,
    Setup,
    Done,
}

impl Step {
    pub const TOUR: [Self; 3] = [Self::Welcome, Self::Setup, Self::Done];

    pub fn position(self) -> usize {
        Self::TOUR
            .iter()
            .position(|step| *step == self)
            .unwrap_or(0)
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Welcome => "welcome",
            Self::Setup => "setup",
            Self::Done => "done",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SetupState {
    pub accessibility: bool,
    pub launch_at_login: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Input {
    Continue,
    Back,
    GrantAccessibility,
    ToggleLaunchAtLogin,
    Status(SetupState),
    Dismiss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    None,
    OpenAccessibility,
    ToggleLaunchAtLogin,
    /// Accessibility just turned on: hold for a beat so the checkmark settles, then advance.
    AdvanceSoon,
    Finish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct View {
    pub step: Step,
    /// `1` when moving forward, `-1` when going back, `0` on initial paint.
    pub direction: i8,
    /// Position among the dotted steps (`Welcome`, `Setup`, `Done`).
    pub index: usize,
    pub count: usize,
    pub accessibility: PermissionControl,
    pub launch_at_login: bool,
    pub ready: bool,
    pub can_go_back: bool,
    pub manual: bool,
}

#[derive(Clone, Debug)]
pub struct Flow {
    step: Step,
    direction: i8,
    status: SetupState,
    asked_accessibility: bool,
    advance_due: bool,
    manual: bool,
}

impl Flow {
    pub fn new(start: Step, status: SetupState, manual: bool) -> Self {
        let step = if !manual && start == Step::Setup && status.accessibility {
            // Resuming after macOS Quit & Reopen once Accessibility is granted lands on Done.
            Step::Done
        } else if start.position() > Step::Setup.position() && !status.accessibility {
            Step::Setup
        } else {
            start
        };
        Self {
            step,
            direction: 0,
            status,
            asked_accessibility: false,
            advance_due: false,
            manual,
        }
    }

    pub fn step(&self) -> Step {
        self.step
    }

    pub fn manual(&self) -> bool {
        self.manual
    }

    pub fn view(&self) -> View {
        let accessibility = if self.status.accessibility {
            PermissionControl::Allowed {
                animated: self.asked_accessibility,
            }
        } else if self.asked_accessibility {
            PermissionControl::Waiting
        } else {
            PermissionControl::Allow
        };
        View {
            step: self.step,
            direction: self.direction,
            index: self.step.position(),
            count: Step::TOUR.len(),
            accessibility,
            launch_at_login: self.status.launch_at_login,
            ready: self.status.accessibility,
            can_go_back: self.step != Step::Welcome,
            manual: self.manual,
        }
    }

    pub fn on(&mut self, input: Input) -> Effect {
        match input {
            Input::Continue => {
                if self.step == Step::Setup && !self.status.accessibility {
                    return Effect::None;
                }
                if self.step == Step::Done {
                    return Effect::Finish;
                }
                self.direction = 1;
                self.advance_due = false;
                self.step = Step::TOUR[self.step.position() + 1];
                Effect::None
            }
            Input::Back => {
                let pos = self.step.position();
                if pos == 0 {
                    return Effect::None;
                }
                self.direction = -1;
                self.advance_due = false;
                self.step = Step::TOUR[pos - 1];
                Effect::None
            }
            Input::GrantAccessibility => {
                if self.step == Step::Setup && !self.status.accessibility {
                    self.asked_accessibility = true;
                    Effect::OpenAccessibility
                } else {
                    Effect::None
                }
            }
            Input::ToggleLaunchAtLogin => Effect::ToggleLaunchAtLogin,
            Input::Status(next) => {
                let was_ready = self.status.accessibility;
                self.status = next;
                if self.step == Step::Setup && !was_ready && next.accessibility {
                    self.advance_due = true;
                }
                if std::mem::take(&mut self.advance_due) && self.step == Step::Setup {
                    Effect::AdvanceSoon
                } else {
                    Effect::None
                }
            }
            // Dismissing ends the tour whether it was the first-run setup or a
            // manual re-open; finishing a first-run tour is what starts tiling.
            Input::Dismiss => Effect::Finish,
        }
    }
}

pub fn launch_step(accessibility_trusted: bool) -> Option<Step> {
    let support = spaceman::config::application_support_path("onboarding-complete")?;
    let directory = support.parent()?;
    launch_step_at(directory, accessibility_trusted)
}

pub fn record_step(step: Step) {
    if let Some(path) = spaceman::config::application_support_path("onboarding-step")
        && let Some(directory) = path.parent()
    {
        drop(record_step_at(directory, step));
    }
}

pub fn record_completion() {
    if let Some(path) = spaceman::config::application_support_path("onboarding-complete")
        && let Some(directory) = path.parent()
    {
        drop(record_completion_at(directory));
    }
}

/// Decides whether a launch opens into the setup tour, and at which step.
/// Existing trusted installs with no in-progress tour record completion silently
/// so unattended `spaceman-updater` restarts stay windowless.
pub(crate) fn launch_step_at(directory: &Path, accessibility_trusted: bool) -> Option<Step> {
    if completion_recorded_at(directory) {
        return None;
    }
    let saved = saved_step_at(directory);
    if !accessibility_trusted || saved.is_some() {
        return Some(saved.unwrap_or(Step::Welcome));
    }
    drop(record_completion_at(directory));
    None
}

pub(crate) fn completion_recorded_at(directory: &Path) -> bool {
    directory.join("onboarding-complete").is_file()
}

pub(crate) fn saved_step_at(directory: &Path) -> Option<Step> {
    match fs::read_to_string(directory.join("onboarding-step"))
        .ok()?
        .trim()
    {
        "setup" => Some(Step::Setup),
        // Step::Rules was removed in 0.3.2; an in-progress tour from before then
        // has already granted Accessibility, so it resumes at Done.
        "rules" => Some(Step::Done),
        "done" => Some(Step::Done),
        _ => None,
    }
}

pub(crate) fn record_step_at(directory: &Path, step: Step) -> Result<(), String> {
    write_owner_only(&directory.join("onboarding-step"), step.key().as_bytes())
}

pub(crate) fn record_completion_at(directory: &Path) -> Result<(), String> {
    let temp = directory.join(".onboarding-complete.tmp");
    let dest = directory.join("onboarding-complete");
    write_owner_only(&temp, b"1\n")?;
    fs::rename(&temp, &dest).map_err(|error| error.to_string())?;
    drop(fs::remove_file(directory.join("onboarding-step")));
    Ok(())
}

fn write_owner_only(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    file.write_all(contents)
        .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}
