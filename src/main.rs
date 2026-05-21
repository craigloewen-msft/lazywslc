mod app;
mod event;
mod ui;
mod wslc;

use std::io;
use std::time::Duration;
use anyhow::Result;
use crossterm::{
    event::{KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, ConfirmAction, DetailTab, FocusPanel, InputMode, ResourceSection};
use event::{AppEvent, poll_event, is_quit};

const TICK_RATE: Duration = Duration::from_millis(250);
const STATS_INTERVAL: u8 = 8;   // every 8 ticks × 250ms = ~2 seconds
const REFRESH_INTERVAL: u8 = 4; // every 4 ticks × 250ms = ~1 second
const SPLASH_DURATION: u16 = 20; // 20 ticks × 250ms = 5 seconds

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    // Initial data load
    refresh_data(app).await;

    let mut tick_counter: u8 = 0;
    let mut refresh_counter: u8 = 0;
    let mut show_help = false;

    loop {
        // Draw
        terminal.draw(|f| {
            ui::layout::draw(f, app);
            if show_help {
                ui::help::draw_help(f, f.area());
            }
        })?;

        // Handle events
        match poll_event(TICK_RATE)? {
            AppEvent::Key(key) => {
                // Help overlay intercepts all keys
                if show_help {
                    show_help = false;
                    continue;
                }

                match app.input_mode {
                    InputMode::Normal => {
                        handle_normal_key(app, key.code, key.modifiers, &mut show_help).await;
                    }
                    InputMode::Filter => {
                        handle_filter_key(app, key.code);
                    }
                    InputMode::Confirm => {
                        handle_confirm_key(app, key.code).await;
                    }
                    InputMode::ActionMenu => {
                        handle_action_menu_key(app, key.code).await;
                    }
                    InputMode::PullInput => {
                        handle_pull_input_key(app, key.code).await;
                    }
                }
            }
            AppEvent::Tick => {
                app.tick_flash();
                tick_counter += 1;
                refresh_counter += 1;

                // Splash timeout
                if app.show_splash {
                    app.splash_ticks += 1;
                    if app.splash_ticks >= SPLASH_DURATION {
                        app.show_splash = false;
                    }
                }

                // Auto-refresh data every ~1 second
                if refresh_counter >= REFRESH_INTERVAL {
                    refresh_counter = 0;
                    refresh_data(app).await;
                }

                // Periodic stats refresh for running containers
                if tick_counter >= STATS_INTERVAL {
                    tick_counter = 0;
                    if app.active_section == ResourceSection::Containers
                        && app.detail_tab == DetailTab::Stats
                    {
                        if let Some(c) = app.selected_container() {
                            if c.is_running() {
                                let id = c.id.clone();
                                fetch_stats(app, &id).await;
                            }
                        }
                    }
                }
            }
            AppEvent::Resize(_, _) => {}
        }

        if !app.running {
            return Ok(());
        }
    }
}

async fn handle_normal_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    show_help: &mut bool,
) {
    if is_quit(&crossterm::event::KeyEvent::new(code, modifiers)) {
        app.running = false;
        return;
    }

    match code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Tab => {
            app.focus = match app.focus {
                FocusPanel::ResourceList => FocusPanel::Detail,
                FocusPanel::Detail => FocusPanel::ResourceList,
            };
        }

        // Section switching
        KeyCode::Char('1') => {
            app.active_section = ResourceSection::Containers;
            app.detail_tab = DetailTab::Info;
            load_inspect_for_selected(app).await;
        }
        KeyCode::Char('2') => {
            app.active_section = ResourceSection::Images;
            app.detail_tab = DetailTab::Info;
            load_inspect_for_selected(app).await;
        }
        KeyCode::Char('3') => {
            app.active_section = ResourceSection::Volumes;
            app.detail_tab = DetailTab::Info;
            load_inspect_for_selected(app).await;
        }

        // Detail tab switching (when in detail focus)
        KeyCode::Right | KeyCode::Char('L') => {
            app.next_tab();
            if app.detail_tab == DetailTab::Logs {
                load_logs_for_selected(app).await;
            }
            if app.detail_tab == DetailTab::Stats {
                load_stats_for_selected(app).await;
            }
        }
        KeyCode::Left | KeyCode::Char('H') => {
            app.prev_tab();
            if app.detail_tab == DetailTab::Logs {
                load_logs_for_selected(app).await;
            }
            if app.detail_tab == DetailTab::Stats {
                load_stats_for_selected(app).await;
            }
        }

        // Collapse/expand
        KeyCode::Enter => {
            match app.active_section {
                ResourceSection::Containers => app.containers_collapsed = !app.containers_collapsed,
                ResourceSection::Images => app.images_collapsed = !app.images_collapsed,
                ResourceSection::Volumes => app.volumes_collapsed = !app.volumes_collapsed,
            }
            load_inspect_for_selected(app).await;
        }

        // Actions
        KeyCode::Char(' ') => {
            app.build_action_menu();
        }
        KeyCode::Char('s') => {
            if app.active_section == ResourceSection::Containers {
                if let Some(c) = app.selected_container() {
                    if !c.is_running() {
                        let id = c.id.clone();
                        app.loading = true;
                        match wslc::commands::start_container(&id).await {
                            Ok(_) => {
                                app.set_flash("Container started".into());
                                refresh_data(app).await;
                            }
                            Err(e) => app.set_flash(format!("Error: {}", e)),
                        }
                        app.loading = false;
                    }
                }
            }
        }
        KeyCode::Char('S') => {
            if app.active_section == ResourceSection::Containers {
                if let Some(c) = app.selected_container() {
                    if c.is_running() {
                        let id = c.id.clone();
                        app.loading = true;
                        match wslc::commands::stop_container(&id).await {
                            Ok(_) => {
                                app.set_flash("Container stopped".into());
                                refresh_data(app).await;
                            }
                            Err(e) => app.set_flash(format!("Error: {}", e)),
                        }
                        app.loading = false;
                    }
                }
            }
        }
        KeyCode::Char('K') => {
            if app.active_section == ResourceSection::Containers {
                if let Some(c) = app.selected_container() {
                    if c.is_running() {
                        let id = c.id.clone();
                        app.loading = true;
                        match wslc::commands::kill_container(&id).await {
                            Ok(_) => {
                                app.set_flash("Container killed".into());
                                refresh_data(app).await;
                            }
                            Err(e) => app.set_flash(format!("Error: {}", e)),
                        }
                        app.loading = false;
                    }
                }
            }
        }
        KeyCode::Char('x') => {
            prompt_remove(app);
        }
        KeyCode::Char('p') => {
            if app.active_section == ResourceSection::Images {
                app.pull_input.clear();
                app.input_mode = InputMode::PullInput;
            }
        }
        KeyCode::Char('l') => {
            app.detail_tab = DetailTab::Logs;
            load_logs_for_selected(app).await;
        }
        KeyCode::Char('R') => {
            refresh_data(app).await;
            app.set_flash("Data refreshed".into());
        }
        KeyCode::Char('/') => {
            app.filter_text.clear();
            app.input_mode = InputMode::Filter;
        }
        KeyCode::Char('?') => {
            *show_help = true;
        }

        // Scroll logs
        KeyCode::PageDown => {
            app.logs_scroll = app.logs_scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            app.logs_scroll = app.logs_scroll.saturating_sub(10);
        }

        _ => {}
    }
}

fn handle_filter_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter | KeyCode::Esc => {
            if code == KeyCode::Esc {
                app.filter_text.clear();
            }
            app.clamp_indices();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char(c) => {
            app.filter_text.push(c);
            app.clamp_indices();
        }
        KeyCode::Backspace => {
            app.filter_text.pop();
            app.clamp_indices();
        }
        _ => {}
    }
}

async fn handle_confirm_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(action) = app.confirm_action.take() {
                app.loading = true;
                let result = match action {
                    ConfirmAction::RemoveContainer(id) => {
                        wslc::commands::remove_container(&id).await.map(|_| "Container removed")
                    }
                    ConfirmAction::RemoveImage(id) => {
                        wslc::commands::remove_image(&id).await.map(|_| "Image removed")
                    }
                    ConfirmAction::RemoveVolume(name) => {
                        wslc::commands::remove_volume(&name).await.map(|_| "Volume removed")
                    }
                };
                match result {
                    Ok(msg) => {
                        app.set_flash(msg.into());
                        refresh_data(app).await;
                    }
                    Err(e) => app.set_flash(format!("Error: {}", e)),
                }
                app.loading = false;
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.confirm_action = None;
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

async fn handle_action_menu_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.action_menu_index > 0 {
                app.action_menu_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.action_menu_index < app.action_menu_items.len().saturating_sub(1) {
                app.action_menu_index += 1;
            }
        }
        KeyCode::Enter => {
            let hotkey = app.action_menu_items.get(app.action_menu_index)
                .map(|item| item.hotkey);
            app.input_mode = InputMode::Normal;
            if let Some(key) = hotkey {
                execute_action_hotkey(app, key).await;
            }
        }
        KeyCode::Char(c) => {
            // Direct hotkey press
            let found = app.action_menu_items.iter().any(|item| item.hotkey == c);
            if found {
                app.input_mode = InputMode::Normal;
                execute_action_hotkey(app, c).await;
            }
        }
        _ => {}
    }
}

async fn execute_action_hotkey(app: &mut App, hotkey: char) {
    match hotkey {
        's' => {
            if let Some(c) = app.selected_container() {
                let id = c.id.clone();
                app.loading = true;
                match wslc::commands::start_container(&id).await {
                    Ok(_) => {
                        app.set_flash("Container started".into());
                        refresh_data(app).await;
                    }
                    Err(e) => app.set_flash(format!("Error: {}", e)),
                }
                app.loading = false;
            }
        }
        'S' => {
            if let Some(c) = app.selected_container() {
                let id = c.id.clone();
                app.loading = true;
                match wslc::commands::stop_container(&id).await {
                    Ok(_) => {
                        app.set_flash("Container stopped".into());
                        refresh_data(app).await;
                    }
                    Err(e) => app.set_flash(format!("Error: {}", e)),
                }
                app.loading = false;
            }
        }
        'K' => {
            if let Some(c) = app.selected_container() {
                let id = c.id.clone();
                app.loading = true;
                match wslc::commands::kill_container(&id).await {
                    Ok(_) => {
                        app.set_flash("Container killed".into());
                        refresh_data(app).await;
                    }
                    Err(e) => app.set_flash(format!("Error: {}", e)),
                }
                app.loading = false;
            }
        }
        'x' => {
            prompt_remove(app);
        }
        'l' => {
            app.detail_tab = DetailTab::Logs;
            load_logs_for_selected(app).await;
        }
        'p' => {
            app.pull_input.clear();
            app.input_mode = InputMode::PullInput;
        }
        _ => {}
    }
}

async fn handle_pull_input_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            if !app.pull_input.is_empty() {
                let name = app.pull_input.clone();
                app.input_mode = InputMode::Normal;
                app.loading = true;
                app.set_flash(format!("Pulling '{}'...", name));
                match wslc::commands::pull_image(&name).await {
                    Ok(_) => {
                        app.set_flash(format!("Pulled '{}'", name));
                        refresh_data(app).await;
                    }
                    Err(e) => app.set_flash(format!("Pull failed: {}", e)),
                }
                app.loading = false;
            }
        }
        KeyCode::Char(c) => {
            app.pull_input.push(c);
        }
        KeyCode::Backspace => {
            app.pull_input.pop();
        }
        _ => {}
    }
}

async fn refresh_data(app: &mut App) {
    app.loading = true;

    let (containers, images, volumes) = tokio::join!(
        wslc::commands::list_containers(),
        wslc::commands::list_images(),
        wslc::commands::list_volumes(),
    );

    app.containers = containers.unwrap_or_default();
    app.images = images.unwrap_or_default();
    app.volumes = volumes.unwrap_or_default();

    app.clamp_indices();
    app.loading = false;

    load_inspect_for_selected(app).await;
}

async fn load_inspect_for_selected(app: &mut App) {
    let id = app.selected_resource_id();
    if let Some(id) = id {
        match wslc::commands::inspect_object(&id).await {
            Ok(text) => app.inspect_text = text,
            Err(_) => app.inspect_text.clear(),
        }
    } else {
        app.inspect_text.clear();
    }
}

async fn load_logs_for_selected(app: &mut App) {
    if app.active_section != ResourceSection::Containers {
        return;
    }
    if let Some(c) = app.selected_container() {
        let id = c.id.clone();
        match wslc::commands::container_logs(&id, 200).await {
            Ok(text) => {
                app.logs_text = text;
                app.logs_scroll = 0;
            }
            Err(_) => app.logs_text.clear(),
        }
    }
}

async fn load_stats_for_selected(app: &mut App) {
    if app.active_section != ResourceSection::Containers {
        return;
    }
    if let Some(c) = app.selected_container() {
        if c.is_running() {
            let id = c.id.clone();
            fetch_stats(app, &id).await;
        }
    }
}

async fn fetch_stats(app: &mut App, container_id: &str) {
    match wslc::commands::container_stats(container_id).await {
        Ok(text) => {
            app.stats_text = text.clone();
            // Try to parse stats JSON
            let trimmed = text.trim();
            // Stats can be a JSON object or array
            if let Ok(stats) = serde_json::from_str::<wslc::types::Stats>(trimmed) {
                // Parse CPU percentage
                let cpu_val = parse_percent(stats.cpu_perc.as_deref().unwrap_or("0"));
                let mem_val = parse_percent(stats.mem_perc.as_deref().unwrap_or("0"));
                app.push_stats_sample(container_id, cpu_val, mem_val);
                app.current_stats = Some(stats);
            } else if let Ok(stats_arr) = serde_json::from_str::<Vec<wslc::types::Stats>>(trimmed) {
                if let Some(stats) = stats_arr.into_iter().next() {
                    let cpu_val = parse_percent(stats.cpu_perc.as_deref().unwrap_or("0"));
                    let mem_val = parse_percent(stats.mem_perc.as_deref().unwrap_or("0"));
                    app.push_stats_sample(container_id, cpu_val, mem_val);
                    app.current_stats = Some(stats);
                }
            }
        }
        Err(_) => {
            app.stats_text.clear();
            app.current_stats = None;
        }
    }
}

fn parse_percent(s: &str) -> f64 {
    let cleaned = s.trim().trim_end_matches('%').trim();
    cleaned.parse::<f64>().unwrap_or(0.0)
}

fn prompt_remove(app: &mut App) {
    match app.active_section {
        ResourceSection::Containers => {
            let info = app.selected_container().map(|c| (c.name.clone(), c.id.clone()));
            if let Some((name, id)) = info {
                app.confirm_message = format!("Remove container '{}'?", name);
                app.confirm_action = Some(ConfirmAction::RemoveContainer(id));
                app.input_mode = InputMode::Confirm;
            }
        }
        ResourceSection::Images => {
            let info = app.selected_image().map(|i| (i.display_name(), i.id.clone()));
            if let Some((name, id)) = info {
                app.confirm_message = format!("Remove image '{}'?", name);
                app.confirm_action = Some(ConfirmAction::RemoveImage(id));
                app.input_mode = InputMode::Confirm;
            }
        }
        ResourceSection::Volumes => {
            let info = app.selected_volume().map(|v| v.name.clone());
            if let Some(name) = info {
                app.confirm_message = format!("Remove volume '{}'?", name);
                app.confirm_action = Some(ConfirmAction::RemoveVolume(name));
                app.input_mode = InputMode::Confirm;
            }
        }
    }
}

