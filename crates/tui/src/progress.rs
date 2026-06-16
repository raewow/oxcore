//! A cheap, shared progress handle the slow server (world) drives during startup and the
//! loading screen reads. Generic: it knows nothing about what is being loaded.

use std::sync::Arc;

use parking_lot::Mutex;

struct ProgressInner {
    current: u32,
    total: u32,
    label: String,
    done: bool,
}

/// Cloneable progress handle (all clones share one state).
#[derive(Clone)]
pub struct Progress {
    inner: Arc<Mutex<ProgressInner>>,
}

/// A point-in-time view of progress for rendering.
#[derive(Clone)]
pub struct ProgressSnapshot {
    pub current: u32,
    pub total: u32,
    pub label: String,
    pub done: bool,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProgressInner {
                current: 0,
                total: 0,
                label: String::new(),
                done: false,
            })),
        }
    }

    /// Set the total number of steps (enables the determinate bar).
    pub fn set_total(&self, n: u32) {
        self.inner.lock().total = n;
    }

    /// Update the label without advancing the step counter.
    pub fn set_label(&self, label: impl Into<String>) {
        self.inner.lock().label = label.into();
    }

    /// Advance to the next step and set its label.
    pub fn step(&self, label: impl Into<String>) {
        let mut g = self.inner.lock();
        g.current = g.current.saturating_add(1);
        g.label = label.into();
    }

    /// Mark complete (current = total).
    pub fn finish(&self) {
        let mut g = self.inner.lock();
        g.current = g.total;
        g.done = true;
    }

    pub fn snapshot(&self) -> ProgressSnapshot {
        let g = self.inner.lock();
        ProgressSnapshot {
            current: g.current,
            total: g.total,
            label: g.label.clone(),
            done: g.done,
        }
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}
