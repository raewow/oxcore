//! TUI application state and input/routing logic (rendering lives in `ui`).

use std::collections::VecDeque;
use std::sync::Arc;

use oxcore_shared::console::ConsoleCommand;
use tokio::sync::mpsc;
use tracing::Level;

use crate::log_layer::{LogFilter, LogSource, LogStore};
use crate::metrics::{MetricsSnapshot, MetricsSource};

const HISTORY_LEN: usize = 120;

/// One server exposed in the UI.
pub struct ServerPane {
    /// Display name, e.g. "Auth" / "World".
    pub name: String,
    pub source: LogSource,
    pub metrics: Box<dyn MetricsSource>,
    pub cmd_tx: mpsc::Sender<ConsoleCommand>,
    /// Registered command names, for autosuggest.
    pub commands: Vec<String>,
}

/// A bounded numeric series for sparklines.
pub struct Series {
    data: VecDeque<u64>,
}

impl Series {
    fn new() -> Self {
        Self {
            data: VecDeque::with_capacity(HISTORY_LEN),
        }
    }

    fn push(&mut self, v: u64) {
        if self.data.len() >= HISTORY_LEN {
            self.data.pop_front();
        }
        self.data.push_back(v);
    }

    pub fn values(&self) -> Vec<u64> {
        self.data.iter().copied().collect()
    }
}

/// Per-pane history for the Performance tab.
pub struct PaneSeries {
    pub connections: Series,
    pub players: Series,
    pub tps: Series,
    pub tick_ms_x100: Series,
}

impl PaneSeries {
    fn new() -> Self {
        Self {
            connections: Series::new(),
            players: Series::new(),
            tps: Series::new(),
            tick_ms_x100: Series::new(),
        }
    }
}

/// Logical tabs, built dynamically from the panes.
#[derive(Clone, Copy, PartialEq)]
pub enum TabKind {
    Both,
    Pane(usize),
    Performance,
}

pub struct App {
    pub panes: Vec<ServerPane>,
    pub store: Arc<LogStore>,
    pub tabs: Vec<TabKind>,
    pub active: usize,
    pub input: String,
    pub history: Vec<String>,
    pub history_cursor: Option<usize>,
    /// Lines scrolled up from the bottom of the log view.
    pub scroll: usize,
    pub should_quit: bool,
    pub perf: Vec<PaneSeries>,
}

impl App {
    pub fn new(panes: Vec<ServerPane>, store: Arc<LogStore>) -> Self {
        let mut tabs = Vec::new();
        if panes.len() > 1 {
            tabs.push(TabKind::Both);
        }
        for i in 0..panes.len() {
            tabs.push(TabKind::Pane(i));
        }
        tabs.push(TabKind::Performance);

        let perf = panes.iter().map(|_| PaneSeries::new()).collect();

        Self {
            panes,
            store,
            tabs,
            active: 0,
            input: String::new(),
            history: Vec::new(),
            history_cursor: None,
            scroll: 0,
            should_quit: false,
            perf,
        }
    }

    pub fn current_tab(&self) -> TabKind {
        self.tabs[self.active]
    }

    pub fn tab_titles(&self) -> Vec<String> {
        self.tabs
            .iter()
            .map(|t| match t {
                TabKind::Both => "Both".to_string(),
                TabKind::Pane(i) => self.panes[*i].name.clone(),
                TabKind::Performance => "Performance".to_string(),
            })
            .collect()
    }

    pub fn next_tab(&mut self) {
        self.active = (self.active + 1) % self.tabs.len();
        self.scroll = 0;
    }

    pub fn prev_tab(&mut self) {
        self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
        self.scroll = 0;
    }

    /// The log filter for the current tab.
    pub fn log_filter(&self) -> LogFilter {
        match self.current_tab() {
            TabKind::Pane(i) => LogFilter::Source(self.panes[i].source),
            _ => LogFilter::All,
        }
    }

    /// Whether the OXCORE logo should be shown. Shown once on the "Both" tab, or — when
    /// there is no Both tab (single-pane standalone) — on the single server tab.
    pub fn show_logo(&self) -> bool {
        let has_both = self.tabs.iter().any(|t| matches!(t, TabKind::Both));
        match self.current_tab() {
            TabKind::Both => true,
            TabKind::Pane(_) if !has_both => true,
            _ => false,
        }
    }

    /// Find a pane index by name fragment ("auth"/"world").
    fn pane_by_name(&self, name: &str) -> Option<usize> {
        self.panes
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name))
    }

    fn default_pane(&self) -> Option<usize> {
        // Prefer world as the bare-command default, else first pane.
        self.pane_by_name("world").or(Some(0)).filter(|_| !self.panes.is_empty())
    }

    /// Resolve (pane index, command text) for the current input on the current tab.
    fn resolve_target(&self) -> Option<(usize, String)> {
        let input = self.input.trim();
        if input.is_empty() {
            return None;
        }
        match self.current_tab() {
            TabKind::Performance => None,
            TabKind::Pane(i) => Some((i, input.to_string())),
            TabKind::Both => {
                if let Some(rest) = strip_prefix_ci(input, "auth:") {
                    self.pane_by_name("auth").map(|i| (i, rest.trim().to_string()))
                } else if let Some(rest) = strip_prefix_ci(input, "world:") {
                    self.pane_by_name("world").map(|i| (i, rest.trim().to_string()))
                } else {
                    self.default_pane().map(|i| (i, input.to_string()))
                }
            }
        }
    }

    /// Candidate command list + the word currently being completed.
    /// Returns (candidates, word) where `word` is the command token being typed.
    fn completion_context(&self) -> Option<(&Vec<String>, String)> {
        if matches!(self.current_tab(), TabKind::Performance) {
            return None;
        }
        let input = &self.input;
        // Determine the segment after any routing prefix.
        let (pane_idx, segment) = match self.current_tab() {
            TabKind::Pane(i) => (Some(i), input.as_str()),
            TabKind::Both => {
                if let Some(rest) = strip_prefix_ci(input, "auth:") {
                    (self.pane_by_name("auth"), rest)
                } else if let Some(rest) = strip_prefix_ci(input, "world:") {
                    (self.pane_by_name("world"), rest)
                } else {
                    (self.default_pane(), input.as_str())
                }
            }
            TabKind::Performance => return None,
        };
        let trimmed = segment.trim_start();
        // Only suggest while typing the first word (no space yet).
        if trimmed.is_empty() || trimmed.contains(' ') {
            return None;
        }
        let idx = pane_idx?;
        Some((&self.panes[idx].commands, trimmed.to_lowercase()))
    }

    /// The autosuggest ghost text (the completion of the current command word).
    pub fn suggestion(&self) -> Option<String> {
        let (candidates, word) = self.completion_context()?;
        candidates
            .iter()
            .find(|c| c.starts_with(&word) && c.len() > word.len())
            .map(|c| c[word.len()..].to_string())
    }

    pub fn accept_suggestion(&mut self) {
        if let Some(ghost) = self.suggestion() {
            self.input.push_str(&ghost);
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
        self.history_cursor = None;
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.history_cursor = None;
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.history_cursor {
            None => self.history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_cursor = Some(next);
        self.input = self.history[next].clone();
    }

    pub fn history_down(&mut self) {
        match self.history_cursor {
            Some(i) if i + 1 < self.history.len() => {
                self.history_cursor = Some(i + 1);
                self.input = self.history[i + 1].clone();
            }
            _ => {
                self.history_cursor = None;
                self.input.clear();
            }
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_add(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    /// Submit the current input as a console command to the resolved pane.
    pub fn submit(&mut self) {
        let entry = self.input.trim().to_string();
        if entry.is_empty() {
            return;
        }
        if let Some((pane_idx, cmd_text)) = self.resolve_target() {
            let pane = &self.panes[pane_idx];
            // Echo into the log view so the user sees what was sent.
            self.store
                .push_synthetic(pane.source, Level::INFO, format!("> {}", cmd_text));
            let cmd = ConsoleCommand::parse(&cmd_text);
            if !cmd.name.is_empty() {
                if pane.cmd_tx.try_send(cmd).is_err() {
                    self.store.push_synthetic(
                        pane.source,
                        Level::ERROR,
                        "console channel full, command dropped".to_string(),
                    );
                }
            }
        }
        self.history.push(entry);
        self.history_cursor = None;
        self.input.clear();
        self.scroll = 0;
    }

    /// Sample metrics into the perf history (called ~1/s).
    pub fn sample(&mut self) {
        for (i, pane) in self.panes.iter().enumerate() {
            let s = pane.metrics.snapshot();
            let series = &mut self.perf[i];
            series.connections.push(s.connections);
            series.players.push(s.players_online);
            series.tps.push(s.tps.round() as u64);
            series.tick_ms_x100.push((s.tick_ms * 100.0).round() as u64);
        }
    }

    pub fn snapshot(&self, pane_idx: usize) -> MetricsSnapshot {
        self.panes[pane_idx].metrics.snapshot()
    }
}

fn strip_prefix_ci<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    if input.len() >= prefix.len() && input[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&input[prefix.len()..])
    } else {
        None
    }
}
