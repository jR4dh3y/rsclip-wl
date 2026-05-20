use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::models::{ClipboardEntry, EntryFilter, EntryKind, NewEntry, SortMode};

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
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_entry(&self, entry: &NewEntry) -> Result<i64> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            r#"
            INSERT INTO entries (
              content_hash, kind, mime_type, title, preview_text, text_content,
              file_path, thumb_path, source_app, link_url, link_domain, link_icon,
              color_value, color_format, copied_at, updated_at, size_bytes, deleted
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, 0)
            ON CONFLICT(content_hash) DO UPDATE SET
              kind=excluded.kind,
              mime_type=excluded.mime_type,
              title=excluded.title,
              preview_text=excluded.preview_text,
              text_content=excluded.text_content,
              file_path=COALESCE(excluded.file_path, entries.file_path),
              thumb_path=COALESCE(excluded.thumb_path, entries.thumb_path),
              source_app=excluded.source_app,
              link_url=excluded.link_url,
              link_domain=excluded.link_domain,
              link_icon=excluded.link_icon,
              color_value=excluded.color_value,
              color_format=excluded.color_format,
              updated_at=excluded.updated_at,
              size_bytes=excluded.size_bytes,
              deleted=0
            "#,
            params![
                entry.content_hash,
                entry.kind.as_str(),
                entry.mime_type,
                entry.title,
                entry.preview_text,
                entry.text_content,
                entry.file_path,
                entry.thumb_path,
                entry.source_app,
                entry.link_url,
                entry.link_domain,
                entry.link_icon,
                entry.color_value,
                entry.color_format,
                now,
                now,
                entry.size_bytes,
            ],
        )?;
        let id = self.conn.query_row(
            "SELECT id FROM entries WHERE content_hash = ?1",
            params![entry.content_hash],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn list_entries(
        &self,
        query: &str,
        filter: EntryFilter,
        sort: SortMode,
        limit: usize,
    ) -> Result<Vec<ClipboardEntry>> {
        let mut sql = String::from(
            r#"
            SELECT
              e.id, e.content_hash, e.kind, e.mime_type, e.title, e.preview_text,
              e.text_content, e.file_path, e.thumb_path, e.source_app, e.link_url,
              e.link_domain, e.link_icon, e.color_value, e.color_format, e.pinned,
              e.favorite, e.copied_at, e.updated_at, e.last_used_at, e.use_count,
              e.size_bytes, o.text
            FROM entries e
            LEFT JOIN ocr_results o ON o.entry_id = e.id
            WHERE e.deleted = 0
            "#,
        );

        let filter_clause = match filter {
            EntryFilter::All => "",
            EntryFilter::Text => " AND e.kind = 'text'",
            EntryFilter::Images => " AND e.kind = 'image'",
            EntryFilter::Links => " AND e.kind = 'link'",
            EntryFilter::Colors => " AND e.kind = 'color'",
            EntryFilter::Pinned => " AND e.pinned = 1",
        };
        sql.push_str(filter_clause);

        let has_query = !query.trim().is_empty();
        if has_query {
            sql.push_str(
                r#"
                AND (
                  e.title LIKE ?1 OR e.preview_text LIKE ?1 OR e.text_content LIKE ?1
                  OR e.link_url LIKE ?1 OR e.link_domain LIKE ?1 OR e.color_value LIKE ?1
                  OR o.text LIKE ?1
                )
                "#,
            );
        }

        sql.push_str(match sort {
            SortMode::Default => " ORDER BY e.pinned DESC, e.updated_at DESC",
            SortMode::Recent => " ORDER BY e.updated_at DESC",
            SortMode::Oldest => " ORDER BY e.updated_at ASC",
            SortMode::Type => " ORDER BY e.kind ASC, e.updated_at DESC",
            SortMode::MostUsed => " ORDER BY e.use_count DESC, e.updated_at DESC",
        });
        sql.push_str(" LIMIT ");
        sql.push_str(&limit.min(1000).to_string());

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = if has_query {
            let pattern = format!("%{}%", query.trim());
            stmt.query_map(params![pattern], entry_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map([], entry_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
    }

    pub fn get_entry(&self, id: i64) -> Result<Option<ClipboardEntry>> {
        self.conn
            .query_row(
                r#"
                SELECT
                  e.id, e.content_hash, e.kind, e.mime_type, e.title, e.preview_text,
                  e.text_content, e.file_path, e.thumb_path, e.source_app, e.link_url,
                  e.link_domain, e.link_icon, e.color_value, e.color_format, e.pinned,
                  e.favorite, e.copied_at, e.updated_at, e.last_used_at, e.use_count,
                  e.size_bytes, o.text
                FROM entries e
                LEFT JOIN ocr_results o ON o.entry_id = e.id
                WHERE e.id = ?1 AND e.deleted = 0
                "#,
                params![id],
                entry_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE entries SET pinned = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, pinned, Utc::now().timestamp()],
        )?;
        Ok(())
    }

    pub fn delete_entry(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE entries SET deleted = 1, updated_at = ?2 WHERE id = ?1",
            params![id, Utc::now().timestamp()],
        )?;
        Ok(())
    }

    pub fn touch_used(&self, id: i64) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            "UPDATE entries SET last_used_at = ?2, use_count = use_count + 1 WHERE id = ?1",
            params![id, now],
        )?;
        Ok(())
    }

    pub fn save_ocr_result(&self, entry_id: i64, language: &str, text: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn.execute(
            r#"
            INSERT INTO ocr_results (entry_id, status, text, language, created_at, updated_at)
            VALUES (?1, 'done', ?2, ?3, ?4, ?5)
            ON CONFLICT(entry_id) DO UPDATE SET
              status='done',
              text=excluded.text,
              language=excluded.language,
              updated_at=excluded.updated_at
            "#,
            params![entry_id, text, language, now, now],
        )?;
        Ok(())
    }
}

fn entry_from_row(row: &Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    let kind: String = row.get(2)?;
    Ok(ClipboardEntry {
        id: row.get(0)?,
        content_hash: row.get(1)?,
        kind: EntryKind::from_str(&kind).unwrap_or(EntryKind::Unknown),
        mime_type: row.get(3)?,
        title: row.get(4)?,
        preview_text: row.get(5)?,
        text_content: row.get(6)?,
        file_path: row.get(7)?,
        thumb_path: row.get(8)?,
        source_app: row.get(9)?,
        link_url: row.get(10)?,
        link_domain: row.get(11)?,
        link_icon: row.get(12)?,
        color_value: row.get(13)?,
        color_format: row.get(14)?,
        pinned: row.get::<_, i64>(15)? != 0,
        favorite: row.get::<_, i64>(16)? != 0,
        copied_at: row.get(17)?,
        updated_at: row.get(18)?,
        last_used_at: row.get(19)?,
        use_count: row.get(20)?,
        size_bytes: row.get(21)?,
        ocr_text: row.get(22)?,
    })
}
