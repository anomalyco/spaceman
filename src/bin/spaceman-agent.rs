fn main() {
    redirect_stderr_to_log();
    if let Err(error) = spaceman::app::run() {
        drop(std::io::Write::write_all(
            &mut std::io::stderr(),
            format!("Spaceman failed: {error}\n").as_bytes(),
        ));
        std::process::exit(1);
    }
}

/// When launched as an app bundle, stderr goes nowhere. Redirect it to a
/// size-capped, owner-only log file so runtime diagnostics survive. Terminal
/// launches keep stderr on the terminal.
fn redirect_stderr_to_log() {
    use std::{
        io::IsTerminal,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };
    if std::io::stderr().is_terminal() {
        return;
    }
    let Some(path) = spaceman::config::application_support_path("agent.log") else {
        return;
    };
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
    if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 5 * 1024 * 1024) {
        // Keep one previous generation: the history before a restart is
        // usually what explains it.
        drop(std::fs::rename(&path, path.with_extension("log.1")));
    }
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&path)
    else {
        return;
    };
    drop(file.set_permissions(std::fs::Permissions::from_mode(0o600)));
    spaceman::macos::redirect_stderr_to(&file);
}
