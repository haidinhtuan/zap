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

use app::{Action, App, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use event::{Event, EventHandler};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

/// Map a key event to an application action based on the current mode.
pub fn map_key_to_action(key: KeyEvent, mode: &Mode) -> Action {
    match mode {
        Mode::Normal => match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('j') => Action::RoomNext,
            KeyCode::Char('k') => Action::RoomPrev,
            KeyCode::Char('i') => Action::ModeInsert,
            KeyCode::Char(':') => Action::ModeCommand,
            KeyCode::Char('G') => Action::RoomLast,
            KeyCode::Char('/') => Action::RoomFilter,
            KeyCode::Char('r') => Action::MarkRead,
            KeyCode::Char('R') => Action::MarkAllRead,
            KeyCode::Enter => Action::OpenRoom,
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::ScrollUp,
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::ScrollDown,
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
/// Processes terminal events, dispatches actions, and renders the UI.
/// The `matrix_rx` channel is a placeholder for future Matrix sync events.
pub async fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    events: &mut EventHandler,
    matrix_rx: &mut mpsc::UnboundedReceiver<String>,
) -> color_eyre::Result<()> {
    loop {
        tokio::select! {
            event = events.next() => {
                match event? {
                    Event::Key(key) => {
                        let action = map_key_to_action(key, &app.mode);
                        app.handle_action(action);
                    }
                    Event::Render => {
                        terminal.draw(|frame| {
                            ui::draw(frame, app);
                        })?;
                    }
                    Event::Tick => {
                        // Placeholder: periodic housekeeping.
                    }
                    _ => {}
                }
            }
            Some(_msg) = matrix_rx.recv() => {
                // Placeholder: handle incoming Matrix messages.
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
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
