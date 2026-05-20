use crate::models::{ClipboardEntry, EntryKind};

pub fn secret_value_from_entry(entry: &ClipboardEntry) -> Option<String> {
    match entry.kind {
        EntryKind::Image => entry
            .ocr_text
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
        EntryKind::Color | EntryKind::File => None,
        EntryKind::Link => entry
            .link_url
            .as_deref()
            .or(entry.text_content.as_deref())
            .or(entry.preview_text.as_deref())
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
        _ => entry
            .text_content
            .as_deref()
            .or(entry.preview_text.as_deref())
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
    }
}

pub fn default_secret_alias(entry: &ClipboardEntry) -> String {
    if matches!(entry.kind, EntryKind::Image)
        && entry
            .ocr_text
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
    {
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
    use crate::models::{ClipboardEntry, EntryKind};

    use super::{default_secret_alias, normalize_secret_alias, secret_value_from_entry};

    fn entry(kind: EntryKind) -> ClipboardEntry {
        ClipboardEntry {
            id: 1,
            content_hash: "hash".to_string(),
            kind,
            mime_type: "text/plain".to_string(),
            title: "Title".to_string(),
            preview_text: None,
            text_content: None,
            file_path: None,
            thumb_path: None,
            source_app: None,
            link_url: None,
            link_domain: None,
            link_icon: None,
            color_value: None,
            color_format: None,
            pinned: false,
            favorite: false,
            copied_at: 0,
            updated_at: 0,
            last_used_at: None,
            use_count: 0,
            size_bytes: 0,
            ocr_text: None,
        }
    }

    #[test]
    fn extracts_text_like_secret_values() {
        let mut text = entry(EntryKind::Text);
        text.text_content = Some("secret".to_string());
        assert_eq!(secret_value_from_entry(&text).as_deref(), Some("secret"));

        let mut link = entry(EntryKind::Link);
        link.link_url = Some("https://example.com".to_string());
        assert_eq!(
            secret_value_from_entry(&link).as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn extracts_image_ocr_but_rejects_non_text_entries() {
        let mut image = entry(EntryKind::Image);
        assert_eq!(secret_value_from_entry(&image), None);
        image.ocr_text = Some("image text".to_string());
        assert_eq!(
            secret_value_from_entry(&image).as_deref(),
            Some("image text")
        );

        assert_eq!(secret_value_from_entry(&entry(EntryKind::Color)), None);
        assert_eq!(secret_value_from_entry(&entry(EntryKind::File)), None);
    }

    #[test]
    fn chooses_default_secret_alias() {
        let mut image = entry(EntryKind::Image);
        image.ocr_text = Some("image text".to_string());
        assert_eq!(default_secret_alias(&image), "OCR text");

        let mut untitled = entry(EntryKind::Text);
        untitled.title = "  ".to_string();
        assert_eq!(default_secret_alias(&untitled), "Untitled secret");
    }

    #[test]
    fn normalizes_secret_alias() {
        assert_eq!(normalize_secret_alias("  Named  "), "Named");
        assert_eq!(normalize_secret_alias("  "), "Untitled secret");
    }
}
