use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{DisplaySelector, ObservedWindow, WorkspaceId, Workspaces};

#[derive(Clone, Copy, Debug)]
struct AppSettings {
    hud: bool,
    hud_style: HudStyle,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hud: true,
            hud_style: HudStyle::default(),
        }
    }
}

/// How the Workspace HUD draws each workspace in its row: `settings.hud_style`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HudStyle {
    /// Each workspace as its number and the icons of its apps.
    #[default]
    Icons,
    /// Each workspace as its layout, with an app icon in every tile; the current one larger.
    Layouts,
}

impl HudStyle {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Layouts => "layouts",
            Self::Icons => "icons",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "layouts" => Some(Self::Layouts),
            "icons" => Some(Self::Icons),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    settings: AppSettings,
    rules: Vec<WindowRule>,
    workspace_monitors: Vec<(WorkspaceId, DisplaySelector)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowDisposition {
    Managed,
    Floating,
    Route(WorkspaceId),
    /// Never tiled, owned, moved, or parked. For windows whose applications
    /// insist on managing them themselves.
    Ignored,
}

#[derive(Clone, Debug)]
struct WindowRule {
    matcher: WindowMatcher,
    action: WindowDisposition,
}

#[derive(Clone, Debug)]
struct WindowMatcher {
    bundle_id: Option<String>,
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    version: u32,
    #[serde(default)]
    settings: RawSettings,
    #[serde(default)]
    rules: Vec<RawWindowRule>,
    #[serde(default)]
    workspaces: Vec<RawWorkspace>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSettings {
    #[serde(default = "default_hud")]
    hud: bool,
    #[serde(default)]
    hud_style: HudStyle,
}

impl Default for RawSettings {
    fn default() -> Self {
        Self {
            hud: true,
            hud_style: HudStyle::default(),
        }
    }
}

const fn default_hud() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWindowRule {
    bundle_id: Option<String>,
    title: Option<String>,
    action: String,
    workspace: Option<WorkspaceId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWorkspace {
    workspace: WorkspaceId,
    monitor: String,
}

impl Config {
    #[must_use]
    #[cfg(test)]
    pub fn has_rules(&self) -> bool {
        !self.rules.is_empty()
    }

    pub fn parse(source: &str) -> Result<Self, String> {
        let raw: RawConfig = toml::from_str(source).map_err(|error| error.to_string())?;
        if raw.version != 1 {
            return Err(format!(
                "unsupported config version {}; expected 1",
                raw.version
            ));
        }
        let mut rules = Vec::with_capacity(raw.rules.len());
        for rule in raw.rules {
            if rule.bundle_id.is_none() && rule.title.is_none() {
                return Err("window rule must contain at least one matcher".into());
            }
            let action = match (rule.action.as_str(), rule.workspace) {
                ("float", None) => WindowDisposition::Floating,
                ("ignore", None) => WindowDisposition::Ignored,
                ("route", Some(workspace)) => WindowDisposition::Route(workspace),
                ("float", Some(_)) => return Err("float rule cannot specify workspace".into()),
                ("ignore", Some(_)) => return Err("ignore rule cannot specify workspace".into()),
                ("route", None) => return Err("route rule requires workspace".into()),
                (action, _) => return Err(format!("unknown window rule action {action:?}")),
            };
            rules.push(WindowRule {
                matcher: WindowMatcher {
                    bundle_id: rule.bundle_id,
                    title: rule.title,
                },
                action,
            });
        }
        let mut workspace_monitors = Vec::with_capacity(raw.workspaces.len());
        for workspace in raw.workspaces {
            let id = workspace.workspace;
            if workspace.monitor.trim().is_empty() {
                return Err(format!("workspace {id} monitor cannot be empty"));
            }
            if workspace_monitors
                .iter()
                .any(|(existing, _)| *existing == id)
            {
                return Err(format!("workspace {} is configured more than once", id));
            }
            workspace_monitors.push((id, DisplaySelector::parse(&workspace.monitor)));
        }
        Ok(Self {
            settings: AppSettings {
                hud: raw.settings.hud,
                hud_style: raw.settings.hud_style,
            },
            rules,
            workspace_monitors,
        })
    }

    #[must_use]
    pub const fn hud(&self) -> bool {
        self.settings.hud
    }

    #[must_use]
    pub const fn hud_style(&self) -> HudStyle {
        self.settings.hud_style
    }

    pub fn load() -> Result<Self, String> {
        let path = config_path().ok_or("HOME is unavailable")?;
        match std::fs::read_to_string(&path) {
            Ok(source) => parse_path(&path, &source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if let Some(legacy) = fallback_legacy_config_path(&path) {
                    if let Ok(Some(_)) = migrate_config_file(&path, std::slice::from_ref(&legacy))
                        && let Ok(source) = std::fs::read_to_string(&path)
                    {
                        return parse_path(&path, &source);
                    }
                    let source = std::fs::read_to_string(&legacy).map_err(|read_error| {
                        format!("failed to read {}: {read_error}", legacy.display())
                    })?;
                    return parse_path(&legacy, &source);
                }
                Ok(Self::default())
            }
            Err(error) => Err(format!("failed to read {}: {error}", path.display())),
        }
    }

    fn matching_rule(&self, bundle_id: &str, window: &ObservedWindow) -> Option<&WindowRule> {
        self.rules
            .iter()
            .find(|rule| rule.matcher.matches(bundle_id, window))
    }

    #[must_use]
    pub fn disposition(&self, bundle_id: &str, window: &ObservedWindow) -> WindowDisposition {
        self.matching_rule(bundle_id, window)
            .map_or(WindowDisposition::Managed, |rule| rule.action)
    }

    #[must_use]
    pub fn route_matches_title(&self, bundle_id: &str, window: &ObservedWindow) -> bool {
        self.matching_rule(bundle_id, window).is_some_and(|rule| {
            matches!(rule.action, WindowDisposition::Route(_)) && rule.matcher.title.is_some()
        })
    }

    #[must_use]
    pub fn has_bundle_route(&self, bundle_id: &str) -> bool {
        self.rules
            .iter()
            .find(|rule| {
                rule.matcher.title.is_none()
                    && rule
                        .matcher
                        .bundle_id
                        .as_deref()
                        .is_some_and(|value| value == bundle_id)
            })
            .is_some_and(|rule| matches!(rule.action, WindowDisposition::Route(_)))
    }

    pub fn configure_workspaces(&self, workspaces: &mut Workspaces) {
        for workspace in self.named_workspaces() {
            workspaces.enable(workspace);
        }
        for (workspace, selector) in &self.workspace_monitors {
            workspaces.set_assignment(*workspace, selector.clone());
        }
    }

    pub fn named_workspaces(&self) -> impl Iterator<Item = WorkspaceId> + '_ {
        self.workspace_monitors
            .iter()
            .map(|(workspace, _)| *workspace)
            .chain(self.rules.iter().filter_map(|rule| match rule.action {
                WindowDisposition::Route(workspace) => Some(workspace),
                _ => None,
            }))
            .filter(|workspace| workspace.letter().is_some())
    }
}

impl WindowMatcher {
    fn matches(&self, bundle_id: &str, window: &ObservedWindow) -> bool {
        self.bundle_id
            .as_deref()
            .is_none_or(|value| value == bundle_id)
            && self
                .title
                .as_deref()
                .is_none_or(|value| window.title.as_deref() == Some(value))
    }
}

/// Wall-clock duration since the Unix epoch.
#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "config::now is the central wall-clock adapter"
)]
pub fn now() -> std::time::Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
}

const MIGRATION_MARKER: &str = ".migrated-from-spaceman";

/// Durable state files copied from `~/Library/Application Support/Spaceman` to
/// `~/Library/Application Support/Pane` on first launch. Runtime sockets, locks,
/// and log files are intentionally excluded.
const MIGRATED_STATE_FILES: &[&str] = &[
    "parked-windows.json",
    "workspace-state.json",
    "window-memory.json",
    "onboarding-complete",
    "onboarding-step",
    "journal.jsonl",
    "journal.jsonl.1",
];

/// Presence of any of these files in `~/Library/Application Support/Pane` means
/// the directory is already initialized and legacy state must not overwrite it.
const INITIALIZED_SENTINELS: &[&str] = &[
    MIGRATION_MARKER,
    "parked-windows.json",
    "workspace-state.json",
    "window-memory.json",
    "onboarding-complete",
    "journal.jsonl",
];

/// Summary of files copied from legacy Spaceman paths on first launch.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MigrationReport {
    pub copied_state_files: Vec<String>,
    pub copied_config_from: Option<PathBuf>,
}

impl MigrationReport {
    #[must_use]
    pub fn migrated_anything(&self) -> bool {
        !self.copied_state_files.is_empty() || self.copied_config_from.is_some()
    }
}

/// Optional `PANE_MANAGED_DISPLAY` (or legacy `SPACEMAN_MANAGED_DISPLAY`) override read at startup.
#[expect(
    clippy::disallowed_methods,
    reason = "startup environment variables are read in config.rs"
)]
pub fn managed_display_env() -> Result<Option<String>, String> {
    for variable in ["PANE_MANAGED_DISPLAY", "SPACEMAN_MANAGED_DISPLAY"] {
        match std::env::var(variable) {
            Ok(value) => return Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => continue,
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(format!("{variable} must be valid UTF-8"));
            }
        }
    }
    Ok(None)
}

/// When the config file last changed, or `None` while it doesn't exist.
#[must_use]
pub fn modified() -> Option<std::time::SystemTime> {
    std::fs::metadata(config_path()?).ok()?.modified().ok()
}

pub fn config_path() -> Option<PathBuf> {
    config_path_from(
        std::env::var_os("PANE_CONFIG"),
        std::env::var_os("SPACEMAN_CONFIG"),
        std::env::var_os("PANE_APPLICATION_SUPPORT"),
        std::env::var_os("SPACEMAN_APPLICATION_SUPPORT"),
        std::env::var_os("HOME"),
    )
}

pub fn application_support_path(file: &str) -> Option<PathBuf> {
    application_support_dir_from(
        std::env::var_os("PANE_APPLICATION_SUPPORT"),
        std::env::var_os("SPACEMAN_APPLICATION_SUPPORT"),
        std::env::var_os("HOME"),
    )
    .map(|directory| directory.join(file))
}

fn config_path_from(
    pane_config: Option<OsString>,
    spaceman_config: Option<OsString>,
    pane_support: Option<OsString>,
    spaceman_support: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    pane_config
        .or(spaceman_config)
        .map(PathBuf::from)
        .or_else(|| {
            pane_support
                .or(spaceman_support)
                .map(PathBuf::from)
                .map(|directory| directory.join("config.toml"))
        })
        .or_else(|| {
            home.map(PathBuf::from)
                .map(|home| home.join(".config/pane/config.toml"))
        })
}

fn application_support_dir_from(
    pane_support: Option<OsString>,
    spaceman_support: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    pane_support
        .or(spaceman_support)
        .map(PathBuf::from)
        .or_else(|| {
            home.map(PathBuf::from)
                .map(|home| home.join("Library/Application Support/Pane"))
        })
}

fn legacy_config_candidates(home: &Path) -> [PathBuf; 2] {
    [
        home.join(".config/spaceman/config.toml"),
        home.join("Library/Application Support/Spaceman/config.toml"),
    ]
}

fn fallback_legacy_config_path(primary: &Path) -> Option<PathBuf> {
    if std::env::var_os("PANE_CONFIG").is_some()
        || std::env::var_os("SPACEMAN_CONFIG").is_some()
        || std::env::var_os("PANE_APPLICATION_SUPPORT").is_some()
        || std::env::var_os("SPACEMAN_APPLICATION_SUPPORT").is_some()
    {
        return None;
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    if primary != home.join(".config/pane/config.toml") {
        return None;
    }
    legacy_config_candidates(&home)
        .into_iter()
        .find(|candidate| candidate.is_file())
}

/// Copies legacy Spaceman configuration and recovery/state files into Pane's default
/// paths when Pane does not have them yet. Never deletes or modifies legacy originals,
/// and never runs when isolated environment overrides are active.
pub fn migrate_legacy_data() -> Result<MigrationReport, String> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Ok(MigrationReport::default());
    };
    let migrate_support = std::env::var_os("PANE_APPLICATION_SUPPORT").is_none()
        && std::env::var_os("SPACEMAN_APPLICATION_SUPPORT").is_none();
    let migrate_config = migrate_support
        && std::env::var_os("PANE_CONFIG").is_none()
        && std::env::var_os("SPACEMAN_CONFIG").is_none();
    migrate_legacy_data_in_home(&home, migrate_support, migrate_config)
}

/// Copies legacy Spaceman config into `~/.config/pane/config.toml` if needed and no
/// config/support environment override is set.
pub fn ensure_config_migrated() {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    if std::env::var_os("PANE_CONFIG").is_some()
        || std::env::var_os("SPACEMAN_CONFIG").is_some()
        || std::env::var_os("PANE_APPLICATION_SUPPORT").is_some()
        || std::env::var_os("SPACEMAN_APPLICATION_SUPPORT").is_some()
    {
        return;
    }
    let target = home.join(".config/pane/config.toml");
    let candidates = legacy_config_candidates(&home);
    drop(migrate_config_file(&target, &candidates));
}

pub(crate) fn migrate_legacy_data_in_home(
    home: &Path,
    migrate_support: bool,
    migrate_config: bool,
) -> Result<MigrationReport, String> {
    let mut report = MigrationReport::default();
    if migrate_support {
        let new_dir = home.join("Library/Application Support/Pane");
        let legacy_dir = home.join("Library/Application Support/Spaceman");
        report.copied_state_files = migrate_support_dir(&new_dir, &legacy_dir)?;
    }
    if migrate_config {
        let new_config = home.join(".config/pane/config.toml");
        let candidates = legacy_config_candidates(home);
        report.copied_config_from = migrate_config_file(&new_config, &candidates)?;
    }
    Ok(report)
}

fn migrate_support_dir(new_dir: &Path, legacy_dir: &Path) -> Result<Vec<String>, String> {
    if !legacy_dir.is_dir() {
        return Ok(Vec::new());
    }
    if INITIALIZED_SENTINELS
        .iter()
        .any(|name| new_dir.join(name).exists())
    {
        return Ok(Vec::new());
    }
    let existing: Vec<&str> = MIGRATED_STATE_FILES
        .iter()
        .copied()
        .filter(|name| legacy_dir.join(name).is_file())
        .collect();
    if existing.is_empty() {
        return Ok(Vec::new());
    }
    std::fs::create_dir_all(new_dir).map_err(|error| {
        format!(
            "failed to create Pane state directory {}: {error}",
            new_dir.display()
        )
    })?;
    set_dir_owner_only(new_dir)?;
    let mut copied = Vec::with_capacity(existing.len());
    for name in existing {
        let source = legacy_dir.join(name);
        let destination = new_dir.join(name);
        if destination.exists() {
            continue;
        }
        // Copy raw bytes verbatim so malformed recovery state (such as parked-windows.json)
        // is never silently discarded during migration.
        let bytes = std::fs::read(&source).map_err(|error| {
            format!("failed to read legacy state {}: {error}", source.display())
        })?;
        crate::state_file::write_atomic(&destination, &bytes).map_err(|error| {
            format!(
                "failed to migrate {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
        set_file_owner_only(&destination)?;
        copied.push(name.to_owned());
    }
    let marker = new_dir.join(MIGRATION_MARKER);
    crate::state_file::write_atomic(
        &marker, b"1
",
    )
    .map_err(|error| {
        format!(
            "failed to write migration marker {}: {error}",
            marker.display()
        )
    })?;
    set_file_owner_only(&marker)?;
    Ok(copied)
}

fn migrate_config_file(
    new_config: &Path,
    legacy_candidates: &[PathBuf],
) -> Result<Option<PathBuf>, String> {
    if new_config.exists() {
        return Ok(None);
    }
    let Some(source) = legacy_candidates.iter().find(|path| path.is_file()) else {
        return Ok(None);
    };
    let bytes = std::fs::read(source)
        .map_err(|error| format!("failed to read legacy config {}: {error}", source.display()))?;
    crate::state_file::write_atomic(new_config, &bytes).map_err(|error| {
        format!(
            "failed to migrate config {} to {}: {error}",
            source.display(),
            new_config.display()
        )
    })?;
    Ok(Some(source.clone()))
}

#[cfg(unix)]
fn set_dir_owner_only(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(|error| {
        format!(
            "failed to secure Pane state directory {}: {error}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_dir_owner_only(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_file_owner_only(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|error| {
        format!(
            "failed to secure migrated state file {}: {error}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_file_owner_only(_path: &Path) -> Result<(), String> {
    Ok(())
}

pub fn load_path(path: &Path) -> Result<Config, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    parse_path(path, &source)
}

fn parse_path(path: &Path, source: &str) -> Result<Config, String> {
    Config::parse(source).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use crate::{AppInstanceId, PhysicalWindowId, Rect};

    use super::*;

    fn window(title: Option<&str>) -> ObservedWindow {
        ObservedWindow {
            id: PhysicalWindowId {
                app: AppInstanceId {
                    pid: 1,
                    launched_at: 1,
                },
                window_id: 1,
            },
            frame: Rect::default(),
            resizable: true,
            title: title.map(std::sync::Arc::from),
        }
    }

    #[test]
    fn ordered_rules_match_all_supplied_fields() {
        let config = Config::parse(
            r#"
version = 1

[[rules]]
bundle_id = "com.example.App"
title = "Settings"
action = "float"

[[rules]]
bundle_id = "com.example.App"
action = "route"
workspace = 7
"#,
        )
        .unwrap();

        assert_eq!(
            config.disposition("com.example.App", &window(Some("Settings"))),
            WindowDisposition::Floating
        );
        assert_eq!(
            config.disposition("com.example.App", &window(Some("Document"))),
            WindowDisposition::Route(WorkspaceId::new(7).unwrap())
        );
    }

    #[test]
    fn rejects_invalid_rules_and_workspaces() {
        assert!(Config::parse("version = 2").is_err());
        assert!(Config::parse("version = 1\n[[rules]]\naction = 'float'").is_err());
        assert!(
            Config::parse(
                "version = 1\n[[rules]]\nbundle_id = 'x'\naction = 'route'\nworkspace = 11"
            )
            .is_err()
        );
        assert!(
            Config::parse(
                "version = 1\n[[rules]]\nbundle_id = 'x'\naction = 'route'\nworkspace = 'AA'"
            )
            .is_err()
        );
    }

    #[test]
    fn accepts_named_workspace_routes_and_assignments() {
        let config = Config::parse(
            r#"
version = 1
[[rules]]
bundle_id = "ai.opencode.desktop"
action = "route"
workspace = "D"
[[workspaces]]
workspace = "D"
monitor = "main"
"#,
        )
        .unwrap();
        assert_eq!(
            config.disposition("ai.opencode.desktop", &window(None)),
            WindowDisposition::Route(WorkspaceId::named('D').unwrap())
        );
        assert_eq!(
            config.named_workspaces().collect::<Vec<_>>(),
            vec![
                WorkspaceId::named('D').unwrap(),
                WorkspaceId::named('D').unwrap()
            ]
        );
    }

    #[test]
    fn rejects_obsolete_keyboard_mapping_override() {
        assert!(Config::parse("version = 1\nkey_mapping = 'dvorak'").is_err());
    }

    #[test]
    fn parses_hud_setting_with_enabled_default() {
        assert!(Config::default().hud());
        assert!(Config::parse("version = 1").unwrap().hud());
        assert!(Config::parse("version = 1\n[settings]\n").unwrap().hud());
        assert!(
            !Config::parse("version = 1\n[settings]\nhud = false\n")
                .unwrap()
                .hud()
        );
        assert!(Config::parse("version = 1\n[settings]\nunknown = true\n").is_err());
    }

    #[test]
    fn parses_hud_style_with_icons_default() {
        assert_eq!(Config::default().hud_style(), HudStyle::Icons);
        assert_eq!(
            Config::parse("version = 1").unwrap().hud_style(),
            HudStyle::Icons
        );
        assert_eq!(
            Config::parse("version = 1\n[settings]\nhud = false\n")
                .unwrap()
                .hud_style(),
            HudStyle::Icons
        );
        for style in [HudStyle::Icons, HudStyle::Layouts] {
            let source = format!(
                "version = 1\n[settings]\nhud_style = {:?}\n",
                style.as_str()
            );
            assert_eq!(Config::parse(&source).unwrap().hud_style(), style);
            assert_eq!(HudStyle::parse(style.as_str()), Some(style));
        }
        assert!(Config::parse("version = 1\n[settings]\nhud_style = \"grid\"\n").is_err());
        assert!(Config::parse("version = 1\n[settings]\nhud_style = true\n").is_err());
        assert_eq!(HudStyle::parse("Icons"), None);
    }

    fn temp_home(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "pane-config-test-{label}-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolves_pane_default_paths_and_honors_legacy_env_overrides() {
        let home = Some(OsString::from("/Users/alex"));
        assert_eq!(
            application_support_dir_from(None, None, home.clone()),
            Some(PathBuf::from(
                "/Users/alex/Library/Application Support/Pane"
            ))
        );
        assert_eq!(
            config_path_from(None, None, None, None, home.clone()),
            Some(PathBuf::from("/Users/alex/.config/pane/config.toml"))
        );
        // Legacy SPACEMAN_APPLICATION_SUPPORT overrides both state and default config path.
        let legacy_support = Some(OsString::from("/tmp/legacy-state"));
        assert_eq!(
            application_support_dir_from(None, legacy_support.clone(), home.clone()),
            Some(PathBuf::from("/tmp/legacy-state"))
        );
        assert_eq!(
            config_path_from(None, None, None, legacy_support.clone(), home.clone()),
            Some(PathBuf::from("/tmp/legacy-state/config.toml"))
        );
        // PANE_* overrides win over SPACEMAN_* overrides.
        assert_eq!(
            application_support_dir_from(
                Some(OsString::from("/tmp/pane-state")),
                legacy_support.clone(),
                home.clone()
            ),
            Some(PathBuf::from("/tmp/pane-state"))
        );
        assert_eq!(
            config_path_from(
                Some(OsString::from("/tmp/pane.toml")),
                Some(OsString::from("/tmp/spaceman.toml")),
                Some(OsString::from("/tmp/pane-state")),
                legacy_support,
                home
            ),
            Some(PathBuf::from("/tmp/pane.toml"))
        );
    }

    #[test]
    fn migrates_legacy_dot_config_and_preserves_original() {
        let home = temp_home("dot-config");
        let legacy = home.join(".config/spaceman/config.toml");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let source = "version = 1\n[settings]\nhud_style = \"layouts\"\n";
        std::fs::write(&legacy, source).unwrap();

        let report = migrate_legacy_data_in_home(&home, true, true).unwrap();
        let migrated = home.join(".config/pane/config.toml");
        assert_eq!(report.copied_config_from.as_deref(), Some(legacy.as_path()));
        assert_eq!(std::fs::read_to_string(&migrated).unwrap(), source);
        // Legacy original stays intact.
        assert_eq!(std::fs::read_to_string(&legacy).unwrap(), source);
        assert_eq!(load_path(&migrated).unwrap().hud_style(), HudStyle::Layouts);

        // Existing Pane config is never overwritten by legacy config.
        let updated = "version = 1\n[settings]\nhud = false\n";
        std::fs::write(&migrated, updated).unwrap();
        let second = migrate_legacy_data_in_home(&home, true, true).unwrap();
        assert_eq!(second.copied_config_from, None);
        assert_eq!(std::fs::read_to_string(&migrated).unwrap(), updated);

        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn migrates_legacy_application_support_config_when_dot_config_absent() {
        let home = temp_home("app-support-config");
        let legacy = home.join("Library/Application Support/Spaceman/config.toml");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let source =
            "version = 1\n[[rules]]\nbundle_id = \"com.example.App\"\naction = \"float\"\n";
        std::fs::write(&legacy, source).unwrap();

        let report = migrate_legacy_data_in_home(&home, true, true).unwrap();
        let migrated = home.join(".config/pane/config.toml");
        assert_eq!(report.copied_config_from.as_deref(), Some(legacy.as_path()));
        assert!(load_path(&migrated).unwrap().has_rules());
        assert!(legacy.is_file());

        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn dot_config_spaceman_takes_precedence_over_app_support_spaceman_config() {
        let home = temp_home("config-precedence");
        let dot_legacy = home.join(".config/spaceman/config.toml");
        let support_legacy = home.join("Library/Application Support/Spaceman/config.toml");
        std::fs::create_dir_all(dot_legacy.parent().unwrap()).unwrap();
        std::fs::create_dir_all(support_legacy.parent().unwrap()).unwrap();
        std::fs::write(&dot_legacy, "version = 1\n[settings]\nhud = false\n").unwrap();
        std::fs::write(&support_legacy, "version = 1\n[settings]\nhud = true\n").unwrap();

        let report = migrate_legacy_data_in_home(&home, true, true).unwrap();
        assert_eq!(
            report.copied_config_from.as_deref(),
            Some(dot_legacy.as_path())
        );
        let migrated = home.join(".config/pane/config.toml");
        assert!(!load_path(&migrated).unwrap().hud());

        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn migrates_legacy_state_files_without_sockets_or_deleting_originals() {
        let home = temp_home("state-migration");
        let legacy_dir = home.join("Library/Application Support/Spaceman");
        let new_dir = home.join("Library/Application Support/Pane");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        std::fs::write(legacy_dir.join("parked-windows.json"), "[]").unwrap();
        std::fs::write(legacy_dir.join("workspace-state.json"), "{}").unwrap();
        std::fs::write(legacy_dir.join("window-memory.json"), "{}").unwrap();
        std::fs::write(legacy_dir.join("onboarding-complete"), "1\n").unwrap();
        std::fs::write(legacy_dir.join("control.sock"), "ephemeral").unwrap();
        std::fs::write(legacy_dir.join("control.lock"), "ephemeral").unwrap();
        // Simulate agent.log existing in Pane/ before migration runs.
        std::fs::create_dir_all(&new_dir).unwrap();
        std::fs::write(new_dir.join("agent.log"), "started\n").unwrap();

        let report = migrate_legacy_data_in_home(&home, true, true).unwrap();
        assert!(
            report
                .copied_state_files
                .contains(&"parked-windows.json".to_owned())
        );
        assert!(
            report
                .copied_state_files
                .contains(&"workspace-state.json".to_owned())
        );
        assert!(
            report
                .copied_state_files
                .contains(&"window-memory.json".to_owned())
        );
        assert!(
            report
                .copied_state_files
                .contains(&"onboarding-complete".to_owned())
        );
        assert!(!new_dir.join("control.sock").exists());
        assert!(!new_dir.join("control.lock").exists());
        assert!(new_dir.join(MIGRATION_MARKER).is_file());
        // Legacy originals remain intact.
        assert!(legacy_dir.join("parked-windows.json").is_file());
        assert!(legacy_dir.join("workspace-state.json").is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&new_dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(new_dir.join("parked-windows.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }

        // Subsequent runs do not overwrite Pane's updated state.
        std::fs::write(new_dir.join("parked-windows.json"), "[{\"updated\":true}]").unwrap();
        let second = migrate_legacy_data_in_home(&home, true, true).unwrap();
        assert!(second.copied_state_files.is_empty());
        assert_eq!(
            std::fs::read_to_string(new_dir.join("parked-windows.json")).unwrap(),
            "[{\"updated\":true}]"
        );

        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn migration_preserves_malformed_parked_windows_verbatim() {
        let home = temp_home("malformed-parked");
        let legacy_dir = home.join("Library/Application Support/Spaceman");
        let new_dir = home.join("Library/Application Support/Pane");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        let malformed = "{not valid json";
        std::fs::write(legacy_dir.join("parked-windows.json"), malformed).unwrap();

        let report = migrate_legacy_data_in_home(&home, true, true).unwrap();
        assert_eq!(report.copied_state_files, vec!["parked-windows.json"]);
        assert_eq!(
            std::fs::read_to_string(new_dir.join("parked-windows.json")).unwrap(),
            malformed
        );
        assert_eq!(
            std::fs::read_to_string(legacy_dir.join("parked-windows.json")).unwrap(),
            malformed
        );

        std::fs::remove_dir_all(home).unwrap();
    }
}
