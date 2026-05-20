use crate::models::{ClipboardEntry, EntryData};

pub fn secret_value_from_entry(entry: &ClipboardEntry) -> Option<String> {
    match &entry.data {
        EntryData::Image { ocr_text, .. } => ocr_text
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
        EntryData::Color { .. } | EntryData::File { .. } => None,
        EntryData::Link { url, .. } => Some(url.clone()),
        EntryData::Text | EntryData::Unknown => entry
            .text_content
            .as_deref()
            .or(entry.preview_text.as_deref())
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
    }
}

pub fn default_secret_alias(entry: &ClipboardEntry) -> String {
    if matches!(
        &entry.data,
        EntryData::Image {
            ocr_text: Some(text),
            ..
        } if !text.trim().is_empty()
    ) {
        "OCR text".to_string()
    } else if entry.title.trim().is_empty() {
        "Untitled secret".to_string()
    } else {
        entry.title.clone()
    }
}

pub fn normalize_secret_alias(alias: &str) -> &str {
    let normalized = alias.trim();
    if normalized.is_empty() {
        "Untitled secret"
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{ClipboardEntry, EntryData, EntryKind};

    use super::{default_secret_alias, normalize_secret_alias, secret_value_from_entry};

    fn make_entry(kind: EntryKind) -> ClipboardEntry {
        let data = match kind {
            EntryKind::Text => EntryData::Text,
            EntryKind::Image => EntryData::Image {
                file_path: String::new(),
                thumb_path: None,
                ocr_text: None,
            },
            EntryKind::Link => EntryData::Link {
                url: String::new(),
                domain: String::new(),
                icon: String::new(),
            },
            EntryKind::Color => EntryData::Color {
                value: String::new(),
                format: String::new(),
            },
            EntryKind::File => EntryData::File { source_app: None },
            EntryKind::Unknown => EntryData::Unknown,
        };
        ClipboardEntry {
            id: 1,
            content_hash: "hash".to_string(),
            kind,
            mime_type: "text/plain".to_string(),
            title: "Title".to_string(),
            preview_text: None,
            text_content: None,
            pinned: false,
            favorite: false,
            copied_at: 0,
            updated_at: 0,
            last_used_at: None,
            use_count: 0,
            size_bytes: 0,
            data,
        }
    }

    #[test]
    fn extracts_text_like_secret_values() {
        let mut text = make_entry(EntryKind::Text);
        text.text_content = Some("secret".to_string());
        assert_eq!(secret_value_from_entry(&text).as_deref(), Some("secret"));

        let mut link = make_entry(EntryKind::Link);
        link.data = EntryData::Link {
            url: "https://example.com".to_string(),
            domain: "example.com".to_string(),
            icon: "globe".to_string(),
        };
        assert_eq!(
            secret_value_from_entry(&link).as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn extracts_image_ocr_but_rejects_non_text_entries() {
        let mut image = make_entry(EntryKind::Image);
        assert_eq!(secret_value_from_entry(&image), None);
        image.data = EntryData::Image {
            file_path: String::new(),
            thumb_path: None,
            ocr_text: Some("image text".to_string()),
        };
        assert_eq!(
            secret_value_from_entry(&image).as_deref(),
            Some("image text")
        );

        assert_eq!(secret_value_from_entry(&make_entry(EntryKind::Color)), None);
        assert_eq!(secret_value_from_entry(&make_entry(EntryKind::File)), None);
    }

    #[test]
    fn chooses_default_secret_alias() {
        let mut image = make_entry(EntryKind::Image);
        image.data = EntryData::Image {
            file_path: String::new(),
            thumb_path: None,
            ocr_text: Some("image text".to_string()),
        };
        assert_eq!(default_secret_alias(&image), "OCR text");

        let mut untitled = make_entry(EntryKind::Text);
        untitled.title = "  ".to_string();
        assert_eq!(default_secret_alias(&untitled), "Untitled secret");
    }

    #[test]
    fn normalizes_secret_alias() {
        assert_eq!(normalize_secret_alias("  Named  "), "Named");
        assert_eq!(normalize_secret_alias("  "), "Untitled secret");
    }
}
