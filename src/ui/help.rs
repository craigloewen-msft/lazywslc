use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::ui::layout::centered_rect;

pub fn draw_help(f: &mut Frame, area: Rect) {
    let popup = centered_rect(70, 22, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        help_line("  ↑/k, ↓/j", "Move selection up/down"),
        help_line("  Tab", "Switch focus between panels"),
        help_line("  1/2/3", "Switch to Containers/Images/Volumes"),
        help_line("  ←/→ or h/l", "Switch detail tabs"),
        help_line("  Enter", "Collapse/expand section"),
        Line::from(""),
        Line::from(Span::styled("  Actions", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        help_line("  Space", "Open action menu"),
        help_line("  s", "Start container"),
        help_line("  S", "Stop container"),
        help_line("  K", "Kill container"),
        help_line("  x", "Remove selected item"),
        help_line("  p", "Pull image"),
        help_line("  R", "Refresh all data"),
        help_line("  /", "Filter current list"),
        Line::from(""),
        help_line("  q / Ctrl-C", "Quit"),
        help_line("  ?", "Toggle this help"),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(Clear, popup);
    f.render_widget(paragraph, popup);
}

fn help_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<16}", key), Style::default().fg(Color::Yellow)),
        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
    ])
}
