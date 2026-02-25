use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Top-level application configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub matrix: MatrixConfig,
    pub ui: UiConfig,
    pub behavior: BehaviorConfig,
}

/// Matrix homeserver connection settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub homeserver: String,
    pub username: String,
}

/// UI display settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub room_list_width: u16,
    pub timestamp_format: String,
    pub show_help_bar: bool,
}

/// Behavioral settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub vim_mode: bool,
    pub send_read_receipts: bool,
}

/// Theme color palette.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub colors: ThemeColors,
}

/// Individual theme color fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeColors {
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub my_message: String,
    pub their_message: String,
    pub timestamp: String,
    pub unread_badge: String,
    pub border: String,
    pub selected_room: String,
    pub status_bar_bg: String,
    pub help_bar_fg: String,
}

/// Keymap configuration for different modes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeymapConfig {
    pub normal: HashMap<String, String>,
    pub insert: HashMap<String, String>,
    #[serde(default)]
    pub message_select: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Default implementations
// ---------------------------------------------------------------------------

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            matrix: MatrixConfig::default(),
            ui: UiConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }
}

impl Default for MatrixConfig {
    fn default() -> Self {
        Self {
            homeserver: "http://localhost:6167".to_string(),
            username: String::new(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            room_list_width: 30,
            timestamp_format: "%H:%M".to_string(),
            show_help_bar: true,
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            vim_mode: true,
            send_read_receipts: true,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            colors: ThemeColors::default(),
        }
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            bg: "#1e1e2e".to_string(),
            fg: "#cdd6f4".to_string(),
            accent: "#89b4fa".to_string(),
            my_message: "#a6e3a1".to_string(),
            their_message: "#cdd6f4".to_string(),
            timestamp: "#6c7086".to_string(),
            unread_badge: "#f38ba8".to_string(),
            border: "#45475a".to_string(),
            selected_room: "#313244".to_string(),
            status_bar_bg: "#181825".to_string(),
            help_bar_fg: "#6c7086".to_string(),
        }
    }
}

impl Default for KeymapConfig {
    fn default() -> Self {
        let mut normal = HashMap::new();
        normal.insert("q".to_string(), "quit".to_string());
        normal.insert("j".to_string(), "room_next".to_string());
        normal.insert("k".to_string(), "room_prev".to_string());
        normal.insert("Up".to_string(), "room_prev".to_string());
        normal.insert("Down".to_string(), "room_next".to_string());
        normal.insert("i".to_string(), "insert_mode".to_string());
        normal.insert(":".to_string(), "command_mode".to_string());
        normal.insert("G".to_string(), "room_last".to_string());
        normal.insert("/".to_string(), "room_filter".to_string());
        normal.insert("Enter".to_string(), "enter_message_select".to_string());
        normal.insert("r".to_string(), "mark_read".to_string());
        normal.insert("R".to_string(), "mark_all_read".to_string());
        normal.insert("Ctrl+u".to_string(), "scroll_up".to_string());
        normal.insert("Ctrl+d".to_string(), "scroll_down".to_string());

        let mut insert = HashMap::new();
        insert.insert("Esc".to_string(), "normal_mode".to_string());
        insert.insert("Enter".to_string(), "send_message".to_string());

        let mut message_select = HashMap::new();
        message_select.insert("j".to_string(), "message_next".to_string());
        message_select.insert("k".to_string(), "message_prev".to_string());
        message_select.insert("Up".to_string(), "message_prev".to_string());
        message_select.insert("Down".to_string(), "message_next".to_string());
        message_select.insert("r".to_string(), "reply_to".to_string());
        message_select.insert("d".to_string(), "delete_message".to_string());
        message_select.insert("i".to_string(), "mode_insert".to_string());
        message_select.insert("q".to_string(), "quit".to_string());
        message_select.insert("Esc".to_string(), "mode_normal".to_string());

        Self { normal, insert, message_select }
    }
}

// ---------------------------------------------------------------------------
// XDG path helpers
// ---------------------------------------------------------------------------

/// Returns the XDG configuration directory for Zap (~/.config/zap).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("zap")
}

/// Returns the XDG data directory for Zap (~/.local/share/zap).
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join("zap")
}

/// Returns the log directory for Zap (inside the data directory).
pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

// ---------------------------------------------------------------------------
// Parse and serialize helpers
// ---------------------------------------------------------------------------

/// Parse an `AppConfig` from a TOML string.
pub fn parse_config(toml_str: &str) -> Result<AppConfig, toml::de::Error> {
    toml::from_str(toml_str)
}

/// Parse a `ThemeConfig` from a TOML string.
pub fn parse_theme(toml_str: &str) -> Result<ThemeConfig, toml::de::Error> {
    toml::from_str(toml_str)
}

/// Parse a `KeymapConfig` from a TOML string.
pub fn parse_keymap(toml_str: &str) -> Result<KeymapConfig, toml::de::Error> {
    toml::from_str(toml_str)
}

/// Serialize an `AppConfig` to a TOML string.
pub fn serialize_config(config: &AppConfig) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(config)
}

/// Serialize a `ThemeConfig` to a TOML string.
pub fn serialize_theme(theme: &ThemeConfig) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(theme)
}

/// Serialize a `KeymapConfig` to a TOML string.
pub fn serialize_keymap(keymap: &KeymapConfig) -> Result<String, toml::ser::Error> {
    toml::to_string_pretty(keymap)
}

// ---------------------------------------------------------------------------
// File management
// ---------------------------------------------------------------------------

/// Ensure the default configuration files exist on disk.
/// Creates directories and writes default TOML files if they are absent.
pub fn ensure_config_files() -> std::io::Result<()> {
    let cfg_dir = config_dir();
    fs::create_dir_all(&cfg_dir)?;
    fs::create_dir_all(data_dir())?;
    fs::create_dir_all(log_dir())?;

    let config_path = cfg_dir.join("config.toml");
    if !config_path.exists() {
        let default_config = serialize_config(&AppConfig::default())
            .expect("failed to serialize default config");
        fs::write(&config_path, default_config)?;
    }

    let theme_path = cfg_dir.join("theme.toml");
    if !theme_path.exists() {
        let default_theme = serialize_theme(&ThemeConfig::default())
            .expect("failed to serialize default theme");
        fs::write(&theme_path, default_theme)?;
    }

    let keymap_path = cfg_dir.join("keymap.toml");
    if !keymap_path.exists() {
        let default_keymap = serialize_keymap(&KeymapConfig::default())
            .expect("failed to serialize default keymap");
        fs::write(&keymap_path, default_keymap)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_app_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.matrix.homeserver, "http://localhost:6167");
        assert_eq!(cfg.matrix.username, "");
        assert_eq!(cfg.ui.theme, "default");
        assert_eq!(cfg.ui.room_list_width, 30);
        assert_eq!(cfg.ui.timestamp_format, "%H:%M");
        assert!(cfg.ui.show_help_bar);
        assert!(cfg.behavior.vim_mode);
        assert!(cfg.behavior.send_read_receipts);
    }

    #[test]
    fn test_default_theme_colors() {
        let theme = ThemeConfig::default();
        assert_eq!(theme.colors.bg, "#1e1e2e");
        assert_eq!(theme.colors.fg, "#cdd6f4");
        assert_eq!(theme.colors.accent, "#89b4fa");
        assert_eq!(theme.colors.my_message, "#a6e3a1");
        assert_eq!(theme.colors.their_message, "#cdd6f4");
        assert_eq!(theme.colors.timestamp, "#6c7086");
        assert_eq!(theme.colors.unread_badge, "#f38ba8");
        assert_eq!(theme.colors.border, "#45475a");
        assert_eq!(theme.colors.selected_room, "#313244");
        assert_eq!(theme.colors.status_bar_bg, "#181825");
        assert_eq!(theme.colors.help_bar_fg, "#6c7086");
    }

    #[test]
    fn test_default_keymap() {
        let km = KeymapConfig::default();
        assert_eq!(km.normal.get("q").unwrap(), "quit");
        assert_eq!(km.normal.get("j").unwrap(), "room_next");
        assert_eq!(km.insert.get("Esc").unwrap(), "normal_mode");
        assert_eq!(km.insert.get("Enter").unwrap(), "send_message");
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[matrix]
homeserver = "https://matrix.example.com"
username = "@alice:example.com"

[ui]
theme = "dark"
room_list_width = 40
timestamp_format = "%Y-%m-%d %H:%M"
show_help_bar = false

[behavior]
vim_mode = false
send_read_receipts = false
"#;
        let cfg = parse_config(toml_str).unwrap();
        assert_eq!(cfg.matrix.homeserver, "https://matrix.example.com");
        assert_eq!(cfg.matrix.username, "@alice:example.com");
        assert_eq!(cfg.ui.theme, "dark");
        assert_eq!(cfg.ui.room_list_width, 40);
        assert!(!cfg.ui.show_help_bar);
        assert!(!cfg.behavior.vim_mode);
        assert!(!cfg.behavior.send_read_receipts);
    }

    #[test]
    fn test_parse_theme_toml() {
        let toml_str = r##"
[colors]
bg = "#000000"
fg = "#ffffff"
accent = "#ff0000"
my_message = "#00ff00"
their_message = "#0000ff"
timestamp = "#888888"
unread_badge = "#ff00ff"
border = "#444444"
selected_room = "#222222"
status_bar_bg = "#111111"
help_bar_fg = "#999999"
"##;
        let theme = parse_theme(toml_str).unwrap();
        assert_eq!(theme.colors.bg, "#000000");
        assert_eq!(theme.colors.fg, "#ffffff");
        assert_eq!(theme.colors.help_bar_fg, "#999999");
    }

    #[test]
    fn test_round_trip_config() {
        let original = AppConfig::default();
        let serialized = serialize_config(&original).unwrap();
        let deserialized = parse_config(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_round_trip_theme() {
        let original = ThemeConfig::default();
        let serialized = serialize_theme(&original).unwrap();
        let deserialized = parse_theme(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_round_trip_keymap() {
        let original = KeymapConfig::default();
        let serialized = serialize_keymap(&original).unwrap();
        let deserialized = parse_keymap(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_xdg_config_dir() {
        let path = config_dir();
        assert!(path.ends_with("zap"));
    }

    #[test]
    fn test_xdg_data_dir() {
        let path = data_dir();
        assert!(path.ends_with("zap"));
    }

    #[test]
    fn test_xdg_log_dir() {
        let path = log_dir();
        assert!(path.ends_with("logs"));
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = parse_config("this is not valid toml {{{{");
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_config_files() {
        // Use a temporary directory to avoid polluting the real config.
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join("config.toml");
        let theme = tmp.path().join("theme.toml");
        let keymap = tmp.path().join("keymap.toml");

        // Write defaults manually to the temp dir to verify serialization works.
        let default_config = serialize_config(&AppConfig::default()).unwrap();
        fs::write(&cfg, &default_config).unwrap();
        let default_theme = serialize_theme(&ThemeConfig::default()).unwrap();
        fs::write(&theme, &default_theme).unwrap();
        let default_keymap = serialize_keymap(&KeymapConfig::default()).unwrap();
        fs::write(&keymap, &default_keymap).unwrap();

        // Verify all files exist and are parseable.
        assert!(cfg.exists());
        assert!(theme.exists());
        assert!(keymap.exists());

        let parsed = parse_config(&fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(parsed, AppConfig::default());
    }
}
