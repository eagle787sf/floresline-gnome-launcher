//! Scan XDG application directories for `.desktop` files.
//!
//! TODO: walk system / Flatpak / Snap / `~/.local` dirs and parse entries.

/// Placeholder app entry until desktop scanning is implemented.
#[derive(Debug, Clone)]
pub struct DesktopApp {
    pub name: String,
    pub exec: String,
    pub desktop_id: String,
}

/// Scan standard application directories for launchable apps.
///
/// Currently returns an empty list (stub).
pub fn scan_apps() -> Vec<DesktopApp> {
    // TODO: scan dirs under XDG_DATA_DIRS / ~/.local/share/applications
    Vec::new()
}
