use crossterm::event::{self, Event, KeyEvent, KeyCode, KeyModifiers, MouseEvent};
use std::time::Duration;

pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    Resize(u16, u16),
}

pub fn poll_event(tick_rate: Duration) -> anyhow::Result<AppEvent> {
    if event::poll(tick_rate)? {
        match event::read()? {
            Event::Key(key) => {
                // Ignore key release events on Windows
                if key.kind == crossterm::event::KeyEventKind::Press {
                    return Ok(AppEvent::Key(key));
                }
            }
            Event::Mouse(mouse) => return Ok(AppEvent::Mouse(mouse)),
            Event::Resize(w, h) => return Ok(AppEvent::Resize(w, h)),
            _ => {}
        }
    }
    Ok(AppEvent::Tick)
}

pub fn is_quit(key: &KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('q'), KeyModifiers::NONE)
            | (KeyCode::Char('c'), KeyModifiers::CONTROL)
    )
}
