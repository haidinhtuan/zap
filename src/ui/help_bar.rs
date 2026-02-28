use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Mode};

/// Render context-sensitive keyboard hints at the bottom of the screen.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match &app.mode {
        Mode::Normal => {
            " [c]ompose  [/]search  [:]command  j/k:nav  q:quit"
        }
        Mode::Insert => {
            " [Enter]send  [Shift+Enter]newline  [Ctrl+T]vn  [Esc]back"
        }
        Mode::MessageSelect => {
            if app.confirm_delete {
                " Delete message? y:yes  n:cancel"
            } else {
                " j/k:nav  r:reply  e:edit  d:delete  c:compose  Esc:back"
            }
        }
        Mode::Command(_) => {
            " [Enter]execute  [Esc]cancel"
        }
        Mode::RoomFilter => {
            " Type to filter  @:DMs  #:groups  [Enter]select  [Esc]cancel"
        }
        Mode::ContactSearch => {
            " Type to search contacts  [Enter]open DM  [Esc]cancel"
        }
    };

    let bar = Paragraph::new(Line::raw(help_text))
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(bar, area);
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

    fn render_help_bar(app: &App, width: u16) -> Buffer {
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

    fn buffer_content(buf: &Buffer) -> String {
        (0..buf.area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect()
    }

    #[test]
    fn test_help_bar_normal_mode() {
        let app = App::new();
        let buf = render_help_bar(&app, 80);
        let content = buffer_content(&buf);
        assert!(
            content.contains("[c]ompose"),
            "Normal help should contain '[c]ompose', got: {}",
            content
        );
        assert!(
            content.contains("q:quit"),
            "Normal help should contain 'q:quit', got: {}",
            content
        );
        assert!(
            content.contains("j/k:nav"),
            "Normal help should contain 'j/k:nav', got: {}",
            content
        );
    }

    #[test]
    fn test_help_bar_insert_mode() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        let buf = render_help_bar(&app, 80);
        let content = buffer_content(&buf);
        assert!(
            content.contains("[Enter]send"),
            "Insert help should contain '[Enter]send', got: {}",
            content
        );
        assert!(
            content.contains("[Esc]back"),
            "Insert help should contain '[Esc]back', got: {}",
            content
        );
    }

    #[test]
    fn test_help_bar_command_mode() {
        let mut app = App::new();
        app.mode = Mode::Command(String::new());
        let buf = render_help_bar(&app, 80);
        let content = buffer_content(&buf);
        assert!(
            content.contains("[Enter]execute"),
            "Command help should contain '[Enter]execute', got: {}",
            content
        );
        assert!(
            content.contains("[Esc]cancel"),
            "Command help should contain '[Esc]cancel', got: {}",
            content
        );
    }

    #[test]
    fn test_help_bar_no_zap_branding() {
        let app = App::new();
        let buf = render_help_bar(&app, 80);
        let content = buffer_content(&buf);
        assert!(
            !content.contains("zap"),
            "Help bar should NOT contain 'zap' branding, got: {}",
            content
        );
    }

    #[test]
    fn test_help_bar_message_select_shows_edit() {
        let mut app = App::new();
        app.mode = Mode::MessageSelect;
        let buf = render_help_bar(&app, 80);
        let content = buffer_content(&buf);
        assert!(
            content.contains("e:edit"),
            "MessageSelect help should contain 'e:edit', got: {}",
            content
        );
    }

    #[test]
    fn test_help_bar_room_filter_mode() {
        let mut app = App::new();
        app.mode = Mode::RoomFilter;
        let buf = render_help_bar(&app, 80);
        let content = buffer_content(&buf);
        assert!(
            content.contains("filter"),
            "RoomFilter help should contain 'filter', got: {}",
            content
        );
    }
}
