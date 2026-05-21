use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, ResourceSection, FocusPanel};

pub fn draw_resource_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::ResourceList;
    let border_color = if focused { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .title(" Resources ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split inner area for three sections
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Containers header
            Constraint::Length(if app.containers_collapsed { 0 } else {
                app.filtered_containers().len().min(8) as u16
            }),
            Constraint::Length(1), // Images header
            Constraint::Length(if app.images_collapsed { 0 } else {
                app.filtered_images().len().min(6) as u16
            }),
            Constraint::Length(1), // Volumes header
            Constraint::Min(0),   // Volumes list (fills remaining)
        ])
        .split(inner);

    // Containers section
    draw_section_header(f, "Containers", app.containers.len(),
        app.active_section == ResourceSection::Containers && focused,
        app.containers_collapsed, sections[0]);
    if !app.containers_collapsed {
        draw_container_list(f, app, sections[1]);
    }

    // Images section
    draw_section_header(f, "Images", app.images.len(),
        app.active_section == ResourceSection::Images && focused,
        app.images_collapsed, sections[2]);
    if !app.images_collapsed {
        draw_image_list(f, app, sections[3]);
    }

    // Volumes section
    draw_section_header(f, "Volumes", app.volumes.len(),
        app.active_section == ResourceSection::Volumes && focused,
        app.volumes_collapsed, sections[4]);
    if !app.volumes_collapsed {
        draw_volume_list(f, app, sections[5]);
    }
}

fn draw_section_header(f: &mut Frame, title: &str, count: usize, active: bool, collapsed: bool, area: Rect) {
    let arrow = if collapsed { "▸" } else { "▾" };
    let style = if active {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let line = Line::from(vec![
        Span::styled(format!(" {} ", arrow), style),
        Span::styled(title, style),
        Span::styled(format!(" ({})", count), Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(ratatui::widgets::Paragraph::new(line), area);
}

fn draw_container_list(f: &mut Frame, app: &App, area: Rect) {
    let containers = app.filtered_containers();
    let is_active = app.active_section == ResourceSection::Containers
        && app.focus == FocusPanel::ResourceList;

    let items: Vec<ListItem> = containers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let dot = if c.is_running() { "●" } else { "○" };
            let dot_color = if c.is_running() { Color::Green } else { Color::Red };

            let name = if c.name.len() > 18 {
                format!("{}…", &c.name[..17])
            } else {
                c.name.clone()
            };

            let style = if is_active && i == app.container_index {
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

    let list = List::new(items);
    f.render_widget(list, area);
}

fn draw_image_list(f: &mut Frame, app: &App, area: Rect) {
    let images = app.filtered_images();
    let is_active = app.active_section == ResourceSection::Images
        && app.focus == FocusPanel::ResourceList;

    let items: Vec<ListItem> = images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            let name = img.display_name();
            let display = if name.len() > 20 {
                format!("{}…", &name[..19])
            } else {
                name
            };

            let style = if is_active && i == app.image_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::raw("   "),
                Span::styled(display, style),
            ]))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, area);
}

fn draw_volume_list(f: &mut Frame, app: &App, area: Rect) {
    let volumes = app.filtered_volumes();
    let is_active = app.active_section == ResourceSection::Volumes
        && app.focus == FocusPanel::ResourceList;

    let items: Vec<ListItem> = volumes
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let style = if is_active && i == app.volume_index {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::raw("   "),
                Span::styled(&v.name, style),
            ]))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, area);
}
