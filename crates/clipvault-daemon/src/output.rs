use anyhow::Result;
use clipvault_core::models::ClipboardEntry;

pub fn print_entries(entries: &[ClipboardEntry], json: bool) -> Result<()> {
    clipvault_core::cli::print_entries(entries, json)
}
