use url::Url;

#[derive(Clone, Debug)]
pub struct LinkInfo {
    pub url: String,
    pub domain: String,
    pub icon: String,
}

pub fn detect_single_url(text: &str) -> Option<LinkInfo> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.split_whitespace().count() != 1 {
        return None;
    }

    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else if trimmed.contains('.') {
        format!("https://{trimmed}")
    } else {
        return None;
    };

    let url = Url::parse(&with_scheme).ok()?;
    let domain = url
        .host_str()?
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    Some(LinkInfo {
        url: url.to_string(),
        icon: icon_for_domain(&domain).to_string(),
        domain,
    })
}

pub fn icon_for_domain(domain: &str) -> &'static str {
    match domain {
        "github.com" => "github",
        "gitlab.com" => "gitlab",
        "linkedin.com" => "linkedin",
        "youtube.com" | "youtu.be" => "youtube",
        "x.com" | "twitter.com" => "x-twitter",
        "reddit.com" => "reddit",
        "instagram.com" => "instagram",
        "facebook.com" => "facebook",
        "discord.com" | "discord.gg" => "discord",
        "stackoverflow.com" => "stackoverflow",
        "docs.rs" | "crates.io" => "rust",
        "npmjs.com" => "npm",
        "pypi.org" => "python",
        "archlinux.org" | "wiki.archlinux.org" => "arch",
        _ => "globe",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_known_url() {
        let link = detect_single_url("https://github.com/owner/repo").unwrap();
        assert_eq!(link.domain, "github.com");
        assert_eq!(link.icon, "github");
    }

    #[test]
    fn ignores_multi_word_text() {
        assert!(detect_single_url("open https://github.com now").is_none());
    }
}
