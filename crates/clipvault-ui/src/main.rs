use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use clipvault_core::models::{ClipboardEntry, EntryFilter, EntryKind, SortMode};
use clipvault_core::paste::paste_entry;
use clipvault_core::{ClipvaultPaths, Database};
use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;

const APP_ID: &str = "io.github.radhey.clipvault";

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "clipvault=warn".into()),
        )
        .init();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("list") {
        if let Err(err) = cmd_list(&args[1..]) {
            eprintln!("clipvault: {err:#}");
            std::process::exit(1);
        }
        return;
    }

    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        if let Err(err) = build_ui(app) {
            eprintln!("clipvault: {err:#}");
            app.quit();
        }
    });
    app.run();
}

struct AppState {
    db_path: PathBuf,
    entries: RefCell<Vec<ClipboardEntry>>,
    query: RefCell<String>,
    filter: RefCell<EntryFilter>,
    sort: RefCell<SortMode>,
    list: gtk::ListBox,
    preview: gtk::Box,
    details: gtk::Box,
    footer: gtk::Label,
}

fn build_ui(app: &gtk::Application) -> Result<()> {
    let paths = ClipvaultPaths::discover()?;
    paths.ensure()?;
    Database::open(&paths.db_path)?;

    load_css();

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Clipvault")
        .default_width(920)
        .default_height(620)
        .resizable(true)
        .build();
    window.add_css_class("clipvault-window");

    let shell = gtk::Box::new(gtk::Orientation::Vertical, 0);
    shell.add_css_class("app-shell");
    window.set_child(Some(&shell));

    let topbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    topbar.add_css_class("topbar");
    shell.append(&topbar);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search clipboard..."));
    search.add_css_class("search-box");
    search.set_hexpand(true);
    topbar.append(&search);

    let filter =
        gtk::DropDown::from_strings(&["All", "Text", "Images", "Links", "Colors", "Pinned"]);
    filter.set_selected(0);
    topbar.append(&filter);

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_wide_handle(true);
    paned.set_vexpand(true);
    shell.append(&paned);

    let list_scroller = gtk::ScrolledWindow::builder()
        .min_content_width(340)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    list_scroller.add_css_class("sidebar");
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list_scroller.set_child(Some(&list));
    paned.set_start_child(Some(&list_scroller));

    let preview_shell = gtk::Box::new(gtk::Orientation::Vertical, 12);
    preview_shell.set_vexpand(true);
    preview_shell.add_css_class("preview-pane");

    let preview = gtk::Box::new(gtk::Orientation::Vertical, 12);
    preview.set_vexpand(true);
    let preview_scroller = gtk::ScrolledWindow::builder()
        .min_content_width(480)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&preview)
        .build();
    preview_shell.append(&preview_scroller);

    let details = gtk::Box::new(gtk::Orientation::Vertical, 6);
    details.add_css_class("details-panel");
    details.set_hexpand(true);
    preview_shell.append(&details);

    paned.set_end_child(Some(&preview_shell));
    paned.set_position(360);

    let footer = gtk::Label::new(Some(
        "Enter paste  Ctrl+Enter copy  Ctrl+P pin  Ctrl+D delete  Esc close",
    ));
    footer.add_css_class("footer");
    footer.add_css_class("muted");
    footer.set_xalign(0.0);
    footer.set_wrap(true);
    footer.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    shell.append(&footer);

    let state = Rc::new(AppState {
        db_path: paths.db_path,
        entries: RefCell::new(Vec::new()),
        query: RefCell::new(String::new()),
        filter: RefCell::new(EntryFilter::All),
        sort: RefCell::new(SortMode::Default),
        list: list.clone(),
        preview: preview.clone(),
        details: details.clone(),
        footer,
    });

    refresh_entries(&state)?;

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
        list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let index = row.index();
                if index >= 0 {
                    if let Some(entry) = state.entries.borrow().get(index as usize) {
                        render_preview(&state, entry);
                    }
                }
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let app = app.clone();
        list.connect_row_activated(move |_, row| {
            if let Some(entry) = state.entries.borrow().get(row.index() as usize).cloned() {
                if let Err(err) = paste_selected(&state, &entry, true) {
                    set_footer(&state, &format!("Paste failed: {err:#}"));
                    return;
                }
                app.quit();
            }
        });
    }

    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let state = Rc::clone(&state);
        let app = app.clone();
        controller.connect_key_pressed(move |_, key, _, modifiers| {
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
                    if let Some(entry) = current_entry(&state) {
                        if let Err(err) = paste_selected(&state, &entry, true) {
                            set_footer(&state, &format!("Paste failed: {err:#}"));
                        } else {
                            app.quit();
                        }
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::Return | gdk::Key::KP_Enter, true) => {
                    if let Some(entry) = current_entry(&state) {
                        if let Err(err) = paste_selected(&state, &entry, false) {
                            set_footer(&state, &format!("Copy failed: {err:#}"));
                        } else {
                            set_footer(&state, "Copied selected entry");
                        }
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::p | gdk::Key::P, true) => {
                    if let Err(err) = toggle_pin(&state) {
                        set_footer(&state, &format!("Pin failed: {err:#}"));
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::d | gdk::Key::D, true) => {
                    if let Err(err) = delete_current(&state) {
                        set_footer(&state, &format!("Delete failed: {err:#}"));
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
                    *state.filter.borrow_mut() = EntryFilter::Images;
                    if let Err(err) = refresh_entries(&state) {
                        set_footer(&state, &format!("Filter failed: {err:#}"));
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::l | gdk::Key::L, true) => {
                    *state.filter.borrow_mut() = EntryFilter::Links;
                    if let Err(err) = refresh_entries(&state) {
                        set_footer(&state, &format!("Filter failed: {err:#}"));
                    }
                    glib::Propagation::Stop
                }
                (gdk::Key::c | gdk::Key::C, true) => {
                    *state.filter.borrow_mut() = EntryFilter::Colors;
                    if let Err(err) = refresh_entries(&state) {
                        set_footer(&state, &format!("Filter failed: {err:#}"));
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

fn refresh_entries(state: &Rc<AppState>) -> Result<()> {
    let db = Database::open(&state.db_path)?;
    let entries = db.list_entries(
        &state.query.borrow(),
        *state.filter.borrow(),
        *state.sort.borrow(),
        200,
    )?;
    *state.entries.borrow_mut() = entries;
    clear_box_like(&state.list);

    for entry in state.entries.borrow().iter() {
        state.list.append(&entry_row(entry));
    }

    if let Some(first) = state.list.row_at_index(0) {
        state.list.select_row(Some(&first));
        if let Some(entry) = state.entries.borrow().first() {
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
            "{} entries | Enter paste | Ctrl+Enter copy | Ctrl+P pin | Ctrl+D delete | Esc close",
            state.entries.borrow().len()
        ),
    );
    Ok(())
}

fn entry_row(entry: &ClipboardEntry) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("entry-row");

    let outer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let icon = gtk::Label::new(Some(kind_icon(entry)));
    icon.add_css_class("entry-kind");
    icon.set_width_request(36);
    icon.set_xalign(0.5);
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
        let pinned = gtk::Label::new(Some("PIN"));
        pinned.add_css_class("kind-badge");
        outer.append(&pinned);
    }

    row.set_child(Some(&outer));
    row
}

fn render_preview(state: &Rc<AppState>, entry: &ClipboardEntry) {
    clear_box(&state.preview);
    clear_box(&state.details);

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

    render_details(&state.details, entry);
}

fn render_image_preview(container: &gtk::Box, entry: &ClipboardEntry) {
    if let Some(path) = entry.file_path.as_deref() {
        let file = gio::File::for_path(path);
        let picture = gtk::Picture::for_file(&file);
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_size_request(360, 300);
        container.append(&picture);
    } else {
        container.append(&muted_label("Image file is missing"));
    }
}

fn render_color_preview(container: &gtk::Box, entry: &ClipboardEntry) {
    if let Some(hex) = entry.color_value.as_deref() {
        let swatch = gtk::DrawingArea::new();
        swatch.add_css_class("color-swatch");
        swatch.set_content_width(400);
        swatch.set_content_height(220);
        swatch.set_hexpand(true);
        let color = parse_hex_rgb(hex).unwrap_or((0.2, 0.2, 0.2));
        swatch.set_draw_func(move |_, cr, width, height| {
            cr.set_source_rgb(color.0, color.1, color.2);
            cr.rectangle(0.0, 0.0, f64::from(width), f64::from(height));
            let _ = cr.fill();
        });
        container.append(&swatch);
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
        .min_content_height(240)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&view)
        .build();
    container.append(&scroller);
}

fn render_details(container: &gtk::Box, entry: &ClipboardEntry) {
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

    container.append(&details);
}

fn current_entry(state: &Rc<AppState>) -> Option<ClipboardEntry> {
    let row = state.list.selected_row()?;
    let index = row.index();
    if index < 0 {
        return None;
    }
    state.entries.borrow().get(index as usize).cloned()
}

fn move_selection(state: &Rc<AppState>, delta: i32) {
    let count = state.entries.borrow().len() as i32;
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
    }
}

fn paste_selected(state: &Rc<AppState>, entry: &ClipboardEntry, auto_paste: bool) -> Result<()> {
    paste_entry(entry, auto_paste, 80)?;
    let db = Database::open(&state.db_path)?;
    db.touch_used(entry.id)?;
    Ok(())
}

fn toggle_pin(state: &Rc<AppState>) -> Result<()> {
    let entry = current_entry(state).context("no selected entry")?;
    let db = Database::open(&state.db_path)?;
    db.set_pinned(entry.id, !entry.pinned)?;
    refresh_entries(state)
}

fn delete_current(state: &Rc<AppState>) -> Result<()> {
    let entry = current_entry(state).context("no selected entry")?;
    let db = Database::open(&state.db_path)?;
    db.delete_entry(entry.id)?;
    refresh_entries(state)
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

fn kind_icon(entry: &ClipboardEntry) -> &'static str {
    match entry.kind {
        EntryKind::Text => "T",
        EntryKind::Image => "IMG",
        EntryKind::Link => match entry.link_icon.as_deref() {
            Some("github") => "GH",
            Some("youtube") => "YT",
            Some("rust") => "RS",
            _ => "URL",
        },
        EntryKind::Color => "COL",
        EntryKind::File => "FILE",
        EntryKind::Unknown => "?",
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
