pub(crate) mod details;
pub(crate) mod footer;
pub(crate) mod labels;
pub(crate) mod list;
pub(crate) mod preview;
pub(crate) mod topbar;

use gtk::prelude::*;
use gtk4 as gtk;

pub(crate) fn clear_box(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

pub(crate) fn clear_list(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}
