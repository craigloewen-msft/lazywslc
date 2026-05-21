use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn draw_logs_tab(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Logs ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.logs_text.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No logs available.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, inner);
        return;
    }

    let lines: Vec<Line> = sanitize_output(&app.logs_text)
        .lines()
        .map(|l| Line::from(Span::raw(format!("  {}", l))))
        .collect();

    let total = lines.len() as u16;
    let visible = inner.height;
    let max_scroll = total.saturating_sub(visible);

    // Auto-scroll to bottom unless user has manually scrolled up
    let scroll = if app.logs_scroll == 0 {
        max_scroll // default: show latest logs at bottom
    } else {
        app.logs_scroll.min(max_scroll)
    };

    let paragraph = Paragraph::new(lines).scroll((scroll, 0));
    f.render_widget(paragraph, inner);
}

/// Strip ANSI escape sequences and control characters that corrupt TUI rendering.
pub fn sanitize_output(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // Strip ANSI escape sequences: ESC [ ... final_byte
            '\x1b' => {
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    // consume until we hit a letter (the final byte of the sequence)
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next.is_ascii_alphabetic() || next == '~' {
                            break;
                        }
                    }
                } else if chars.peek() == Some(&']') {
                    // OSC sequence: ESC ] ... ST (or BEL)
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next == '\x07' || next == '\\' {
                            break;
                        }
                    }
                }
                // else ignore lone ESC
            }
            // Carriage return: take only the last segment (mimics terminal \r overwrite)
            '\r' => {
                // Find the last \r-separated segment on this line by discarding what came before
                if chars.peek() != Some(&'\n') {
                    // Find end of the current content up to the last \r or \n
                    let rest_of_line: String = chars.by_ref().take_while(|&ch| ch != '\n' && ch != '\r').collect();
                    // Pop back to the start of the current line in result
                    while result.ends_with(|c: char| c != '\n') {
                        result.pop();
                    }
                    result.push_str(&rest_of_line);
                }
                // \r\n is just a newline
            }
            '\n' => result.push('\n'),
            // Strip other control characters except tab
            c if c.is_control() && c != '\t' => {}
            c => result.push(c),
        }
    }

    result
}
