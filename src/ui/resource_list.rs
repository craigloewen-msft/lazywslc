use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, ResourceSection, FocusPanel};

pub fn draw_container_panel(f: &mut Frame, app: &App, area: Rect) {
    let active = app.active_section == ResourceSection::Containers;
    let focused = active && app.focus == FocusPanel::ResourceList;
    let border_color = if focused { Color::Cyan } else { Color::DarkGray };

    let title = format!(" Containers ({}) ", app.filtered_containers().len());
    let block = Block::default()
        .title(Span::styled(
            title,
            if active { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) }
            else { Style::default().fg(Color::White) },
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let containers = app.filtered_containers();
    let items: Vec<ListItem> = containers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let dot = if c.is_running() { "●" } else { "○" };
            let dot_color = if c.is_running() { Color::Green } else { Color::Red };
            let max_len = (inner.width as usize).saturating_sub(4);
            let name = truncate_str(&c.name, max_len);

            let style = if focused && i == app.container_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(dot, Style::default().fg(dot_color)),
                Span::raw(" "),
                Span::styled(name, style),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

pub fn draw_image_panel(f: &mut Frame, app: &App, area: Rect) {
    let active = app.active_section == ResourceSection::Images;
    let focused = active && app.focus == FocusPanel::ResourceList;
    let border_color = if focused { Color::Cyan } else { Color::DarkGray };

    let title = format!(" Images ({}) ", app.filtered_images().len());
    let block = Block::default()
        .title(Span::styled(
            title,
            if active { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) }
            else { Style::default().fg(Color::White) },
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let images = app.filtered_images();
    let max_len = (inner.width as usize).saturating_sub(3);
    let items: Vec<ListItem> = images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            let name = truncate_str(&img.display_name(), max_len);
            let style = if focused && i == app.image_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(name, style),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

pub fn draw_volume_panel(f: &mut Frame, app: &App, area: Rect) {
    let active = app.active_section == ResourceSection::Volumes;
    let focused = active && app.focus == FocusPanel::ResourceList;
    let border_color = if focused { Color::Cyan } else { Color::DarkGray };

    let title = format!(" Volumes ({}) ", app.filtered_volumes().len());
    let block = Block::default()
        .title(Span::styled(
            title,
            if active { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) }
            else { Style::default().fg(Color::White) },
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let volumes = app.filtered_volumes();
    let max_len = (inner.width as usize).saturating_sub(3);
    let items: Vec<ListItem> = volumes
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let name = truncate_str(&v.name, max_len);
            let style = if focused && i == app.volume_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(name, style),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max && max > 1 {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
