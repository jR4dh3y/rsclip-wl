use anyhow::{Result, bail};
use rsclip_core::notify::notify_favicons_changed;
use rsclip_core::{RsclipPaths, favicons};

pub fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("clear") => clear(),
        Some(command) => bail!("unknown favicons command: {command}"),
        None => bail!("missing favicons command"),
    }
}

fn clear() -> Result<()> {
    let paths = RsclipPaths::discover()?;
    paths.ensure()?;
    clear_paths(&paths)?;
    notify_favicons_changed(&paths);
    println!("cleared favicon cache");
    Ok(())
}

fn clear_paths(paths: &RsclipPaths) -> Result<()> {
    favicons::clear_cache(&paths)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_paths(name: &str) -> RsclipPaths {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "rsclip-favicons-command-test-{name}-{}-{unique}",
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
            favicon_dir: root.join("data").join("favicons"),
            favicon_icon_dir: root.join("data").join("favicons").join("icons"),
            favicon_queue_dir: root.join("data").join("favicons").join("queue"),
            favicon_miss_dir: root.join("data").join("favicons").join("misses"),
            log_path: root.join("state").join("rsclip.log"),
            socket_path: root.join("rsclip.sock"),
        }
    }

    #[test]
    fn clear_paths_clears_temp_favicon_directory() {
        let paths = test_paths("clear");
        paths.ensure().unwrap();
        fs::write(paths.favicon_icon_dir.join("icon.png"), b"png").unwrap();
        fs::write(paths.favicon_miss_dir.join("domain.miss"), b"miss").unwrap();
        fs::write(paths.favicon_queue_dir.join("domain.json"), b"{}").unwrap();

        clear_paths(&paths).unwrap();

        assert!(
            fs::read_dir(&paths.favicon_icon_dir)
                .unwrap()
                .next()
                .is_none()
        );
        assert!(
            fs::read_dir(&paths.favicon_miss_dir)
                .unwrap()
                .next()
                .is_none()
        );
        assert!(
            fs::read_dir(&paths.favicon_queue_dir)
                .unwrap()
                .next()
                .is_none()
        );
    }
}
