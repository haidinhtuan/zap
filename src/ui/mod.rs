pub mod status_bar;
pub mod room_list;
pub mod message_view;
pub mod compose_bar;
pub mod help_bar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use crate::app::App;

/// Render the full application layout into the given frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let [status_area, body_area, help_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [room_area, chat_area] = Layout::horizontal([
        Constraint::Length(22), // room list width + borders
        Constraint::Fill(1),
    ])
    .areas(body_area);

    // Split chat_area into message view and compose bar.
    let compose_height = if app.reply_context.is_some() || app.edit_context.is_some() { 4 } else { 3 };
    let [message_area, compose_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(compose_height),
    ])
    .areas(chat_area);

    status_bar::draw(frame, app, status_area);
    room_list::draw(frame, app, room_area);
    message_view::draw(frame, app, message_area);
    compose_bar::draw(frame, app, compose_area);
    help_bar::draw(frame, app, help_area);
}
