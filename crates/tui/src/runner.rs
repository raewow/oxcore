//! Terminal setup and the async TUI event loop.

use std::io::Stdout;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::broadcast;

use crate::app::{App, ServerPane};
use crate::input;
use crate::log_layer::LogStore;
use crate::ui;

/// Restores the terminal to a sane state on drop (including on panic/early return).
struct TermGuard;

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Run the TUI until the user quits, then broadcast shutdown to the servers.
pub async fn run_tui(
    panes: Vec<ServerPane>,
    store: Arc<LogStore>,
    shutdown_tx: broadcast::Sender<()>,
) -> Result<()> {
    let _guard = TermGuard;
    let mut terminal = setup_terminal()?;

    let mut app = App::new(panes, store);
    let mut events = EventStream::new();
    let mut render_tick = tokio::time::interval(Duration::from_millis(50));
    let mut sample_tick = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = render_tick.tick() => {
                terminal.draw(|f| ui::render(f, &mut app))?;
            }
            _ = sample_tick.tick() => {
                app.sample();
            }
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        input::handle_key(&mut app, key);
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    let _ = shutdown_tx.send(());
    Ok(())
}
