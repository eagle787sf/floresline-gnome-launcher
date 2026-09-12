//! User settings from `~/.config/floresline-launcher/config.toml`.
//! Missing file = sensible defaults. Copy `config.example.toml` from the repo to start.

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

/// Defaults match the lean Floresline experience (Super+\ recommended; Super alone opt-in).
#[derive(Debug, Clone)]
pub struct Config {
    pub max_results: usize,
    pub hint: String,
    pub preferred_binding: String,
    pub enable_super_alone_extension: bool,
    pub bind_super_slash: bool,
    pub bind_xf86_search: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_results: 9,
            hint: "?/ddg/gs search · Alt+1-9 · ↑↓ · Enter · Esc".into(),
            preferred_binding: "Super+backslash".into(),
            enable_super_alone_extension: false,
            bind_super_slash: true,
            bind_xf86_search: true,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct File {
    #[serde(default)]
    max_results: Option<u32>,
    #[serde(default)]
    hint: Option<String>,
    #[serde(default)]
    preferred_binding: Option<String>,
    #[serde(default)]
    enable_super_alone_extension: Option<bool>,
    #[serde(default)]
    bind_super_slash: Option<bool>,
    #[serde(default)]
    bind_xf86_search: Option<bool>,
}

fn config_path() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("floresline-launcher/config.toml");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    home.join(".config/floresline-launcher/config.toml")
}

/// Load settings if present; otherwise defaults. Invalid TOML logs and falls back.
pub fn load() -> Config {
    let path = config_path();
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Config::default(),
    };
    let file: File = match toml::from_str(&text) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("config.toml: {e}");
            return Config::default();
        }
    };
    let mut cfg = Config::default();
    if let Some(n) = file.max_results {
        cfg.max_results = (n as usize).clamp(1, 50);
    }
    if let Some(h) = file.hint {
        let h = h.trim().to_string();
        if !h.is_empty() {
            cfg.hint = h;
        }
    }
    if let Some(b) = file.preferred_binding {
        let b = b.trim().to_string();
        if !b.is_empty() {
            cfg.preferred_binding = b;
        }
    }
    if let Some(v) = file.enable_super_alone_extension {
        cfg.enable_super_alone_extension = v;
    }
    if let Some(v) = file.bind_super_slash {
        cfg.bind_super_slash = v;
    }
    if let Some(v) = file.bind_xf86_search {
        cfg.bind_xf86_search = v;
    }
    cfg
}
