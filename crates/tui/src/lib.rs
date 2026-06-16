//! Shared ratatui terminal UI for oxcore servers.
//!
//! Used by the unified `oxcore` runtime (Auth + World + Performance tabs) and by the
//! standalone `auth` / `world` binaries (single-pane view). The UI is generic: it knows
//! nothing about auth/world internals beyond the [`MetricsSource`] trait and a console
//! command channel.

pub mod app;
pub mod input;
pub mod log_layer;
pub mod logging;
pub mod metrics;
pub mod runner;
pub mod ui;

pub use app::ServerPane;
pub use log_layer::{LogFilter, LogRecord, LogSource, LogStore, TuiLogLayer};
pub use logging::{install_headless_subscriber, install_tui_subscriber, LogSettings};
pub use metrics::{MetricsSnapshot, MetricsSource};
pub use runner::run_tui;
pub use ui::LOGO;
