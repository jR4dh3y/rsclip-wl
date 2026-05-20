use anyhow::Result;
use clipvault_core::cli::{flag, positional_i64};
use clipvault_core::notify::notify_changed;
use clipvault_core::{ClipvaultPaths, Database};

pub fn run(args: &[String]) -> Result<()> {
    let id = positional_i64(args, 0, "entry id")?;
    let pinned = !flag(args, "--off");
    let paths = ClipvaultPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    db.set_pinned(id, pinned)?;
    notify_changed(&paths);
    Ok(())
}
