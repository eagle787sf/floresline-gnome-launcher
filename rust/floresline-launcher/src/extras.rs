//! Optional custom commands from `~/.config/floresline-launcher/extras.toml`.
//! Missing file = no-op. Never required for core launcher features.

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ExtraCommand {
    pub prefix: String,
    pub label: String,
    pub exec: String,
}

#[derive(Debug, Default, Deserialize)]
struct ExtrasFile {
    #[serde(default)]
    commands: Vec<ExtraCommand>,
}

fn config_path() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("floresline-launcher/extras.toml");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    home.join(".config/floresline-launcher/extras.toml")
}

/// Load custom commands if the config file exists; otherwise empty.
pub fn load() -> Vec<ExtraCommand> {
    let path = config_path();
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    match toml::from_str::<ExtrasFile>(&text) {
        Ok(f) => f.commands,
        Err(e) => {
            eprintln!("extras.toml: {e}");
            Vec::new()
        }
    }
}

/// True when query equals prefix or starts with `prefix` + whitespace/end.
pub fn matches(query: &str, prefix: &str) -> bool {
    let q = query.trim();
    if q.is_empty() || prefix.is_empty() {
        return false;
    }
    let ql = q.to_lowercase();
    let pl = prefix.to_lowercase();
    ql == pl || ql.starts_with(&(pl + " "))
}
