use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};

use crate::app::{App, ResourceSection};

pub fn draw_stats_tab(f: &mut Frame, app: &App, area: Rect) {
    if app.active_section != ResourceSection::Containers {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  Stats are only available for containers.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(msg, area);
        return;
    }

    let Some(c) = app.selected_container() else {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  No container selected.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(msg, area);
        return;
    };

    if !c.is_running() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  Container is not running. Start it to see stats.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(msg, area);
        return;
    }

    // Layout: current stats on top, sparkline graphs below
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // current stats
            Constraint::Min(4),   // graphs
        ])
        .split(area);

    // Current stats text
    draw_current_stats(f, app, layout[0]);

    // Sparkline graphs
    draw_sparkline_graphs(f, app, &c.id, layout[1]);
}

fn draw_current_stats(f: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(ref stats) = app.current_stats {
        vec![
            Line::from(""),
            stat_line("  CPU", stats.cpu_perc.as_deref().unwrap_or("—")),
            stat_line("  Memory", stats.mem_usage.as_deref().unwrap_or("—")),
            stat_line("  Mem %", stats.mem_perc.as_deref().unwrap_or("—")),
            stat_line("  Net I/O", stats.net_io.as_deref().unwrap_or("—")),
            stat_line("  Block I/O", stats.block_io.as_deref().unwrap_or("—")),
            stat_line("  PIDs", &stats.pids.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "—".into())),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled("  Loading stats...", Style::default().fg(Color::DarkGray))),
        ]
    };

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn draw_sparkline_graphs(f: &mut Frame, app: &App, container_id: &str, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    if let Some(history) = app.get_stats_history(container_id) {
        // CPU sparkline — auto-scale based on max observed value
        let cpu_data: Vec<u64> = history.cpu.iter().map(|v| (*v * 100.0) as u64).collect();
        let cpu_max = cpu_data.iter().copied().max().unwrap_or(100);
        let cpu_ceil = (cpu_max + cpu_max / 5).max(100); // 20% headroom, min 1.00%
        let cpu_label = format!(" CPU % (max: {:.1}%) ", cpu_ceil as f64 / 100.0);
        let cpu_sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(Span::styled(cpu_label, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .data(&cpu_data)
            .max(cpu_ceil)
            .style(Style::default().fg(Color::Green));
        f.render_widget(cpu_sparkline, layout[0]);

        // Memory sparkline — auto-scale
        let mem_data: Vec<u64> = history.memory.iter().map(|v| (*v * 100.0) as u64).collect();
        let mem_max = mem_data.iter().copied().max().unwrap_or(100);
        let mem_ceil = (mem_max + mem_max / 5).max(100); // 20% headroom, min 1.00%
        let mem_label = format!(" Memory % (max: {:.1}%) ", mem_ceil as f64 / 100.0);
        let mem_sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(Span::styled(mem_label, Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .data(&mem_data)
            .max(mem_ceil)
            .style(Style::default().fg(Color::Blue));
        f.render_widget(mem_sparkline, layout[1]);
    } else {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  Collecting stats history...",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(msg, layout[0]);
    }
}

fn stat_line(label: &str, value: &str) -> Line<'static> {
    let padded = format!("{:<14}", label);
    Line::from(vec![
        Span::styled(padded, Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ])
}
