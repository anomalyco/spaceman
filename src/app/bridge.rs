use std::sync::{Arc, Mutex, mpsc::Sender};

use crate::{Axis, Rect, WorkspaceId, login_item, sparkle};

use super::{Action, WorkerAction};

/// Normalized preview coordinate space for HUD split rectangles.
pub const HUD_PREVIEW_RECT: Rect = Rect {
    x: 0,
    y: 0,
    width: 1500,
    height: 1100,
};

/// Actions the in-process Rune window can send to the agent loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAction {
    ToggleAutomatic,
    OpenAccessibility,
    CheckForUpdates,
    ToggleLaunchAtLogin,
    SetHud(bool),
    MarkWrong,
}

impl From<UiAction> for Action {
    fn from(action: UiAction) -> Self {
        match action {
            UiAction::ToggleAutomatic => Self::ToggleAutomatic,
            UiAction::OpenAccessibility => Self::OpenAccessibility,
            UiAction::CheckForUpdates => Self::CheckForUpdates,
            UiAction::ToggleLaunchAtLogin => Self::ToggleLaunchAtLogin,
            UiAction::SetHud(enabled) => Self::SetHud(enabled),
            UiAction::MarkWrong => Self::Worker(WorkerAction::MarkWrong),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiHudKind {
    Workspace,
    Moved,
    Layout { axis: Axis },
    Fullscreen { enabled: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiHudLayout {
    Split(Vec<Rect>),
    Fullscreen,
}

/// Title-free Workspace HUD event emitted on explicit user actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiHudEvent {
    pub workspace: WorkspaceId,
    pub kind: UiHudKind,
    pub preview: UiHudLayout,
    pub windows: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum UiLaunchAtLogin {
    #[default]
    Off,
    On,
    NeedsApproval,
    Failed(String),
}

impl UiLaunchAtLogin {
    pub(super) fn from_result(result: Result<login_item::Status, String>) -> Self {
        match result {
            Ok(login_item::Status::Enabled) => Self::On,
            Ok(login_item::Status::Disabled) => Self::Off,
            Ok(login_item::Status::RequiresApproval) => Self::NeedsApproval,
            Err(error) => Self::Failed(error),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiUpdateStatus {
    #[default]
    Unavailable,
    Idle,
    Checking,
    UpToDate,
    Available,
}

impl From<sparkle::UpdateStatus> for UiUpdateStatus {
    fn from(status: sparkle::UpdateStatus) -> Self {
        match status {
            sparkle::UpdateStatus::Unavailable => Self::Unavailable,
            sparkle::UpdateStatus::Idle => Self::Idle,
            sparkle::UpdateStatus::Checking => Self::Checking,
            sparkle::UpdateStatus::UpToDate => Self::UpToDate,
            sparkle::UpdateStatus::UpdateAvailable => Self::Available,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiAgentSnapshot {
    pub accessibility: bool,
    pub automatic: bool,
    pub hud: bool,
    pub launch_at_login: UiLaunchAtLogin,
    pub update: UiUpdateStatus,
    pub active_workspaces: Vec<WorkspaceId>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiNews {
    pub snapshot: Option<UiAgentSnapshot>,
    pub hud: Option<UiHudEvent>,
    pub config_changed: bool,
}

#[derive(Default)]
struct SharedState {
    last: Option<UiAgentSnapshot>,
    pending_snapshot: Option<UiAgentSnapshot>,
    pending_hud: Option<UiHudEvent>,
    config_changed: bool,
    waker: Option<Arc<dyn Fn() + Send + Sync>>,
}

/// Thread-safe bridge between the main-thread agent pump and the Rune window.
#[derive(Clone)]
pub struct UiBridge {
    actions: Sender<Action>,
    shared: Arc<Mutex<SharedState>>,
}

impl UiBridge {
    pub(super) fn new(actions: Sender<Action>) -> Self {
        Self {
            actions,
            shared: Arc::new(Mutex::new(SharedState::default())),
        }
    }

    pub fn set_waker(&self, waker: Arc<dyn Fn() + Send + Sync>) {
        let mut shared = self
            .shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if shared.pending_snapshot.is_none() {
            shared.pending_snapshot = shared.last.clone();
        }
        let has_news = shared.pending_snapshot.is_some()
            || shared.pending_hud.is_some()
            || shared.config_changed;
        shared.waker = Some(Arc::clone(&waker));
        drop(shared);
        if has_news {
            waker();
        }
    }

    #[must_use]
    pub fn take_news(&self) -> UiNews {
        let mut shared = self
            .shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        UiNews {
            snapshot: shared.pending_snapshot.take(),
            hud: shared.pending_hud.take(),
            config_changed: std::mem::take(&mut shared.config_changed),
        }
    }

    pub fn send(&self, action: UiAction) {
        drop(self.actions.send(action.into()));
    }

    pub(super) fn publish_hud(&self, event: UiHudEvent) {
        let waker = {
            let mut shared = self
                .shared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            shared.pending_hud = Some(event);
            shared.waker.clone()
        };
        if let Some(waker) = waker {
            waker();
        }
    }

    pub(super) fn publish(&self, snapshot: UiAgentSnapshot, config_changed: bool) {
        let waker = {
            let mut shared = self
                .shared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let snapshot_changed = shared.last.as_ref() != Some(&snapshot);
            if !snapshot_changed && !config_changed {
                return;
            }
            if snapshot_changed {
                shared.last = Some(snapshot.clone());
                shared.pending_snapshot = Some(snapshot);
            }
            shared.config_changed |= config_changed;
            shared.waker.clone()
        };
        if let Some(waker) = waker {
            waker();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::channel,
    };

    use super::*;

    #[test]
    fn deduplicates_identical_snapshots_and_wakes_late_subscriber() {
        let (sender, receiver) = channel();
        let bridge = UiBridge::new(sender);
        let wakes = Arc::new(AtomicUsize::new(0));

        let initial = UiAgentSnapshot {
            accessibility: true,
            automatic: true,
            hud: true,
            launch_at_login: UiLaunchAtLogin::On,
            update: UiUpdateStatus::Idle,
            active_workspaces: vec![WorkspaceId::ONE],
            last_error: None,
        };
        bridge.publish(initial.clone(), false);

        let wakes_for_hook = Arc::clone(&wakes);
        bridge.set_waker(Arc::new(move || {
            wakes_for_hook.fetch_add(1, Ordering::SeqCst);
        }));
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        assert_eq!(
            bridge.take_news(),
            UiNews {
                snapshot: Some(initial.clone()),
                hud: None,
                config_changed: false,
            }
        );

        // Identical snapshot without config change does not wake again.
        bridge.publish(initial.clone(), false);
        assert_eq!(wakes.load(Ordering::SeqCst), 1);

        // Config change wakes even when snapshot is unchanged.
        bridge.publish(initial, true);
        assert_eq!(wakes.load(Ordering::SeqCst), 2);
        assert_eq!(
            bridge.take_news(),
            UiNews {
                snapshot: None,
                hud: None,
                config_changed: true,
            }
        );

        let hud_event = UiHudEvent {
            workspace: WorkspaceId::ONE,
            kind: UiHudKind::Workspace,
            preview: UiHudLayout::Fullscreen,
            windows: 2,
        };
        bridge.publish_hud(hud_event.clone());
        assert_eq!(wakes.load(Ordering::SeqCst), 3);
        assert_eq!(
            bridge.take_news(),
            UiNews {
                snapshot: None,
                hud: Some(hud_event),
                config_changed: false,
            }
        );

        bridge.send(UiAction::ToggleAutomatic);
        assert_eq!(receiver.try_recv().ok(), Some(Action::ToggleAutomatic));
    }
}
