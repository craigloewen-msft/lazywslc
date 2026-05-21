use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, InputMode, ResourceSection};

pub fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let hints = match app.input_mode {
        InputMode::Normal => normal_hints(app),
        InputMode::Filter => vec![
            hint("Enter", "apply"),
            hint("Esc", "cancel"),
        ],
        InputMode::Confirm => vec![
            hint("y", "yes"),
            hint("n/Esc", "no"),
        ],
        InputMode::ActionMenu => vec![
            hint("↑↓", "navigate"),
            hint("Enter", "select"),
            hint("Esc", "close"),
        ],
        InputMode::PullInput => vec![
            hint("Enter", "pull"),
            hint("Esc", "cancel"),
        ],
    };

    let spans: Vec<Span> = hints
        .into_iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(
                    format!(" {} ", key),
                    Style::default().fg(Color::Black).bg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{}  ", desc),
                    Style::default().fg(Color::DarkGray),
                ),
            ]
        })
        .collect();

    let loading_indicator = if app.loading {
        Span::styled(" ⟳ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        Span::raw("")
    };

    let mut all_spans = vec![Span::raw(" "), loading_indicator];
    all_spans.extend(spans);

    let bar = Paragraph::new(Line::from(all_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(bar, area);
}

fn normal_hints(app: &App) -> Vec<(&'static str, &'static str)> {
    let mut h = vec![
        ("q", "quit"),
        ("↑↓", "nav"),
        ("Tab", "panel"),
        ("Space", "actions"),
        ("?", "help"),
        ("/", "filter"),
        ("R", "refresh"),
    ];

    if app.active_section == ResourceSection::Containers {
        if let Some(c) = app.selected_container() {
            if c.is_running() {
                h.push(("S", "stop"));
                h.push(("K", "kill"));
            } else {
                h.push(("s", "start"));
            }
        }
        h.push(("x", "remove"));
    } else if app.active_section == ResourceSection::Images {
        h.push(("p", "pull"));
        h.push(("x", "remove"));
    } else {
        h.push(("x", "remove"));
    }

    h
}

fn hint(key: &'static str, desc: &'static str) -> (&'static str, &'static str) {
    (key, desc)
}
