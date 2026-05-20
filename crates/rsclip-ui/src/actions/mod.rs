pub(crate) mod clipboard;
pub(crate) mod ocr;
pub(crate) mod refresh;
pub(crate) mod secrets;
pub(crate) mod selection;

use std::rc::Rc;

use gtk::prelude::*;
use gtk4 as gtk;

use crate::state::{AppState, AppView};

pub(crate) fn set_footer(state: &Rc<AppState>, text: &str) {
    state.footer.set_text(text);
}

pub(crate) fn update_mode_controls(state: &Rc<AppState>) {
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
