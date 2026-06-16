//! Key event handling for the TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Focus};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Quit-confirmation popup captures all input while open.
    if app.confirm_quit {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm_quit_yes(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_quit(),
            _ => {}
        }
        return;
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Ctrl+C asks to quit (now via the confirmation popup).
    if ctrl && key.code == KeyCode::Char('c') {
        app.request_quit();
        return;
    }

    // Ctrl+F toggles the log filter input.
    if ctrl && key.code == KeyCode::Char('f') {
        app.toggle_filter_focus();
        return;
    }

    // Tab switching works regardless of focus.
    match key.code {
        KeyCode::Tab => {
            app.next_tab();
            return;
        }
        KeyCode::BackTab => {
            app.prev_tab();
            return;
        }
        _ => {}
    }

    match app.focus {
        Focus::Filter => match key.code {
            KeyCode::Enter => app.toggle_filter_focus(), // confirm, keep filter
            KeyCode::Esc => app.clear_filter(),
            KeyCode::Backspace => app.backspace(),
            KeyCode::Char(c) => app.push_char(c),
            _ => {}
        },
        Focus::Command => match key.code {
            KeyCode::Esc => app.request_quit(),
            KeyCode::Enter => app.submit(),
            KeyCode::Backspace => app.backspace(),
            KeyCode::Delete => app.delete_char(),
            KeyCode::Left => app.cursor_left(),
            KeyCode::Right => app.cursor_right(),
            KeyCode::Home => app.cursor_home(),
            KeyCode::End => app.cursor_end(),
            KeyCode::Up => app.history_up(),
            KeyCode::Down => app.history_down(),
            KeyCode::PageUp => app.scroll_up(10),
            KeyCode::PageDown => app.scroll_down(10),
            KeyCode::Char('q') if app.input.is_empty() => app.request_quit(),
            KeyCode::Char(c) => app.push_char(c),
            _ => {}
        },
    }
}
