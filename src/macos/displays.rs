//! Display discovery: each connected screen's stable ID, name and visible frame.

use std::{collections::HashMap, ptr::NonNull};

use objc2::{MainThreadMarker, rc::autoreleasepool};
use objc2_app_kit::NSScreen;
use objc2_core_foundation::{CFRetained, CFUUID};
use objc2_foundation::{NSNumber, ns_string};

use crate::{DisplaySelector, Rect};

use super::SnapshotError;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *const CFUUID;
}

pub(super) fn main_output() -> Result<Rect, SnapshotError> {
    Ok(outputs()?.main().frame)
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct DisplayOutput {
    pub id: String,
    pub name: String,
    pub frame: Rect,
    pub is_main: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayOutputs {
    displays: Vec<DisplayOutput>,
    main_index: usize,
}

impl DisplayOutputs {
    pub fn new(displays: Vec<DisplayOutput>) -> Result<Self, SnapshotError> {
        let Some(main_index) = displays.iter().position(|display| display.is_main) else {
            return Err(SnapshotError::MainScreenUnavailable);
        };
        Ok(Self {
            displays,
            main_index,
        })
    }

    #[must_use]
    pub fn displays(&self) -> &[DisplayOutput] {
        &self.displays
    }

    #[must_use]
    pub fn main(&self) -> &DisplayOutput {
        &self.displays[self.main_index]
    }

    #[must_use]
    pub fn secondary(&self) -> Option<&DisplayOutput> {
        self.displays.iter().find(|display| !display.is_main)
    }

    #[must_use]
    pub fn resolve_exact(&self, selector: &DisplaySelector) -> Option<&DisplayOutput> {
        match selector {
            DisplaySelector::Main => Some(self.main()),
            DisplaySelector::Secondary => self.secondary(),
            DisplaySelector::Preferred(preferred) => self
                .displays
                .iter()
                .find(|display| display.id == *preferred || display.name == *preferred),
        }
    }

    #[must_use]
    pub fn resolve(&self, selector: &DisplaySelector) -> &DisplayOutput {
        self.resolve_exact(selector).unwrap_or_else(|| self.main())
    }

    /// The canonical `config.toml` selector for `display`: `main`, `secondary` (the first
    /// non-main display), or the display's own ID for every display past those two.
    #[must_use]
    pub fn selector_for(&self, display: &DisplayOutput) -> DisplaySelector {
        if display.is_main {
            DisplaySelector::Main
        } else if self
            .secondary()
            .is_some_and(|secondary| secondary.id == display.id)
        {
            DisplaySelector::Secondary
        } else {
            DisplaySelector::Preferred(display.id.clone())
        }
    }

    /// Every display left to right, then top to bottom, ties broken by ID.
    #[must_use]
    pub fn ordered(&self) -> Vec<&DisplayOutput> {
        let mut displays: Vec<_> = self.displays.iter().collect();
        displays.sort_by(|left, right| {
            (left.frame.x, left.frame.y, left.id.as_str()).cmp(&(
                right.frame.x,
                right.frame.y,
                right.id.as_str(),
            ))
        });
        displays
    }

    pub fn frames(&self) -> impl Iterator<Item = Rect> + '_ {
        self.displays.iter().map(|display| display.frame)
    }

    #[must_use]
    pub fn union(&self) -> Rect {
        self.frames().fold(self.main().frame, union_rect)
    }
}

pub fn outputs() -> Result<DisplayOutputs, SnapshotError> {
    autoreleasepool(|_| {
        let marker = MainThreadMarker::new().ok_or(SnapshotError::MainThreadRequired)?;
        let screens = NSScreen::screens(marker);
        let primary_height = (0..screens.count())
            .map(|index| screens.objectAtIndex(index).frame())
            .find(|frame| frame.origin.x == 0.0 && frame.origin.y == 0.0)
            .map(|frame| frame.size.height)
            .ok_or(SnapshotError::MainScreenUnavailable)?;
        let mut displays = Vec::with_capacity(screens.count());
        for index in 0..screens.count() {
            let screen = screens.objectAtIndex(index);
            let frame = screen.frame();
            let display_id =
                screen_display_id(&screen).ok_or(SnapshotError::MainScreenUnavailable)?;
            displays.push(DisplayOutput {
                id: display_uuid(display_id).unwrap_or_else(|| display_id.to_string()),
                name: screen.localizedName().to_string(),
                frame: screen_output(&screen, primary_height),
                is_main: frame.origin.x == 0.0 && frame.origin.y == 0.0,
            });
        }
        DisplayOutputs::new(displays)
    })
}

/// Each connected display's Core Graphics ID, keyed by the ID `outputs` gives it. Main thread
/// only; empty elsewhere.
#[must_use]
pub fn display_ids() -> HashMap<String, u32> {
    autoreleasepool(|_| {
        let Some(marker) = MainThreadMarker::new() else {
            return HashMap::new();
        };
        let screens = NSScreen::screens(marker);
        (0..screens.count())
            .filter_map(|index| screen_display_id(&screens.objectAtIndex(index)))
            .map(|display_id| {
                let id = display_uuid(display_id).unwrap_or_else(|| display_id.to_string());
                (id, display_id)
            })
            .collect()
    })
}

pub(super) fn screen_display_id(screen: &NSScreen) -> Option<u32> {
    let description = screen.deviceDescription();
    let number = description.objectForKey(ns_string!("NSScreenNumber"))?;
    number
        .downcast_ref::<NSNumber>()
        .map(NSNumber::unsignedIntValue)
}

fn display_uuid(display_id: u32) -> Option<String> {
    // SAFETY: `CGDisplayCreateUUIDFromDisplayID` returns null or a +1 retained `CFUUIDRef`.
    let raw = unsafe { CGDisplayCreateUUIDFromDisplayID(display_id) };
    // SAFETY: `raw` is non-null and +1 retained under the Core Foundation Create rule.
    let uuid = unsafe { CFRetained::from_raw(NonNull::new(raw.cast_mut())?) };
    CFUUID::new_string(None, Some(&uuid)).map(|uuid| uuid.to_string())
}

fn screen_output(screen: &NSScreen, primary_height: f64) -> Rect {
    let visible = screen.visibleFrame();
    Rect {
        x: visible.origin.x.round() as i32,
        y: (primary_height - visible.origin.y - visible.size.height).round() as i32,
        width: visible.size.width.round() as i32,
        height: visible.size.height.round() as i32,
    }
}

fn union_rect(left: Rect, right: Rect) -> Rect {
    let x = left.x.min(right.x);
    let y = left.y.min(right.y);
    Rect {
        x,
        y,
        width: (left.x + left.width).max(right.x + right.width) - x,
        height: (left.y + left.height).max(right.y + right.height) - y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(id: &str, name: &str, x: i32, is_main: bool) -> DisplayOutput {
        DisplayOutput {
            id: id.into(),
            name: name.into(),
            frame: Rect {
                x,
                y: 0,
                width: 100,
                height: 100,
            },
            is_main,
        }
    }

    fn three_displays() -> DisplayOutputs {
        DisplayOutputs::new(vec![
            display("built-in", "Built-in Retina Display", 0, true),
            display("studio-a", "Studio Display", 100, false),
            display("studio-b", "Studio Display", -100, false),
        ])
        .unwrap()
    }

    #[test]
    fn selector_for_names_main_secondary_and_every_further_display_by_id() {
        let outputs = three_displays();
        let selectors: Vec<String> = outputs
            .displays()
            .iter()
            .map(|display| outputs.selector_for(display).as_str().to_owned())
            .collect();
        assert_eq!(selectors, ["main", "secondary", "studio-b"]);
    }

    #[test]
    fn ordered_lists_displays_left_to_right() {
        let outputs = three_displays();
        let ids: Vec<&str> = outputs
            .ordered()
            .iter()
            .map(|display| display.id.as_str())
            .collect();
        assert_eq!(ids, ["studio-b", "built-in", "studio-a"]);
    }
}
