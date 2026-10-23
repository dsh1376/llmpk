use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use super::chart::{chart_capacity, render_metric_chart, ChartPreference, ChartRow};
use super::{
    accent_color_for_provider, color_for_seed, detail_line, finish_table, fmt_compact_f,
    fmt_duration, fmt_f, fmt_price, fmt_tokens, header_cell, header_row_style, highlight_matches,
    panel, price_color, push_unique, score_color, selected_row_style, slop_metric, truncate,
    AppState, SlopKey, TableHit,
};
use crate::board::Board;
use crate::slopalytics;
use ratatui::widgets::TableState;

/// Per-task cost bands for price coloring, USD.
const COST_LOW: f64 = 0.5;
const COST_HIGH: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SlopColumn {
    Index,
    Model,
    Lab,
    Intelligence,
    Cost,
    Time,
    Tokens,
    Speed,
    Total,
}

impl SlopColumn {
    /// The sort key a click on this column's header selects.
    fn sort_key(self) -> Option<char> {
        match self {
            SlopColumn::Intelligence => Some('i'),
            SlopColumn::Cost => Some('p'),
            SlopColumn::Time => Some('t'),
            SlopColumn::Tokens => Some('u'),
            SlopColumn::Speed => Some('s'),
            _ => None,
        }
    }

    fn header(self) -> &'static str {
        match self {
            SlopColumn::Index => "#",
            SlopColumn::Model => "Model",
            SlopColumn::Lab => "Lab",
            SlopColumn::Intelligence => "Intel",
            SlopColumn::Cost => "$/task",
            SlopColumn::Time => "Time",
            SlopColumn::Tokens => "Tokens",
            SlopColumn::Speed => "t/s",
            SlopColumn::Total => "Run $",
        }
    }

    fn width(self) -> Constraint {
        match self {
            SlopColumn::Index => Constraint::Length(3),
            SlopColumn::Model => Constraint::Min(20),
            SlopColumn::Lab => Constraint::Length(12),
            SlopColumn::Intelligence => Constraint::Length(6),
            SlopColumn::Cost => Constraint::Length(8),
            SlopColumn::Time => Constraint::Length(7),
            SlopColumn::Tokens => Constraint::Length(7),
            SlopColumn::Speed => Constraint::Length(5),
            SlopColumn::Total => Constraint::Length(8),
        }
    }

    fn order(self) -> u8 {
        match self {
            SlopColumn::Index => 0,
            SlopColumn::Model => 1,
            SlopColumn::Lab => 2,
            SlopColumn::Intelligence => 3,
            SlopColumn::Cost => 4,
            SlopColumn::Time => 5,
            SlopColumn::Tokens => 6,
            SlopColumn::Speed => 7,
            SlopColumn::Total => 8,
        }
    }
}

pub(super) fn slop_columns(width: u16, sort_key: SlopKey) -> Vec<SlopColumn> {
    let mut columns = vec![
        SlopColumn::Index,
        SlopColumn::Model,
        SlopColumn::Intelligence,
        SlopColumn::Cost,
    ];
    push_unique(&mut columns, slop_column_for_key(sort_key));
    if width >= 62 {
        push_unique(&mut columns, SlopColumn::Time);
    }
    if width >= 72 {
        push_unique(&mut columns, SlopColumn::Tokens);
    }
    if width >= 86 {
        push_unique(&mut columns, SlopColumn::Lab);
    }
    if width >= 94 {
        push_unique(&mut columns, SlopColumn::Speed);
    }
    if width >= 104 {
        push_unique(&mut columns, SlopColumn::Total);
    }
    columns.sort_by_key(|column| column.order());
    columns
}

fn slop_column_for_key(key: SlopKey) -> SlopColumn {
    match key {
        SlopKey::Intelligence => SlopColumn::Intelligence,
        SlopKey::Cost => SlopColumn::Cost,
        SlopKey::Time => SlopColumn::Time,
        SlopKey::Tokens => SlopColumn::Tokens,
        SlopKey::Speed => SlopColumn::Speed,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_slop_table(
    frame: &mut Frame,
    area: Rect,
    all_rows: &[slopalytics::Row],
    indices: &[usize],
    table_state: &mut std::collections::HashMap<Board, TableState>,
    board: Board,
    sort_key: SlopKey,
    filter_tokens: &[String],
    compact: bool,
) -> TableHit {
    let columns = slop_columns(area.width, sort_key);
    let sorted = slop_column_for_key(sort_key);
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
                    .map(|column| slop_cell(*column, i, row, filter_tokens)),
            )
        })
    });

    let widths: Vec<Constraint> = columns.iter().map(|column| column.width()).collect();
    let title = format!("Slopalytics ({})", indices.len());

    let spacing = if compact { 0 } else { 1 };
    let table = Table::new(table_rows, widths.clone())
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

fn slop_cell(
    column: SlopColumn,
    index: usize,
    row: &slopalytics::Row,
    filter_tokens: &[String],
) -> Cell<'static> {
    match column {
        SlopColumn::Index => {
            Cell::from(format!("{:>2}", index + 1)).style(Style::default().fg(Color::DarkGray))
        }
        SlopColumn::Model => Cell::from(highlight_matches(
            &row.display_name(),
            filter_tokens,
            Style::default(),
        )),
        SlopColumn::Lab => {
            Cell::from(row.lab().to_string()).style(Style::default().fg(lab_color(row)))
        }
        SlopColumn::Intelligence => {
            Cell::from(fmt_intelligence(row)).style(score_color(row.intelligence, 30.0, 60.0))
        }
        SlopColumn::Cost => {
            Cell::from(fmt_price(row.cost, 2, "")).style(price_color(row.cost, COST_LOW, COST_HIGH))
        }
        SlopColumn::Time => {
            Cell::from(fmt_duration(row.time)).style(Style::default().fg(Color::Blue))
        }
        SlopColumn::Tokens => Cell::from(fmt_compact_f(row.tokens)),
        SlopColumn::Speed => Cell::from(fmt_f(row.speed, 0)),
        SlopColumn::Total => {
            Cell::from(fmt_run_cost(row.cost_total)).style(Style::default().fg(Color::DarkGray))
        }
    }
}

pub(super) fn render_slop_chart(
    frame: &mut Frame,
    area: Rect,
    all_rows: &[slopalytics::Row],
    indices: &[usize],
    app: &AppState,
) {
    let key = app.slop_sort.key;
    let max_bars = chart_capacity(area);
    let chart_rows: Vec<ChartRow> = indices
        .iter()
        .filter_map(|&idx| {
            let row = all_rows.get(idx)?;
            let value = slop_metric(row, key)?;
            if !value.is_finite() {
                return None;
            }
            Some(ChartRow {
                name: row.display_name(),
                meta: row.lab().to_string(),
                value,
                value_label: slop_chart_text_value(row, key),
                color: Some(lab_color(row)),
                color_seed: row.id.clone(),
            })
        })
        .take(max_bars)
        .collect();

    let title = format!(
        "Slopalytics chart - {} {} (top {}/{})",
        slop_key_label(key),
        app.slop_sort.dir.arrow(),
        chart_rows.len(),
        indices.len(),
    );
    let empty = format!(
        "no data for {} - switch sort key (i/p/t/u/s) or press m for table",
        slop_key_label(key)
    );
    render_metric_chart(
        frame,
        area,
        &title,
        &empty,
        &chart_rows,
        slop_chart_preference(key),
    );
}

pub(super) fn render_slop_detail(frame: &mut Frame, area: Rect, row: Option<&slopalytics::Row>) {
    let max = area.width.saturating_sub(4) as usize;
    let lines = match row {
        Some(row) => vec![
            Line::styled(
                truncate(&row.name, max.max(8)),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            detail_line(
                "Variant",
                row.variant.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            detail_line("Lab", row.lab(), Style::default().fg(lab_color(row))),
            detail_line("Effort", row.effort().unwrap_or("-"), Style::default()),
            detail_line(
                "Intel",
                if row.estimated {
                    format!("{} (estimated)", fmt_f(row.intelligence, 1))
                } else {
                    fmt_f(row.intelligence, 1)
                },
                score_color(row.intelligence, 30.0, 60.0),
            ),
            detail_line(
                "Cost",
                fmt_price(row.cost, 3, "/task"),
                price_color(row.cost, COST_LOW, COST_HIGH),
            ),
            detail_line(
                "Time",
                fmt_duration(row.time),
                Style::default().fg(Color::Blue),
            ),
            detail_line("Tokens", fmt_compact_f(row.tokens), Style::default()),
            detail_line(
                "Speed",
                format!("{} t/s", fmt_f(row.speed, 0)),
                Style::default(),
            ),
            Line::from(""),
            detail_line("Run cost", fmt_run_cost(row.cost_total), Style::default()),
            detail_line(
                "Run tokens",
                fmt_compact_f(row.tokens_total),
                Style::default(),
            ),
            detail_line(
                "Input",
                fmt_price(row.input_price, 2, "/M"),
                Style::default(),
            ),
            detail_line(
                "Output",
                fmt_price(row.output_price, 2, "/M"),
                Style::default(),
            ),
            detail_line(
                "Context",
                row.context
                    .filter(|c| c.is_finite() && *c >= 0.0)
                    .map(|c| fmt_tokens(c as u64))
                    .unwrap_or_else(|| "-".into()),
                Style::default(),
            ),
            detail_line(
                "Release",
                row.released.clone().unwrap_or_else(|| "-".into()),
                Style::default().fg(Color::Gray),
            ),
        ],
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

/// AA marks some index scores as estimated; a leading `~` keeps them
/// distinguishable without another column.
fn fmt_intelligence(row: &slopalytics::Row) -> String {
    match (row.intelligence, row.estimated) {
        (Some(v), true) => format!("~{v:.1}"),
        (v, _) => fmt_f(v, 1),
    }
}

/// Full index-run cost, compact: `$860`, `$8.7k`.
fn fmt_run_cost(v: Option<f64>) -> String {
    match v {
        Some(x) if x >= 1_000.0 => format!("${:.1}k", x / 1_000.0),
        Some(x) => format!("${x:.0}"),
        None => "-".into(),
    }
}

fn lab_color(row: &slopalytics::Row) -> Color {
    accent_color_for_provider(row.lab()).unwrap_or_else(|| color_for_seed(row.lab()))
}

fn slop_chart_preference(key: SlopKey) -> ChartPreference {
    match key {
        SlopKey::Cost | SlopKey::Time | SlopKey::Tokens => ChartPreference::Lower,
        SlopKey::Intelligence | SlopKey::Speed => ChartPreference::Higher,
    }
}

fn slop_chart_text_value(row: &slopalytics::Row, key: SlopKey) -> String {
    match key {
        SlopKey::Intelligence => fmt_intelligence(row),
        SlopKey::Cost => fmt_price(row.cost, 2, "/task"),
        SlopKey::Time => fmt_duration(row.time),
        SlopKey::Tokens => fmt_compact_f(row.tokens),
        SlopKey::Speed => format!("{} t/s", fmt_f(row.speed, 0)),
    }
}

pub(super) fn slop_key_label(k: SlopKey) -> &'static str {
    match k {
        SlopKey::Intelligence => "Intelligence",
        SlopKey::Cost => "Cost/task",
        SlopKey::Time => "Time/task",
        SlopKey::Tokens => "Tokens/task",
        SlopKey::Speed => "Speed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cost_is_compact() {
        assert_eq!(fmt_run_cost(Some(860.3)), "$860");
        assert_eq!(fmt_run_cost(Some(8708.2)), "$8.7k");
        assert_eq!(fmt_run_cost(None), "-");
    }

    #[test]
    fn columns_keep_the_sort_metric_when_narrow() {
        let cols = slop_columns(48, SlopKey::Speed);
        assert!(cols.contains(&SlopColumn::Speed));
        assert!(!cols.contains(&SlopColumn::Total));
        assert!(slop_columns(120, SlopKey::Intelligence).contains(&SlopColumn::Total));
    }
}
