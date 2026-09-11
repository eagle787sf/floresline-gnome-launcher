//! Calculator prefix (`=` / `＝`) — live preview via meval/`bc`; gnome-calculator on Enter only.
//!
//! Why gnome-calculator is not used per-keystroke: `gnome-calculator -s` is a process
//! spawn + busy-wait (up to ~3s). `parse_calc` runs from `rebuild_list` on every calc
//! keystroke on the GTK main thread, so calling it there freezes typing even though the
//! same CLI works fine in a terminal. Live UI uses meval (then `bc`); Enter may re-solve
//! with gnome-calculator off the main thread.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

/// One calculator list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalcRow {
    /// Expression (or hint) shown as the bold title.
    pub title: String,
    /// Result / error / install note shown as dim subtitle.
    pub subtitle: String,
    /// Value to copy on Enter, if any.
    pub result: Option<String>,
}

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

/// True when `gnome-calculator` is on PATH (cached).
pub fn has_gnome_calculator() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| which("gnome-calculator").is_some())
}

fn has_bc() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| which("bc").is_some())
}

fn debug_log(query: &str, backend: &str) {
    if std::env::var_os("FLORESLINE_DEBUG").is_none() {
        return;
    }
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/floresline-calc.log")
        .and_then(|mut f| writeln!(f, "query={query:?} backend={backend}"));
}

/// Strip `=` / `＝` prefix (space optional). Returns `None` if not a calc query.
pub fn strip_calc_prefix(query: &str) -> Option<&str> {
    let q = query.trim();
    if let Some(rest) = q.strip_prefix('=') {
        Some(rest.trim())
    } else if let Some(rest) = q.strip_prefix('＝') {
        Some(rest.trim())
    } else {
        None
    }
}

/// True when the query is (or is becoming) a calculator expression.
pub fn is_calc_query(query: &str) -> bool {
    let t = query.trim_start();
    t.starts_with('=') || t.starts_with('＝')
}

fn format_meval(v: f64) -> String {
    if (v - v.round()).abs() < 1e-10 && v.abs() < 1e15 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v}")
    }
}

/// Spawn `gnome-calculator -s` with a busy-wait timeout. Call off the GTK main thread.
pub fn eval_gnome_calculator(expr: &str) -> Result<String, String> {
    eval_gnome_calculator_timeout(expr, Duration::from_secs(3))
}

fn eval_gnome_calculator_timeout(expr: &str, timeout: Duration) -> Result<String, String> {
    let bin = which("gnome-calculator").ok_or_else(|| "gnome-calculator not found".to_string())?;
    let mut child = Command::new(bin)
        .args(["-s", expr])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("gnome-calculator timed out".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e.to_string()),
        }
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() && !stdout.is_empty() && !stdout.to_lowercase().starts_with("error")
    {
        return Ok(stdout);
    }

    let hint = if !stdout.is_empty() {
        stdout
    } else if !stderr.is_empty() {
        stderr
            .lines()
            .find(|l| l.to_lowercase().contains("error") || !l.starts_with("**"))
            .unwrap_or("invalid expression")
            .to_string()
    } else {
        "invalid expression".into()
    };
    Err(hint)
}

fn eval_meval(expr: &str) -> Result<String, ()> {
    match meval::eval_str(expr) {
        Ok(v) => Ok(format_meval(v)),
        Err(_) => Err(()),
    }
}

fn eval_bc(expr: &str) -> Result<String, String> {
    let bin = which("bc").ok_or_else(|| "bc not found".to_string())?;
    let script = format!("scale=9; {expr}\n");
    let output = Command::new(bin)
        .arg("-l")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(script.as_bytes())?;
            }
            child.wait_with_output()
        })
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() && !stdout.is_empty() && stderr.is_empty() {
        if let Ok(v) = stdout.parse::<f64>() {
            return Ok(format_meval(v));
        }
        return Ok(stdout);
    }
    Err(if !stderr.is_empty() {
        stderr
    } else {
        "invalid expression".into()
    })
}

/// Fast live-preview backends only — never spawns gnome-calculator.
fn eval_live(expr: &str) -> Result<(String, &'static str), String> {
    if let Ok(v) = eval_meval(expr) {
        return Ok((v, "meval"));
    }
    if has_bc() {
        if let Ok(v) = eval_bc(expr) {
            return Ok((v, "bc"));
        }
    }
    Err("invalid expression".into())
}

/// Parse `=` / `＝` calculator prefix into a list row (live / rebuild path).
///
/// Uses meval then `bc` only — never gnome-calculator (see module docs).
/// Returns `None` if the query is not a calc prefix.
pub fn parse_calc(query: &str) -> Option<CalcRow> {
    let expr = strip_calc_prefix(query)?;
    if expr.is_empty() {
        let subtitle = if has_gnome_calculator() {
            "e.g. 2+2 · Enter copies result".to_string()
        } else {
            "e.g. 2+2 · install gnome-calculator for GNOME solve".to_string()
        };
        debug_log(query, "hint");
        return Some(CalcRow {
            title: "= type expression".to_string(),
            subtitle,
            result: None,
        });
    }

    match eval_live(expr) {
        Ok((result, backend)) => {
            debug_log(query, backend);
            let subtitle = if has_gnome_calculator() {
                result.clone()
            } else {
                format!("{result}  ·  uses built-in (apt install gnome-calculator)")
            };
            Some(CalcRow {
                title: expr.to_string(),
                subtitle,
                result: Some(result),
            })
        }
        Err(err) => {
            debug_log(query, "invalid");
            let short = err.lines().next().unwrap_or("invalid expression");
            let short = if short.len() > 80 {
                format!("{}…", &short[..77])
            } else {
                short.to_string()
            };
            Some(CalcRow {
                title: expr.to_string(),
                subtitle: format!("invalid · {short}"),
                result: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_prefix_ascii_and_fullwidth() {
        assert_eq!(strip_calc_prefix("=2+2"), Some("2+2"));
        assert_eq!(strip_calc_prefix("= 2+2"), Some("2+2"));
        assert_eq!(strip_calc_prefix("＝3*3"), Some("3*3"));
        assert_eq!(strip_calc_prefix("＝ 3*3"), Some("3*3"));
        assert_eq!(strip_calc_prefix("="), Some(""));
        assert_eq!(strip_calc_prefix("＝"), Some(""));
        assert_eq!(strip_calc_prefix("=   "), Some(""));
        assert!(strip_calc_prefix("hello").is_none());
        assert!(strip_calc_prefix("").is_none());
    }

    #[test]
    fn is_calc_detects_prefix() {
        assert!(is_calc_query("=2"));
        assert!(is_calc_query("  =2"));
        assert!(is_calc_query("＝1"));
        assert!(!is_calc_query("2+2"));
        assert!(!is_calc_query(""));
    }

    #[test]
    fn bare_equals_hint() {
        let row = parse_calc("=").unwrap();
        assert!(row.result.is_none());
        assert!(row.title.contains("type expression"));
        assert!(!row.subtitle.is_empty());

        let row = parse_calc("=   ").unwrap();
        assert!(row.result.is_none());
    }

    #[test]
    fn calc_basic_arithmetic() {
        let row = parse_calc("=2+2").unwrap();
        assert_eq!(row.result.as_deref(), Some("4"));
        assert_eq!(row.title, "2+2");
        assert!(row.subtitle.contains('4'));

        let row = parse_calc("= 2+2").unwrap();
        assert_eq!(row.result.as_deref(), Some("4"));

        let row = parse_calc("＝3*3").unwrap();
        assert_eq!(row.result.as_deref(), Some("9"));
        assert_eq!(row.title, "3*3");
    }

    #[test]
    fn calc_invalid_no_copy() {
        let row = parse_calc("=2+").unwrap();
        assert!(row.result.is_none());
        assert!(row.subtitle.to_lowercase().contains("invalid"));
        assert!(parse_calc("hello").is_none());
    }

    #[test]
    fn live_preview_does_not_require_gnome() {
        // meval path must work regardless of gnome-calculator.
        let row = parse_calc("=10/2").unwrap();
        assert_eq!(row.result.as_deref(), Some("5"));
        assert_eq!(row.title, "10/2");
    }

    #[test]
    fn gnome_calculator_solve_when_present() {
        if !has_gnome_calculator() {
            return;
        }
        let v = eval_gnome_calculator("10/2").expect("gnome-calculator -s");
        assert_eq!(v.trim(), "5");
    }
}
