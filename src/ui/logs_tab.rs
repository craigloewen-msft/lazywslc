use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn draw_logs_tab(f: &mut Frame, app: &App, area: Rect) {
    if app.logs_text.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No logs available. Select a container and press 'l' to load logs.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    }

    let lines: Vec<Line> = app
        .logs_text
        .lines()
        .map(|l| Line::from(Span::raw(format!("  {}", l))))
        .collect();

    let total = lines.len() as u16;
    let visible = area.height;
    let max_scroll = total.saturating_sub(visible);
    let scroll = app.logs_scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines).scroll((scroll, 0));
    f.render_widget(paragraph, area);
}
