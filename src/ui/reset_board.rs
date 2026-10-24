use std::collections::BTreeMap;

use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use super::tracker_board::{fmt_age, unix_now};
use super::{
    detail_line, finish_table, header_cell, header_row_style, highlight_matches, panel,
    selected_row_style, TableHit,
};
use crate::aihot;
use crate::board::Board;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResetColumn {
    Index,
    Date,
    Age,
    Kind,
    Status,
    Scope,
    Post,
}

impl ResetColumn {
    /// The sort key a click on this column's header selects.
    fn sort_key(self) -> Option<char> {
        match self {
            ResetColumn::Date | ResetColumn::Age => Some('t'),
            _ => None,
        }
    }

    fn header(self) -> &'static str {
        match self {
            ResetColumn::Index => "#",
            ResetColumn::Date => "Announced",
            ResetColumn::Age => "Age",
            ResetColumn::Kind => "Kind",
            ResetColumn::Status => "Status",
            ResetColumn::Scope => "Scope",
            ResetColumn::Post => "Latest post",
        }
    }

    fn width(self) -> Constraint {
        match self {
            ResetColumn::Index => Constraint::Length(3),
            ResetColumn::Date => Constraint::Length(10),
            ResetColumn::Age => Constraint::Length(4),
            ResetColumn::Kind => Constraint::Length(6),
            ResetColumn::Status => Constraint::Length(9),
            ResetColumn::Scope => Constraint::Length(24),
            ResetColumn::Post => Constraint::Min(20),
        }
    }
}

pub(super) fn reset_columns(width: u16) -> Vec<ResetColumn> {
    let mut columns = vec![ResetColumn::Index, ResetColumn::Date];
    if width >= 70 {
        columns.push(ResetColumn::Age);
    }
    columns.push(ResetColumn::Kind);
    columns.push(ResetColumn::Status);
    if width >= 100 {
        columns.push(ResetColumn::Scope);
    }
    columns.push(ResetColumn::Post);
    columns
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_reset_table(
    frame: &mut Frame,
    area: Rect,
    events: &[aihot::ResetEvent],
    indices: &[usize],
    table_state: &mut std::collections::HashMap<Board, TableState>,
    board: Board,
    filter_tokens: &[String],
    compact: bool,
) -> TableHit {
    let now = unix_now();
    let columns = reset_columns(area.width);
    let header = Row::new(
        columns
            .iter()
            .map(|column| header_cell(column.header(), *column == ResetColumn::Date)),
    )
    .style(header_row_style());

    let rows = indices.iter().enumerate().filter_map(|(i, &idx)| {
        events.get(idx).map(|event| {
            Row::new(
                columns
                    .iter()
                    .map(|column| reset_cell(*column, i, event, filter_tokens, now)),
            )
        })
    });

    let widths: Vec<Constraint> = columns.iter().map(|column| column.width()).collect();
    let title = format!("Codex Resets ({})", indices.len());
    let spacing = if compact { 0 } else { 1 };
    let table = Table::new(rows, widths.clone())
        .header(header.height(1))
        .row_highlight_style(selected_row_style())
        .highlight_symbol("> ")
        .block(panel(title))
        .column_spacing(spacing);

    let st = table_state.entry(board).or_default();
    frame.render_stateful_widget(table, area, st);
    finish_table(
        frame,
        area,
        &widths,
        spacing,
        st,
        columns.iter().map(|c| c.sort_key()).collect(),
        indices.len(),
    )
}

fn reset_cell(
    column: ResetColumn,
    index: usize,
    event: &aihot::ResetEvent,
    filter_tokens: &[String],
    now: i64,
) -> Cell<'static> {
    let dim = Style::default().fg(Color::DarkGray);
    match column {
        ResetColumn::Index => Cell::from(format!("{:>2}", index + 1)).style(dim),
        ResetColumn::Date => Cell::from(
            event
                .created_at
                .as_deref()
                .and_then(aihot::fmt_utc)
                .and_then(|s| s.get(..10).map(str::to_string))
                .unwrap_or_else(|| "-".into()),
        ),
        ResetColumn::Age => {
            Cell::from(fmt_age(now, event.created_unix())).style(Style::default().fg(Color::Blue))
        }
        ResetColumn::Kind => Cell::from(event.kind_label().to_string()).style(kind_style(event)),
        ResetColumn::Status => {
            Cell::from(event.status_label().to_string()).style(status_style(event))
        }
        ResetColumn::Scope => Cell::from(event.scope().unwrap_or("-").to_string()).style(dim),
        ResetColumn::Post => Cell::from(highlight_matches(
            event.latest_text().unwrap_or("-"),
            filter_tokens,
            Style::default(),
        )),
    }
}

pub(super) fn render_reset_detail(
    frame: &mut Frame,
    area: Rect,
    event: Option<&aihot::ResetEvent>,
) {
    let dim = Style::default().fg(Color::DarkGray);
    let utc = |s: &Option<String>| {
        s.as_deref()
            .and_then(aihot::fmt_utc)
            .unwrap_or_else(|| "-".into())
    };
    let lines = match event {
        Some(event) => {
            let mut lines = vec![
                Line::styled(
                    match event.kind_label() {
                        "credit" => "Reset credit granted",
                        _ => "Usage reset",
                    },
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Line::from(""),
                detail_line("Status", event.status_label(), status_style(event)),
                detail_line("Kind", event.kind_label(), kind_style(event)),
                detail_line("Scope", event.scope().unwrap_or("-"), Style::default()),
                detail_line("Announced", utc(&event.created_at), Style::default()),
                detail_line(
                    "Expected",
                    event.schedule_label().unwrap_or_else(|| "-".into()),
                    Style::default(),
                ),
                detail_line(
                    "Confirmed",
                    utc(&event.confirmed_at),
                    Style::default().fg(Color::Green),
                ),
            ];
            if !event.posts.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::styled(
                    format!("Posts ({}, newest first)", event.posts.len()),
                    dim,
                ));
            }
            for post in &event.posts {
                let when = utc(&post.published_at);
                lines.push(Line::styled(
                    match post.stage_label() {
                        Some(stage) => format!("{when}  {stage}"),
                        None => when,
                    },
                    Style::default().fg(Color::Blue),
                ));
                if let Some(text) = &post.original_text {
                    lines.push(Line::from(text.clone()));
                }
                if let Some(url) = &post.url {
                    lines.push(Line::styled(url.clone(), dim));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::styled("via AIHOT (aihot.news/codex-reset)", dim));
            lines
        }
        None => vec![Line::styled("No row selected", dim)],
    };

    let p = Paragraph::new(lines)
        .block(panel("Selected"))
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

/// Confirmed in green; deadline passed without a confirmation in yellow.
fn status_style(event: &aihot::ResetEvent) -> Style {
    match event.status() {
        Some("confirmed") => Style::default().fg(Color::Green),
        Some("likely_completed") => Style::default().fg(Color::Yellow),
        Some(_) => Style::default().fg(Color::Magenta),
        None => Style::default().fg(Color::DarkGray),
    }
}

fn kind_style(event: &aihot::ResetEvent) -> Style {
    match event.kind_label() {
        "credit" => Style::default().fg(Color::Cyan),
        _ => Style::default(),
    }
}

// ── Calendar ─────────────────────────────────────────────────────────

/// Each day cell: the day number right-aligned in 3 columns, then up to two
/// event markers.
const CELL_WIDTH: usize = 5;
const MONTH_WIDTH: usize = 7 * CELL_WIDTH;
const MONTH_GAP: usize = 3;
/// Title, weekday header, and six week rows.
const MONTH_HEIGHT: usize = 8;
/// A blank line, the stats line, and the legend.
const FOOTER_HEIGHT: usize = 3;
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Months counted from year 0: `year * 12 + month - 1`.
fn month_of_day(day: i64) -> i64 {
    let (year, month, _) = aihot::civil_from_days(day);
    year * 12 + month - 1
}

fn first_day_of_month(month: i64) -> i64 {
    crate::aitracker::days_from_civil(month.div_euclid(12), month.rem_euclid(12) + 1, 1)
}

/// Monday is 0; 1970-01-01 was a Thursday.
fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// The months to draw, oldest first. The window ends at `newest` unless that
/// would push `selected` out; it never starts before `earliest`.
fn month_window(newest: i64, selected: i64, earliest: i64, capacity: usize) -> (i64, i64) {
    let span = capacity.max(1) as i64 - 1;
    let end = if selected < newest - span {
        selected + span
    } else {
        newest
    };
    (earliest.min(end).max(end - span), end)
}

/// One month as `MONTH_HEIGHT` lines, each `MONTH_WIDTH` columns wide.
fn month_lines(
    month: i64,
    days: &BTreeMap<i64, Vec<&aihot::ResetEvent>>,
    selected_day: Option<i64>,
    today: i64,
) -> Vec<Line<'static>> {
    let first = first_day_of_month(month);
    let next = first_day_of_month(month + 1);
    let count = days.range(first..next).map(|(_, v)| v.len()).sum::<usize>();
    let name = format!(
        "{} {}",
        MONTH_NAMES[month.rem_euclid(12) as usize],
        month.div_euclid(12)
    );
    let mut title = vec![Span::styled(
        name.clone(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];
    if count > 0 {
        let noun = if count == 1 { "event" } else { "events" };
        title.push(Span::styled(
            format!(
                "{:>w$}",
                format!("{count} {noun}"),
                w = MONTH_WIDTH - name.len()
            ),
            Style::default().fg(Color::DarkGray),
        ));
    }

    let mut lines = vec![
        Line::from(title),
        Line::styled(
            ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
                .iter()
                .map(|d| format!("{d:>3}  "))
                .collect::<String>(),
            Style::default().fg(Color::DarkGray),
        ),
    ];
    let mut day = first - weekday(first);
    for _ in 0..6 {
        let mut spans = Vec::with_capacity(14);
        for _ in 0..7 {
            if day < first || day >= next {
                spans.push(Span::raw(" ".repeat(CELL_WIDTH)));
            } else {
                spans.extend(day_cell(day, first, days.get(&day), selected_day, today));
            }
            day += 1;
        }
        lines.push(Line::from(spans));
    }
    lines
}

fn day_cell(
    day: i64,
    first: i64,
    events: Option<&Vec<&aihot::ResetEvent>>,
    selected_day: Option<i64>,
    today: i64,
) -> Vec<Span<'static>> {
    let events = events.map(Vec::as_slice).unwrap_or_default();
    let mut number = match events.first() {
        Some(event) => status_style(event).add_modifier(Modifier::BOLD),
        None => Style::default().fg(Color::Gray),
    };
    if day == today {
        number = number.add_modifier(Modifier::UNDERLINED);
    }
    let mut spans = vec![Span::styled(format!("{:>3}", day - first + 1), number)];
    for slot in 0..2 {
        spans.push(match events.get(slot) {
            Some(_) if slot == 1 && events.len() > 2 => {
                Span::styled("+", Style::default().fg(Color::Gray))
            }
            Some(event) => Span::styled(marker(event), status_style(event)),
            None => Span::raw(" "),
        });
    }
    if selected_day == Some(day) {
        for span in &mut spans {
            span.style = span.style.patch(selected_row_style());
        }
    }
    spans
}

/// `●` for a usage reset, `◆` for a reset credit.
fn marker(event: &aihot::ResetEvent) -> &'static str {
    match event.kind_label() {
        "credit" => "◆",
        _ => "●",
    }
}

/// Counts over the 90 days before `today`, the median gap between resets,
/// and the last reset, as on aihot.news/codex-reset.
fn stats_line(events: &[&aihot::ResetEvent], today: i64) -> String {
    let since = today - 90;
    let recent: Vec<_> = events
        .iter()
        .filter(|e| e.calendar_day().is_some_and(|d| d > since && d <= today))
        .collect();
    let credits = recent.iter().filter(|e| e.kind_label() == "credit").count();
    let mut reset_times: Vec<i64> = recent
        .iter()
        .filter(|e| e.kind_label() != "credit")
        .filter_map(|e| e.effective_unix())
        .collect();
    reset_times.sort_unstable();
    let mut gaps: Vec<i64> = reset_times.windows(2).map(|w| w[1] - w[0]).collect();
    gaps.sort_unstable();
    let median = match gaps.len() {
        0 => None,
        n if n % 2 == 1 => Some(gaps[n / 2] as f64),
        n => Some((gaps[n / 2 - 1] + gaps[n / 2]) as f64 / 2.0),
    };
    let last = events
        .iter()
        .filter(|e| e.kind_label() != "credit")
        .filter_map(|e| e.calendar_day())
        .filter(|&d| d <= today)
        .max();

    let mut out = format!(
        "Last 90 days: {} resets, {credits} credits",
        reset_times.len()
    );
    if let Some(m) = median {
        out.push_str(&format!("  |  median gap {:.1}d", m / 86_400.0));
    }
    if let Some(day) = last {
        let (y, m, d) = aihot::civil_from_days(day);
        out.push_str(&format!("  |  last reset {y:04}-{m:02}-{d:02}"));
    }
    out
}

fn legend_line() -> Line<'static> {
    let dim = Style::default().fg(Color::DarkGray);
    Line::from(vec![
        Span::styled("● ", Style::default()),
        Span::styled("reset  ", dim),
        Span::styled("◆ ", Style::default()),
        Span::styled("credit  ", dim),
        Span::styled("confirmed", Style::default().fg(Color::Green)),
        Span::styled("  ", dim),
        Span::styled("likely landed", Style::default().fg(Color::Yellow)),
        Span::styled("  ", dim),
        Span::styled("announced", Style::default().fg(Color::Magenta)),
        Span::styled("  |  days in UTC, j/k moves between events", dim),
    ])
}

/// Months of resets as a calendar. The selected row picks the highlighted
/// day, and the window of months follows it.
pub(super) fn render_reset_calendar(
    frame: &mut Frame,
    area: Rect,
    events: &[aihot::ResetEvent],
    indices: &[usize],
    selected: usize,
) {
    let visible: Vec<&aihot::ResetEvent> = indices.iter().filter_map(|&i| events.get(i)).collect();
    let mut days: BTreeMap<i64, Vec<&aihot::ResetEvent>> = BTreeMap::new();
    for event in &visible {
        if let Some(day) = event.calendar_day() {
            days.entry(day).or_default().push(event);
        }
    }
    let today = unix_now().div_euclid(86_400);
    let selected_day = visible.get(selected).and_then(|e| e.calendar_day());

    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2) as usize;
    let cols = ((inner_width + MONTH_GAP) / (MONTH_WIDTH + MONTH_GAP)).max(1);
    let rows = ((inner_height.saturating_sub(FOOTER_HEIGHT) + 1) / (MONTH_HEIGHT + 1)).max(1);
    let newest = days.last_key_value().map_or(today, |(&d, _)| d.max(today));
    let earliest = days.first_key_value().map_or(today, |(&d, _)| d);
    let (start, end) = month_window(
        month_of_day(newest),
        month_of_day(selected_day.unwrap_or(newest)),
        month_of_day(earliest),
        cols * rows,
    );

    let months: Vec<Vec<Line>> = (start..=end)
        .map(|m| month_lines(m, &days, selected_day, today))
        .collect();
    let mut lines: Vec<Line> = Vec::new();
    for (row, chunk) in months.chunks(cols).enumerate() {
        if row > 0 {
            lines.push(Line::from(""));
        }
        for i in 0..MONTH_HEIGHT {
            let mut spans = Vec::new();
            for (col, month) in chunk.iter().enumerate() {
                if col > 0 {
                    spans.push(Span::raw(" ".repeat(MONTH_GAP)));
                }
                let line = &month[i];
                spans.extend(
                    line.spans
                        .iter()
                        .cloned()
                        .map(|s| s.patch_style(line.style)),
                );
                let pad = MONTH_WIDTH.saturating_sub(line.width());
                spans.push(Span::raw(" ".repeat(pad)));
            }
            lines.push(Line::from(spans));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(stats_line(&visible, today), Style::default()));
    lines.push(legend_line());

    let title = format!("Codex Resets calendar ({} events)", visible.len());
    frame.render_widget(Paragraph::new(lines).block(panel(title)), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_grow_with_width() {
        assert!(!reset_columns(60).contains(&ResetColumn::Age));
        assert!(reset_columns(70).contains(&ResetColumn::Age));
        assert!(reset_columns(100).contains(&ResetColumn::Scope));
        assert_eq!(reset_columns(40).last(), Some(&ResetColumn::Post));
    }

    fn day(s: &str) -> i64 {
        aihot::parse_timestamp(&format!("{s}T00:00:00Z")).unwrap() / 86_400
    }

    #[test]
    fn weekdays_start_on_monday() {
        assert_eq!(weekday(day("2026-10-19")), 0);
        assert_eq!(weekday(day("2026-10-24")), 5);
        assert_eq!(
            first_day_of_month(month_of_day(day("2026-10-24"))),
            day("2026-10-01")
        );
        assert_eq!(
            first_day_of_month(month_of_day(day("2026-12-31")) + 1),
            day("2027-01-01")
        );
    }

    #[test]
    fn month_window_follows_the_selection() {
        // Newest month 100, history from 90, room for 3 months.
        assert_eq!(month_window(100, 100, 90, 3), (98, 100));
        assert_eq!(month_window(100, 99, 90, 3), (98, 100));
        // An older selection slides the window just far enough to show it.
        assert_eq!(month_window(100, 95, 90, 3), (95, 97));
        // Less history than room: start at the first month with data.
        assert_eq!(month_window(100, 100, 99, 6), (99, 100));
        assert_eq!(month_window(100, 100, 100, 1), (100, 100));
    }

    #[test]
    fn stats_count_the_last_90_days() {
        let fixture = include_str!("../../tests/fixtures/aihot_codex_resets.json");
        let events = aihot::parse_resets(fixture).unwrap();
        let refs: Vec<_> = events.iter().collect();
        // Resets on Oct 20 06:33 and Oct 23 04:47 UTC; credits on Sep 30 and Oct 8.
        let line = stats_line(&refs, day("2026-10-24"));
        assert_eq!(
            line,
            "Last 90 days: 2 resets, 2 credits  |  median gap 2.9d  |  last reset 2026-10-23"
        );
    }
}
