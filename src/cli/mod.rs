use std::{path::Path, process::ExitCode};

pub(crate) mod config;
pub(crate) mod document;
pub(crate) mod help;
pub(crate) mod rules;
pub(crate) mod workspaces;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Outcome {
    pub exit_code: u8,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl Outcome {
    pub(crate) fn ok(stdout: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            stdout: Some(stdout.into()),
            stderr: None,
        }
    }

    pub(crate) fn err(stderr: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            stdout: None,
            stderr: Some(stderr.into()),
        }
    }

    pub(crate) fn usage(stderr: impl Into<String>) -> Self {
        Self {
            exit_code: 2,
            stdout: None,
            stderr: Some(stderr.into()),
        }
    }

    pub(crate) fn into_exit_code(self) -> ExitCode {
        if let Some(stdout) = self.stdout {
            println!("{stdout}");
        }
        if let Some(stderr) = self.stderr {
            eprintln!("{stderr}");
        }
        ExitCode::from(self.exit_code)
    }
}

pub(crate) fn try_run(executable: &str, command: &str, rest: &[&str]) -> Option<Outcome> {
    try_run_with_config_path(executable, command, rest, None)
}

pub(crate) fn try_run_with_config_path(
    executable: &str,
    command: &str,
    rest: &[&str],
    config_path: Option<&Path>,
) -> Option<Outcome> {
    match command {
        "-h" | "--help" => return Some(Outcome::ok(help::root(executable))),
        "-V" | "--version" => return Some(Outcome::ok(help::version())),
        "help" => {
            return Some(match rest {
                [] => Outcome::ok(help::root(executable)),
                [sub] | [sub, ..] => match help::subcommand(executable, sub) {
                    Some(text) => Outcome::ok(text),
                    None => Outcome::usage(format!(
                        "unknown command: {sub}\n\n{}",
                        help::root_usage(executable)
                    )),
                },
            });
        }
        "config" => return Some(config::run(executable, rest, config_path)),
        "rules" => return Some(rules::run(executable, rest, config_path)),
        "workspaces" => return Some(workspaces::run(executable, rest, config_path)),
        _ => {}
    }

    let asks_help = matches!(rest, ["-h" | "--help"])
        || (command == "ctl"
            && (rest == ["help"] || rest.iter().any(|arg| help::is_help_flag(arg))));
    if asks_help && let Some(text) = help::subcommand(executable, command) {
        return Some(Outcome::ok(text));
    }

    None
}
