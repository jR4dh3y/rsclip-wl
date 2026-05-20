use anyhow::Result;

use super::Database;

impl Database {
    pub fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS entries (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              content_hash TEXT NOT NULL,
              kind TEXT NOT NULL,
              mime_type TEXT NOT NULL,
              title TEXT NOT NULL,
              preview_text TEXT,
              text_content TEXT,
              file_path TEXT,
              thumb_path TEXT,
              source_app TEXT,
              link_url TEXT,
              link_domain TEXT,
              link_icon TEXT,
              color_value TEXT,
              color_format TEXT,
              pinned INTEGER NOT NULL DEFAULT 0,
              favorite INTEGER NOT NULL DEFAULT 0,
              copied_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              last_used_at INTEGER,
              use_count INTEGER NOT NULL DEFAULT 0,
              size_bytes INTEGER NOT NULL DEFAULT 0,
              deleted INTEGER NOT NULL DEFAULT 0
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_hash ON entries(content_hash);
            CREATE INDEX IF NOT EXISTS idx_entries_copied_at ON entries(copied_at DESC);
            CREATE INDEX IF NOT EXISTS idx_entries_pinned ON entries(pinned DESC, updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_entries_kind ON entries(kind);
            CREATE INDEX IF NOT EXISTS idx_entries_domain ON entries(link_domain);

            CREATE TABLE IF NOT EXISTS ocr_results (
              entry_id INTEGER PRIMARY KEY,
              status TEXT NOT NULL,
              text TEXT,
              language TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              FOREIGN KEY(entry_id) REFERENCES entries(id)
            );

            CREATE TABLE IF NOT EXISTS tags (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS entry_tags (
              entry_id INTEGER NOT NULL,
              tag_id INTEGER NOT NULL,
              PRIMARY KEY(entry_id, tag_id),
              FOREIGN KEY(entry_id) REFERENCES entries(id),
              FOREIGN KEY(tag_id) REFERENCES tags(id)
            );

            CREATE TABLE IF NOT EXISTS secrets (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              source_entry_id INTEGER UNIQUE,
              alias TEXT NOT NULL,
              value TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              last_used_at INTEGER,
              use_count INTEGER NOT NULL DEFAULT 0,
              deleted INTEGER NOT NULL DEFAULT 0,
              FOREIGN KEY(source_entry_id) REFERENCES entries(id)
            );

            CREATE INDEX IF NOT EXISTS idx_secrets_updated_at ON secrets(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_secrets_alias ON secrets(alias);
            "#,
        )?;
        Ok(())
    }
}
