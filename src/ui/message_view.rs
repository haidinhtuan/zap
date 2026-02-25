use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the message view area showing the room header and message list.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // If no room is selected, show a placeholder.
    let current_room = app.rooms.get(app.selected_room);
    if current_room.is_none() {
        let empty = Paragraph::new("No messages")
            .alignment(Alignment::Center)
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let room = current_room.unwrap();

    // Build room header title.
    let activity_str = match &room.last_activity {
        Some(dt) => dt.format("%H:%M").to_string(),
        None => String::new(),
    };

    let title = if activity_str.is_empty() {
        format!(" {} ", room.name)
    } else {
        format!(" {} \u{2022} {} ", room.name, activity_str)
    };

    let block = block.title(title);

    // Look up messages for this room.
    let messages = app.messages.get(&room.id);

    if messages.is_none() || messages.unwrap().is_empty() {
        let empty = Paragraph::new("No messages")
            .alignment(Alignment::Center)
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let messages = messages.unwrap();

    // Build lines for each message.
    let mut lines: Vec<Line> = Vec::new();
    for msg in messages {
        let timestamp = msg.timestamp.format("%H:%M").to_string();

        let sender_style = if msg.is_own {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let body_style = if msg.is_own {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let timestamp_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);

        lines.push(Line::from(vec![
            Span::styled(timestamp, timestamp_style),
            Span::raw(" "),
            Span::styled(msg.sender.clone(), sender_style),
            Span::raw(": "),
            Span::styled(msg.body.clone(), body_style),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Message, Room};
    use chrono::{TimeZone, Utc};
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::Terminal;
    use std::collections::BTreeMap;

    fn render_message_view(app: &App, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, width, height);
                draw(frame, app, area);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_content(buf: &Buffer) -> String {
        let mut content = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                content.push_str(buf.cell((x, y)).unwrap().symbol());
            }
            content.push('\n');
        }
        content
    }

    fn make_app_with_messages() -> App {
        let mut app = App::new();
        let room = Room {
            id: "!room0:example.com".to_string(),
            name: "General".to_string(),
            unread_count: 0,
            last_activity: Some(Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 0).unwrap()),
        };
        app.rooms.push(room);
        app.selected_room = 0;

        let mut messages = BTreeMap::new();
        messages.insert(
            "!room0:example.com".to_string(),
            vec![
                Message {
                    sender: "alice".to_string(),
                    body: "Hello world".to_string(),
                    timestamp: Utc.with_ymd_and_hms(2025, 1, 15, 14, 0, 0).unwrap(),
                    is_own: true,
                },
                Message {
                    sender: "bob".to_string(),
                    body: "Hi there".to_string(),
                    timestamp: Utc.with_ymd_and_hms(2025, 1, 15, 14, 5, 0).unwrap(),
                    is_own: false,
                },
            ],
        );
        app.messages = messages;
        app
    }

    #[test]
    fn test_message_view_no_rooms() {
        let app = App::new();
        let buf = render_message_view(&app, 60, 10);
        let content = buffer_content(&buf);
        assert!(
            content.contains("No messages"),
            "Should show 'No messages' when no rooms, got:\n{}",
            content
        );
    }

    #[test]
    fn test_message_view_shows_messages() {
        let app = make_app_with_messages();
        let buf = render_message_view(&app, 60, 10);
        let content = buffer_content(&buf);
        assert!(
            content.contains("Hello world"),
            "Should show message body 'Hello world', got:\n{}",
            content
        );
        assert!(
            content.contains("Hi there"),
            "Should show message body 'Hi there', got:\n{}",
            content
        );
    }

    #[test]
    fn test_message_view_shows_senders() {
        let app = make_app_with_messages();
        let buf = render_message_view(&app, 60, 10);
        let content = buffer_content(&buf);
        assert!(
            content.contains("alice"),
            "Should show sender 'alice', got:\n{}",
            content
        );
        assert!(
            content.contains("bob"),
            "Should show sender 'bob', got:\n{}",
            content
        );
    }

    #[test]
    fn test_message_view_shows_room_name() {
        let app = make_app_with_messages();
        let buf = render_message_view(&app, 60, 10);
        let content = buffer_content(&buf);
        assert!(
            content.contains("General"),
            "Should show room name 'General', got:\n{}",
            content
        );
    }

    #[test]
    fn test_message_view_unicode_vietnamese() {
        let mut app = App::new();
        let room = Room {
            id: "!room0:example.com".to_string(),
            name: "Vietnamese".to_string(),
            unread_count: 0,
            last_activity: None,
        };
        app.rooms.push(room);
        app.selected_room = 0;

        let mut messages = BTreeMap::new();
        messages.insert(
            "!room0:example.com".to_string(),
            vec![Message {
                sender: "trang".to_string(),
                body: "Xin ch\u{00e0}o b\u{1ea1}n".to_string(),
                timestamp: Utc.with_ymd_and_hms(2025, 1, 15, 14, 0, 0).unwrap(),
                is_own: false,
            }],
        );
        app.messages = messages;

        let buf = render_message_view(&app, 60, 10);
        let content = buffer_content(&buf);

        // Check that the Unicode content renders. The buffer may split multi-byte
        // characters, so check for substrings that should appear.
        assert!(
            content.contains("Xin"),
            "Should render Vietnamese text starting with 'Xin', got:\n{}",
            content
        );
        assert!(
            content.contains("trang"),
            "Should show sender 'trang', got:\n{}",
            content
        );
    }

    #[test]
    fn test_message_view_empty_messages_for_room() {
        let mut app = App::new();
        let room = Room {
            id: "!room0:example.com".to_string(),
            name: "Empty".to_string(),
            unread_count: 0,
            last_activity: None,
        };
        app.rooms.push(room);
        app.selected_room = 0;
        // No messages inserted for this room.

        let buf = render_message_view(&app, 60, 10);
        let content = buffer_content(&buf);
        assert!(
            content.contains("No messages"),
            "Should show 'No messages' for room with no messages, got:\n{}",
            content
        );
    }
}
