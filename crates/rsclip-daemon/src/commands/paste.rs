use anyhow::{Context, Result};
use rsclip_core::cli::{flag, option_value, positional_i64};
use rsclip_core::notify::notify_changed;
use rsclip_core::paste::paste_entry;
use rsclip_core::{RsclipPaths, Database};

pub fn run(args: &[String]) -> Result<()> {
    let id = positional_i64(args, 0, "entry id")?;
    let auto_paste = !flag(args, "--copy-only");
    let delay_ms = option_value(args, "--delay-ms")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(80);
    let paths = RsclipPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    let entry = db
        .get_entry(id)?
        .with_context(|| format!("entry {id} not found"))?;
    paste_entry(&entry, auto_paste, delay_ms)?;
    db.touch_used(id)?;
    notify_changed(&paths);
    Ok(())
}
