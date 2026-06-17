//! Subscriber installation for both TUI and headless modes.
//!
//! Exactly one of these must be called once at process startup, before any server
//! task begins logging.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
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

#[derive(Clone)]
pub struct LogControl {
    level: Arc<AtomicU8>,
}

impl LogControl {
    pub fn new(level: u8) -> Self {
        Self {
            level: Arc::new(AtomicU8::new(normalize_level(level))),
        }
    }

    pub fn level(&self) -> u8 {
        self.level.load(Ordering::Relaxed)
    }

    pub fn level_name(&self) -> &'static str {
        level_str(self.level())
    }

    pub fn set_level(&self, level: u8) {
        self.level.store(normalize_level(level), Ordering::Relaxed);
    }

    pub fn cycle_debug_levels(&self) -> u8 {
        let next = match self.level() {
            2 => 3,
            3 => 4,
            _ => 2,
        };
        self.set_level(next);
        next
    }

    pub(crate) fn shared_level(&self) -> Arc<AtomicU8> {
        self.level.clone()
    }
}

fn normalize_level(level: u8) -> u8 {
    level.min(4)
}

pub fn level_str(level: u8) -> &'static str {
    match level {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 => "trace",
        _ => "info",
    }
}

pub fn parse_level(level: &str) -> Option<u8> {
    match level.to_ascii_lowercase().as_str() {
        "error" | "err" => Some(0),
        "warn" | "warning" => Some(1),
        "info" => Some(2),
        "debug" => Some(3),
        "trace" => Some(4),
        _ => None,
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
pub fn install_tui_subscriber(store: Arc<LogStore>, settings: &LogSettings) -> Result<LogControl> {
    let control = LogControl::new(settings.console_level);
    let tui_layer = TuiLogLayer::new(store, control.shared_level());

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

    Ok(control)
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
