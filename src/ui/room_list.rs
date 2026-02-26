use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, Mode};

/// Render the room list panel with selection highlight and unread badges.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = match app.mode {
        Mode::Normal | Mode::RoomFilter => Color::Blue,
        _ => Color::DarkGray,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Rooms ");

    if app.rooms.is_empty() {
        let empty = Paragraph::new("No rooms")
            .alignment(Alignment::Center)
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let filtered_indices = app.filtered_room_indices();

    let items: Vec<ListItem> = filtered_indices
        .iter()
        .map(|&idx| {
            let room = &app.rooms[idx];
            let mut spans = Vec::new();

            // Unread indicator.
            if room.unread_count > 0 {
                spans.push(Span::styled(
                    "\u{25cf}",
                    Style::default().fg(Color::Red),
                ));
            } else {
                spans.push(Span::raw(" "));
            }

            // DM vs group indicator.
            let type_indicator = if room.is_direct { "@ " } else { "# " };
            let type_color = if room.is_direct { Color::Green } else { Color::DarkGray };
            spans.push(Span::styled(type_indicator, Style::default().fg(type_color)));

            // Room name.
            let name_style = if room.unread_count > 0 {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            spans.push(Span::styled(&room.name, name_style));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    let selected_in_filtered = filtered_indices.iter().position(|&i| i == app.selected_room);
    state.select(selected_in_filtered);

    frame.render_stateful_widget(list, area, &mut state);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Room};
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    fn render_room_list(app: &App, width: u16, height: u16) -> Buffer {
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

    fn make_rooms() -> Vec<Room> {
        vec![
            Room {
                id: "!room0:example.com".to_string(),
                name: "General".to_string(),
                unread_count: 0,
                last_activity: None,
                is_direct: false,
            },
            Room {
                id: "!room1:example.com".to_string(),
                name: "Random".to_string(),
                unread_count: 3,
                last_activity: None,
                is_direct: false,
            },
            Room {
                id: "!room2:example.com".to_string(),
                name: "Dev".to_string(),
                unread_count: 0,
                last_activity: None,
                is_direct: false,
            },
        ]
    }

    #[test]
    fn test_room_list_no_rooms() {
        let app = App::new();
        let buf = render_room_list(&app, 22, 10);
        let content = buffer_content(&buf);
        assert!(content.contains("No rooms"), "Should show 'No rooms' when empty, got:\n{}", content);
    }

    #[test]
    fn test_room_list_shows_rooms() {
        let mut app = App::new();
        app.rooms = make_rooms();
        let buf = render_room_list(&app, 22, 10);
        let content = buffer_content(&buf);
        assert!(content.contains("General"), "Should show room 'General', got:\n{}", content);
        assert!(content.contains("Random"), "Should show room 'Random', got:\n{}", content);
        assert!(content.contains("Dev"), "Should show room 'Dev', got:\n{}", content);
    }

    #[test]
    fn test_room_list_shows_title() {
        let mut app = App::new();
        app.rooms = make_rooms();
        let buf = render_room_list(&app, 22, 10);
        let content = buffer_content(&buf);
        assert!(content.contains("Rooms"), "Should show title 'Rooms', got:\n{}", content);
    }

    #[test]
    fn test_room_list_unread_badge() {
        let mut app = App::new();
        app.rooms = make_rooms();
        let buf = render_room_list(&app, 22, 10);
        let content = buffer_content(&buf);
        // "Random" has unread_count=3, so should show the unread dot indicator
        assert!(content.contains("●"), "Should show unread dot '●' for Random, got:\n{}", content);
    }

    #[test]
    fn test_room_list_selected_room() {
        let mut app = App::new();
        app.rooms = make_rooms();
        app.selected_room = 1; // "Random" should be selected
        let buf = render_room_list(&app, 22, 10);
        let content = buffer_content(&buf);
        // We can't easily check background color in content string, but we verify the room is rendered
        assert!(content.contains("Random"), "Selected room 'Random' should be visible, got:\n{}", content);
    }
}
