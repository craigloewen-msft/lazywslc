use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Clear};

use crate::app::{App, InputMode};
use super::resource_list::{draw_container_panel, draw_image_panel, draw_volume_panel};
use super::detail::draw_detail;
use super::status_bar::draw_status_bar;
use super::action_menu::draw_action_menu;

/// Precomputed layout areas for mouse hit-testing
pub struct LayoutAreas {
    pub container_inner: Rect,
    pub image_inner: Rect,
    pub volume_inner: Rect,
    pub detail_area: Rect,
    pub tab_bar: Rect,
}

/// Compute layout areas without rendering (for mouse hit-testing)
pub fn compute_areas(app: &App, size: Rect) -> LayoutAreas {
    let mut constraints = Vec::new();
    if app.show_splash {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(10));
    constraints.push(Constraint::Length(3));

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    let body_area = if app.show_splash { outer[1] } else { outer[0] };

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(72),
        ])
        .split(body_area);

    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(2, 5),
            Constraint::Ratio(2, 5),
            Constraint::Ratio(1, 5),
        ])
        .split(body[0]);

    let border_block = Block::default().borders(Borders::ALL);
    let container_inner = border_block.inner(left_panels[0]);
    let image_inner = border_block.inner(left_panels[1]);
    let volume_inner = border_block.inner(left_panels[2]);

    let detail_inner = border_block.inner(body[1]);
    let detail_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(detail_inner);

    LayoutAreas {
        container_inner,
        image_inner,
        volume_inner,
        detail_area: body[1],
        tab_bar: detail_layout[0],
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Main vertical layout: optional splash + body + status
    let mut constraints = Vec::new();
    if app.show_splash {
        constraints.push(Constraint::Length(3)); // splash header
    }
    constraints.push(Constraint::Min(10)); // body
    constraints.push(Constraint::Length(3)); // status bar

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    let (body_area, status_area) = if app.show_splash {
        draw_title(f, outer[0]);
        (outer[1], outer[2])
    } else {
        (outer[0], outer[1])
    };

    // Body: left column (28%) | right panel (72%)
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(72),
        ])
        .split(body_area);

    // Left column: three separate bordered panels stacked vertically
    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(2, 5), // Containers
            Constraint::Ratio(2, 5), // Images
            Constraint::Ratio(1, 5), // Volumes
        ])
        .split(body[0]);

    draw_container_panel(f, app, left_panels[0]);
    draw_image_panel(f, app, left_panels[1]);
    draw_volume_panel(f, app, left_panels[2]);

    // Right panel: detail view
    draw_detail(f, app, body[1]);

    // Status bar
    draw_status_bar(f, app, status_area);

    // Flash message overlay
    if let Some(ref msg) = app.flash_message {
        let flash_area = centered_rect(60, 3, size);
        let flash = Paragraph::new(Line::from(vec![
            Span::styled(" ℹ ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(msg.as_str(), Style::default().fg(Color::White)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(Clear, flash_area);
        f.render_widget(flash, flash_area);
    }

    // Confirm dialog overlay
    if app.input_mode == InputMode::Confirm {
        draw_confirm(f, app, size);
    }

    // Action menu overlay
    if app.input_mode == InputMode::ActionMenu {
        draw_action_menu(f, app, size);
    }

    // Pull input overlay
    if app.input_mode == InputMode::PullInput {
        draw_pull_input(f, app, size);
    }

    // Filter overlay
    if app.input_mode == InputMode::Filter {
        draw_filter_input(f, app, size);
    }
}

fn draw_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" 🐧 lazywslc ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::styled("— WSL Container Dashboard", Style::default().fg(Color::DarkGray)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title_alignment(ratatui::layout::Alignment::Center),
    );
    f.render_widget(title, area);
}

fn draw_confirm(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(50, 5, area);
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(&app.confirm_message, Style::default().fg(Color::Yellow))),
        Line::from(Span::styled("  [y] Yes  [n/Esc] No", Style::default().fg(Color::DarkGray))),
    ])
    .block(
        Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(Clear, popup);
    f.render_widget(text, popup);
}

fn draw_pull_input(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 5, area);
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Image: "),
            Span::styled(&app.pull_input, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(Color::White)),
        ]),
        Line::from(Span::styled("  [Enter] Pull  [Esc] Cancel", Style::default().fg(Color::DarkGray))),
    ])
    .block(
        Block::default()
            .title(" Pull Image ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(Clear, popup);
    f.render_widget(text, popup);
}

fn draw_filter_input(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = Rect::new(area.x, area.bottom().saturating_sub(6), area.width, 3);
    let text = Paragraph::new(Line::from(vec![
        Span::styled(" / ", Style::default().fg(Color::Black).bg(Color::Yellow)),
        Span::raw(" "),
        Span::styled(&app.filter_text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("█", Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .title(" Filter ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(Clear, popup_area);
    f.render_widget(text, popup_area);
}

pub fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
