use gtk::gdk;
use gtk4 as gtk;
use rsclip_core::{AppConfig, UiColors};

pub(crate) fn load_css(config: &AppConfig) -> anyhow::Result<()> {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(&build_css(config)?);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    Ok(())
}

pub(crate) fn build_css(config: &AppConfig) -> anyhow::Result<String> {
    let mut css = String::new();
    for color in theme_colors() {
        let value = (color.configured)(&config.ui.colors).unwrap_or(color.default_value);
        let value = value.trim();
        validate_color(color.name, value)?;
        css.push_str("@define-color ");
        css.push_str(color.name);
        css.push(' ');
        css.push_str(value);
        css.push_str(";\n");
    }
    css.push('\n');
    css.push_str(include_str!("../resources/css/rsclip.css"));
    Ok(css)
}

struct ThemeColor {
    name: &'static str,
    default_value: &'static str,
    configured: fn(&UiColors) -> Option<&str>,
}

fn theme_colors() -> &'static [ThemeColor] {
    &[
        ThemeColor {
            name: "shell_bg",
            default_value: "rgba(30, 30, 32, 0.70)",
            configured: |colors| colors.shell_bg.as_deref(),
        },
        ThemeColor {
            name: "shell_border",
            default_value: "rgba(220, 217, 231, 0.14)",
            configured: |colors| colors.shell_border.as_deref(),
        },
        ThemeColor {
            name: "surface",
            default_value: "#2a2a2c",
            configured: |colors| colors.surface.as_deref(),
        },
        ThemeColor {
            name: "surface_subtle",
            default_value: "rgba(42, 42, 44, 0.54)",
            configured: |colors| colors.surface_subtle.as_deref(),
        },
        ThemeColor {
            name: "surface_overlay",
            default_value: "#1e1e20",
            configured: |colors| colors.surface_overlay.as_deref(),
        },
        ThemeColor {
            name: "preview_bg",
            default_value: "rgba(30, 30, 32, 0.40)",
            configured: |colors| colors.preview_bg.as_deref(),
        },
        ThemeColor {
            name: "preview_text_bg",
            default_value: "rgba(12, 12, 14, 0.34)",
            configured: |colors| colors.preview_text_bg.as_deref(),
        },
        ThemeColor {
            name: "scrim_bg",
            default_value: "rgba(12, 12, 14, 0.42)",
            configured: |colors| colors.scrim_bg.as_deref(),
        },
        ThemeColor {
            name: "text",
            default_value: "#dcd9e7",
            configured: |colors| colors.text.as_deref(),
        },
        ThemeColor {
            name: "text_strong",
            default_value: "#f0eafd",
            configured: |colors| colors.text_strong.as_deref(),
        },
        ThemeColor {
            name: "text_muted",
            default_value: "#9c96ad",
            configured: |colors| colors.text_muted.as_deref(),
        },
        ThemeColor {
            name: "text_selected_muted",
            default_value: "#c8bed6",
            configured: |colors| colors.text_selected_muted.as_deref(),
        },
        ThemeColor {
            name: "border",
            default_value: "#4a4653",
            configured: |colors| colors.border.as_deref(),
        },
        ThemeColor {
            name: "border_subtle",
            default_value: "rgba(220, 217, 231, 0.08)",
            configured: |colors| colors.border_subtle.as_deref(),
        },
        ThemeColor {
            name: "border_preview",
            default_value: "rgba(220, 217, 231, 0.10)",
            configured: |colors| colors.border_preview.as_deref(),
        },
        ThemeColor {
            name: "border_dialog",
            default_value: "#5c516b",
            configured: |colors| colors.border_dialog.as_deref(),
        },
        ThemeColor {
            name: "hover_bg",
            default_value: "#3b304a",
            configured: |colors| colors.hover_bg.as_deref(),
        },
        ThemeColor {
            name: "selected_bg",
            default_value: "#453657",
            configured: |colors| colors.selected_bg.as_deref(),
        },
        ThemeColor {
            name: "accent",
            default_value: "#c3fb5b",
            configured: |colors| colors.accent.as_deref(),
        },
        ThemeColor {
            name: "accent_hover",
            default_value: "#d4ff76",
            configured: |colors| colors.accent_hover.as_deref(),
        },
        ThemeColor {
            name: "accent_text",
            default_value: "#11130f",
            configured: |colors| colors.accent_text.as_deref(),
        },
        ThemeColor {
            name: "destructive",
            default_value: "#6f2d38",
            configured: |colors| colors.destructive.as_deref(),
        },
        ThemeColor {
            name: "destructive_border",
            default_value: "#9a4350",
            configured: |colors| colors.destructive_border.as_deref(),
        },
        ThemeColor {
            name: "destructive_text",
            default_value: "#fff1f3",
            configured: |colors| colors.destructive_text.as_deref(),
        },
    ]
}

fn validate_color(name: &str, value: &str) -> anyhow::Result<()> {
    let trimmed = value.trim();
    if is_hex_color(trimmed) || parse_rgb_color(trimmed).is_some() {
        return Ok(());
    }

    anyhow::bail!(
        "invalid ui.colors.{name}: expected CSS color like #c3fb5b or rgba(30, 30, 32, 0.70)"
    )
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_rgb_color(value: &str) -> Option<()> {
    let (prefix, suffix) = if let Some(inner) = value.strip_prefix("rgb(") {
        ("rgb", inner)
    } else if let Some(inner) = value.strip_prefix("rgba(") {
        ("rgba", inner)
    } else {
        return None;
    };
    let inner = suffix.strip_suffix(')')?;
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
    match (prefix, parts.as_slice()) {
        ("rgb", [red, green, blue]) => {
            parse_rgb_channel(red)?;
            parse_rgb_channel(green)?;
            parse_rgb_channel(blue)?;
            Some(())
        }
        ("rgba", [red, green, blue, alpha]) => {
            parse_rgb_channel(red)?;
            parse_rgb_channel(green)?;
            parse_rgb_channel(blue)?;
            parse_alpha(alpha)?;
            Some(())
        }
        _ => None,
    }
}

fn parse_rgb_channel(value: &str) -> Option<u8> {
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    value.parse::<u8>().ok()
}

fn parse_alpha(value: &str) -> Option<()> {
    if value.is_empty() || value.starts_with('+') || value.starts_with('-') {
        return None;
    }
    let alpha = value.parse::<f32>().ok()?;
    (0.0..=1.0).contains(&alpha).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_generation_includes_every_color_definition() {
        let css = build_css(&AppConfig::default()).expect("default theme CSS should build");

        for color in theme_colors() {
            assert!(css.contains(&format!("@define-color {} ", color.name)));
        }
    }

    #[test]
    fn overridden_colors_appear_in_generated_css() {
        let mut config = AppConfig::default();
        config.ui.colors.accent = Some("#ff00aa".to_string());
        config.ui.colors.accent_text = Some("#000000".to_string());

        let css = build_css(&config).expect("CSS with valid overrides should build");

        assert!(css.contains("@define-color accent #ff00aa;"));
        assert!(css.contains("@define-color accent_text #000000;"));
    }

    #[test]
    fn missing_colors_fall_back_to_defaults() {
        let css = build_css(&AppConfig::default()).expect("default theme CSS should build");

        assert!(css.contains("@define-color text #dcd9e7;"));
        assert!(css.contains("@define-color shell_bg rgba(30, 30, 32, 0.70);"));
    }

    #[test]
    fn invalid_color_returns_offending_key() {
        let mut config = AppConfig::default();
        config.ui.colors.accent = Some("not-a-color".to_string());

        let err = build_css(&config).unwrap_err();

        assert!(format!("{err:#}").contains("ui.colors.accent"));
    }

    #[test]
    fn validates_supported_color_formats() {
        for value in [
            "#abc",
            "#aabbcc",
            "#aabbccdd",
            "rgb(195, 251, 91)",
            "rgba(30, 30, 32, 0.70)",
        ] {
            validate_color("accent", value).expect("supported color format should validate");
        }
    }
}
