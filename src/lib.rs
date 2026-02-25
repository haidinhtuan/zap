// Module declarations added as modules are created.
pub mod app;
pub mod config;
pub mod error;
pub mod event;
pub mod input;
pub mod logging;
pub mod matrix;
pub mod store;
pub mod ui;

use app::{Action, App, Message, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use event::{Event, EventHandler};
use input::KeymapManager;
use matrix::sync::MatrixEvent;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

/// Map a key event to an application action based on the current mode.
///
/// This is the fallback used when no KeymapManager is available (e.g. in tests).
pub fn map_key_to_action(key: KeyEvent, mode: &Mode) -> Action {
    match mode {
        Mode::Normal => match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('j') | KeyCode::Down => Action::RoomNext,
            KeyCode::Char('k') | KeyCode::Up => Action::RoomPrev,
            KeyCode::Char('i') => Action::ModeInsert,
            KeyCode::Char(':') => Action::ModeCommand,
            KeyCode::Char('G') => Action::RoomLast,
            KeyCode::Char('/') => Action::RoomFilter,
            KeyCode::Char('r') => Action::MarkRead,
            KeyCode::Char('R') => Action::MarkAllRead,
            KeyCode::Enter => Action::EnterMessageSelect,
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::ScrollUp,
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::ScrollDown,
            _ => Action::None,
        },
        Mode::MessageSelect => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Action::MessageNext,
            KeyCode::Char('k') | KeyCode::Up => Action::MessagePrev,
            KeyCode::Char('r') => Action::ReplyTo,
            KeyCode::Char('d') => Action::DeleteMessage,
            KeyCode::Char('i') => Action::ModeInsert,
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc => Action::ModeNormal,
            _ => Action::None,
        },
        Mode::Insert => match key.code {
            KeyCode::Esc => Action::ModeNormal,
            KeyCode::Enter => Action::SendMessage,
            _ => Action::None,
        },
        Mode::Command(_) => match key.code {
            KeyCode::Esc => Action::ModeNormal,
            KeyCode::Enter => Action::ModeNormal,
            _ => Action::None,
        },
    }
}

/// Run the main application event loop.
///
/// Integrates the terminal event handler, configurable keybinding manager,
/// Matrix sync channel, and UI rendering.
pub async fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    events: &mut EventHandler,
    keymap: &mut KeymapManager,
    matrix_rx: &mut mpsc::UnboundedReceiver<MatrixEvent>,
    matrix_client: Option<&matrix_sdk::Client>,
) -> color_eyre::Result<()> {
    // Track which rooms we've already loaded history for.
    let mut history_loaded: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        // If we have a selected room and haven't loaded its history yet, do so.
        if let Some(client) = matrix_client {
            if let Some(room) = app.rooms.get(app.selected_room) {
                let room_id = room.id.clone();
                if !history_loaded.contains(&room_id) {
                    history_loaded.insert(room_id.clone());
                    let own_uid = app.own_user_id.clone();
                    if let Ok(rid) = matrix_sdk::ruma::RoomId::parse(&room_id) {
                        if let Some(room) = client.get_room(&rid) {
                            load_room_messages(app, &room, &room_id, own_uid.as_deref()).await;
                        }
                    }
                }
            }
        }

        tokio::select! {
            event = events.next() => {
                match event? {
                    Event::Key(key) => {
                        // In insert mode, handle character input before keybindings.
                        if app.mode == Mode::Insert {
                            match key.code {
                                KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    app.reply_context = None;
                                    continue;
                                }
                                KeyCode::Char(c) => {
                                    app.input_buffer.push(c);
                                    continue;
                                }
                                KeyCode::Backspace => {
                                    app.input_buffer.pop();
                                    continue;
                                }
                                KeyCode::Enter => {
                                    // Send message.
                                    if !app.input_buffer.is_empty() {
                                        let msg_text = app.input_buffer.clone();
                                        app.input_buffer.clear();
                                        let reply_ctx = app.reply_context.take();

                                        if let Some(client) = matrix_client {
                                            if let Some(room_data) = app.rooms.get(app.selected_room) {
                                                let room_id_str = room_data.id.clone();
                                                if let Ok(rid) = matrix_sdk::ruma::RoomId::parse(&room_id_str) {
                                                    if let Some(room) = client.get_room(&rid) {
                                                        let content = if let Some(ref ctx) = reply_ctx {
                                                            if let Ok(event_id) = matrix_sdk::ruma::EventId::parse(&ctx.event_id) {
                                                                use matrix_sdk::ruma::events::room::message::{ForwardThread, AddMentions};
                                                                use matrix_sdk::ruma::events::room::message::ReplyMetadata;
                                                                let reply_meta = ReplyMetadata::new(
                                                                    &event_id,
                                                                    matrix_sdk::ruma::user_id!("@unknown:localhost"),
                                                                    None,
                                                                );
                                                                RoomMessageEventContent::text_plain(&msg_text)
                                                                    .make_reply_to(reply_meta, ForwardThread::Yes, AddMentions::No)
                                                            } else {
                                                                RoomMessageEventContent::text_plain(&msg_text)
                                                            }
                                                        } else {
                                                            RoomMessageEventContent::text_plain(&msg_text)
                                                        };

                                                        if let Err(e) = room.send(content).await {
                                                            tracing::warn!("Failed to send message: {}", e);
                                                            app.reply_context = reply_ctx;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    continue;
                                }
                                KeyCode::Esc => {
                                    app.mode = Mode::Normal;
                                    app.reply_context = None;
                                    app.selected_message = None;
                                    continue;
                                }
                                _ => continue,
                            }
                        }

                        // In command mode, handle character input.
                        if let Mode::Command(ref mut buf) = app.mode {
                            match key.code {
                                KeyCode::Char(c) => {
                                    buf.push(c);
                                    continue;
                                }
                                KeyCode::Backspace => {
                                    buf.pop();
                                    continue;
                                }
                                KeyCode::Esc => {
                                    app.mode = Mode::Normal;
                                    continue;
                                }
                                KeyCode::Enter => {
                                    // Execute command.
                                    let cmd = buf.clone();
                                    app.mode = Mode::Normal;
                                    if cmd == "q" || cmd == "quit" {
                                        app.should_quit = true;
                                    }
                                    continue;
                                }
                                _ => continue,
                            }
                        }

                        // In message select mode, handle delete confirmation or normal actions.
                        if app.mode == Mode::MessageSelect {
                            if app.confirm_delete {
                                // Delete confirmation: y to confirm, n/Esc to cancel.
                                match key.code {
                                    KeyCode::Char('y') => {
                                        // Perform the redact.
                                        if let Some(idx) = app.selected_message {
                                            if let Some(room_data) = app.rooms.get(app.selected_room) {
                                                let room_id_str = room_data.id.clone();
                                                if let Some(msgs) = app.messages.get(&room_id_str) {
                                                    if let Some(msg) = msgs.get(idx) {
                                                        if let Some(ref eid) = msg.event_id {
                                                            let eid_clone = eid.clone();
                                                            if let Some(client) = matrix_client {
                                                                if let Ok(rid) = matrix_sdk::ruma::RoomId::parse(&room_id_str) {
                                                                    if let Some(room) = client.get_room(&rid) {
                                                                        if let Ok(event_id) = matrix_sdk::ruma::EventId::parse(&eid_clone) {
                                                                            match room.redact(&event_id, None, None).await {
                                                                                Ok(_) => {
                                                                                    // Remove from local messages.
                                                                                    if let Some(msgs) = app.messages.get_mut(&room_id_str) {
                                                                                        msgs.remove(idx);
                                                                                        // Adjust selected_message.
                                                                                        if msgs.is_empty() {
                                                                                            app.selected_message = None;
                                                                                            app.mode = Mode::Normal;
                                                                                        } else {
                                                                                            app.selected_message = Some(idx.min(msgs.len() - 1));
                                                                                        }
                                                                                    }
                                                                                }
                                                                                Err(e) => {
                                                                                    tracing::warn!("Failed to delete message: {}", e);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        app.confirm_delete = false;
                                    }
                                    KeyCode::Char('n') | KeyCode::Esc => {
                                        app.confirm_delete = false;
                                    }
                                    _ => {} // Ignore other keys during confirmation.
                                }
                                continue;
                            }

                            let action = map_key_to_action(key, &app.mode);
                            app.handle_action(action);
                            continue;
                        }

                        // Normal mode: use keymap.
                        if let Some(action) = keymap.resolve(key, &app.mode) {
                            app.handle_action(action);
                        } else {
                        }
                    }
                    Event::Render => {
                        terminal.draw(|frame| {
                            ui::draw(frame, app);
                        })?;
                    }
                    Event::Tick => {
                        // Periodic housekeeping
                    }
                    _ => {}
                }
            }
            Some(matrix_event) = matrix_rx.recv() => {
                match matrix_event {
                    MatrixEvent::RoomListUpdate(rooms) => {
                        app.rooms = rooms;
                        // Sort rooms by most recent message timestamp (descending).
                        let msgs = &app.messages;
                        app.rooms.sort_by(|a, b| {
                            let a_ts = msgs.get(&a.id)
                                .and_then(|m| m.last().map(|msg| msg.timestamp))
                                .or(a.last_activity);
                            let b_ts = msgs.get(&b.id)
                                .and_then(|m| m.last().map(|msg| msg.timestamp))
                                .or(b.last_activity);
                            b_ts.cmp(&a_ts)
                        });
                        app.connection_status = app::ConnectionStatus::Connected;
                        tracing::debug!("Room list updated: {} rooms", app.rooms.len());
                    }
                    MatrixEvent::NewMessage { room_id, message } => {
                        // Deduplicate by event_id.
                        let msgs = app.messages
                            .entry(room_id.clone())
                            .or_insert_with(Vec::new);
                        let is_dup = message.event_id.as_ref().is_some_and(|eid| {
                            msgs.iter().any(|m| m.event_id.as_ref() == Some(eid))
                        });
                        if is_dup {
                            continue;
                        }
                        msgs.push(message);
                        // Re-sort rooms after new message.
                        let msgs = &app.messages;
                        app.rooms.sort_by(|a, b| {
                            let a_ts = msgs.get(&a.id)
                                .and_then(|m| m.last().map(|msg| msg.timestamp))
                                .or(a.last_activity);
                            let b_ts = msgs.get(&b.id)
                                .and_then(|m| m.last().map(|msg| msg.timestamp))
                                .or(b.last_activity);
                            b_ts.cmp(&a_ts)
                        });
                    }
                    MatrixEvent::SyncError(err) => {
                        tracing::warn!("Matrix sync error: {}", err);
                        app.connection_status = app::ConnectionStatus::Disconnected;
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

/// Load recent messages from a Matrix room into the app state.
async fn load_room_messages(app: &mut App, room: &matrix_sdk::Room, room_id: &str, own_user_id: Option<&str>) {
    use matrix_sdk::room::MessagesOptions;
    use matrix_sdk::ruma::events::AnySyncMessageLikeEvent;
    use matrix_sdk::ruma::events::AnySyncTimelineEvent;
    use matrix_sdk::ruma::events::room::message::MessageType;

    let mut options = MessagesOptions::backward();
    options.limit = 50u32.into();

    match room.messages(options).await {
        Ok(response) => {
            let mut messages: Vec<Message> = Vec::new();

            for timeline_event in &response.chunk {
                if let Ok(event) = timeline_event.raw().deserialize() {
                    if let AnySyncTimelineEvent::MessageLike(
                        AnySyncMessageLikeEvent::RoomMessage(msg_event),
                    ) = event
                    {
                        if let matrix_sdk::ruma::events::room::message::SyncRoomMessageEvent::Original(orig) = msg_event {
                            let body = match orig.content.msgtype {
                                MessageType::Text(text) => text.body,
                                MessageType::Notice(notice) => notice.body,
                                MessageType::Emote(emote) => format!("* {}", emote.body),
                                _ => continue,
                            };

                            // Resolve display name.
                            let display_name = room
                                .get_member_no_sync(&orig.sender)
                                .await
                                .ok()
                                .flatten()
                                .and_then(|m| m.display_name().map(|n| n.to_string()))
                                .unwrap_or_else(|| orig.sender.localpart().to_string());

                            // Check for reply.
                            let reply_to = orig.content.relates_to.as_ref().and_then(|r| {
                                if let matrix_sdk::ruma::events::room::message::Relation::Reply { in_reply_to } = r {
                                    Some(in_reply_to.event_id.to_string())
                                } else {
                                    None
                                }
                            });

                            // Strip fallback reply prefix.
                            let clean_body = if body.starts_with("> ") {
                                body.lines()
                                    .skip_while(|l| l.starts_with("> ") || l.is_empty())
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            } else {
                                body
                            };

                            let millis = i64::from(orig.origin_server_ts.as_secs()) * 1000;
                            let timestamp = chrono::DateTime::from_timestamp_millis(millis)
                                .unwrap_or_else(chrono::Utc::now);

                            let is_own = own_user_id
                                .map(|uid| orig.sender.as_str() == uid)
                                .unwrap_or(false);

                            messages.push(Message {
                                event_id: Some(orig.event_id.to_string()),
                                sender: display_name,
                                body: clean_body,
                                timestamp,
                                is_own,
                                reply_to,
                            });
                        }
                    }
                }
            }

            // Messages come in reverse order (newest first), so reverse them.
            messages.reverse();

            if !messages.is_empty() {
                tracing::debug!("Loaded {} messages for room {}", messages.len(), room_id);
                app.messages.insert(room_id.to_string(), messages);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load messages for room {}: {}", room_id, e);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

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

    // -- Normal mode mappings --

    #[test]
    fn test_normal_q_quits() {
        assert_eq!(map_key_to_action(key(KeyCode::Char('q')), &Mode::Normal), Action::Quit);
    }

    #[test]
    fn test_normal_j_room_next() {
        assert_eq!(map_key_to_action(key(KeyCode::Char('j')), &Mode::Normal), Action::RoomNext);
    }

    #[test]
    fn test_normal_k_room_prev() {
        assert_eq!(map_key_to_action(key(KeyCode::Char('k')), &Mode::Normal), Action::RoomPrev);
    }

    #[test]
    fn test_normal_i_insert() {
        assert_eq!(map_key_to_action(key(KeyCode::Char('i')), &Mode::Normal), Action::ModeInsert);
    }

    #[test]
    fn test_normal_colon_command() {
        assert_eq!(map_key_to_action(key(KeyCode::Char(':')), &Mode::Normal), Action::ModeCommand);
    }

    #[test]
    fn test_normal_g_room_last() {
        assert_eq!(map_key_to_action(key(KeyCode::Char('G')), &Mode::Normal), Action::RoomLast);
    }

    #[test]
    fn test_normal_ctrl_u_scroll_up() {
        assert_eq!(
            map_key_to_action(key_with_mod(KeyCode::Char('u'), KeyModifiers::CONTROL), &Mode::Normal),
            Action::ScrollUp
        );
    }

    #[test]
    fn test_normal_ctrl_d_scroll_down() {
        assert_eq!(
            map_key_to_action(key_with_mod(KeyCode::Char('d'), KeyModifiers::CONTROL), &Mode::Normal),
            Action::ScrollDown
        );
    }

    #[test]
    fn test_normal_unknown_key_none() {
        assert_eq!(map_key_to_action(key(KeyCode::Char('z')), &Mode::Normal), Action::None);
    }

    // -- Insert mode mappings --

    #[test]
    fn test_insert_esc_normal() {
        assert_eq!(map_key_to_action(key(KeyCode::Esc), &Mode::Insert), Action::ModeNormal);
    }

    #[test]
    fn test_insert_enter_send() {
        assert_eq!(map_key_to_action(key(KeyCode::Enter), &Mode::Insert), Action::SendMessage);
    }

    #[test]
    fn test_insert_char_none() {
        assert_eq!(map_key_to_action(key(KeyCode::Char('a')), &Mode::Insert), Action::None);
    }

    // -- Command mode mappings --

    #[test]
    fn test_command_esc_normal() {
        assert_eq!(
            map_key_to_action(key(KeyCode::Esc), &Mode::Command(String::new())),
            Action::ModeNormal
        );
    }

    #[test]
    fn test_command_enter_normal() {
        assert_eq!(
            map_key_to_action(key(KeyCode::Enter), &Mode::Command(String::new())),
            Action::ModeNormal
        );
    }
}
