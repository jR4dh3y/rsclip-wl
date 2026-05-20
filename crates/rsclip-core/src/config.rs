use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;

#[derive(Clone, Debug)]
pub struct RsclipPaths {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub image_dir: PathBuf,
    pub thumb_dir: PathBuf,
    pub ocr_dir: PathBuf,
    pub log_path: PathBuf,
    pub socket_path: PathBuf,
}

impl RsclipPaths {
    pub fn discover() -> Result<Self> {
        let project = ProjectDirs::from("", "", "rsclip")
            .context("could not resolve XDG directories for rsclip")?;
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);

        let config_dir = project.config_dir().to_path_buf();
        let state_dir = project
            .state_dir()
            .unwrap_or(project.data_local_dir())
            .to_path_buf();
        let data_dir = project.data_dir().to_path_buf();
        let image_dir = data_dir.join("images");
        let thumb_dir = data_dir.join("thumbs");
        let ocr_dir = data_dir.join("ocr");

        Ok(Self {
            db_path: state_dir.join("rsclip.db"),
            log_path: state_dir.join("rsclip.log"),
            socket_path: runtime_dir.join("rsclip.sock"),
            config_dir,
            state_dir,
            data_dir,
            image_dir,
            thumb_dir,
            ocr_dir,
        })
    }

    pub fn ensure(&self) -> Result<()> {
        for dir in [
            &self.config_dir,
            &self.state_dir,
            &self.data_dir,
            &self.image_dir,
            &self.thumb_dir,
            &self.ocr_dir,
        ] {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        Ok(())
    }
}
