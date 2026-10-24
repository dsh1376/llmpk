use anyhow::Result;

use crate::aa;
use crate::aihot;
use crate::aitracker;
use crate::coding_agents;
use crate::frontiercode;
use crate::slopalytics;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Board {
    Aa,
    AaAgents,
    Slop,
    Frontier,
    Tracker,
    Hot,
    Daily,
    Resets,
}

const ALL_BOARDS: [Board; 8] = [
    Board::Aa,
    Board::AaAgents,
    Board::Slop,
    Board::Frontier,
    Board::Tracker,
    Board::Hot,
    Board::Daily,
    Board::Resets,
];

impl Board {
    pub fn all() -> &'static [Board] {
        &ALL_BOARDS
    }

    pub fn label(self) -> &'static str {
        match self {
            Board::Aa => "AA",
            Board::AaAgents => "AA Agents",
            Board::Slop => "Slopalytics",
            Board::Frontier => "FrontierCode",
            Board::Tracker => "AI Tracker",
            Board::Hot => "AIHOT Hot",
            Board::Daily => "AIHOT Daily",
            Board::Resets => "Codex Resets",
        }
    }

    pub fn shortcut(idx: usize) -> Option<char> {
        match idx {
            0 => Some('1'),
            1 => Some('2'),
            2 => Some('3'),
            3 => Some('4'),
            4 => Some('5'),
            5 => Some('6'),
            6 => Some('7'),
            7 => Some('8'),
            _ => None,
        }
    }

    pub fn from_shortcut(c: char) -> Option<usize> {
        match c {
            '1' => Some(0),
            '2' => Some(1),
            '3' => Some(2),
            '4' => Some(3),
            '5' => Some(4),
            '6' => Some(5),
            '7' => Some(6),
            '8' => Some(7),
            _ => None,
        }
    }

    /// Whether `m` switches to a second view: a chart, or the Codex Resets
    /// calendar. Boards without one stay in table view.
    pub fn has_chart(self) -> bool {
        !matches!(self, Board::Tracker | Board::Hot | Board::Daily)
    }
}

#[derive(Debug, Clone)]
pub enum Data {
    Aa(Vec<aa::Model>),
    AaAgents(Vec<coding_agents::AgentRow>),
    Slop(Vec<slopalytics::Row>),
    Frontier(Vec<frontiercode::Row>),
    Tracker(Vec<aitracker::ModelPage>),
    Hot(Vec<aihot::HotTopic>),
    Daily(Vec<aihot::DailyItem>),
    Resets(Vec<aihot::ResetEvent>),
}

#[derive(Debug, Clone)]
pub enum Status {
    Loading,
    Loaded(Data),
    Error(String),
}

pub fn fetch(board: Board) -> Result<Data> {
    match board {
        Board::Aa => aa::fetch().map(Data::Aa),
        Board::AaAgents => coding_agents::fetch().map(Data::AaAgents),
        Board::Slop => slopalytics::fetch().map(Data::Slop),
        Board::Frontier => frontiercode::fetch().map(Data::Frontier),
        Board::Tracker => aitracker::fetch().map(Data::Tracker),
        Board::Hot => aihot::fetch_hot().map(Data::Hot),
        Board::Daily => aihot::fetch_daily().map(Data::Daily),
        Board::Resets => aihot::fetch_resets().map(Data::Resets),
    }
}
