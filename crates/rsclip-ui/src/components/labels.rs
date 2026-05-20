use gtk::prelude::*;
use gtk4 as gtk;

pub(crate) fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("entry-title");
    label.set_xalign(0.0);
    label
}

pub(crate) fn muted_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("muted");
    label.set_xalign(0.0);
    label.set_wrap(true);
    label
}
