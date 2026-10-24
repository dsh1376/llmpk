use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style, Stylize},
    text::Line,
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
pub(super) enum HotColumn {
    Rank,
    Title,
    Sources,
    People,
    Seen,
    Origin,
}

impl HotColumn {
    /// The sort key a click on this column's header selects.
    fn sort_key(self) -> Option<char> {
        match self {
            HotColumn::Rank => Some('i'),
            _ => None,
        }
    }

    fn header(self) -> &'static str {
        match self {
            HotColumn::Rank => "#",
            HotColumn::Title => "Event",
            HotColumn::Sources => "Src",
            HotColumn::People => "Ppl",
            HotColumn::Seen => "Seen",
            HotColumn::Origin => "First source",
        }
    }

    fn width(self) -> Constraint {
        match self {
            HotColumn::Rank => Constraint::Length(3),
            HotColumn::Title => Constraint::Min(30),
            HotColumn::Sources => Constraint::Length(3),
            HotColumn::People => Constraint::Length(4),
            HotColumn::Seen => Constraint::Length(4),
            HotColumn::Origin => Constraint::Length(24),
        }
    }
}

pub(super) fn hot_columns(width: u16) -> Vec<HotColumn> {
    let mut columns = vec![HotColumn::Rank, HotColumn::Title, HotColumn::Sources];
    if width >= 60 {
        columns.push(HotColumn::People);
    }
    columns.push(HotColumn::Seen);
    if width >= 100 {
        columns.push(HotColumn::Origin);
    }
    columns
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_hot_table(
    frame: &mut Frame,
    area: Rect,
    topics: &[aihot::HotTopic],
    indices: &[usize],
    table_state: &mut std::collections::HashMap<Board, TableState>,
    board: Board,
    filter_tokens: &[String],
    compact: bool,
) -> TableHit {
    let now = unix_now();
    let columns = hot_columns(area.width);
    let header = Row::new(
        columns
            .iter()
            .map(|column| header_cell(column.header(), *column == HotColumn::Rank)),
    )
    .style(header_row_style());

    let rows = indices.iter().filter_map(|&idx| {
        topics.get(idx).map(|topic| {
            Row::new(
                columns
                    .iter()
                    .map(|column| hot_cell(*column, topic, filter_tokens, now)),
            )
        })
    });

    let widths: Vec<Constraint> = columns.iter().map(|column| column.width()).collect();
    let title = format!("AIHOT Hot ({})", indices.len());
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

fn hot_cell(
    column: HotColumn,
    topic: &aihot::HotTopic,
    filter_tokens: &[String],
    now: i64,
) -> Cell<'static> {
    let dim = Style::default().fg(Color::DarkGray);
    match column {
        HotColumn::Rank => Cell::from(fmt_count(topic.rank)).style(dim),
        HotColumn::Title => Cell::from(highlight_matches(
            &topic.title,
            filter_tokens,
            Style::default(),
        )),
        HotColumn::Sources => {
            Cell::from(fmt_count(topic.source_count)).style(Style::default().fg(Color::Green))
        }
        HotColumn::People => Cell::from(fmt_count(topic.participant_count)),
        HotColumn::Seen => {
            Cell::from(fmt_age(now, topic.latest_unix())).style(Style::default().fg(Color::Blue))
        }
        HotColumn::Origin => {
            Cell::from(topic.source.name.clone().unwrap_or_else(|| "-".into())).style(dim)
        }
    }
}

pub(super) fn render_hot_detail(frame: &mut Frame, area: Rect, topic: Option<&aihot::HotTopic>) {
    let dim = Style::default().fg(Color::DarkGray);
    let lines = match topic {
        Some(topic) => {
            let mut lines = vec![
                Line::styled(topic.title.clone(), Style::default().fg(Color::Cyan).bold()),
                Line::from(""),
                detail_line("Rank", fmt_count(topic.rank), Style::default()),
                detail_line(
                    "Sources",
                    fmt_count(topic.source_count),
                    Style::default().fg(Color::Green),
                ),
                detail_line("Signals", fmt_count(topic.signal_count), Style::default()),
                detail_line(
                    "People",
                    fmt_count(topic.participant_count),
                    Style::default(),
                ),
                detail_line(
                    "Latest",
                    topic
                        .latest_at
                        .as_deref()
                        .and_then(aihot::fmt_utc)
                        .unwrap_or_else(|| "-".into()),
                    Style::default().fg(Color::Blue),
                ),
                detail_line(
                    "Origin",
                    topic.source.name.clone().unwrap_or_else(|| "-".into()),
                    Style::default(),
                ),
            ];
            if !topic.source_names.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::styled("Covered by", dim));
                lines.extend(
                    topic
                        .source_names
                        .iter()
                        .map(|name| Line::from(format!("  {name}"))),
                );
            }
            lines.push(Line::from(""));
            for url in [
                &topic.links.original,
                &topic.links.story,
                &topic.links.aihot,
            ]
            .into_iter()
            .flatten()
            {
                lines.push(Line::styled(url.clone(), dim));
            }
            lines.push(Line::styled("via AIHOT (aihot.news)", dim));
            lines
        }
        None => vec![Line::styled("No row selected", dim)],
    };

    let p = Paragraph::new(lines)
        .block(panel("Selected"))
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn fmt_count(n: Option<u32>) -> String {
    n.map_or_else(|| "-".into(), |n| n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_grow_with_width() {
        assert!(!hot_columns(50).contains(&HotColumn::People));
        assert!(hot_columns(60).contains(&HotColumn::People));
        assert!(!hot_columns(99).contains(&HotColumn::Origin));
        assert!(hot_columns(100).contains(&HotColumn::Origin));
    }
}
