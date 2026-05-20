use std::io::{self, Read};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clipvault_core::models::{EntryFilter, SortMode};
use clipvault_core::paste::paste_entry;
use clipvault_core::storage::{content_hash, store_image};
use clipvault_core::{ClipvaultPaths, Database, classify_payload};
use tracing::{error, info, warn};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "clipvaultd=info".into()),
        )
        .init();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("store") => cmd_store(&args[1..]),
        Some("list") => cmd_list(&args[1..]),
        Some("watch") | None => cmd_watch(),
        Some("pin") => cmd_pin(&args[1..]),
        Some("delete") => cmd_delete(&args[1..]),
        Some("paste") => cmd_paste(&args[1..]),
        Some("ocr") => cmd_ocr(&args[1..]),
        Some("help" | "--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some(command) => bail!("unknown command: {command}"),
    }
}

fn cmd_store(args: &[String]) -> Result<()> {
    let mime_type = option_value(args, "--mime").unwrap_or("text/plain");
    let mut payload = Vec::new();
    io::stdin()
        .read_to_end(&mut payload)
        .context("reading clipboard payload from stdin")?;

    if payload.is_empty() {
        return Ok(());
    }

    let paths = ClipvaultPaths::discover()?;
    paths.ensure()?;
    let db = Database::open(&paths.db_path)?;
    let hash = content_hash(&payload);
    let mut entry = classify_payload(mime_type, hash.clone(), &payload)?;
    if mime_type.starts_with("image/") {
        let path = store_image(&paths, &hash, mime_type, &payload)?;
        entry.file_path = Some(path.to_string_lossy().to_string());
    }

    let id = db.upsert_entry(&entry)?;
    println!("{id}");
    Ok(())
}

fn cmd_list(args: &[String]) -> Result<()> {
    let query = option_value(args, "--query").unwrap_or("");
    let filter = EntryFilter::parse(option_value(args, "--filter").unwrap_or("all"));
    let sort = SortMode::parse(option_value(args, "--sort").unwrap_or("default"));
    let limit = option_value(args, "--limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100);
    let json = flag(args, "--json");

    let paths = ClipvaultPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    let entries = db.list_entries(query, filter, sort, limit)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        for entry in entries {
            println!(
                "#{:<4} {:<6} {:<1} {}",
                entry.id,
                entry.kind,
                if entry.pinned { "P" } else { " " },
                entry.title
            );
        }
    }
    Ok(())
}

fn cmd_watch() -> Result<()> {
    require_command("wl-paste")?;
    info!("starting clipvault clipboard watchers");
    let mut watchers = vec![
        Watcher::new("text", "text", "text/plain"),
        Watcher::new("image", "image/png", "image/png"),
    ];

    loop {
        for watcher in &mut watchers {
            watcher.ensure_running()?;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn cmd_pin(args: &[String]) -> Result<()> {
    let id = positional_i64(args, 0, "entry id")?;
    let pinned = !flag(args, "--off");
    let paths = ClipvaultPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    db.set_pinned(id, pinned)?;
    Ok(())
}

fn cmd_delete(args: &[String]) -> Result<()> {
    let id = positional_i64(args, 0, "entry id")?;
    let paths = ClipvaultPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    db.delete_entry(id)?;
    Ok(())
}

fn cmd_paste(args: &[String]) -> Result<()> {
    let id = positional_i64(args, 0, "entry id")?;
    let auto_paste = !flag(args, "--copy-only");
    let delay_ms = option_value(args, "--delay-ms")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(80);
    let paths = ClipvaultPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    let entry = db
        .get_entry(id)?
        .with_context(|| format!("entry {id} not found"))?;
    paste_entry(&entry, auto_paste, delay_ms)?;
    db.touch_used(id)?;
    Ok(())
}

fn cmd_ocr(args: &[String]) -> Result<()> {
    let id = positional_i64(args, 0, "entry id")?;
    let language = option_value(args, "--lang").unwrap_or("eng");
    let paths = ClipvaultPaths::discover()?;
    let db = Database::open(&paths.db_path)?;
    let entry = db
        .get_entry(id)?
        .with_context(|| format!("entry {id} not found"))?;
    let image_path = entry
        .file_path
        .as_deref()
        .context("entry does not have an image file path")?;
    let text = clipvault_core::ocr::run_tesseract(image_path, language)?;
    db.save_ocr_result(id, language, &text)?;
    println!("{text}");
    Ok(())
}

struct Watcher {
    label: &'static str,
    wl_type: &'static str,
    mime_type: &'static str,
    child: Option<Child>,
}

impl Watcher {
    fn new(label: &'static str, wl_type: &'static str, mime_type: &'static str) -> Self {
        Self {
            label,
            wl_type,
            mime_type,
            child: None,
        }
    }

    fn ensure_running(&mut self) -> Result<()> {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait()? {
                None => return Ok(()),
                Some(status) => warn!("{} watcher exited with {}", self.label, status),
            }
        }

        let exe = std::env::current_exe().context("finding current executable")?;
        let child = Command::new("wl-paste")
            .arg("--type")
            .arg(self.wl_type)
            .arg("--watch")
            .arg(exe)
            .arg("store")
            .arg("--mime")
            .arg(self.mime_type)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("spawning {} wl-paste watcher", self.label))?;
        info!("started {} watcher", self.label);
        self.child = Some(child);
        Ok(())
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            if let Err(err) = child.kill() {
                error!("failed to stop {} watcher: {err}", self.label);
            }
        }
    }
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].as_str())
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}

fn positional_i64(args: &[String], index: usize, label: &str) -> Result<i64> {
    args.iter()
        .filter(|arg| !arg.starts_with('-'))
        .nth(index)
        .with_context(|| format!("missing {label}"))?
        .parse::<i64>()
        .with_context(|| format!("invalid {label}"))
}

fn require_command(command: &str) -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {command} >/dev/null"))
        .status()
        .with_context(|| format!("checking for {command}"))?;
    if !status.success() {
        bail!("{command} is required but was not found in PATH");
    }
    Ok(())
}

fn print_help() {
    println!(
        r#"clipvaultd

Commands:
  watch                              Start wl-paste watchers
  store --mime text/plain            Store stdin as a clipboard entry
  list [--json] [--query q]           List history
  pin <id> [--off]                   Pin or unpin an entry
  delete <id>                        Soft-delete an entry
  paste <id> [--copy-only]            Restore an entry and optionally paste
  ocr <id> [--lang eng]              Run tesseract OCR for an image entry
"#
    );
}
