//! Subscriber installation for both TUI and headless modes.
//!
//! Exactly one of these must be called once at process startup, before any server
//! task begins logging.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::log_layer::{LogStore, TuiLogLayer};

/// Logging configuration mapped from the server config.
#[derive(Clone)]
pub struct LogSettings {
    /// Numeric console level (0=error .. 4=trace).
    pub console_level: u8,
    /// Numeric file level (0=error .. 4=trace).
    pub file_level: u8,
    /// Optional resolved log file path (already joined with logs_dir).
    pub log_file: Option<PathBuf>,
    /// Truncate the log file on start instead of appending.
    pub wipe: bool,
}

fn level_str(level: u8) -> &'static str {
    match level {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 => "trace",
        _ => "info",
    }
}

fn open_log_file(path: &Path, wipe: bool) -> Result<std::fs::File> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create log directory: {}", parent.display()))?;
        }
    }
    OpenOptions::new()
        .create(true)
        .write(true)
        .append(!wipe)
        .truncate(wipe)
        .open(path)
        .with_context(|| format!("Failed to open log file: {}", path.display()))
}

/// Install the TUI subscriber: log lines are captured into `store` (no stderr output),
/// with an optional non-ANSI file layer.
pub fn install_tui_subscriber(store: Arc<LogStore>, settings: &LogSettings) -> Result<()> {
    let console_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level_str(settings.console_level)));
    let tui_layer = TuiLogLayer::new(store).with_filter(console_filter);

    if let Some(path) = &settings.log_file {
        let file = open_log_file(path, settings.wipe)?;
        let file_layer = fmt::layer()
            .with_writer(file)
            .with_ansi(false)
            .with_target(false)
            .with_filter(EnvFilter::new(level_str(settings.file_level)));
        tracing_subscriber::registry()
            .with(tui_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry().with(tui_layer).init();
    }

    Ok(())
}

/// Install the headless subscriber: ANSI stderr output plus an optional file layer
/// (the pre-TUI behaviour, for systemd/CI/piped logs).
pub fn install_headless_subscriber(settings: &LogSettings) -> Result<()> {
    let console_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level_str(settings.console_level)));
    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(false)
        .with_filter(console_filter);

    if let Some(path) = &settings.log_file {
        let file = open_log_file(path, settings.wipe)?;
        let file_layer = fmt::layer()
            .with_writer(file)
            .with_ansi(false)
            .with_target(false)
            .with_filter(EnvFilter::new(level_str(settings.file_level)));
        tracing_subscriber::registry()
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry().with(console_layer).init();
    }

    Ok(())
}
