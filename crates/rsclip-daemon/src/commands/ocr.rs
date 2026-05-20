use anyhow::{Context, Result};
use rsclip_core::cli::{option_value, positional_i64};
use rsclip_core::notify::notify_changed;
use rsclip_core::{RsclipPaths, Database};

pub fn run(args: &[String]) -> Result<()> {
    let id = positional_i64(args, 0, "entry id")?;
    let language = option_value(args, "--lang").unwrap_or("eng");
    let paths = RsclipPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    let entry = db
        .get_entry(id)?
        .with_context(|| format!("entry {id} not found"))?;
    let image_path = entry
        .file_path
        .as_deref()
        .context("entry does not have an image file path")?;
    let text = rsclip_core::ocr::run_tesseract(image_path, language)?;
    db.save_ocr_result(id, language, &text)?;
    notify_changed(&paths);
    println!("{text}");
    Ok(())
}
