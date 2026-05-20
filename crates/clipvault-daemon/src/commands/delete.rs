use anyhow::Result;
use clipvault_core::cli::positional_i64;
use clipvault_core::notify::notify_changed;
use clipvault_core::{ClipvaultPaths, Database};

pub fn run(args: &[String]) -> Result<()> {
    let id = positional_i64(args, 0, "entry id")?;
    let paths = ClipvaultPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    db.delete_entry(id)?;
    notify_changed(&paths);
    Ok(())
}
