use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Mode};
use crate::ui::theme;

fn date_label(date: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    let today = chrono::Local::now().date_naive();
    let yesterday = today - chrono::Duration::days(1);
    if date == today {
        "Today".to_string()
    } else if date == yesterday {
        "Yesterday".to_string()
    } else if date.year() == today.year() {
        date.format("%b %-d").to_string()
    } else {
        date.format("%b %-d, %Y").to_string()
    }
}

fn word_wrap(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.is_empty() {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if remaining.chars().count() <= max_width {
            lines.push(remaining.to_string());
            break;
        }
        let boundary = remaining
            .char_indices()
            .nth(max_width)
            .map(|(i, _)| i)
            .unwrap_or(remaining.len());
        let chunk = &remaining[..boundary];
        // If the char right at the boundary is a space, break cleanly there.
        if remaining[boundary..].starts_with(' ') {
            lines.push(chunk.to_string());
            remaining = &remaining[boundary + 1..];
        } else if let Some(last_space) = chunk.rfind(' ') {
            lines.push(remaining[..last_space].to_string());
            remaining = &remaining[last_space + 1..];
        } else {
            lines.push(chunk.to_string());
            remaining = &remaining[boundary..];
        }
    }
    lines
}

fn format_timestamp(app: &App, timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    timestamp
        .with_timezone(&chrono::Local)
        .format(&app.timestamp_format)
        .to_string()
}

/// Render the message view area showing the room header and message list.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let border = theme::color(app, |colors| &colors.border, Color::DarkGray);
    let accent = theme::color(app, |colors| &colors.accent, Color::Yellow);
    let own = theme::color(app, |colors| &colors.my_message, Color::Green);
    let theirs = theme::color(app, |colors| &colors.their_message, Color::Cyan);
    let timestamp_color = theme::color(app, |colors| &colors.timestamp, Color::DarkGray);
    let fg = theme::color(app, |colors| &colors.fg, Color::White);

    let border_color = match app.mode {
        Mode::MessageSelect => accent,
        _ => border,
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
        Some(dt) => dt.with_timezone(&chrono::Local).format(&app.timestamp_format).to_string(),
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

    // Prefix width is marker(2) + longest visible timestamp + space(1).
    // Continuation lines are indented to this width so they don't overlap the timestamp column.
    let timestamp_width = messages
        .iter()
        .map(|msg| format_timestamp(app, &msg.timestamp).chars().count())
        .max()
        .unwrap_or(5);
    let prefix_width = 2 + timestamp_width + 1;
    let inner_width = area.width.saturating_sub(2) as usize; // subtract borders
    let inner_height = area.height.saturating_sub(2) as usize;
    let body_width = inner_width.saturating_sub(prefix_width);

    // Build lines for each message, manually wrapping long bodies.
    let mut lines: Vec<Line> = Vec::new();
    let mut prev_date: Option<chrono::NaiveDate> = None;
    for (i, msg) in messages.iter().enumerate() {
        // Date separator between messages from different calendar days.
        let msg_date = msg.timestamp.with_timezone(&chrono::Local).date_naive();
        if prev_date != Some(msg_date) {
            let label = format!(" {} ", date_label(msg_date));
            let dashes_total = inner_width.saturating_sub(label.len());
            let left = dashes_total / 2;
            let right = dashes_total - left;
            let sep = format!("{}{}{}", "─".repeat(left), label, "─".repeat(right));
            lines.push(Line::from(Span::styled(
                sep,
                Style::default().fg(timestamp_color),
            )));
            prev_date = Some(msg_date);
        }

        // Render "new" separator between last-read and first-unread message.
        if let Some(&read_idx) = app.last_read_index.get(&room.id) {
            if i == read_idx && read_idx < messages.len() {
                let label = " new ";
                let dashes = inner_width.saturating_sub(label.len()) / 2;
                let sep = format!(
                    "{}{}{}",
                    "\u{2500}".repeat(dashes),
                    label,
                    "\u{2500}".repeat(inner_width.saturating_sub(dashes + label.len()))
                );
                lines.push(Line::from(Span::styled(
                    sep,
                    Style::default().fg(accent),
                )));
            }
        }

        let timestamp = format!("{:>width$}", format_timestamp(app, &msg.timestamp), width = timestamp_width);

        let is_selected = app.mode == Mode::MessageSelect && app.selected_message == Some(i);
        let is_delete_target = is_selected && app.confirm_delete;

        let sender_style = if is_delete_target {
            Style::default().fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
        } else if msg.is_own {
            Style::default().fg(own)
        } else {
            Style::default().fg(theirs)
        };

        let body_style = if is_delete_target {
            Style::default().fg(Color::Red).add_modifier(Modifier::CROSSED_OUT)
        } else if msg.is_own {
            Style::default().fg(own)
        } else {
            Style::default().fg(fg)
        };

        let timestamp_style = Style::default()
            .fg(timestamp_color)
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
                Span::raw(format!(
                    "{}{}",
                    prefix,
                    " ".repeat(prefix_width.saturating_sub(2))
                )),
                Span::styled(
                    reply_text,
                    Style::default()
                        .fg(timestamp_color)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }

        let (marker, marker_style) = if is_delete_target {
            ("xx", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        } else if is_selected {
            (">>", Style::default().fg(accent).add_modifier(Modifier::BOLD))
        } else {
            ("  ", Style::default())
        };
        let display_name = if msg.is_own { "You".to_string() } else { msg.sender.clone() };

        // Build the full body text: "SenderName: message body"
        let full_body = format!("{}: {}", display_name, msg.body);

        if body_width == 0 || full_body.chars().count() <= body_width {
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
            let wrapped = word_wrap(&full_body, body_width);
            let indent = " ".repeat(prefix_width);
            for (line_idx, chunk) in wrapped.iter().enumerate() {
                if line_idx == 0 {
                    lines.push(Line::from(vec![
                        Span::styled(marker, marker_style),
                        Span::styled(timestamp.clone(), timestamp_style),
                        Span::raw(" "),
                        Span::styled(
                            chunk.clone(),
                            if msg.is_own { sender_style } else { body_style },
                        ),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw(indent.clone()),
                        Span::styled(chunk.clone(), body_style),
                    ]));
                }
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

    #[test]
    fn test_word_wrap_short_text() {
        let result = word_wrap("hello world", 20);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn test_word_wrap_exact_width() {
        let result = word_wrap("hello", 5);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn test_word_wrap_breaks_at_space() {
        let result = word_wrap("hello world foo", 11);
        assert_eq!(result, vec!["hello world", "foo"]);
    }

    #[test]
    fn test_word_wrap_long_word_force_break() {
        let result = word_wrap("abcdefghij", 5);
        assert_eq!(result, vec!["abcde", "fghij"]);
    }

    #[test]
    fn test_word_wrap_multiple_lines() {
        let result = word_wrap("the quick brown fox jumps over", 10);
        assert_eq!(result, vec!["the quick", "brown fox", "jumps over"]);
    }

    #[test]
    fn test_word_wrap_empty() {
        let result = word_wrap("", 10);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn test_word_wrap_zero_width() {
        let result = word_wrap("hello", 0);
        assert_eq!(result, vec!["hello"]);
    }
}
