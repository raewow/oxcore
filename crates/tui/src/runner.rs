//! Terminal setup and the async TUI event loop (loading + running phases).

use std::io::Stdout;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{broadcast, mpsc};

use crate::app::{App, ServerPane};
use crate::input;
use crate::log_layer::LogStore;
use crate::logging::LogControl;
use crate::progress::Progress;
use crate::ui;

/// Messages sent from the background startup task to the loading screen.
pub enum LoadUpdate {
    /// High-level status line (e.g. "auth ready").
    Status(String),
    /// Servers are up; switch to the running view with these panes.
    Ready(Vec<ServerPane>),
    /// Startup failed; show the error and exit.
    Failed(String),
}

enum Phase {
    Loading {
        status: String,
        error: Option<String>,
    },
    Running(App),
}

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

fn is_quit_key(key: &crossterm::event::KeyEvent) -> bool {
    (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        || matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
}

/// Paint the TUI immediately (loading screen) while servers initialise in the background,
/// then swap to the tabbed running view once panes arrive. Broadcasts shutdown on quit.
pub async fn run_tui_loading(
    store: Arc<LogStore>,
    log_control: LogControl,
    progress: Progress,
    shutdown_tx: broadcast::Sender<()>,
    mut updates: mpsc::Receiver<LoadUpdate>,
) -> Result<()> {
    let _guard = TermGuard;
    let mut terminal = setup_terminal()?;

    let mut phase = Phase::Loading {
        status: "starting…".to_string(),
        error: None,
    };
    let mut spinner: usize = 0;
    let mut quit = false;
    let mut failure: Option<String> = None;

    let mut events = EventStream::new();
    let mut log_updates = store.subscribe();
    let mut render_tick = tokio::time::interval(Duration::from_millis(50));
    let mut sample_tick = tokio::time::interval(Duration::from_secs(1));

    loop {
        let mut needs_draw = false;

        tokio::select! {
            _ = render_tick.tick() => {
                spinner = spinner.wrapping_add(1);
                needs_draw = true;
            }
            result = log_updates.changed() => {
                if result.is_ok() {
                    needs_draw = true;
                }
            }
            _ = sample_tick.tick() => {
                if let Phase::Running(app) = &mut phase {
                    app.sample();
                    needs_draw = true;
                }
            }
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        match &mut phase {
                            Phase::Running(app) => {
                                input::handle_key(app, key);
                                if app.should_quit {
                                    quit = true;
                                }
                                needs_draw = true;
                            }
                            Phase::Loading { error, .. } => {
                                // Any key dismisses an error; otherwise only quit keys exit.
                                if error.is_some() {
                                    failure = error.clone();
                                    quit = true;
                                } else if is_quit_key(&key) {
                                    quit = true;
                                }
                                needs_draw = true;
                            }
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
        }

        if needs_draw {
            // Drain any pending load updates while still loading.
            if let Phase::Loading { status, error } = &mut phase {
                while let Ok(update) = updates.try_recv() {
                    match update {
                        LoadUpdate::Status(s) => *status = s,
                        LoadUpdate::Failed(e) => *error = Some(e),
                        LoadUpdate::Ready(panes) => {
                            phase =
                                Phase::Running(App::new(panes, store.clone(), log_control.clone()));
                            break;
                        }
                    }
                }
            }

            match &mut phase {
                Phase::Loading { status, error } => {
                    let snap = progress.snapshot();
                    terminal.draw(|f| {
                        ui::render_loading(f, &store, &snap, status, error.as_deref(), spinner)
                    })?;
                }
                Phase::Running(app) => {
                    terminal.draw(|f| ui::render(f, app))?;
                }
            }
        }

        if quit {
            break;
        }
    }

    let _ = shutdown_tx.send(());

    if let Some(msg) = failure {
        return Err(anyhow::anyhow!(msg));
    }
    Ok(())
}
