use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use gio::prelude::*;
use gtk::prelude::*;
use gtk4 as gtk;
use rsclip_core::models::{EntryFilter, SortMode};
use rsclip_core::{Database, RsclipPaths};

use crate::actions::refresh::refresh_entries;
use crate::actions::update_mode_controls;
use crate::components::{footer, list, preview, topbar};
use crate::state::{AppState, AppView};

const AUTO_PASTE_DELAY: Duration = Duration::from_millis(140);

pub(crate) struct UiRuntime {
    pub(crate) state: Rc<AppState>,
    pub(crate) window: gtk::ApplicationWindow,
    _hold: gio::ApplicationHoldGuard,
}

impl UiRuntime {
    pub(crate) fn show_reset(&self) -> Result<()> {
        use gtk4_layer_shell::{KeyboardMode, LayerShell};

        *self.state.view.borrow_mut() = AppView::Clipboard;
        *self.state.query.borrow_mut() = String::new();
        *self.state.filter.borrow_mut() = EntryFilter::All;
        *self.state.sort.borrow_mut() = SortMode::Default;

        self.state.search_entry.set_text("");
        self.state
            .search_entry
            .set_placeholder_text(Some("Search clipboard..."));
        self.state.filter_select.set_selected(0);
        update_mode_controls(&self.state);
        refresh_entries(&self.state)?;
        *self.state.dirty.borrow_mut() = false;

        self.state.list_adjustment.set_value(0.0);
        if let Some(row) = self.state.list.row_at_index(0) {
            self.state.list.select_row(Some(&row));
        }

        self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        self.window.set_visible(true);
        self.window.present();
        self.state.search_entry.grab_focus();
        Ok(())
    }

    pub(crate) fn toggle(&self) -> Result<()> {
        if self.window.is_visible() {
            self.hide();
            Ok(())
        } else {
            self.show_reset()
        }
    }

    pub(crate) fn hide(&self) {
        hide_overlay(&self.state, &self.window);
    }
}

pub(crate) fn build_ui(app: &gtk::Application) -> Result<UiRuntime> {
    let paths = RsclipPaths::discover()?;
    paths.ensure()?;
    Database::open(&paths.db_path)?;

    crate::style::load_css();

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("rsclip")
        .default_width(760)
        .default_height(480)
        .resizable(false)
        .build();
    window.add_css_class("rsclip-window");
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
        filter: std::cell::RefCell::new(rsclip_core::models::EntryFilter::All),
        sort: std::cell::RefCell::new(rsclip_core::models::SortMode::Default),
        view: std::cell::RefCell::new(AppView::Clipboard),
        dirty: std::cell::RefCell::new(false),
        prompt_active: std::cell::RefCell::new(false),
        search_entry: topbar.search.clone(),
        filter_select: topbar.filter.clone(),
        history_button: topbar.history_button.clone(),
        secrets_button: topbar.secrets_button.clone(),
        count_label: topbar.count.clone(),
        list: list_panel.list.clone(),
        list_adjustment: list_panel.adjustment,
        preview: preview_panel.preview.clone(),
        details: preview_panel.details.clone(),
        footer: footer_bar.footer.clone(),
        ocr_button: footer_bar.ocr_button.clone(),
    });

    crate::notify::install_change_listener(&state, &window, &paths.socket_path)?;
    crate::events::connect(&state, &window);

    Ok(UiRuntime {
        state,
        window,
        _hold: app.hold(),
    })
}

fn configure_overlay_window(window: &gtk::ApplicationWindow) {
    use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

    window.set_decorated(false);
    window.set_resizable(false);

    window.init_layer_shell();
    window.set_namespace(Some("rsclip"));
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_exclusive_zone(-1);

    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);
    window.set_anchor(Edge::Top, false);
    window.set_anchor(Edge::Bottom, false);
}

pub(crate) fn hide_overlay(state: &Rc<AppState>, window: &gtk::ApplicationWindow) {
    use gtk4_layer_shell::{KeyboardMode, LayerShell};

    window.set_keyboard_mode(KeyboardMode::None);
    window.set_visible(false);
    preview::clear_preview_state(state);
    *state.prompt_active.borrow_mut() = false;
}

pub(crate) fn close_overlay_and_paste(state: &Rc<AppState>, window: &gtk::ApplicationWindow) {
    hide_overlay(state, window);

    gtk::glib::timeout_add_local_once(AUTO_PASTE_DELAY, move || {
        if let Err(err) = rsclip_core::paste::trigger_paste() {
            eprintln!("rsclip: Paste failed: {err:#}");
        }
    });
}
