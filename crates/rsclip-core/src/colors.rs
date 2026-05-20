use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Clone, Debug)]
pub struct ColorInfo {
    pub normalized_hex: String,
    pub original_format: String,
    pub rgb: (u8, u8, u8),
}

static HEX_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*(#|0x)?([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})\s*$").unwrap());
static RGB_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^\s*rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*(?:0|1|0?\.\d+))?\s*\)\s*$").unwrap()
});

pub fn parse_color(text: &str) -> Option<ColorInfo> {
    parse_hex(text)
        .or_else(|| parse_rgb(text))
        .or_else(|| parse_named(text))
}

fn parse_hex(text: &str) -> Option<ColorInfo> {
    let caps = HEX_RE.captures(text)?;
    let raw = caps.get(2)?.as_str();
    let hex = match raw.len() {
        3 => raw.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => raw.to_string(),
        8 => raw[0..6].to_string(),
        _ => return None,
    };
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(ColorInfo {
        normalized_hex: format!("#{hex}").to_ascii_lowercase(),
        original_format: "hex".to_string(),
        rgb: (red, green, blue),
    })
}

fn parse_rgb(text: &str) -> Option<ColorInfo> {
    let caps = RGB_RE.captures(text)?;
    let red = parse_channel(caps.get(1)?.as_str())?;
    let green = parse_channel(caps.get(2)?.as_str())?;
    let blue = parse_channel(caps.get(3)?.as_str())?;
    Some(ColorInfo {
        normalized_hex: format!("#{red:02x}{green:02x}{blue:02x}"),
        original_format: "rgb".to_string(),
        rgb: (red, green, blue),
    })
}

fn parse_channel(value: &str) -> Option<u8> {
    let parsed = value.parse::<u16>().ok()?;
    u8::try_from(parsed).ok()
}

fn parse_named(text: &str) -> Option<ColorInfo> {
    let (name, rgb) = match text.trim().to_ascii_lowercase().as_str() {
        "black" => ("black", (0, 0, 0)),
        "white" => ("white", (255, 255, 255)),
        "red" => ("red", (255, 0, 0)),
        "green" => ("green", (0, 128, 0)),
        "blue" => ("blue", (0, 0, 255)),
        "transparent" => ("transparent", (0, 0, 0)),
        "rebeccapurple" => ("rebeccapurple", (102, 51, 153)),
        _ => return None,
    };
    Some(ColorInfo {
        normalized_hex: format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2),
        original_format: name.to_string(),
        rgb,
    })
}

pub fn rgb_text((red, green, blue): (u8, u8, u8)) -> String {
    format!("rgb({red}, {green}, {blue})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_shorthand() {
        let color = parse_color("#f0a").unwrap();
        assert_eq!(color.normalized_hex, "#ff00aa");
        assert_eq!(color.rgb, (255, 0, 170));
    }

    #[test]
    fn parses_rgb() {
        let color = parse_color("rgb(195, 251, 91)").unwrap();
        assert_eq!(color.normalized_hex, "#c3fb5b");
    }
}
