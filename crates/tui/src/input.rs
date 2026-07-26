//! Key event handling for the TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Size;

use crate::app::{App, Focus, TabKind};

const BACK_TO_LATEST: &str = " Back to latest (End) ";

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

    // Ctrl+L cycles the TUI capture level through info/debug/trace.
    if ctrl && matches!(key.code, KeyCode::Char('l') | KeyCode::Char('L')) {
        app.cycle_log_level();
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
        KeyCode::PageUp => {
            app.scroll_up(10);
            return;
        }
        KeyCode::PageDown => {
            app.scroll_down(10);
            return;
        }
        KeyCode::End => {
            app.follow_latest();
            return;
        }
        KeyCode::Up if ctrl => {
            app.scroll_up(1);
            return;
        }
        KeyCode::Down if ctrl => {
            app.scroll_down(1);
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
            KeyCode::Char('q') if app.input.is_empty() => app.request_quit(),
            KeyCode::Char(c) => app.push_char(c),
            _ => {}
        },
    }
}

pub fn handle_mouse(app: &mut App, mouse: MouseEvent, size: Size) {
    if app.confirm_quit {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) if mouse.row == 0 => {
            if let Some(idx) = tab_at(app, mouse.column) {
                app.select_tab(idx);
            }
        }
        MouseEventKind::ScrollUp if over_logs(app, size, mouse.column, mouse.row) => {
            app.scroll_up(3);
        }
        MouseEventKind::ScrollDown if over_logs(app, size, mouse.column, mouse.row) => {
            app.scroll_down(3);
        }
        MouseEventKind::Down(MouseButton::Left)
            if over_back_to_latest(app, size, mouse.column, mouse.row) =>
        {
            app.follow_latest();
        }
        _ => {}
    }
}

fn tab_at(app: &App, x: u16) -> Option<usize> {
    let mut start = 0u16;
    for (idx, title) in app.tab_titles().iter().enumerate() {
        let width = title.chars().count() as u16 + 2;
        if x >= start && x < start.saturating_add(width) {
            return Some(idx);
        }
        start = start.saturating_add(width);
    }
    None
}

fn over_logs(app: &App, size: Size, x: u16, y: u16) -> bool {
    if matches!(app.current_tab(), TabKind::Performance) || size.height <= 4 {
        return false;
    }

    let body_y = 1;
    let body_h = size.height.saturating_sub(4);
    let log_w = size.width.saturating_sub(34);
    if x >= log_w || y < body_y || y >= body_y.saturating_add(body_h) {
        return false;
    }

    let log_y = if app.show_logo() { body_y + 4 } else { body_y };
    y >= log_y && y < body_y.saturating_add(body_h)
}

fn over_back_to_latest(app: &App, size: Size, x: u16, y: u16) -> bool {
    if app.follow_logs || matches!(app.current_tab(), TabKind::Performance) || size.height <= 4 {
        return false;
    }

    let log_width = size.width.saturating_sub(34);
    let button_width = BACK_TO_LATEST.chars().count() as u16;
    let log_y = if app.show_logo() { 5 } else { 1 };
    x >= log_width.saturating_sub(button_width + 1) && x < log_width && y == log_y
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyEvent, KeyModifiers};

    use super::*;
    use crate::log_layer::LogStore;
    use crate::logging::LogControl;

    fn app() -> App {
        let mut app = App::new(Vec::new(), LogStore::new(10), LogControl::new(2));
        app.tabs = vec![TabKind::Both];
        app
    }

    #[test]
    fn end_returns_to_latest_logs() {
        let mut app = app();
        app.scroll_up(5);

        handle_key(&mut app, KeyEvent::new(KeyCode::End, KeyModifiers::NONE));

        assert_eq!(app.scroll, 0);
        assert!(app.follow_logs);
    }

    #[test]
    fn back_to_latest_button_is_clickable() {
        let mut app = app();
        app.scroll_up(5);
        let size = Size::new(120, 30);
        let x = size.width.saturating_sub(34 + BACK_TO_LATEST.len() as u16);
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };

        handle_mouse(&mut app, mouse, size);

        assert!(app.follow_logs);
    }
}
