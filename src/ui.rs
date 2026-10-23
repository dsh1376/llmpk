use ratatui::{
    layout::Margin,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Scrollbar, ScrollbarOrientation, ScrollbarState, TableState},
    Frame,
};

mod aa_board;
mod agents;
mod chart;
mod chrome;
mod filter;
mod frontier_board;
mod slop_board;
mod sort;
pub mod state;
#[cfg(test)]
mod test_helpers;
mod tracker_board;

pub use sort::*;
pub use state::*;

use crate::board::{Data, Status};
use filter::filter_tokens;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn render(frame: &mut Frame, app: &mut AppState) {
    if frame.area().width < 36 || frame.area().height < 8 {
        chrome::render_tiny(frame);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(frame.area());

    chrome::render_top(frame, chunks[0], app);
    render_body(frame, chunks[1], app);
    chrome::render_footer(frame, chunks[2], app);

    if app.is_help_open() {
        chrome::render_help_overlay(frame, frame.area(), &mut app.help_scroll);
    }
}

fn render_body(frame: &mut Frame, area: Rect, app: &mut AppState) {
    // Each table render below records where it drew; anything else clears it.
    app.set_table_hit(None);
    let mut hit = None;
    let board = app.current_board();
    let query = app.current_filter().to_string();
    let view = app.current_view();
    let aa_sort_key = app.aa_sort.key;
    let agent_sort_key = app.agent_sort.key;
    let slop_sort_key = app.slop_sort.key;
    let frontier_sort_key = app.frontier_sort.key;
    let indices = app.filter_cache(board, &query).to_vec();
    let filter_tokens = filter_tokens(&query);
    let table_state = &mut app.table_state;

    match app.status.get(&board) {
        Some(Status::Loaded(Data::Aa(models))) => {
            if indices.is_empty() && filter_is_active(&query) {
                chrome::render_filter_empty(frame, area, &query);
                return;
            }
            let selected = table_state
                .get(&board)
                .and_then(TableState::selected)
                .unwrap_or(0);
            match view {
                View::Table => {
                    let (table_area, detail_area) = split_body(area);
                    hit = Some(aa_board::render_aa_table(
                        frame,
                        table_area,
                        models,
                        &indices,
                        table_state,
                        board,
                        aa_sort_key,
                        &filter_tokens,
                        app.compact,
                    ));
                    if let Some(detail_area) = detail_area {
                        let model = indices.get(selected).and_then(|&i| models.get(i));
                        aa_board::render_aa_detail(frame, detail_area, model);
                    }
                }
                View::Chart => aa_board::render_aa_chart(frame, area, models, &indices, app),
            }
        }
        Some(Status::Loaded(Data::AaAgents(rows))) => {
            if indices.is_empty() && filter_is_active(&query) {
                chrome::render_filter_empty(frame, area, &query);
                return;
            }
            let selected = table_state
                .get(&board)
                .and_then(TableState::selected)
                .unwrap_or(0);
            match view {
                View::Table => {
                    let (table_area, detail_area) = split_agents_body(area);
                    hit = Some(agents::render_agents_table(
                        frame,
                        table_area,
                        rows,
                        &indices,
                        table_state,
                        board,
                        agent_sort_key,
                        &filter_tokens,
                        app.compact,
                    ));
                    if let Some(detail_area) = detail_area {
                        let row = indices.get(selected).and_then(|&i| rows.get(i));
                        agents::render_agent_detail(frame, detail_area, row);
                    }
                }
                View::Chart => agents::render_agents_chart(frame, area, rows, &indices, app),
            }
        }
        Some(Status::Loaded(Data::Slop(rows))) => {
            if indices.is_empty() && filter_is_active(&query) {
                chrome::render_filter_empty(frame, area, &query);
                return;
            }
            let selected = table_state
                .get(&board)
                .and_then(TableState::selected)
                .unwrap_or(0);
            match view {
                View::Table => {
                    let (table_area, detail_area) = split_slop_body(area);
                    hit = Some(slop_board::render_slop_table(
                        frame,
                        table_area,
                        rows,
                        &indices,
                        table_state,
                        board,
                        slop_sort_key,
                        &filter_tokens,
                        app.compact,
                    ));
                    if let Some(detail_area) = detail_area {
                        let row = indices.get(selected).and_then(|&i| rows.get(i));
                        slop_board::render_slop_detail(frame, detail_area, row);
                    }
                }
                View::Chart => slop_board::render_slop_chart(frame, area, rows, &indices, app),
            }
        }
        Some(Status::Loaded(Data::Frontier(rows))) => {
            if indices.is_empty() && filter_is_active(&query) {
                chrome::render_filter_empty(frame, area, &query);
                return;
            }
            let selected = table_state
                .get(&board)
                .and_then(TableState::selected)
                .unwrap_or(0);
            match view {
                View::Table => {
                    let (table_area, detail_area) = split_slop_body(area);
                    hit = Some(frontier_board::render_frontier_table(
                        frame,
                        table_area,
                        rows,
                        &indices,
                        table_state,
                        board,
                        frontier_sort_key,
                        &filter_tokens,
                        app.compact,
                    ));
                    if let Some(detail_area) = detail_area {
                        let row = indices.get(selected).and_then(|&i| rows.get(i));
                        frontier_board::render_frontier_detail(frame, detail_area, row);
                    }
                }
                View::Chart => {
                    frontier_board::render_frontier_chart(frame, area, rows, &indices, app)
                }
            }
        }
        Some(Status::Loaded(Data::Tracker(pages))) => {
            if indices.is_empty() && filter_is_active(&query) {
                chrome::render_filter_empty(frame, area, &query);
                return;
            }
            let selected = table_state
                .get(&board)
                .and_then(TableState::selected)
                .unwrap_or(0);
            let (table_area, detail_area) = split_tracker_body(area);
            hit = Some(tracker_board::render_tracker_table(
                frame,
                table_area,
                pages,
                &indices,
                table_state,
                board,
                &filter_tokens,
                app.compact,
            ));
            if let Some(detail_area) = detail_area {
                let page = indices.get(selected).and_then(|&i| pages.get(i));
                tracker_board::render_tracker_detail(frame, detail_area, page);
            }
        }
        Some(Status::Error(e)) => {
            chrome::render_error(frame, area, board, e);
        }
        Some(Status::Loading) | None => {
            chrome::render_loading(frame, area, board);
        }
    }
    app.set_table_hit(hit);
}

fn split_body(area: Rect) -> (Rect, Option<Rect>) {
    if area.width < 116 || area.height < 12 {
        return (area, None);
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(72), Constraint::Length(34)])
        .split(area);
    (chunks[0], Some(chunks[1]))
}

fn split_agents_body(area: Rect) -> (Rect, Option<Rect>) {
    if area.width < 132 || area.height < 12 {
        return (area, None);
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(88), Constraint::Length(38)])
        .split(area);
    (chunks[0], Some(chunks[1]))
}

fn split_slop_body(area: Rect) -> (Rect, Option<Rect>) {
    if area.width < 110 || area.height < 12 {
        return (area, None);
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(72), Constraint::Length(34)])
        .split(area);
    (chunks[0], Some(chunks[1]))
}

fn split_tracker_body(area: Rect) -> (Rect, Option<Rect>) {
    if area.width < 110 || area.height < 12 {
        return (area, None);
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(72), Constraint::Length(38)])
        .split(area);
    (chunks[0], Some(chunks[1]))
}

pub(crate) fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if !items.contains(&item) {
        items.push(item);
    }
}

/// Background of the column-header bar; the row style fills the gaps
/// between cells so the bar reads as one strip.
const HEADER_BG: Color = Color::Rgb(0x2c, 0x32, 0x3a);
/// Background of the selected row.
const SELECTED_BG: Color = Color::Rgb(0x1f, 0x3a, 0x52);

/// Column header label; the active sort column is drawn in the accent color.
pub(crate) fn header_cell(label: &str, sorted: bool) -> Cell<'_> {
    let fg = if sorted { Color::Cyan } else { Color::Gray };
    Cell::from(label).style(Style::default().fg(fg).add_modifier(Modifier::BOLD))
}

/// Draw a vertical scrollbar over the right border of `area`, for `total`
/// lines of which `visible` show from `offset`. Nothing is drawn when
/// everything fits. Returns the track's cells (one column wide).
pub(crate) fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    total: usize,
    visible: usize,
    offset: usize,
) -> Option<Rect> {
    if total <= visible || visible == 0 || area.height == 0 {
        return None;
    }
    // Positions run 0..=total-visible, so the thumb reaches the bottom on
    // the last page and its length is the visible share.
    let mut state = ScrollbarState::new(total - visible + 1)
        .position(offset.min(total - visible))
        .viewport_content_length(visible);
    // Single-line track to match the panel borders it sits on.
    let bar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .symbols(ratatui::symbols::scrollbar::VERTICAL)
        .begin_symbol(None)
        .end_symbol(None)
        .track_style(Style::default().fg(Color::DarkGray))
        .thumb_style(Style::default().fg(Color::Gray));
    frame.render_stateful_widget(bar, area, &mut state);
    Some(Rect {
        x: area.right().saturating_sub(1),
        width: 1,
        ..area
    })
}

/// Finish a bordered table: draw its scrollbar and record where it drew its
/// header, columns, and rows for mouse hit-testing. Mirrors the `Table`
/// layout: a two-cell gutter for the `"> "` highlight symbol when a row is
/// selected, then the columns with `Flex::Start` and the spacing.
pub(crate) fn finish_table(
    frame: &mut Frame,
    area: Rect,
    widths: &[Constraint],
    spacing: u16,
    state: &TableState,
    sort_keys: Vec<Option<char>>,
    total: usize,
) -> TableHit {
    let inner = area.inner(Margin::new(1, 1));
    let rows = Rect {
        y: inner.y.saturating_add(1),
        height: inner.height.saturating_sub(1),
        ..inner
    };
    let scroll_track = render_scrollbar(
        frame,
        Rect {
            x: area.x,
            width: area.width,
            ..rows
        },
        total,
        usize::from(rows.height),
        state.offset(),
    );
    let gutter = if state.selected().is_some() { 2 } else { 0 };
    let [_, columns_area] = Layout::horizontal([Constraint::Length(gutter), Constraint::Fill(0)])
        .areas(Rect::new(inner.x, 0, inner.width, 1));
    let rects = Layout::horizontal(widths.iter().copied())
        .flex(ratatui::layout::Flex::Start)
        .spacing(spacing)
        .split(columns_area);
    TableHit {
        header_y: inner.y,
        rows,
        offset: state.offset(),
        scroll_track,
        total,
        columns: rects
            .iter()
            .zip(sort_keys)
            .map(|(r, key)| (r.x, r.width, key))
            .collect(),
    }
}

/// Bordered panel shared by tables, charts, and detail panes. The border is
/// dim so the data, not the frames, carries the color.
pub(crate) fn panel<'a>(title: impl Into<String>) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" {} ", title.into()),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        ))
}

pub(crate) fn header_row_style() -> Style {
    Style::default().bg(HEADER_BG)
}

pub(crate) fn selected_row_style() -> Style {
    Style::default()
        .bg(SELECTED_BG)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn color_for_seed(seed: &str) -> Color {
    const PALETTE: &[Color] = &[
        Color::Cyan,
        Color::Magenta,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Red,
        Color::LightCyan,
        Color::LightMagenta,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightRed,
    ];
    let mut h: u64 = 1469598103934665603;
    for b in seed.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    PALETTE[(h as usize) % PALETTE.len()]
}

pub(crate) fn color_from_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim().trim_start_matches('#');
    let bytes = hex.as_bytes();
    if bytes.len() != 6 && bytes.len() != 8 {
        return None;
    }
    let r = parse_hex_byte(bytes, 0)?;
    let g = parse_hex_byte(bytes, 2)?;
    let b = parse_hex_byte(bytes, 4)?;
    Some(Color::Rgb(r, g, b))
}

fn parse_hex_byte(bytes: &[u8], offset: usize) -> Option<u8> {
    Some(hex_nibble(bytes[offset])? << 4 | hex_nibble(bytes[offset + 1])?)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn readable_color_for_dark_bg(color: Color) -> Color {
    const MIN_LUMA: f64 = 128.0;

    match color {
        Color::Black | Color::DarkGray => Color::Gray,
        Color::Rgb(r, g, b) => {
            let luma = rgb_luma(r, g, b);
            if luma >= MIN_LUMA {
                return color;
            }

            let mix = ((MIN_LUMA - luma) / (255.0 - luma)).clamp(0.0, 1.0);
            Color::Rgb(
                lift_channel(r, mix),
                lift_channel(g, mix),
                lift_channel(b, mix),
            )
        }
        _ => color,
    }
}

pub(crate) fn accent_color_for_provider(provider: &str) -> Option<Color> {
    let color = match normalized_provider(provider).as_str() {
        "openai" => Color::Rgb(0x1f, 0x1f, 0x1f),
        "anthropic" => Color::Rgb(0xcc, 0x78, 0x5c),
        "google" => Color::Rgb(0x34, 0xa8, 0x53),
        "meta" => Color::Rgb(0x00, 0x89, 0xf4),
        "deepseek" => Color::Rgb(0x22, 0x43, 0xe6),
        "mistral" => Color::Rgb(0xfd, 0x6f, 0x00),
        "xai" => Color::Rgb(0x73, 0x6c, 0xd3),
        "amazon" | "aws" | "amazonbedrock" => Color::Rgb(0xff, 0x99, 0x00),
        "microsoft" | "azure" | "microsoftazure" => Color::Rgb(0x00, 0x78, 0xd5),
        "minimax" => Color::Rgb(0xeb, 0x35, 0x68),
        "nvidia" => Color::Rgb(0x86, 0xb7, 0x37),
        "kimi" | "moonshot" | "moonshotai" => Color::Rgb(0x04, 0x7a, 0xfe),
        "alibaba" | "qwen" | "alibabacloud" => Color::Rgb(0xff, 0x70, 0x18),
        "zai" | "z" | "zhipu" | "zhipuai" => Color::Rgb(0x1c, 0x7f, 0xf8),
        "upstage" => Color::Rgb(0x7c, 0x59, 0xf5),
        "ibm" => Color::Rgb(0x0f, 0x62, 0xfe),
        "stepfun" => Color::Rgb(0x01, 0x7a, 0xff),
        "perplexity" => Color::Rgb(0x1b, 0x81, 0x8e),
        "cohere" => Color::Rgb(0x39, 0x60, 0x99),
        "xaiinc" => Color::Rgb(0x73, 0x6c, 0xd3),
        "xiaomi" | "mimo" => Color::Rgb(0xff, 0x69, 0x00),
        _ => return None,
    };
    Some(readable_color_for_dark_bg(color))
}

fn normalized_provider(provider: &str) -> String {
    provider
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn rgb_luma(r: u8, g: u8, b: u8) -> f64 {
    f64::from(r) * 0.2126 + f64::from(g) * 0.7152 + f64::from(b) * 0.0722
}

fn lift_channel(channel: u8, mix: f64) -> u8 {
    (f64::from(channel) + (255.0 - f64::from(channel)) * mix).round() as u8
}

pub(crate) fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('\u{2026}');
        out
    }
}

pub(crate) fn detail_line(label: &str, value: impl Into<String>, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<12}"), Style::default().fg(Color::DarkGray)),
        Span::styled(value.into(), style),
    ])
}

pub(crate) fn filter_is_active(query: &str) -> bool {
    !query.trim().is_empty()
}

pub(crate) fn format_filter_label(query: &str, editing: bool) -> String {
    let mut label = if query.is_empty() {
        "<empty>".to_string()
    } else {
        truncate(query, 32)
    };
    if editing {
        label.push('|');
    }
    label
}

pub(crate) fn fmt_f(v: Option<f64>, decimals: usize) -> String {
    match v {
        Some(x) => format!("{x:.*}", decimals),
        None => "-".into(),
    }
}

/// Formats `v` at `decimals`; positive values that would render as all zeros come
/// back as `<0.01`-style bounds, matching how artificialanalysis.ai prints prices.
pub(crate) fn fmt_price(v: Option<f64>, decimals: usize, suffix: &str) -> String {
    let unit = 10f64.powi(-(decimals as i32));
    match v {
        Some(x) if x > 0.0 && x < unit / 2.0 => format!("<${unit:.decimals$}{suffix}"),
        Some(x) => format!("${x:.*}{suffix}", decimals),
        None => "-".into(),
    }
}

pub(crate) fn fmt_percent(v: Option<f64>) -> String {
    match v {
        Some(x) if x.is_finite() => format!("{}%", (x * 100.0).round()),
        Some(_) | None => "-".into(),
    }
}

pub(crate) fn fmt_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        n.to_string()
    }
}

pub(crate) fn format_compact(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{}K", n / 1_000)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub(crate) fn fmt_duration(v: Option<f64>) -> String {
    let Some(seconds) = v else {
        return "-".into();
    };
    if seconds >= 3600.0 {
        format!("{:.1}h", seconds / 3600.0)
    } else if seconds >= 60.0 {
        format!("{:.1}m", seconds / 60.0)
    } else {
        format!("{seconds:.0}s")
    }
}

pub(crate) fn fmt_compact_f(v: Option<f64>) -> String {
    match v {
        Some(x) if x.is_finite() && x >= 0.0 => format_compact(x.round() as u64),
        Some(_) | None => "-".into(),
    }
}

pub(crate) fn score_color(v: Option<f64>, low: f64, high: f64) -> Style {
    let Some(x) = v else {
        return Style::default().fg(Color::DarkGray);
    };
    let color = if x >= high - (high - low) * 0.2 {
        Color::Green
    } else if x >= (high + low) / 2.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    Style::default().fg(color).bold()
}

pub(crate) fn price_color(v: Option<f64>, low: f64, high: f64) -> Style {
    let Some(x) = v else {
        return Style::default().fg(Color::DarkGray);
    };
    let color = if x <= low {
        Color::Green
    } else if x <= high {
        Color::Yellow
    } else {
        Color::Red
    };
    Style::default().fg(color)
}

/// Highlight matching tokens in text. Returns a Line with highlighted matches.
pub(crate) fn highlight_matches(text: &str, tokens: &[String], base_style: Style) -> Line<'static> {
    if tokens.is_empty() || tokens.iter().all(|t| t.is_empty()) {
        return Line::styled(text.to_string(), base_style);
    }
    let lower = text.to_lowercase();
    let mut spans = Vec::new();
    let mut last_end = 0;

    // Find first match of any token.
    loop {
        let mut earliest: Option<(usize, usize)> = None; // (start, end)
        for token in tokens {
            if let Some(pos) = lower[last_end..].find(token.as_str()) {
                let abs_pos = last_end + pos;
                let end = abs_pos + token.len();
                match earliest {
                    Some((s, _)) if abs_pos < s => earliest = Some((abs_pos, end)),
                    Some(_) => {}
                    None => earliest = Some((abs_pos, end)),
                }
            }
        }
        if let Some((start, end)) = earliest {
            if start > last_end {
                spans.push(Span::styled(text[last_end..start].to_string(), base_style));
            }
            spans.push(Span::styled(
                text[start..end].to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            last_end = end;
        } else {
            break;
        }
    }
    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_string(), base_style));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::chart::{
        chart_bar_len, chart_capacity, render_metric_chart, ChartPreference, ChartRow,
    };
    use super::test_helpers::*;
    use super::*;
    use crate::board::Board;

    #[test]
    fn parses_hex_colors_and_maps_provider_accents() {
        assert_eq!(
            color_from_hex("#cc785c"),
            Some(Color::Rgb(0xcc, 0x78, 0x5c))
        );
        assert_eq!(
            color_from_hex("#7b61ff00"),
            Some(Color::Rgb(0x7b, 0x61, 0xff))
        );
        assert_eq!(
            accent_color_for_provider("Z.ai"),
            Some(readable_color_for_dark_bg(Color::Rgb(0x1c, 0x7f, 0xf8)))
        );
        assert_eq!(
            accent_color_for_provider("Moonshot AI"),
            Some(readable_color_for_dark_bg(Color::Rgb(0x04, 0x7a, 0xfe)))
        );
        assert_eq!(color_from_hex("#nothex"), None);
        assert_eq!(color_from_hex("aébcd"), None);
    }

    #[test]
    fn dark_provider_accents_are_lifted_for_black_backgrounds() {
        assert_eq!(
            readable_color_for_dark_bg(Color::Rgb(0x1f, 0x1f, 0x1f)),
            Color::Rgb(0x80, 0x80, 0x80)
        );
        assert_eq!(
            readable_color_for_dark_bg(Color::Rgb(0xcc, 0x78, 0x5c)),
            Color::Rgb(0xcc, 0x78, 0x5c)
        );
        assert_eq!(
            accent_color_for_provider("OpenAI"),
            Some(Color::Rgb(0x80, 0x80, 0x80))
        );
    }

    #[test]
    fn horizontal_chart_capacity_scales_with_height() {
        let short = chart_capacity(Rect::new(0, 0, 120, 12));
        let tall = chart_capacity(Rect::new(0, 0, 120, 24));
        let narrow_same_height = chart_capacity(Rect::new(0, 0, 70, 24));

        assert!(tall > short);
        assert_eq!(tall, narrow_same_height);
        assert!(short >= 1);
    }

    #[test]
    fn chart_bar_lengths_handle_direction_and_equal_ranges() {
        let high_better = chart_bar_len(10.0, 1.0, 10.0, 12, ChartPreference::Higher);
        let low_better = chart_bar_len(1.0, 1.0, 10.0, 12, ChartPreference::Lower);
        let equal = chart_bar_len(5.0, 5.0, 5.0, 12, ChartPreference::Higher);
        let missing = chart_bar_len(f64::NAN, 1.0, 10.0, 12, ChartPreference::Higher);

        assert_eq!(high_better, 12);
        assert_eq!(low_better, 12);
        assert_eq!(equal, 12);
        assert_eq!(missing, 0);

        // The worst value in range still draws one cell, so its row does
        // not read as missing data.
        assert_eq!(
            chart_bar_len(1.0, 1.0, 10.0, 12, ChartPreference::Higher),
            1
        );
        assert_eq!(
            chart_bar_len(10.0, 1.0, 10.0, 12, ChartPreference::Lower),
            1
        );
    }

    #[test]
    fn narrow_chart_uses_compact_text_without_bars() {
        let rows = vec![ChartRow {
            name: "Very Long Model Name".to_string(),
            meta: "Anthropic".to_string(),
            value: 50.0,
            value_label: "50.0".to_string(),
            color: Some(Color::Rgb(0xcc, 0x78, 0x5c)),
            color_seed: "claude".to_string(),
        }];
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(36, 8)).unwrap();
        terminal
            .draw(|frame| {
                render_metric_chart(
                    frame,
                    Rect::new(0, 0, 36, 8),
                    "chart",
                    "empty",
                    &rows,
                    ChartPreference::Higher,
                )
            })
            .unwrap();
        let text = rendered_text(terminal.backend());

        assert!(text.contains("Top 1 visible"));
        assert!(text.contains("50.0"));
        assert!(text.contains("Very Long Model"));
        assert!(!text.contains("█"));
    }

    #[test]
    fn wide_chart_keeps_value_close_to_label() {
        let rows = vec![ChartRow {
            name: "Codex".to_string(),
            meta: "OpenAI".to_string(),
            value: 60.0,
            value_label: "60.0".to_string(),
            color: Some(Color::Rgb(0xbe, 0xa5, 0xff)),
            color_seed: "codex".to_string(),
        }];
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 8)).unwrap();
        terminal
            .draw(|frame| {
                render_metric_chart(
                    frame,
                    Rect::new(0, 0, 100, 8),
                    "chart",
                    "empty",
                    &rows,
                    ChartPreference::Higher,
                )
            })
            .unwrap();

        let line = rendered_line(terminal.backend(), 100, 2);
        let name_pos = line.find("Codex").expect("row should include chart label");
        let value_pos = line.find("60.0").expect("row should include value");

        assert!(value_pos - (name_pos + "Codex".len()) <= 14);
    }

    #[test]
    fn price_formatting_bounds_sub_cent_values() {
        assert_eq!(fmt_price(Some(0.0036), 2, "/M"), "<$0.01/M");
        assert_eq!(fmt_price(Some(0.006), 2, "/M"), "$0.01/M");
        assert_eq!(fmt_price(Some(0.0), 2, "/M"), "$0.00/M");
        assert_eq!(fmt_price(None, 2, "/M"), "-");
    }

    #[test]
    fn percent_formatting_rounds_and_handles_missing() {
        assert_eq!(fmt_percent(Some(0.9)), "90%");
        assert_eq!(fmt_percent(Some(0.9917)), "99%");
        assert_eq!(fmt_percent(Some(0.004)), "0%");
        assert_eq!(fmt_percent(None), "-");
    }

    #[test]
    fn responsive_aa_columns_keep_active_sort_metric_visible() {
        let cols = aa_board::aa_columns(48, AaKey::Speed);

        assert!(cols.contains(&aa_board::AaColumn::Speed));
        assert!(cols.contains(&aa_board::AaColumn::Model));
        assert!(cols.contains(&aa_board::AaColumn::Intelligence));
        assert!(cols.contains(&aa_board::AaColumn::Price));
        assert!(!cols.contains(&aa_board::AaColumn::Released));

        let cache = aa_board::aa_columns(70, AaKey::Cache);
        assert!(cache.contains(&aa_board::AaColumn::Cache));
        assert!(aa_board::aa_columns(70, AaKey::Intelligence).contains(&aa_board::AaColumn::Cache));
        assert!(!aa_board::aa_columns(69, AaKey::Intelligence).contains(&aa_board::AaColumn::Cache));
    }

    #[test]
    fn responsive_agent_columns_keep_active_sort_metric_visible() {
        let cols = agents::agent_columns(48, AgentKey::Time);

        assert!(cols.contains(&agents::AgentColumn::Time));
        assert!(cols.contains(&agents::AgentColumn::Agent));
        assert!(cols.contains(&agents::AgentColumn::Score));
        assert!(!cols.contains(&agents::AgentColumn::Released));
    }

    #[test]
    fn render_draws_tiny_and_wide_layouts() {
        let mut tiny = AppState::new();
        let mut tiny_terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(35, 7)).unwrap();
        tiny_terminal
            .draw(|frame| render(frame, &mut tiny))
            .unwrap();
        assert!(rendered_text(tiny_terminal.backend()).contains("larger terminal"));

        let mut wide = AppState::new();
        wide.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![aa_model(
                "claude-sonnet",
                "Claude Sonnet",
                "Anthropic",
            )])),
        );
        let mut wide_terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(128, 32)).unwrap();
        wide_terminal
            .draw(|frame| render(frame, &mut wide))
            .unwrap();
        let text = rendered_text(wide_terminal.backend());

        assert!(text.contains("AA ("));
        assert!(text.contains("Selected"));
        assert!(text.contains("Claude Sonnet"));
    }

    #[test]
    fn render_draws_aa_chart_view() {
        let mut app = AppState::new();
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![
                aa_model("claude-sonnet", "Claude Sonnet", "Anthropic"),
                aa_model("gpt-5", "GPT-5", "OpenAI"),
            ])),
        );
        app.toggle_view();

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(132, 32)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());

        assert!(text.contains("AA chart"));
        assert!(text.contains("Top 2 visible"));
        assert!(text.contains("Anthropic"));
        assert!(text.contains("OpenAI"));
        assert!(text.contains("Claude Sonnet"));
        assert!(text.contains("50.0"));
        assert!(text.contains("view: chart"));
    }

    #[test]
    fn render_draws_agents_chart_view() {
        let mut app = AppState::new();
        app.select_board(1);
        app.set_status(
            Board::AaAgents,
            Status::Loaded(Data::AaAgents(vec![
                agent_row("claude-code", "Claude Code", "Anthropic"),
                agent_row("codex", "Codex", "OpenAI"),
            ])),
        );
        app.toggle_view();

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(132, 32)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());

        assert!(text.contains("AA Agents chart"));
        assert!(text.contains("Top 2 visible"));
        assert!(text.contains("Anthropic"));
        assert!(text.contains("view: chart"));
        assert!(text.contains("Claude Code"));
        assert!(text.contains("60.0"));
    }

    #[test]
    fn render_draws_agents_table_with_detail_panel() {
        let mut app = AppState::new();
        app.select_board(1);
        app.set_status(
            Board::AaAgents,
            Status::Loaded(Data::AaAgents(vec![
                agent_row("claude-code", "Claude Code", "Anthropic"),
                agent_row("codex", "Codex", "OpenAI"),
                agent_row("cursor", "Cursor CLI", "Anysphere"),
                agent_row("gemini", "Gemini CLI", "Google"),
                agent_row("opencode", "Opencode", "SST"),
                agent_row("goose", "Goose", "Block"),
            ])),
        );

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(150, 34)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());

        assert!(text.contains("AA Agents ("));
        assert!(!text.contains("Radar"));
        assert!(!text.contains("Legend"));
        assert!(text.contains("Selected"));
        assert!(text.contains("Claude Code"));
        assert!(text.contains("Codex"));
        assert!(text.contains("Pass@1"));
        assert!(text.contains("$1.25/task"));
    }

    fn rendered_text(backend: &ratatui::backend::TestBackend) -> String {
        backend
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn rendered_line(backend: &ratatui::backend::TestBackend, width: usize, row: usize) -> String {
        backend.buffer().content()[row * width..(row + 1) * width]
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn render_shows_version_in_title() {
        let mut app = AppState::new();
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![aa_model("test", "Test Model", "Provider")])),
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());

        assert!(text.contains("llmpk v"));
    }

    #[test]
    fn render_shows_position_indicator() {
        let mut app = AppState::new();
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![
                aa_model("m0", "Model 0", "P"),
                aa_model("m1", "Model 1", "P"),
                aa_model("m2", "Model 2", "P"),
                aa_model("m3", "Model 3", "P"),
                aa_model("m4", "Model 4", "P"),
            ])),
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());

        assert!(text.contains("1/5"));
    }

    fn tracker_page(
        slug: &str,
        model: &str,
        maker: &str,
        stage: &str,
    ) -> crate::aitracker::ModelPage {
        crate::aitracker::ModelPage {
            slug: slug.to_string(),
            model: model.to_string(),
            maker: Some(maker.to_string()),
            stage: Some(stage.to_string()),
            last_seen_at: Some("2026-10-22T15:43:41.669Z".to_string()),
            path: Some(format!("/{slug}")),
            post_ids: vec!["1".to_string()],
            event_ids: Vec::new(),
        }
    }

    #[test]
    fn mouse_hits_the_tab_under_the_cursor() {
        let wide = [
            "1 AA",
            "2 AA Agents",
            "3 Slopalytics",
            "4 FrontierCode",
            "5 AI Tracker",
        ];
        let short = ["1 AA", "2 AAg", "3 Slop", "4 FC", "5 AIT"];
        for (width, labels) in [(120u16, wide), (44, short)] {
            let mut app = AppState::new();
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 30)).unwrap();
            terminal.draw(|frame| render(frame, &mut app)).unwrap();
            let tab_row = rendered_line(terminal.backend(), width as usize, 1);

            for (idx, label) in labels.iter().enumerate() {
                let byte = tab_row.find(label).unwrap_or_else(|| {
                    panic!("{width} cols: tab bar should show {label:?}: {tab_row:?}")
                });
                let col = tab_row[..byte].chars().count();
                for offset in 0..label.chars().count() {
                    assert_eq!(
                        app.tab_at((col + offset) as u16, 1),
                        Some(idx),
                        "{width} cols: column {} of {label:?}",
                        col + offset
                    );
                }
            }
            // Space right of the last tab, and the info line below the tabs.
            assert_eq!(app.tab_at(width - 3, 1), None);
            assert_eq!(app.tab_at(3, 2), None);
        }
    }

    #[test]
    fn help_overlay_scrolls_to_its_last_line() {
        let mut app = AppState::new();
        app.toggle_help();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(132, 30)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());
        assert!(text.contains("Boards"));
        assert!(!text.contains("Press ?, h, or Esc to close"));

        app.scroll_help(i32::MAX);
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());
        assert!(text.contains("Ctrl-U"));
        assert!(text.contains("Press ?, h, or Esc to close"));

        // Scrolling back past the top stops at the first line.
        app.scroll_help(i32::MIN);
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        assert!(rendered_text(terminal.backend()).contains("Navigation"));
    }

    #[test]
    fn render_draws_tracker_board_without_chart_toggle() {
        let mut app = AppState::new();
        app.select_board(4);
        app.set_status(
            Board::Tracker,
            Status::Loaded(Data::Tracker(vec![
                tracker_page("minicpm-ocr-1.0", "MiniCPM-OCR 1.0", "OpenBMB", "leak"),
                tracker_page("rerank-v5.0-fast", "rerank-v5.0-fast", "Cohere", "release"),
            ])),
        );
        app.toggle_view();

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(132, 30)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());

        assert_eq!(app.current_view(), View::Table);
        assert!(text.contains("AI Tracker (2)"));
        assert!(text.contains("MiniCPM-OCR 1.0"));
        assert!(text.contains("release"));
        assert!(text.contains("2026-10-22 15:43 UTC"));
        assert!(text.contains("sort: Last seen ↓"));
        assert!(!text.contains("view:"));
        assert!(!text.contains(" chart "));
    }

    #[test]
    fn error_view_keeps_status_and_source_visible() {
        let mut app = AppState::new();
        app.select_board(4);
        app.set_status(
            Board::Tracker,
            Status::Error(
                "HTTP 530 <unknown status code> from https://live.aitracker.bot/api/model-pages"
                    .to_string(),
            ),
        );

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());

        assert!(text.contains("HTTP 530"));
        assert!(text.contains("live.aitracker.bot/api/model-pages"));
        assert!(text.contains("live.aitracker.bot  |  load failed"));
        assert!(text.contains("press r to retry"));
        assert!(text.contains("failed to load AI Tracker"));

        app.set_status(Board::Tracker, Status::Loading);
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());
        assert!(text.contains("loading AI Tracker"));
        assert!(text.contains("fetching live.aitracker.bot"));
    }

    #[test]
    fn header_fit_uses_display_width_not_bytes() {
        let header_row = |query: &str, width: u16| {
            let mut app = AppState::new();
            app.set_status(
                Board::Aa,
                Status::Loaded(Data::Aa(vec![aa_model("m", "Model", "Provider")])),
            );
            app.begin_filter();
            for c in query.chars() {
                app.push_filter_char(c);
            }
            app.finish_filter();
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 24)).unwrap();
            terminal.draw(|frame| render(frame, &mut app)).unwrap();
            rendered_line(terminal.backend(), width as usize, 2)
        };
        // Same display width, different byte length.
        for width in 60..140 {
            let ascii = header_row("eeeeee", width);
            let accented = header_row("éééééé", width);
            assert_eq!(
                ascii.contains("sort:"),
                accented.contains("sort:"),
                "width {width}: {ascii:?} vs {accented:?}"
            );
            assert_eq!(ascii.contains("models"), accented.contains("models"));
        }
    }

    #[test]
    fn narrow_header_and_footer_drop_whole_items() {
        let mut app = AppState::new();
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![aa_model("m", "Model", "Provider")])),
        );
        let width = 64;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 14)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let backend = terminal.backend();

        let header = rendered_line(backend, width as usize, 2);
        assert!(
            !header.trim_start_matches('│').starts_with("  |"),
            "header starts with a separator: {header:?}"
        );
        assert!(header.contains("sort:"));

        for row in [12, 13] {
            let footer = rendered_line(backend, width as usize, row);
            let trimmed = footer.trim_end();
            assert!(
                trimmed.chars().count() < width as usize,
                "footer row {row} is clipped: {footer:?}"
            );
        }
        let footer = rendered_line(backend, width as usize, 12);
        assert!(footer.contains("help") && footer.contains("quit"));
        assert!(
            footer.contains("1/1"),
            "position indicator kept: {footer:?}"
        );
    }

    #[test]
    fn agents_table_hides_release_column_without_dates() {
        let render_header = |release_date: Option<&str>| {
            let mut app = AppState::new();
            app.select_board(1);
            let mut row = agent_row("claude-code", "Claude Code", "Anthropic");
            row.release_date = release_date.map(str::to_string);
            app.set_status(Board::AaAgents, Status::Loaded(Data::AaAgents(vec![row])));
            // Wide enough for the column, narrow enough to skip the detail pane.
            let width = 128;
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 24)).unwrap();
            terminal.draw(|frame| render(frame, &mut app)).unwrap();
            rendered_line(terminal.backend(), width as usize, 5)
        };
        assert!(render_header(Some("2026-01-01")).contains("Release"));
        assert!(!render_header(None).contains("Release"));
    }

    #[test]
    fn agents_view_fits_names_and_skips_opaque_ids() {
        let mut app = AppState::new();
        app.select_board(1);
        let mut row = agent_row(
            "a2c87c062f3cef73d8525e7578f14742",
            "Antigravity CLI",
            "Google",
        );
        row.release_date = None;
        app.set_status(Board::AaAgents, Status::Loaded(Data::AaAgents(vec![row])));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(140, 30)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());

        let table_row = rendered_line(terminal.backend(), 140, 6);
        assert!(table_row.contains("Antigravity CLI"), "{table_row:?}");
        assert!(!text.contains("a2c87c062f3c"), "hash id shown");
        assert!(!text.contains("Release"), "empty release shown");
    }

    #[test]
    fn mouse_clicks_sort_by_header_and_select_rows() {
        let mut app = AppState::new();
        let mut models = make_models(3);
        models[0].median_output_speed = Some(10.0);
        models[1].median_output_speed = Some(30.0);
        models[2].median_output_speed = Some(20.0);
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(models)));
        let width = 100u16;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        // Rows 0-3 are the top bar, 4 is the table border, 5 the header.
        let header = rendered_line(terminal.backend(), width as usize, 5);
        let byte = header.find("t/s").expect("speed column shown");
        let col = header[..byte].chars().count() as u16;
        assert_eq!(app.hit_test(col, 5), Click::Header('s'));
        assert_eq!(app.hit_test(col + 1, 5), Click::Header('s'));
        // The index column has no sort key.
        assert_eq!(app.hit_test(3, 5), Click::Nothing);

        // Data rows start at row 6.
        assert_eq!(app.hit_test(10, 6), Click::Row(0));
        assert_eq!(app.hit_test(10, 8), Click::Row(2));
        // Below the last row, and the tab bar, keep their own meaning.
        assert_eq!(app.hit_test(10, 9), Click::Nothing);
        assert_eq!(app.hit_test(3, 1), Click::Tab(0));

        app.cycle_sort('s');
        app.select_row(2);
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        assert_eq!(app.selected_idx(), 2);
        let selected = rendered_line(terminal.backend(), width as usize, 8);
        assert!(selected.contains(">"), "row 3 is highlighted: {selected:?}");
        // Sorted by speed, descending: 30, 20, 10.
        assert!(rendered_line(terminal.backend(), width as usize, 6).contains("Model 1"));
    }

    /// The table's right border column, from the first data row down.
    fn scrollbar_column(
        backend: &ratatui::backend::TestBackend,
        x: u16,
        rows: std::ops::Range<u16>,
    ) -> String {
        rows.map(|y| backend.buffer()[(x, y)].symbol().to_string())
            .collect()
    }

    #[test]
    fn long_tables_show_a_scrollbar_that_tracks_the_view() {
        let mut app = AppState::new();
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(make_models(60))));
        // 100x24: table panel spans rows 4..22, header at 5, data rows 6..21.
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let bar = scrollbar_column(terminal.backend(), 99, 6..21);
        assert!(bar.starts_with('█'), "thumb at the top: {bar:?}");
        assert!(!bar.ends_with('█'), "track below the thumb: {bar:?}");

        app.move_to_bottom();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let bar = scrollbar_column(terminal.backend(), 99, 6..21);
        assert!(bar.ends_with('█'), "thumb at the bottom: {bar:?}");
        assert!(!bar.starts_with('█'), "track above the thumb: {bar:?}");
    }

    #[test]
    fn short_tables_hide_the_scrollbar() {
        let mut app = AppState::new();
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(make_models(3))));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let bar = scrollbar_column(terminal.backend(), 99, 6..21);
        assert!(!bar.contains('█'), "no thumb when everything fits: {bar:?}");
    }

    #[test]
    fn clicking_the_scrollbar_jumps_to_that_part_of_the_table() {
        let mut app = AppState::new();
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(make_models(60))));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        // The track covers rows 6..=20 on the right border, column 99.
        assert_eq!(app.hit_test(99, 6), Click::Row(0));
        assert_eq!(app.hit_test(99, 20), Click::Row(59));
        assert!(matches!(app.hit_test(99, 13), Click::Row(i) if (25..35).contains(&i)));
        // The border above the track is not part of it.
        assert_eq!(app.hit_test(99, 5), Click::Nothing);
    }

    #[test]
    fn help_overlay_shows_a_scrollbar_when_it_overflows() {
        let mut app = AppState::new();
        app.toggle_help();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(132, 30)).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let text = rendered_text(terminal.backend());
        assert!(text.contains('█'), "help overlay has a thumb");
    }
}
