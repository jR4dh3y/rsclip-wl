use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::state::{AppState, AppView};

pub(crate) fn move_selection(state: &Rc<AppState>, delta: i32) {
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

pub(crate) fn mark_selected_row(list: &gtk::ListBox, selected: Option<&gtk::ListBoxRow>) {
    let mut child = list.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        widget.remove_css_class("selected-entry");
    }

    if let Some(row) = selected {
        row.add_css_class("selected-entry");
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
