use clipvault_core::format::{masked_secret, relative_time};
use clipvault_core::models::{ClipboardEntry, EntryKind, SecretEntry};
use gtk::prelude::*;
use gtk4 as gtk;

pub(crate) struct ListPanel {
    pub(crate) scroller: gtk::ScrolledWindow,
    pub(crate) list: gtk::ListBox,
    pub(crate) adjustment: gtk::Adjustment,
}

pub(crate) fn build_panel() -> ListPanel {
    let scroller = gtk::ScrolledWindow::builder()
        .min_content_width(220)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    scroller.add_css_class("sidebar");

    let list = gtk::ListBox::new();
    list.add_css_class("entry-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    scroller.set_child(Some(&list));

    let adjustment = scroller.vadjustment();
    list.set_adjustment(Some(&adjustment));

    ListPanel {
        scroller,
        list,
        adjustment,
    }
}

pub(crate) fn entry_row(entry: &ClipboardEntry) -> gtk::ListBoxRow {
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

pub(crate) fn secret_row(secret: &SecretEntry) -> gtk::ListBoxRow {
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
