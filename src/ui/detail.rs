use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Tabs};

use crate::app::{App, DetailTab, FocusPanel, ResourceSection};
use super::info_tab::draw_info_tab;
use super::logs_tab::draw_logs_tab;
use super::stats_tab::draw_stats_tab;
use super::env_tab::draw_env_tab;

pub fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::Detail;
    let border_color = if focused { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Tab bar + content
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // tab bar
            Constraint::Min(1),   // content
        ])
        .split(inner);

    // Tab titles depend on resource type
    let tab_titles = match app.active_section {
        ResourceSection::Containers => vec!["Info", "Logs", "Stats", "Env"],
        ResourceSection::Images => vec!["Info", "Logs", "Stats", "Env"],
        ResourceSection::Volumes => vec!["Info", "Logs", "Stats", "Env"],
    };

    let selected = match app.detail_tab {
        DetailTab::Info => 0,
        DetailTab::Logs => 1,
        DetailTab::Stats => 2,
        DetailTab::Env => 3,
    };

    let tabs = Tabs::new(tab_titles.iter().map(|t| Line::from(*t)).collect::<Vec<_>>())
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .style(Style::default().fg(Color::DarkGray))
        .divider(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));

    f.render_widget(tabs, layout[0]);

    // Draw the active tab content
    match app.detail_tab {
        DetailTab::Info => draw_info_tab(f, app, layout[1]),
        DetailTab::Logs => draw_logs_tab(f, app, layout[1]),
        DetailTab::Stats => draw_stats_tab(f, app, layout[1]),
        DetailTab::Env => draw_env_tab(f, app, layout[1]),
    }
}
