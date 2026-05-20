use anyhow::Result;

use crate::colors::{parse_color, rgb_text};
use crate::links::detect_single_url;
use crate::mime::kind_from_mime;
use crate::models::{EntryKind, NewEntry};

pub fn classify_payload(mime_type: &str, content_hash: String, payload: &[u8]) -> Result<NewEntry> {
    let size_bytes = i64::try_from(payload.len()).unwrap_or(i64::MAX);
    if mime_type.starts_with("text/") {
        let text = String::from_utf8_lossy(payload).to_string();
        Ok(classify_text(mime_type, content_hash, text, size_bytes))
    } else {
        let kind = kind_from_mime(mime_type);
        Ok(NewEntry {
            content_hash,
            kind,
            mime_type: mime_type.to_string(),
            title: title_for_binary(mime_type, size_bytes),
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
            size_bytes,
        })
    }
}

pub fn classify_text(
    mime_type: &str,
    content_hash: String,
    text: String,
    size_bytes: i64,
) -> NewEntry {
    let trimmed = text.trim();

    if let Some(color) = parse_color(trimmed) {
        return NewEntry {
            content_hash,
            kind: EntryKind::Color,
            mime_type: mime_type.to_string(),
            title: color.normalized_hex.clone(),
            preview_text: Some(format!("{}  {}", color.normalized_hex, rgb_text(color.rgb))),
            text_content: Some(text),
            file_path: None,
            thumb_path: None,
            source_app: None,
            link_url: None,
            link_domain: None,
            link_icon: None,
            color_value: Some(color.normalized_hex),
            color_format: Some(color.original_format),
            size_bytes,
        };
    }

    if let Some(link) = detect_single_url(trimmed) {
        return NewEntry {
            content_hash,
            kind: EntryKind::Link,
            mime_type: mime_type.to_string(),
            title: link.domain.clone(),
            preview_text: Some(link.url.clone()),
            text_content: Some(text),
            file_path: None,
            thumb_path: None,
            source_app: None,
            link_url: Some(link.url),
            link_domain: Some(link.domain),
            link_icon: Some(link.icon),
            color_value: None,
            color_format: None,
            size_bytes,
        };
    }

    NewEntry {
        content_hash,
        kind: EntryKind::Text,
        mime_type: mime_type.to_string(),
        title: first_line_title(trimmed),
        preview_text: Some(preview_text(trimmed)),
        text_content: Some(text),
        file_path: None,
        thumb_path: None,
        source_app: None,
        link_url: None,
        link_domain: None,
        link_icon: None,
        color_value: None,
        color_format: None,
        size_bytes,
    }
}

fn first_line_title(text: &str) -> String {
    let title = text.lines().next().unwrap_or("").trim();
    let title = if title.is_empty() {
        "Untitled text"
    } else {
        title
    };
    truncate(title, 96)
}

fn preview_text(text: &str) -> String {
    truncate(&text.split_whitespace().collect::<Vec<_>>().join(" "), 320)
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        format!(
            "{}...",
            value
                .chars()
                .take(max_chars.saturating_sub(3))
                .collect::<String>()
        )
    }
}

fn title_for_binary(mime_type: &str, size_bytes: i64) -> String {
    if mime_type.starts_with("image/") {
        format!("Image ({})", human_bytes(size_bytes))
    } else {
        format!("{mime_type} ({})", human_bytes(size_bytes))
    }
}

fn human_bytes(bytes: i64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_link() {
        let entry =
            classify_payload("text/plain", "hash".to_string(), b"https://youtu.be/abc").unwrap();
        assert_eq!(entry.kind, EntryKind::Link);
        assert_eq!(entry.link_icon.as_deref(), Some("youtube"));
    }

    #[test]
    fn classifies_color() {
        let entry = classify_payload("text/plain", "hash".to_string(), b"#c59edc").unwrap();
        assert_eq!(entry.kind, EntryKind::Color);
        assert_eq!(entry.color_value.as_deref(), Some("#c59edc"));
    }
}
