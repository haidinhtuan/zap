use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, ConnectionStatus, Mode};

// Vibrant pastel palette (matching starship/tmux theme)
const BASE: Color = Color::Rgb(30, 30, 46);        // #1e1e2e
const MANTLE: Color = Color::Rgb(24, 24, 37);      // #181825
const GREEN: Color = Color::Rgb(189, 240, 185);    // #bdf0b9
const PEACH: Color = Color::Rgb(255, 198, 161);    // #ffc6a1
const YELLOW: Color = Color::Rgb(255, 240, 194);   // #fff0c2
const MAUVE: Color = Color::Rgb(164, 197, 255);    // #a4c5ff
const TEAL: Color = Color::Rgb(168, 240, 229);     // #a8f0e5
const BLUE: Color = Color::Rgb(164, 197, 255);     // #a4c5ff

// Powerline characters
const ROUND_RIGHT: &str = "\u{e0b4}"; // pill close
const ROUND_LEFT: &str = "\u{e0b6}";  // pill open

/// Render the top status bar with powerline-style mode badge.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let (mode_str, mode_color) = match &app.mode {
        Mode::Normal => (" NORMAL ", GREEN),
        Mode::Insert => (" COMPOSE ", PEACH),
        Mode::MessageSelect => (" SELECT ", YELLOW),
        Mode::Command(_) => (" COMMAND ", MAUVE),
        Mode::RoomFilter => (" FILTER ", TEAL),
        Mode::ContactSearch => (" SEARCH ", BLUE),
    };

    let conn_text = match &app.connection_status {
        ConnectionStatus::Connected => "Connected",
        ConnectionStatus::Connecting => "Connecting",
        ConnectionStatus::Disconnected => "Disconnected",
    };

    // Left side: [MODE](pill close)
    let left_spans = vec![
        // Mode pill
        Span::styled(
            mode_str,
            Style::default().fg(BASE).bg(mode_color).add_modifier(Modifier::BOLD),
        ),
        // Pill close
        Span::styled(
            ROUND_RIGHT,
            Style::default().fg(mode_color).bg(MANTLE),
        ),
    ];

    // Right side: (pill open)[connection status]
    let conn_text_full = format!(" {} ", conn_text);
    let right_spans = vec![
        // Pill open for connection
        Span::styled(
            ROUND_LEFT,
            Style::default().fg(MAUVE).bg(MANTLE),
        ),
        // Connection status
        Span::styled(
            conn_text_full.clone(),
            Style::default().fg(BASE).bg(MAUVE).add_modifier(Modifier::BOLD),
        ),
    ];

    // Calculate padding
    let left_len: usize = mode_str.len() + 1; // mode + pill close
    let right_len: usize = 1 + conn_text_full.len(); // pill open + text
    let padding = if area.width as usize > left_len + right_len {
        area.width as usize - left_len - right_len
    } else {
        1
    };

    let mut spans = left_spans;
    spans.push(Span::styled(
        " ".repeat(padding),
        Style::default().bg(MANTLE),
    ));
    spans.extend(right_spans);

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(Style::default().bg(MANTLE));

    frame.render_widget(bar, area);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render_status_bar(app: &App, width: u16) -> Buffer {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, width, 1);
                draw(frame, app, area);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn test_status_bar_shows_normal_mode() {
        let app = App::new();
        let buf = render_status_bar(&app, 80);
        let content: String = (0..buf.area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(content.contains("NORMAL"), "Status bar should contain 'NORMAL', got: {}", content);
    }

    #[test]
    fn test_status_bar_shows_insert_mode() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        let buf = render_status_bar(&app, 80);
        let content: String = (0..buf.area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(content.contains("COMPOSE"), "Status bar should contain 'COMPOSE', got: {}", content);
    }

    #[test]
    fn test_status_bar_shows_connected() {
        let mut app = App::new();
        app.connection_status = ConnectionStatus::Connected;
        let buf = render_status_bar(&app, 80);
        let content: String = (0..buf.area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(content.contains("Connected"), "Status bar should contain 'Connected', got: {}", content);
    }

    #[test]
    fn test_status_bar_shows_disconnected() {
        let app = App::new(); // default is Disconnected
        let buf = render_status_bar(&app, 80);
        let content: String = (0..buf.area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(content.contains("Disconnected"), "Status bar should contain 'Disconnected', got: {}", content);
    }
}
