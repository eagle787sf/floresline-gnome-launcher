//! Fuzzy matching / ranking for launcher queries.
//!
//! TODO: implement a real score against app names and keywords.

use crate::desktop::DesktopApp;

/// Score how well `query` matches `app`. Higher is better; `None` = no match.
///
/// Currently a stub that treats empty queries as matching everything with score 0.
pub fn score(query: &str, app: &DesktopApp) -> Option<i32> {
    let q = query.trim();
    if q.is_empty() {
        return Some(0);
    }
    // TODO: fuzzy score against app.name / keywords
    let _ = app;
    None
}

/// Filter and sort apps by fuzzy score (stub).
pub fn rank(query: &str, apps: &[DesktopApp]) -> Vec<(i32, DesktopApp)> {
    let mut scored: Vec<(i32, DesktopApp)> = apps
        .iter()
        .filter_map(|app| score(query, app).map(|s| (s, app.clone())))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored
}
