use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::{App, ResourceSection, FocusPanel};
use crate::wslc::types::relative_time;

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
    let max_name = (inner.width as usize).saturating_sub(4);

    let mut lines: Vec<Line> = Vec::new();
    for (i, c) in containers.iter().enumerate() {
        let is_selected = focused && i == app.container_index;
        let dot = if c.is_running() { "●" } else { "○" };
        let dot_color = if c.is_running() { Color::Green } else { Color::Red };
        let age = relative_time(c.created_at);

        // Line 1: dot + name + right-aligned age
        let name = truncate_str(&c.name, max_name.saturating_sub(age.len() + 1));
        let pad = (inner.width as usize).saturating_sub(3 + name.len() + age.len());
        let name_style = if is_selected {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(dot, Style::default().fg(dot_color)),
            Span::raw(" "),
            Span::styled(name, name_style),
            Span::raw(" ".repeat(pad)),
            Span::styled(age, Style::default().fg(Color::DarkGray)),
        ]));

        // Line 2 (selected only): image + state
        if is_selected {
            let state_color = if c.is_running() { Color::Green } else { Color::Red };
            let state_label = c.state_label();
            let img_max = (inner.width as usize).saturating_sub(4 + state_label.len() + 3);
            let img_name = truncate_str(&c.image, img_max);

            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(img_name, Style::default().fg(Color::DarkGray)),
                Span::styled(" • ", Style::default().fg(Color::DarkGray)),
                Span::styled(state_label.to_string(), Style::default().fg(state_color)),
            ]));
        }
    }

    let selected_height: usize = if focused { 2 } else { 1 };
    let scroll_offset = (app.container_index + selected_height)
        .saturating_sub(inner.height as usize) as u16;
    let paragraph = Paragraph::new(lines).scroll((scroll_offset, 0));
    f.render_widget(paragraph, inner);
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

    let mut lines: Vec<Line> = Vec::new();
    for (i, img) in images.iter().enumerate() {
        let is_selected = focused && i == app.image_index;
        let size_str = img.human_size();

        // Line 1: name + right-aligned size
        let max_name = (inner.width as usize).saturating_sub(2 + size_str.len() + 1);
        let name = truncate_str(&img.display_name(), max_name);
        let pad = (inner.width as usize).saturating_sub(2 + name.len() + size_str.len());
        let name_style = if is_selected {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(name, name_style),
            Span::raw(" ".repeat(pad)),
            Span::styled(size_str, Style::default().fg(Color::DarkGray)),
        ]));

        // Line 2 (selected only): age + in-use container count
        if is_selected {
            let age = relative_time(img.created);
            let in_use = app.containers.iter()
                .filter(|c| crate::ui::info_tab::image_matches(
                    &c.image, &img.display_name(),
                    img.repository.as_deref(), img.tag.as_deref()))
                .count();
            let use_text = if in_use > 0 {
                format!(" • ●{} container{}", in_use, if in_use == 1 { "" } else { "s" })
            } else {
                String::new()
            };

            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(format!("{} ago", age), Style::default().fg(Color::DarkGray)),
                Span::styled(use_text, Style::default().fg(Color::Yellow)),
            ]));
        }
    }

    let selected_height: usize = if focused { 2 } else { 1 };
    let scroll_offset = (app.image_index + selected_height)
        .saturating_sub(inner.height as usize) as u16;
    let paragraph = Paragraph::new(lines).scroll((scroll_offset, 0));
    f.render_widget(paragraph, inner);
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

    let scroll_offset = (app.volume_index + 1)
        .saturating_sub(inner.height as usize);
    let mut list_state = ListState::default().with_offset(scroll_offset);
    f.render_stateful_widget(List::new(items), inner, &mut list_state);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max && max > 1 {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
