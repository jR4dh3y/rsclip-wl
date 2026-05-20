use anyhow::Result;
use rsclip_core::models::ClipboardEntry;

pub fn print_entries(entries: &[ClipboardEntry], json: bool) -> Result<()> {
    rsclip_core::cli::print_entries(entries, json)
}
