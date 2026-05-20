use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tracing::{error, info, warn};

pub fn run_watchers() -> Result<()> {
    require_command("wl-paste")?;
    info!("starting rsclip clipboard watchers");
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
        if let Some(child) = self.child.as_mut()
            && let Err(err) = child.kill()
        {
            error!("failed to stop {} watcher: {err}", self.label);
        }
    }
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
