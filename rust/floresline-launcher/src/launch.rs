//! Launch selected desktop applications.
//!
//! TODO: exec with proper argv splitting, %f/%u expansion, and Terminal= handling.

use crate::desktop::DesktopApp;

/// Launch `app` (stub — does not exec yet).
pub fn launch_app(app: &DesktopApp) -> Result<(), String> {
    // TODO: parse Exec=, spawn process, close launcher on success
    Err(format!(
        "launch not implemented yet (would run: {} [{}])",
        app.exec, app.desktop_id
    ))
}
