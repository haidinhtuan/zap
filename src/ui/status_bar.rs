use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, ConnectionStatus, Mode};
use crate::ui::theme;

const DEFAULT_BG: Color = Color::Rgb(24, 24, 37);
const DEFAULT_FG: Color = Color::Rgb(205, 214, 244);
const DEFAULT_ACCENT: Color = Color::Rgb(137, 180, 250);

// Powerline characters
const ROUND_RIGHT: &str = "\u{e0b4}"; // pill close
const ROUND_LEFT: &str = "\u{e0b6}";  // pill open

/// Render the top status bar with powerline-style mode badge.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let bg = theme::color(app, |colors| &colors.status_bar_bg, DEFAULT_BG);
    let fg = theme::color(app, |colors| &colors.fg, DEFAULT_FG);
    let accent = theme::color(app, |colors| &colors.accent, DEFAULT_ACCENT);

    let (mode_str, mode_color) = match &app.mode {
        Mode::Normal => (" NORMAL ", accent),
        Mode::Insert => (" COMPOSE ", accent),
        Mode::MessageSelect => (" SELECT ", accent),
        Mode::Command(_) => (" COMMAND ", accent),
        Mode::RoomFilter => (" FILTER ", accent),
        Mode::ContactSearch => (" SEARCH ", accent),
    };

    let (conn_text, conn_color) = match &app.connection_status {
        ConnectionStatus::Connected => (
            "Connected",
            theme::color(app, |colors| &colors.my_message, Color::Green),
        ),
        ConnectionStatus::Connecting => ("Connecting", accent),
        ConnectionStatus::Disconnected => (
            "Disconnected",
            theme::color(app, |colors| &colors.unread_badge, Color::Red),
        ),
    };

    // Left side: [MODE](pill close)
    let left_spans = vec![
        // Mode pill
        Span::styled(
            mode_str,
            Style::default().fg(bg).bg(mode_color).add_modifier(Modifier::BOLD),
        ),
        // Pill close
        Span::styled(
            ROUND_RIGHT,
            Style::default().fg(mode_color).bg(bg),
        ),
    ];

    // Right side: (pill open)[connection status]
    let conn_text_full = format!(" {} ", conn_text);
    let right_spans = vec![
        // Pill open for connection
        Span::styled(
            ROUND_LEFT,
            Style::default().fg(conn_color).bg(bg),
        ),
        // Connection status
        Span::styled(
            conn_text_full.clone(),
            Style::default().fg(bg).bg(conn_color).add_modifier(Modifier::BOLD),
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
        Style::default().fg(fg).bg(bg),
    ));
    spans.extend(right_spans);

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(Style::default().fg(fg).bg(bg));

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
