use std::process::Command;

use anyhow::{bail, Context, Result};

pub fn run_tesseract(image_path: &str, language: &str) -> Result<String> {
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")
        .arg("-l")
        .arg(language)
        .output()
        .context("spawning tesseract")?;
    if !output.status.success() {
        bail!(
            "tesseract failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
