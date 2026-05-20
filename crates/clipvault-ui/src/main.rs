use std::cell::RefCell;
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use clipvault_core::models::{ClipboardEntry, EntryFilter, EntryKind, SecretEntry, SortMode};
use clipvault_core::notify::CHANGE_EVENT;
use clipvault_core::ocr::run_tesseract;
use clipvault_core::paste::{copy_entry, trigger_paste, write_clipboard};
use clipvault_core::{ClipvaultPaths, Database};
use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;

const APP_ID: &str = "io.github.radhey.clipvault";
const AUTO_PASTE_DELAY: Duration = Duration::from_millis(140);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppView {
    Clipboard,
    Secrets,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "clipvault=warn".into()),
        )
        .init();

    if let Err(err) = run() {
        eprintln!("clipvault: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("list") {
        return cmd_list(&args[1..]);
    }

    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        anyhow::bail!("Clipvault overlay requires Wayland");
    }

    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        if let Err(err) = build_ui(app) {
            eprintln!("clipvault: {err:#}");
            app.quit();
        }
    });
    app.run();
    Ok(())
}

struct AppState {
    db_path: PathBuf,
    entries: RefCell<Vec<ClipboardEntry>>,
    secrets: RefCell<Vec<SecretEntry>>,
    query: RefCell<String>,
    filter: RefCell<EntryFilter>,
    sort: RefCell<SortMode>,
    view: RefCell<AppView>,
    prompt_active: RefCell<bool>,
    search_entry: gtk::SearchEntry,
    filter_select: gtk::DropDown,
    history_button: gtk::Button,
    secrets_button: gtk::Button,
    list: gtk::ListBox,
    list_adjustment: gtk::Adjustment,
    preview: gtk::Box,
    details: gtk::Box,
    footer: gtk::Label,
    ocr_button: gtk::Button,
}

fn build_ui(app: &gtk::Application) -> Result<()> {
    let paths = ClipvaultPaths::discover()?;
    paths.ensure()?;
    Database::open(&paths.db_path)?;

    load_css();

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

    let topbar = gtk::Box::new(gtk::Orientation::Horizontal, 7);
    topbar.add_css_class("topbar");
    shell.append(&topbar);

    let history_button = gtk::Button::with_label("Clipboard");
    history_button.add_css_class("mode-button");
    history_button.add_css_class("active-mode");
    topbar.append(&history_button);

    let secrets_button = gtk::Button::with_label("Secrets");
    secrets_button.add_css_class("mode-button");
    topbar.append(&secrets_button);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search clipboard..."));
    search.add_css_class("search-box");
    search.set_hexpand(true);
    topbar.append(&search);

    let filter =
        gtk::DropDown::from_strings(&["All", "Text", "Images", "Links", "Colors", "Pinned"]);
    filter.add_css_class("filter-select");
    filter.set_selected(0);
    topbar.append(&filter);

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_wide_handle(true);
    paned.set_vexpand(true);
    shell.append(&paned);

    let list_scroller = gtk::ScrolledWindow::builder()
        .min_content_width(220)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    list_scroller.add_css_class("sidebar");
    let list = gtk::ListBox::new();
    list.add_css_class("entry-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    list_scroller.set_child(Some(&list));
    let list_adjustment = list_scroller.vadjustment();
    list.set_adjustment(Some(&list_adjustment));
    paned.set_start_child(Some(&list_scroller));

    let preview_shell = gtk::Box::new(gtk::Orientation::Vertical, 8);
    preview_shell.set_vexpand(true);
    preview_shell.add_css_class("preview-pane");

    let preview = gtk::Box::new(gtk::Orientation::Vertical, 8);
    preview.set_vexpand(true);
    let preview_scroller = gtk::ScrolledWindow::builder()
        .min_content_width(150)
        .min_content_height(64)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&preview)
        .build();
    preview_shell.append(&preview_scroller);

    let details = gtk::Box::new(gtk::Orientation::Vertical, 4);
    details.add_css_class("details-panel");
    details.set_hexpand(true);
    preview_shell.append(&details);

    paned.set_end_child(Some(&preview_shell));
    paned.set_position(290);

    let footer_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    footer_bar.set_valign(gtk::Align::Center);
    footer_bar.add_css_class("footer");

    let footer = gtk::Label::new(Some(
        "Enter paste  Ctrl+Enter copy  Ctrl+S save secret  Ctrl+P pin  Ctrl+D delete  Esc close",
    ));
    footer.add_css_class("footer-label");
    footer.add_css_class("muted");
    footer.set_xalign(0.0);
    footer.set_wrap(true);
    footer.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    footer.set_hexpand(true);
    footer_bar.append(&footer);

    let ocr_button = gtk::Button::with_label("OCR");
    ocr_button.add_css_class("ocr-button");
    ocr_button.set_opacity(0.0);
    ocr_button.set_sensitive(false);
    footer_bar.append(&ocr_button);

    shell.append(&footer_bar);

    let state = Rc::new(AppState {
        db_path: paths.db_path,
        entries: RefCell::new(Vec::new()),
        secrets: RefCell::new(Vec::new()),
        query: RefCell::new(String::new()),
        filter: RefCell::new(EntryFilter::All),
        sort: RefCell::new(SortMode::Default),
        view: RefCell::new(AppView::Clipboard),
        prompt_active: RefCell::new(false),
        search_entry: search.clone(),
        filter_select: filter.clone(),
        history_button: history_button.clone(),
        secrets_button: secrets_button.clone(),
        list: list.clone(),
        list_adjustment,
        preview: preview.clone(),
        details: details.clone(),
        footer,
        ocr_button: ocr_button.clone(),
    });

    refresh_entries(&state)?;
    install_change_listener(&state, &paths.socket_path)?;

    {
        let state = Rc::clone(&state);
        let search = search.clone();
        history_button.connect_clicked(move |_| {
            *state.view.borrow_mut() = AppView::Clipboard;
            *state.query.borrow_mut() = String::new();
            search.set_text("");
            search.set_placeholder_text(Some("Search clipboard..."));
            update_mode_controls(&state);
            if let Err(err) = refresh_entries(&state) {
                set_footer(&state, &format!("Switch failed: {err:#}"));
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let search = search.clone();
        secrets_button.connect_clicked(move |_| {
            *state.view.borrow_mut() = AppView::Secrets;
            *state.query.borrow_mut() = String::new();
            search.set_text("");
            search.set_placeholder_text(Some("Search secrets by name..."));
            update_mode_controls(&state);
            if let Err(err) = refresh_entries(&state) {
                set_footer(&state, &format!("Switch failed: {err:#}"));
            }
        });
    }

    {
        let state = Rc::clone(&state);
        ocr_button.connect_clicked(move |_| {
            if let Some(entry) = current_entry(&state) {
                if matches!(entry.kind, EntryKind::Image) {
                    if let Err(err) = run_ocr_for_entry(&state, entry.id) {
                        set_footer(&state, &format!("OCR failed: {err:#}"));
                    }
                }
            }
        });
    }

    {
        let state = Rc::clone(&state);
        search.connect_search_changed(move |entry| {
            *state.query.borrow_mut() = entry.text().to_string();
            if let Err(err) = refresh_entries(&state) {
                set_footer(&state, &format!("Search failed: {err:#}"));
            }
        });
    }

    {
        let state = Rc::clone(&state);
        filter.connect_selected_notify(move |dropdown| {
            *state.filter.borrow_mut() = match dropdown.selected() {
                1 => EntryFilter::Text,
                2 => EntryFilter::Images,
                3 => EntryFilter::Links,
                4 => EntryFilter::Colors,
                5 => EntryFilter::Pinned,
                _ => EntryFilter::All,
            };
            if let Err(err) = refresh_entries(&state) {
                set_footer(&state, &format!("Filter failed: {err:#}"));
            }
        });
    }

    {
        let state = Rc::clone(&state);
        list.connect_row_selected(move |list, row| {
            mark_selected_row(list, row);
            if let Some(row) = row {
                let index = row.index();
                if index >= 0 {
                    match *state.view.borrow() {
                        AppView::Clipboard => {
                            if let Some(entry) = state.entries.borrow().get(index as usize) {
                                render_preview(&state, entry);
                            }
                        }
                        AppView::Secrets => {
                            if let Some(secret) = state.secrets.borrow().get(index as usize) {
                                render_secret_preview(&state, secret);
                            }
                        }
                    }
                }
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let app = app.clone();
        let window = window.clone();
        list.connect_row_activated(move |_, row| match *state.view.borrow() {
            AppView::Clipboard => {
                if let Some(entry) = state.entries.borrow().get(row.index() as usize).cloned() {
                    if let Err(err) = copy_selected_entry(&state, &entry) {
                        set_footer(&state, &format!("Paste failed: {err:#}"));
                        return;
                    }
                    close_overlay_and_paste(&app, &window);
                }
            }
            AppView::Secrets => {
                if let Some(secret) = state.secrets.borrow().get(row.index() as usize).cloned() {
                    if let Err(err) = copy_secret(&state, &secret) {
                        set_footer(&state, &format!("Copy failed: {err:#}"));
                        return;
                    }
                    app.quit();
                }
            }
        });
    }

    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let state = Rc::clone(&state);
        let app = app.clone();
        let window = window.clone();
        controller.connect_key_pressed(move |_, key, _, modifiers| {
            if *state.prompt_active.borrow() {
                return glib::Propagation::Proceed;
            }

            let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
            match (key, ctrl) {
                (gdk::Key::Down, false) => {
                    move_selection(&state, 1);
                    glib::Propagation::Stop
                }
                (gdk::Key::Up, false) => {
                    move_selection(&state, -1);
                    glib::Propagation::Stop
                }
                (gdk::Key::Escape, _) => {
                    app.quit();
                    glib::Propagation::Stop
                }
                (gdk::Key::Return | gdk::Key::KP_Enter, false) => {
                    match *state.view.borrow() {
                        AppView::Clipboard => {
                            if let Some(entry) = current_entry(&state) {
                                if let Err(err) = copy_selected_entry(&state, &entry) {
                                    set_footer(&state, &format!("Paste failed: {err:#}"));
                                } else {
                                    close_overlay_and_paste(&app, &window);
                                }
                            }
                        }
                        AppView::Secrets => {
                            if let Some(secret) = current_secret(&state) {
                                if let Err(err) = copy_secret(&state, &secret) {
                                    set_footer(&state, &format!("Copy failed: {err:#}"));
                                } else {
                                    app.quit();
                                }
                            }
                        }
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::Return | gdk::Key::KP_Enter, true) => {
                    match *state.view.borrow() {
                        AppView::Clipboard => {
                            if let Some(entry) = current_entry(&state) {
                                if let Err(err) = copy_selected_entry(&state, &entry) {
                                    set_footer(&state, &format!("Copy failed: {err:#}"));
                                } else {
                                    set_footer(&state, "Copied selected entry");
                                }
                            }
                        }
                        AppView::Secrets => {
                            if let Some(secret) = current_secret(&state) {
                                if let Err(err) = copy_secret(&state, &secret) {
                                    set_footer(&state, &format!("Copy failed: {err:#}"));
                                } else {
                                    set_footer(&state, "Copied secret");
                                }
                            }
                        }
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::s | gdk::Key::S, true) => {
                    match *state.view.borrow() {
                        AppView::Clipboard => {
                            save_current_as_secret_dialog(&state, window.upcast_ref())
                        }
                        AppView::Secrets => {
                            if let Some(secret) = current_secret(&state) {
                                if let Err(err) = copy_secret(&state, &secret) {
                                    set_footer(&state, &format!("Copy failed: {err:#}"));
                                } else {
                                    set_footer(&state, "Copied secret");
                                }
                            }
                        }
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::p | gdk::Key::P, true) => {
                    if *state.view.borrow() == AppView::Clipboard {
                        if let Err(err) = toggle_pin(&state) {
                            set_footer(&state, &format!("Pin failed: {err:#}"));
                        }
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::d | gdk::Key::D, true) => {
                    if let Err(err) = delete_current(&state) {
                        set_footer(&state, &format!("Delete failed: {err:#}"));
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::e | gdk::Key::E, true) => {
                    if *state.view.borrow() == AppView::Secrets {
                        rename_current_secret_dialog(&state, window.upcast_ref());
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::r | gdk::Key::R, true) => {
                    if let Err(err) = refresh_entries(&state) {
                        set_footer(&state, &format!("Refresh failed: {err:#}"));
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::i | gdk::Key::I, true) => {
                    if *state.view.borrow() == AppView::Clipboard {
                        *state.filter.borrow_mut() = EntryFilter::Images;
                        if let Err(err) = refresh_entries(&state) {
                            set_footer(&state, &format!("Filter failed: {err:#}"));
                        }
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::l | gdk::Key::L, true) => {
                    if *state.view.borrow() == AppView::Clipboard {
                        *state.filter.borrow_mut() = EntryFilter::Links;
                        if let Err(err) = refresh_entries(&state) {
                            set_footer(&state, &format!("Filter failed: {err:#}"));
                        }
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::c | gdk::Key::C, true) => {
                    match *state.view.borrow() {
                        AppView::Clipboard => {
                            *state.filter.borrow_mut() = EntryFilter::Colors;
                            if let Err(err) = refresh_entries(&state) {
                                set_footer(&state, &format!("Filter failed: {err:#}"));
                            }
                        }
                        AppView::Secrets => {
                            if let Some(secret) = current_secret(&state) {
                                if let Err(err) = copy_secret(&state, &secret) {
                                    set_footer(&state, &format!("Copy failed: {err:#}"));
                                } else {
                                    set_footer(&state, "Copied secret");
                                }
                            }
                        }
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
    }
    window.add_controller(controller);

    window.present();
    search.grab_focus();
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

fn close_overlay_and_paste(app: &gtk::Application, window: &gtk::ApplicationWindow) {
    use gtk4_layer_shell::{KeyboardMode, LayerShell};

    window.set_keyboard_mode(KeyboardMode::None);
    window.set_visible(false);

    let app = app.clone();
    glib::timeout_add_local_once(AUTO_PASTE_DELAY, move || {
        if let Err(err) = trigger_paste() {
            eprintln!("clipvault: Paste failed: {err:#}");
        }
        app.quit();
    });
}

fn cmd_list(args: &[String]) -> Result<()> {
    let query = option_value(args, "--query").unwrap_or("");
    let filter = EntryFilter::parse(option_value(args, "--filter").unwrap_or("all"));
    let sort = SortMode::parse(option_value(args, "--sort").unwrap_or("default"));
    let limit = option_value(args, "--limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100);
    let json = args.iter().any(|arg| arg == "--json");

    let paths = ClipvaultPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    let entries = db.list_entries(query, filter, sort, limit)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        for entry in entries {
            println!(
                "#{:<4} {:<6} {:<1} {}",
                entry.id,
                entry.kind,
                if entry.pinned { "P" } else { " " },
                entry.title
            );
        }
    }
    Ok(())
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].as_str())
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("../resources/css/clipvault.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn install_change_listener(state: &Rc<AppState>, socket_path: &Path) -> Result<()> {
    match std::fs::remove_file(socket_path) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "removing stale notification socket {}",
                    socket_path.display()
                )
            });
        }
    }

    let socket = UnixDatagram::bind(socket_path)
        .with_context(|| format!("binding notification socket {}", socket_path.display()))?;
    socket
        .set_nonblocking(true)
        .context("setting notification socket nonblocking")?;
    let fd = socket.as_raw_fd();

    {
        let state = Rc::clone(state);
        glib::source::unix_fd_add_local(fd, glib::IOCondition::IN, move |_, _| {
            let mut buf = [0_u8; 64];
            let mut changed = false;
            loop {
                match socket.recv(&mut buf) {
                    Ok(size) => {
                        changed |= &buf[..size] == CHANGE_EVENT;
                    }
                    Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                    Err(err) => {
                        set_footer(&state, &format!("Notification listener failed: {err}"));
                        return glib::ControlFlow::Break;
                    }
                }
            }

            if changed {
                if let Err(err) = refresh_entries_if_changed(&state) {
                    set_footer(&state, &format!("Refresh failed: {err:#}"));
                }
            }
            glib::ControlFlow::Continue
        });
    }

    Ok(())
}

fn refresh_entries(state: &Rc<AppState>) -> Result<()> {
    let db = Database::open(&state.db_path)?;

    match *state.view.borrow() {
        AppView::Clipboard => {
            let selected_id = current_entry(state).map(|entry| entry.id);
            let entries = db.list_entries(
                &state.query.borrow(),
                *state.filter.borrow(),
                *state.sort.borrow(),
                200,
            )?;
            *state.entries.borrow_mut() = entries;
            render_clipboard_list(state, selected_id);
        }
        AppView::Secrets => {
            let selected_id = current_secret(state).map(|secret| secret.id);
            let secrets = db.list_secrets(&state.query.borrow(), 200)?;
            *state.secrets.borrow_mut() = secrets;
            render_secrets_list(state, selected_id);
        }
    }
    Ok(())
}

fn refresh_entries_if_changed(state: &Rc<AppState>) -> Result<()> {
    let db = Database::open(&state.db_path)?;

    match *state.view.borrow() {
        AppView::Clipboard => {
            let entries = db.list_entries(
                &state.query.borrow(),
                *state.filter.borrow(),
                *state.sort.borrow(),
                200,
            )?;
            if state.entries.borrow().as_slice() == entries.as_slice() {
                return Ok(());
            }
            let selected_id = current_entry(state).map(|entry| entry.id);
            *state.entries.borrow_mut() = entries;
            render_clipboard_list(state, selected_id);
        }
        AppView::Secrets => {
            let secrets = db.list_secrets(&state.query.borrow(), 200)?;
            if state.secrets.borrow().as_slice() == secrets.as_slice() {
                return Ok(());
            }
            let selected_id = current_secret(state).map(|secret| secret.id);
            *state.secrets.borrow_mut() = secrets;
            render_secrets_list(state, selected_id);
        }
    }
    Ok(())
}

fn render_clipboard_list(state: &Rc<AppState>, selected_id: Option<i64>) {
    state.secrets.borrow_mut().clear();
    clear_box_like(&state.list);

    for entry in state.entries.borrow().iter() {
        state.list.append(&entry_row(entry));
    }

    let selected_index = selected_id
        .and_then(|id| {
            state
                .entries
                .borrow()
                .iter()
                .position(|entry| entry.id == id)
        })
        .unwrap_or(0);

    if let Some(row) = state.list.row_at_index(selected_index as i32) {
        state.list.select_row(Some(&row));
        if let Some(entry) = state.entries.borrow().get(selected_index) {
            render_preview(state, entry);
        }
    } else {
        clear_box(&state.preview);
        clear_box(&state.details);
        state
            .preview
            .append(&muted_label("No clipboard entries yet"));
    }
    set_footer(
        state,
        &format!(
            "{} entries | Enter paste | Ctrl+Enter copy | Ctrl+S save secret | Ctrl+P pin | Ctrl+D delete | Esc close",
            state.entries.borrow().len()
        ),
    );
}

fn render_secrets_list(state: &Rc<AppState>, selected_id: Option<i64>) {
    state.entries.borrow_mut().clear();
    clear_box_like(&state.list);

    for secret in state.secrets.borrow().iter() {
        state.list.append(&secret_row(secret));
    }

    let selected_index = selected_id
        .and_then(|id| {
            state
                .secrets
                .borrow()
                .iter()
                .position(|secret| secret.id == id)
        })
        .unwrap_or(0);

    if let Some(row) = state.list.row_at_index(selected_index as i32) {
        state.list.select_row(Some(&row));
        if let Some(secret) = state.secrets.borrow().get(selected_index) {
            render_secret_preview(state, secret);
        }
    } else {
        clear_box(&state.preview);
        clear_box(&state.details);
        state.preview.append(&muted_label("No secrets saved yet"));
    }
    set_footer(
        state,
        &format!(
            "{} secrets | Enter copy | Ctrl+S copy | Ctrl+E rename | Ctrl+D delete | Esc close",
            state.secrets.borrow().len()
        ),
    );
}

fn entry_row(entry: &ClipboardEntry) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("entry-row");

    let outer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    outer.add_css_class("entry-row-content");
    outer.set_hexpand(true);
    let icon = row_icon(entry_icon_name(entry), entry_kind_label(entry));
    outer.append(&icon);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);
    let title = gtk::Label::new(Some(&entry.title));
    title.add_css_class("entry-title");
    title.set_xalign(0.0);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&title);

    let subtitle = gtk::Label::new(Some(&subtitle(entry)));
    subtitle.add_css_class("entry-subtitle");
    subtitle.set_xalign(0.0);
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&subtitle);
    outer.append(&text);

    if entry.pinned {
        let pinned = badge_icon("view-pin-symbolic", "Pinned");
        outer.append(&pinned);
    }

    row.set_child(Some(&outer));
    row
}

fn secret_row(secret: &SecretEntry) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("entry-row");

    let outer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    outer.add_css_class("entry-row-content");
    outer.set_hexpand(true);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);

    let title = gtk::Label::new(Some(&secret.alias));
    title.add_css_class("entry-title");
    title.set_xalign(0.0);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&title);

    let subtitle = gtk::Label::new(Some(&format!(
        "{} - {}",
        masked_secret(&secret.value),
        relative_time(secret.updated_at)
    )));
    subtitle.add_css_class("entry-subtitle");
    subtitle.set_xalign(0.0);
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    text.append(&subtitle);
    outer.append(&text);

    row.set_child(Some(&outer));
    row
}

fn render_secret_preview(state: &Rc<AppState>, secret: &SecretEntry) {
    clear_box(&state.preview);
    clear_box(&state.details);
    state.ocr_button.set_opacity(0.0);
    state.ocr_button.set_sensitive(false);

    let title = section_label(&secret.alias);
    state.preview.append(&title);

    let buffer = gtk::TextBuffer::new(None);
    buffer.set_text(&masked_secret(&secret.value));
    let view = gtk::TextView::with_buffer(&buffer);
    view.add_css_class("preview-text");
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_monospace(true);
    view.set_vexpand(true);

    let scroller = gtk::ScrolledWindow::builder()
        .min_content_height(80)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&view)
        .build();
    state.preview.append(&scroller);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    let copy_button = gtk::Button::with_label("Copy");
    {
        let state = Rc::clone(state);
        let secret = secret.clone();
        copy_button.connect_clicked(move |_| {
            if let Err(err) = copy_secret(&state, &secret) {
                set_footer(&state, &format!("Copy failed: {err:#}"));
            } else {
                set_footer(&state, "Copied secret");
            }
        });
    }
    actions.append(&copy_button);

    let reveal_button = gtk::Button::with_label("Reveal");
    {
        let buffer = buffer.clone();
        let value = secret.value.clone();
        let masked = masked_secret(&secret.value);
        reveal_button.connect_clicked(move |button| {
            if button.label().as_deref() == Some("Reveal") {
                buffer.set_text(&value);
                button.set_label("Hide");
            } else {
                buffer.set_text(&masked);
                button.set_label("Reveal");
            }
        });
    }
    actions.append(&reveal_button);

    let rename_button = gtk::Button::with_label("Rename");
    {
        let state = Rc::clone(state);
        let window = state
            .list
            .root()
            .and_then(|root| root.downcast::<gtk::Window>().ok());
        rename_button.connect_clicked(move |_| {
            if let Some(window) = window.as_ref() {
                rename_current_secret_dialog(&state, window);
            }
        });
    }
    actions.append(&rename_button);

    let delete_button = gtk::Button::with_label("Delete");
    delete_button.add_css_class("destructive-button");
    {
        let state = Rc::clone(state);
        delete_button.connect_clicked(move |_| {
            if let Err(err) = delete_current(&state) {
                set_footer(&state, &format!("Delete failed: {err:#}"));
            } else {
                set_footer(&state, "Deleted secret");
            }
        });
    }
    actions.append(&delete_button);

    state.preview.append(&actions);
    render_secret_details(state, secret);
}

fn render_preview(state: &Rc<AppState>, entry: &ClipboardEntry) {
    clear_box(&state.preview);
    clear_box(&state.details);
    state
        .ocr_button
        .set_opacity(if matches!(entry.kind, EntryKind::Image) {
            1.0
        } else {
            0.0
        });
    state
        .ocr_button
        .set_sensitive(matches!(entry.kind, EntryKind::Image));

    match entry.kind {
        EntryKind::Image => render_image_preview(&state.preview, entry),
        EntryKind::Color => render_color_preview(&state.preview, entry),
        EntryKind::Link => {
            render_text_preview(
                &state.preview,
                entry.link_url.as_deref().or(entry.text_content.as_deref()),
            );
        }
        _ => render_text_preview(
            &state.preview,
            entry
                .text_content
                .as_deref()
                .or(entry.preview_text.as_deref()),
        ),
    }

    if let Some(ocr) = entry.ocr_text.as_deref().filter(|text| !text.is_empty()) {
        state.preview.append(&section_label("OCR"));
        render_text_preview(&state.preview, Some(ocr));
    }

    render_details(state, entry);
}

fn render_image_preview(container: &gtk::Box, entry: &ClipboardEntry) {
    if let Some(path) = entry.file_path.as_deref() {
        let file = gio::File::for_path(path);
        if let Ok(texture) = gdk::Texture::from_file(&file) {
            let ratio = (texture.width() as f32 / texture.height().max(1) as f32).clamp(0.2, 8.0);
            let frame = gtk::AspectFrame::new(0.5, 0.5, ratio, false);
            frame.set_hexpand(true);
            frame.set_vexpand(true);

            let picture = gtk::Picture::for_paintable(&texture);
            picture.set_content_fit(gtk::ContentFit::Contain);
            picture.set_can_shrink(true);
            picture.set_hexpand(true);
            picture.set_vexpand(true);
            frame.set_child(Some(&picture));
            container.append(&frame);
        } else {
            container.append(&muted_label("Image preview is unavailable"));
        }
    } else {
        container.append(&muted_label("Image file is missing"));
    }
}

fn render_color_preview(container: &gtk::Box, entry: &ClipboardEntry) {
    if let Some(hex) = entry.color_value.as_deref() {
        let frame = gtk::AspectFrame::new(0.5, 0.5, 16.0 / 9.0, false);
        frame.set_hexpand(true);
        frame.set_vexpand(true);

        let swatch = gtk::DrawingArea::new();
        swatch.add_css_class("color-swatch");
        swatch.set_hexpand(true);
        swatch.set_vexpand(true);
        let color = parse_hex_rgb(hex).unwrap_or((0.2, 0.2, 0.2));
        swatch.set_draw_func(move |_, cr, width, height| {
            cr.set_source_rgb(color.0, color.1, color.2);
            cr.rectangle(0.0, 0.0, f64::from(width), f64::from(height));
            let _ = cr.fill();
        });
        frame.set_child(Some(&swatch));
        container.append(&frame);
        render_text_preview(container, Some(hex));
    }
}

fn render_text_preview(container: &gtk::Box, text: Option<&str>) {
    let buffer = gtk::TextBuffer::new(None);
    buffer.set_text(text.unwrap_or(""));
    let view = gtk::TextView::with_buffer(&buffer);
    view.add_css_class("preview-text");
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_monospace(true);
    view.set_vexpand(true);

    let scroller = gtk::ScrolledWindow::builder()
        .min_content_height(80)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&view)
        .build();
    container.append(&scroller);
}

fn render_details(state: &Rc<AppState>, entry: &ClipboardEntry) {
    let details = gtk::Box::new(gtk::Orientation::Vertical, 6);
    details.add_css_class("details-grid");
    details.set_hexpand(true);

    let rows = [
        ("Type", entry.kind.to_string()),
        ("MIME", entry.mime_type.clone()),
        ("Size", human_size(entry.size_bytes)),
        ("First copied", format_full_time(entry.copied_at)),
    ];

    for (label, value) in rows {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.add_css_class("details-row");
        row.set_hexpand(true);

        let key = muted_label(label);
        key.add_css_class("details-key");
        key.set_width_request(96);

        let value = gtk::Label::new(Some(&value));
        value.add_css_class("details-value");
        value.set_hexpand(true);
        value.set_xalign(1.0);
        value.set_justify(gtk::Justification::Right);
        value.set_wrap(true);
        value.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        value.set_selectable(true);

        row.append(&key);
        row.append(&value);
        details.append(&row);
    }

    state.details.append(&details);
}

fn render_secret_details(state: &Rc<AppState>, secret: &SecretEntry) {
    let details = gtk::Box::new(gtk::Orientation::Vertical, 6);
    details.add_css_class("details-grid");
    details.set_hexpand(true);

    let rows = [
        ("Type", "secret".to_string()),
        ("Updated", format_full_time(secret.updated_at)),
        ("Created", format_full_time(secret.created_at)),
        ("Copied", secret.use_count.to_string()),
    ];

    for (label, value) in rows {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.add_css_class("details-row");
        row.set_hexpand(true);

        let key = muted_label(label);
        key.add_css_class("details-key");
        key.set_width_request(96);

        let value = gtk::Label::new(Some(&value));
        value.add_css_class("details-value");
        value.set_hexpand(true);
        value.set_xalign(1.0);
        value.set_justify(gtk::Justification::Right);
        value.set_wrap(true);
        value.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        value.set_selectable(true);

        row.append(&key);
        row.append(&value);
        details.append(&row);
    }

    state.details.append(&details);
}

fn run_ocr_for_entry(state: &Rc<AppState>, entry_id: i64) -> Result<()> {
    set_footer(state, "Running OCR...");

    let db = Database::open(&state.db_path)?;
    let entry = db
        .get_entry(entry_id)?
        .with_context(|| format!("entry {entry_id} not found"))?;
    let image_path = entry
        .file_path
        .as_deref()
        .context("entry does not have an image file path")?;
    let text = run_tesseract(image_path, "eng")?;
    db.save_ocr_result(entry_id, "eng", &text)?;

    let updated = db
        .get_entry(entry_id)?
        .with_context(|| format!("entry {entry_id} not found after OCR"))?;
    if let Some(slot) = state
        .entries
        .borrow_mut()
        .iter_mut()
        .find(|entry| entry.id == entry_id)
    {
        *slot = updated.clone();
    }
    render_preview(state, &updated);
    set_footer(state, "OCR complete");
    Ok(())
}

fn current_entry(state: &Rc<AppState>) -> Option<ClipboardEntry> {
    let row = state.list.selected_row()?;
    let index = row.index();
    if index < 0 {
        return None;
    }
    state.entries.borrow().get(index as usize).cloned()
}

fn current_secret(state: &Rc<AppState>) -> Option<SecretEntry> {
    let row = state.list.selected_row()?;
    let index = row.index();
    if index < 0 {
        return None;
    }
    state.secrets.borrow().get(index as usize).cloned()
}

fn move_selection(state: &Rc<AppState>, delta: i32) {
    let count = match *state.view.borrow() {
        AppView::Clipboard => state.entries.borrow().len() as i32,
        AppView::Secrets => state.secrets.borrow().len() as i32,
    };
    if count == 0 {
        return;
    }

    let current = state
        .list
        .selected_row()
        .map(|row| row.index())
        .unwrap_or(0);
    let next = (current + delta).clamp(0, count - 1);
    if let Some(row) = state.list.row_at_index(next) {
        state.list.select_row(Some(&row));
        scroll_row_into_view(state, &row);
    }
}

fn scroll_row_into_view(state: &Rc<AppState>, row: &gtk::ListBoxRow) {
    let Some(bounds) = row.compute_bounds(&state.list) else {
        return;
    };

    let adjustment = &state.list_adjustment;
    let viewport_top = adjustment.value();
    let viewport_bottom = viewport_top + adjustment.page_size();
    let row_top = f64::from(bounds.y());
    let row_bottom = row_top + f64::from(bounds.height());

    let target = if row_top < viewport_top {
        row_top
    } else if row_bottom > viewport_bottom {
        row_bottom - adjustment.page_size()
    } else {
        return;
    };

    let max_value = (adjustment.upper() - adjustment.page_size()).max(adjustment.lower());
    adjustment.set_value(target.clamp(adjustment.lower(), max_value));
}

fn copy_selected_entry(state: &Rc<AppState>, entry: &ClipboardEntry) -> Result<()> {
    copy_entry(entry)?;
    let db = Database::open(&state.db_path)?;
    db.touch_used(entry.id)?;
    Ok(())
}

fn copy_secret(state: &Rc<AppState>, secret: &SecretEntry) -> Result<()> {
    write_clipboard("text/plain", secret.value.as_bytes())?;
    let db = Database::open(&state.db_path)?;
    db.touch_secret_used(secret.id)?;
    Ok(())
}

fn save_current_as_secret_dialog(state: &Rc<AppState>, parent: &gtk::Window) {
    let Some(entry) = current_entry(state) else {
        set_footer(state, "No selected entry to save");
        return;
    };
    let Some(value) = secret_value_from_entry(&entry) else {
        set_footer(state, "Only text-like entries can be saved as secrets");
        return;
    };

    let default_alias = if matches!(entry.kind, EntryKind::Image)
        && entry
            .ocr_text
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
    {
        "OCR text".to_string()
    } else if entry.title.trim().is_empty() {
        "Untitled secret".to_string()
    } else {
        entry.title.clone()
    };

    prompt_secret_alias(
        state,
        parent,
        "Save Secret",
        &default_alias,
        move |state, alias| {
            let db = Database::open(&state.db_path)?;
            db.save_secret(Some(entry.id), &alias, &value)?;
            db.delete_entry(entry.id)?;
            *state.view.borrow_mut() = AppView::Secrets;
            *state.query.borrow_mut() = String::new();
            state.search_entry.set_text("");
            state
                .search_entry
                .set_placeholder_text(Some("Search secrets by name..."));
            update_mode_controls(state);
            refresh_entries(state)?;
            set_footer(state, "Saved secret");
            Ok(())
        },
    );
}

fn rename_current_secret_dialog(state: &Rc<AppState>, parent: &gtk::Window) {
    let Some(secret) = current_secret(state) else {
        set_footer(state, "No selected secret to rename");
        return;
    };

    prompt_secret_alias(
        state,
        parent,
        "Rename Secret",
        &secret.alias,
        move |state, alias| {
            let db = Database::open(&state.db_path)?;
            db.rename_secret(secret.id, &alias)?;
            refresh_entries(state)?;
            set_footer(state, "Renamed secret");
            Ok(())
        },
    );
}

fn prompt_secret_alias<F>(
    state: &Rc<AppState>,
    parent: &gtk::Window,
    title: &str,
    default_alias: &str,
    on_accept: F,
) where
    F: Fn(&Rc<AppState>, String) -> Result<()> + 'static,
{
    let Some(root) = parent
        .child()
        .and_then(|child| child.downcast::<gtk::Overlay>().ok())
    else {
        set_footer(state, "Secret prompt failed: overlay root is unavailable");
        return;
    };

    let scrim = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scrim.add_css_class("inline-dialog-scrim");
    scrim.set_halign(gtk::Align::Fill);
    scrim.set_valign(gtk::Align::Fill);
    scrim.set_hexpand(true);
    scrim.set_vexpand(true);

    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.add_css_class("inline-dialog");
    panel.set_halign(gtk::Align::Center);
    panel.set_valign(gtk::Align::Center);
    panel.set_width_request(320);

    let title_label = gtk::Label::new(Some(title));
    title_label.add_css_class("inline-dialog-title");
    title_label.set_xalign(0.0);
    panel.append(&title_label);

    let label = gtk::Label::new(Some("Name"));
    label.add_css_class("muted");
    label.set_xalign(0.0);
    panel.append(&label);

    let alias = gtk::Entry::new();
    alias.add_css_class("inline-dialog-entry");
    alias.set_text(default_alias);
    panel.append(&alias);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    actions.add_css_class("inline-dialog-actions");
    actions.set_halign(gtk::Align::End);

    let cancel = gtk::Button::with_label("Cancel");
    let save = gtk::Button::with_label("Save");
    save.add_css_class("primary-button");
    actions.append(&cancel);
    actions.append(&save);
    panel.append(&actions);

    root.add_overlay(&scrim);
    root.add_overlay(&panel);
    *state.prompt_active.borrow_mut() = true;

    let on_accept = Rc::new(on_accept);
    {
        let root = root.clone();
        let panel = panel.clone();
        let scrim = scrim.clone();
        let state = Rc::clone(state);
        cancel.connect_clicked(move |_| {
            *state.prompt_active.borrow_mut() = false;
            root.remove_overlay(&panel);
            root.remove_overlay(&scrim);
            state.search_entry.grab_focus();
        });
    }

    {
        let root = root.clone();
        let panel = panel.clone();
        let scrim = scrim.clone();
        let state = Rc::clone(state);
        let alias = alias.clone();
        let on_accept = Rc::clone(&on_accept);
        save.connect_clicked(move |_| {
            let alias = alias.text().to_string();
            if let Err(err) = on_accept(&state, alias) {
                set_footer(&state, &format!("Secret failed: {err:#}"));
                return;
            }
            *state.prompt_active.borrow_mut() = false;
            root.remove_overlay(&panel);
            root.remove_overlay(&scrim);
            state.search_entry.grab_focus();
        });
    }

    {
        let root = root.clone();
        let panel = panel.clone();
        let scrim = scrim.clone();
        let state = Rc::clone(state);
        let on_accept = Rc::clone(&on_accept);
        alias.connect_activate(move |entry| {
            let alias = entry.text().to_string();
            if let Err(err) = on_accept(&state, alias) {
                set_footer(&state, &format!("Secret failed: {err:#}"));
                return;
            }
            *state.prompt_active.borrow_mut() = false;
            root.remove_overlay(&panel);
            root.remove_overlay(&scrim);
            state.search_entry.grab_focus();
        });
    }

    let controller = gtk::EventControllerKey::new();
    {
        let root = root.clone();
        let panel = panel.clone();
        let scrim = scrim.clone();
        let state = Rc::clone(state);
        controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                *state.prompt_active.borrow_mut() = false;
                root.remove_overlay(&panel);
                root.remove_overlay(&scrim);
                state.search_entry.grab_focus();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    alias.add_controller(controller);

    alias.grab_focus();
}

fn toggle_pin(state: &Rc<AppState>) -> Result<()> {
    let entry = current_entry(state).context("no selected entry")?;
    let db = Database::open(&state.db_path)?;
    db.set_pinned(entry.id, !entry.pinned)?;
    refresh_entries(state)
}

fn delete_current(state: &Rc<AppState>) -> Result<()> {
    let db = Database::open(&state.db_path)?;
    match *state.view.borrow() {
        AppView::Clipboard => {
            let entry = current_entry(state).context("no selected entry")?;
            db.delete_entry(entry.id)?;
        }
        AppView::Secrets => {
            let secret = current_secret(state).context("no selected secret")?;
            db.delete_secret(secret.id)?;
        }
    }
    refresh_entries(state)
}

fn update_mode_controls(state: &Rc<AppState>) {
    let view = *state.view.borrow();
    state
        .filter_select
        .set_sensitive(matches!(view, AppView::Clipboard));

    state.history_button.remove_css_class("active-mode");
    state.secrets_button.remove_css_class("active-mode");
    match view {
        AppView::Clipboard => state.history_button.add_css_class("active-mode"),
        AppView::Secrets => state.secrets_button.add_css_class("active-mode"),
    }
}

fn secret_value_from_entry(entry: &ClipboardEntry) -> Option<String> {
    match entry.kind {
        EntryKind::Image => entry
            .ocr_text
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
        EntryKind::Color | EntryKind::File => None,
        EntryKind::Link => entry
            .link_url
            .as_deref()
            .or(entry.text_content.as_deref())
            .or(entry.preview_text.as_deref())
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
        _ => entry
            .text_content
            .as_deref()
            .or(entry.preview_text.as_deref())
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
    }
}

fn masked_secret(value: &str) -> String {
    let visible_tail = value.chars().rev().take(4).collect::<Vec<_>>();
    let tail = visible_tail.into_iter().rev().collect::<String>();
    if tail.is_empty() {
        "********".to_string()
    } else {
        format!("********{tail}")
    }
}

fn clear_box(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn clear_box_like(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn mark_selected_row(list: &gtk::ListBox, selected: Option<&gtk::ListBoxRow>) {
    let mut child = list.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        widget.remove_css_class("selected-entry");
    }

    if let Some(row) = selected {
        row.add_css_class("selected-entry");
    }
}

fn set_footer(state: &Rc<AppState>, text: &str) {
    state.footer.set_text(text);
}

fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("entry-title");
    label.set_xalign(0.0);
    label
}

fn muted_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("muted");
    label.set_xalign(0.0);
    label.set_wrap(true);
    label
}

fn row_icon(icon_name: &str, tooltip: &str) -> gtk::Image {
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class("entry-kind");
    icon.set_tooltip_text(Some(tooltip));
    icon.set_pixel_size(16);
    icon
}

fn badge_icon(icon_name: &str, tooltip: &str) -> gtk::Image {
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class("kind-badge");
    icon.set_tooltip_text(Some(tooltip));
    icon.set_pixel_size(12);
    icon
}

fn entry_icon_name(entry: &ClipboardEntry) -> &'static str {
    match entry.kind {
        EntryKind::Text => "text-x-generic-symbolic",
        EntryKind::Image => "image-x-generic-symbolic",
        EntryKind::Link => match entry.link_icon.as_deref() {
            Some("github") => "code-context-symbolic",
            Some("youtube") => "video-x-generic-symbolic",
            Some("rust") => "application-x-executable-symbolic",
            _ => "emblem-shared-symbolic",
        },
        EntryKind::Color => "color-select-symbolic",
        EntryKind::File => "folder-symbolic",
        EntryKind::Unknown => "dialog-question-symbolic",
    }
}

fn entry_kind_label(entry: &ClipboardEntry) -> &'static str {
    match entry.kind {
        EntryKind::Text => "Text",
        EntryKind::Image => "Image",
        EntryKind::Link => "Link",
        EntryKind::Color => "Color",
        EntryKind::File => "File",
        EntryKind::Unknown => "Unknown",
    }
}

fn subtitle(entry: &ClipboardEntry) -> String {
    relative_time(entry.updated_at)
}

fn format_full_time(timestamp: i64) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%a %b %-d %H:%M:%S %Y")
                .to_string()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn relative_time(timestamp: i64) -> String {
    let seconds = (Utc::now().timestamp() - timestamp).max(0);
    if seconds < 60 {
        "now".to_string()
    } else if seconds < 3_600 {
        let minutes = seconds / 60;
        format!("{minutes} min")
    } else if seconds < 86_400 {
        let hours = seconds / 3_600;
        format!("{hours} hr")
    } else {
        let days = seconds / 86_400;
        format!("{days} day")
    }
}

fn human_size(bytes: i64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn parse_hex_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
    Some((red, green, blue))
}
