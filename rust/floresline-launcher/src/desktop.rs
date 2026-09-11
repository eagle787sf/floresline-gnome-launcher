//! Scan XDG application directories for `.desktop` files (mirrors Python).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec_cmd: String,
    pub desktop_id: String,
    pub comment: String,
    pub keywords: String,
    pub icon: String,
    pub terminal: bool,
}

impl AppEntry {
    pub fn haystack(&self) -> String {
        format!(
            "{} {} {} {}",
            self.name, self.comment, self.keywords, self.desktop_id
        )
        .to_lowercase()
    }
}

fn desktop_dirs() -> Vec<PathBuf> {
    let home = dirs_home();
    vec![
        home.join(".local/share/applications"),
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        home.join(".local/share/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ]
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Strip Exec field codes: `\s*%[fFuUdDnNickvm]` (same as Python).
fn strip_exec_codes(exec_cmd: &str) -> String {
    let chars: Vec<char> = exec_cmd.chars().collect();
    let mut out = String::with_capacity(exec_cmd.len());
    let mut i = 0;
    while i < chars.len() {
        let mut j = i;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j + 1 < chars.len() && chars[j] == '%' {
            let code = chars[j + 1];
            if "fFuUdDnNickvm".contains(code) {
                // \s*%X — consume optional whitespace + field code
                i = j + 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out.trim().to_string()
}

pub fn parse_desktop(path: &Path) -> Option<AppEntry> {
    let text = fs::read_to_string(path).ok()?;
    if !text.contains("[Desktop Entry]") {
        return None;
    }
    let mut section: Vec<&str> = Vec::new();
    let mut in_entry = false;
    for line in text.lines() {
        if line.trim() == "[Desktop Entry]" {
            in_entry = true;
            continue;
        }
        if line.starts_with('[') && in_entry {
            break;
        }
        if in_entry {
            section.push(line);
        }
    }
    let mut data: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for line in section {
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let (k, v) = line.split_once('=')?;
        data.insert(k.trim(), v.trim());
    }
    if data.get("Type").copied().unwrap_or("Application") != "Application" {
        return None;
    }
    if data
        .get("NoDisplay")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return None;
    }
    if data
        .get("Hidden")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return None;
    }
    let name = data.get("Name")?.to_string();
    let exec_raw = data.get("Exec")?.to_string();
    let exec_cmd = strip_exec_codes(&exec_raw);
    if exec_cmd.is_empty() {
        return None;
    }
    let desktop_id = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let comment = data.get("Comment").unwrap_or(&"").to_string();
    let keywords = data
        .get("Keywords")
        .unwrap_or(&"")
        .replace(';', " ");
    let icon = data
        .get("Icon")
        .copied()
        .unwrap_or("application-x-executable")
        .to_string();
    let terminal = data
        .get("Terminal")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    Some(AppEntry {
        name,
        exec_cmd,
        desktop_id,
        comment,
        keywords,
        icon,
        terminal,
    })
}

pub fn load_apps() -> Vec<AppEntry> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut apps: Vec<AppEntry> = Vec::new();
    for d in desktop_dirs() {
        if !d.is_dir() {
            continue;
        }
        let mut entries: Vec<PathBuf> = match fs::read_dir(&d) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e == "desktop")
                        .unwrap_or(false)
                })
                .collect(),
            Err(_) => continue,
        };
        entries.sort();
        for p in entries {
            let name = match p.file_name().and_then(|s| s.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if seen.contains(&name) {
                continue;
            }
            if let Some(entry) = parse_desktop(&p) {
                seen.insert(name);
                apps.push(entry);
            }
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

#[cfg(test)]
mod tests {
    use super::strip_exec_codes;

    #[test]
    fn strips_field_codes() {
        assert_eq!(strip_exec_codes("foo %u %F"), "foo");
        assert_eq!(strip_exec_codes("%f bar"), "bar");
        assert_eq!(strip_exec_codes("app --flag"), "app --flag");
    }
}
