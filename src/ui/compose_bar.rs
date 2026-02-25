use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Mode};

/// Render the compose bar input area with mode-sensitive prefix and cursor.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::layout::{Constraint, Layout};

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // If there's a reply context, split area into reply preview + input.
    let (reply_area, input_area) = if app.reply_context.is_some() {
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
            .split(block.inner(area));
        (Some(chunks[0]), chunks[1])
    } else {
        (None, block.inner(area))
    };

    // Render the outer block.
    frame.render_widget(block, area);

    // Render reply preview line if active.
    if let Some(ref ctx) = app.reply_context {
        if let Some(reply_rect) = reply_area {
            let truncated_body: String = ctx.body.chars().take(40).collect();
            let reply_line = Line::from(vec![
                Span::styled(" \u{21a9} ", Style::default().fg(Color::Yellow)),
                Span::styled(&ctx.sender, Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!(": \"{}\"", truncated_body),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            frame.render_widget(Paragraph::new(reply_line), reply_rect);
        }
    }

    // Render input line.
    let (prefix, prefix_style) = match &app.mode {
        Mode::Normal => (">", Style::default().fg(Color::DarkGray)),
        Mode::Insert => (">>>", Style::default().fg(Color::Green)),
        Mode::MessageSelect => (">", Style::default().fg(Color::Yellow)),
        Mode::Command(_) => (":", Style::default().fg(Color::Yellow)),
    };

    let mut spans = vec![
        Span::styled(prefix, prefix_style),
        Span::raw(" "),
        Span::raw(&app.input_buffer),
    ];

    // Show a cursor block in Insert mode.
    if app.mode == Mode::Insert {
        spans.push(Span::styled("\u{2588}", Style::default().fg(Color::White)));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), input_area);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    fn render_compose_bar(app: &App, width: u16, height: u16) -> Buffer {
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

    #[test]
    fn test_compose_bar_normal_mode_prefix() {
        let app = App::new(); // Normal mode
        let buf = render_compose_bar(&app, 40, 3);
        let content = buffer_content(&buf);
        assert!(
            content.contains(">"),
            "Normal mode should show '>' prefix, got:\n{}",
            content
        );
    }

    #[test]
    fn test_compose_bar_insert_mode_prefix() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        let buf = render_compose_bar(&app, 40, 3);
        let content = buffer_content(&buf);
        assert!(
            content.contains(">>>"),
            "Insert mode should show '>>>' prefix, got:\n{}",
            content
        );
    }

    #[test]
    fn test_compose_bar_command_mode_prefix() {
        let mut app = App::new();
        app.mode = Mode::Command(String::new());
        let buf = render_compose_bar(&app, 40, 3);
        let content = buffer_content(&buf);
        assert!(
            content.contains(":"),
            "Command mode should show ':' prefix, got:\n{}",
            content
        );
    }

    #[test]
    fn test_compose_bar_shows_input_buffer() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        app.input_buffer = "hello world".to_string();
        let buf = render_compose_bar(&app, 40, 3);
        let content = buffer_content(&buf);
        assert!(
            content.contains("hello world"),
            "Should show input buffer content, got:\n{}",
            content
        );
    }

    #[test]
    fn test_compose_bar_shows_reply_preview() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        app.reply_context = Some(crate::app::ReplyContext {
            event_id: "$ev1".to_string(),
            sender: "alice".to_string(),
            body: "hello world".to_string(),
        });
        let buf = render_compose_bar(&app, 60, 4);
        let content = buffer_content(&buf);
        assert!(content.contains("alice"), "Should show reply sender, got:\n{}", content);
    }
}
