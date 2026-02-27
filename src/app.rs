use std::collections::BTreeMap;

use crate::config::ThemeConfig;

/// Represents a Matrix room in the room list.
#[derive(Debug, Clone)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub unread_count: u32,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    pub is_direct: bool,
}

/// Represents a single chat message.
#[derive(Debug, Clone)]
pub struct Message {
    pub event_id: Option<String>,
    pub sender: String,
    pub body: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub is_own: bool,
    /// If this is a reply, the event_id of the message being replied to.
    pub reply_to: Option<String>,
}

/// Context for an in-progress reply.
#[derive(Debug, Clone)]
pub struct ReplyContext {
    pub event_id: String,
    pub sender: String,
    pub body: String,
}

/// Context for an in-progress message edit.
#[derive(Debug, Clone)]
pub struct EditContext {
    pub event_id: String,
    pub room_id: String,
}

/// A user from the Matrix user directory search.
#[derive(Debug, Clone)]
pub struct UserSearchResult {
    pub user_id: String,
    pub display_name: Option<String>,
}

/// The current input mode of the application.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    MessageSelect,
    Command(String),
    RoomFilter,
    ContactSearch,
}

/// Connection status to the Matrix homeserver.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
}

/// Actions that can be dispatched within the application.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    ModeNormal,
    ModeInsert,
    ModeCommand,
    EnterMessageSelect,
    MessageNext,
    MessagePrev,
    ReplyTo,
    CancelReply,
    DeleteMessage,
    ConfirmDelete,
    CancelDelete,
    RoomNext,
    RoomPrev,
    RoomFirst,
    RoomLast,
    OpenRoom,
    RoomFilter,
    ScrollUp,
    ScrollDown,
    SendMessage,
    MarkRead,
    MarkAllRead,
    EditMessage,
    NewMessage,
    None,
}

/// The central application state.
pub struct App {
    pub rooms: Vec<Room>,
    pub selected_room: usize,
    pub messages: BTreeMap<String, Vec<Message>>,
    pub mode: Mode,
    pub textarea: tui_textarea::TextArea<'static>,
    pub scroll_offset: usize,
    pub should_quit: bool,
    pub connection_status: ConnectionStatus,
    pub theme: Option<ThemeConfig>,
    pub room_list_width: u16,
    pub selected_message: Option<usize>,
    pub reply_context: Option<ReplyContext>,
    pub edit_context: Option<EditContext>,
    /// When true, the UI shows a delete confirmation prompt.
    pub confirm_delete: bool,
    /// The logged-in user's Matrix ID (e.g. "@haidinhtuan:localhost").
    pub own_user_id: Option<String>,
    /// Current room filter string (for RoomFilter mode).
    pub room_filter: String,
    /// Current contact search string (for ContactSearch mode).
    pub contact_search: String,
    /// Results from the Matrix user directory search.
    pub contact_results: Vec<UserSearchResult>,
    /// Currently selected index in the contact search results.
    pub selected_contact: usize,
    /// Whether Vietnamese input mode (Telex) is enabled.
    pub vigo_enabled: bool,
    /// Vigo FastEngine for Vietnamese input.
    pub vigo_engine: vigo::FastEngine,
    /// Number of chars of current Vigo composition in the textarea.
    pub vigo_comp_len: usize,
}

impl App {
    /// Create a new App with default state.
    pub fn new() -> Self {
        Self {
            rooms: Vec::new(),
            selected_room: 0,
            messages: BTreeMap::new(),
            mode: Mode::Normal,
            textarea: {
                let mut ta = tui_textarea::TextArea::default();
                ta.set_cursor_line_style(ratatui::style::Style::default());
                ta.set_block(ratatui::widgets::Block::default());
                ta
            },
            scroll_offset: 0,
            should_quit: false,
            connection_status: ConnectionStatus::Disconnected,
            theme: None,
            room_list_width: 30,
            selected_message: None,
            reply_context: None,
            edit_context: None,
            confirm_delete: false,
            own_user_id: None,
            room_filter: String::new(),
            contact_search: String::new(),
            contact_results: Vec::new(),
            selected_contact: 0,
            vigo_enabled: false,
            vigo_engine: vigo::FastEngine::telex(),
            vigo_comp_len: 0,
        }
    }

    /// Extract the current text from the TextArea as a single string.
    pub fn textarea_text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Clear the TextArea content.
    pub fn textarea_clear(&mut self) {
        self.textarea = {
            let mut ta = tui_textarea::TextArea::default();
            ta.set_cursor_line_style(ratatui::style::Style::default());
            ta.set_block(ratatui::widgets::Block::default());
            ta
        };
        self.vigo_engine.clear();
        self.vigo_comp_len = 0;
    }

    /// Commit current Vigo composition and reset engine state.
    pub fn vigo_commit(&mut self) {
        self.vigo_engine.clear();
        self.vigo_comp_len = 0;
    }

    /// Get the list of room indices that match the current filter.
    ///
    /// Supports type prefixes: `@` filters DMs only, `#` filters groups only.
    pub fn filtered_room_indices(&self) -> Vec<usize> {
        if self.room_filter.is_empty() {
            (0..self.rooms.len()).collect()
        } else {
            // Check for type prefix.
            let (type_filter, name_filter) = if self.room_filter.starts_with('@') {
                (Some(true), self.room_filter[1..].to_lowercase())
            } else if self.room_filter.starts_with('#') {
                (Some(false), self.room_filter[1..].to_lowercase())
            } else {
                (None, self.room_filter.to_lowercase())
            };

            self.rooms
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    let type_match = type_filter.map_or(true, |dm| r.is_direct == dm);
                    let name_match = name_filter.is_empty()
                        || r.name.to_lowercase().contains(&name_filter);
                    type_match && name_match
                })
                .map(|(i, _)| i)
                .collect()
        }
    }

    /// Dispatch an action and mutate application state accordingly.
    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::ModeNormal => {
                self.mode = Mode::Normal;
                self.selected_message = None;
                self.edit_context = None;
                self.room_filter.clear();
                self.contact_search.clear();
                self.contact_results.clear();
                self.selected_contact = 0;
            }
            Action::ModeInsert => {
                if self.mode == Mode::Normal {
                    self.mode = Mode::Insert;
                }
            }
            Action::ModeCommand => {
                if self.mode == Mode::Normal {
                    self.mode = Mode::Command(String::new());
                }
            }
            Action::RoomNext => {
                if self.mode == Mode::Normal && !self.rooms.is_empty() {
                    self.selected_room = (self.selected_room + 1).min(self.rooms.len() - 1);
                    self.scroll_offset = 0;
                    self.rooms[self.selected_room].unread_count = 0;
                }
            }
            Action::RoomPrev => {
                if self.mode == Mode::Normal {
                    self.selected_room = self.selected_room.saturating_sub(1);
                    self.scroll_offset = 0;
                    if let Some(room) = self.rooms.get_mut(self.selected_room) {
                        room.unread_count = 0;
                    }
                }
            }
            Action::RoomFirst => {
                if self.mode == Mode::Normal && !self.rooms.is_empty() {
                    self.selected_room = 0;
                    self.scroll_offset = 0;
                    self.rooms[self.selected_room].unread_count = 0;
                }
            }
            Action::RoomLast => {
                if self.mode == Mode::Normal && !self.rooms.is_empty() {
                    self.selected_room = self.rooms.len() - 1;
                    self.scroll_offset = 0;
                    self.rooms[self.selected_room].unread_count = 0;
                }
            }
            Action::OpenRoom => {
                // Placeholder: will open a room view in the future.
            }
            Action::RoomFilter => {
                if self.mode == Mode::Normal {
                    self.room_filter.clear();
                    self.mode = Mode::RoomFilter;
                }
            }
            Action::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_add(5);
            }
            Action::ScrollDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
            }
            Action::SendMessage => {
                // Placeholder: will send the message in the future.
            }
            Action::MarkRead => {
                if let Some(room) = self.rooms.get_mut(self.selected_room) {
                    room.unread_count = 0;
                }
            }
            Action::MarkAllRead => {
                for room in &mut self.rooms {
                    room.unread_count = 0;
                }
            }
            Action::EnterMessageSelect => {
                if self.mode == Mode::Normal {
                    if let Some(room) = self.rooms.get(self.selected_room) {
                        if let Some(msgs) = self.messages.get(&room.id) {
                            if !msgs.is_empty() {
                                self.mode = Mode::MessageSelect;
                                self.selected_message = Some(msgs.len() - 1);
                            }
                        }
                    }
                }
            }
            Action::MessageNext => {
                if self.mode == Mode::MessageSelect {
                    if let Some(idx) = self.selected_message {
                        if let Some(room) = self.rooms.get(self.selected_room) {
                            if let Some(msgs) = self.messages.get(&room.id) {
                                self.selected_message = Some((idx + 1).min(msgs.len() - 1));
                            }
                        }
                    }
                }
            }
            Action::MessagePrev => {
                if self.mode == Mode::MessageSelect {
                    if let Some(idx) = self.selected_message {
                        self.selected_message = Some(idx.saturating_sub(1));
                    }
                }
            }
            Action::ReplyTo => {
                if self.mode == Mode::MessageSelect {
                    if let Some(idx) = self.selected_message {
                        if let Some(room) = self.rooms.get(self.selected_room) {
                            if let Some(msgs) = self.messages.get(&room.id) {
                                if let Some(msg) = msgs.get(idx) {
                                    if let Some(ref eid) = msg.event_id {
                                        self.reply_context = Some(ReplyContext {
                                            event_id: eid.clone(),
                                            sender: msg.sender.clone(),
                                            body: msg.body.clone(),
                                        });
                                        self.mode = Mode::Insert;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Action::CancelReply => {
                self.reply_context = None;
                self.edit_context = None;
            }
            Action::DeleteMessage => {
                // Only trigger from MessageSelect when a message is selected.
                if self.mode == Mode::MessageSelect && self.selected_message.is_some() {
                    self.confirm_delete = true;
                }
            }
            Action::ConfirmDelete => {
                // Handled in the event loop (needs async Matrix call).
                // This action is dispatched but actual redact happens in run_app.
            }
            Action::CancelDelete => {
                self.confirm_delete = false;
            }
            Action::EditMessage => {
                if self.mode == Mode::MessageSelect {
                    if let Some(idx) = self.selected_message {
                        // Extract needed data before mutating self.
                        let edit_info = self.rooms.get(self.selected_room).and_then(|room| {
                            self.messages.get(&room.id).and_then(|msgs| {
                                msgs.get(idx).and_then(|msg| {
                                    if msg.is_own {
                                        msg.event_id.as_ref().map(|eid| {
                                            (eid.clone(), room.id.clone(), msg.body.clone())
                                        })
                                    } else {
                                        None
                                    }
                                })
                            })
                        });
                        if let Some((event_id, room_id, body)) = edit_info {
                            self.edit_context = Some(EditContext { event_id, room_id });
                            self.textarea_clear();
                            self.textarea.insert_str(&body);
                            self.mode = Mode::Insert;
                        }
                    }
                }
            }
            Action::NewMessage => {
                if self.mode == Mode::Normal {
                    self.contact_search.clear();
                    self.contact_results.clear();
                    self.selected_contact = 0;
                    self.mode = Mode::ContactSearch;
                }
            }
            Action::None => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rooms(n: usize) -> Vec<Room> {
        (0..n)
            .map(|i| Room {
                id: format!("!room{}:example.com", i),
                name: format!("Room {}", i),
                unread_count: i as u32,
                last_activity: None,
                is_direct: false,
            })
            .collect()
    }

    // -- Mode transitions --

    #[test]
    fn test_initial_mode_is_normal() {
        let app = App::new();
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_mode_normal_to_insert() {
        let mut app = App::new();
        app.handle_action(Action::ModeInsert);
        assert_eq!(app.mode, Mode::Insert);
    }

    #[test]
    fn test_mode_insert_to_normal() {
        let mut app = App::new();
        app.handle_action(Action::ModeInsert);
        app.handle_action(Action::ModeNormal);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_mode_normal_to_command() {
        let mut app = App::new();
        app.handle_action(Action::ModeCommand);
        assert_eq!(app.mode, Mode::Command(String::new()));
    }

    #[test]
    fn test_mode_command_to_normal() {
        let mut app = App::new();
        app.handle_action(Action::ModeCommand);
        app.handle_action(Action::ModeNormal);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_insert_from_insert_stays_insert() {
        let mut app = App::new();
        app.handle_action(Action::ModeInsert);
        app.handle_action(Action::ModeInsert);
        assert_eq!(app.mode, Mode::Insert);
    }

    #[test]
    fn test_command_from_insert_stays_insert() {
        let mut app = App::new();
        app.handle_action(Action::ModeInsert);
        app.handle_action(Action::ModeCommand);
        // ModeCommand only works from Normal mode
        assert_eq!(app.mode, Mode::Insert);
    }

    // -- Quit --

    #[test]
    fn test_quit_sets_flag() {
        let mut app = App::new();
        app.handle_action(Action::Quit);
        assert!(app.should_quit);
    }

    // -- Room navigation --

    #[test]
    fn test_room_next() {
        let mut app = App::new();
        app.rooms = make_rooms(5);
        app.handle_action(Action::RoomNext);
        assert_eq!(app.selected_room, 1);
    }

    #[test]
    fn test_room_prev() {
        let mut app = App::new();
        app.rooms = make_rooms(5);
        app.selected_room = 3;
        app.handle_action(Action::RoomPrev);
        assert_eq!(app.selected_room, 2);
    }

    #[test]
    fn test_room_next_clamp() {
        let mut app = App::new();
        app.rooms = make_rooms(3);
        app.selected_room = 2;
        app.handle_action(Action::RoomNext);
        assert_eq!(app.selected_room, 2); // clamped to last index
    }

    #[test]
    fn test_room_prev_clamp() {
        let mut app = App::new();
        app.rooms = make_rooms(3);
        app.selected_room = 0;
        app.handle_action(Action::RoomPrev);
        assert_eq!(app.selected_room, 0); // saturating_sub
    }

    #[test]
    fn test_room_first() {
        let mut app = App::new();
        app.rooms = make_rooms(5);
        app.selected_room = 3;
        app.handle_action(Action::RoomFirst);
        assert_eq!(app.selected_room, 0);
    }

    #[test]
    fn test_room_last() {
        let mut app = App::new();
        app.rooms = make_rooms(5);
        app.handle_action(Action::RoomLast);
        assert_eq!(app.selected_room, 4);
    }

    // -- Scroll --

    #[test]
    fn test_scroll_up_increases_offset() {
        let mut app = App::new();
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.scroll_offset, 5);
    }

    #[test]
    fn test_scroll_down_decreases_offset() {
        let mut app = App::new();
        app.scroll_offset = 15;
        app.handle_action(Action::ScrollDown);
        assert_eq!(app.scroll_offset, 10);
    }

    #[test]
    fn test_scroll_down_saturating() {
        let mut app = App::new();
        app.scroll_offset = 3;
        app.handle_action(Action::ScrollDown);
        assert_eq!(app.scroll_offset, 0);
    }

    // -- Mark read --

    #[test]
    fn test_mark_read() {
        let mut app = App::new();
        app.rooms = make_rooms(3);
        app.rooms[0].unread_count = 5;
        app.handle_action(Action::MarkRead);
        assert_eq!(app.rooms[0].unread_count, 0);
    }

    #[test]
    fn test_mark_all_read() {
        let mut app = App::new();
        app.rooms = make_rooms(3);
        app.handle_action(Action::MarkAllRead);
        assert!(app.rooms.iter().all(|r| r.unread_count == 0));
    }

    // -- Insert mode blocks navigation --

    #[test]
    fn test_insert_mode_blocks_room_nav() {
        let mut app = App::new();
        app.rooms = make_rooms(5);
        app.handle_action(Action::ModeInsert);
        app.handle_action(Action::RoomNext);
        assert_eq!(app.selected_room, 0); // did not move
    }

    // -- MessageSelect mode --

    #[test]
    fn test_enter_message_select_mode() {
        let mut app = App::new();
        app.rooms = make_rooms(3);
        // Add messages for room 0.
        app.messages.insert(
            "!room0:example.com".to_string(),
            vec![Message {
                event_id: Some("$ev1".to_string()),
                sender: "alice".to_string(),
                body: "hello".to_string(),
                timestamp: chrono::Utc::now(),
                is_own: false,
                reply_to: None,
            }],
        );
        app.handle_action(Action::EnterMessageSelect);
        assert_eq!(app.mode, Mode::MessageSelect);
        // selected_message should be last message index (newest).
        assert_eq!(app.selected_message, Some(0));
    }

    #[test]
    fn test_message_select_next_prev() {
        let mut app = App::new();
        app.rooms = make_rooms(1);
        app.messages.insert(
            "!room0:example.com".to_string(),
            vec![
                Message {
                    event_id: Some("$ev1".to_string()),
                    sender: "a".to_string(),
                    body: "first".to_string(),
                    timestamp: chrono::Utc::now(),
                    is_own: false,
                    reply_to: None,
                },
                Message {
                    event_id: Some("$ev2".to_string()),
                    sender: "b".to_string(),
                    body: "second".to_string(),
                    timestamp: chrono::Utc::now(),
                    is_own: false,
                    reply_to: None,
                },
            ],
        );
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(1); // start at last
        app.handle_action(Action::MessagePrev);
        assert_eq!(app.selected_message, Some(0));
        app.handle_action(Action::MessageNext);
        assert_eq!(app.selected_message, Some(1));
    }

    #[test]
    fn test_message_select_clamp() {
        let mut app = App::new();
        app.rooms = make_rooms(1);
        app.messages.insert(
            "!room0:example.com".to_string(),
            vec![Message {
                event_id: None,
                sender: "a".to_string(),
                body: "only".to_string(),
                timestamp: chrono::Utc::now(),
                is_own: false,
                reply_to: None,
            }],
        );
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(0);
        app.handle_action(Action::MessagePrev); // can't go below 0
        assert_eq!(app.selected_message, Some(0));
        app.handle_action(Action::MessageNext); // can't go above 0
        assert_eq!(app.selected_message, Some(0));
    }

    #[test]
    fn test_reply_to_sets_context() {
        let mut app = App::new();
        app.rooms = make_rooms(1);
        app.messages.insert(
            "!room0:example.com".to_string(),
            vec![Message {
                event_id: Some("$ev1".to_string()),
                sender: "alice".to_string(),
                body: "hello world".to_string(),
                timestamp: chrono::Utc::now(),
                is_own: false,
                reply_to: None,
            }],
        );
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(0);
        app.handle_action(Action::ReplyTo);
        assert_eq!(app.mode, Mode::Insert);
        assert!(app.reply_context.is_some());
        let ctx = app.reply_context.as_ref().unwrap();
        assert_eq!(ctx.event_id, "$ev1");
        assert_eq!(ctx.sender, "alice");
    }

    #[test]
    fn test_cancel_reply() {
        let mut app = App::new();
        app.reply_context = Some(ReplyContext {
            event_id: "$ev1".to_string(),
            sender: "alice".to_string(),
            body: "hello".to_string(),
        });
        app.mode = Mode::Insert;
        app.handle_action(Action::CancelReply);
        assert!(app.reply_context.is_none());
        assert_eq!(app.mode, Mode::Insert); // stays in insert
    }

    #[test]
    fn test_esc_from_message_select_goes_normal() {
        let mut app = App::new();
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(2);
        app.handle_action(Action::ModeNormal);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.selected_message, None);
    }

    #[test]
    fn test_delete_message_sets_confirm() {
        let mut app = App::new();
        app.rooms = make_rooms(1);
        app.messages.insert(
            "!room0:example.com".to_string(),
            vec![Message {
                event_id: Some("$ev1".to_string()),
                sender: "alice".to_string(),
                body: "hello".to_string(),
                timestamp: chrono::Utc::now(),
                is_own: true,
                reply_to: None,
            }],
        );
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(0);
        app.handle_action(Action::DeleteMessage);
        assert!(app.confirm_delete);
    }

    #[test]
    fn test_cancel_delete() {
        let mut app = App::new();
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(0);
        app.confirm_delete = true;
        app.handle_action(Action::CancelDelete);
        assert!(!app.confirm_delete);
        assert_eq!(app.mode, Mode::MessageSelect); // stays in select
    }

    #[test]
    fn test_delete_requires_message_select_mode() {
        let mut app = App::new();
        app.mode = Mode::Normal;
        app.selected_message = Some(0);
        app.handle_action(Action::DeleteMessage);
        assert!(!app.confirm_delete); // should not trigger outside MessageSelect
    }

    #[test]
    fn test_edit_message_sets_context() {
        let mut app = App::new();
        app.rooms = make_rooms(1);
        app.messages.insert(
            "!room0:example.com".to_string(),
            vec![Message {
                event_id: Some("$ev1".to_string()),
                sender: "alice".to_string(),
                body: "hello world".to_string(),
                timestamp: chrono::Utc::now(),
                is_own: true,
                reply_to: None,
            }],
        );
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(0);
        app.handle_action(Action::EditMessage);
        assert_eq!(app.mode, Mode::Insert);
        assert!(app.edit_context.is_some());
        let ctx = app.edit_context.as_ref().unwrap();
        assert_eq!(ctx.event_id, "$ev1");
        assert_eq!(app.textarea_text(), "hello world");
    }

    #[test]
    fn test_edit_message_ignores_others_messages() {
        let mut app = App::new();
        app.rooms = make_rooms(1);
        app.messages.insert(
            "!room0:example.com".to_string(),
            vec![Message {
                event_id: Some("$ev1".to_string()),
                sender: "bob".to_string(),
                body: "hello".to_string(),
                timestamp: chrono::Utc::now(),
                is_own: false,
                reply_to: None,
            }],
        );
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(0);
        app.handle_action(Action::EditMessage);
        assert_eq!(app.mode, Mode::MessageSelect);
        assert!(app.edit_context.is_none());
    }

    // -- Room filter --

    #[test]
    fn test_filtered_room_indices_no_filter() {
        let mut app = App::new();
        app.rooms = make_rooms(3);
        let indices = app.filtered_room_indices();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_filtered_room_indices_with_filter() {
        let mut app = App::new();
        app.rooms = vec![
            Room {
                id: "!a:x".to_string(),
                name: "General".to_string(),
                unread_count: 0,
                last_activity: None,
                is_direct: false,
            },
            Room {
                id: "!b:x".to_string(),
                name: "Random".to_string(),
                unread_count: 0,
                last_activity: None,
                is_direct: false,
            },
            Room {
                id: "!c:x".to_string(),
                name: "Dev General".to_string(),
                unread_count: 0,
                last_activity: None,
                is_direct: false,
            },
        ];
        app.room_filter = "gen".to_string();
        let indices = app.filtered_room_indices();
        assert_eq!(indices, vec![0, 2]); // "General" and "Dev General"
    }

    #[test]
    fn test_room_filter_case_insensitive() {
        let mut app = App::new();
        app.rooms = vec![
            Room {
                id: "!a:x".to_string(),
                name: "General".to_string(),
                unread_count: 0,
                last_activity: None,
                is_direct: false,
            },
            Room {
                id: "!b:x".to_string(),
                name: "Random".to_string(),
                unread_count: 0,
                last_activity: None,
                is_direct: false,
            },
        ];
        app.room_filter = "GENERAL".to_string();
        let indices = app.filtered_room_indices();
        assert_eq!(indices, vec![0]);
    }

    #[test]
    fn test_room_filter_action_enters_mode() {
        let mut app = App::new();
        app.handle_action(Action::RoomFilter);
        assert_eq!(app.mode, Mode::RoomFilter);
        assert!(app.room_filter.is_empty());
    }
}
