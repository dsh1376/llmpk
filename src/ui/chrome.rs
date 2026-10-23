use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};

use super::{
    aa_board, agents, filter_is_active, format_filter_label, frontier_board, panel,
    render_scrollbar, slop_board, truncate, AppState, View, SPINNER_FRAMES, VERSION,
};
use crate::board::{Board, Data, Status};

pub(super) fn render_help_overlay(frame: &mut Frame, area: Rect, scroll: &mut u16) {
    let dim = Style::default().fg(Color::DarkGray);
    let head = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::styled("Boards", head),
        help_row("AA", "artificialanalysis.ai composite ranking of LLMs"),
        help_row(
            "AA Agents",
            "artificialanalysis.ai coding-agent benchmark index",
        ),
        help_row(
            "Slopalytics",
            "slopalytics.com cost, time, and tokens per task by reasoning effort",
        ),
        help_row(
            "FrontierCode",
            "cognition.com mergeability benchmark for coding agents",
        ),
        help_row(
            "AI Tracker",
            "live.aitracker.bot feed of new model releases and leaks",
        ),
        Line::from(""),
        Line::styled("Navigation", head),
        help_row("q  Esc  Ctrl-C", "Quit llmpk"),
        help_row("?  h", "Toggle this help overlay (j/k, PgUp/PgDn scroll)"),
        help_row("[   ]", "Previous / next board (cycles)"),
        help_row("1 - 5", "Jump directly to board 1-5"),
        help_row("r", "Reload the current board (refetch from source)"),
        help_row("y", "Copy selected model name to clipboard (OSC 52)"),
        help_row("↑  ↓  k  j", "Move the highlighted row"),
        help_row("PgUp  PgDn", "Move by 10 rows"),
        help_row("Home  End  g  G", "Jump to first / last row"),
        help_row("mouse scroll", "Scroll the table (when supported)"),
        help_row(
            "mouse click",
            "Tab: switch board; header: sort; row: select; scrollbar: jump",
        ),
        Line::from(""),
        Line::styled("View & sort", head),
        help_row("m", "Toggle table / chart view (not on AI Tracker)"),
        help_row("o", "Reverse current sort direction (asc <-> desc)"),
        help_row("z", "Toggle compact mode (tighter column spacing)"),
        Line::styled("  AA sort keys", dim),
        help_row("i", "Intelligence Index — composite quality score"),
        help_row("s", "Output Speed — tokens generated per second"),
        help_row("p", "Blended Price — USD/1M, 7:2:1 cache-hit:input:output"),
        help_row("c", "Context Window — max tokens the model accepts"),
        help_row(
            "d",
            "Cache Discount — share of the input price saved on cache hits",
        ),
        Line::styled("  AA Agents sort keys", dim),
        help_row("i", "Index — Artificial Analysis Coding Agent Index"),
        help_row("a", "Pass@1 — mean benchmark reward"),
        help_row("p", "Cost — mean USD per task"),
        help_row("t", "Time — mean wall-clock task runtime"),
        help_row("u", "Tokens — mean total token usage per task"),
        help_row("s", "Turns — mean agent turns per task"),
        Line::styled("  Slopalytics sort keys", dim),
        help_row("i", "Intelligence — AA index (~ marks AA estimates)"),
        help_row("p", "Cost — mean USD per index task"),
        help_row("t", "Time — mean wall-clock time per index task"),
        help_row("u", "Tokens — mean output tokens per index task"),
        help_row("s", "Speed — output tokens per second"),
        Line::styled("  FrontierCode sort keys", dim),
        help_row("i", "Score — mergeability of the agent's PR (best effort)"),
        help_row("a", "Pass — share of tasks whose tests pass"),
        help_row("p", "Cost — USD per rollout"),
        help_row("u", "Tokens — output tokens per rollout"),
        help_row("t", "Time — minutes per rollout"),
        Line::styled("  AI Tracker sort keys", dim),
        help_row(
            "t",
            "Last seen — newest sighting first; stage is release or leak",
        ),
        Line::from(""),
        Line::styled("Filter", head),
        help_row(
            "/",
            "Begin editing a substring filter for the current board",
        ),
        help_row("type", "Narrows visible rows in real time"),
        help_row("Backspace", "Delete a character while editing"),
        help_row("Ctrl-U", "Clear the filter"),
        help_row("Enter  Esc", "Finish editing (filter stays applied)"),
        Line::from(""),
        Line::styled(
            format!("llmpk v{VERSION}"),
            Style::default().fg(Color::DarkGray),
        ),
        Line::styled("Press ?, h, or Esc to close", dim),
    ];

    // Lines are kept shorter than the popup, so one line is one row.
    let content_height = lines.len() as u16;
    let w = 82.min(area.width.saturating_sub(2));
    let h = content_height
        .saturating_add(2)
        .min(area.height.saturating_sub(2));
    let popup = centered_rect(area, w, h);
    frame.render_widget(ratatui::widgets::Clear, popup);

    let visible = h.saturating_sub(2);
    let max_scroll = content_height.saturating_sub(visible);
    *scroll = (*scroll).min(max_scroll);
    let title = if max_scroll > 0 {
        format!(
            " Keybindings & metrics ({}/{}, j/k scroll) ",
            *scroll + visible,
            content_height
        )
    } else {
        " Keybindings & metrics ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::Gray));
    let p = Paragraph::new(lines).block(block).scroll((*scroll, 0));
    frame.render_widget(p, popup);
    render_scrollbar(
        frame,
        popup.inner(ratatui::layout::Margin::new(0, 1)),
        usize::from(content_height),
        usize::from(visible),
        usize::from(*scroll),
    );
}

fn help_row(keys: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {keys:<16}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::raw(desc.to_string()),
    ])
}

pub(super) fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

pub(super) fn render_tiny(frame: &mut Frame) {
    let p = Paragraph::new("llmpk needs a larger terminal")
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("llmpk v{VERSION}")),
        );
    frame.render_widget(p, frame.area());
}

/// Top bar: one bordered block titled with the version, holding the tab row
/// and the board info line.
pub(super) fn render_top(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" llmpk v{VERSION} "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    render_tabs(frame, Rect { height: 1, ..inner }, app);
    if inner.height > 1 {
        let info = Rect {
            y: inner.y + 1,
            height: 1,
            ..inner
        };
        render_header(frame, info, app);
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &mut AppState) {
    // Full board names when they all fit, short ones otherwise.
    let titles_for = |label: fn(Board) -> &'static str| -> Vec<Line<'static>> {
        app.boards
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let key = Board::shortcut(i).unwrap_or(' ');
                Line::from(vec![
                    Span::styled(
                        format!("{key}"),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    status_marker(app.status.get(b)),
                    Span::raw(label(*b)),
                ])
            })
            .collect()
    };
    // Each title takes its width plus two padding cells and a divider cell.
    let row_width = |titles: &[Line]| -> usize { titles.iter().map(|t| t.width() + 3).sum() };
    let long = titles_for(Board::label);
    let titles = if row_width(&long) <= area.width as usize {
        long
    } else {
        titles_for(tab_label)
    };

    // Mirror the Tabs layout: each title gets one cell of padding on both
    // sides and a one-cell divider after it.
    let right = area.right();
    let mut x = area.x;
    let mut spans = Vec::with_capacity(titles.len());
    for title in &titles {
        let start = x;
        let end = start.saturating_add(title.width() as u16 + 2).min(right);
        if start >= end {
            break;
        }
        spans.push((start, end));
        x = end.saturating_add(1);
    }
    app.set_tab_spans(area, spans);

    let tabs = Tabs::new(titles)
        .select(app.current)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" ");
    frame.render_widget(tabs, area);
}

fn status_marker(status: Option<&Status>) -> Span<'static> {
    match status {
        Some(Status::Loading) => Span::styled(spinner_frame(), Style::default().fg(Color::Yellow)),
        Some(Status::Error(_)) => Span::styled("!", Style::default().fg(Color::Red)),
        _ => Span::raw(" "),
    }
}

fn tab_label(board: Board) -> &'static str {
    match board {
        Board::Aa => "AA",
        Board::AaAgents => "AAg",
        Board::Slop => "Slop",
        Board::Frontier => "FC",
        Board::Tracker => "AIT",
    }
}

pub(super) fn spinner_frame() -> &'static str {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let idx = (ms / 80) as usize % SPINNER_FRAMES.len();
    SPINNER_FRAMES[idx]
}

fn render_header(frame: &mut Frame, area: Rect, app: &AppState) {
    let board = app.current_board();
    let query = app.current_filter();
    let status_span = match app.status.get(&board) {
        Some(Status::Loaded(Data::Aa(v))) => {
            let visible = app.row_count(board);
            let label = if filter_is_active(query) {
                format!("{visible}/{} models", v.len())
            } else {
                format!("{} models", v.len())
            };
            Span::styled(label, Style::default().fg(Color::Green))
        }
        Some(Status::Loaded(Data::AaAgents(v))) => {
            let visible = app.row_count(board);
            let label = if filter_is_active(query) {
                format!("{visible}/{} agents", v.len())
            } else {
                format!("{} agents", v.len())
            };
            Span::styled(label, Style::default().fg(Color::Green))
        }
        Some(Status::Loaded(Data::Slop(v))) => {
            let visible = app.row_count(board);
            let label = if filter_is_active(query) {
                format!("{visible}/{} models", v.len())
            } else {
                format!("{} models", v.len())
            };
            Span::styled(label, Style::default().fg(Color::Green))
        }
        Some(Status::Loaded(Data::Frontier(v))) => {
            let visible = app.row_count(board);
            let label = if filter_is_active(query) {
                format!("{visible}/{} models", v.len())
            } else {
                format!("{} models", v.len())
            };
            Span::styled(label, Style::default().fg(Color::Green))
        }
        Some(Status::Loaded(Data::Tracker(v))) => {
            let visible = app.row_count(board);
            let label = if filter_is_active(query) {
                format!("{visible}/{} models", v.len())
            } else {
                format!("{} models", v.len())
            };
            Span::styled(label, Style::default().fg(Color::Green))
        }
        Some(Status::Loading) | None => Span::styled(
            format!("{} loading...", spinner_frame()),
            Style::default().fg(Color::Yellow),
        ),
        Some(Status::Error(_)) => Span::styled(
            "load failed".to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    };

    let sort_label = match board {
        Board::Aa => format!(
            "sort: {} {}",
            aa_board::aa_key_label(app.aa_sort.key),
            app.aa_sort.dir.arrow()
        ),
        Board::AaAgents => format!(
            "sort: {} {}",
            agents::agent_key_label(app.agent_sort.key),
            app.agent_sort.dir.arrow()
        ),
        Board::Slop => format!(
            "sort: {} {}",
            slop_board::slop_key_label(app.slop_sort.key),
            app.slop_sort.dir.arrow()
        ),
        Board::Frontier => format!(
            "sort: {} {}",
            frontier_board::frontier_key_label(app.frontier_sort.key),
            app.frontier_sort.dir.arrow()
        ),
        Board::Tracker => format!("sort: Last seen {}", app.tracker_sort.dir.arrow()),
    };
    let source = board_source(board);

    // Higher priority survives longer when the header is narrow.
    let mut items = vec![
        (
            0,
            vec![Span::styled(
                source,
                Style::default().add_modifier(Modifier::BOLD),
            )],
        ),
        (1, vec![status_span]),
    ];
    if board.has_chart() {
        items.push((
            2,
            vec![Span::styled(
                view_label(app.current_view()),
                Style::default().fg(Color::Blue),
            )],
        ));
    }
    items.push((
        3,
        vec![Span::styled(sort_label, Style::default().fg(Color::Cyan))],
    ));
    if app.is_filter_editing() || filter_is_active(query) {
        let label = format!(
            "filter: {}",
            format_filter_label(query, app.is_filter_editing())
        );
        items.push((
            4,
            vec![Span::styled(label, Style::default().fg(Color::Yellow))],
        ));
    }
    // Copy feedback — show "Copied!" for 2 seconds after clipboard copy.
    if app
        .copy_feedback_at
        .is_some_and(|t| t.elapsed().as_secs() < 2)
    {
        items.push((
            5,
            vec![Span::styled(
                "Copied!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )],
        ));
    }

    let line = fit_items(items, "  |  ", area.width as usize);
    frame.render_widget(Paragraph::new(line), area);
}

pub(super) fn render_filter_empty(frame: &mut Frame, area: Rect, query: &str) {
    let p = Paragraph::new(format!(
        "no rows match filter: {}\n\npress / to edit or Ctrl-U to clear",
        truncate(query, 48)
    ))
    .style(Style::default().fg(Color::Yellow))
    .block(panel("Leaderboard"))
    .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

/// Where a board's data comes from, as shown in the info line.
fn board_source(board: Board) -> &'static str {
    match board {
        Board::Aa => "artificialanalysis.ai/leaderboards/models",
        Board::AaAgents => "artificialanalysis.ai/agents/coding-agents",
        Board::Slop => "slopalytics.com",
        Board::Frontier => "cognition.com/frontiercode",
        Board::Tracker => "live.aitracker.bot",
    }
}

/// Host part of the source, short enough for the loading box.
fn board_host(board: Board) -> &'static str {
    let source = board_source(board);
    source.split('/').next().unwrap_or(source)
}

pub(super) fn render_loading(frame: &mut Frame, area: Rect, board: Board) {
    let spinner = spinner_frame();
    let inner = centered_rect(area, 44, 7);
    frame.render_widget(ratatui::widgets::Clear, inner);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                spinner,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("loading {}", board.label()),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::styled(
            format!("  fetching {}", board_host(board)),
            Style::default().fg(Color::DarkGray),
        ),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " llmpk ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, inner);
}

pub(super) fn render_error(frame: &mut Frame, area: Rect, board: Board, error: &str) {
    let inner = centered_rect(area, 52, 11);
    frame.render_widget(ratatui::widgets::Clear, inner);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "\u{2717}",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("failed to load {}", board.label()),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::styled(format!("  {error}"), Style::default().fg(Color::DarkGray)),
        Line::from(""),
        Line::from(vec![
            Span::raw("  press "),
            Span::styled(
                "r",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to retry"),
        ]),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(
            " Error ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(p, inner);
}

pub(super) fn render_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    // Leave the last column free so a full line never looks clipped.
    let width = area.width.saturating_sub(1) as usize;
    let style = Style::default().fg(Color::Gray);

    if app.is_filter_editing() {
        let items = vec![
            (9, hint("type", " narrow filter")),
            (6, hint("Backspace", " delete char")),
            (7, hint("Ctrl-U", " clear filter")),
            (8, hint("Enter/Esc", " done")),
        ];
        let p = Paragraph::new(fit_items(items, "  ", width)).style(style);
        frame.render_widget(p, area);
        return;
    }

    let board = app.current_board();
    let total = app.row_count(board);
    let selected = app.selected_idx();
    let unfiltered = match app.status.get(&board) {
        Some(Status::Loaded(Data::Aa(v))) => v.len(),
        Some(Status::Loaded(Data::AaAgents(v))) => v.len(),
        Some(Status::Loaded(Data::Slop(v))) => v.len(),
        Some(Status::Loaded(Data::Frontier(v))) => v.len(),
        Some(Status::Loaded(Data::Tracker(v))) => v.len(),
        _ => 0,
    };

    let mut nav = vec![
        (9, hint("? / h", " help")),
        (9, hint("q", " quit")),
        (6, hint("[ ]", " board")),
        (5, hint("j/k", " row")),
        (2, hint("g/G", " top/bot")),
        (4, hint("r", " reload")),
        (3, hint("y", " copy")),
    ];
    if total > 0 && app.current_view() == View::Table {
        nav.push((
            8,
            vec![Span::styled(
                format!("{}/{}", selected + 1, total),
                Style::default().fg(Color::Cyan),
            )],
        ));
    }
    if filter_is_active(app.current_filter()) && total != unfiltered {
        nav.push((
            7,
            vec![Span::styled(
                format!("showing {total}/{unfiltered}"),
                Style::default().fg(Color::Yellow),
            )],
        ));
    }

    let sort_keys = match board {
        Board::Aa => "i/s/p/d/c",
        Board::AaAgents => "i/a/p/t/u/s",
        Board::Slop => "i/p/t/u/s",
        Board::Frontier => "i/a/p/u/t",
        Board::Tracker => "t",
    };
    let mut actions = vec![(9, hint(sort_keys, " sort")), (6, hint("o", " asc/desc"))];
    if board.has_chart() {
        let view_action = match app.current_view() {
            View::Table => " chart",
            View::Chart => " table",
        };
        actions.push((7, hint("m", view_action)));
    }
    actions.push((8, hint("/", " filter")));
    if filter_is_active(app.current_filter()) {
        actions.push((5, hint("Ctrl-U", " clear filter")));
    }

    let p = Paragraph::new(vec![
        fit_items(nav, "  ", width),
        fit_items(actions, "  ", width),
    ])
    .style(style);
    frame.render_widget(p, area);
}

/// Keep the highest-priority items that fit in `width`, in their original
/// order, joined by `sep`. The lowest priority drops first; ties drop the
/// rightmost item. Widths are display widths, not byte lengths.
fn fit_items<'a>(mut items: Vec<(u8, Vec<Span<'a>>)>, sep: &'a str, width: usize) -> Line<'a> {
    let sep_width = Span::raw(sep).width();
    let line_width = |items: &[(u8, Vec<Span>)]| -> usize {
        let content: usize = items
            .iter()
            .flat_map(|(_, spans)| spans.iter().map(Span::width))
            .sum();
        content + sep_width * items.len().saturating_sub(1)
    };
    while items.len() > 1 && line_width(&items) > width {
        let drop = items
            .iter()
            .enumerate()
            .min_by_key(|(i, (priority, _))| (*priority, std::cmp::Reverse(*i)))
            .map(|(i, _)| i)
            .unwrap_or(0);
        items.remove(drop);
    }

    let mut spans = Vec::new();
    for (i, (_, item)) in items.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(sep));
        }
        spans.extend(item);
    }
    Line::from(spans)
}

fn hint<'a>(keys: &'a str, action: &'a str) -> Vec<Span<'a>> {
    vec![key(keys), text(action)]
}

fn key(s: &str) -> Span<'_> {
    Span::styled(
        s,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
}

fn text(s: &str) -> Span<'_> {
    Span::raw(s)
}

fn view_label(view: View) -> &'static str {
    match view {
        View::Table => "view: table",
        View::Chart => "view: chart",
    }
}
