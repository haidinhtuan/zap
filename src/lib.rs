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
use store::LocalStore;
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
            KeyCode::Char('c') | KeyCode::Char('i') => Action::ModeInsert,
            KeyCode::Char(':') => Action::ModeCommand,
            KeyCode::Char('G') => Action::RoomLast,
            KeyCode::Char('/') => Action::RoomFilter,
            KeyCode::Char('n') => Action::NewMessage,
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
            KeyCode::Char('e') => Action::EditMessage,
            KeyCode::Char('d') => Action::DeleteMessage,
            KeyCode::Char('c') | KeyCode::Char('i') => Action::ModeInsert,
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
        Mode::RoomFilter => Action::None,
        Mode::ContactSearch => Action::None,
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
    store: Option<&LocalStore>,
) -> color_eyre::Result<()> {
    // Track which rooms we've already loaded history for.
    let mut history_loaded: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Track which room we last sent a read receipt for.
    let mut last_receipt_room: Option<String> = None;

    loop {
        prime_selected_room(
            app,
            matrix_client,
            &mut history_loaded,
            &mut last_receipt_room,
        )
        .await;

        tokio::select! {
            event = events.next() => {
                match event? {
                    Event::Key(key) => {
                        handle_key_event(app, key, keymap, matrix_client, store).await;
                    }
                    Event::Render => {
                        terminal.draw(|frame| {
                            ui::draw(frame, app);
                        })?;
                    }
                    Event::Tick => {
                        // Periodic housekeeping
                    }
                    Event::Mouse(mouse) => {
                        handle_mouse_event(app, mouse);
                    }
                    _ => {}
                }
            }
            Some(matrix_event) = matrix_rx.recv() => {
                handle_matrix_event(app, matrix_event);
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

async fn prime_selected_room(
    app: &mut App,
    matrix_client: Option<&matrix_sdk::Client>,
    history_loaded: &mut std::collections::HashSet<String>,
    last_receipt_room: &mut Option<String>,
) {
    let Some(client) = matrix_client else {
        return;
    };

    let Some(room) = app.rooms.get(app.selected_room) else {
        return;
    };

    let room_id = room.id.clone();

    if history_loaded.insert(room_id.clone()) {
        let own_uid = app.own_user_id.clone();
        if let Ok(rid) = matrix_sdk::ruma::RoomId::parse(&room_id) {
            if let Some(room) = client.get_room(&rid) {
                load_room_messages(app, &room, &room_id, own_uid.as_deref()).await;
            }
        }
    }

    if last_receipt_room.as_deref() != Some(&room_id) {
        *last_receipt_room = Some(room_id.clone());
        if let Some(current_room) = app.rooms.get_mut(app.selected_room) {
            current_room.unread_count = 0;
        }
        maybe_send_read_receipt_for_room(app, matrix_client, &room_id).await;
    }
}

async fn handle_key_event(
    app: &mut App,
    key: KeyEvent,
    keymap: &mut KeymapManager,
    matrix_client: Option<&matrix_sdk::Client>,
    store: Option<&LocalStore>,
) {
    if app.mode == Mode::Insert {
        handle_insert_key(app, key, matrix_client, store).await;
        return;
    }

    if app.mode == Mode::RoomFilter {
        handle_room_filter_key(app, key);
        return;
    }

    if app.mode == Mode::ContactSearch {
        handle_contact_search_key(app, key, matrix_client, store).await;
        return;
    }

    if matches!(app.mode, Mode::Command(_)) {
        handle_command_key(app, key);
        return;
    }

    if app.mode == Mode::MessageSelect {
        handle_message_select_key(app, key, matrix_client, store).await;
        return;
    }

    handle_normal_key(app, key, keymap, matrix_client, store).await;
}

async fn handle_insert_key(
    app: &mut App,
    key: KeyEvent,
    matrix_client: Option<&matrix_sdk::Client>,
    store: Option<&LocalStore>,
) {
    match key.code {
        KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            delete_selected_room_draft(app, store);
            clear_compose_state(app);
        }
        KeyCode::Enter if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            submit_compose_buffer(app, matrix_client, store).await;
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.selected_message = None;
            persist_or_clear_selected_room_draft(app, store);
            clear_compose_state(app);
        }
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.vigo_enabled = !app.vigo_enabled;
            app.vigo_commit();
            save_vigo_preference(store, app.vigo_enabled);
        }
        _ => {
            handle_insert_text_input(app, key);
            persist_plain_message_draft(app, store);
        }
    }
}

fn handle_insert_text_input(app: &mut App, key: KeyEvent) {
    if app.vigo_enabled {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_alphanumeric() => {
                for _ in 0..app.vigo_comp_len {
                    app.textarea.delete_char();
                }
                app.vigo_engine.feed(c);
                let output = app.vigo_engine.output().to_string();
                app.textarea.insert_str(&output);
                app.vigo_comp_len = output.chars().count();
            }
            KeyCode::Backspace => {
                if app.vigo_comp_len > 0 {
                    for _ in 0..app.vigo_comp_len {
                        app.textarea.delete_char();
                    }
                    app.vigo_engine.backspace();
                    let output = app.vigo_engine.output().to_string();
                    app.textarea.insert_str(&output);
                    app.vigo_comp_len = output.chars().count();
                } else {
                    app.textarea.input(key);
                }
            }
            _ => {
                app.vigo_commit();
                app.textarea.input(key);
            }
        }
    } else {
        app.textarea.input(key);
    }
}

fn handle_room_filter_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.room_filter.clear();
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            app.room_filter.pop();
            select_first_filtered_room(app);
        }
        KeyCode::Down => {
            let filtered = app.filtered_room_indices();
            if let Some(pos) = filtered.iter().position(|&i| i == app.selected_room) {
                if pos + 1 < filtered.len() {
                    app.selected_room = filtered[pos + 1];
                }
            }
        }
        KeyCode::Up => {
            let filtered = app.filtered_room_indices();
            if let Some(pos) = filtered.iter().position(|&i| i == app.selected_room) {
                if pos > 0 {
                    app.selected_room = filtered[pos - 1];
                }
            }
        }
        KeyCode::Char(c) => {
            app.room_filter.push(c);
            select_first_filtered_room(app);
        }
        _ => {}
    }
}

async fn handle_contact_search_key(
    app: &mut App,
    key: KeyEvent,
    matrix_client: Option<&matrix_sdk::Client>,
    store: Option<&LocalStore>,
) {
    match key.code {
        KeyCode::Esc => {
            reset_contact_search(app);
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            if let Some(contact) = app.contact_results.get(app.selected_contact).cloned() {
                open_contact_search_result(app, &contact, matrix_client, store).await;
            }

            reset_contact_search(app);
            if app.mode == Mode::ContactSearch {
                app.mode = Mode::Normal;
            }
        }
        KeyCode::Backspace => {
            app.contact_search.pop();
            refresh_contact_results(app, matrix_client).await;
        }
        KeyCode::Down => {
            if !app.contact_results.is_empty() {
                app.selected_contact =
                    (app.selected_contact + 1).min(app.contact_results.len() - 1);
            }
        }
        KeyCode::Up => {
            app.selected_contact = app.selected_contact.saturating_sub(1);
        }
        KeyCode::Char(c) => {
            app.contact_search.push(c);
            refresh_contact_results(app, matrix_client).await;
        }
        _ => {}
    }
}

async fn open_contact_search_result(
    app: &mut App,
    contact: &app::UserSearchResult,
    matrix_client: Option<&matrix_sdk::Client>,
    store: Option<&LocalStore>,
) {
    let display_name = contact.display_name.clone().unwrap_or_default();
    let search_term = app.contact_search.to_lowercase();
    let name_lower = display_name.to_lowercase();

    let is_live_match = |room: &crate::app::Room, query: &str| -> bool {
        let room_name = room.name.to_lowercase();
        !room_name.starts_with("empty room") && room_name.contains(query)
    };

    let found = if !name_lower.is_empty() {
        app.rooms.iter().position(|room| {
            room.is_direct
                && !room.name.to_lowercase().starts_with("empty room")
                && room.name.to_lowercase() == name_lower
        })
    } else {
        None
    };

    let found = found.or_else(|| {
        if search_term.is_empty() {
            None
        } else {
            app.rooms
                .iter()
                .position(|room| room.is_direct && is_live_match(room, &search_term))
        }
    });

    if let Some(pos) = found {
        tracing::info!("Contact search: found room by name at position {}", pos);
        app.selected_room = pos;
        app.mode = Mode::Insert;
        restore_selected_room_draft(app, store);
        return;
    }

    let Some(client) = matrix_client else {
        tracing::warn!("Contact search: no matrix client available");
        return;
    };

    let user_id = contact.user_id.clone();
    tracing::info!(
        "Contact search: no name match, trying API for user_id={}",
        user_id
    );

    let room_id = matrix::contacts::find_existing_dm(client, &user_id).await;
    tracing::info!("Contact search: find_existing_dm returned {:?}", room_id);

    let room_id = match room_id {
        Some(id) => Some(id),
        None => {
            tracing::info!("Contact search: creating new unencrypted DM");
            let result = matrix::contacts::create_dm_unencrypted(client, &user_id).await;
            tracing::info!("Contact search: create_dm result {:?}", result);
            result
        }
    };

    if let Some(room_id) = room_id {
        app.rooms = matrix::sync::get_room_list(client).await;
        tracing::info!("Contact search: refreshed rooms, count={}", app.rooms.len());
        if let Some(pos) = app.rooms.iter().position(|room| room.id == room_id) {
            app.selected_room = pos;
            app.mode = Mode::Insert;
            restore_selected_room_draft(app, store);
            tracing::info!("Contact search: switched to room at position {}", pos);
        } else {
            tracing::warn!("Contact search: room {} not found in room list", room_id);
        }
    }
}

async fn refresh_contact_results(
    app: &mut App,
    matrix_client: Option<&matrix_sdk::Client>,
) {
    if app.contact_search.len() >= 2 {
        if let Some(client) = matrix_client {
            app.contact_results =
                matrix::contacts::search_users(client, &app.contact_search).await;
            app.selected_contact = 0;
        }
    } else {
        app.contact_results.clear();
        app.selected_contact = 0;
    }
}

fn handle_command_key(app: &mut App, key: KeyEvent) {
    let Mode::Command(ref mut buf) = app.mode else {
        return;
    };

    match key.code {
        KeyCode::Char(c) => buf.push(c),
        KeyCode::Backspace => {
            buf.pop();
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            let command = buf.clone();
            app.mode = Mode::Normal;
            if command == "q" || command == "quit" {
                app.should_quit = true;
            }
        }
        _ => {}
    }
}

async fn handle_message_select_key(
    app: &mut App,
    key: KeyEvent,
    matrix_client: Option<&matrix_sdk::Client>,
    store: Option<&LocalStore>,
) {
    if app.confirm_delete {
        handle_delete_confirmation(app, key, matrix_client).await;
        return;
    }

    if should_ignore_vim_navigation_key(app, key) {
        return;
    }

    let action = map_key_to_action(key, &app.mode);
    let previous_room = app.selected_room;
    app.handle_action(action.clone());
    finalize_action(app, previous_room, &action, matrix_client, store).await;
}

async fn handle_normal_key(
    app: &mut App,
    key: KeyEvent,
    keymap: &mut KeymapManager,
    matrix_client: Option<&matrix_sdk::Client>,
    store: Option<&LocalStore>,
) {
    if should_ignore_vim_navigation_key(app, key) {
        return;
    }

    if let Some(action) = keymap.resolve(key, &app.mode) {
        let previous_room = app.selected_room;
        app.handle_action(action.clone());
        finalize_action(app, previous_room, &action, matrix_client, store).await;
    }
}

async fn handle_delete_confirmation(
    app: &mut App,
    key: KeyEvent,
    matrix_client: Option<&matrix_sdk::Client>,
) {
    match key.code {
        KeyCode::Char('y') => {
            delete_selected_message(app, matrix_client).await;
            app.confirm_delete = false;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.confirm_delete = false;
        }
        _ => {}
    }
}

async fn delete_selected_message(
    app: &mut App,
    matrix_client: Option<&matrix_sdk::Client>,
) {
    let Some(selected_idx) = app.selected_message else {
        return;
    };
    let Some(room_id) = app.rooms.get(app.selected_room).map(|room| room.id.clone()) else {
        return;
    };
    let Some(event_id) = app
        .messages
        .get(&room_id)
        .and_then(|msgs| msgs.get(selected_idx))
        .and_then(|msg| msg.event_id.clone())
    else {
        return;
    };
    let Some(client) = matrix_client else {
        return;
    };

    if let Ok(room_id) = matrix_sdk::ruma::RoomId::parse(&room_id) {
        if let Some(room) = client.get_room(&room_id) {
            if let Ok(event_id) = matrix_sdk::ruma::EventId::parse(&event_id) {
                match room.redact(&event_id, None, None).await {
                    Ok(_) => {
                        if let Some(msgs) = app.messages.get_mut(room.room_id().as_str()) {
                            msgs.remove(selected_idx);
                            if msgs.is_empty() {
                                app.selected_message = None;
                                app.mode = Mode::Normal;
                            } else {
                                app.selected_message = Some(selected_idx.min(msgs.len() - 1));
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

async fn submit_compose_buffer(
    app: &mut App,
    matrix_client: Option<&matrix_sdk::Client>,
    store: Option<&LocalStore>,
) {
    app.vigo_commit();
    let msg_text = app.textarea_text();
    if msg_text.is_empty() {
        return;
    }

    let Some(client) = matrix_client else {
        return;
    };
    let Some(room_id_str) = app.rooms.get(app.selected_room).map(|room| room.id.clone()) else {
        return;
    };
    let Ok(room_id) = matrix_sdk::ruma::RoomId::parse(&room_id_str) else {
        return;
    };
    let Some(room) = client.get_room(&room_id) else {
        return;
    };

    app.textarea_clear();
    let reply_ctx = app.reply_context.take();
    let edit_ctx = app.edit_context.take();

    if let Some(ref edit_ctx) = edit_ctx {
        if let Ok(original_event_id) = matrix_sdk::ruma::EventId::parse(&edit_ctx.event_id) {
            use matrix_sdk::ruma::events::room::message::{
                ReplacementMetadata,
                RoomMessageEventContentWithoutRelation,
            };

            let new_content =
                RoomMessageEventContentWithoutRelation::text_plain(&msg_text);
            let replacement = new_content
                .make_replacement(ReplacementMetadata::new(original_event_id, None));

            if let Err(e) = room.send(replacement).await {
                tracing::warn!("Failed to send edit: {}", e);
                app.edit_context = edit_ctx.clone().into();
                app.textarea.insert_str(&msg_text);
                persist_plain_message_draft(app, store);
            }
        }

        return;
    }

    let content = if let Some(ref reply_ctx) = reply_ctx {
        if let Ok(event_id) = matrix_sdk::ruma::EventId::parse(&reply_ctx.event_id) {
            use matrix_sdk::ruma::events::room::message::{
                AddMentions,
                ForwardThread,
                ReplyMetadata,
            };

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
        app.textarea.insert_str(&msg_text);
        persist_plain_message_draft(app, store);
    } else {
        delete_selected_room_draft(app, store);
    }
}

async fn finalize_action(
    app: &mut App,
    previous_room: usize,
    action: &Action,
    matrix_client: Option<&matrix_sdk::Client>,
    store: Option<&LocalStore>,
) {
    if matches!(action, Action::ModeInsert) {
        restore_selected_room_draft(app, store);
    }

    if app.selected_room != previous_room {
        record_last_read_position(app, previous_room);
    }

    if !app.send_read_receipts {
        return;
    }

    let Some(client) = matrix_client else {
        return;
    };

    if matches!(action, Action::MarkAllRead) {
        for room in &app.rooms {
            send_read_receipt(client, &room.id, &app.messages).await;
        }
        return;
    }

    if app.selected_room != previous_room
        || matches!(action, Action::MarkRead | Action::MarkAllRead)
    {
        maybe_send_read_receipt_for_selected_room(app, matrix_client).await;
    }
}

fn record_last_read_position(app: &mut App, room_index: usize) {
    if let Some(old_room) = app.rooms.get(room_index) {
        if let Some(msgs) = app.messages.get(&old_room.id) {
            app.last_read_index.insert(old_room.id.clone(), msgs.len());
        }
    }
}

async fn maybe_send_read_receipt_for_selected_room(
    app: &App,
    matrix_client: Option<&matrix_sdk::Client>,
) {
    let Some(room) = app.rooms.get(app.selected_room) else {
        return;
    };

    maybe_send_read_receipt_for_room(app, matrix_client, &room.id).await;
}

async fn maybe_send_read_receipt_for_room(
    app: &App,
    matrix_client: Option<&matrix_sdk::Client>,
    room_id: &str,
) {
    if !app.send_read_receipts {
        return;
    }

    if let Some(client) = matrix_client {
        send_read_receipt(client, room_id, &app.messages).await;
    }
}

fn handle_mouse_event(app: &mut App, mouse: crossterm::event::MouseEvent) {
    use crossterm::event::MouseEventKind;

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.scroll_offset = app.scroll_offset.saturating_add(3);
        }
        MouseEventKind::ScrollDown => {
            app.scroll_offset = app.scroll_offset.saturating_sub(3);
        }
        _ => {}
    }
}

fn handle_matrix_event(app: &mut App, matrix_event: MatrixEvent) {
    match matrix_event {
        MatrixEvent::RoomListUpdate(rooms) => {
            let current_room_id = current_room_id(app);
            app.rooms = rooms;
            sort_rooms_by_activity(app, current_room_id);
            app.connection_status = app::ConnectionStatus::Connected;
            tracing::debug!("Room list updated: {} rooms", app.rooms.len());
        }
        MatrixEvent::NewMessage { room_id, message } => {
            let msgs = app.messages.entry(room_id).or_default();
            let is_duplicate = message
                .event_id
                .as_ref()
                .is_some_and(|event_id| msgs.iter().any(|msg| msg.event_id.as_ref() == Some(event_id)));
            if is_duplicate {
                return;
            }

            msgs.push(message);
            let current_room_id = current_room_id(app);
            sort_rooms_by_activity(app, current_room_id);
        }
        MatrixEvent::MessageEdited {
            room_id,
            event_id,
            new_body,
        } => {
            if let Some(msgs) = app.messages.get_mut(&room_id) {
                if let Some(msg) = msgs
                    .iter_mut()
                    .find(|msg| msg.event_id.as_deref() == Some(&event_id))
                {
                    msg.body = new_body;
                }
            }
        }
        MatrixEvent::SyncError(err) => {
            tracing::warn!("Matrix sync error: {}", err);
            app.connection_status = app::ConnectionStatus::Disconnected;
        }
    }
}

fn sort_rooms_by_activity(app: &mut App, current_room_id: Option<String>) {
    let messages = &app.messages;
    app.rooms.sort_by(|a, b| {
        let a_ts = messages
            .get(&a.id)
            .and_then(|msgs| msgs.last().map(|msg| msg.timestamp))
            .or(a.last_activity);
        let b_ts = messages
            .get(&b.id)
            .and_then(|msgs| msgs.last().map(|msg| msg.timestamp))
            .or(b.last_activity);
        b_ts.cmp(&a_ts)
    });

    if let Some(room_id) = current_room_id {
        if let Some(pos) = app.rooms.iter().position(|room| room.id == room_id) {
            app.selected_room = pos;
        }
    } else if app.selected_room >= app.rooms.len() {
        app.selected_room = app.rooms.len().saturating_sub(1);
    }
}

fn current_room_id(app: &App) -> Option<String> {
    app.rooms.get(app.selected_room).map(|room| room.id.clone())
}

fn should_ignore_vim_navigation_key(app: &App, key: KeyEvent) -> bool {
    if app.vim_mode {
        return false;
    }

    matches!(
        (&app.mode, key.code),
        (Mode::Normal | Mode::MessageSelect, KeyCode::Char('j' | 'k' | 'g' | 'G'))
    )
}

fn save_vigo_preference(store: Option<&LocalStore>, enabled: bool) {
    if let Some(store) = store {
        if let Err(err) = store.save_preference("vigo_enabled", if enabled { "true" } else { "false" }) {
            tracing::warn!("Failed to save Vigo preference: {}", err);
        }
    }
}

fn restore_selected_room_draft(app: &mut App, store: Option<&LocalStore>) {
    if app.reply_context.is_some() || app.edit_context.is_some() || !app.textarea_text().is_empty() {
        return;
    }

    let Some(room_id) = current_room_id(app) else {
        return;
    };
    let Some(store) = store else {
        return;
    };

    match store.load_draft(&room_id) {
        Ok(Some(draft)) if !draft.is_empty() => {
            app.textarea_clear();
            app.textarea.insert_str(&draft);
        }
        Ok(_) => {}
        Err(err) => {
            tracing::warn!("Failed to restore draft for room {}: {}", room_id, err);
        }
    }
}

fn persist_plain_message_draft(app: &App, store: Option<&LocalStore>) {
    if app.reply_context.is_some() || app.edit_context.is_some() {
        return;
    }

    let Some(room_id) = current_room_id(app) else {
        return;
    };
    let Some(store) = store else {
        return;
    };

    let draft = app.textarea_text();
    let result = if draft.is_empty() {
        store.delete_draft(&room_id)
    } else {
        store.save_draft(&room_id, &draft)
    };

    if let Err(err) = result {
        tracing::warn!("Failed to persist draft for room {}: {}", room_id, err);
    }
}

fn persist_or_clear_selected_room_draft(app: &App, store: Option<&LocalStore>) {
    if app.reply_context.is_some() || app.edit_context.is_some() {
        delete_selected_room_draft(app, store);
    } else {
        persist_plain_message_draft(app, store);
    }
}

fn delete_selected_room_draft(app: &App, store: Option<&LocalStore>) {
    let Some(room_id) = current_room_id(app) else {
        return;
    };
    let Some(store) = store else {
        return;
    };

    if let Err(err) = store.delete_draft(&room_id) {
        tracing::warn!("Failed to delete draft for room {}: {}", room_id, err);
    }
}

fn clear_compose_state(app: &mut App) {
    app.vigo_commit();
    app.reply_context = None;
    app.edit_context = None;
    app.textarea_clear();
}

fn select_first_filtered_room(app: &mut App) {
    if let Some(&first) = app.filtered_room_indices().first() {
        app.selected_room = first;
    }
}

fn reset_contact_search(app: &mut App) {
    app.contact_search.clear();
    app.contact_results.clear();
    app.selected_contact = 0;
}

/// Load recent messages from a Matrix room into the app state.
async fn load_room_messages(app: &mut App, room: &matrix_sdk::Room, room_id: &str, own_user_id: Option<&str>) {
    use matrix_sdk::room::MessagesOptions;
    use matrix_sdk::ruma::events::AnySyncMessageLikeEvent;
    use matrix_sdk::ruma::events::AnySyncTimelineEvent;
    use matrix_sdk::ruma::events::room::message::MessageType;

    let mut options = MessagesOptions::backward();
    options.limit = 50u32.into();
    // Filter to only fetch message events so state events (member joins,
    // room config, etc.) don't consume the limit in bridged/group rooms.
    options.filter.types = Some(vec![
        "m.room.message".to_owned(),
        "m.room.encrypted".to_owned(),
    ]);

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
                            // Detect replacement (edit) relation.
                            if let Some(matrix_sdk::ruma::events::room::message::Relation::Replacement(replacement)) = &orig.content.relates_to {
                                let target_eid = replacement.event_id.to_string();
                                let new_body = match &replacement.new_content.msgtype {
                                    MessageType::Text(text) => text.body.clone(),
                                    MessageType::Notice(notice) => notice.body.clone(),
                                    MessageType::Emote(emote) => format!("* {}", emote.body),
                                    _ => continue,
                                };
                                if let Some(original) = messages.iter_mut().find(|m| m.event_id.as_deref() == Some(&target_eid)) {
                                    original.body = new_body;
                                }
                                continue;
                            }

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

/// Send a read receipt to the Matrix server for the last message in a room.
///
/// This tells the server the user has read up to this point, clearing the
/// unread count server-side so it persists across restarts.
async fn send_read_receipt(client: &matrix_sdk::Client, room_id: &str, messages: &std::collections::BTreeMap<String, Vec<Message>>) {
    // Find the last message with an event_id in this room.
    let last_event_id = messages
        .get(room_id)
        .and_then(|msgs| {
            msgs.iter()
                .rev()
                .find_map(|m| m.event_id.as_deref())
        });

    if let Some(eid) = last_event_id {
        if let Ok(event_id) = matrix_sdk::ruma::EventId::parse(eid) {
            if let Ok(rid) = matrix_sdk::ruma::RoomId::parse(room_id) {
                if let Some(room) = client.get_room(&rid) {
                    use matrix_sdk::ruma::api::client::receipt::create_receipt::v3::ReceiptType;
                    if let Err(e) = room.send_single_receipt(ReceiptType::Read, matrix_sdk::ruma::events::receipt::ReceiptThread::Unthreaded, event_id).await {
                        tracing::warn!("Failed to send read receipt: {}", e);
                    }
                }
            }
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
    fn test_normal_c_compose() {
        assert_eq!(map_key_to_action(key(KeyCode::Char('c')), &Mode::Normal), Action::ModeInsert);
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
