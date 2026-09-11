//! Floresline GNOME launcher — Rust + GTK4 (feature-parity with Python).

mod desktop;
mod fuzzy;
mod launch;

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gtk::gdk::{Display, Key};
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

struct UiState {
    results: RefCell<Vec<desktop::AppEntry>>,
    refresh_id: Cell<Option<glib::SourceId>>,
    ignore_changed: Cell<bool>,
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

fn rebuild_list(
    list: &ListBox,
    apps: &[desktop::AppEntry],
    query: &str,
    state: &UiState,
) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }

    let scored = fuzzy::rank(query, apps);
    let top: Vec<desktop::AppEntry> = scored.into_iter().take(50).map(|(_, a)| a).collect();

    for a in &top {
        let row = ListBoxRow::new();
        row.set_can_focus(false);
        row.set_focusable(false);

        let box_ = GtkBox::new(Orientation::Horizontal, 12);
        box_.set_margin_start(10);
        box_.set_margin_end(10);
        box_.set_margin_top(6);
        box_.set_margin_bottom(6);

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
        row.set_child(Some(&box_));
        list.append(&row);
    }

    *state.results.borrow_mut() = top;
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

fn launch_selected(list: &ListBox, state: &UiState, app: &Application) {
    let idx = list.selected_row().map(|r| r.index()).unwrap_or(0) as usize;
    let results = state.results.borrow();
    if idx < results.len() {
        let _ = launch::launch_app(&results[idx]);
        app.quit();
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

    let hint = Label::new(Some("Type full words · ↑↓ select · Enter launch · Esc close"));
    hint.add_css_class("dim-label");
    hint.set_margin_top(6);
    hint.set_margin_bottom(8);
    outer.append(&hint);

    // Key controller on entry (CAPTURE)
    {
        let app_c = app.clone();
        let list_c = list.clone();
        let state_c = state.clone();
        let entry_c = entry.clone();
        keys.connect_key_pressed(move |_, keyval, _code, _state| {
            if keyval == Key::Escape {
                app_c.quit();
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
                launch_selected(&list_c, &state_c, &app_c);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        entry.add_controller(keys);
    }

    // Activate (Enter on entry)
    {
        let app_c = app.clone();
        let list_c = list.clone();
        let state_c = state.clone();
        entry.connect_activate(move |_| {
            launch_selected(&list_c, &state_c, &app_c);
        });
    }

    // Row activated
    {
        let app_c = app.clone();
        let list_c = list.clone();
        let state_c = state.clone();
        list.connect_row_activated(move |_, _| {
            launch_selected(&list_c, &state_c, &app_c);
        });
    }

    // Row selected → yank focus back
    {
        let entry_c = entry.clone();
        list.connect_row_selected(move |_, _| {
            let e = entry_c.clone();
            glib::idle_add_local_once(move || focus_entry(&e));
        });
    }

    // Focus leave on list → yank back
    {
        let focus_ctrl = EventControllerFocus::new();
        let entry_c = entry.clone();
        focus_ctrl.connect_leave(move |_| {
            let e = entry_c.clone();
            glib::idle_add_local_once(move || focus_entry(&e));
        });
        list.add_controller(focus_ctrl);
    }

    // Window is-active notify
    {
        let entry_c = entry.clone();
        window.connect_is_active_notify(move |win| {
            if win.is_active() && !entry_c.has_focus() {
                let e = entry_c.clone();
                glib::idle_add_local_once(move || focus_entry(&e));
            }
        });
    }

    // Debounced changed
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
            let entry_d = entry_c.clone();
            let list_d = list_c.clone();
            let state_d = state_c.clone();
            let apps_d = apps_c.clone();
            let sid = glib::timeout_add_local(Duration::from_millis(40), move || {
                state_d.refresh_id.set(None);
                if entry_d.text() != q {
                    return ControlFlow::Break;
                }
                let pos = entry_d.position();
                rebuild_list(&list_d, &apps_d, &q, &state_d);
                state_d.ignore_changed.set(true);
                entry_d.grab_focus();
                let set_pos = if pos >= 0 { pos } else { q.len() as i32 };
                entry_d.set_position(set_pos);
                state_d.ignore_changed.set(false);
                ControlFlow::Break
            });
            state_c.refresh_id.set(Some(sid));
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
        // Single window: if already open, present
        if let Some(win) = app.active_window() {
            win.present();
            return;
        }
        apply_css();
        build_ui(app, apps.clone());
    });
    app.run()
}

mod raw_libc {
    extern "C" {
        pub fn getuid() -> u32;
    }
}
