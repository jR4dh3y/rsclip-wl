use crate::models::EntryKind;

pub fn kind_from_mime(mime: &str) -> EntryKind {
    if mime.starts_with("image/") {
        EntryKind::Image
    } else if mime == "text/uri-list" {
        EntryKind::File
    } else if mime.starts_with("text/") {
        EntryKind::Text
    } else {
        EntryKind::Unknown
    }
}

pub fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/png" => "png",
        _ => "bin",
    }
}
