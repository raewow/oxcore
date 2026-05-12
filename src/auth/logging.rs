use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::auth::config::Config;

pub fn init_basic() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(true)
        .with_target(false)
        .init();

    Ok(())
}

pub fn init(config: &Config) -> Result<()> {
    let console_log_level = match config.log_level {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 => "trace",
        _ => "info",
    };

    let file_log_level = match config.log_file_level {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 => "trace",
        _ => "error",
    };

    let console_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(console_log_level));
    let file_filter = EnvFilter::new(file_log_level);

    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(false)
        .with_filter(console_filter);

    if !config.log_file.is_empty() {
        let log_path = resolve_log_path(&config.logs_dir, &config.log_file)
            .context("Failed to resolve log file path")?;

        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create log directory: {}", parent.display()))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("Failed to open log file: {}", log_path.display()))?;

        let file_layer = fmt::layer()
            .with_writer(file)
            .with_ansi(false)
            .with_target(false)
            .with_filter(file_filter);

        tracing_subscriber::registry()
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry().with(console_layer).init();
    }

    Ok(())
}

fn resolve_log_path(logs_dir: &Path, log_file: &str) -> Result<PathBuf> {
    if logs_dir.as_os_str().is_empty() {
        Ok(PathBuf::from(log_file))
    } else {
        Ok(logs_dir.join(log_file))
    }
}
