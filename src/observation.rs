use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{AppInstanceId, PhysicalWindowId, Rect};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedApplication {
    pub id: AppInstanceId,
    pub bundle_id: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedWindow {
    pub id: PhysicalWindowId,
    pub frame: Rect,
    #[serde(default = "default_true")]
    pub resizable: bool,
    #[serde(skip)]
    pub title: Option<Arc<str>>,
}

impl std::fmt::Debug for ObservedWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservedWindow")
            .field("id", &self.id)
            .field("frame", &self.frame)
            .field("resizable", &self.resizable)
            .finish()
    }
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use crate::AppInstanceId;

    use super::*;

    const TITLE: &str = "Private document";

    fn observations(
        title: Option<Arc<str>>,
    ) -> (ObservedWindow, AppObservation, NativeObservation) {
        let window = ObservedWindow {
            id: PhysicalWindowId {
                app: AppInstanceId {
                    pid: 1,
                    launched_at: 1,
                },
                window_id: 1,
            },
            frame: Rect {
                x: 10,
                y: 20,
                width: 800,
                height: 600,
            },
            resizable: false,
            title,
        };
        let app = AppObservation::Complete {
            app: ObservedApplication {
                id: window.id.app.clone(),
                bundle_id: "org.example.observation-test".to_owned(),
            },
            focused: Some(window.id.clone()),
            windows: vec![window.clone()],
        };
        let native = NativeObservation {
            cycle: 1,
            mouse_down: false,
            applications: vec![app.clone()],
        };
        (window, app, native)
    }

    #[test]
    fn diagnostics_do_not_debug_window_titles() {
        for title in [None, Some(Arc::from(TITLE))] {
            let (window, app, native) = observations(title);
            for debug in [
                format!("{window:?}"),
                format!("{window:#?}"),
                format!("{app:?}"),
                format!("{app:#?}"),
                format!("{native:?}"),
                format!("{native:#?}"),
            ] {
                assert!(!debug.contains(TITLE), "Debug contains a window title");
                assert!(!debug.contains("title:"), "Debug contains a title field");
                for field in [
                    "ObservedWindow",
                    "id: PhysicalWindowId",
                    "pid: 1",
                    "launched_at: 1",
                    "window_id: 1",
                    "frame: Rect",
                    "x: 10",
                    "y: 20",
                    "width: 800",
                    "height: 600",
                    "resizable: false",
                ] {
                    assert!(debug.contains(field), "Debug lost {field}");
                }
            }
        }
    }

    #[test]
    fn diagnostics_do_not_serialize_window_titles() {
        let (window, app, native) = observations(Some(Arc::from(TITLE)));
        for json in [
            serde_json::to_string(&window).unwrap(),
            serde_json::to_string_pretty(&window).unwrap(),
            serde_json::to_string(&app).unwrap(),
            serde_json::to_string_pretty(&app).unwrap(),
            serde_json::to_string(&native).unwrap(),
            serde_json::to_string_pretty(&native).unwrap(),
        ] {
            assert!(!json.contains(TITLE), "JSON contains a window title");
            assert!(!json.contains("\"title\""), "JSON contains a title field");
        }
        let restored: ObservedWindow =
            serde_json::from_str(&serde_json::to_string(&window).unwrap()).unwrap();
        assert!(restored.title.is_none());
        assert_eq!(restored.id, window.id);
        assert_eq!(restored.frame, window.frame);
        assert_eq!(restored.resizable, window.resizable);
    }

    #[test]
    fn diagnostics_preserve_window_titles_in_memory() {
        let (window, app, native) = observations(Some(Arc::from(TITLE)));
        assert!(
            !format!("{window:?} {window:#?} {app:?} {app:#?} {native:?} {native:#?}").is_empty()
        );
        assert!(!serde_json::to_string(&native).unwrap().is_empty());
        assert!(window.title.as_deref() == Some(TITLE));
        for observation in [&app, &native.applications[0]] {
            let AppObservation::Complete { windows, .. } = observation else {
                panic!("expected complete observation");
            };
            assert!(windows[0].title.as_deref() == Some(TITLE));
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AppObservation {
    Complete {
        app: ObservedApplication,
        focused: Option<PhysicalWindowId>,
        windows: Vec<ObservedWindow>,
    },
    Unavailable {
        app: AppInstanceId,
        error: String,
    },
    Terminated {
        app: AppInstanceId,
    },
}

impl AppObservation {
    #[must_use]
    pub const fn app_id(&self) -> &AppInstanceId {
        match self {
            Self::Complete { app, .. } => &app.id,
            Self::Unavailable { app, .. } | Self::Terminated { app } => app,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NativeObservation {
    pub cycle: u64,
    pub mouse_down: bool,
    pub applications: Vec<AppObservation>,
}
