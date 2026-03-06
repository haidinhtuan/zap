use ratatui::style::Color;

use crate::app::App;
use crate::config::ThemeColors;

pub fn color(
    app: &App,
    pick: fn(&ThemeColors) -> &str,
    fallback: Color,
) -> Color {
    app.theme
        .as_ref()
        .and_then(|theme| parse_hex_color(pick(&theme.colors)))
        .unwrap_or(fallback)
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color_valid() {
        assert_eq!(parse_hex_color("#112233"), Some(Color::Rgb(0x11, 0x22, 0x33)));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert!(parse_hex_color("not-a-color").is_none());
        assert!(parse_hex_color("#123").is_none());
    }
}
