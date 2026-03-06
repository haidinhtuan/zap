pub mod status_bar;
pub mod room_list;
pub mod message_view;
pub mod compose_bar;
pub mod help_bar;
pub mod theme;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use crate::app::App;

/// Render the full application layout into the given frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let vertical = if app.show_help_bar {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
    } else {
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
    };

    let (status_area, body_area, help_area) = if app.show_help_bar {
        let [status_area, body_area, help_area] = vertical.areas(frame.area());
        (status_area, body_area, Some(help_area))
    } else {
        let [status_area, body_area] = vertical.areas(frame.area());
        (status_area, body_area, None)
    };

    let [room_area, chat_area] = Layout::horizontal([
        Constraint::Length(app.room_list_width + 2), // + borders
        Constraint::Fill(1),
    ])
    .areas(body_area);

    // Split chat_area into message view and compose bar.
    // Compose height grows with textarea content (up to 6 lines total).
    let textarea_lines = if app.mode == crate::app::Mode::Insert {
        app.textarea.lines().len().max(1)
    } else {
        1
    };
    let context_extra = if app.reply_context.is_some() || app.edit_context.is_some() { 1 } else { 0 };
    let compose_height = ((textarea_lines as u16) + 2 + context_extra).min(6);
    let [message_area, compose_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(compose_height),
    ])
    .areas(chat_area);

    status_bar::draw(frame, app, status_area);
    room_list::draw(frame, app, room_area);
    message_view::draw(frame, app, message_area);
    compose_bar::draw(frame, app, compose_area);
    if let Some(help_area) = help_area {
        help_bar::draw(frame, app, help_area);
    }
}
