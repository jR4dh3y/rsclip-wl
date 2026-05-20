use std::rc::Rc;

use anyhow::{Context, Result};
use rsclip_core::Database;
use rsclip_core::ocr::run_tesseract;
use rsclip_core::EntryData;

use crate::actions::set_footer;
use crate::components::preview::render_preview;
use crate::state::AppState;

pub(crate) fn run_ocr_for_entry(state: &Rc<AppState>, entry_id: i64) -> Result<()> {
    set_footer(state, "Running OCR...");

    let db = Database::open(&state.db_path)?;
    let entry = db
        .get_entry(entry_id)?
        .with_context(|| format!("entry {entry_id} not found"))?;
    let image_path = match &entry.data {
        EntryData::Image { file_path, .. } => file_path.as_str(),
        _ => anyhow::bail!("entry is not an image"),
    };
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
