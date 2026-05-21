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

/// Strip ALL terminal escape sequences and control characters.
/// Handles: CSI (ESC[), OSC (ESC]), DCS (ESCP), SOS/PM/APC, SS2/SS3,
/// single-character escapes (ESC followed by one char), bare \x9B CSI,
/// \r overwrite behavior, and all C0/C1 control characters.
pub fn sanitize_output(input: &str) -> String {
    // First pass: strip all escape sequences
    let stripped = strip_escapes(input);
    // Second pass: handle \r (carriage return) line overwrites
    handle_carriage_returns(&stripped)
}

fn strip_escapes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        match b {
            // ESC (0x1B) — start of escape sequence
            0x1B => {
                i += 1;
                if i >= len { break; }
                match bytes[i] {
                    // CSI: ESC [ ... final_byte (0x40-0x7E)
                    b'[' => {
                        i += 1;
                        while i < len && !(0x40..=0x7E).contains(&bytes[i]) {
                            i += 1;
                        }
                        if i < len { i += 1; } // skip final byte
                    }
                    // OSC: ESC ] ... (terminated by BEL \x07 or ST = ESC \)
                    b']' => {
                        i += 1;
                        while i < len {
                            if bytes[i] == 0x07 { i += 1; break; }
                            if bytes[i] == 0x1B && i + 1 < len && bytes[i + 1] == b'\\' {
                                i += 2; break;
                            }
                            i += 1;
                        }
                    }
                    // DCS: ESC P ... ST
                    b'P' => {
                        i += 1;
                        while i < len {
                            if bytes[i] == 0x1B && i + 1 < len && bytes[i + 1] == b'\\' {
                                i += 2; break;
                            }
                            i += 1;
                        }
                    }
                    // SOS, PM, APC: ESC X/^/_ ... ST
                    b'X' | b'^' | b'_' => {
                        i += 1;
                        while i < len {
                            if bytes[i] == 0x1B && i + 1 < len && bytes[i + 1] == b'\\' {
                                i += 2; break;
                            }
                            i += 1;
                        }
                    }
                    // SS2/SS3: ESC N/O + one character
                    b'N' | b'O' => {
                        i += 1;
                        if i < len { i += 1; }
                    }
                    // Any other single-char escape: ESC + one byte (e.g., ESC =, ESC >, ESC c)
                    _ => { i += 1; }
                }
            }
            // Bare CSI (0x9B) — 8-bit CSI without ESC prefix
            0x9B => {
                i += 1;
                while i < len && !(0x40..=0x7E).contains(&bytes[i]) {
                    i += 1;
                }
                if i < len { i += 1; }
            }
            // C1 control range (0x80-0x9F) minus 0x9B which is handled above
            0x80..=0x9A | 0x9C..=0x9F => {
                i += 1;
            }
            // C0 control characters — keep \n, \r, \t; strip the rest
            b if b < 0x20 && b != b'\n' && b != b'\r' && b != b'\t' => {
                i += 1;
            }
            // DEL
            0x7F => { i += 1; }
            // Normal character — copy it
            _ => {
                // Safe to index since we checked it's not a control byte
                if let Some(ch) = std::str::from_utf8(&bytes[i..]).ok().and_then(|s| s.chars().next()) {
                    result.push(ch);
                    i += ch.len_utf8();
                } else {
                    i += 1;
                }
            }
        }
    }

    result
}

/// Handle \r: for each line, if it contains \r (not followed by \n),
/// only keep the text after the last \r (mimics terminal overwrite).
fn handle_carriage_returns(input: &str) -> String {
    let mut result = String::with_capacity(input.len());

    for line in input.split('\n') {
        if !result.is_empty() {
            result.push('\n');
        }
        // Only keep text after the last bare \r
        if let Some(pos) = line.rfind('\r') {
            let after = &line[pos + 1..];
            result.push_str(after);
        } else {
            result.push_str(line);
        }
    }

    result
}
