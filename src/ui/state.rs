use std::collections::HashMap;
use std::time::Instant;

use ratatui::layout::Rect;
use ratatui::widgets::TableState;

use crate::board::{Board, Data, Status};

use super::filter::*;
use super::sort::*;

/// What a left click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    Tab(usize),
    /// A column header; the char is that column's sort key.
    Header(char),
    /// A data row, as an index into the visible (filtered) rows.
    Row(usize),
    Nothing,
}

/// Where the last rendered table put its header and rows, for mouse
/// hit-testing.
#[derive(Debug, Clone, Default)]
pub struct TableHit {
    pub header_y: u16,
    pub rows: Rect,
    /// Index of the first visible row.
    pub offset: usize,
    /// Each column's `(x, width, sort key)`.
    pub columns: Vec<(u16, u16, Option<char>)>,
    /// The scrollbar's track, when the table overflows.
    pub scroll_track: Option<Rect>,
    /// Visible (filtered) row count.
    pub total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Table,
    Chart,
}

pub struct AppState {
    pub boards: &'static [Board],
    pub current: usize,
    pub status: HashMap<Board, Status>,
    pub table_state: HashMap<Board, TableState>,
    pub views: HashMap<Board, View>,
    pub filters: HashMap<Board, String>,
    pub filter_editing: bool,
    pub help_open: bool,
    /// First visible line of the help overlay; clamped when rendered.
    pub help_scroll: u16,
    pub compact: bool,
    pub aa_sort: AaSort,
    pub agent_sort: AgentSort,
    pub slop_sort: SlopSort,
    pub frontier_sort: FrontierSort,
    pub tracker_sort: TrackerSort,
    pub hot_sort: DirSort,
    pub daily_sort: DirSort,
    pub reset_sort: DirSort,
    filter_caches: HashMap<Board, FilterCache>,
    pub copy_feedback_at: Option<Instant>,
    /// Tab bar area and each tab's `[start, end)` columns from the last
    /// render, for mouse hit-testing.
    tab_bar: Rect,
    tab_spans: Vec<(u16, u16)>,
    table_hit: Option<TableHit>,
}

/// Map a cell on the scrollbar track to the row at that share of the table.
fn track_row(track: Rect, row: u16, total: usize) -> usize {
    let last = total.saturating_sub(1);
    if track.height <= 1 {
        return 0;
    }
    let share = f64::from(row - track.y) / f64::from(track.height - 1);
    ((share * last as f64).round() as usize).min(last)
}

/// Codex Resets opens on its calendar; every other board on its table.
fn default_view(board: Board) -> View {
    if board == Board::Resets {
        View::Chart
    } else {
        View::Table
    }
}

impl AppState {
    pub fn new() -> Self {
        let boards = Board::all();
        Self {
            boards,
            current: 0,
            status: HashMap::new(),
            table_state: HashMap::new(),
            views: HashMap::new(),
            filters: HashMap::new(),
            filter_editing: false,
            help_open: false,
            help_scroll: 0,
            compact: false,
            aa_sort: AaSort {
                key: AaKey::Intelligence,
                dir: SortDir::Desc,
            },
            agent_sort: AgentSort {
                key: AgentKey::Index,
                dir: SortDir::Desc,
            },
            slop_sort: SlopSort {
                key: SlopKey::Intelligence,
                dir: SortDir::Desc,
            },
            frontier_sort: FrontierSort {
                key: FrontierKey::Score,
                dir: SortDir::Desc,
            },
            tracker_sort: TrackerSort { dir: SortDir::Desc },
            hot_sort: DirSort { dir: SortDir::Asc },
            daily_sort: DirSort { dir: SortDir::Asc },
            reset_sort: DirSort { dir: SortDir::Desc },
            filter_caches: HashMap::new(),
            copy_feedback_at: None,
            tab_bar: Rect::default(),
            tab_spans: Vec::new(),
            table_hit: None,
        }
    }

    pub fn current_view(&self) -> View {
        let board = self.current_board();
        self.views
            .get(&board)
            .copied()
            .unwrap_or_else(|| default_view(board))
    }

    pub fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
        self.help_scroll = 0;
    }

    pub fn scroll_help(&mut self, delta: i32) {
        let next = i64::from(self.help_scroll) + i64::from(delta);
        self.help_scroll = next.clamp(0, i64::from(u16::MAX)) as u16;
    }

    pub(super) fn set_tab_spans(&mut self, bar: Rect, spans: Vec<(u16, u16)>) {
        self.tab_bar = bar;
        self.tab_spans = spans;
    }

    pub(super) fn set_table_hit(&mut self, hit: Option<TableHit>) {
        self.table_hit = hit;
    }

    /// Map a click to the tab, column header, or table row drawn there.
    pub fn hit_test(&self, column: u16, row: u16) -> Click {
        if let Some(idx) = self.tab_at(column, row) {
            return Click::Tab(idx);
        }
        let Some(hit) = &self.table_hit else {
            return Click::Nothing;
        };
        if row == hit.header_y {
            return hit
                .columns
                .iter()
                .find(|&&(x, width, _)| column >= x && column < x.saturating_add(width))
                .and_then(|&(_, _, key)| key)
                .map_or(Click::Nothing, Click::Header);
        }
        if let Some(track) = hit.scroll_track {
            if column == track.x && row >= track.y && row < track.bottom() {
                return Click::Row(track_row(track, row, hit.total));
            }
        }
        let rows = hit.rows;
        let in_rows =
            column >= rows.x && column < rows.right() && row >= rows.y && row < rows.bottom();
        if in_rows {
            let idx = hit.offset + usize::from(row - rows.y);
            if idx < self.row_count(self.current_board()) {
                return Click::Row(idx);
            }
        }
        Click::Nothing
    }

    pub fn select_row(&mut self, idx: usize) {
        let board = self.current_board();
        if idx < self.row_count(board) {
            self.table_state.entry(board).or_default().select(Some(idx));
        }
    }

    /// Index of the board whose tab was drawn at this cell, if any.
    pub fn tab_at(&self, column: u16, row: u16) -> Option<usize> {
        let bar = self.tab_bar;
        if row < bar.y || row >= bar.y.saturating_add(bar.height) {
            return None;
        }
        self.tab_spans
            .iter()
            .position(|&(start, end)| (start..end).contains(&column))
    }

    pub fn close_help(&mut self) {
        self.help_open = false;
    }

    pub fn is_help_open(&self) -> bool {
        self.help_open
    }

    pub fn toggle_view(&mut self) {
        let board = self.current_board();
        if !board.has_chart() {
            return;
        }
        let next = match self.current_view() {
            View::Table => View::Chart,
            View::Chart => View::Table,
        };
        if next == default_view(board) {
            self.views.remove(&board);
        } else {
            self.views.insert(board, next);
        }
    }

    pub fn current_board(&self) -> Board {
        self.boards[self.current]
    }

    pub fn set_status(&mut self, board: Board, status: Status) {
        match status {
            Status::Loaded(mut data) => {
                self.sort_data(&mut data);
                self.status.insert(board, Status::Loaded(data));
                self.reset_selection(board);
            }
            other => {
                self.status.insert(board, other);
            }
        }
        self.filter_caches.remove(&board);
    }

    pub fn select_board(&mut self, idx: usize) {
        if idx >= self.boards.len() {
            return;
        }
        self.current = idx;
        self.clamp_selection(self.current_board());
    }

    pub fn cycle_board(&mut self, delta: i32) {
        let n = self.boards.len() as i32;
        let next = ((self.current as i32 + delta).rem_euclid(n)) as usize;
        self.current = next;
        self.clamp_selection(self.current_board());
    }

    pub fn move_down(&mut self) {
        let board = self.current_board();
        let len = self.row_count(board);
        if len == 0 {
            return;
        }
        let st = self.table_state.entry(board).or_default();
        let i = st.selected().map(|i| i + 1).unwrap_or(0).min(len - 1);
        st.select(Some(i));
    }

    pub fn move_up(&mut self) {
        let board = self.current_board();
        let st = self.table_state.entry(board).or_default();
        let i = st.selected().unwrap_or(0).saturating_sub(1);
        st.select(Some(i));
    }

    pub fn move_page_down(&mut self) {
        let board = self.current_board();
        let len = self.row_count(board);
        if len == 0 {
            return;
        }
        let page = 10;
        let st = self.table_state.entry(board).or_default();
        let i = st.selected().map(|i| i + page).unwrap_or(0).min(len - 1);
        st.select(Some(i));
    }

    pub fn move_page_up(&mut self) {
        let board = self.current_board();
        let st = self.table_state.entry(board).or_default();
        let i = st.selected().unwrap_or(0).saturating_sub(10);
        st.select(Some(i));
    }

    pub fn move_to_top(&mut self) {
        let board = self.current_board();
        if self.row_count(board) > 0 {
            let st = self.table_state.entry(board).or_default();
            st.select(Some(0));
        }
    }

    pub fn move_to_bottom(&mut self) {
        let board = self.current_board();
        let len = self.row_count(board);
        if len > 0 {
            let st = self.table_state.entry(board).or_default();
            st.select(Some(len - 1));
        }
    }

    pub fn row_count(&self, board: Board) -> usize {
        let query = self.filter_query(board);
        match self.status.get(&board) {
            Some(Status::Loaded(Data::Aa(v))) => count_matching_aa(v, query),
            Some(Status::Loaded(Data::AaAgents(v))) => count_matching_agents(v, query),
            Some(Status::Loaded(Data::Slop(v))) => count_matching_slop(v, query),
            Some(Status::Loaded(Data::Frontier(v))) => count_matching_frontier(v, query),
            Some(Status::Loaded(Data::Tracker(v))) => count_matching_tracker(v, query),
            Some(Status::Loaded(Data::Hot(v))) => count_matching(v, query, hot_matches_filter),
            Some(Status::Loaded(Data::Daily(v))) => count_matching(v, query, daily_matches_filter),
            Some(Status::Loaded(Data::Resets(v))) => count_matching(v, query, reset_matches_filter),
            _ => 0,
        }
    }

    pub fn selected_idx(&self) -> usize {
        self.table_state
            .get(&self.current_board())
            .and_then(TableState::selected)
            .unwrap_or(0)
    }

    pub fn begin_filter(&mut self) {
        self.filter_editing = true;
        let board = self.current_board();
        self.filters.entry(board).or_default();
    }

    pub fn finish_filter(&mut self) {
        self.filter_editing = false;
        self.drop_empty_current_filter();
    }

    pub fn is_filter_editing(&self) -> bool {
        self.filter_editing
    }

    pub fn filter_query(&self, board: Board) -> &str {
        self.filters.get(&board).map(String::as_str).unwrap_or("")
    }

    pub fn current_filter(&self) -> &str {
        self.filter_query(self.current_board())
    }

    pub fn push_filter_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        let board = self.current_board();
        self.filters.entry(board).or_default().push(c);
        self.filter_caches.remove(&board);
        self.reset_selection(board);
    }

    pub fn pop_filter_char(&mut self) {
        let board = self.current_board();
        if let Some(query) = self.filters.get_mut(&board) {
            query.pop();
        }
        self.drop_empty_filter(board);
        self.filter_caches.remove(&board);
        self.reset_selection(board);
    }

    pub fn clear_current_filter(&mut self) {
        let board = self.current_board();
        self.filters.remove(&board);
        self.filter_caches.remove(&board);
        self.reset_selection(board);
    }

    fn drop_empty_current_filter(&mut self) {
        let board = self.current_board();
        self.drop_empty_filter(board);
    }

    fn drop_empty_filter(&mut self, board: Board) {
        if self
            .filters
            .get(&board)
            .is_some_and(|query| query.is_empty())
        {
            self.filters.remove(&board);
        }
    }

    fn reset_selection(&mut self, board: Board) {
        let len = self.row_count(board);
        let st = self.table_state.entry(board).or_default();
        st.select(if len > 0 { Some(0) } else { None });
    }

    fn clamp_selection(&mut self, board: Board) {
        let len = self.row_count(board);
        let st = self.table_state.entry(board).or_default();
        if len == 0 {
            st.select(None);
            return;
        }
        let idx = st.selected().unwrap_or(0).min(len - 1);
        st.select(Some(idx));
    }

    pub fn cycle_sort(&mut self, key: char) {
        let board = self.current_board();
        match board {
            Board::Aa => {
                let new_key = match key {
                    'i' => Some(AaKey::Intelligence),
                    's' => Some(AaKey::Speed),
                    'p' => Some(AaKey::Price),
                    'c' => Some(AaKey::Context),
                    'd' => Some(AaKey::Cache),
                    _ => None,
                };
                if let Some(k) = new_key {
                    if self.aa_sort.key == k {
                        self.aa_sort.dir = self.aa_sort.dir.toggle();
                    } else {
                        self.aa_sort.key = k;
                        self.aa_sort.dir = if matches!(k, AaKey::Price) {
                            SortDir::Asc
                        } else {
                            SortDir::Desc
                        };
                    }
                }
            }
            Board::AaAgents => {
                let new_key = match key {
                    'i' => Some(AgentKey::Index),
                    'a' => Some(AgentKey::Pass),
                    'p' => Some(AgentKey::Cost),
                    't' => Some(AgentKey::Time),
                    'u' => Some(AgentKey::Tokens),
                    's' => Some(AgentKey::Turns),
                    _ => None,
                };
                if let Some(k) = new_key {
                    if self.agent_sort.key == k {
                        self.agent_sort.dir = self.agent_sort.dir.toggle();
                    } else {
                        self.agent_sort.key = k;
                        self.agent_sort.dir = if matches!(
                            k,
                            AgentKey::Cost | AgentKey::Time | AgentKey::Tokens | AgentKey::Turns
                        ) {
                            SortDir::Asc
                        } else {
                            SortDir::Desc
                        };
                    }
                }
            }
            Board::Slop => {
                let new_key = match key {
                    'i' => Some(SlopKey::Intelligence),
                    'p' => Some(SlopKey::Cost),
                    't' => Some(SlopKey::Time),
                    'u' => Some(SlopKey::Tokens),
                    's' => Some(SlopKey::Speed),
                    _ => None,
                };
                if let Some(k) = new_key {
                    if self.slop_sort.key == k {
                        self.slop_sort.dir = self.slop_sort.dir.toggle();
                    } else {
                        self.slop_sort.key = k;
                        self.slop_sort.dir =
                            if matches!(k, SlopKey::Cost | SlopKey::Time | SlopKey::Tokens) {
                                SortDir::Asc
                            } else {
                                SortDir::Desc
                            };
                    }
                }
            }
            Board::Frontier => {
                let new_key = match key {
                    'i' => Some(FrontierKey::Score),
                    'a' => Some(FrontierKey::Pass),
                    'p' => Some(FrontierKey::Cost),
                    'u' => Some(FrontierKey::Tokens),
                    't' => Some(FrontierKey::Time),
                    _ => None,
                };
                if let Some(k) = new_key {
                    if self.frontier_sort.key == k {
                        self.frontier_sort.dir = self.frontier_sort.dir.toggle();
                    } else {
                        self.frontier_sort.key = k;
                        self.frontier_sort.dir = if matches!(
                            k,
                            FrontierKey::Cost | FrontierKey::Tokens | FrontierKey::Time
                        ) {
                            SortDir::Asc
                        } else {
                            SortDir::Desc
                        };
                    }
                }
            }
            Board::Tracker => {
                if key == 't' {
                    self.tracker_sort.dir = self.tracker_sort.dir.toggle();
                }
            }
            Board::Hot => {
                if key == 'i' {
                    self.hot_sort.dir = self.hot_sort.dir.toggle();
                }
            }
            Board::Daily => {
                if key == 'i' {
                    self.daily_sort.dir = self.daily_sort.dir.toggle();
                }
            }
            Board::Resets => {
                if key == 't' {
                    self.reset_sort.dir = self.reset_sort.dir.toggle();
                }
            }
        }
        self.resort_current();
    }

    pub fn selected_name(&mut self) -> Option<String> {
        let board = self.current_board();
        let idx = self.selected_idx();
        let query = self.filter_query(board).to_string();
        let indices = self.filter_cache(board, &query);
        let data_idx = *indices.get(idx)?;
        match self.status.get(&board) {
            Some(Status::Loaded(Data::Aa(models))) => models.get(data_idx).map(|m| m.name.clone()),
            Some(Status::Loaded(Data::AaAgents(rows))) => rows.get(data_idx).map(|r| r.label()),
            Some(Status::Loaded(Data::Slop(rows))) => rows.get(data_idx).map(|r| r.display_name()),
            Some(Status::Loaded(Data::Frontier(rows))) => {
                rows.get(data_idx).map(|r| r.display_name())
            }
            Some(Status::Loaded(Data::Tracker(pages))) => {
                pages.get(data_idx).map(|p| p.model.clone())
            }
            Some(Status::Loaded(Data::Hot(topics))) => {
                topics.get(data_idx).map(|t| t.title.clone())
            }
            Some(Status::Loaded(Data::Daily(items))) => {
                items.get(data_idx).map(|i| i.title.clone())
            }
            Some(Status::Loaded(Data::Resets(events))) => events
                .get(data_idx)
                .map(|e| e.post_url().unwrap_or(&e.id).to_string()),
            _ => None,
        }
    }

    pub fn toggle_dir(&mut self) {
        match self.current_board() {
            Board::Aa => self.aa_sort.dir = self.aa_sort.dir.toggle(),
            Board::AaAgents => self.agent_sort.dir = self.agent_sort.dir.toggle(),
            Board::Slop => self.slop_sort.dir = self.slop_sort.dir.toggle(),
            Board::Frontier => self.frontier_sort.dir = self.frontier_sort.dir.toggle(),
            Board::Tracker => self.tracker_sort.dir = self.tracker_sort.dir.toggle(),
            Board::Hot => self.hot_sort.dir = self.hot_sort.dir.toggle(),
            Board::Daily => self.daily_sort.dir = self.daily_sort.dir.toggle(),
            Board::Resets => self.reset_sort.dir = self.reset_sort.dir.toggle(),
        }
        self.resort_current();
    }

    fn resort_current(&mut self) {
        let board = self.current_board();
        if let Some(Status::Loaded(data)) = self.status.get_mut(&board) {
            sort_with(
                data,
                &self.aa_sort,
                &self.agent_sort,
                &self.slop_sort,
                &self.frontier_sort,
                &self.tracker_sort,
                &self.hot_sort,
                &self.daily_sort,
                &self.reset_sort,
            );
        }
        self.filter_caches.remove(&board);
        let has_rows = self.row_count(board) > 0;
        if let Some(st) = self.table_state.get_mut(&board) {
            st.select(if has_rows { Some(0) } else { None });
        }
    }

    fn sort_data(&self, data: &mut Data) {
        sort_with(
            data,
            &self.aa_sort,
            &self.agent_sort,
            &self.slop_sort,
            &self.frontier_sort,
            &self.tracker_sort,
            &self.hot_sort,
            &self.daily_sort,
            &self.reset_sort,
        );
    }

    /// Return cached filter indices for the given board. Rebuilds if query changed.
    pub fn filter_cache(&mut self, board: Board, query: &str) -> &[usize] {
        let needs_rebuild = self
            .filter_caches
            .get(&board)
            .is_none_or(|c| c.query != query);

        if needs_rebuild {
            let indices = match self.status.get(&board) {
                Some(Status::Loaded(Data::Aa(models))) => {
                    let tokens = filter_tokens(query);
                    if tokens.is_empty() {
                        (0..models.len()).collect()
                    } else {
                        models
                            .iter()
                            .enumerate()
                            .filter(|(_, m)| aa_matches_filter(m, &tokens))
                            .map(|(i, _)| i)
                            .collect()
                    }
                }
                Some(Status::Loaded(Data::AaAgents(rows))) => {
                    let tokens = filter_tokens(query);
                    if tokens.is_empty() {
                        (0..rows.len()).collect()
                    } else {
                        rows.iter()
                            .enumerate()
                            .filter(|(_, r)| agent_matches_filter(r, &tokens))
                            .map(|(i, _)| i)
                            .collect()
                    }
                }
                Some(Status::Loaded(Data::Slop(rows))) => {
                    let tokens = filter_tokens(query);
                    if tokens.is_empty() {
                        (0..rows.len()).collect()
                    } else {
                        rows.iter()
                            .enumerate()
                            .filter(|(_, r)| slop_matches_filter(r, &tokens))
                            .map(|(i, _)| i)
                            .collect()
                    }
                }
                Some(Status::Loaded(Data::Frontier(rows))) => {
                    let tokens = filter_tokens(query);
                    if tokens.is_empty() {
                        (0..rows.len()).collect()
                    } else {
                        rows.iter()
                            .enumerate()
                            .filter(|(_, r)| frontier_matches_filter(r, &tokens))
                            .map(|(i, _)| i)
                            .collect()
                    }
                }
                Some(Status::Loaded(Data::Tracker(pages))) => {
                    let tokens = filter_tokens(query);
                    if tokens.is_empty() {
                        (0..pages.len()).collect()
                    } else {
                        pages
                            .iter()
                            .enumerate()
                            .filter(|(_, p)| tracker_matches_filter(p, &tokens))
                            .map(|(i, _)| i)
                            .collect()
                    }
                }
                Some(Status::Loaded(Data::Hot(topics))) => {
                    matching_indices(topics, query, hot_matches_filter)
                }
                Some(Status::Loaded(Data::Daily(items))) => {
                    matching_indices(items, query, daily_matches_filter)
                }
                Some(Status::Loaded(Data::Resets(events))) => {
                    matching_indices(events, query, reset_matches_filter)
                }
                _ => Vec::new(),
            };
            self.filter_caches.insert(
                board,
                FilterCache {
                    indices,
                    query: query.to_string(),
                },
            );
        }

        &self.filter_caches[&board].indices
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;

    #[test]
    fn current_board_filter_counts_aa_rows_and_can_clear() {
        let mut app = AppState::new();
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![
                aa_model("claude-sonnet", "Claude Sonnet", "Anthropic"),
                aa_model("gpt-5", "GPT-5", "OpenAI"),
            ])),
        );

        assert_eq!(app.row_count(Board::Aa), 2);

        app.begin_filter();
        for c in "anthropic".chars() {
            app.push_filter_char(c);
        }

        assert_eq!(app.row_count(Board::Aa), 1);
        assert_eq!(
            app.table_state
                .get(&Board::Aa)
                .and_then(|state| state.selected()),
            Some(0)
        );

        app.clear_current_filter();

        assert_eq!(app.row_count(Board::Aa), 2);
    }

    #[test]
    fn filters_are_scoped_to_the_current_board() {
        let mut app = AppState::new();
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![aa_model(
                "claude-sonnet",
                "Claude Sonnet",
                "Anthropic",
            )])),
        );
        app.set_status(
            Board::AaAgents,
            Status::Loaded(Data::AaAgents(vec![
                agent_row("claude-code", "Claude Code", "Anthropic"),
                agent_row("codex", "Codex", "OpenAI"),
            ])),
        );

        app.select_board(1);
        app.begin_filter();
        for c in "openai".chars() {
            app.push_filter_char(c);
        }

        assert_eq!(app.row_count(Board::AaAgents), 1);
        assert_eq!(app.row_count(Board::Aa), 1);
        assert_eq!(app.filter_query(Board::Aa), "");
        assert_eq!(app.filter_query(Board::AaAgents), "openai");
    }

    #[test]
    fn chart_view_is_scoped_to_the_current_board() {
        let mut app = AppState::new();

        assert_eq!(app.current_view(), View::Table);
        app.toggle_view();
        assert_eq!(app.current_view(), View::Chart);

        app.select_board(1);
        assert_eq!(app.current_board(), Board::AaAgents);
        assert_eq!(app.current_view(), View::Table);

        app.toggle_view();
        assert_eq!(app.current_view(), View::Chart);

        app.select_board(0);
        assert_eq!(app.current_view(), View::Chart);
    }

    #[test]
    fn page_down_moves_by_ten_rows() {
        let mut app = AppState::new();
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(make_models(30))));
        assert_eq!(app.selected_idx(), 0);

        app.move_page_down();
        assert_eq!(app.selected_idx(), 10);

        app.move_page_down();
        assert_eq!(app.selected_idx(), 20);

        app.move_page_down();
        assert_eq!(app.selected_idx(), 29);
    }

    #[test]
    fn page_up_stops_at_zero() {
        let mut app = AppState::new();
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(make_models(30))));
        app.move_to_bottom();
        assert_eq!(app.selected_idx(), 29);

        app.move_page_up();
        assert_eq!(app.selected_idx(), 19);

        app.move_page_up();
        assert_eq!(app.selected_idx(), 9);

        app.move_page_up();
        assert_eq!(app.selected_idx(), 0);

        app.move_page_up();
        assert_eq!(app.selected_idx(), 0);
    }

    #[test]
    fn move_to_top_and_bottom() {
        let mut app = AppState::new();
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(make_models(20))));

        app.move_to_bottom();
        assert_eq!(app.selected_idx(), 19);

        app.move_to_top();
        assert_eq!(app.selected_idx(), 0);
    }

    #[test]
    fn move_to_bottom_on_empty_list() {
        let mut app = AppState::new();
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(vec![])));

        app.move_to_bottom();
        assert_eq!(
            app.table_state.get(&Board::Aa).and_then(|s| s.selected()),
            None
        );
    }

    #[test]
    fn filter_cache_returns_all_indices_when_no_filter() {
        let mut app = AppState::new();
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(make_models(5))));
        let indices = app.filter_cache(Board::Aa, "");
        assert_eq!(indices, &[0, 1, 2, 3, 4]);
    }

    #[test]
    fn filter_cache_returns_matching_indices() {
        let mut app = AppState::new();
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![
                aa_model("claude-sonnet", "Claude Sonnet", "Anthropic"),
                aa_model("gpt-5", "GPT-5", "OpenAI"),
                aa_model("claude-opus", "Claude Opus", "Anthropic"),
            ])),
        );
        let indices = app.filter_cache(Board::Aa, "anthropic");
        assert_eq!(indices, &[0, 2]);
    }

    #[test]
    fn filter_cache_invalidates_on_query_change() {
        let mut app = AppState::new();
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![
                aa_model("claude-sonnet", "Claude Sonnet", "Anthropic"),
                aa_model("gpt-5", "GPT-5", "OpenAI"),
            ])),
        );
        let indices = app.filter_cache(Board::Aa, "openai");
        assert_eq!(indices, &[1]);
        let indices = app.filter_cache(Board::Aa, "anthropic");
        assert_eq!(indices, &[0]);
    }

    #[test]
    fn sort_invalidates_filter_cache() {
        let mut app = AppState::new();
        let mut m1 = aa_model("claude", "Claude", "Anthropic");
        m1.intelligence_index = Some(60.0);
        m1.price_1m_output_tokens = Some(40.0);
        let mut m2 = aa_model("gpt", "GPT", "OpenAI");
        m2.intelligence_index = Some(50.0);
        m2.price_1m_output_tokens = Some(3.0);
        app.set_status(Board::Aa, Status::Loaded(Data::Aa(vec![m1, m2])));

        // Initially sorted by intelligence desc: [claude(60), gpt(50)]
        let indices = app.filter_cache(Board::Aa, "").to_vec();
        assert_eq!(indices, &[0, 1]);

        // Switch sort to price (ascending) — data re-sorted to [gpt, claude]
        app.cycle_sort('p');

        // Cache should be invalidated; filter should see the new order
        let indices = app.filter_cache(Board::Aa, "").to_vec();
        if let Some(Status::Loaded(Data::Aa(models))) = app.status.get(&Board::Aa) {
            assert_eq!(
                models[indices[0]].name, "GPT",
                "gpt should be first (cheaper)"
            );
            assert_eq!(models[indices[1]].name, "Claude", "claude should be second");
        }

        // Verify filter still works correctly after sort
        let indices = app.filter_cache(Board::Aa, "claude").to_vec();
        assert_eq!(indices.len(), 1);
        if let Some(Status::Loaded(Data::Aa(models))) = app.status.get(&Board::Aa) {
            assert_eq!(models[indices[0]].name, "Claude");
        }
    }
}
