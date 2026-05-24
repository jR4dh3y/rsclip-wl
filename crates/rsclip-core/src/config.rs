use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;

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

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default)]
    pub colors: UiColors,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            colors: UiColors::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct UiColors {
    pub shell_bg: Option<String>,
    pub shell_border: Option<String>,
    pub surface: Option<String>,
    pub surface_subtle: Option<String>,
    pub surface_overlay: Option<String>,
    pub preview_bg: Option<String>,
    pub preview_text_bg: Option<String>,
    pub scrim_bg: Option<String>,

    pub text: Option<String>,
    pub text_strong: Option<String>,
    pub text_muted: Option<String>,
    pub text_selected_muted: Option<String>,

    pub border: Option<String>,
    pub border_subtle: Option<String>,
    pub border_preview: Option<String>,
    pub border_dialog: Option<String>,

    pub hover_bg: Option<String>,
    pub selected_bg: Option<String>,

    pub accent: Option<String>,
    pub accent_hover: Option<String>,
    pub accent_text: Option<String>,

    pub destructive: Option<String>,
    pub destructive_border: Option<String>,
    pub destructive_text: Option<String>,
}

fn default_theme() -> String {
    "nonchalant-dark".to_string()
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

    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }
}

impl AppConfig {
    pub fn load(paths: &RsclipPaths) -> Result<Self> {
        let path = paths.config_path();
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
        };

        toml::from_str(&contents).with_context(|| format!("parsing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_paths(name: &str) -> RsclipPaths {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rsclip-config-test-{name}-{}-{unique}",
            std::process::id()
        ));
        RsclipPaths {
            config_dir: root.join("config"),
            state_dir: root.join("state"),
            data_dir: root.join("data"),
            db_path: root.join("state").join("rsclip.db"),
            image_dir: root.join("data").join("images"),
            thumb_dir: root.join("data").join("thumbs"),
            ocr_dir: root.join("data").join("ocr"),
            log_path: root.join("state").join("rsclip.log"),
            socket_path: root.join("rsclip.sock"),
        }
    }

    #[test]
    fn missing_config_file_returns_defaults() {
        let config = AppConfig::load(&test_paths("missing"))
            .expect("missing config file should load defaults");

        assert_eq!(config.ui.theme, "nonchalant-dark");
        assert!(config.ui.colors.accent.is_none());
    }

    #[test]
    fn empty_config_file_returns_defaults() {
        let paths = test_paths("empty");
        fs::create_dir_all(&paths.config_dir).expect("test config dir should be created");
        fs::write(paths.config_path(), "").expect("empty config file should be written");

        let config = AppConfig::load(&paths).expect("empty config file should load defaults");

        assert_eq!(config.ui.theme, "nonchalant-dark");
        assert!(config.ui.colors.text.is_none());
    }

    #[test]
    fn partial_colors_only_override_provided_fields() {
        let paths = test_paths("partial");
        fs::create_dir_all(&paths.config_dir).expect("test config dir should be created");
        fs::write(
            paths.config_path(),
            r##"
[ui.colors]
accent = "#ff00aa"
accent_text = "#000000"
"##,
        )
        .expect("partial config file should be written");

        let config = AppConfig::load(&paths).expect("partial config file should load");

        assert_eq!(config.ui.colors.accent.as_deref(), Some("#ff00aa"));
        assert_eq!(config.ui.colors.accent_text.as_deref(), Some("#000000"));
        assert!(config.ui.colors.text.is_none());
    }

    #[test]
    fn invalid_toml_returns_error_with_path_context() {
        let paths = test_paths("invalid");
        fs::create_dir_all(&paths.config_dir).expect("test config dir should be created");
        let path = paths.config_path();
        fs::write(&path, "[ui").expect("invalid config fixture should be written");

        let err = AppConfig::load(&paths).unwrap_err();

        assert!(format!("{err:#}").contains(&path.display().to_string()));
    }

    #[test]
    fn theme_defaults_to_nonchalant_dark() {
        assert_eq!(AppConfig::default().ui.theme, "nonchalant-dark");
    }
}
