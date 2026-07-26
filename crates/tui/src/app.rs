//! TUI application state and input/routing logic (rendering lives in `ui`).

use std::collections::VecDeque;
use std::sync::Arc;

use oxcore_shared::console::ConsoleCommand;
use tokio::sync::mpsc;
use tracing::Level;

use crate::log_layer::{LogFilter, LogSource, LogStore};
use crate::logging::{level_str, parse_level, LogControl};
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

/// Which text input currently receives keystrokes.
#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Command,
    Filter,
}

pub struct App {
    pub panes: Vec<ServerPane>,
    pub store: Arc<LogStore>,
    pub tabs: Vec<TabKind>,
    pub active: usize,
    pub input: String,
    /// Cursor position within `input`, as a char index (0..=len).
    pub input_cursor: usize,
    pub history: Vec<String>,
    pub history_cursor: Option<usize>,
    /// Lines scrolled up from the bottom of the log view.
    pub scroll: usize,
    /// Whether new log records should keep the viewport pinned to the newest lines.
    pub follow_logs: bool,
    /// Wrapped line count, width, and store revision from the most recent log render.
    last_log_layout: Option<(usize, u16, u64)>,
    pub should_quit: bool,
    pub perf: Vec<PaneSeries>,
    /// Active log filter substring (empty = no filtering).
    pub filter: String,
    /// Which input has keyboard focus.
    pub focus: Focus,
    /// Whether the quit-confirmation popup is showing.
    pub confirm_quit: bool,
    /// Whether terminal mouse capture is enabled for wheel scrolling/tab clicks.
    pub mouse_enabled: bool,
    pub log_control: LogControl,
}

impl App {
    pub fn new(panes: Vec<ServerPane>, store: Arc<LogStore>, log_control: LogControl) -> Self {
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
            input_cursor: 0,
            history: Vec::new(),
            history_cursor: None,
            scroll: 0,
            follow_logs: true,
            last_log_layout: None,
            should_quit: false,
            perf,
            filter: String::new(),
            focus: Focus::Command,
            confirm_quit: false,
            mouse_enabled: false,
            log_control,
        }
    }

    pub fn set_mouse_enabled(&mut self, enabled: bool) {
        self.mouse_enabled = enabled;
    }

    pub fn log_level_name(&self) -> &'static str {
        self.log_control.level_name()
    }

    pub fn cycle_log_level(&mut self) {
        let level = self.log_control.cycle_debug_levels();
        self.store.push_synthetic(
            LogSource::Other,
            Level::INFO,
            format!("TUI log level set to {}", level_str(level)),
        );
    }

    pub fn current_tab(&self) -> TabKind {
        self.tabs[self.active]
    }

    pub fn tab_titles(&self) -> Vec<String> {
        self.tabs
            .iter()
            .map(|t| match t {
                TabKind::Both => "All".to_string(),
                TabKind::Pane(i) => self.panes[*i].name.clone(),
                TabKind::Performance => "Performance".to_string(),
            })
            .collect()
    }

    pub fn next_tab(&mut self) {
        self.active = (self.active + 1) % self.tabs.len();
        self.follow_latest();
    }

    pub fn prev_tab(&mut self) {
        self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
        self.follow_latest();
    }

    pub fn select_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active = idx;
            self.follow_latest();
        }
    }

    /// The log filter for the current tab.
    pub fn log_filter(&self) -> LogFilter {
        match self.current_tab() {
            TabKind::Pane(i) => LogFilter::Source(self.panes[i].source),
            _ => LogFilter::All,
        }
    }

    /// Whether the OXCORE logo should be shown. Shown once on the "All" tab, or — when
    /// there is no All tab (single-pane standalone) — on the single server tab.
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
        self.pane_by_name("world")
            .or(Some(0))
            .filter(|_| !self.panes.is_empty())
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
                    self.pane_by_name("auth")
                        .map(|i| (i, rest.trim().to_string()))
                } else if let Some(rest) = strip_prefix_ci(input, "world:") {
                    self.pane_by_name("world")
                        .map(|i| (i, rest.trim().to_string()))
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
        if self.focus != Focus::Command {
            return None;
        }
        // Only suggest when the cursor is at the end of the line.
        if self.input_cursor != char_len(&self.input) {
            return None;
        }
        let (candidates, word) = self.completion_context()?;
        candidates
            .iter()
            .find(|c| c.starts_with(&word) && c.len() > word.len())
            .map(|c| c[word.len()..].to_string())
    }

    pub fn accept_suggestion(&mut self) {
        if let Some(ghost) = self.suggestion() {
            self.input.push_str(&ghost);
            self.input_cursor = char_len(&self.input);
        }
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Focus::Filter => {
                self.filter.push(c);
                self.follow_latest();
            }
            Focus::Command => {
                let byte = byte_index(&self.input, self.input_cursor);
                self.input.insert(byte, c);
                self.input_cursor += 1;
                self.history_cursor = None;
            }
        }
    }

    pub fn backspace(&mut self) {
        match self.focus {
            Focus::Filter => {
                self.filter.pop();
                self.follow_latest();
            }
            Focus::Command => {
                if self.input_cursor > 0 {
                    let byte = byte_index(&self.input, self.input_cursor - 1);
                    self.input.remove(byte);
                    self.input_cursor -= 1;
                }
                self.history_cursor = None;
            }
        }
    }

    /// Delete the character at the cursor (Del key), command input only.
    pub fn delete_char(&mut self) {
        if self.focus != Focus::Command {
            return;
        }
        if self.input_cursor < char_len(&self.input) {
            let byte = byte_index(&self.input, self.input_cursor);
            self.input.remove(byte);
        }
    }

    /// Move the cursor left (command input only).
    pub fn cursor_left(&mut self) {
        if self.focus == Focus::Command {
            self.input_cursor = self.input_cursor.saturating_sub(1);
        }
    }

    /// Move the cursor right; if already at the end, accept any autosuggestion.
    pub fn cursor_right(&mut self) {
        if self.focus != Focus::Command {
            return;
        }
        if self.input_cursor < char_len(&self.input) {
            self.input_cursor += 1;
        } else {
            self.accept_suggestion();
        }
    }

    pub fn cursor_home(&mut self) {
        if self.focus == Focus::Command {
            self.input_cursor = 0;
        }
    }

    pub fn cursor_end(&mut self) {
        if self.focus == Focus::Command {
            self.input_cursor = char_len(&self.input);
        }
    }

    /// Toggle keyboard focus between the command box and the filter box.
    pub fn toggle_filter_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Command => Focus::Filter,
            Focus::Filter => Focus::Command,
        };
    }

    /// Whether the filter box should be drawn (focused or holding a query).
    pub fn filter_active(&self) -> bool {
        self.focus == Focus::Filter || !self.filter.is_empty()
    }

    /// Clear the filter and return focus to the command box.
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.focus = Focus::Command;
        self.follow_latest();
    }

    /// Case-insensitive substring match over level + source tag + message.
    pub fn matches_filter(&self, rec: &crate::log_layer::LogRecord) -> bool {
        if self.filter.is_empty() {
            return true;
        }
        let needle = self.filter.to_lowercase();
        let hay = format!(
            "{} {} {}",
            rec.level.as_str(),
            rec.source.tag(),
            rec.message
        )
        .to_lowercase();
        hay.contains(&needle)
    }

    pub fn clamp_scroll_to(&mut self, total_lines: usize, viewport_height: usize) {
        let max_scroll = total_lines.saturating_sub(viewport_height);
        self.scroll = self.scroll.min(max_scroll);
    }

    /// Preserve the visible rows while new matching logs arrive during manual scrolling.
    pub fn cached_log_lines(&self, width: u16, revision: u64) -> Option<usize> {
        self.last_log_layout
            .filter(|(_, previous_width, previous_revision)| {
                *previous_width == width && *previous_revision == revision
            })
            .map(|(total, _, _)| total)
    }

    pub fn update_log_layout(&mut self, total_lines: usize, width: u16, revision: u64) {
        if !self.follow_logs {
            if let Some((previous_total, previous_width, _)) = self.last_log_layout {
                if previous_width == width && total_lines > previous_total {
                    self.scroll = self.scroll.saturating_add(total_lines - previous_total);
                }
            }
        }
        self.last_log_layout = Some((total_lines, width, revision));
    }

    /// Begin the quit-confirmation flow (shows the popup; does not quit yet).
    pub fn request_quit(&mut self) {
        self.confirm_quit = true;
    }

    /// Confirm the quit from the popup.
    pub fn confirm_quit_yes(&mut self) {
        self.should_quit = true;
    }

    /// Dismiss the quit popup.
    pub fn cancel_quit(&mut self) {
        self.confirm_quit = false;
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
        self.input_cursor = char_len(&self.input);
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
        self.input_cursor = char_len(&self.input);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_add(amount);
        self.follow_logs = false;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
        if self.scroll == 0 {
            self.follow_logs = true;
        }
    }

    pub fn follow_latest(&mut self) {
        self.scroll = 0;
        self.follow_logs = true;
        self.last_log_layout = None;
    }

    /// Submit the current input as a console command to the resolved pane.
    pub fn submit(&mut self) {
        let entry = self.input.trim().to_string();
        if entry.is_empty() {
            return;
        }
        if entry == ":log" || entry.starts_with(":log ") {
            self.submit_log_command(entry.strip_prefix(":log").unwrap_or_default().trim());
            self.finish_submit(entry);
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
        self.finish_submit(entry);
    }

    fn submit_log_command(&mut self, arg: &str) {
        if arg.is_empty() {
            self.store.push_synthetic(
                LogSource::Other,
                Level::INFO,
                format!("TUI log level is {}", self.log_control.level_name()),
            );
            return;
        }

        match parse_level(arg) {
            Some(level) => {
                self.log_control.set_level(level);
                self.store.push_synthetic(
                    LogSource::Other,
                    Level::INFO,
                    format!("TUI log level set to {}", level_str(level)),
                );
            }
            None => self.store.push_synthetic(
                LogSource::Other,
                Level::ERROR,
                "usage: :log error|warn|info|debug|trace".to_string(),
            ),
        }
    }

    fn finish_submit(&mut self, entry: String) {
        self.history.push(entry);
        self.history_cursor = None;
        self.input.clear();
        self.input_cursor = 0;
        self.follow_latest();
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

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Byte offset of the `n`th char (clamped to the string end).
fn byte_index(s: &str, n: usize) -> usize {
    s.char_indices()
        .nth(n)
        .map(|(b, _)| b)
        .unwrap_or_else(|| s.len())
}

fn strip_prefix_ci<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    if input.len() >= prefix.len() && input[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&input[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(Vec::new(), LogStore::new(10), LogControl::new(2))
    }

    #[test]
    fn paused_scroll_preserves_visible_rows_when_logs_arrive() {
        let mut app = app();
        app.update_log_layout(20, 80, 1);
        app.scroll_up(5);

        app.update_log_layout(23, 80, 2);

        assert_eq!(app.scroll, 8);
        assert!(!app.follow_logs);
    }

    #[test]
    fn returning_to_latest_resets_scroll_tracking() {
        let mut app = app();
        app.update_log_layout(20, 80, 1);
        app.scroll_up(5);
        app.follow_latest();

        assert_eq!(app.scroll, 0);
        assert!(app.follow_logs);
        assert_eq!(app.last_log_layout, None);
    }
}
