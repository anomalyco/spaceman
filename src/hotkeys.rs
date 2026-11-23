mod delivery;

use std::{collections::BTreeMap, ffi::c_void, ptr, sync::mpsc::Sender};

pub(crate) use delivery::DeliveryWatch;

use crate::{
    Direction, ResizeDirection, WorkspaceId,
    app::{Action, WorkerAction},
    keyboard_layout::{LayoutFingerprint, LayoutSnapshot, LogicalKey, SemanticKey},
};

const EVENT_CLASS_KEYBOARD: u32 = u32::from_be_bytes(*b"keyb");
const EVENT_HOTKEY_PRESSED: u32 = 5;
pub(crate) const COMMAND_KEY: u32 = 1 << 8;
pub(crate) const SHIFT_KEY: u32 = 1 << 9;
pub(crate) const OPTION_KEY: u32 = 1 << 11;
pub(crate) const CONTROL_KEY: u32 = 1 << 12;
const SIGNATURE: u32 = u32::from_be_bytes(*b"SPMN");

#[repr(C)]
struct EventTypeSpec {
    event_class: u32,
    event_kind: u32,
}

#[repr(C)]
struct EventHotKeyId {
    signature: u32,
    id: u32,
}

type EventHandler = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn InstallEventHandler(
        target: *mut c_void,
        handler: EventHandler,
        event_type_count: u32,
        event_types: *const EventTypeSpec,
        user_data: *mut c_void,
        handler_ref: *mut *mut c_void,
    ) -> i32;
    fn RemoveEventHandler(handler_ref: *mut c_void) -> i32;
    fn GetApplicationEventTarget() -> *mut c_void;
    fn RegisterEventHotKey(
        key_code: u32,
        modifiers: u32,
        hotkey_id: EventHotKeyId,
        target: *mut c_void,
        options: u32,
        hotkey_ref: *mut *mut c_void,
    ) -> i32;
    fn UnregisterEventHotKey(hotkey_ref: *mut c_void) -> i32;
    fn GetEventParameter(
        event: *mut c_void,
        name: u32,
        desired_type: u32,
        actual_type: *mut u32,
        buffer_size: u32,
        actual_size: *mut u32,
        data: *mut c_void,
    ) -> i32;
}

pub struct Hotkeys {
    registrations: Vec<*mut c_void>,
    handler: *mut c_void,
    context: Option<Box<HandlerContext>>,
    layout_fingerprint: LayoutFingerprint,
    /// A hotkey's ID is its index here.
    bindings: Vec<Registration>,
}

/// Read by the Carbon handler.
struct HandlerContext {
    sender: Sender<Action>,
}

#[derive(Debug)]
pub(crate) struct HotkeyPlan {
    registrations: Vec<Registration>,
    layout_fingerprint: LayoutFingerprint,
    /// Shortcuts this keyboard layout could not map. Each one is skipped on its own; the
    /// rest of the plan, and automatic tiling, keep running.
    warnings: Vec<String>,
}

#[derive(Clone, Debug)]
struct Registration {
    key_code: u32,
    modifiers: u32,
    action: WorkerAction,
    description: String,
}

impl HotkeyPlan {
    pub(crate) fn current(enabled: &[WorkspaceId]) -> Result<Self, String> {
        let layout = LayoutSnapshot::current().map_err(|error| error.to_string())?;
        Self::from_layout(enabled, &layout)
    }

    fn from_layout(enabled: &[WorkspaceId], layout: &LayoutSnapshot) -> Result<Self, String> {
        let mut plan = PlanBuilder::new(layout, 14 + enabled.len() * 2);
        let claimed_letters = enabled
            .iter()
            .filter_map(|workspace| workspace.letter().map(|letter| letter.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        for workspace in WorkspaceId::all().filter(|workspace| enabled.contains(workspace)) {
            let key = match (workspace.number(), workspace.letter()) {
                (Some(number), _) => LogicalKey::Semantic(SemanticKey::Workspace(number)),
                (_, Some(letter)) => LogicalKey::Character(letter.to_ascii_lowercase()),
                _ => unreachable!("workspace ID is always valid"),
            };
            plan.push(
                key,
                OPTION_KEY,
                WorkerAction::SwitchWorkspace(workspace),
                format!("switch workspace {workspace}"),
            );
            plan.push(
                key,
                OPTION_KEY | SHIFT_KEY,
                WorkerAction::MoveFocused(workspace),
                format!("move window to workspace {workspace}"),
            );
        }
        for (character, direction, name) in [
            ('h', Direction::Left, "left"),
            ('j', Direction::Down, "down"),
            ('k', Direction::Up, "up"),
            ('l', Direction::Right, "right"),
        ] {
            if claimed_letters.contains(&character) {
                continue;
            }
            plan.push(
                LogicalKey::Character(character),
                OPTION_KEY,
                WorkerAction::Focus(direction),
                format!("focus {name}"),
            );
            plan.push(
                LogicalKey::Character(character),
                OPTION_KEY | SHIFT_KEY,
                WorkerAction::Reorder(direction),
                format!("move window {name}"),
            );
        }
        for (key, modifiers, action, description) in [
            (
                LogicalKey::Character('f'),
                OPTION_KEY | SHIFT_KEY,
                WorkerAction::ToggleFullscreenLayout,
                "toggle fullscreen layout",
            ),
            (
                LogicalKey::Character('/'),
                OPTION_KEY,
                WorkerAction::ToggleLayout,
                "toggle layout",
            ),
            (
                LogicalKey::Character('`'),
                OPTION_KEY,
                WorkerAction::CycleFocus,
                "cycle workspace windows",
            ),
            (
                LogicalKey::Semantic(SemanticKey::Tab),
                OPTION_KEY,
                WorkerAction::WorkspaceBackAndForth,
                "workspace back-and-forth",
            ),
            (
                LogicalKey::Character('-'),
                OPTION_KEY,
                WorkerAction::Resize(ResizeDirection::Shrink),
                "shrink layout",
            ),
            (
                LogicalKey::Character('='),
                OPTION_KEY,
                WorkerAction::Resize(ResizeDirection::Grow),
                "grow layout",
            ),
        ] {
            if matches!(key, LogicalKey::Character(character) if claimed_letters.contains(&character))
            {
                continue;
            }
            plan.push(key, modifiers, action, description);
        }

        if plan.registrations.is_empty() {
            return Err(match plan.warnings.first() {
                Some(warning) => format!("no shortcut could be planned: {warning}"),
                None => "no shortcut could be planned".into(),
            });
        }
        Ok(Self {
            registrations: plan.registrations,
            layout_fingerprint: layout.fingerprint().clone(),
            warnings: plan.warnings,
        })
    }

    pub(crate) fn matches(&self, hotkeys: &Hotkeys) -> bool {
        self.layout_fingerprint == hotkeys.layout_fingerprint
    }
}

/// Builds a plan in priority order, keeping the first shortcut that claims a chord and
/// skipping later ones that would collide. Unusual layouts (Italian, AZERTY) reset two
/// characters onto one physical key with different Shift states, and that must cost one
/// shortcut rather than every shortcut and automatic tiling.
struct PlanBuilder<'a> {
    layout: &'a LayoutSnapshot,
    registrations: Vec<Registration>,
    occupied: BTreeMap<(u32, u32), String>,
    /// Shortcuts this layout could not map, kept for diagnosis instead of pausing the agent.
    warnings: Vec<String>,
}

impl<'a> PlanBuilder<'a> {
    fn new(layout: &'a LayoutSnapshot, capacity: usize) -> Self {
        Self {
            layout,
            registrations: Vec::with_capacity(capacity),
            occupied: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }

    fn push(
        &mut self,
        key: LogicalKey,
        modifiers: u32,
        action: WorkerAction,
        description: impl Into<String>,
    ) {
        let description = description.into();
        let candidates = match self.layout.candidates(key) {
            Ok(candidates) => candidates,
            Err(error) => {
                self.warnings
                    .push(format!("skipped {description}: {error}"));
                return;
            }
        };
        for candidate in candidates {
            let modifiers = modifiers | if candidate.shift { SHIFT_KEY } else { 0 };
            let chord = (candidate.key_code, modifiers);
            if self.occupied.contains_key(&chord) {
                continue;
            }
            self.occupied.insert(chord, description.clone());
            self.registrations.push(Registration {
                key_code: chord.0,
                modifiers,
                action,
                description,
            });
            return;
        }
        self.warnings.push(format!(
            "skipped {description}: every key it can use is already taken"
        ));
    }
}

impl Hotkeys {
    pub fn install(sender: Sender<Action>, plan: HotkeyPlan) -> Result<Self, String> {
        let context = Box::new(HandlerContext { sender });
        let context_pointer = (&raw const *context).cast_mut();
        let event_type = EventTypeSpec {
            event_class: EVENT_CLASS_KEYBOARD,
            event_kind: EVENT_HOTKEY_PRESSED,
        };
        let mut handler = ptr::null_mut();
        // SAFETY: `event_type` lives across installation, `context` stays heap-pinned in `self`
        // (or is leaked if removal fails), and `handler` is a valid out-pointer.
        let status = unsafe {
            InstallEventHandler(
                GetApplicationEventTarget(),
                hotkey_handler,
                1,
                &raw const event_type,
                context_pointer.cast(),
                &raw mut handler,
            )
        };
        if status != 0 {
            return Err(format!("failed to install hotkey handler ({status})"));
        }

        let HotkeyPlan {
            registrations,
            layout_fingerprint,
            warnings,
        } = plan;
        let mut hotkeys = Self {
            registrations: Vec::with_capacity(registrations.len()),
            handler,
            context: Some(context),
            layout_fingerprint,
            bindings: Vec::new(),
        };
        // Shortcuts the layout already skipped are the first reasons a plan can be thin.
        let mut failures = warnings;
        for registration in registrations {
            // A chord another app already owns (or macOS refuses) costs only that shortcut.
            let id = u32::try_from(hotkeys.bindings.len())
                .map_err(|_| "too many hotkeys to register".to_owned())?;
            match hotkeys.register(registration.key_code, registration.modifiers, id) {
                Ok(()) => hotkeys.bindings.push(registration),
                Err(error) => {
                    failures.push(format!("skipped {}: {error}", registration.description))
                }
            }
        }
        if hotkeys.bindings.is_empty() {
            return Err(match failures.first() {
                Some(failure) => failure.clone(),
                None => "no shortcut could be registered".into(),
            });
        }
        Ok(hotkeys)
    }

    pub(crate) fn len(&self) -> usize {
        self.bindings.len()
    }

    /// The action and description of the hotkey Carbon delivered as `id`.
    pub(crate) fn binding(&self, id: u32) -> Option<(&WorkerAction, &str)> {
        let registration = self.bindings.get(usize::try_from(id).ok()?)?;
        Some((&registration.action, &registration.description))
    }

    /// The hotkey registered for this key and these Carbon modifiers.
    pub(crate) fn id_for(&self, key_code: u32, modifiers: u32) -> Option<u32> {
        let index = self.bindings.iter().position(|registration| {
            registration.key_code == key_code && registration.modifiers == modifiers
        })?;
        u32::try_from(index).ok()
    }

    pub fn matches_current_layout(&self) -> Result<bool, String> {
        LayoutSnapshot::current_fingerprint()
            .map(|fingerprint| self.layout_fingerprint == fingerprint)
            .map_err(|error| error.to_string())
    }

    fn register(&mut self, key_code: u32, modifiers: u32, id: u32) -> Result<(), String> {
        let mut registration = ptr::null_mut();
        // SAFETY: Carbon's application event target is valid on the main thread and `registration`
        // is a valid out-pointer.
        let status = unsafe {
            RegisterEventHotKey(
                key_code,
                modifiers,
                EventHotKeyId {
                    signature: SIGNATURE,
                    id,
                },
                GetApplicationEventTarget(),
                0,
                &raw mut registration,
            )
        };
        if status != 0 {
            return Err(format!("hotkey is unavailable ({status})"));
        }
        self.registrations.push(registration);
        Ok(())
    }
}

impl Drop for Hotkeys {
    fn drop(&mut self) {
        for registration in self.registrations.drain(..) {
            // SAFETY: Each registration handle came from a successful `RegisterEventHotKey` call.
            unsafe { UnregisterEventHotKey(registration) };
        }
        // SAFETY: `self.handler` came from a successful `InstallEventHandler` call.
        let removed = unsafe { RemoveEventHandler(self.handler) };
        if removed != 0
            && let Some(context) = self.context.take()
        {
            Box::leak(context);
        }
    }
}

unsafe extern "C" fn hotkey_handler(
    _: *mut c_void,
    event: *mut c_void,
    user_data: *mut c_void,
) -> i32 {
    // SAFETY: `user_data` points to the `HandlerContext` boxed in `Hotkeys::install`, which
    // outlives the installed Carbon event handler.
    let context = unsafe { &*user_data.cast::<HandlerContext>() };
    let mut hotkey_id = EventHotKeyId {
        signature: 0,
        id: 0,
    };
    // SAFETY: `event` is provided by Carbon for `EVENT_HOTKEY_PRESSED`, and `hotkey_id` matches
    // the expected `EventHotKeyId` layout and byte count.
    let status = unsafe {
        GetEventParameter(
            event,
            u32::from_be_bytes(*b"----"),
            u32::from_be_bytes(*b"hkid"),
            ptr::null_mut(),
            u32::try_from(std::mem::size_of::<EventHotKeyId>()).unwrap_or_default(),
            ptr::null_mut(),
            (&raw mut hotkey_id).cast(),
        )
    };
    if status == 0 && hotkey_id.signature == SIGNATURE {
        drop(context.sender.send(Action::Hotkey(hotkey_id.id)));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> LayoutSnapshot {
        LayoutSnapshot::for_test(
            vec![
                (60, 'h', false, false),
                (61, 'j', false, false),
                (62, 'k', false, false),
                (63, 'l', false, false),
                (64, 'f', false, false),
                (65, '/', false, false),
                (66, '-', false, false),
                (67, '=', false, false),
                (68, '`', false, false),
            ],
            vec![],
        )
    }

    fn action<'a>(plan: &'a HotkeyPlan, description: &str) -> Option<&'a WorkerAction> {
        plan.registrations
            .iter()
            .find(|registration| registration.description == description)
            .map(|registration| &registration.action)
    }

    #[test]
    fn registrations_carry_switch_move_and_layout_actions_including_zero_key() {
        let enabled = WorkspaceId::numbered().collect::<Vec<_>>();
        let plan = HotkeyPlan::from_layout(&enabled, &layout()).unwrap();
        let one = WorkspaceId::new(1).unwrap();
        let ten = WorkspaceId::new(10).unwrap();
        for (description, expected) in [
            ("switch workspace 1", WorkerAction::SwitchWorkspace(one)),
            (
                "toggle fullscreen layout",
                WorkerAction::ToggleFullscreenLayout,
            ),
            ("toggle layout", WorkerAction::ToggleLayout),
            (
                "workspace back-and-forth",
                WorkerAction::WorkspaceBackAndForth,
            ),
            (
                "shrink layout",
                WorkerAction::Resize(ResizeDirection::Shrink),
            ),
            ("grow layout", WorkerAction::Resize(ResizeDirection::Grow)),
            ("cycle workspace windows", WorkerAction::CycleFocus),
            ("focus left", WorkerAction::Focus(Direction::Left)),
            ("focus right", WorkerAction::Focus(Direction::Right)),
            ("move window left", WorkerAction::Reorder(Direction::Left)),
            ("move window right", WorkerAction::Reorder(Direction::Right)),
            ("switch workspace 10", WorkerAction::SwitchWorkspace(ten)),
            (
                "move window to workspace 10",
                WorkerAction::MoveFocused(ten),
            ),
        ] {
            assert_eq!(action(&plan, description), Some(&expected), "{description}");
        }
    }

    #[test]
    fn full_plan_contains_all_thirty_four_registrations() {
        let enabled = WorkspaceId::numbered().collect::<Vec<_>>();
        let plan = HotkeyPlan::from_layout(&enabled, &layout()).unwrap();
        assert_eq!(plan.registrations.len(), 34);
        assert!(plan.registrations.capacity() >= 34);
    }

    #[test]
    fn named_workspace_claims_its_letter_shortcuts() {
        let mut enabled = WorkspaceId::numbered().collect::<Vec<_>>();
        enabled.push(WorkspaceId::named('H').unwrap());
        let plan = HotkeyPlan::from_layout(&enabled, &layout()).unwrap();
        let h = WorkspaceId::named('H').unwrap();
        assert_eq!(
            action(&plan, "switch workspace H"),
            Some(&WorkerAction::SwitchWorkspace(h))
        );
        assert_eq!(
            action(&plan, "move window to workspace H"),
            Some(&WorkerAction::MoveFocused(h))
        );
        assert_eq!(action(&plan, "focus left"), None);
        assert_eq!(action(&plan, "move window left"), None);
    }

    #[test]
    fn layout_required_shift_is_composed_with_command_modifiers() {
        let layout = LayoutSnapshot::for_test(
            vec![
                (60, 'h', false, false),
                (61, 'j', false, false),
                (62, 'k', false, false),
                (63, 'l', false, false),
                (64, 'f', false, false),
                (65, '/', true, false),
                (66, '-', false, false),
                (67, '=', false, false),
                (68, '`', false, false),
            ],
            vec![],
        );
        let plan = HotkeyPlan::from_layout(&[], &layout).unwrap();
        let toggle_layout = plan
            .registrations
            .iter()
            .find(|registration| registration.action == WorkerAction::ToggleLayout)
            .unwrap();
        assert_eq!(toggle_layout.modifiers, OPTION_KEY | SHIFT_KEY);
    }

    #[test]
    fn duplicate_chords_skip_only_the_conflicting_shortcut() {
        let layout = LayoutSnapshot::for_test(
            vec![
                (60, 'h', false, false),
                (60, 'j', false, false),
                (62, 'k', false, false),
                (63, 'l', false, false),
                (64, 'f', false, false),
                (65, '/', false, false),
                (66, '-', false, false),
                (67, '=', false, false),
                (68, '`', false, false),
            ],
            vec![],
        );
        let plan = HotkeyPlan::from_layout(&[], &layout).unwrap();
        assert_eq!(
            action(&plan, "focus left"),
            Some(&WorkerAction::Focus(Direction::Left))
        );
        assert_eq!(action(&plan, "focus down"), None);
        assert_eq!(
            action(&plan, "toggle layout"),
            Some(&WorkerAction::ToggleLayout)
        );
        assert!(
            plan.warnings
                .iter()
                .any(|warning| warning.contains("skipped focus down")),
            "{:?}",
            plan.warnings
        );
    }

    #[test]
    fn italian_pro_without_a_grave_keeps_tiling_and_remaps_shifted_punctuation() {
        // Italian – Pro: no key types a plain grave, `/` is Shift+7 (virtual key 0x1a, the
        // physical 7 key), and `=` is Shift+0 (0x1d, the physical 0 key).
        let italian = LayoutSnapshot::for_test(
            vec![
                (4, 'h', false, false),
                (38, 'j', false, false),
                (40, 'k', false, false),
                (37, 'l', false, false),
                (3, 'f', false, false),
                (0x1a, '/', true, false),
                (0x1b, '-', false, false),
                (0x1d, '=', true, false),
            ],
            vec![],
        );
        let enabled = WorkspaceId::numbered().collect::<Vec<_>>();
        let plan = HotkeyPlan::from_layout(&enabled, &italian).unwrap();
        let registration = |description: &str| {
            plan.registrations
                .iter()
                .find(|registration| registration.description == description)
                .unwrap_or_else(|| panic!("missing {description}: {:?}", plan.warnings))
        };

        // The semantic number row still owns Option+Shift+7 and Option+Shift+0.
        assert_eq!(registration("move window to workspace 7").key_code, 0x1a);
        assert_eq!(registration("move window to workspace 10").key_code, 0x1d);

        // Each shifted punctuation shortcut moves to its plain ANSI physical key instead of
        // colliding with a workspace chord or disappearing.
        for (description, key_code, modifiers) in [
            ("toggle layout", 0x2c, OPTION_KEY),
            ("grow layout", 0x18, OPTION_KEY),
            ("shrink layout", 0x1b, OPTION_KEY),
            // The missing grave falls back to the key above Tab.
            ("cycle workspace windows", 0x32, OPTION_KEY),
        ] {
            let registration = registration(description);
            assert_eq!(registration.key_code, key_code, "{description}");
            assert_eq!(registration.modifiers, modifiers, "{description}");
        }
        let focus_left = registration("focus left");
        assert_eq!(focus_left.key_code, 4);
        assert_eq!(focus_left.modifiers, OPTION_KEY);
        assert_eq!(focus_left.action, WorkerAction::Focus(Direction::Left));
        assert_eq!(
            registration("switch workspace 1").key_code,
            0x12,
            "the semantic number row keeps tiling reachable"
        );
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    }

    #[test]
    fn german_dead_grave_key_still_plans_the_cycle_shortcut() {
        // German QWERTZ exposes no live grave; the only one is a dead key on 0x0a.
        let german = LayoutSnapshot::for_test(
            vec![
                (4, 'h', false, false),
                (38, 'j', false, false),
                (40, 'k', false, false),
                (37, 'l', false, false),
                (3, 'f', false, false),
                (0x2c, '/', true, false),
                (0x1b, '-', false, false),
                (0x18, '=', false, false),
                (0x0a, '`', false, true),
            ],
            vec![],
        );
        let plan = HotkeyPlan::from_layout(&[], &german).unwrap();
        let cycle = plan
            .registrations
            .iter()
            .find(|registration| registration.action == WorkerAction::CycleFocus)
            .expect("cycle shortcut is planned from the dead key");
        assert_eq!(cycle.key_code, 0x0a);
        assert_eq!(cycle.modifiers, OPTION_KEY);
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    }

    #[test]
    fn layout_without_letters_keeps_semantic_and_punctuation_shortcuts() {
        // Cyrillic without an ASCII-capable fallback cannot type h, j, k or l at all.
        let non_latin_without_ascii_fallback = LayoutSnapshot::for_test(
            vec![
                (60, 'ж', false, false),
                (61, 'д', false, false),
                (62, 'л', false, false),
                (63, 'о', false, false),
                (65, '/', false, false),
                (66, '-', false, false),
                (67, '=', false, false),
            ],
            vec![],
        );

        let plan = HotkeyPlan::from_layout(&[], &non_latin_without_ascii_fallback).unwrap();
        assert_eq!(action(&plan, "focus left"), None);
        assert_eq!(
            action(&plan, "toggle layout"),
            Some(&WorkerAction::ToggleLayout)
        );
        assert_eq!(
            action(&plan, "workspace back-and-forth"),
            Some(&WorkerAction::WorkspaceBackAndForth)
        );
        assert_eq!(
            action(&plan, "cycle workspace windows"),
            Some(&WorkerAction::CycleFocus)
        );
        assert!(
            plan.warnings
                .iter()
                .any(|warning| warning.contains("skipped focus left")
                    && warning.contains("does not produce 'h'")),
            "{:?}",
            plan.warnings
        );
    }
}
