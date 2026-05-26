use std::rc::Rc;

use anyhow::Result;
use rsclip_core::Database;
use rsclip_core::models::{ClipboardEntry, SecretEntry};
use rsclip_core::paste::{copy_entry, write_clipboard};

use crate::state::AppState;

pub(crate) fn copy_selected_entry(state: &Rc<AppState>, entry: &ClipboardEntry) -> Result<()> {
    copy_entry(entry)?;
    let db = Database::open(&state.db_path)?;
    db.touch_used(entry.id)?;
    Ok(())
}

pub(crate) fn copy_secret(state: &Rc<AppState>, secret: &SecretEntry) -> Result<()> {
    write_clipboard("text/plain", secret.value.as_bytes())?;
    let db = Database::open(&state.db_path)?;
    db.touch_secret_used(secret.id)?;
    Ok(())
}

pub(crate) fn copy_text(text: &str) -> Result<()> {
    write_clipboard("text/plain", text.as_bytes())
}
