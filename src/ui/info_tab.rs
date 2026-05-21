use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, ResourceSection};

pub fn draw_info_tab(f: &mut Frame, app: &App, area: Rect) {
    let lines = match app.active_section {
        ResourceSection::Containers => container_info(app),
        ResourceSection::Images => image_info(app),
        ResourceSection::Volumes => volume_info(app),
    };

    let paragraph = Paragraph::new(lines).scroll((app.info_scroll, 0));
    f.render_widget(paragraph, area);
}

fn container_info(app: &App) -> Vec<Line<'static>> {
    let Some(c) = app.selected_container() else {
        return vec![Line::from(Span::styled("  No container selected", Style::default().fg(Color::DarkGray)))];
    };

    let state_color = if c.is_running() { Color::Green } else { Color::Red };
    let dot = if c.is_running() { "●" } else { "○" };

    let created = format_timestamp(c.created_at);

    let mut lines = vec![
        Line::from(""),
        info_line("  Name", &c.name),
        info_line("  Image", &c.image),
        Line::from(vec![
            Span::styled("  State       ", Style::default().fg(Color::DarkGray)),
            Span::styled(dot, Style::default().fg(state_color)),
            Span::raw(" "),
            Span::styled(c.state_label().to_string(), Style::default().fg(state_color).add_modifier(Modifier::BOLD)),
        ]),
        info_line("  ID", c.short_id()),
        info_line("  Created", &created),
    ];

    if !c.ports.is_empty() {
        let ports_str: Vec<String> = c.ports.iter().map(|p| {
            format!("{}:{}/{}",
                p.host_port.map(|v| v.to_string()).unwrap_or_default(),
                p.container_port.map(|v| v.to_string()).unwrap_or_default(),
                p.protocol.as_deref().unwrap_or("tcp"))
        }).collect();
        lines.push(info_line("  Ports", &ports_str.join(", ")));
    } else {
        lines.push(info_line("  Ports", "—"));
    }

    // Inspect data
    if !app.inspect_text.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  ─── Inspect Data ───", Style::default().fg(Color::Cyan))));
        lines.push(Line::from(""));
        for line in app.inspect_text.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", line))));
        }
    }

    lines
}

fn image_info(app: &App) -> Vec<Line<'static>> {
    let Some(img) = app.selected_image() else {
        return vec![Line::from(Span::styled("  No image selected", Style::default().fg(Color::DarkGray)))];
    };

    let created = format_timestamp(img.created);

    let mut lines = vec![
        Line::from(""),
        info_line("  Repository", img.repository.as_deref().unwrap_or("<none>")),
        info_line("  Tag", img.tag.as_deref().unwrap_or("<none>")),
        info_line("  ID", img.short_id()),
        info_line("  Size", &img.human_size()),
        info_line("  Created", &created),
    ];

    if !app.inspect_text.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  ─── Inspect Data ───", Style::default().fg(Color::Cyan))));
        lines.push(Line::from(""));
        for line in app.inspect_text.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", line))));
        }
    }

    lines
}

fn volume_info(app: &App) -> Vec<Line<'static>> {
    let Some(v) = app.selected_volume() else {
        return vec![Line::from(Span::styled("  No volume selected", Style::default().fg(Color::DarkGray)))];
    };

    let mut lines = vec![
        Line::from(""),
        info_line("  Name", &v.name),
        info_line("  Driver", &v.driver),
    ];

    if !app.inspect_text.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  ─── Inspect Data ───", Style::default().fg(Color::Cyan))));
        lines.push(Line::from(""));
        for line in app.inspect_text.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", line))));
        }
    }

    lines
}

fn info_line(label: &str, value: &str) -> Line<'static> {
    let padded = format!("{:<14}", label);
    Line::from(vec![
        Span::styled(padded, Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ts.to_string())
}
