use std::io::{self, Read};

use anyhow::{Context, Result};
use rsclip_core::cli::option_value;
use rsclip_core::notify::notify_changed;
use rsclip_core::storage::{content_hash, store_image};
use rsclip_core::{RsclipPaths, Database, classify_payload};

pub fn run(args: &[String]) -> Result<()> {
    let mime_type = option_value(args, "--mime").unwrap_or("text/plain");
    let mut payload = Vec::new();
    io::stdin()
        .read_to_end(&mut payload)
        .context("reading clipboard payload from stdin")?;

    if payload.is_empty() {
        return Ok(());
    }

    let paths = RsclipPaths::discover()?;
    paths.ensure()?;
    let db = Database::open(&paths.db_path)?;
    let hash = content_hash(&payload);
    let mut entry = classify_payload(mime_type, hash.clone(), &payload)?;
    if mime_type.starts_with("image/") {
        let path = store_image(&paths, &hash, mime_type, &payload)?;
        entry.file_path = Some(path.to_string_lossy().to_string());
    }

    let id = db.upsert_entry(&entry)?;
    notify_changed(&paths);
    println!("{id}");
    Ok(())
}
