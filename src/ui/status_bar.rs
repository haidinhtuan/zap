use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, ConnectionStatus, Mode};

/// Render the top status bar showing the app name, current mode, and connection status.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mode_str = match &app.mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
        Mode::Command(_) => "COMMAND",
    };

    let (conn_symbol, conn_text, conn_color) = match &app.connection_status {
        ConnectionStatus::Connected => ("\u{25c6}", "Connected", Color::Green),
        ConnectionStatus::Connecting => ("\u{25c7}", "Connecting", Color::Yellow),
        ConnectionStatus::Disconnected => ("\u{25c7}", "Disconnected", Color::Red),
    };

    let left = Span::styled(
        "\u{26a1} Zap",
        Style::default().fg(Color::Cyan),
    );

    let right_text = format!(" {} {} {} ", mode_str, conn_symbol, conn_text);
    let right = Span::styled(right_text, Style::default().fg(conn_color));

    // Calculate padding to right-align the status info.
    let left_len = 5; // "⚡ Zap" display width (emoji + space + 3 chars)
    let right_len = right.content.len();
    let padding = if area.width as usize > left_len + right_len {
        area.width as usize - left_len - right_len
    } else {
        1
    };

    let line = Line::from(vec![
        left,
        Span::raw(" ".repeat(padding)),
        right,
    ]);

    let bar = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));

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
    fn test_status_bar_contains_zap() {
        let app = App::new();
        let buf = render_status_bar(&app, 80);
        let content: String = (0..buf.area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(content.contains("Zap"), "Status bar should contain 'Zap', got: {}", content);
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
        assert!(content.contains("INSERT"), "Status bar should contain 'INSERT', got: {}", content);
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
