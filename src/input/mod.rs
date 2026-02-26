use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::app::{Action, Mode};
use crate::config::KeymapConfig;

/// Manages keybindings for all application modes, including multi-key sequences.
pub struct KeymapManager {
    normal: HashMap<String, Action>,
    insert: HashMap<String, Action>,
    message_select: HashMap<String, Action>,
    pending_key: Option<(char, Instant)>,
    multi_key_timeout: Duration,
}

impl KeymapManager {
    /// Build a KeymapManager from a KeymapConfig.
    pub fn from_config(config: &KeymapConfig) -> Self {
        let mut normal = HashMap::new();
        for (key, action_str) in &config.normal {
            if let Some(action) = Self::parse_action(action_str) {
                normal.insert(key.clone(), action);
            }
        }

        let mut insert = HashMap::new();
        for (key, action_str) in &config.insert {
            if let Some(action) = Self::parse_action(action_str) {
                insert.insert(key.clone(), action);
            }
        }

        let mut message_select = HashMap::new();
        for (key, action_str) in &config.message_select {
            if let Some(action) = Self::parse_action(action_str) {
                message_select.insert(key.clone(), action);
            }
        }
        // Fall back to defaults if config section was empty.
        if message_select.is_empty() {
            let default = Self::default_keymap();
            message_select = default.message_select;
        }

        Self {
            normal,
            insert,
            message_select,
            pending_key: None,
            multi_key_timeout: Duration::from_millis(500),
        }
    }

    /// Create a KeymapManager with hardcoded default keybindings.
    pub fn default_keymap() -> Self {
        let mut normal = HashMap::new();
        normal.insert("q".to_string(), Action::Quit);
        normal.insert("j".to_string(), Action::RoomNext);
        normal.insert("k".to_string(), Action::RoomPrev);
        normal.insert("G".to_string(), Action::RoomLast);
        normal.insert("i".to_string(), Action::ModeInsert);
        normal.insert(":".to_string(), Action::ModeCommand);
        normal.insert("/".to_string(), Action::RoomFilter);
        normal.insert("Enter".to_string(), Action::EnterMessageSelect);
        normal.insert("r".to_string(), Action::MarkRead);
        normal.insert("R".to_string(), Action::MarkAllRead);
        normal.insert("Ctrl+u".to_string(), Action::ScrollUp);
        normal.insert("Ctrl+d".to_string(), Action::ScrollDown);

        let mut insert = HashMap::new();
        insert.insert("Esc".to_string(), Action::ModeNormal);
        insert.insert("Enter".to_string(), Action::SendMessage);

        let mut message_select = HashMap::new();
        message_select.insert("j".to_string(), Action::MessageNext);
        message_select.insert("k".to_string(), Action::MessagePrev);
        message_select.insert("Down".to_string(), Action::MessageNext);
        message_select.insert("Up".to_string(), Action::MessagePrev);
        message_select.insert("r".to_string(), Action::ReplyTo);
        message_select.insert("d".to_string(), Action::DeleteMessage);
        message_select.insert("e".to_string(), Action::EditMessage);
        message_select.insert("i".to_string(), Action::ModeInsert);
        message_select.insert("q".to_string(), Action::Quit);
        message_select.insert("Esc".to_string(), Action::ModeNormal);

        Self {
            normal,
            insert,
            message_select,
            pending_key: None,
            multi_key_timeout: Duration::from_millis(500),
        }
    }

    /// Resolve a key event into an action based on the current mode.
    ///
    /// Handles multi-key sequences: pressing 'g' starts a pending state,
    /// and a second 'g' within the timeout triggers RoomFirst.
    pub fn resolve(&mut self, key: KeyEvent, mode: &Mode) -> Option<Action> {
        match mode {
            Mode::Normal => self.resolve_normal(key),
            Mode::Insert => self.resolve_insert(key),
            Mode::MessageSelect => self.resolve_message_select(key),
            Mode::Command(_) => self.resolve_command(key),
            Mode::RoomFilter => None, // Handled directly in the event loop.
        }
    }

    fn resolve_normal(&mut self, key: KeyEvent) -> Option<Action> {
        let key_str = Self::key_event_to_string(&key);

        // Check for multi-key sequence completion.
        if let Some((pending_char, instant)) = self.pending_key.take() {
            if instant.elapsed() < self.multi_key_timeout {
                if pending_char == 'g' && key.code == KeyCode::Char('g') {
                    return Some(Action::RoomFirst);
                }
            }
            // Timeout expired or different key: fall through to normal lookup.
        }

        // Check for multi-key sequence start.
        if key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::NONE {
            // Only start pending if 'g' is not mapped to something else,
            // or if we want 'gg' to take priority.
            self.pending_key = Some(('g', Instant::now()));
            return None;
        }

        self.normal.get(&key_str).cloned()
    }

    fn resolve_insert(&mut self, key: KeyEvent) -> Option<Action> {
        let key_str = Self::key_event_to_string(&key);
        self.insert.get(&key_str).cloned()
    }

    fn resolve_command(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => Some(Action::ModeNormal),
            KeyCode::Enter => Some(Action::ModeNormal),
            _ => None,
        }
    }

    fn resolve_message_select(&mut self, key: KeyEvent) -> Option<Action> {
        let key_str = Self::key_event_to_string(&key);
        self.message_select.get(&key_str).cloned()
    }

    /// Parse an action name string into the corresponding Action enum variant.
    pub fn parse_action(s: &str) -> Option<Action> {
        match s {
            "quit" => Some(Action::Quit),
            "room_next" => Some(Action::RoomNext),
            "room_prev" => Some(Action::RoomPrev),
            "room_first" => Some(Action::RoomFirst),
            "room_last" => Some(Action::RoomLast),
            "mode_insert" | "insert_mode" => Some(Action::ModeInsert),
            "mode_normal" | "normal_mode" => Some(Action::ModeNormal),
            "mode_command" | "command_mode" => Some(Action::ModeCommand),
            "room_filter" => Some(Action::RoomFilter),
            "open_room" => Some(Action::OpenRoom),
            "scroll_up" | "scroll_up_half" => Some(Action::ScrollUp),
            "scroll_down" | "scroll_down_half" => Some(Action::ScrollDown),
            "send_message" => Some(Action::SendMessage),
            "mark_read" => Some(Action::MarkRead),
            "mark_all_read" => Some(Action::MarkAllRead),
            "enter_message_select" => Some(Action::EnterMessageSelect),
            "message_next" => Some(Action::MessageNext),
            "message_prev" => Some(Action::MessagePrev),
            "reply_to" => Some(Action::ReplyTo),
            "cancel_reply" => Some(Action::CancelReply),
            "delete_message" => Some(Action::DeleteMessage),
            "confirm_delete" => Some(Action::ConfirmDelete),
            "cancel_delete" => Some(Action::CancelDelete),
            "edit_message" => Some(Action::EditMessage),
            _ => None,
        }
    }

    /// Convert a KeyEvent into a human-readable config-style string.
    ///
    /// Examples: "q", "G", "Enter", "Esc", "Ctrl+u", "Ctrl+d"
    pub fn key_event_to_string(key: &KeyEvent) -> String {
        let mut parts = Vec::new();

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl".to_string());
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt".to_string());
        }
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            // For regular characters, the shift is implicit in the uppercase letter.
            // Only add "Shift" for special keys.
            match key.code {
                KeyCode::Char(_) => {}
                _ => parts.push("Shift".to_string()),
            }
        }

        let key_name = match key.code {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::F(n) => format!("F{}", n),
            _ => "Unknown".to_string(),
        };

        parts.push(key_name);
        parts.join("+")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_parse_action_quit() {
        assert_eq!(KeymapManager::parse_action("quit"), Some(Action::Quit));
    }

    #[test]
    fn test_parse_action_room_next() {
        assert_eq!(KeymapManager::parse_action("room_next"), Some(Action::RoomNext));
    }

    #[test]
    fn test_parse_action_room_prev() {
        assert_eq!(KeymapManager::parse_action("room_prev"), Some(Action::RoomPrev));
    }

    #[test]
    fn test_parse_action_room_first() {
        assert_eq!(KeymapManager::parse_action("room_first"), Some(Action::RoomFirst));
    }

    #[test]
    fn test_parse_action_room_last() {
        assert_eq!(KeymapManager::parse_action("room_last"), Some(Action::RoomLast));
    }

    #[test]
    fn test_parse_action_mode_insert() {
        assert_eq!(KeymapManager::parse_action("mode_insert"), Some(Action::ModeInsert));
        assert_eq!(KeymapManager::parse_action("insert_mode"), Some(Action::ModeInsert));
    }

    #[test]
    fn test_parse_action_mode_normal() {
        assert_eq!(KeymapManager::parse_action("mode_normal"), Some(Action::ModeNormal));
        assert_eq!(KeymapManager::parse_action("normal_mode"), Some(Action::ModeNormal));
    }

    #[test]
    fn test_parse_action_mode_command() {
        assert_eq!(KeymapManager::parse_action("mode_command"), Some(Action::ModeCommand));
        assert_eq!(KeymapManager::parse_action("command_mode"), Some(Action::ModeCommand));
    }

    #[test]
    fn test_parse_action_send_message() {
        assert_eq!(KeymapManager::parse_action("send_message"), Some(Action::SendMessage));
    }

    #[test]
    fn test_parse_action_scroll_up() {
        assert_eq!(KeymapManager::parse_action("scroll_up"), Some(Action::ScrollUp));
        assert_eq!(KeymapManager::parse_action("scroll_up_half"), Some(Action::ScrollUp));
    }

    #[test]
    fn test_parse_action_scroll_down() {
        assert_eq!(KeymapManager::parse_action("scroll_down"), Some(Action::ScrollDown));
        assert_eq!(KeymapManager::parse_action("scroll_down_half"), Some(Action::ScrollDown));
    }

    #[test]
    fn test_parse_action_mark_read() {
        assert_eq!(KeymapManager::parse_action("mark_read"), Some(Action::MarkRead));
    }

    #[test]
    fn test_parse_action_mark_all_read() {
        assert_eq!(KeymapManager::parse_action("mark_all_read"), Some(Action::MarkAllRead));
    }

    #[test]
    fn test_parse_action_unknown() {
        assert_eq!(KeymapManager::parse_action("nonexistent"), None);
    }

    #[test]
    fn test_resolve_normal_mode_quit() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Char('q')), &Mode::Normal);
        assert_eq!(result, Some(Action::Quit));
    }

    #[test]
    fn test_resolve_normal_mode_room_next() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Char('j')), &Mode::Normal);
        assert_eq!(result, Some(Action::RoomNext));
    }

    #[test]
    fn test_resolve_normal_mode_room_prev() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Char('k')), &Mode::Normal);
        assert_eq!(result, Some(Action::RoomPrev));
    }

    #[test]
    fn test_resolve_normal_mode_ctrl_u_scroll_up() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(
            key_with_mod(KeyCode::Char('u'), KeyModifiers::CONTROL),
            &Mode::Normal,
        );
        assert_eq!(result, Some(Action::ScrollUp));
    }

    #[test]
    fn test_resolve_insert_esc_to_normal() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Esc), &Mode::Insert);
        assert_eq!(result, Some(Action::ModeNormal));
    }

    #[test]
    fn test_resolve_insert_enter_sends() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Enter), &Mode::Insert);
        assert_eq!(result, Some(Action::SendMessage));
    }

    #[test]
    fn test_resolve_insert_unmapped_key() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Char('a')), &Mode::Insert);
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_normal_unmapped_key() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Char('z')), &Mode::Normal);
        assert_eq!(result, None);
    }

    #[test]
    fn test_multi_key_gg_returns_room_first() {
        let mut km = KeymapManager::default_keymap();
        // First 'g' starts pending state.
        let result1 = km.resolve(key(KeyCode::Char('g')), &Mode::Normal);
        assert_eq!(result1, None);
        // Second 'g' completes the sequence.
        let result2 = km.resolve(key(KeyCode::Char('g')), &Mode::Normal);
        assert_eq!(result2, Some(Action::RoomFirst));
    }

    #[test]
    fn test_multi_key_g_then_other_key() {
        let mut km = KeymapManager::default_keymap();
        // First 'g' starts pending state.
        let result1 = km.resolve(key(KeyCode::Char('g')), &Mode::Normal);
        assert_eq!(result1, None);
        // A different key cancels the pending and does normal lookup.
        let result2 = km.resolve(key(KeyCode::Char('j')), &Mode::Normal);
        assert_eq!(result2, Some(Action::RoomNext));
    }

    #[test]
    fn test_from_config_loads_keymap() {
        let config = KeymapConfig::default();
        let mut km = KeymapManager::from_config(&config);
        // Verify that the config keybindings are loaded correctly.
        let result = km.resolve(key(KeyCode::Char('q')), &Mode::Normal);
        assert_eq!(result, Some(Action::Quit));
        let result = km.resolve(key(KeyCode::Char('j')), &Mode::Normal);
        assert_eq!(result, Some(Action::RoomNext));
    }

    #[test]
    fn test_from_config_insert_keymap() {
        let config = KeymapConfig::default();
        let mut km = KeymapManager::from_config(&config);
        let result = km.resolve(key(KeyCode::Esc), &Mode::Insert);
        assert_eq!(result, Some(Action::ModeNormal));
        let result = km.resolve(key(KeyCode::Enter), &Mode::Insert);
        assert_eq!(result, Some(Action::SendMessage));
    }

    #[test]
    fn test_key_event_to_string_char() {
        let s = KeymapManager::key_event_to_string(&key(KeyCode::Char('q')));
        assert_eq!(s, "q");
    }

    #[test]
    fn test_key_event_to_string_uppercase() {
        let k = key_with_mod(KeyCode::Char('G'), KeyModifiers::SHIFT);
        let s = KeymapManager::key_event_to_string(&k);
        assert_eq!(s, "G");
    }

    #[test]
    fn test_key_event_to_string_enter() {
        let s = KeymapManager::key_event_to_string(&key(KeyCode::Enter));
        assert_eq!(s, "Enter");
    }

    #[test]
    fn test_key_event_to_string_esc() {
        let s = KeymapManager::key_event_to_string(&key(KeyCode::Esc));
        assert_eq!(s, "Esc");
    }

    #[test]
    fn test_key_event_to_string_ctrl_u() {
        let k = key_with_mod(KeyCode::Char('u'), KeyModifiers::CONTROL);
        let s = KeymapManager::key_event_to_string(&k);
        assert_eq!(s, "Ctrl+u");
    }

    #[test]
    fn test_key_event_to_string_ctrl_d() {
        let k = key_with_mod(KeyCode::Char('d'), KeyModifiers::CONTROL);
        let s = KeymapManager::key_event_to_string(&k);
        assert_eq!(s, "Ctrl+d");
    }

    #[test]
    fn test_key_event_to_string_f1() {
        let s = KeymapManager::key_event_to_string(&key(KeyCode::F(1)));
        assert_eq!(s, "F1");
    }

    #[test]
    fn test_command_mode_esc_returns_normal() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Esc), &Mode::Command(String::new()));
        assert_eq!(result, Some(Action::ModeNormal));
    }

    #[test]
    fn test_command_mode_enter_returns_normal() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Enter), &Mode::Command(String::new()));
        assert_eq!(result, Some(Action::ModeNormal));
    }

    #[test]
    fn test_command_mode_unmapped_key() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Char('x')), &Mode::Command(String::new()));
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_action_edit_message() {
        assert_eq!(KeymapManager::parse_action("edit_message"), Some(Action::EditMessage));
    }

    #[test]
    fn test_resolve_message_select_edit() {
        let mut km = KeymapManager::default_keymap();
        let result = km.resolve(key(KeyCode::Char('e')), &Mode::MessageSelect);
        assert_eq!(result, Some(Action::EditMessage));
    }
}
