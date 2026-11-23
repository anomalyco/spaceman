use std::path::{Path, PathBuf};

use serde::Serialize;
use spaceman::config::{HudStyle, load_path};

use super::{
    Outcome,
    document::{
        ConfigDoc, EffectiveSettings, RuleView, WorkspaceDoc, resolve_config_path, to_json_pretty,
    },
    help,
};

#[derive(Serialize)]
struct PathOutput {
    path: String,
    exists: bool,
}

#[derive(Serialize)]
struct ShowOutput {
    path: String,
    exists: bool,
    version: u32,
    settings: EffectiveSettings,
    rules: Vec<RuleView>,
    workspaces: Vec<WorkspaceDoc>,
}

#[derive(Serialize)]
struct SettingOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    key: &'static str,
    value: serde_json::Value,
}

#[derive(Serialize)]
struct CheckOutput {
    path: String,
    exists: bool,
    valid: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingKey {
    Hud,
    HudStyle,
}

impl SettingKey {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.strip_prefix("settings.").unwrap_or(raw) {
            "hud" => Ok(Self::Hud),
            "hud_style" => Ok(Self::HudStyle),
            _ => Err(format!(
                "unknown setting {raw:?}; valid keys: hud, hud_style"
            )),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Hud => "hud",
            Self::HudStyle => "hud_style",
        }
    }
}

pub(crate) fn run(executable: &str, args: &[&str], path_override: Option<&Path>) -> Outcome {
    let Some((&sub, rest)) = args.split_first() else {
        return Outcome::usage(help::config_help(executable));
    };
    if help::is_help_flag(sub) || sub == "help" || rest.iter().any(|arg| help::is_help_flag(arg)) {
        return Outcome::ok(help::config_help(executable));
    }

    let result = match sub {
        "path" => run_path(rest, path_override),
        "show" => run_show(rest, path_override),
        "get" => run_get(rest, path_override),
        "set" => run_set(rest, path_override),
        "check" => run_check(rest, path_override),
        _ => {
            return Outcome::usage(format!(
                "unknown config subcommand: {sub}\n\n{}",
                help::config_help(executable)
            ));
        }
    };

    match result {
        Ok(output) => Outcome::ok(output),
        Err(error) => Outcome::err(error),
    }
}

fn split_json_flag<'a>(args: &[&'a str]) -> Result<(bool, Vec<&'a str>), String> {
    let mut json = false;
    let mut positional = Vec::new();
    let mut after_double_dash = false;
    for &arg in args {
        if !after_double_dash && arg == "--" {
            after_double_dash = true;
            continue;
        }
        if !after_double_dash && arg == "--json" {
            json = true;
            continue;
        }
        if !after_double_dash && arg.starts_with('-') {
            return Err(format!("unknown option: {arg}"));
        }
        positional.push(arg);
    }
    Ok((json, positional))
}

fn run_path(args: &[&str], path_override: Option<&Path>) -> Result<String, String> {
    let (json, positional) = split_json_flag(args)?;
    if !positional.is_empty() {
        return Err(format!("unexpected argument: {}", positional[0]));
    }
    let path = resolve_config_path(path_override)?;
    if json {
        to_json_pretty(&PathOutput {
            path: path.display().to_string(),
            exists: path.exists(),
        })
    } else {
        Ok(path.display().to_string())
    }
}

fn run_show(args: &[&str], path_override: Option<&Path>) -> Result<String, String> {
    let (json, positional) = split_json_flag(args)?;
    if !positional.is_empty() {
        return Err(format!("unexpected argument: {}", positional[0]));
    }
    let path = resolve_config_path(path_override)?;
    let (doc, exists) = ConfigDoc::load_or_default(&path)?;
    if json {
        to_json_pretty(&ShowOutput {
            path: path.display().to_string(),
            exists,
            version: doc.version,
            settings: doc.effective_settings(),
            rules: doc.rule_views(),
            workspaces: doc.workspaces,
        })
    } else {
        doc.render_effective_toml()
            .map(|rendered| rendered.trim_end().to_owned())
    }
}

fn run_get(args: &[&str], path_override: Option<&Path>) -> Result<String, String> {
    let (json, positional) = split_json_flag(args)?;
    let [raw_key] = positional.as_slice() else {
        return Err("usage: spaceman config get <key> [--json]".into());
    };
    let key = SettingKey::parse(raw_key)?;
    let path = resolve_config_path(path_override)?;
    let (doc, _) = ConfigDoc::load_or_default(&path)?;
    let (text, value) = match key {
        SettingKey::Hud => {
            let hud = doc.settings.effective_hud();
            (hud.to_string(), serde_json::Value::Bool(hud))
        }
        SettingKey::HudStyle => {
            let style = doc.settings.effective_hud_style().as_str();
            (
                style.to_owned(),
                serde_json::Value::String(style.to_owned()),
            )
        }
    };
    if json {
        to_json_pretty(&SettingOutput {
            path: None,
            key: key.as_str(),
            value,
        })
    } else {
        Ok(text)
    }
}

fn run_set(args: &[&str], path_override: Option<&Path>) -> Result<String, String> {
    let (json, positional) = split_json_flag(args)?;
    let [raw_key, raw_value] = positional.as_slice() else {
        return Err("usage: spaceman config set <key> <value> [--json]".into());
    };
    let key = SettingKey::parse(raw_key)?;
    let path = resolve_config_path(path_override)?;
    let (mut doc, _) = ConfigDoc::load_or_default(&path)?;

    let (text, value) = match key {
        SettingKey::Hud => {
            let parsed = match *raw_value {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(format!(
                        "invalid value for hud: expected 'true' or 'false', got {raw_value:?}"
                    ));
                }
            };
            doc.settings.hud = Some(parsed);
            (
                format!("set hud = {parsed}"),
                serde_json::Value::Bool(parsed),
            )
        }
        SettingKey::HudStyle => {
            let parsed = HudStyle::parse(raw_value).ok_or_else(|| {
                format!(
                    "invalid value for hud_style: expected 'icons' or 'layouts', got {raw_value:?}"
                )
            })?;
            doc.settings.hud_style = Some(parsed);
            (
                format!("set hud_style = {}", parsed.as_str()),
                serde_json::Value::String(parsed.as_str().to_owned()),
            )
        }
    };

    doc.save_atomic(&path)?;
    if json {
        to_json_pretty(&SettingOutput {
            path: Some(path.display().to_string()),
            key: key.as_str(),
            value,
        })
    } else {
        Ok(text)
    }
}

fn run_check(args: &[&str], path_override: Option<&Path>) -> Result<String, String> {
    let (json, positional) = split_json_flag(args)?;
    let (path, explicit): (PathBuf, bool) = match positional.as_slice() {
        [] => (resolve_config_path(path_override)?, false),
        [target] => (PathBuf::from(target), true),
        _ => return Err("usage: spaceman config check [path] [--json]".into()),
    };

    let exists = if explicit {
        load_path(&path)?;
        true
    } else {
        let (_, exists) = ConfigDoc::load_or_default(&path)?;
        exists
    };

    if json {
        to_json_pretty(&CheckOutput {
            path: path.display().to_string(),
            exists,
            valid: true,
        })
    } else if exists {
        Ok("configuration is valid".into())
    } else {
        Ok(format!(
            "configuration is valid (using defaults; no file at {})",
            path.display()
        ))
    }
}
