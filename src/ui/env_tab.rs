use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn draw_env_tab(f: &mut Frame, app: &App, area: Rect) {
    // Extract env from inspect JSON
    let lines = extract_env_lines(&app.inspect_text);

    if lines.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No environment variables found.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(empty, area);
        return;
    }

    let paragraph = Paragraph::new(lines).scroll((0, 0));
    f.render_widget(paragraph, area);
}

fn extract_env_lines(inspect_text: &str) -> Vec<Line<'static>> {
    // Try to parse inspect as JSON and extract Env
    let Ok(json) = serde_json::from_str::<serde_json::Value>(inspect_text) else {
        if inspect_text.is_empty() {
            return vec![];
        }
        // Fallback: show raw text
        return inspect_text
            .lines()
            .filter(|l| l.contains('='))
            .map(|l| {
                let l = l.trim().trim_matches('"').trim_end_matches(',');
                format_env_line(l)
            })
            .collect();
    };

    // Search for Env array in the JSON (could be nested)
    let env_arr = find_env_array(&json);
    let Some(env) = env_arr else {
        return vec![Line::from(Span::styled(
            "  No environment variables found in inspect data.",
            Style::default().fg(Color::DarkGray),
        ))];
    };

    let mut lines = vec![Line::from("")];
    for val in env {
        if let Some(s) = val.as_str() {
            lines.push(format_env_line(s));
        }
    }
    lines
}

fn find_env_array(val: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    match val {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Array(arr)) = map.get("Env") {
                return Some(arr);
            }
            for (_, v) in map {
                if let Some(arr) = find_env_array(v) {
                    return Some(arr);
                }
            }
            None
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                if let Some(result) = find_env_array(v) {
                    return Some(result);
                }
            }
            None
        }
        _ => None,
    }
}

fn format_env_line(s: &str) -> Line<'static> {
    if let Some((key, value)) = s.split_once('=') {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(key.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("=", Style::default().fg(Color::DarkGray)),
            Span::styled(value.to_string(), Style::default().fg(Color::White)),
        ])
    } else {
        Line::from(Span::raw(format!("  {}", s)))
    }
}
