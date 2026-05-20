mod entries;
mod ocr;
mod rows;
mod schema;
mod secrets;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating database directory {}", parent.display()))?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::models::{EntryFilter, EntryKind, NewEntry, SortMode};

    use super::Database;

    fn temp_db_path() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rsclip-core-db-test-{}-{unique}.sqlite",
            std::process::id()
        ))
    }

    fn text_entry(hash: &str, title: &str) -> NewEntry {
        NewEntry {
            content_hash: hash.to_string(),
            kind: EntryKind::Text,
            mime_type: "text/plain".to_string(),
            title: title.to_string(),
            preview_text: Some(title.to_string()),
            text_content: Some(title.to_string()),
            file_path: None,
            thumb_path: None,
            source_app: None,
            link_url: None,
            link_domain: None,
            link_icon: None,
            color_value: None,
            color_format: None,
            size_bytes: title.len() as i64,
        }
    }

    #[test]
    fn database_entry_secret_and_ocr_smoke_test() {
        let path = temp_db_path();
        let db = Database::open(&path).unwrap();

        let entry_id = db
            .upsert_entry(&text_entry("hash-1", "secret text"))
            .unwrap();
        let entries = db
            .list_entries("", EntryFilter::All, SortMode::Default, 100)
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry_id);

        let entry = db.get_entry(entry_id).unwrap().unwrap();
        assert_eq!(entry.title, "secret text");

        db.save_ocr_result(entry_id, "eng", "ocr body").unwrap();
        let entry = db.get_entry(entry_id).unwrap().unwrap();
        assert_eq!(entry.ocr_text.as_deref(), Some("ocr body"));

        let secret_id = db
            .save_secret(Some(entry_id), "Alias", "secret value")
            .unwrap();
        let secrets = db.list_secrets("", 100).unwrap();
        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0].id, secret_id);

        db.rename_secret(secret_id, "Renamed").unwrap();
        let secret = db.list_secrets("Renamed", 100).unwrap().remove(0);
        assert_eq!(secret.alias, "Renamed");

        db.delete_secret(secret_id).unwrap();
        assert!(db.list_secrets("", 100).unwrap().is_empty());

        drop(db);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    }
}
