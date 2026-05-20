use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::models::{ClipboardEntry, EntryData};

pub fn copy_entry(entry: &ClipboardEntry) -> Result<()> {
    match &entry.data {
        EntryData::Image { file_path, .. } => {
            let bytes = fs::read(file_path).with_context(|| format!("reading {file_path}"))?;
            write_clipboard(&entry.mime_type, &bytes)
        }
        _ => {
            let text = entry
                .text_content
                .as_deref()
                .or(entry.preview_text.as_deref())
                .context("entry has no text content")?;
            write_clipboard("text/plain", text.as_bytes())
        }
    }
}

pub fn paste_entry(entry: &ClipboardEntry, auto_paste: bool, delay_ms: u64) -> Result<()> {
    copy_entry(entry)?;
    if auto_paste {
        thread::sleep(Duration::from_millis(delay_ms));
        trigger_paste()?;
    }
    Ok(())
}

pub fn write_clipboard(mime_type: &str, bytes: &[u8]) -> Result<()> {
    let mut child = Command::new("wl-copy")
        .arg("--type")
        .arg(mime_type)
        .stdin(Stdio::piped())
        .spawn()
        .context("spawning wl-copy")?;
    child
        .stdin
        .as_mut()
        .context("opening wl-copy stdin")?
        .write_all(bytes)
        .context("writing clipboard payload")?;
    let status = child.wait().context("waiting for wl-copy")?;
    if !status.success() {
        bail!("wl-copy exited with {status}");
    }
    Ok(())
}

pub fn trigger_paste() -> Result<()> {
    let status = Command::new("wtype")
        .args(["-M", "ctrl", "v", "-m", "ctrl"])
        .status()
        .context("spawning wtype")?;
    if !status.success() {
        bail!("wtype exited with {status}");
    }
    Ok(())
}
