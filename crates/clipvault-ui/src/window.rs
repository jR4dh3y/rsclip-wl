use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use clipvault_core::{ClipvaultPaths, Database};
use gtk::prelude::*;
use gtk4 as gtk;

use crate::actions::refresh::refresh_entries;
use crate::components::{footer, list, preview, topbar};
use crate::state::{AppState, AppView};

const AUTO_PASTE_DELAY: Duration = Duration::from_millis(140);

pub(crate) fn build_ui(app: &gtk::Application) -> Result<()> {
    let paths = ClipvaultPaths::discover()?;
    paths.ensure()?;
    Database::open(&paths.db_path)?;

    crate::style::load_css();

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Clipvault")
        .default_width(760)
        .default_height(480)
        .resizable(false)
        .build();
    window.add_css_class("clipvault-window");
    configure_overlay_window(&window);

    let root = gtk::Overlay::new();
    window.set_child(Some(&root));

    let shell = gtk::Box::new(gtk::Orientation::Vertical, 0);
    shell.add_css_class("app-shell");
    root.set_child(Some(&shell));

    let topbar = topbar::build();
    shell.append(&topbar.container);

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_wide_handle(true);
    paned.set_vexpand(true);
    shell.append(&paned);

    let list_panel = list::build_panel();
    paned.set_start_child(Some(&list_panel.scroller));

    let preview_panel = preview::build_panel();
    paned.set_end_child(Some(&preview_panel.shell));
    paned.set_position(290);

    let footer_bar = footer::build();
    shell.append(&footer_bar.container);

    let state = Rc::new(AppState {
        db_path: paths.db_path,
        entries: std::cell::RefCell::new(Vec::new()),
        secrets: std::cell::RefCell::new(Vec::new()),
        query: std::cell::RefCell::new(String::new()),
        filter: std::cell::RefCell::new(clipvault_core::models::EntryFilter::All),
        sort: std::cell::RefCell::new(clipvault_core::models::SortMode::Default),
        view: std::cell::RefCell::new(AppView::Clipboard),
        prompt_active: std::cell::RefCell::new(false),
        search_entry: topbar.search.clone(),
        filter_select: topbar.filter.clone(),
        history_button: topbar.history_button.clone(),
        secrets_button: topbar.secrets_button.clone(),
        list: list_panel.list.clone(),
        list_adjustment: list_panel.adjustment,
        preview: preview_panel.preview.clone(),
        details: preview_panel.details.clone(),
        footer: footer_bar.footer.clone(),
        ocr_button: footer_bar.ocr_button.clone(),
    });

    refresh_entries(&state)?;
    crate::notify::install_change_listener(&state, &paths.socket_path)?;
    crate::events::connect(&state, app, &window);

    window.present();
    topbar.search.grab_focus();
    Ok(())
}

fn configure_overlay_window(window: &gtk::ApplicationWindow) {
    use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

    window.set_decorated(false);
    window.set_resizable(false);

    window.init_layer_shell();
    window.set_namespace(Some("clipvault"));
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_exclusive_zone(-1);

    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);
    window.set_anchor(Edge::Top, false);
    window.set_anchor(Edge::Bottom, false);
}

pub(crate) fn close_overlay_and_paste(app: &gtk::Application, window: &gtk::ApplicationWindow) {
    use gtk4_layer_shell::{KeyboardMode, LayerShell};

    window.set_keyboard_mode(KeyboardMode::None);
    window.set_visible(false);

    let app = app.clone();
    gtk::glib::timeout_add_local_once(AUTO_PASTE_DELAY, move || {
        if let Err(err) = clipvault_core::paste::trigger_paste() {
            eprintln!("clipvault: Paste failed: {err:#}");
        }
        app.quit();
    });
}
