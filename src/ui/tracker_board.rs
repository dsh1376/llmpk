use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use super::{
    accent_color_for_provider, color_for_seed, detail_line, finish_table, header_cell,
    header_row_style, highlight_matches, panel, selected_row_style, truncate, TableHit,
};
use crate::aitracker;
use crate::board::Board;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TrackerColumn {
    Index,
    Model,
    Maker,
    Stage,
    Seen,
    Date,
    Posts,
}

impl TrackerColumn {
    /// The sort key a click on this column's header selects.
    fn sort_key(self) -> Option<char> {
        match self {
            TrackerColumn::Seen => Some('t'),
            TrackerColumn::Date => Some('t'),
            _ => None,
        }
    }

    fn header(self) -> &'static str {
        match self {
            TrackerColumn::Index => "#",
            TrackerColumn::Model => "Model",
            TrackerColumn::Maker => "Maker",
            TrackerColumn::Stage => "Stage",
            TrackerColumn::Seen => "Seen",
            TrackerColumn::Date => "Date",
            TrackerColumn::Posts => "Posts",
        }
    }

    fn width(self) -> Constraint {
        match self {
            TrackerColumn::Index => Constraint::Length(3),
            TrackerColumn::Model => Constraint::Min(20),
            TrackerColumn::Maker => Constraint::Length(18),
            TrackerColumn::Stage => Constraint::Length(7),
            TrackerColumn::Seen => Constraint::Length(4),
            TrackerColumn::Date => Constraint::Length(10),
            TrackerColumn::Posts => Constraint::Length(5),
        }
    }
}

pub(super) fn tracker_columns(width: u16) -> Vec<TrackerColumn> {
    let mut columns = vec![TrackerColumn::Index, TrackerColumn::Model];
    if width >= 60 {
        columns.push(TrackerColumn::Maker);
    }
    columns.push(TrackerColumn::Stage);
    columns.push(TrackerColumn::Seen);
    if width >= 76 {
        columns.push(TrackerColumn::Date);
    }
    if width >= 90 {
        columns.push(TrackerColumn::Posts);
    }
    columns
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_tracker_table(
    frame: &mut Frame,
    area: Rect,
    pages: &[aitracker::ModelPage],
    indices: &[usize],
    table_state: &mut std::collections::HashMap<Board, TableState>,
    board: Board,
    filter_tokens: &[String],
    compact: bool,
) -> TableHit {
    let now = unix_now();
    let columns = tracker_columns(area.width);
    let sorted = TrackerColumn::Seen;
    let header = Row::new(
        columns
            .iter()
            .map(|column| header_cell(column.header(), *column == sorted)),
    )
    .style(header_row_style());

    let rows = indices.iter().enumerate().filter_map(|(i, &idx)| {
        pages.get(idx).map(|page| {
            Row::new(
                columns
                    .iter()
                    .map(|column| tracker_cell(*column, i, page, filter_tokens, now)),
            )
        })
    });

    let widths: Vec<Constraint> = columns.iter().map(|column| column.width()).collect();
    let title = format!("AI Tracker ({})", indices.len());
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

fn tracker_cell(
    column: TrackerColumn,
    index: usize,
    page: &aitracker::ModelPage,
    filter_tokens: &[String],
    now: i64,
) -> Cell<'static> {
    match column {
        TrackerColumn::Index => {
            Cell::from(format!("{:>2}", index + 1)).style(Style::default().fg(Color::DarkGray))
        }
        TrackerColumn::Model => Cell::from(highlight_matches(
            &page.model,
            filter_tokens,
            Style::default(),
        )),
        TrackerColumn::Maker => Cell::from(page.maker().unwrap_or("-").to_string())
            .style(Style::default().fg(maker_color(page))),
        TrackerColumn::Stage => {
            Cell::from(page.stage().unwrap_or("-").to_string()).style(stage_style(page.stage()))
        }
        TrackerColumn::Seen => {
            Cell::from(fmt_age(now, page.last_seen_unix())).style(Style::default().fg(Color::Blue))
        }
        TrackerColumn::Date => Cell::from(
            page.last_seen_at
                .as_deref()
                .and_then(|s| s.get(..10))
                .unwrap_or("-")
                .to_string(),
        )
        .style(Style::default().fg(Color::DarkGray)),
        TrackerColumn::Posts => {
            Cell::from(page.post_ids.len().to_string()).style(Style::default().fg(Color::DarkGray))
        }
    }
}

pub(super) fn render_tracker_detail(
    frame: &mut Frame,
    area: Rect,
    page: Option<&aitracker::ModelPage>,
) {
    let max = area.width.saturating_sub(4) as usize;
    let lines = match page {
        Some(page) => {
            let seen = match (page.last_seen_at.as_deref(), page.last_seen_unix()) {
                (Some(raw), Some(_)) => fmt_timestamp(raw),
                _ => "-".into(),
            };
            vec![
                Line::styled(
                    truncate(&page.model, max.max(8)),
                    Style::default().fg(Color::Cyan).bold(),
                ),
                detail_line("Slug", &page.slug, Style::default().fg(Color::DarkGray)),
                Line::from(""),
                detail_line(
                    "Maker",
                    page.maker().unwrap_or("-"),
                    Style::default().fg(maker_color(page)),
                ),
                detail_line(
                    "Stage",
                    page.stage().unwrap_or("-"),
                    stage_style(page.stage()),
                ),
                detail_line("Last seen", seen, Style::default()),
                detail_line(
                    "Age",
                    match page.last_seen_unix() {
                        Some(t) => format!("{} ago", fmt_age(unix_now(), Some(t))),
                        None => "-".into(),
                    },
                    Style::default().fg(Color::Blue),
                ),
                detail_line("Posts", page.post_ids.len().to_string(), Style::default()),
                detail_line("Events", page.event_ids.len().to_string(), Style::default()),
                Line::from(""),
                Line::styled(
                    page.url().unwrap_or_else(|| "-".into()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]
        }
        None => vec![Line::styled(
            "No row selected",
            Style::default().fg(Color::DarkGray),
        )],
    };

    let p = Paragraph::new(lines)
        .block(panel("Selected"))
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

/// Releases in green and leaks in yellow, as on the live site.
fn stage_style(stage: Option<&str>) -> Style {
    match stage {
        Some("release") => Style::default().fg(Color::Green),
        Some("leak") => Style::default().fg(Color::Yellow),
        _ => Style::default().fg(Color::DarkGray),
    }
}

fn maker_color(page: &aitracker::ModelPage) -> Color {
    let Some(maker) = page.maker() else {
        return Color::DarkGray;
    };
    // Makers read like "Alibaba Qwen" or "Xiaomi MiMo"; the first word is the
    // brand the shared palette knows.
    accent_color_for_provider(maker)
        .or_else(|| {
            maker
                .split_whitespace()
                .next()
                .and_then(accent_color_for_provider)
        })
        .unwrap_or_else(|| color_for_seed(maker))
}

pub(super) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Compact age such as `45s`, `12m`, `5h`, `3d`, or `8w`. Future timestamps
/// (clock skew) read as `now`.
pub(super) fn fmt_age(now: i64, then: Option<i64>) -> String {
    let Some(then) = then else {
        return "-".into();
    };
    let secs = now - then;
    match secs {
        s if s < 0 => "now".into(),
        s if s < 60 => format!("{s}s"),
        s if s < 3_600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3_600),
        s if s < 14 * 86_400 => format!("{}d", s / 86_400),
        s => format!("{}w", s / (7 * 86_400)),
    }
}

/// `2026-10-22T15:43:41.669Z` -> `2026-10-22 15:43 UTC`.
fn fmt_timestamp(raw: &str) -> String {
    match (raw.get(..10), raw.get(11..16)) {
        (Some(date), Some(time)) => format!("{date} {time} UTC"),
        _ => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_uses_the_largest_whole_unit() {
        assert_eq!(fmt_age(100, Some(100)), "0s");
        assert_eq!(fmt_age(159, Some(100)), "59s");
        assert_eq!(fmt_age(3_599, Some(0)), "59m");
        assert_eq!(fmt_age(3_600, Some(0)), "1h");
        assert_eq!(fmt_age(90_000, Some(0)), "1d");
        assert_eq!(fmt_age(3 * 86_400, Some(0)), "3d");
        assert_eq!(fmt_age(60 * 86_400, Some(0)), "8w");
    }

    #[test]
    fn age_handles_missing_and_future_timestamps() {
        assert_eq!(fmt_age(100, None), "-");
        assert_eq!(fmt_age(100, Some(200)), "now");
    }

    #[test]
    fn timestamp_drops_seconds_and_names_the_zone() {
        assert_eq!(
            fmt_timestamp("2026-10-22T15:43:41.669Z"),
            "2026-10-22 15:43 UTC"
        );
    }

    #[test]
    fn columns_grow_with_width() {
        assert!(!tracker_columns(50).contains(&TrackerColumn::Maker));
        assert!(tracker_columns(60).contains(&TrackerColumn::Maker));
        assert!(tracker_columns(90).contains(&TrackerColumn::Posts));
    }
}
