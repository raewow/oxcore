//! Rendering for the TUI (tabs, log panes, status box, performance tab).

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Tabs, Wrap};
use ratatui::Frame;
use tracing::Level;

use crate::app::{App, TabKind};
use crate::log_layer::{LogFilter, LogRecord, LogStore};
use crate::progress::ProgressSnapshot;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub const LOGO: [&str; 3] = [
    "▄████▄ ▄▄ ▄▄ ▄█████  ▄▄▄  ▄▄▄▄  ▄▄▄▄▄",
    "██  ██ ▀█▄█▀ ██     ██▀██ ██▄█▄ ██▄▄ ",
    "▀████▀ ██ ██ ▀█████ ▀███▀ ██ ██ ██▄▄▄",
];

const ACCENT: Color = Color::Rgb(0xff, 0x8c, 0x32);

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_tabs(f, chunks[0], app);

    match app.current_tab() {
        TabKind::Performance => render_perf(f, chunks[1], app),
        _ => render_body(f, chunks[1], app),
    }

    render_input(f, chunks[2], app);
}

fn render_tabs(f: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = app
        .tab_titles()
        .into_iter()
        .map(|t| Line::from(format!(" {} ", t)))
        .collect();
    let tabs = Tabs::new(titles)
        .select(app.active)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .divider("");
    f.render_widget(tabs, area);
}

fn render_body(f: &mut Frame, area: Rect, app: &mut App) {
    // Split into logs (left) and status (right).
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(34)])
        .split(area);

    let mut log_area = cols[0];

    if app.show_logo() {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(cols[0]);
        render_logo(f, rows[0]);
        log_area = rows[1];
    }

    render_logs(f, log_area, app);
    render_status(f, cols[1], app);
}

/// The startup loading screen: logo, progress bar (or spinner), and a live log tail.
pub fn render_loading(
    f: &mut Frame,
    store: &LogStore,
    progress: &ProgressSnapshot,
    status: &str,
    error: Option<&str>,
    spinner: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(f.area());

    render_logo(f, chunks[0]);
    render_progress(f, chunks[1], progress, status, error, spinner);

    // Live tail of init logs (all sources).
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" startup log ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(chunks[2]);
    f.render_widget(block, chunks[2]);

    let records = store.filtered(LogFilter::All);
    let height = inner.height as usize;
    let start = records.len().saturating_sub(height);
    let lines: Vec<Line> = records[start..]
        .iter()
        .map(|r| record_line(r, true))
        .collect();
    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_progress(
    f: &mut Frame,
    area: Rect,
    progress: &ProgressSnapshot,
    status: &str,
    error: Option<&str>,
    spinner: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(err) = error {
        let lines = vec![
            Line::from(Span::styled(
                format!("startup failed: {}", err),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "press any key to exit",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        f.render_widget(
            Paragraph::new(Text::from(lines)).alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let frame = SPINNER[spinner % SPINNER.len()];

    if progress.total > 0 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        let ratio = (progress.current as f64 / progress.total as f64).clamp(0.0, 1.0);
        let pct = (ratio * 100.0).round() as u16;
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(ACCENT))
            .ratio(ratio)
            .label(format!("{}%", pct));
        f.render_widget(gauge, rows[0]);

        let label = Line::from(vec![
            Span::styled(format!("{} ", frame), Style::default().fg(ACCENT)),
            Span::styled(
                format!("Step {}/{}: ", progress.current, progress.total),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(progress.label.clone(), Style::default().fg(Color::White)),
        ]);
        f.render_widget(Paragraph::new(label).alignment(Alignment::Center), rows[1]);
    } else {
        // Indeterminate: spinner + status/label.
        let text = if progress.label.is_empty() {
            status.to_string()
        } else {
            progress.label.clone()
        };
        let line = Line::from(vec![
            Span::styled(format!("{} ", frame), Style::default().fg(ACCENT)),
            Span::styled(text, Style::default().fg(Color::White)),
        ]);
        f.render_widget(Paragraph::new(line).alignment(Alignment::Center), inner);
    }
}

fn render_logo(f: &mut Frame, area: Rect) {
    let lines: Vec<Line> = LOGO
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(ACCENT))))
        .collect();
    let para = Paragraph::new(Text::from(lines)).alignment(Alignment::Center);
    f.render_widget(para, area);
}

fn level_style(level: Level) -> Style {
    let color = match level {
        Level::ERROR => Color::Red,
        Level::WARN => Color::Yellow,
        Level::INFO => Color::Green,
        Level::DEBUG => Color::Cyan,
        Level::TRACE => Color::DarkGray,
    };
    Style::default().fg(color)
}

fn record_line(rec: &LogRecord, show_source: bool) -> Line<'static> {
    let mut spans = vec![
        Span::styled(format!("{} ", rec.time), Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>5} ", rec.level.as_str()),
            level_style(rec.level).add_modifier(Modifier::BOLD),
        ),
    ];
    if show_source {
        spans.push(Span::styled(
            format!("[{}] ", rec.source.tag()),
            Style::default().fg(Color::Magenta),
        ));
    }
    spans.push(Span::raw(rec.message.clone()));
    Line::from(spans)
}

fn render_logs(f: &mut Frame, area: Rect, app: &App) {
    let filter = app.log_filter();
    let show_source = matches!(app.current_tab(), TabKind::Both);
    let records = app.store.filtered(filter);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" logs ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let height = inner.height as usize;
    let total = records.len();
    // `scroll` lines up from the bottom.
    let end = total.saturating_sub(app.scroll);
    let start = end.saturating_sub(height);
    let visible = &records[start..end];

    let lines: Vec<Line> = visible.iter().map(|r| record_line(r, show_source)).collect();
    let para = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let idxs: Vec<usize> = match app.current_tab() {
        TabKind::Both => (0..app.panes.len()).collect(),
        TabKind::Pane(i) => vec![i],
        TabKind::Performance => vec![],
    };

    if idxs.is_empty() {
        return;
    }

    let constraints: Vec<Constraint> = idxs
        .iter()
        .map(|_| Constraint::Ratio(1, idxs.len() as u32))
        .collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (slot, &pane_idx) in idxs.iter().enumerate() {
        let snap = app.snapshot(pane_idx);
        let pane = &app.panes[pane_idx];

        let mut lines = vec![Line::from(vec![
            Span::styled("conns ", Style::default().fg(Color::Gray)),
            Span::styled(
                snap.connections.to_string(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
        ])];
        for (label, value) in &snap.gauges {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<14}", label), Style::default().fg(Color::Gray)),
                Span::styled(value.clone(), Style::default().fg(Color::White)),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", pane.name))
            .border_style(Style::default().fg(ACCENT));
        let para = Paragraph::new(Text::from(lines)).block(block);
        f.render_widget(para, rows[slot]);
    }
}

fn render_perf(f: &mut Frame, area: Rect, app: &mut App) {
    let n = app.panes.len();
    if n == 0 {
        return;
    }
    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for i in 0..n {
        render_perf_pane(f, rows[i], app, i);
    }
}

fn render_perf_pane(f: &mut Frame, area: Rect, app: &App, idx: usize) {
    let pane = &app.panes[idx];
    let snap = pane.metrics.snapshot();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} — performance ", pane.name))
        .border_style(Style::default().fg(ACCENT));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Left column: textual gauges. Right column: sparklines.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(inner);

    let mut lines = vec![Line::from(vec![
        Span::styled("connections  ", Style::default().fg(Color::Gray)),
        Span::styled(
            snap.connections.to_string(),
            Style::default().fg(Color::White),
        ),
    ])];
    for (label, value) in &snap.gauges {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<13}", label), Style::default().fg(Color::Gray)),
            Span::styled(value.clone(), Style::default().fg(Color::White)),
        ]));
    }
    f.render_widget(Paragraph::new(Text::from(lines)), cols[0]);

    // Choose two series to chart depending on server type.
    let series = &app.perf[idx];
    let is_world = snap.tps > 0.0 || snap.players_online > 0 || snap.tick_ms > 0.0;
    let (label_a, data_a, color_a, label_b, data_b, color_b) = if is_world {
        (
            format!("TPS  {:.1}", snap.tps),
            series.tps.values(),
            Color::Green,
            format!("tick {:.2} ms", snap.tick_ms),
            series.tick_ms_x100.values(),
            Color::Cyan,
        )
    } else {
        (
            format!("active conns  {}", snap.connections),
            series.connections.values(),
            Color::Green,
            "players".to_string(),
            series.players.values(),
            Color::Cyan,
        )
    };

    let spark_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(cols[1]);

    render_spark(f, spark_rows[0], &label_a, &data_a, color_a);
    render_spark(f, spark_rows[1], &label_b, &data_b, color_b);
}

fn render_spark(f: &mut Frame, area: Rect, title: &str, data: &[u64], color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(Color::DarkGray));
    let spark = Sparkline::default()
        .block(block)
        .data(data)
        .style(Style::default().fg(color));
    f.render_widget(spark, area);
}

fn render_input(f: &mut Frame, area: Rect, app: &App) {
    let disabled = matches!(app.current_tab(), TabKind::Performance);
    let title = if disabled {
        " input (switch tab to send commands) "
    } else {
        " input  (Tab: switch · → : complete · ↑↓ history · PgUp/PgDn scroll · q: quit) "
    };

    let mut spans = vec![
        Span::styled("> ", Style::default().fg(ACCENT)),
        Span::raw(app.input.clone()),
    ];
    if !disabled {
        if let Some(ghost) = app.suggestion() {
            spans.push(Span::styled(ghost, Style::default().fg(Color::DarkGray)));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(if disabled { Color::DarkGray } else { ACCENT }));
    let inner = block.inner(area);
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);

    if !disabled {
        // Cursor sits right after the typed input (before ghost text).
        let x = inner.x + 2 + app.input.chars().count() as u16;
        let cx = x.min(inner.x + inner.width.saturating_sub(1));
        f.set_cursor_position((cx, inner.y));
    }
}
