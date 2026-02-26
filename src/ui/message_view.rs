use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Mode};

/// Render the message view area showing the room header and message list.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = match app.mode {
        Mode::MessageSelect => Color::Yellow,
        _ => Color::DarkGray,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

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

    // Fixed prefix width: marker(2) + timestamp(5) + space(1) = 8 chars.
    // Continuation lines are indented to this width so they don't overlap the timestamp column.
    let prefix_width = 8;
    let inner_width = area.width.saturating_sub(2) as usize; // subtract borders
    let inner_height = area.height.saturating_sub(2) as usize;
    let body_width = inner_width.saturating_sub(prefix_width);

    // Build lines for each message, manually wrapping long bodies.
    let mut lines: Vec<Line> = Vec::new();
    for (i, msg) in messages.iter().enumerate() {
        let timestamp = msg.timestamp.format("%H:%M").to_string();

        let is_selected = app.mode == Mode::MessageSelect && app.selected_message == Some(i);
        let is_delete_target = is_selected && app.confirm_delete;

        let sender_style = if is_delete_target {
            Style::default().fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
        } else if msg.is_own {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let body_style = if is_delete_target {
            Style::default().fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
        } else if msg.is_own {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        let timestamp_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);

        // Show reply indicator if this message is a reply.
        if let Some(ref reply_eid) = msg.reply_to {
            let prefix = if is_selected { ">>" } else { "  " };
            let reply_text = messages.iter()
                .find(|m| m.event_id.as_deref() == Some(reply_eid.as_str()))
                .map(|m| {
                    let body: String = m.body.chars().take(40).collect();
                    let name = if m.is_own { "You" } else { &m.sender };
                    format!("| {} : {}", name, body)
                })
                .unwrap_or_else(|| "| reply".to_string());
            lines.push(Line::from(vec![
                Span::raw(format!("{}     ", prefix)),
                Span::styled(
                    reply_text,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }

        let (marker, marker_style) = if is_delete_target {
            ("xx", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        } else if is_selected {
            (">>", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        } else {
            ("  ", Style::default())
        };
        let display_name = if msg.is_own { "You".to_string() } else { msg.sender.clone() };

        // Build the full body text: "SenderName: message body"
        let full_body = format!("{}: {}", display_name, msg.body);

        if body_width == 0 || full_body.len() <= body_width {
            // Fits on one line.
            lines.push(Line::from(vec![
                Span::styled(marker, marker_style),
                Span::styled(timestamp, timestamp_style),
                Span::raw(" "),
                Span::styled(display_name, sender_style),
                Span::raw(": "),
                Span::styled(msg.body.clone(), body_style),
            ]));
        } else {
            // First line: marker + timestamp + start of body.
            let first_chunk: String = full_body.chars().take(body_width).collect();
            lines.push(Line::from(vec![
                Span::styled(marker, marker_style),
                Span::styled(timestamp, timestamp_style),
                Span::raw(" "),
                Span::styled(first_chunk, if msg.is_own { sender_style } else { body_style }),
            ]));

            // Continuation lines: indented past the timestamp column.
            let indent = " ".repeat(prefix_width);
            let remaining: String = full_body.chars().skip(body_width).collect();
            let mut rest = remaining.as_str();
            while !rest.is_empty() {
                let chunk_len = rest.chars().take(body_width).count();
                let chunk: String = rest.chars().take(chunk_len).collect();
                rest = &rest[chunk.len()..];
                lines.push(Line::from(vec![
                    Span::raw(indent.clone()),
                    Span::styled(chunk, body_style),
                ]));
            }
        }
    }

    // Scroll so the newest messages (bottom) are visible, then apply user scroll offset.
    let total_lines = lines.len();
    let auto_scroll = total_lines.saturating_sub(inner_height);
    let scroll_offset = auto_scroll.saturating_sub(app.scroll_offset);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Message, Mode, Room};
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
            is_direct: false,
        };
        app.rooms.push(room);
        app.selected_room = 0;

        let mut messages = BTreeMap::new();
        messages.insert(
            "!room0:example.com".to_string(),
            vec![
                Message {
                    event_id: None,
                    sender: "alice".to_string(),
                    body: "Hello world".to_string(),
                    timestamp: Utc.with_ymd_and_hms(2025, 1, 15, 14, 0, 0).unwrap(),
                    is_own: true,
                    reply_to: None,
                },
                Message {
                    event_id: None,
                    sender: "bob".to_string(),
                    body: "Hi there".to_string(),
                    timestamp: Utc.with_ymd_and_hms(2025, 1, 15, 14, 5, 0).unwrap(),
                    is_own: false,
                    reply_to: None,
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
            content.contains("You"),
            "Should show 'You' for own messages instead of sender name, got:\n{}",
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
            is_direct: false,
        };
        app.rooms.push(room);
        app.selected_room = 0;

        let mut messages = BTreeMap::new();
        messages.insert(
            "!room0:example.com".to_string(),
            vec![Message {
                event_id: None,
                sender: "trang".to_string(),
                body: "Xin ch\u{00e0}o b\u{1ea1}n".to_string(),
                timestamp: Utc.with_ymd_and_hms(2025, 1, 15, 14, 0, 0).unwrap(),
                is_own: false,
                reply_to: None,
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
            is_direct: false,
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

    #[test]
    fn test_message_view_shows_selection_marker() {
        let mut app = make_app_with_messages();
        app.mode = Mode::MessageSelect;
        app.selected_message = Some(1);
        let buf = render_message_view(&app, 60, 10);
        let content = buffer_content(&buf);
        assert!(content.contains(">>"), "Should show '>>' marker, got:\n{}", content);
    }
}
