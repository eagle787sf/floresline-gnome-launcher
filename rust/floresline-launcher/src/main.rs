//! Floresline GNOME launcher — Rust + GTK4 (Pop-style prefixes + recents).

mod desktop;
mod extras;
mod fuzzy;
mod launch;
mod recents;

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gtk::gdk::{Display, Key, ModifierType};
use gtk::glib::{self, ControlFlow};
use gtk::pango;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, CssProvider, Entry, EventControllerFocus,
    EventControllerKey, Image, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    PropagationPhase, ScrolledWindow, SelectionMode,
};

const APP_ID: &str = "dev.floresline.Launcher";

const CSS: &str = r#"
window {
  background-color: rgba(15, 31, 20, 0.96);
  color: #e8ffe8;
  border-radius: 16px;
  border: 1px solid rgba(124, 191, 58, 0.45);
}
entry {
  margin: 14px 14px 8px 14px;
  padding: 12px 14px;
  font-size: 16px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.35);
  color: #f0fff0;
  caret-color: #7cbf3a;
}
list { background: transparent; margin: 0 8px 4px 8px; }
row { border-radius: 8px; margin: 2px 4px; }
row:selected { background: rgba(124, 191, 58, 0.28); }
.title { font-weight: 600; }
.dim-label { opacity: 0.65; font-size: 12px; margin: 4px 14px; }
.hotkey-badge {
  opacity: 0.45;
  font-size: 11px;
  font-weight: 600;
  min-width: 14px;
}
"#;

fn pidfile_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let uid = unsafe { raw_libc::getuid() };
            PathBuf::from(format!("/tmp/{uid}"))
        });
    dir.join("floresline-launcher.pid")
}

struct PidGuard(Option<PathBuf>);

impl PidGuard {
    fn acquire() -> Self {
        let path = pidfile_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::write(&path, std::process::id().to_string()) {
            Ok(()) => Self(Some(path)),
            Err(e) => {
                eprintln!("pidfile warning: {e}");
                Self(None)
            }
        }
    }
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_file(path);
        }
    }
}

/// Visible result row: app or a special action (calc / web / URL / extra).
#[derive(Clone)]
enum ResultItem {
    App(desktop::AppEntry),
    Calc { display: String, result: Option<String> },
    Web { title: String, url: String },
    Extra { label: String, exec: String },
}

struct UiState {
    results: RefCell<Vec<ResultItem>>,
    refresh_id: Cell<Option<glib::SourceId>>,
    ignore_changed: Cell<bool>,
    recents: RefCell<recents::Recents>,
    extras: Vec<extras::ExtraCommand>,
}

fn apply_css() {
    let provider = CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn focus_entry(entry: &Entry) {
    entry.grab_focus();
    let text = entry.text();
    let len = text.len() as i32;
    entry.select_region(len, len);
    entry.set_position(len);
}

fn copy_to_clipboard(text: &str) {
    if let Some(display) = Display::default() {
        display.clipboard().set_text(text);
    }
    // Wayland: GTK clipboard often does not stick after immediate quit.
    let _ = launch::copy_via_wl_copy(text);
    // X11 / XWayland fallback when wl-clipboard is not installed.
    let _ = launch::copy_via_xclip(text);
}

/// Delay quit so detached children / clipboard clients can start.
fn quit_soon(app: &Application) {
    quit_after(app, 200);
}

fn quit_after(app: &Application, ms: u64) {
    let app = app.clone();
    glib::timeout_add_local_once(Duration::from_millis(ms), move || {
        app.quit();
    });
}

/// Pop-style web prefixes and bare URLs. Returns (title, url) if matched.
pub(crate) fn parse_web_action(query: &str) -> Option<(String, String)> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    let lower = q.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some((format!("Open {q}"), q.to_string()));
    }
    let (prefix, engine, base) = if lower.starts_with("ddg ") {
        ("ddg ", "DuckDuckGo", "https://duckduckgo.com/?q=")
    } else if lower.starts_with("google ") {
        ("google ", "Google", "https://www.google.com/search?q=")
    } else if lower.starts_with("gs ") {
        ("gs ", "Google", "https://www.google.com/search?q=")
    } else if lower.starts_with("? ") || (lower.starts_with('?') && lower.len() > 1) {
        let rest = if lower.starts_with("? ") {
            &q[2..]
        } else {
            &q[1..]
        };
        let rest = rest.trim();
        if rest.is_empty() {
            return None;
        }
        let url = format!(
            "https://duckduckgo.com/?q={}",
            urlencoding::encode(rest)
        );
        return Some((format!("Search DuckDuckGo: {rest}"), url));
    } else {
        return None;
    };
    let rest = q[prefix.len()..].trim();
    if rest.is_empty() {
        return None;
    }
    let url = format!("{base}{}", urlencoding::encode(rest));
    Some((format!("Search {engine}: {rest}"), url))
}

/// True when the query is (or is becoming) a calculator expression.
fn is_calc_query(query: &str) -> bool {
    let t = query.trim_start();
    t.starts_with('=') || t.starts_with('＝')
}

/// Parse `=` / `＝` calculator prefix. Returns (display, Some(result)) on success,
/// (display, None) for bare `=` hint or invalid expressions, or None if not calc.
pub(crate) fn parse_calc(query: &str) -> Option<(String, Option<String>)> {
    let q = query.trim();
    let expr = if let Some(rest) = q.strip_prefix('=') {
        rest.trim()
    } else if let Some(rest) = q.strip_prefix('＝') {
        rest.trim()
    } else {
        return None;
    };
    if expr.is_empty() {
        return Some(("= type expression (e.g. 2+2)".to_string(), None));
    }
    match meval::eval_str(expr) {
        Ok(v) => {
            let result = if (v - v.round()).abs() < 1e-10 && v.abs() < 1e15 {
                format!("{}", v.round() as i64)
            } else {
                format!("{v}")
            };
            Some((format!("= {result}"), Some(result)))
        }
        Err(_) => Some(("= (invalid expression)".to_string(), None)),
    }
}

fn rebuild_list(
    list: &ListBox,
    apps: &[desktop::AppEntry],
    query: &str,
    state: &UiState,
) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }

    let mut items: Vec<ResultItem> = Vec::new();

    if let Some((display, result)) = parse_calc(query) {
        items.push(ResultItem::Calc { display, result });
    } else if let Some((title, url)) = parse_web_action(query) {
        items.push(ResultItem::Web { title, url });
    } else {
        for ex in &state.extras {
            if extras::matches(query, &ex.prefix) {
                items.push(ResultItem::Extra {
                    label: ex.label.clone(),
                    exec: ex.exec.clone(),
                });
            }
        }
        let recents = state.recents.borrow();
        let scored = fuzzy::rank(query, apps, &recents);
        for (_, a) in scored.into_iter().take(50) {
            items.push(ResultItem::App(a));
        }
    }

    for (i, item) in items.iter().enumerate() {
        let row = ListBoxRow::new();
        row.set_can_focus(false);
        row.set_focusable(false);

        let box_ = GtkBox::new(Orientation::Horizontal, 12);
        box_.set_margin_start(10);
        box_.set_margin_end(10);
        box_.set_margin_top(6);
        box_.set_margin_bottom(6);

        if i < 9 {
            let badge = Label::new(Some(&format!("{}", i + 1)));
            badge.add_css_class("hotkey-badge");
            badge.set_width_chars(2);
            badge.set_xalign(0.5);
            box_.append(&badge);
        }

        match item {
            ResultItem::App(a) => {
                let icon = if a.icon.starts_with('/') {
                    let path = std::path::Path::new(&a.icon);
                    if path.is_file() {
                        Image::from_file(path)
                    } else {
                        Image::from_icon_name("application-x-executable")
                    }
                } else {
                    Image::from_icon_name(&a.icon)
                };
                icon.set_pixel_size(28);
                box_.append(&icon);

                let col = GtkBox::new(Orientation::Vertical, 0);
                let name = Label::new(Some(&a.name));
                name.set_xalign(0.0);
                name.add_css_class("title");
                col.append(&name);
                if !a.comment.is_empty() {
                    let c = Label::new(Some(&a.comment));
                    c.set_xalign(0.0);
                    c.add_css_class("dim-label");
                    c.set_ellipsize(pango::EllipsizeMode::End);
                    col.append(&c);
                }
                box_.append(&col);
            }
            ResultItem::Calc { display, .. } => {
                let icon = Image::from_icon_name("accessories-calculator");
                icon.set_pixel_size(28);
                box_.append(&icon);
                let name = Label::new(Some(display));
                name.set_xalign(0.0);
                name.add_css_class("title");
                box_.append(&name);
            }
            ResultItem::Web { title, .. } => {
                let icon = Image::from_icon_name("web-browser");
                icon.set_pixel_size(28);
                box_.append(&icon);
                let name = Label::new(Some(title));
                name.set_xalign(0.0);
                name.add_css_class("title");
                name.set_ellipsize(pango::EllipsizeMode::End);
                box_.append(&name);
            }
            ResultItem::Extra { label, .. } => {
                let icon = Image::from_icon_name("system-run");
                icon.set_pixel_size(28);
                box_.append(&icon);
                let name = Label::new(Some(label));
                name.set_xalign(0.0);
                name.add_css_class("title");
                name.set_ellipsize(pango::EllipsizeMode::End);
                box_.append(&name);
            }
        }

        row.set_child(Some(&box_));
        list.append(&row);
    }

    *state.results.borrow_mut() = items;
    if !state.results.borrow().is_empty() {
        if let Some(r) = list.row_at_index(0) {
            list.select_row(Some(&r));
        }
    }
}

fn move_sel(list: &ListBox, state: &UiState, entry: &Entry, delta: i32) -> bool {
    let n = state.results.borrow().len() as i32;
    if n == 0 {
        return true;
    }
    let idx = list.selected_row().map(|r| r.index()).unwrap_or(0);
    let nxt = (idx + delta).clamp(0, n - 1);
    if let Some(r) = list.row_at_index(nxt) {
        list.select_row(Some(&r));
    }
    let entry = entry.clone();
    glib::idle_add_local_once(move || focus_entry(&entry));
    true
}

fn activate_index(
    list: &ListBox,
    state: &UiState,
    app: &Application,
    entry: &Entry,
    apps: &[desktop::AppEntry],
    idx: usize,
) {
    let item = {
        let results = state.results.borrow();
        if idx >= results.len() {
            return;
        }
        results[idx].clone()
    };
    match item {
        ResultItem::App(a) => {
            state.recents.borrow_mut().record(&a.desktop_id);
            if let Err(e) = launch::launch_app(&a) {
                eprintln!("launch_app({}): {e}", a.name);
            }
            quit_soon(app);
        }
        ResultItem::Calc { result, .. } => {
            let Some(result) = result else {
                // Hint / invalid expression row — no-op on Enter.
                return;
            };
            // Show the answer in the entry and calc row before quitting so
            // Enter feels successful even when clipboard helpers are missing.
            state.ignore_changed.set(true);
            entry.set_text(&result);
            state.ignore_changed.set(false);
            let calc_q = format!("={result}");
            rebuild_list(list, apps, &calc_q, state);
            copy_to_clipboard(&result);
            let _ = launch::notify_send("Floresline calc", &format!("Copied: {result}"));
            // Stay open long enough for notify-send + clipboard clients on Wayland.
            quit_after(app, 900);
        }
        ResultItem::Web { url, .. } => {
            if let Err(e) = launch::open_uri(&url) {
                eprintln!("open_uri: {e}");
            }
            // Give xdg-open time to start before GTK tears down.
            quit_soon(app);
        }
        ResultItem::Extra { exec, .. } => {
            if let Err(e) = launch::run_shell(&exec) {
                eprintln!("run_shell: {e}");
            }
            quit_soon(app);
        }
    }
}

fn launch_selected(
    list: &ListBox,
    state: &UiState,
    app: &Application,
    entry: &Entry,
    apps: &[desktop::AppEntry],
) {
    let idx = list.selected_row().map(|r| r.index()).unwrap_or(0) as usize;
    activate_index(list, state, app, entry, apps, idx);
}

fn alt_digit_index(keyval: Key, modifiers: ModifierType) -> Option<usize> {
    if !modifiers.contains(ModifierType::ALT_MASK) {
        return None;
    }
    if modifiers.contains(ModifierType::CONTROL_MASK)
        || modifiers.contains(ModifierType::SUPER_MASK)
    {
        return None;
    }
    match keyval {
        Key::_1 | Key::KP_1 => Some(0),
        Key::_2 | Key::KP_2 => Some(1),
        Key::_3 | Key::KP_3 => Some(2),
        Key::_4 | Key::KP_4 => Some(3),
        Key::_5 | Key::KP_5 => Some(4),
        Key::_6 | Key::KP_6 => Some(5),
        Key::_7 | Key::KP_7 => Some(6),
        Key::_8 | Key::KP_8 => Some(7),
        Key::_9 | Key::KP_9 => Some(8),
        _ => None,
    }
}

fn build_ui(app: &Application, apps: Rc<Vec<desktop::AppEntry>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Launcher")
        .default_width(640)
        .default_height(440)
        .resizable(false)
        .decorated(false)
        .modal(true)
        .build();

    let state = Rc::new(UiState {
        results: RefCell::new(Vec::new()),
        refresh_id: Cell::new(None),
        ignore_changed: Cell::new(false),
        recents: RefCell::new(recents::Recents::load()),
        extras: extras::load(),
    });

    let outer = GtkBox::new(Orientation::Vertical, 0);
    window.set_child(Some(&outer));

    let entry = Entry::new();
    entry.set_placeholder_text(Some("Type to search apps…"));
    entry.set_hexpand(true);
    entry.set_can_focus(true);
    outer.append(&entry);

    let keys = EventControllerKey::new();
    keys.set_propagation_phase(PropagationPhase::Capture);
    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .min_content_height(320)
        .can_focus(false)
        .build();
    outer.append(&scrolled);

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::Browse);
    list.set_can_focus(false);
    list.set_activate_on_single_click(true);
    scrolled.set_child(Some(&list));

    let hint = Label::new(Some(
        "= 2+2 calc · ?/ddg/gs search · Alt+1-9 · ↑↓ · Enter · Esc",
    ));
    hint.add_css_class("dim-label");
    hint.set_margin_top(6);
    hint.set_margin_bottom(8);
    outer.append(&hint);

    {
        let app_c = app.clone();
        let list_c = list.clone();
        let state_c = state.clone();
        let entry_c = entry.clone();
        let apps_c = apps.clone();
        keys.connect_key_pressed(move |_, keyval, _code, modifiers| {
            if keyval == Key::Escape {
                app_c.quit();
                return glib::Propagation::Stop;
            }
            if let Some(n) = alt_digit_index(keyval, modifiers) {
                activate_index(&list_c, &state_c, &app_c, &entry_c, &apps_c, n);
                return glib::Propagation::Stop;
            }
            if keyval == Key::Down || keyval == Key::KP_Down || keyval == Key::Tab {
                move_sel(&list_c, &state_c, &entry_c, 1);
                return glib::Propagation::Stop;
            }
            if keyval == Key::Up || keyval == Key::KP_Up || keyval == Key::ISO_Left_Tab {
                move_sel(&list_c, &state_c, &entry_c, -1);
                return glib::Propagation::Stop;
            }
            if keyval == Key::Return || keyval == Key::KP_Enter {
                launch_selected(&list_c, &state_c, &app_c, &entry_c, &apps_c);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        entry.add_controller(keys);
    }

    {
        let app_c = app.clone();
        let list_c = list.clone();
        let state_c = state.clone();
        let entry_c = entry.clone();
        let apps_c = apps.clone();
        entry.connect_activate(move |_| {
            launch_selected(&list_c, &state_c, &app_c, &entry_c, &apps_c);
        });
    }

    {
        let app_c = app.clone();
        let list_c = list.clone();
        let state_c = state.clone();
        let entry_c = entry.clone();
        let apps_c = apps.clone();
        list.connect_row_activated(move |_, _| {
            launch_selected(&list_c, &state_c, &app_c, &entry_c, &apps_c);
        });
    }

    {
        let entry_c = entry.clone();
        list.connect_row_selected(move |_, _| {
            let e = entry_c.clone();
            glib::idle_add_local_once(move || focus_entry(&e));
        });
    }

    {
        let focus_ctrl = EventControllerFocus::new();
        let entry_c = entry.clone();
        focus_ctrl.connect_leave(move |_| {
            let e = entry_c.clone();
            glib::idle_add_local_once(move || focus_entry(&e));
        });
        list.add_controller(focus_ctrl);
    }

    {
        let entry_c = entry.clone();
        window.connect_is_active_notify(move |win| {
            if win.is_active() && !entry_c.has_focus() {
                let e = entry_c.clone();
                glib::idle_add_local_once(move || focus_entry(&e));
            }
        });
    }

    {
        let entry_c = entry.clone();
        let list_c = list.clone();
        let state_c = state.clone();
        let apps_c = apps.clone();
        entry.connect_changed(move |ent| {
            if state_c.ignore_changed.get() {
                return;
            }
            if let Some(id) = state_c.refresh_id.take() {
                id.remove();
            }
            let q = ent.text().to_string();
            let immediate = is_calc_query(&q);
            let entry_d = entry_c.clone();
            let list_d = list_c.clone();
            let state_d = state_c.clone();
            let apps_d = apps_c.clone();
            let refresh = move || {
                state_d.refresh_id.set(None);
                if entry_d.text() != q {
                    return;
                }
                let pos = entry_d.position();
                rebuild_list(&list_d, &apps_d, &q, &state_d);
                state_d.ignore_changed.set(true);
                entry_d.grab_focus();
                let set_pos = if pos >= 0 { pos } else { q.len() as i32 };
                entry_d.set_position(set_pos);
                state_d.ignore_changed.set(false);
            };
            // Calc prefixes feel laggy with debounce — rebuild immediately.
            if immediate {
                refresh();
            } else {
                let sid = glib::timeout_add_local(Duration::from_millis(40), move || {
                    refresh();
                    ControlFlow::Break
                });
                state_c.refresh_id.set(Some(sid));
            }
        });
    }

    rebuild_list(&list, &apps, "", &state);

    {
        let entry_c = entry.clone();
        glib::timeout_add_local_once(Duration::from_millis(50), move || {
            focus_entry(&entry_c);
        });
    }

    window.present();
}

fn main() -> glib::ExitCode {
    let _pid = PidGuard::acquire();
    let apps = Rc::new(desktop::load_apps());

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| {
        if let Some(win) = app.active_window() {
            win.present();
            return;
        }
        apply_css();
        build_ui(app, apps.clone());
    });
    app.run()
}


#[cfg(test)]
mod tests {
    use super::{parse_calc, parse_web_action};

    #[test]
    fn calc_with_and_without_space() {
        let (d, r) = parse_calc("=2+2").unwrap();
        assert_eq!(r.as_deref(), Some("4"));
        assert!(d.contains('4'));

        let (d, r) = parse_calc("= 2+2").unwrap();
        assert_eq!(r.as_deref(), Some("4"));
        assert!(d.contains('4'));
    }

    #[test]
    fn calc_bare_equals_shows_hint() {
        let (d, r) = parse_calc("=").unwrap();
        assert!(r.is_none());
        assert!(d.contains("type expression"));

        let (d, r) = parse_calc("=   ").unwrap();
        assert!(r.is_none());
        assert!(d.contains("type expression"));
    }

    #[test]
    fn calc_fullwidth_equals() {
        let (d, r) = parse_calc("＝2+2").unwrap();
        assert_eq!(r.as_deref(), Some("4"));
        assert!(d.contains('4'));

        let (d, r) = parse_calc("＝ 3*3").unwrap();
        assert_eq!(r.as_deref(), Some("9"));
        assert!(d.contains('9'));

        let (d, r) = parse_calc("＝").unwrap();
        assert!(r.is_none());
        assert!(d.contains("type expression"));
    }

    #[test]
    fn calc_invalid() {
        let (d, r) = parse_calc("=2+").unwrap();
        assert!(r.is_none());
        assert!(d.contains("invalid"));
        assert!(parse_calc("hello").is_none());
    }

    #[test]
    fn web_question_with_and_without_space() {
        let (t, u) = parse_web_action("?rust").unwrap();
        assert!(t.contains("DuckDuckGo"));
        assert!(u.contains("duckduckgo.com"));
        assert!(u.contains("rust"));

        let (t, u) = parse_web_action("? rust lang").unwrap();
        assert!(t.contains("DuckDuckGo"));
        assert!(u.contains("rust"));
    }

    #[test]
    fn web_engines_and_empty() {
        assert!(parse_web_action("ddg").is_none());
        assert!(parse_web_action("ddg ").is_none());
        assert!(parse_web_action("?").is_none());

        let (t, u) = parse_web_action("ddg hello").unwrap();
        assert!(t.contains("DuckDuckGo"));
        assert!(u.contains("hello"));

        let (t, u) = parse_web_action("gs hello").unwrap();
        assert!(t.contains("Google"));
        assert!(u.contains("google.com"));

        let (t, u) = parse_web_action("google hello world").unwrap();
        assert!(t.contains("Google"));
        assert!(u.contains("hello"));

        let (t, u) = parse_web_action("https://example.com").unwrap();
        assert!(t.contains("Open"));
        assert_eq!(u, "https://example.com");
    }
}

mod raw_libc {
    extern "C" {
        pub fn getuid() -> u32;
    }
}
