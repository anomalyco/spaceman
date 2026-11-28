//! Pure decision logic for keeping the current application visible when
//! Option-clicking another application's window.
//!
//! macOS hides the frontmost app on Option-click (and hides all other apps on
//! Option+Command-click) when the click lands on a window belonging to another
//! app. Spaceman strips `Option` from that `LeftMouseDown` event only when the
//! click lands on another app's normal/floating window — never inside a menu
//! bar strip, system chrome, popup menu, Dock, desktop wallpaper, or the
//! already-frontmost app itself.

use crate::Rect;

/// `kCGDockWindowLevel`: the Dock window sits at layer 20.
pub const DOCK_LAYER: i64 = 20;
/// `kCGMainMenuWindowLevel`: menu bar (24), status items (25), and overlays above it.
pub const MAIN_MENU_LAYER: i64 = 24;
/// `kCGPopUpMenuWindowLevel`: open menu bar and context menus.
pub const POP_UP_MENU_LAYER: i64 = 101;

const SYSTEM_CHROME_OWNERS: &[&str] = &[
    "Dock",
    "Control Center",
    "ControlCenter",
    "SystemUIServer",
    "Notification Center",
    "NotificationCenter",
    "Window Server",
    "WindowServer",
    "Wallpaper",
    "Spotlight",
    "loginwindow",
];

const SYSTEM_CHROME_BUNDLES: &[&str] = &[
    "com.apple.dock",
    "com.apple.controlcenter",
    "com.apple.systemuiserver",
    "com.apple.notificationcenterui",
    "com.apple.Spotlight",
    "com.apple.wallpaper.agent",
    "com.apple.loginwindow",
];

/// One on-screen window-server window in front-to-back order, read purely from
/// Core Graphics (`CGWindowListCopyWindowInfo`) without Accessibility calls.
#[derive(Clone, Debug, PartialEq)]
pub struct HitWindow<'a> {
    pub owner_pid: i32,
    pub layer: i64,
    pub alpha: f64,
    pub frame: Rect,
    /// `kCGWindowOwnerName` as reported by WindowServer, if present.
    pub owner_name: Option<&'a str>,
    /// Application bundle identifier when known (handles localized owner names).
    pub bundle_id: Option<&'a str>,
}

/// Snapshot of a `LeftMouseDown` click used to decide whether `Option` should
/// be stripped from its modifier flags.
#[derive(Clone, Copy, Debug)]
pub struct OptionClickContext<'a> {
    pub enabled: bool,
    pub option_held: bool,
    pub point: (i32, i32),
    pub frontmost_pid: Option<i32>,
    pub self_pid: i32,
    pub menu_bar_strips: &'a [Rect],
    /// On-screen windows in front-to-back order.
    pub windows: &'a [HitWindow<'a>],
}

/// Whether `owner_name` or `bundle_id` belongs to macOS system chrome whose
/// Option-click behavior must always stay untouched.
#[must_use]
pub fn is_system_chrome(owner_name: Option<&str>, bundle_id: Option<&str>) -> bool {
    owner_name.is_some_and(|name| SYSTEM_CHROME_OWNERS.contains(&name))
        || bundle_id.is_some_and(|bundle| SYSTEM_CHROME_BUNDLES.contains(&bundle))
}

/// Decides whether Spaceman should strip `kCGEventFlagMaskAlternate` from a
/// `LeftMouseDown` event while passing the click through untouched.
#[must_use]
pub fn should_strip_option(ctx: &OptionClickContext<'_>) -> bool {
    if !ctx.enabled || !ctx.option_held {
        return false;
    }
    if ctx
        .menu_bar_strips
        .iter()
        .any(|strip| strip.contains(ctx.point.0, ctx.point.1))
    {
        return false;
    }
    let Some(frontmost_pid) = ctx.frontmost_pid else {
        return false;
    };

    for window in ctx.windows {
        if !window.frame.contains(ctx.point.0, ctx.point.1) {
            continue;
        }
        // Skip fully transparent / invisible overlay windows.
        if window.alpha.is_nan() || window.alpha <= 0.0 {
            continue;
        }
        // Never strip Option on open popup menus, including Spaceman's own menu.
        if window.layer >= POP_UP_MENU_LAYER {
            return false;
        }
        // Skip Spaceman's own non-menu overlay windows (HUD, Window Overlay) so
        // they never mask the real app window underneath. Spaceman's menu bar
        // icon is protected by `menu_bar_strips` and the system menu bar at
        // layer 24 directly behind it.
        if window.owner_pid == ctx.self_pid {
            continue;
        }
        // Stop on desktop/wallpaper (layer < 0), Dock (layer 20), menu bar /
        // status bar / overlays (layer >= 24), or any system chrome window.
        if window.layer < 0 || window.layer >= DOCK_LAYER {
            return false;
        }
        if is_system_chrome(window.owner_name, window.bundle_id) {
            return false;
        }
        // Normal or floating app window: strip Option only when switching to a
        // *different* app, keeping all in-app Option-clicks intact.
        return window.owner_pid > 0 && window.owner_pid != frontmost_pid;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const SELF_PID: i32 = 999;
    const FRONT_PID: i32 = 100;
    const OTHER_PID: i32 = 200;

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn app_window(pid: i32, layer: i64, frame: Rect) -> HitWindow<'static> {
        HitWindow {
            owner_pid: pid,
            layer,
            alpha: 1.0,
            frame,
            owner_name: Some("App"),
            bundle_id: Some("com.example.App"),
        }
    }

    fn menu_bars() -> [Rect; 2] {
        [rect(0, 0, 1728, 37), rect(1728, 0, 2560, 25)]
    }

    #[test]
    fn strips_option_on_cross_app_normal_and_floating_windows() {
        let bars = menu_bars();
        let windows = [
            app_window(OTHER_PID, 0, rect(100, 100, 800, 600)),
            app_window(FRONT_PID, 0, rect(0, 37, 1728, 1080)),
        ];
        assert!(should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (300, 300),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &windows,
        }));

        // Floating palette of another app (layer 3) also activates that app on Option-click.
        let floating = [app_window(OTHER_PID, 3, rect(100, 100, 400, 300))];
        assert!(should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (200, 200),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &floating,
        }));
    }

    #[test]
    fn preserves_option_on_same_app_clicks() {
        let bars = menu_bars();
        let windows = [
            app_window(FRONT_PID, 0, rect(100, 100, 800, 600)),
            app_window(OTHER_PID, 0, rect(0, 37, 1728, 1080)),
        ];
        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (300, 300),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &windows,
        }));
    }

    #[test]
    fn preserves_option_in_menu_bar_strips_and_status_or_popup_layers() {
        let bars = menu_bars();
        let windows = [app_window(OTHER_PID, 0, rect(0, 0, 2560, 1440))];

        // Primary display's notched menu bar strip.
        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (1400, 18),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &windows,
        }));

        // Secondary display's menu bar strip.
        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (2000, 12),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &windows,
        }));

        // Auto-hidden menu bar or status item (layer >= 24) even with empty menu_bar_strips.
        for layer in [MAIN_MENU_LAYER, 25, POP_UP_MENU_LAYER] {
            let stacked = [
                HitWindow {
                    owner_pid: 55,
                    layer,
                    alpha: 1.0,
                    frame: rect(0, 0, 1728, 30),
                    owner_name: Some("ThirdPartyStatus"),
                    bundle_id: Some("com.example.Status"),
                },
                app_window(OTHER_PID, 0, rect(0, 0, 1728, 1080)),
            ];
            assert!(
                !should_strip_option(&OptionClickContext {
                    enabled: true,
                    option_held: true,
                    point: (500, 15),
                    frontmost_pid: Some(FRONT_PID),
                    self_pid: SELF_PID,
                    menu_bar_strips: &[],
                    windows: &stacked,
                }),
                "layer {layer} must preserve Option"
            );
        }

        // Spaceman's own open menu bar dropdown (layer 101) preserves Option too.
        let own_menu = [
            HitWindow {
                owner_pid: SELF_PID,
                layer: POP_UP_MENU_LAYER,
                alpha: 1.0,
                frame: rect(1200, 37, 260, 320),
                owner_name: Some("Spaceman"),
                bundle_id: Some("anomaly.spaceman"),
            },
            app_window(OTHER_PID, 0, rect(0, 37, 1728, 1080)),
        ];
        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (1300, 100),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &own_menu,
        }));
    }

    #[test]
    fn preserves_option_on_dock_control_center_and_system_chrome() {
        let bars = menu_bars();
        // Dock at layer 20.
        let dock = [
            HitWindow {
                owner_pid: 42,
                layer: DOCK_LAYER,
                alpha: 1.0,
                frame: rect(400, 1010, 900, 70),
                owner_name: Some("Dock"),
                bundle_id: Some("com.apple.dock"),
            },
            app_window(OTHER_PID, 0, rect(0, 37, 1728, 1080)),
        ];
        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (600, 1040),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &dock,
        }));

        // Control Center / SystemUIServer / Notification Center even on a floating layer (3)
        // and even when owner_name is localized and only bundle_id identifies it.
        for (owner_name, bundle_id) in [
            (Some("Control Center"), Some("com.apple.controlcenter")),
            (Some("Centre de contrôle"), Some("com.apple.controlcenter")),
            (Some("SystemUIServer"), None),
            (Some("Notification Center"), None),
            (None, Some("com.apple.notificationcenterui")),
            (Some("Spotlight"), Some("com.apple.Spotlight")),
        ] {
            let windows = [
                HitWindow {
                    owner_pid: 77,
                    layer: 3,
                    alpha: 1.0,
                    frame: rect(1200, 45, 360, 420),
                    owner_name,
                    bundle_id,
                },
                app_window(OTHER_PID, 0, rect(0, 37, 1728, 1080)),
            ];
            assert!(
                !should_strip_option(&OptionClickContext {
                    enabled: true,
                    option_held: true,
                    point: (1300, 150),
                    frontmost_pid: Some(FRONT_PID),
                    self_pid: SELF_PID,
                    menu_bar_strips: &bars,
                    windows: &windows,
                }),
                "system chrome ({owner_name:?}, {bundle_id:?}) must preserve Option"
            );
        }
    }

    #[test]
    fn preserves_option_on_desktop_wallpaper_and_empty_desktop() {
        let bars = menu_bars();
        let wallpaper = [HitWindow {
            owner_pid: 50,
            layer: -2_147_483_623,
            alpha: 1.0,
            frame: rect(0, 0, 1728, 1117),
            owner_name: Some("Wallpaper"),
            bundle_id: Some("com.apple.wallpaper.agent"),
        }];
        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (500, 500),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &wallpaper,
        }));

        // Empty window list (desktop excluded by ExcludeDesktopElements).
        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (500, 500),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &[],
        }));
    }

    #[test]
    fn skips_transparent_windows_and_spaceman_overlays() {
        let bars = menu_bars();
        let windows = [
            // Invisible alpha=0 overlay above everything.
            HitWindow {
                owner_pid: 333,
                layer: 25,
                alpha: 0.0,
                frame: rect(0, 0, 1728, 1117),
                owner_name: Some("GhostOverlay"),
                bundle_id: None,
            },
            // NaN alpha treated as invisible too.
            HitWindow {
                owner_pid: 334,
                layer: 25,
                alpha: f64::NAN,
                frame: rect(0, 0, 1728, 1117),
                owner_name: Some("NanOverlay"),
                bundle_id: None,
            },
            // Spaceman's own non-activating HUD / Window Overlay.
            HitWindow {
                owner_pid: SELF_PID,
                layer: 25,
                alpha: 1.0,
                frame: rect(0, 0, 1728, 1117),
                owner_name: Some("Spaceman"),
                bundle_id: Some("anomaly.spaceman"),
            },
            // Real cross-app window underneath.
            app_window(OTHER_PID, 0, rect(100, 100, 800, 600)),
        ];
        assert!(should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (300, 300),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &windows,
        }));

        // When Spaceman's menu bar icon (self_pid, layer 25) sits over the
        // auto-hidden system menu bar (layer 24), skipping self_pid still hits
        // the system menu bar and preserves Option.
        let auto_hidden_bar = [
            HitWindow {
                owner_pid: SELF_PID,
                layer: 25,
                alpha: 1.0,
                frame: rect(1200, 0, 48, 24),
                owner_name: Some("Spaceman"),
                bundle_id: Some("anomaly.spaceman"),
            },
            HitWindow {
                owner_pid: 88,
                layer: MAIN_MENU_LAYER,
                alpha: 1.0,
                frame: rect(0, 0, 1728, 24),
                owner_name: Some("Window Server"),
                bundle_id: None,
            },
            app_window(OTHER_PID, 0, rect(0, 0, 1728, 1080)),
        ];
        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (1220, 12),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &[],
            windows: &auto_hidden_bar,
        }));
    }

    #[test]
    fn skips_when_disabled_option_not_held_or_no_frontmost_app() {
        let bars = menu_bars();
        let windows = [app_window(OTHER_PID, 0, rect(100, 100, 800, 600))];

        assert!(!should_strip_option(&OptionClickContext {
            enabled: false,
            option_held: true,
            point: (300, 300),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &windows,
        }));

        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: false,
            point: (300, 300),
            frontmost_pid: Some(FRONT_PID),
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &windows,
        }));

        assert!(!should_strip_option(&OptionClickContext {
            enabled: true,
            option_held: true,
            point: (300, 300),
            frontmost_pid: None,
            self_pid: SELF_PID,
            menu_bar_strips: &bars,
            windows: &windows,
        }));
    }
}
