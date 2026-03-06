use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Mode};
use crate::ui::theme;

/// Render the compose bar input area with mode-sensitive prefix and cursor.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::layout::{Constraint, Layout};

    let border = theme::color(app, |colors| &colors.border, Color::DarkGray);
    let accent = theme::color(app, |colors| &colors.accent, Color::Yellow);
    let own = theme::color(app, |colors| &colors.my_message, Color::Green);
    let theirs = theme::color(app, |colors| &colors.their_message, Color::Cyan);
    let timestamp = theme::color(app, |colors| &colors.timestamp, Color::DarkGray);
    let fg = theme::color(app, |colors| &colors.fg, Color::White);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border));

    // If there's a reply or edit context, split area into context preview + input.
    let has_context = app.reply_context.is_some() || app.edit_context.is_some();
    let (context_area, input_area) = if has_context {
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
            .split(block.inner(area));
        (Some(chunks[0]), chunks[1])
    } else {
        (None, block.inner(area))
    };

    // Render the outer block.
    frame.render_widget(block, area);

    // Render edit indicator if active.
    if app.edit_context.is_some() {
        if let Some(ctx_rect) = context_area {
            let edit_line = Line::from(vec![
                Span::styled(" \u{270e} Editing message", Style::default().fg(accent)),
            ]);
            frame.render_widget(Paragraph::new(edit_line), ctx_rect);
        }
    }
    // Render reply preview line if active (and no edit context).
    else if let Some(ref ctx) = app.reply_context {
        if let Some(reply_rect) = context_area {
            let truncated_body: String = ctx.body.chars().take(40).collect();
            let reply_line = Line::from(vec![
                Span::styled(" \u{21a9} ", Style::default().fg(accent)),
                Span::styled(&ctx.sender, Style::default().fg(theirs)),
                Span::styled(
                    format!(": \"{}\"", truncated_body),
                    Style::default().fg(timestamp),
                ),
            ]);
            frame.render_widget(Paragraph::new(reply_line), reply_rect);
        }
    }

    // Render input line.
    let (prefix, prefix_style) = match &app.mode {
        Mode::Normal => (">", Style::default().fg(timestamp)),
        Mode::Insert => {
            if app.vigo_enabled {
                (">VI>", Style::default().fg(own))
            } else {
                (">EN>", Style::default().fg(own))
            }
        }
        Mode::MessageSelect => (">", Style::default().fg(accent)),
        Mode::Command(_) => (":", Style::default().fg(accent)),
        Mode::RoomFilter => ("/", Style::default().fg(accent)),
        Mode::ContactSearch => ("@", Style::default().fg(accent)),
    };

    if app.mode == Mode::Insert {
        // In Insert mode, show prefix then render TextArea widget.
        let prefix_width = prefix.len() as u16 + 1; // prefix + space
        let prefix_line = Line::from(vec![
            Span::styled(prefix, prefix_style),
            Span::raw(" "),
        ]);
        let prefix_area = Rect::new(input_area.x, input_area.y, prefix_width, 1);
        frame.render_widget(Paragraph::new(prefix_line), prefix_area);

        let ta_area = Rect::new(
            input_area.x + prefix_width,
            input_area.y,
            input_area.width.saturating_sub(prefix_width),
            input_area.height,
        );
        frame.render_widget(&app.textarea, ta_area);
    } else {
        // Non-insert modes: show static text.
        let display_text = match &app.mode {
            Mode::Command(buf) => buf.clone(),
            Mode::RoomFilter => app.room_filter.clone(),
            Mode::ContactSearch => app.contact_search.clone(),
            _ => String::new(),
        };
        let mut spans = vec![
            Span::styled(prefix, prefix_style),
            Span::raw(" "),
            Span::styled(display_text, Style::default().fg(fg)),
        ];
        // Show cursor in RoomFilter and ContactSearch modes.
        if matches!(app.mode, Mode::RoomFilter | Mode::ContactSearch) {
            spans.push(Span::styled("\u{2588}", Style::default().fg(fg)));
        }
        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line), input_area);
    }
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
        // Vigo (Vietnamese input) is enabled by default
        assert!(
            content.contains(">VI>"),
            "Insert mode should show '>VI>' prefix (Vigo enabled by default), got:\n{}",
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
        app.textarea.insert_str("hello world");
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

    #[test]
    fn test_compose_bar_shows_edit_preview() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        app.edit_context = Some(crate::app::EditContext {
            event_id: "$ev1".to_string(),
            room_id: "!room:example.com".to_string(),
        });
        app.textarea.insert_str("edited text");
        let buf = render_compose_bar(&app, 60, 4);
        let content = buffer_content(&buf);
        assert!(content.contains("Editing") || content.contains("edit"),
            "Should show edit indicator, got:\n{}", content);
    }
}
