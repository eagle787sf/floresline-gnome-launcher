//! Persist launch counts / timestamps for ranking boost.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentEntry {
    pub count: u32,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Recents {
    #[serde(default)]
    pub entries: HashMap<String, RecentEntry>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn state_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(xdg).join("floresline-launcher");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    home.join(".local/state/floresline-launcher")
}

fn path() -> PathBuf {
    state_dir().join("recents.json")
}

impl Recents {
    pub fn load() -> Self {
        let p = path();
        match fs::read_to_string(&p) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let dir = state_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("recents: mkdir: {e}");
            return;
        }
        let p = path();
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&p, json) {
                    eprintln!("recents: write: {e}");
                }
            }
            Err(e) => eprintln!("recents: serialize: {e}"),
        }
    }

    pub fn record(&mut self, desktop_id: &str) {
        if desktop_id.is_empty() {
            return;
        }
        let e = self.entries.entry(desktop_id.to_string()).or_default();
        e.count = e.count.saturating_add(1);
        e.last_used = now_secs();
        self.save();
    }

    /// Ranking bonus: count + recency.
    pub fn bonus(&self, desktop_id: &str) -> i32 {
        let Some(e) = self.entries.get(desktop_id) else {
            return 0;
        };
        let count_bonus = (e.count as i32 * 20).min(200);
        let age = now_secs().saturating_sub(e.last_used);
        let recency = if age < 3_600 {
            50
        } else if age < 86_400 {
            30
        } else if age < 604_800 {
            15
        } else if e.count > 0 {
            5
        } else {
            0
        };
        count_bonus + recency
    }
}
