use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use rsclip_core::models::{ClipboardEntry, EntryFilter, SecretEntry, SortMode};
use gtk::prelude::*;
use gtk4 as gtk;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppView {
    Clipboard,
    Secrets,
}

pub(crate) struct AppState {
    pub(crate) db_path: PathBuf,
    pub(crate) entries: RefCell<Vec<ClipboardEntry>>,
    pub(crate) secrets: RefCell<Vec<SecretEntry>>,
    pub(crate) query: RefCell<String>,
    pub(crate) filter: RefCell<EntryFilter>,
    pub(crate) sort: RefCell<SortMode>,
    pub(crate) view: RefCell<AppView>,
    pub(crate) prompt_active: RefCell<bool>,
    pub(crate) search_entry: gtk::SearchEntry,
    pub(crate) filter_select: gtk::DropDown,
    pub(crate) history_button: gtk::Button,
    pub(crate) secrets_button: gtk::Button,
    pub(crate) count_label: gtk::Label,
    pub(crate) list: gtk::ListBox,
    pub(crate) list_adjustment: gtk::Adjustment,
    pub(crate) preview: gtk::Box,
    pub(crate) details: gtk::Box,
    pub(crate) footer: gtk::Label,
    pub(crate) ocr_button: gtk::Button,
}

pub(crate) fn current_entry(state: &Rc<AppState>) -> Option<ClipboardEntry> {
    let row = state.list.selected_row()?;
    let index = row.index();
    if index < 0 {
        return None;
    }
    state.entries.borrow().get(index as usize).cloned()
}

pub(crate) fn current_secret(state: &Rc<AppState>) -> Option<SecretEntry> {
    let row = state.list.selected_row()?;
    let index = row.index();
    if index < 0 {
        return None;
    }
    state.secrets.borrow().get(index as usize).cloned()
}
