//! Floresline GNOME launcher — Rust + GTK4 skeleton (WIP).
//!
//! Python `bin/floresline-launcher` remains the supported default until this
//! port reaches feature parity. The Shell extension stays GJS.

mod desktop;
mod fuzzy;
mod launch;

use gtk::gdk::Key;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, EventControllerKey, ListBox, Orientation,
    PolicyType, ScrolledWindow, SearchEntry,
};

const APP_ID: &str = "dev.floresline.Launcher";

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Floresline Launcher")
        .default_width(560)
        .default_height(420)
        .resizable(true)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 8);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    let search = SearchEntry::new();
    search.set_placeholder_text(Some("Search apps…"));
    search.set_hexpand(true);

    let list = ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);

    let scrolled = ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(280)
        .build();

    vbox.append(&search);
    vbox.append(&scrolled);
    window.set_child(Some(&vbox));

    // Keep the search field focused so multi-letter queries work (same UX as Python).
    search.connect_changed(glib::clone!(
        #[weak]
        search,
        move |_| {
            search.grab_focus_without_selecting();
        }
    ));

    // Stub: refresh results when the query changes (desktop/fuzzy not wired yet).
    search.connect_search_changed(glib::clone!(
        #[weak]
        list,
        move |entry| {
            let query = entry.text();
            while let Some(row) = list.row_at_index(0) {
                list.remove(&row);
            }
            let _ranked = fuzzy::rank(&query, &desktop::scan_apps());
            // TODO: populate ListBox from ranked results
        }
    ));

    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed(glib::clone!(
        #[weak]
        window,
        #[weak]
        list,
        #[weak]
        search,
        move |_, key, _code, _mods| {
            match key {
                Key::Escape => {
                    window.close();
                    glib::Propagation::Stop
                }
                Key::Up => {
                    // Stub: move selection up
                    if let Some(row) = list.selected_row() {
                        let idx = row.index();
                        if idx > 0 {
                            if let Some(prev) = list.row_at_index(idx - 1) {
                                list.select_row(Some(&prev));
                            }
                        }
                    }
                    search.grab_focus_without_selecting();
                    glib::Propagation::Stop
                }
                Key::Down => {
                    // Stub: move selection down
                    let next_idx = list
                        .selected_row()
                        .map(|r| r.index() + 1)
                        .unwrap_or(0);
                    if let Some(next) = list.row_at_index(next_idx) {
                        list.select_row(Some(&next));
                    }
                    search.grab_focus_without_selecting();
                    glib::Propagation::Stop
                }
                Key::Return | Key::KP_Enter => {
                    // Stub: launch selected (launch.rs TODO)
                    if let Some(_row) = list.selected_row() {
                        let _ = launch::launch_app(&desktop::DesktopApp {
                            name: String::new(),
                            exec: String::new(),
                            desktop_id: String::new(),
                        });
                    }
                    search.grab_focus_without_selecting();
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        }
    ));
    window.add_controller(key_controller);

    window.connect_show(glib::clone!(
        #[weak]
        search,
        move |_| {
            search.grab_focus_without_selecting();
        }
    ));

    window.present();
}

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}
