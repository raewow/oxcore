//! Generic metrics abstraction consumed by the TUI status box and Performance tab.
//!
//! Each server implements [`MetricsSource`] in its own crate (auth maps its atomic
//! `Metrics`, world reads its session/player managers and tick stats).

/// A point-in-time view of one server's headline metrics.
#[derive(Clone, Default)]
pub struct MetricsSnapshot {
    /// Active connections / sessions.
    pub connections: u64,
    /// Online players (0 for auth).
    pub players_online: u64,
    /// World ticks per second (0 for auth).
    pub tps: f64,
    /// Last world update-loop duration in milliseconds (0 for auth).
    pub tick_ms: f64,
    /// Named extra rows rendered in the status box / perf tab.
    pub gauges: Vec<(String, String)>,
}

/// Implemented by each server to expose live metrics to the TUI.
pub trait MetricsSource: Send + Sync {
    fn snapshot(&self) -> MetricsSnapshot;
}
