use chrono::{DateTime, Local, Utc};

pub fn human_bytes(bytes: i64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn human_size(bytes: i64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_full_time(timestamp: i64) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%a %b %-d %H:%M:%S %Y")
                .to_string()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn relative_time(timestamp: i64) -> String {
    let seconds = (Utc::now().timestamp() - timestamp).max(0);
    if seconds < 60 {
        "now".to_string()
    } else if seconds < 3_600 {
        let minutes = seconds / 60;
        format!("{minutes} min")
    } else if seconds < 86_400 {
        let hours = seconds / 3_600;
        format!("{hours} hr")
    } else {
        let days = seconds / 86_400;
        format!("{days} day")
    }
}

pub fn masked_secret(value: &str) -> String {
    let visible_tail = value.chars().rev().take(4).collect::<Vec<_>>();
    let tail = visible_tail.into_iter().rev().collect::<String>();
    if tail.is_empty() {
        "********".to_string()
    } else {
        format!("********{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::{human_bytes, human_size, masked_secret};

    #[test]
    fn formats_human_bytes() {
        assert_eq!(human_bytes(12), "12 B");
        assert_eq!(human_bytes(2048), "2.0 KB");
        assert_eq!(human_bytes(1_572_864), "1.5 MB");
    }

    #[test]
    fn formats_display_sizes() {
        assert_eq!(human_size(12), "12 B");
        assert_eq!(human_size(2048), "2 KB");
        assert_eq!(human_size(1_572_864), "1.5 MB");
    }

    #[test]
    fn masks_secrets() {
        assert_eq!(masked_secret(""), "********");
        assert_eq!(masked_secret("abc"), "********abc");
        assert_eq!(masked_secret("secret-token"), "********oken");
    }
}
