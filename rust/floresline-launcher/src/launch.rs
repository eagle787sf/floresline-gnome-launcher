//! Launch selected desktop applications (mirrors Python `launch`).

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::desktop::AppEntry;

fn which(cmd: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let full = dir.join(cmd);
        if full.is_file() {
            return Some(full);
        }
    }
    None
}

fn spawn_detached(mut cmd: Command) -> Result<(), String> {
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Launch `app` via terminal / gtk-launch / shlex-split Exec.
pub fn launch_app(app: &AppEntry) -> Result<(), String> {
    if app.terminal {
        if let Some(term) = which("ptyxis") {
            let mut c = Command::new(&term);
            c.args(["--", "bash", "-lc", &app.exec_cmd]);
            return spawn_detached(c);
        }
        if let Some(term) = which("gnome-terminal") {
            let mut c = Command::new(&term);
            c.args(["-e", &app.exec_cmd]);
            return spawn_detached(c);
        }
        let argv = shlex::split(&app.exec_cmd)
            .ok_or_else(|| format!("failed to parse Exec: {}", app.exec_cmd))?;
        if argv.is_empty() {
            return Err("empty Exec".into());
        }
        let mut c = Command::new(&argv[0]);
        c.args(&argv[1..]);
        return spawn_detached(c);
    }

    let id = app
        .desktop_id
        .strip_suffix(".desktop")
        .unwrap_or(&app.desktop_id);
    let mut gtk_launch = Command::new("gtk-launch");
    gtk_launch.arg(id);
    if spawn_detached(gtk_launch).is_ok() {
        return Ok(());
    }

    let argv = shlex::split(&app.exec_cmd)
        .ok_or_else(|| format!("failed to parse Exec: {}", app.exec_cmd))?;
    if argv.is_empty() {
        return Err("empty Exec".into());
    }
    let mut c = Command::new(&argv[0]);
    c.args(&argv[1..]);
    spawn_detached(c)
}
