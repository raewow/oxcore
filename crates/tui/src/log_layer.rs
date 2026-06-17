//! Tracing layer that captures log events into an in-memory ring buffer for the TUI.
//!
//! Events are routed to a logical [`LogSource`] by their target (module path) so the
//! unified runtime can show auth/world logs in separate tabs while running in one process.

use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::watch;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Which server a log line originated from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogSource {
    Auth,
    World,
    /// Shared/runtime/other crates.
    Other,
}

impl LogSource {
    pub fn tag(&self) -> &'static str {
        match self {
            LogSource::Auth => "auth",
            LogSource::World => "world",
            LogSource::Other => "core",
        }
    }
}

/// Filter applied when reading records for a given tab.
#[derive(Clone, Copy)]
pub enum LogFilter {
    /// Show everything (the "Both" tab / standalone single-pane view).
    All,
    /// Show records from this source plus shared/other records.
    Source(LogSource),
}

impl LogFilter {
    fn matches(&self, src: LogSource) -> bool {
        match self {
            LogFilter::All => true,
            LogFilter::Source(s) => src == *s || src == LogSource::Other,
        }
    }
}

/// A single captured log line.
#[derive(Clone)]
pub struct LogRecord {
    pub source: LogSource,
    pub level: Level,
    pub time: String,
    pub message: String,
}

/// Bounded, shared ring buffer of log records.
pub struct LogStore {
    records: Mutex<VecDeque<LogRecord>>,
    capacity: usize,
    revision: AtomicU64,
    updates: watch::Sender<u64>,
}

impl LogStore {
    pub fn new(capacity: usize) -> Arc<Self> {
        let (updates, _) = watch::channel(0);
        Arc::new(Self {
            records: Mutex::new(VecDeque::with_capacity(capacity.min(1024))),
            capacity,
            revision: AtomicU64::new(0),
            updates,
        })
    }

    pub fn push(&self, rec: LogRecord) {
        {
            let mut q = self.records.lock();
            if q.len() >= self.capacity {
                q.pop_front();
            }
            q.push_back(rec);
        }

        let revision = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.updates.send(revision);
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.updates.subscribe()
    }

    /// Push a synthetic line (e.g. an echo of a typed console command).
    pub fn push_synthetic(&self, source: LogSource, level: Level, message: String) {
        self.push(LogRecord {
            source,
            level,
            time: now_string(),
            message,
        });
    }

    /// Collect records matching `filter`, in chronological order.
    pub fn filtered(&self, filter: LogFilter) -> Vec<LogRecord> {
        let q = self.records.lock();
        q.iter()
            .filter(|r| filter.matches(r.source))
            .cloned()
            .collect()
    }
}

fn now_string() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}

/// The tracing layer. Attach with a level filter via `.with_filter(..)`.
pub struct TuiLogLayer {
    store: Arc<LogStore>,
}

impl TuiLogLayer {
    pub fn new(store: Arc<LogStore>) -> Self {
        Self { store }
    }
}

fn source_for_target(target: &str) -> LogSource {
    if target.starts_with("oxcore_auth") {
        LogSource::Auth
    } else if target.starts_with("oxcore_world") {
        LogSource::World
    } else {
        LogSource::Other
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            // `message` is recorded as the Debug of format_args!, which is the
            // already-formatted string with no surrounding quotes.
            self.message = format!("{:?}", value);
        } else {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            self.fields
                .push_str(&format!("{}={:?}", field.name(), value));
        }
    }
}

impl<S: Subscriber> Layer<S> for TuiLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let mut message = visitor.message;
        if !visitor.fields.is_empty() {
            if !message.is_empty() {
                message.push(' ');
            }
            message.push_str(&visitor.fields);
        }

        self.store.push(LogRecord {
            source: source_for_target(meta.target()),
            level: *meta.level(),
            time: now_string(),
            message,
        });
    }
}
