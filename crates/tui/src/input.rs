//! Key event handling for the TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Ctrl+C always quits (in raw mode it arrives as a key event).
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match key.code {
        KeyCode::Esc => app.should_quit = true,
        KeyCode::Tab => app.next_tab(),
        KeyCode::BackTab => app.prev_tab(),
        KeyCode::Enter => app.submit(),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Right => app.accept_suggestion(),
        KeyCode::Up => app.history_up(),
        KeyCode::Down => app.history_down(),
        KeyCode::PageUp => app.scroll_up(10),
        KeyCode::PageDown => app.scroll_down(10),
        KeyCode::Char('q') if app.input.is_empty() => app.should_quit = true,
        KeyCode::Char(c) => app.push_char(c),
        _ => {}
    }
}
