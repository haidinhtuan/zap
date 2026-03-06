use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Mode};
use crate::ui::theme;

/// Render context-sensitive keyboard hints at the bottom of the screen.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match &app.mode {
        Mode::Normal => {
            if app.vim_mode {
                " [c]/[i]compose  [/]search  [:]command  j/k:nav  q:quit"
            } else {
                " [c]/[i]compose  [/]search  [:]command  arrows:nav  q:quit"
            }
        }
        Mode::Insert => {
            " [Enter]send  [Shift+Enter]newline  [Ctrl+T]vn  [Esc]save  [Ctrl+X]clear"
        }
        Mode::MessageSelect => {
            if app.confirm_delete {
                " Delete message? y:yes  n:cancel"
            } else {
                if app.vim_mode {
                    " j/k:nav  r:reply  e:edit  d:delete  c/i:compose  Esc:back"
                } else {
                    " arrows:nav  r:reply  e:edit  d:delete  c/i:compose  Esc:back"
                }
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
        .style(Style::default().fg(theme::color(
            app,
            |colors| &colors.help_bar_fg,
            Color::DarkGray,
        )));

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
            content.contains("[c]/[i]compose"),
            "Normal help should contain '[c]/[i]compose', got: {}",
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
            content.contains("[Esc]save"),
            "Insert help should contain '[Esc]save', got: {}",
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
        assert!(
            content.contains("c/i:compose"),
            "MessageSelect help should contain 'c/i:compose', got: {}",
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

    #[test]
    fn test_help_bar_non_vim_mode_shows_arrows() {
        let mut app = App::new();
        app.vim_mode = false;
        let buf = render_help_bar(&app, 80);
        let content = buffer_content(&buf);
        assert!(content.contains("arrows:nav"), "Non-vim help should contain 'arrows:nav', got: {}", content);
    }
}
