//! Fuzzy matching / ranking — same algorithm as Python `fuzzy_score`, plus recents boost.

use crate::desktop::AppEntry;
use crate::recents::Recents;

/// Score how well `query` matches `app`. Higher is better; `None` = no match.
pub fn score(query: &str, app: &AppEntry) -> Option<i32> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    let name = app.name.to_lowercase();
    let h = app.haystack();

    if name.starts_with(&q) {
        return Some(1000 - name.len() as i32);
    }
    if let Some(pos) = name.find(&q) {
        return Some(800 - pos as i32);
    }
    // all tokens in haystack
    if q.split_whitespace().all(|tok| h.contains(tok)) {
        return Some(500);
    }
    // subsequence: every char of q appears in order in h
    let mut it = h.chars();
    if q.chars().all(|ch| it.any(|c| c == ch)) {
        return Some(200 - h.len() as i32);
    }
    None
}

/// Filter and sort apps by fuzzy score + recents bonus (descending), then name.
/// Empty query prefers recents at the top.
pub fn rank(query: &str, apps: &[AppEntry], recents: &Recents) -> Vec<(i32, AppEntry)> {
    let mut scored: Vec<(i32, AppEntry)> = apps
        .iter()
        .filter_map(|app| {
            score(query, app).map(|s| {
                let bonus = recents.bonus(&app.desktop_id);
                (s + bonus, app.clone())
            })
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()))
    });
    scored
}
