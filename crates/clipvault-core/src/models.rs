use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EntryKind {
    Text,
    Image,
    Link,
    Color,
    File,
    Unknown,
}

impl EntryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Link => "link",
            Self::Color => "color",
            Self::File => "file",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EntryKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "text" => Self::Text,
            "image" => Self::Image,
            "link" => Self::Link,
            "color" => Self::Color,
            "file" => Self::File,
            _ => Self::Unknown,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content_hash: String,
    pub kind: EntryKind,
    pub mime_type: String,
    pub title: String,
    pub preview_text: Option<String>,
    pub text_content: Option<String>,
    pub file_path: Option<String>,
    pub thumb_path: Option<String>,
    pub source_app: Option<String>,
    pub link_url: Option<String>,
    pub link_domain: Option<String>,
    pub link_icon: Option<String>,
    pub color_value: Option<String>,
    pub color_format: Option<String>,
    pub pinned: bool,
    pub favorite: bool,
    pub copied_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
    pub use_count: i64,
    pub size_bytes: i64,
    pub ocr_text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewEntry {
    pub content_hash: String,
    pub kind: EntryKind,
    pub mime_type: String,
    pub title: String,
    pub preview_text: Option<String>,
    pub text_content: Option<String>,
    pub file_path: Option<String>,
    pub thumb_path: Option<String>,
    pub source_app: Option<String>,
    pub link_url: Option<String>,
    pub link_domain: Option<String>,
    pub link_icon: Option<String>,
    pub color_value: Option<String>,
    pub color_format: Option<String>,
    pub size_bytes: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryFilter {
    All,
    Text,
    Images,
    Links,
    Colors,
    Pinned,
}

impl EntryFilter {
    pub fn parse(value: &str) -> Self {
        match value {
            "text" => Self::Text,
            "images" | "image" => Self::Images,
            "links" | "link" => Self::Links,
            "colors" | "color" => Self::Colors,
            "pinned" => Self::Pinned,
            _ => Self::All,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortMode {
    Default,
    Recent,
    Oldest,
    Type,
    MostUsed,
}

impl SortMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "recent" => Self::Recent,
            "oldest" => Self::Oldest,
            "type" => Self::Type,
            "most-used" => Self::MostUsed,
            _ => Self::Default,
        }
    }
}
