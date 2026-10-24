use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use super::{
    color_for_seed, detail_line, finish_table, header_cell, header_row_style, highlight_matches,
    panel, selected_row_style, TableHit,
};
use crate::aihot;
use crate::board::Board;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DailyColumn {
    Index,
    Section,
    Title,
    Source,
}

impl DailyColumn {
    /// The sort key a click on this column's header selects.
    fn sort_key(self) -> Option<char> {
        match self {
            DailyColumn::Index => Some('i'),
            _ => None,
        }
    }

    fn header(self) -> &'static str {
        match self {
            DailyColumn::Index => "#",
            DailyColumn::Section => "Section",
            DailyColumn::Title => "Title",
            DailyColumn::Source => "Source",
        }
    }

    fn width(self) -> Constraint {
        match self {
            DailyColumn::Index => Constraint::Length(3),
            // The widest section label, `模型发布/更新`, is 13 columns.
            DailyColumn::Section => Constraint::Length(13),
            DailyColumn::Title => Constraint::Min(30),
            DailyColumn::Source => Constraint::Length(24),
        }
    }
}

pub(super) fn daily_columns(width: u16) -> Vec<DailyColumn> {
    let mut columns = vec![DailyColumn::Index];
    if width >= 60 {
        columns.push(DailyColumn::Section);
    }
    columns.push(DailyColumn::Title);
    if width >= 100 {
        columns.push(DailyColumn::Source);
    }
    columns
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_daily_table(
    frame: &mut Frame,
    area: Rect,
    items: &[aihot::DailyItem],
    indices: &[usize],
    table_state: &mut std::collections::HashMap<Board, TableState>,
    board: Board,
    filter_tokens: &[String],
    compact: bool,
) -> TableHit {
    let columns = daily_columns(area.width);
    let header = Row::new(
        columns
            .iter()
            .map(|column| header_cell(column.header(), *column == DailyColumn::Index)),
    )
    .style(header_row_style());

    let rows = indices.iter().filter_map(|&idx| {
        items.get(idx).map(|item| {
            Row::new(
                columns
                    .iter()
                    .map(|column| daily_cell(*column, item, filter_tokens)),
            )
        })
    });

    let widths: Vec<Constraint> = columns.iter().map(|column| column.width()).collect();
    let date = items.first().map_or("", |i| i.date.as_str());
    let title = format!("AIHOT Daily {date} ({})", indices.len());
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

fn daily_cell(
    column: DailyColumn,
    item: &aihot::DailyItem,
    filter_tokens: &[String],
) -> Cell<'static> {
    match column {
        DailyColumn::Index => Cell::from(format!("{:>2}", item.position + 1))
            .style(Style::default().fg(Color::DarkGray)),
        DailyColumn::Section => {
            Cell::from(item.section.clone()).style(Style::default().fg(section_color(item)))
        }
        DailyColumn::Title => Cell::from(highlight_matches(
            &item.title,
            filter_tokens,
            Style::default(),
        )),
        DailyColumn::Source => Cell::from(item.source.clone().unwrap_or_else(|| "-".into()))
            .style(Style::default().fg(Color::DarkGray)),
    }
}

pub(super) fn render_daily_detail(frame: &mut Frame, area: Rect, item: Option<&aihot::DailyItem>) {
    let dim = Style::default().fg(Color::DarkGray);
    let lines = match item {
        Some(item) => {
            let mut lines = vec![
                Line::styled(item.title.clone(), Style::default().fg(Color::Cyan).bold()),
                Line::from(""),
                detail_line(
                    "Section",
                    item.section.clone(),
                    Style::default().fg(section_color(item)),
                ),
                detail_line(
                    "Source",
                    item.source.clone().unwrap_or_else(|| "-".into()),
                    Style::default(),
                ),
                detail_line("Report", item.date.clone(), Style::default()),
            ];
            if let Some(published) = item.published_at.as_deref().and_then(aihot::fmt_utc) {
                lines.push(detail_line(
                    "Published",
                    published,
                    Style::default().fg(Color::Blue),
                ));
            }
            if let Some(summary) = &item.summary {
                lines.push(Line::from(""));
                lines.push(Line::from(summary.clone()));
            }
            lines.push(Line::from(""));
            for url in [&item.links.original, &item.links.aihot]
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

fn section_color(item: &aihot::DailyItem) -> Color {
    if item.section == aihot::FLASH_SECTION {
        Color::Yellow
    } else {
        color_for_seed(&item.section)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_grow_with_width() {
        assert!(!daily_columns(50).contains(&DailyColumn::Section));
        assert!(daily_columns(60).contains(&DailyColumn::Section));
        assert!(daily_columns(100).contains(&DailyColumn::Source));
    }
}
