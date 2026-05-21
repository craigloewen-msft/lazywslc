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

    // Build tab titles and selected index based on section
    let (tab_titles, selected): (Vec<&str>, usize) = match app.active_section {
        ResourceSection::Containers => {
            let titles = vec!["Main", "Info", "Env"];
            let sel = match app.detail_tab {
                DetailTab::Main => 0,
                DetailTab::Info => 1,
                DetailTab::Env => 2,
            };
            (titles, sel)
        }
        ResourceSection::Images => {
            let titles = vec!["Info", "Env"];
            let sel = match app.detail_tab {
                DetailTab::Env => 1,
                _ => 0,
            };
            (titles, sel)
        }
        ResourceSection::Volumes => {
            let titles = vec!["Info"];
            (titles, 0)
        }
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

    match app.detail_tab {
        DetailTab::Main => draw_main_view(f, app, layout[1]),
        DetailTab::Info => draw_info_tab(f, app, layout[1]),
        DetailTab::Env => draw_env_tab(f, app, layout[1]),
    }
}

/// Combined Logs + Stats view (default for containers)
fn draw_main_view(f: &mut Frame, app: &App, area: Rect) {
    if app.active_section == ResourceSection::Containers {
        if let Some(c) = app.selected_container() {
            if c.is_running() {
                // Running container: logs on top, stats on bottom
                let split = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(50), // logs
                        Constraint::Percentage(50), // stats + graphs
                    ])
                    .split(area);

                draw_logs_tab(f, app, split[0]);
                draw_stats_tab(f, app, split[1]);
                return;
            }
        }
    }
    // Non-container or stopped: just show logs (or empty message)
    draw_logs_tab(f, app, area);
}
