use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use super::chart::{chart_capacity, render_metric_chart, ChartPreference, ChartRow};
use super::{
    color_for_seed, color_from_hex, detail_line, fmt_compact_f, fmt_duration, fmt_f, fmt_price,
    frontier_metric, header_cell, header_row_style, highlight_matches, panel, price_color,
    push_unique, readable_color_for_dark_bg, score_color, selected_row_style, table_hit, truncate,
    AppState, FrontierKey, TableHit,
};
use crate::board::Board;
use crate::frontiercode;

/// Cost-per-rollout bands for price coloring, USD.
const COST_LOW: f64 = 1.0;
const COST_HIGH: f64 = 5.0;
/// Score bands, percent.
const SCORE_LOW: f64 = 35.0;
const SCORE_HIGH: f64 = 50.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FrontierColumn {
    Index,
    Model,
    Lab,
    Score,
    Pass,
    Flag,
    Cost,
    Tokens,
    Time,
    Harness,
}

impl FrontierColumn {
    /// The sort key a click on this column's header selects.
    fn sort_key(self) -> Option<char> {
        match self {
            FrontierColumn::Score => Some('i'),
            FrontierColumn::Pass => Some('a'),
            FrontierColumn::Cost => Some('p'),
            FrontierColumn::Tokens => Some('u'),
            FrontierColumn::Time => Some('t'),
            _ => None,
        }
    }

    fn header(self) -> &'static str {
        match self {
            FrontierColumn::Index => "#",
            FrontierColumn::Model => "Model",
            FrontierColumn::Lab => "Lab",
            FrontierColumn::Score => "Score",
            FrontierColumn::Pass => "Pass",
            FrontierColumn::Flag => "Flag",
            FrontierColumn::Cost => "$/run",
            FrontierColumn::Tokens => "Tokens",
            FrontierColumn::Time => "Time",
            FrontierColumn::Harness => "Harness",
        }
    }

    fn width(self) -> Constraint {
        match self {
            FrontierColumn::Index => Constraint::Length(3),
            FrontierColumn::Model => Constraint::Min(20),
            FrontierColumn::Lab => Constraint::Length(12),
            FrontierColumn::Score => Constraint::Length(6),
            FrontierColumn::Pass => Constraint::Length(6),
            FrontierColumn::Flag => Constraint::Length(5),
            FrontierColumn::Cost => Constraint::Length(7),
            FrontierColumn::Tokens => Constraint::Length(7),
            FrontierColumn::Time => Constraint::Length(6),
            FrontierColumn::Harness => Constraint::Length(14),
        }
    }

    fn order(self) -> u8 {
        match self {
            FrontierColumn::Index => 0,
            FrontierColumn::Model => 1,
            FrontierColumn::Lab => 2,
            FrontierColumn::Score => 3,
            FrontierColumn::Pass => 4,
            FrontierColumn::Flag => 5,
            FrontierColumn::Cost => 6,
            FrontierColumn::Tokens => 7,
            FrontierColumn::Time => 8,
            FrontierColumn::Harness => 9,
        }
    }
}

pub(super) fn frontier_columns(width: u16, sort_key: FrontierKey) -> Vec<FrontierColumn> {
    let mut columns = vec![
        FrontierColumn::Index,
        FrontierColumn::Model,
        FrontierColumn::Score,
        FrontierColumn::Cost,
    ];
    push_unique(&mut columns, frontier_column_for_key(sort_key));
    if width >= 60 {
        push_unique(&mut columns, FrontierColumn::Pass);
    }
    if width >= 70 {
        push_unique(&mut columns, FrontierColumn::Tokens);
    }
    if width >= 82 {
        push_unique(&mut columns, FrontierColumn::Lab);
    }
    if width >= 90 {
        push_unique(&mut columns, FrontierColumn::Time);
    }
    if width >= 98 {
        push_unique(&mut columns, FrontierColumn::Flag);
    }
    if width >= 114 {
        push_unique(&mut columns, FrontierColumn::Harness);
    }
    columns.sort_by_key(|column| column.order());
    columns
}

fn frontier_column_for_key(key: FrontierKey) -> FrontierColumn {
    match key {
        FrontierKey::Score => FrontierColumn::Score,
        FrontierKey::Pass => FrontierColumn::Pass,
        FrontierKey::Cost => FrontierColumn::Cost,
        FrontierKey::Tokens => FrontierColumn::Tokens,
        FrontierKey::Time => FrontierColumn::Time,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_frontier_table(
    frame: &mut Frame,
    area: Rect,
    all_rows: &[frontiercode::Row],
    indices: &[usize],
    table_state: &mut std::collections::HashMap<Board, TableState>,
    board: Board,
    sort_key: FrontierKey,
    filter_tokens: &[String],
    compact: bool,
) -> TableHit {
    let columns = frontier_columns(area.width, sort_key);
    let sorted = frontier_column_for_key(sort_key);
    let header = Row::new(
        columns
            .iter()
            .map(|column| header_cell(column.header(), *column == sorted)),
    )
    .style(header_row_style());

    let table_rows = indices.iter().enumerate().filter_map(|(i, &idx)| {
        all_rows.get(idx).map(|row| {
            Row::new(
                columns
                    .iter()
                    .map(|column| frontier_cell(*column, i, row, filter_tokens)),
            )
        })
    });

    let widths: Vec<Constraint> = columns.iter().map(|column| column.width()).collect();
    let spacing = if compact { 0 } else { 1 };
    let table = Table::new(table_rows, widths.clone())
        .header(header.height(1))
        .row_highlight_style(selected_row_style())
        .highlight_symbol("> ")
        .block(panel(format!("FrontierCode ({})", indices.len())))
        .column_spacing(spacing);

    let st = table_state.entry(board).or_default();
    frame.render_stateful_widget(table, area, st);
    table_hit(
        area,
        &widths,
        spacing,
        st,
        columns.iter().map(|c| c.sort_key()).collect(),
    )
}

fn frontier_cell(
    column: FrontierColumn,
    index: usize,
    row: &frontiercode::Row,
    filter_tokens: &[String],
) -> Cell<'static> {
    match column {
        FrontierColumn::Index => {
            Cell::from(format!("{:>2}", index + 1)).style(Style::default().fg(Color::DarkGray))
        }
        FrontierColumn::Model => Cell::from(highlight_matches(
            &row.display_name(),
            filter_tokens,
            Style::default(),
        )),
        FrontierColumn::Lab => {
            Cell::from(row.lab().to_string()).style(Style::default().fg(lab_color(row)))
        }
        FrontierColumn::Score => Cell::from(fmt_pct(row.score)).style(score_style(row.score)),
        FrontierColumn::Pass => Cell::from(fmt_pct(row.pass_rate)),
        FrontierColumn::Flag => {
            Cell::from(fmt_pct(row.flag_rate)).style(Style::default().fg(Color::DarkGray))
        }
        FrontierColumn::Cost => {
            Cell::from(fmt_price(row.cost, 2, "")).style(price_color(row.cost, COST_LOW, COST_HIGH))
        }
        FrontierColumn::Tokens => Cell::from(fmt_compact_f(row.tokens)),
        FrontierColumn::Time => Cell::from(fmt_duration(minutes_to_secs(row.minutes)))
            .style(Style::default().fg(Color::Blue)),
        FrontierColumn::Harness => Cell::from(row.harness.clone().unwrap_or_else(|| "-".into()))
            .style(Style::default().fg(Color::DarkGray)),
    }
}

pub(super) fn render_frontier_chart(
    frame: &mut Frame,
    area: Rect,
    all_rows: &[frontiercode::Row],
    indices: &[usize],
    app: &AppState,
) {
    let key = app.frontier_sort.key;
    let max_bars = chart_capacity(area);
    let chart_rows: Vec<ChartRow> = indices
        .iter()
        .filter_map(|&idx| {
            let row = all_rows.get(idx)?;
            // Plot rates in percent so the chart's range line matches the labels.
            let value = match key {
                FrontierKey::Score | FrontierKey::Pass => frontier_metric(row, key)? * 100.0,
                _ => frontier_metric(row, key)?,
            };
            if !value.is_finite() {
                return None;
            }
            Some(ChartRow {
                name: row.display_name(),
                meta: row.lab().to_string(),
                value,
                value_label: frontier_chart_text_value(row, key),
                color: Some(lab_color(row)),
                color_seed: row.model.clone(),
            })
        })
        .take(max_bars)
        .collect();

    let title = format!(
        "FrontierCode chart - {} {} (top {}/{})",
        frontier_key_label(key),
        app.frontier_sort.dir.arrow(),
        chart_rows.len(),
        indices.len(),
    );
    let empty = format!(
        "no data for {} - switch sort key (i/a/p/u/t) or press m for table",
        frontier_key_label(key)
    );
    render_metric_chart(
        frame,
        area,
        &title,
        &empty,
        &chart_rows,
        frontier_chart_preference(key),
    );
}

pub(super) fn render_frontier_detail(
    frame: &mut Frame,
    area: Rect,
    row: Option<&frontiercode::Row>,
) {
    let max = area.width.saturating_sub(4) as usize;
    let lines = match row {
        Some(row) => {
            let mut lines = vec![
                Line::styled(
                    truncate(&row.model, max.max(8)),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                detail_line(
                    "Harness",
                    row.harness.clone().unwrap_or_else(|| "-".into()),
                    Style::default().fg(Color::DarkGray),
                ),
                Line::from(""),
                detail_line("Lab", row.lab(), Style::default().fg(lab_color(row))),
                detail_line(
                    "Effort",
                    row.effort.as_deref().unwrap_or("-"),
                    Style::default(),
                ),
                detail_line("Score", fmt_pct(row.score), score_style(row.score)),
                detail_line("Pass", fmt_pct(row.pass_rate), Style::default()),
                detail_line(
                    "Flagged",
                    fmt_pct(row.flag_rate),
                    Style::default().fg(Color::Gray),
                ),
                detail_line(
                    "Cost",
                    fmt_price(row.cost, 2, "/run"),
                    price_color(row.cost, COST_LOW, COST_HIGH),
                ),
                detail_line("Tokens", fmt_compact_f(row.tokens), Style::default()),
                detail_line(
                    "Time",
                    fmt_duration(minutes_to_secs(row.minutes)),
                    Style::default().fg(Color::Blue),
                ),
                detail_line("Steps", fmt_f(row.steps, 0), Style::default()),
            ];
            if row.efforts.len() > 1 {
                lines.push(Line::from(""));
                lines.push(Line::styled(
                    "Score by effort",
                    Style::default().fg(Color::DarkGray),
                ));
                for (effort, score) in &row.efforts {
                    let style = if row.effort.as_deref() == Some(effort.as_str()) {
                        score_style(*score).add_modifier(Modifier::BOLD)
                    } else {
                        score_style(*score)
                    };
                    lines.push(detail_line(effort, fmt_pct(*score), style));
                }
            }
            lines
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

fn fmt_pct(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{:.1}%", x * 100.0),
        None => "-".into(),
    }
}

fn score_style(v: Option<f64>) -> Style {
    score_color(v.map(|x| x * 100.0), SCORE_LOW, SCORE_HIGH)
}

fn minutes_to_secs(v: Option<f64>) -> Option<f64> {
    v.map(|m| m * 60.0)
}

/// The site's own lab color, lifted for a dark background.
fn lab_color(row: &frontiercode::Row) -> Color {
    row.color
        .as_deref()
        .and_then(color_from_hex)
        .map(readable_color_for_dark_bg)
        .unwrap_or_else(|| color_for_seed(row.lab()))
}

fn frontier_chart_preference(key: FrontierKey) -> ChartPreference {
    match key {
        FrontierKey::Cost | FrontierKey::Tokens | FrontierKey::Time => ChartPreference::Lower,
        FrontierKey::Score | FrontierKey::Pass => ChartPreference::Higher,
    }
}

fn frontier_chart_text_value(row: &frontiercode::Row, key: FrontierKey) -> String {
    match key {
        FrontierKey::Score => fmt_pct(row.score),
        FrontierKey::Pass => fmt_pct(row.pass_rate),
        FrontierKey::Cost => fmt_price(row.cost, 2, "/run"),
        FrontierKey::Tokens => fmt_compact_f(row.tokens),
        FrontierKey::Time => fmt_duration(minutes_to_secs(row.minutes)),
    }
}

pub(super) fn frontier_key_label(k: FrontierKey) -> &'static str {
    match k {
        FrontierKey::Score => "Score",
        FrontierKey::Pass => "Pass",
        FrontierKey::Cost => "Cost/run",
        FrontierKey::Tokens => "Tokens",
        FrontierKey::Time => "Time",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentages_keep_one_decimal() {
        assert_eq!(fmt_pct(Some(0.5464)), "54.6%");
        assert_eq!(fmt_pct(Some(0.0)), "0.0%");
        assert_eq!(fmt_pct(None), "-");
    }

    #[test]
    fn columns_keep_the_sort_metric_when_narrow() {
        let cols = frontier_columns(48, FrontierKey::Time);
        assert!(cols.contains(&FrontierColumn::Time));
        assert!(!cols.contains(&FrontierColumn::Harness));
        assert!(frontier_columns(120, FrontierKey::Score).contains(&FrontierColumn::Harness));
    }
}
