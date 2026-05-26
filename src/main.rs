mod app;
mod event;
mod ui;
mod wslc;

use std::io;
use std::time::Duration;
use anyhow::Result;
use crossterm::{
    event::{KeyCode, KeyModifiers, MouseEventKind, MouseButton, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use app::{App, ConfirmAction, DetailTab, InputMode, ResourceSection};
use event::{AppEvent, poll_event, is_quit};

const TICK_RATE: Duration = Duration::from_millis(250);
const STATS_INTERVAL: u8 = 8;   // every 8 ticks × 250ms = ~2 seconds
const REFRESH_INTERVAL: u8 = 4; // every 4 ticks × 250ms = ~1 second

/// Messages sent from background tasks back to the main event loop.
enum BgMessage {
    DataRefreshed {
        containers: Vec<wslc::types::Container>,
        images: Vec<wslc::types::Image>,
        volumes: Vec<wslc::types::Volume>,
    },
    StatsLoaded {
        container_id: String,
        text: String,
    },
    LogsLoaded {
        text: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
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
    let (bg_tx, mut bg_rx) = mpsc::unbounded_channel::<BgMessage>();

    // Kick off initial data load in background so the UI renders instantly
    app.loading = true;
    spawn_refresh(bg_tx.clone());

    let mut tick_counter: u8 = 0;
    let mut refresh_counter: u8 = 0;
    let mut show_help = false;

    loop {
        // Drain background results (non-blocking)
        while let Ok(msg) = bg_rx.try_recv() {
            handle_bg_message(app, msg);
        }

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
                        handle_normal_key(app, key.code, key.modifiers, &mut show_help, &bg_tx).await;
                    }
                    InputMode::Filter => {
                        handle_filter_key(app, key.code);
                    }
                    InputMode::Confirm => {
                        handle_confirm_key(app, key.code, &bg_tx).await;
                    }
                    InputMode::ActionMenu => {
                        handle_action_menu_key(app, key.code, &bg_tx).await;
                    }
                    InputMode::PullInput => {
                        handle_pull_input_key(app, key.code, &bg_tx).await;
                    }
                }
            }
            AppEvent::Mouse(mouse) => {
                if app.input_mode == InputMode::Normal && !show_help {
                    let size = terminal.get_frame().area();
                    handle_mouse(app, mouse.kind, mouse.column, mouse.row, size).await;
                }
            }
            AppEvent::Tick => {
                app.tick_flash();
                tick_counter += 1;
                refresh_counter += 1;

                // Auto-refresh data in background (skip if already loading or still showing splash)
                if refresh_counter >= REFRESH_INTERVAL && !app.show_splash {
                    refresh_counter = 0;
                    if !app.loading {
                        app.loading = true;
                        spawn_refresh(bg_tx.clone());
                    }
                }

                // Periodic stats/logs refresh in background
                if tick_counter >= STATS_INTERVAL && !app.show_splash {
                    tick_counter = 0;
                    if app.active_section == ResourceSection::Containers {
                        if let Some(c) = app.selected_container() {
                            if c.is_running() {
                                spawn_stats(bg_tx.clone(), c.id.clone());
                            }
                        }
                        if let Some(c) = app.selected_container() {
                            spawn_logs(bg_tx.clone(), c.id.clone());
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
    bg_tx: &mpsc::UnboundedSender<BgMessage>,
) {
    if is_quit(&crossterm::event::KeyEvent::new(code, modifiers)) {
        app.running = false;
        return;
    }

    match code {
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            let prev = app.active_section;
            app.move_up();
            if app.active_section != prev {
                load_inspect_for_selected(app).await;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let prev = app.active_section;
            app.move_down();
            if app.active_section != prev {
                load_inspect_for_selected(app).await;
            }
        }
        KeyCode::Tab => {
            app.next_section();
            load_inspect_for_selected(app).await;
        }

        // Section switching
        KeyCode::Char('1') => {
            app.active_section = ResourceSection::Containers;
            app.detail_tab = app.default_tab();
            app.info_scroll = 0;
            load_inspect_for_selected(app).await;
        }
        KeyCode::Char('2') => {
            app.active_section = ResourceSection::Images;
            app.detail_tab = app.default_tab();
            app.info_scroll = 0;
            load_inspect_for_selected(app).await;
        }
        KeyCode::Char('3') => {
            app.active_section = ResourceSection::Volumes;
            app.detail_tab = app.default_tab();
            app.info_scroll = 0;
            load_inspect_for_selected(app).await;
        }

        // Detail tab switching
        KeyCode::Right | KeyCode::Char('L') => {
            app.next_tab();
            app.info_scroll = 0;
        }
        KeyCode::Left | KeyCode::Char('H') => {
            app.prev_tab();
            app.info_scroll = 0;
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
                                spawn_refresh(bg_tx.clone());
                            }
                            Err(e) => {
                                app.set_flash(format!("Error: {}", e));
                                app.loading = false;
                            }
                        }
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
                                spawn_refresh(bg_tx.clone());
                            }
                            Err(e) => {
                                app.set_flash(format!("Error: {}", e));
                                app.loading = false;
                            }
                        }
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
                                spawn_refresh(bg_tx.clone());
                            }
                            Err(e) => {
                                app.set_flash(format!("Error: {}", e));
                                app.loading = false;
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('x') => {
            prompt_remove(app);
        }
        KeyCode::Char('p') => {
            match app.active_section {
                ResourceSection::Containers => {
                    let stopped_count = app.containers.iter().filter(|c| !c.is_running()).count();
                    if stopped_count == 0 {
                        app.set_flash("No stopped containers to prune".into());
                    } else {
                        app.confirm_message = format!(
                            "Prune {} stopped container{}? This cannot be undone.",
                            stopped_count,
                            if stopped_count == 1 { "" } else { "s" }
                        );
                        app.confirm_action = Some(ConfirmAction::PruneContainers);
                        app.input_mode = InputMode::Confirm;
                    }
                }
                ResourceSection::Images => {
                    app.confirm_message = "Prune all dangling images? This cannot be undone.".into();
                    app.confirm_action = Some(ConfirmAction::PruneImages);
                    app.input_mode = InputMode::Confirm;
                }
                ResourceSection::Volumes => {
                    app.set_flash("Prune not available for volumes".into());
                }
            }
        }
        KeyCode::Char('l') => {
            app.detail_tab = DetailTab::Main;
            load_logs_for_selected(app).await;
        }
        KeyCode::Char('R') => {
            app.loading = true;
            spawn_refresh(bg_tx.clone());
            app.set_flash("Refreshing...".into());
        }
        KeyCode::Char('/') => {
            app.filter_text.clear();
            app.input_mode = InputMode::Filter;
        }
        KeyCode::Char('?') => {
            *show_help = true;
        }

        // Scroll logs (logs_scroll = offset from bottom, so PageUp = increase)
        KeyCode::PageDown => {
            app.logs_scroll = app.logs_scroll.saturating_sub(10);
        }
        KeyCode::PageUp => {
            app.logs_scroll = app.logs_scroll.saturating_add(10);
        }

        // Scroll info (f = forward, b = backward)
        KeyCode::Char('f') => {
            app.info_scroll = app.info_scroll.saturating_add(10);
        }
        KeyCode::Char('b') => {
            app.info_scroll = app.info_scroll.saturating_sub(10);
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

async fn handle_confirm_key(app: &mut App, code: KeyCode, bg_tx: &mpsc::UnboundedSender<BgMessage>) {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(action) = app.confirm_action.take() {
                app.loading = true;
                let result = match action {
                    ConfirmAction::RemoveContainer(id) => {
                        wslc::commands::remove_container(&id).await.map(|_| "Container removed".to_string())
                    }
                    ConfirmAction::RemoveImage(id) => {
                        wslc::commands::remove_image(&id).await.map(|_| "Image removed".to_string())
                    }
                    ConfirmAction::RemoveVolume(name) => {
                        wslc::commands::remove_volume(&name).await.map(|_| "Volume removed".to_string())
                    }
                    ConfirmAction::PruneContainers => {
                        let stopped_ids: Vec<String> = app.containers
                            .iter()
                            .filter(|c| !c.is_running())
                            .map(|c| c.id.clone())
                            .collect();
                        wslc::commands::prune_containers(&stopped_ids).await
                            .map(|n| format!("Pruned {} container{}", n, if n == 1 { "" } else { "s" }))
                    }
                    ConfirmAction::PruneImages => {
                        wslc::commands::prune_images().await
                            .map(|_| "Dangling images pruned".to_string())
                    }
                };
                match result {
                    Ok(msg) => {
                        app.set_flash(msg);
                        spawn_refresh(bg_tx.clone());
                    }
                    Err(e) => {
                        app.set_flash(format!("Error: {}", e));
                        app.loading = false;
                    }
                }
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

async fn handle_action_menu_key(app: &mut App, code: KeyCode, bg_tx: &mpsc::UnboundedSender<BgMessage>) {
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
                execute_action_hotkey(app, key, bg_tx).await;
            }
        }
        KeyCode::Char(c) => {
            // Direct hotkey press
            let found = app.action_menu_items.iter().any(|item| item.hotkey == c);
            if found {
                app.input_mode = InputMode::Normal;
                execute_action_hotkey(app, c, bg_tx).await;
            }
        }
        _ => {}
    }
}

async fn execute_action_hotkey(app: &mut App, hotkey: char, bg_tx: &mpsc::UnboundedSender<BgMessage>) {
    match hotkey {
        's' => {
            if let Some(c) = app.selected_container() {
                let id = c.id.clone();
                app.loading = true;
                match wslc::commands::start_container(&id).await {
                    Ok(_) => {
                        app.set_flash("Container started".into());
                        spawn_refresh(bg_tx.clone());
                    }
                    Err(e) => {
                        app.set_flash(format!("Error: {}", e));
                        app.loading = false;
                    }
                }
            }
        }
        'S' => {
            if let Some(c) = app.selected_container() {
                let id = c.id.clone();
                app.loading = true;
                match wslc::commands::stop_container(&id).await {
                    Ok(_) => {
                        app.set_flash("Container stopped".into());
                        spawn_refresh(bg_tx.clone());
                    }
                    Err(e) => {
                        app.set_flash(format!("Error: {}", e));
                        app.loading = false;
                    }
                }
            }
        }
        'K' => {
            if let Some(c) = app.selected_container() {
                let id = c.id.clone();
                app.loading = true;
                match wslc::commands::kill_container(&id).await {
                    Ok(_) => {
                        app.set_flash("Container killed".into());
                        spawn_refresh(bg_tx.clone());
                    }
                    Err(e) => {
                        app.set_flash(format!("Error: {}", e));
                        app.loading = false;
                    }
                }
            }
        }
        'x' => {
            prompt_remove(app);
        }
        'l' => {
            app.detail_tab = DetailTab::Main;
            load_logs_for_selected(app).await;
        }
        'p' => {
            // Prune from action menu — delegate to same logic as the 'p' normal key
            match app.active_section {
                ResourceSection::Containers => {
                    let stopped_count = app.containers.iter().filter(|c| !c.is_running()).count();
                    if stopped_count == 0 {
                        app.set_flash("No stopped containers to prune".into());
                    } else {
                        app.confirm_message = format!(
                            "Prune {} stopped container{}? This cannot be undone.",
                            stopped_count,
                            if stopped_count == 1 { "" } else { "s" }
                        );
                        app.confirm_action = Some(ConfirmAction::PruneContainers);
                        app.input_mode = InputMode::Confirm;
                    }
                }
                ResourceSection::Images => {
                    app.confirm_message = "Prune all dangling images? This cannot be undone.".into();
                    app.confirm_action = Some(ConfirmAction::PruneImages);
                    app.input_mode = InputMode::Confirm;
                }
                ResourceSection::Volumes => {
                    app.set_flash("Prune not available for volumes".into());
                }
            }
        }
        'P' => {
            app.pull_input.clear();
            app.input_mode = InputMode::PullInput;
        }
        _ => {}
    }
}

async fn handle_pull_input_key(app: &mut App, code: KeyCode, bg_tx: &mpsc::UnboundedSender<BgMessage>) {
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
                        spawn_refresh(bg_tx.clone());
                    }
                    Err(e) => {
                        app.set_flash(format!("Pull failed: {}", e));
                        app.loading = false;
                    }
                }
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

async fn handle_mouse(
    app: &mut App,
    kind: MouseEventKind,
    col: u16,
    row: u16,
    size: ratatui::layout::Rect,
) {
    let areas = ui::layout::compute_areas(app, size);

    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Click in container panel
            if rect_contains(&areas.container_inner, col, row) {
                let rel_row = (row - areas.container_inner.y) as usize;
                let len = app.filtered_containers().len();
                let sel = if app.active_section == ResourceSection::Containers { app.container_index } else { usize::MAX };
                if let Some(idx) = row_to_item_index(rel_row, sel, len) {
                    app.active_section = ResourceSection::Containers;
                    app.container_index = idx;
                    app.detail_tab = app.default_tab();
                    app.info_scroll = 0;
                    load_inspect_for_selected(app).await;
                }
            }
            // Click in image panel
            else if rect_contains(&areas.image_inner, col, row) {
                let rel_row = (row - areas.image_inner.y) as usize;
                let len = app.filtered_images().len();
                let sel = if app.active_section == ResourceSection::Images { app.image_index } else { usize::MAX };
                if let Some(idx) = row_to_item_index(rel_row, sel, len) {
                    app.active_section = ResourceSection::Images;
                    app.image_index = idx;
                    app.detail_tab = app.default_tab();
                    app.info_scroll = 0;
                    load_inspect_for_selected(app).await;
                }
            }
            // Click in volume panel (volumes are still 1 row each)
            else if rect_contains(&areas.volume_inner, col, row) {
                let idx = (row - areas.volume_inner.y) as usize;
                if idx < app.filtered_volumes().len() {
                    app.active_section = ResourceSection::Volumes;
                    app.volume_index = idx;
                    app.detail_tab = app.default_tab();
                    app.info_scroll = 0;
                    load_inspect_for_selected(app).await;
                }
            }
            // Click on detail tab bar
            else if rect_contains(&areas.tab_bar, col, row) {
                handle_tab_click(app, col, &areas);
            }
        }
        MouseEventKind::ScrollUp => {
            if rect_contains(&areas.container_inner, col, row)
                || rect_contains(&areas.image_inner, col, row)
                || rect_contains(&areas.volume_inner, col, row)
            {
                app.move_up();
            } else if rect_contains(&areas.detail_area, col, row) {
                match app.detail_tab {
                    DetailTab::Main => app.logs_scroll = app.logs_scroll.saturating_add(3),
                    DetailTab::Info => app.info_scroll = app.info_scroll.saturating_sub(3),
                    _ => {}
                }
            }
        }
        MouseEventKind::ScrollDown => {
            if rect_contains(&areas.container_inner, col, row)
                || rect_contains(&areas.image_inner, col, row)
                || rect_contains(&areas.volume_inner, col, row)
            {
                app.move_down();
            } else if rect_contains(&areas.detail_area, col, row) {
                match app.detail_tab {
                    DetailTab::Main => app.logs_scroll = app.logs_scroll.saturating_sub(3),
                    DetailTab::Info => app.info_scroll = app.info_scroll.saturating_add(3),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn rect_contains(rect: &ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// Map a click row to an item index, accounting for the selected item taking 2 rows.
/// Items before the selected index occupy 1 row each, the selected item occupies 2,
/// and items after continue at 1 row each.
fn row_to_item_index(rel_row: usize, selected: usize, count: usize) -> Option<usize> {
    let mut row_acc = 0;
    for i in 0..count {
        let height = if i == selected { 2 } else { 1 };
        if rel_row >= row_acc && rel_row < row_acc + height {
            return Some(i);
        }
        row_acc += height;
    }
    None
}

fn handle_tab_click(app: &mut App, col: u16, areas: &ui::layout::LayoutAreas) {
    let tab_titles: Vec<&str> = match app.active_section {
        ResourceSection::Containers => vec!["Main", "Info", "Env"],
        ResourceSection::Images => vec!["Info", "Env"],
        ResourceSection::Volumes => vec!["Info"],
    };

    let relative_x = (col - areas.tab_bar.x) as usize;
    // Ratatui Tabs renders each tab as: " {title} " (1 char pad each side)
    // with divider " │ " (3 chars) between them.
    // So each tab region = 1 + title.len() + 1, then 3 for divider.
    let mut pos = 0;
    for (i, title) in tab_titles.iter().enumerate() {
        let tab_width = 1 + title.len() + 1; // padding + title + padding
        let end = pos + tab_width;
        if relative_x >= pos && relative_x < end {
            let tab = match app.active_section {
                ResourceSection::Containers => match i {
                    0 => DetailTab::Main,
                    1 => DetailTab::Info,
                    _ => DetailTab::Env,
                },
                ResourceSection::Images => match i {
                    0 => DetailTab::Info,
                    _ => DetailTab::Env,
                },
                ResourceSection::Volumes => DetailTab::Info,
            };
            app.detail_tab = tab;
            app.info_scroll = 0;
            return;
        }
        pos = end + 3; // " │ " divider
    }
}

// ---------------------------------------------------------------------------
// Background task spawners
// ---------------------------------------------------------------------------

fn spawn_refresh(tx: mpsc::UnboundedSender<BgMessage>) {
    tokio::spawn(async move {
        let (containers, images, volumes) = tokio::join!(
            wslc::commands::list_containers(),
            wslc::commands::list_images(),
            wslc::commands::list_volumes(),
        );
        let _ = tx.send(BgMessage::DataRefreshed {
            containers: containers.unwrap_or_default(),
            images: images.unwrap_or_default(),
            volumes: volumes.unwrap_or_default(),
        });
    });
}

fn spawn_stats(tx: mpsc::UnboundedSender<BgMessage>, container_id: String) {
    tokio::spawn(async move {
        let text = wslc::commands::container_stats(&container_id)
            .await
            .unwrap_or_default();
        let _ = tx.send(BgMessage::StatsLoaded { container_id, text });
    });
}

fn spawn_logs(tx: mpsc::UnboundedSender<BgMessage>, container_id: String) {
    tokio::spawn(async move {
        let text = wslc::commands::container_logs(&container_id, 200)
            .await
            .unwrap_or_default();
        let _ = tx.send(BgMessage::LogsLoaded { text });
    });
}

// ---------------------------------------------------------------------------
// Background message handler
// ---------------------------------------------------------------------------

fn handle_bg_message(app: &mut App, msg: BgMessage) {
    match msg {
        BgMessage::DataRefreshed { containers, images, volumes } => {
            app.containers = containers;
            let mut imgs = images;
            imgs.sort_by(|a, b| a.display_name().cmp(&b.display_name()));
            app.images = imgs;
            let mut vols = volumes;
            vols.sort_by(|a, b| a.name.cmp(&b.name));
            app.volumes = vols;
            app.clamp_indices();
            app.loading = false;
            // Hide splash screen now that we have data
            app.show_splash = false;
            // Note: We don't load inspect here to keep message handler fast.
            // It will be loaded on-demand when user navigates.
        }
        BgMessage::StatsLoaded { container_id, text } => {
            app.stats_text = text.clone();
            let trimmed = text.trim();
            if let Ok(stats) = serde_json::from_str::<wslc::types::Stats>(trimmed) {
                let cpu_val = parse_percent(stats.cpu_perc.as_deref().unwrap_or("0"));
                let mem_val = parse_percent(stats.mem_perc.as_deref().unwrap_or("0"));
                app.push_stats_sample(&container_id, cpu_val, mem_val);
                app.current_stats = Some(stats);
            } else if let Ok(stats_arr) = serde_json::from_str::<Vec<wslc::types::Stats>>(trimmed) {
                if let Some(stats) = stats_arr.into_iter().next() {
                    let cpu_val = parse_percent(stats.cpu_perc.as_deref().unwrap_or("0"));
                    let mem_val = parse_percent(stats.mem_perc.as_deref().unwrap_or("0"));
                    app.push_stats_sample(&container_id, cpu_val, mem_val);
                    app.current_stats = Some(stats);
                }
            }
        }
        BgMessage::LogsLoaded { text } => {
            app.logs_text = text;
            app.logs_scroll = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers (still used inline for navigation / on-demand loads)
// ---------------------------------------------------------------------------

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
