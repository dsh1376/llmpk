use std::cmp::Ordering;

use crate::aa;
use crate::aihot;
use crate::aitracker;
use crate::coding_agents;
use crate::frontiercode;
use crate::slopalytics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Desc,
    Asc,
}

impl SortDir {
    pub fn arrow(self) -> &'static str {
        match self {
            SortDir::Desc => "↓",
            SortDir::Asc => "↑",
        }
    }
    pub fn toggle(self) -> Self {
        match self {
            SortDir::Desc => SortDir::Asc,
            SortDir::Asc => SortDir::Desc,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AaKey {
    Intelligence,
    Speed,
    Price,
    Context,
    Cache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKey {
    Index,
    Pass,
    Cost,
    Time,
    Tokens,
    Turns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlopKey {
    Intelligence,
    Cost,
    Time,
    Tokens,
    Speed,
}

#[derive(Debug, Clone)]
pub struct AaSort {
    pub key: AaKey,
    pub dir: SortDir,
}

#[derive(Debug, Clone)]
pub struct AgentSort {
    pub key: AgentKey,
    pub dir: SortDir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontierKey {
    Score,
    Pass,
    Cost,
    Tokens,
    Time,
}

#[derive(Debug, Clone)]
pub struct FrontierSort {
    pub key: FrontierKey,
    pub dir: SortDir,
}

#[derive(Debug, Clone)]
pub struct SlopSort {
    pub key: SlopKey,
    pub dir: SortDir,
}

/// The tracker board sorts only by last-seen time.
#[derive(Debug, Clone)]
pub struct TrackerSort {
    pub dir: SortDir,
}

/// The AIHOT boards each sort on one key: hot rank, daily report order, and
/// reset announcement time.
#[derive(Debug, Clone)]
pub struct DirSort {
    pub dir: SortDir,
}

// ── Sort functions ───────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn sort_with(
    data: &mut crate::board::Data,
    aa_sort: &AaSort,
    agent_sort: &AgentSort,
    slop_sort: &SlopSort,
    frontier_sort: &FrontierSort,
    tracker_sort: &TrackerSort,
    hot_sort: &DirSort,
    daily_sort: &DirSort,
    reset_sort: &DirSort,
) {
    match data {
        crate::board::Data::Aa(models) => sort_aa(models, aa_sort),
        crate::board::Data::AaAgents(rows) => sort_agents(rows, agent_sort),
        crate::board::Data::Slop(rows) => sort_slop(rows, slop_sort),
        crate::board::Data::Frontier(rows) => sort_frontier(rows, frontier_sort),
        crate::board::Data::Tracker(pages) => sort_tracker(pages, tracker_sort),
        crate::board::Data::Hot(topics) => {
            sort_by_metric(topics, hot_sort, |t| t.rank.map(f64::from))
        }
        crate::board::Data::Daily(items) => {
            sort_by_metric(items, daily_sort, |i: &aihot::DailyItem| {
                Some(i.position as f64)
            })
        }
        crate::board::Data::Resets(events) => {
            sort_by_metric(events, reset_sort, |e| e.created_unix().map(|t| t as f64))
        }
    }
}

fn sort_aa(models: &mut [aa::Model], sort: &AaSort) {
    let key = sort.key;
    let dir = sort.dir;
    models.sort_by(|a, b| {
        let av = aa_metric(a, key);
        let bv = aa_metric(b, key);
        cmp_opt_for_dir(av, bv, dir)
    });
}

pub fn aa_metric(m: &aa::Model, key: AaKey) -> Option<f64> {
    match key {
        AaKey::Intelligence => m.intelligence_index,
        AaKey::Speed => m.speed(),
        AaKey::Price => m.price_1m_blended(),
        AaKey::Context => m.context_window_tokens.map(|x| x as f64),
        AaKey::Cache => m.cache_hit_discount(),
    }
}

fn sort_agents(rows: &mut [coding_agents::AgentRow], sort: &AgentSort) {
    let key = sort.key;
    let dir = sort.dir;
    rows.sort_by(|a, b| {
        let av = agent_metric(a, key);
        let bv = agent_metric(b, key);
        cmp_opt_for_dir(av, bv, dir)
    });
}

pub fn agent_metric(row: &coding_agents::AgentRow, key: AgentKey) -> Option<f64> {
    match key {
        AgentKey::Index => row.index_score,
        AgentKey::Pass => row.mean.reward,
        AgentKey::Cost => row.mean.cost_usd,
        AgentKey::Time => row.mean.agent_wall_time_sec,
        AgentKey::Tokens => row.mean.total_tokens,
        AgentKey::Turns => row.mean.steps,
    }
}

fn sort_slop(rows: &mut [slopalytics::Row], sort: &SlopSort) {
    let key = sort.key;
    let dir = sort.dir;
    rows.sort_by(|a, b| {
        let av = slop_metric(a, key);
        let bv = slop_metric(b, key);
        cmp_opt_for_dir(av, bv, dir)
    });
}

pub fn slop_metric(row: &slopalytics::Row, key: SlopKey) -> Option<f64> {
    match key {
        SlopKey::Intelligence => row.intelligence,
        SlopKey::Cost => row.cost,
        SlopKey::Time => row.time,
        SlopKey::Tokens => row.tokens,
        SlopKey::Speed => row.speed,
    }
}

fn sort_frontier(rows: &mut [frontiercode::Row], sort: &FrontierSort) {
    rows.sort_by(|a, b| {
        let av = frontier_metric(a, sort.key);
        let bv = frontier_metric(b, sort.key);
        cmp_opt_for_dir(av, bv, sort.dir)
    });
}

pub fn frontier_metric(row: &frontiercode::Row, key: FrontierKey) -> Option<f64> {
    match key {
        FrontierKey::Score => row.score,
        FrontierKey::Pass => row.pass_rate,
        FrontierKey::Cost => row.cost,
        FrontierKey::Tokens => row.tokens,
        FrontierKey::Time => row.minutes,
    }
}

fn sort_tracker(pages: &mut [aitracker::ModelPage], sort: &TrackerSort) {
    pages.sort_by(|a, b| {
        let av = a.last_seen_unix().map(|t| t as f64);
        let bv = b.last_seen_unix().map(|t| t as f64);
        cmp_opt_for_dir(av, bv, sort.dir)
    });
}

fn sort_by_metric<T>(rows: &mut [T], sort: &DirSort, metric: impl Fn(&T) -> Option<f64>) {
    rows.sort_by(|a, b| cmp_opt_for_dir(metric(a), metric(b), sort.dir));
}

fn cmp_opt_for_dir(a: Option<f64>, b: Option<f64>, dir: SortDir) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => {
            let ord = x.partial_cmp(&y).unwrap_or(Ordering::Equal);
            match dir {
                SortDir::Desc => ord.reverse(),
                SortDir::Asc => ord,
            }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::super::AppState;
    use crate::board::{Board, Data, Status};

    #[test]
    fn descending_sorts_keep_missing_metrics_last() {
        let mut app = AppState::new();
        let mut missing = aa_model("missing", "Missing Intel", "Unknown");
        missing.intelligence_index = None;
        app.set_status(
            Board::Aa,
            Status::Loaded(Data::Aa(vec![
                missing,
                aa_model("scored", "Scored Intel", "OpenAI"),
            ])),
        );

        let Some(Status::Loaded(Data::Aa(models))) = app.status.get(&Board::Aa) else {
            panic!("AA data should be loaded");
        };
        assert_eq!(models[0].name, "Scored Intel");
        assert_eq!(models[1].name, "Missing Intel");
    }

    #[test]
    fn descending_agent_sorts_keep_missing_metrics_last() {
        let mut app = AppState::new();
        let mut missing = agent_row("missing", "Missing Agent", "Unknown");
        missing.index_score = None;
        app.set_status(
            Board::AaAgents,
            Status::Loaded(Data::AaAgents(vec![
                missing,
                agent_row("scored", "Scored Agent", "Known"),
            ])),
        );

        let Some(Status::Loaded(Data::AaAgents(rows))) = app.status.get(&Board::AaAgents) else {
            panic!("AA Agents data should be loaded");
        };
        assert_eq!(rows[0].agent(), "Scored Agent");
        assert_eq!(rows[1].agent(), "Missing Agent");
    }
}
