use std::str::FromStr;

use rusqlite::Row;

use crate::models::{ClipboardEntry, EntryData, EntryKind, SecretEntry};

pub(super) fn entry_from_row(row: &Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    let kind: String = row.get(2)?;
    let kind = EntryKind::from_str(&kind).unwrap_or(EntryKind::Unknown);

    let data = match kind {
        EntryKind::Text => EntryData::Text,
        EntryKind::Image => EntryData::Image {
            file_path: row.get(7)?,
            thumb_path: row.get(8)?,
            ocr_text: row.get(22)?,
        },
        EntryKind::Link => EntryData::Link {
            url: row.get(10)?,
            domain: row.get(11)?,
            icon: row.get(12)?,
        },
        EntryKind::Color => EntryData::Color {
            value: row.get(13)?,
            format: row.get(14)?,
        },
        EntryKind::File => EntryData::File {
            source_app: row.get(9)?,
        },
        EntryKind::Unknown => EntryData::Unknown,
    };

    Ok(ClipboardEntry {
        id: row.get(0)?,
        content_hash: row.get(1)?,
        kind,
        mime_type: row.get(3)?,
        title: row.get(4)?,
        preview_text: row.get(5)?,
        text_content: row.get(6)?,
        pinned: row.get::<_, i64>(15)? != 0,
        favorite: row.get::<_, i64>(16)? != 0,
        copied_at: row.get(17)?,
        updated_at: row.get(18)?,
        last_used_at: row.get(19)?,
        use_count: row.get(20)?,
        size_bytes: row.get(21)?,
        data,
    })
}

pub(super) fn secret_from_row(row: &Row<'_>) -> rusqlite::Result<SecretEntry> {
    Ok(SecretEntry {
        id: row.get(0)?,
        source_entry_id: row.get(1)?,
        alias: row.get(2)?,
        value: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        last_used_at: row.get(6)?,
        use_count: row.get(7)?,
    })
}
