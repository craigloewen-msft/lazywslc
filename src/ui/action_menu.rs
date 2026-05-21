use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::app::App;
use crate::ui::layout::centered_rect;

pub fn draw_action_menu(f: &mut Frame, app: &App, area: Rect) {
    if app.action_menu_items.is_empty() {
        return;
    }

    let height = (app.action_menu_items.len() as u16) + 2; // +2 for borders
    let popup = centered_rect(40, height.min(12), area);

    let items: Vec<ListItem> = app
        .action_menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let selected = i == app.action_menu_index;
            let style = if selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if selected { "▸ " } else { "  " };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&item.label, style),
                Span::styled(
                    format!("  [{}]", item.hotkey),
                    Style::default().fg(Color::Yellow),
                ),
            ]))
        })
        .collect();

    let title = match app.active_section {
        crate::app::ResourceSection::Containers => " Container Actions ",
        crate::app::ResourceSection::Images => " Image Actions ",
        crate::app::ResourceSection::Volumes => " Volume Actions ",
    };

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(Clear, popup);
    f.render_widget(list, popup);
}
