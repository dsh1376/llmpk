use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::rsc;

/// The JSON the leaderboard on cognition.com/frontiercode loads.
const DATA_URL: &str = "https://cognition.com/data/frontiercode-leaderboard/data.json";
/// The leaderboard ranks on this subset; `extended` is a larger task set.
const SUBSET: &str = "main";
/// Reasoning efforts in display order; anything else sorts after these.
const EFFORT_ORDER: [&str; 7] = ["none", "minimal", "low", "medium", "high", "xhigh", "max"];

/// One model on the FrontierCode leaderboard, at the effort that scored best,
/// as the site shows it.
#[derive(Debug, Clone)]
pub struct Row {
    pub model: String,
    pub lab: Option<String>,
    /// Lab color from the site, `#rrggbb`.
    pub color: Option<String>,
    pub harness: Option<String>,
    pub effort: Option<String>,
    /// Mergeability score, 0..1: the leaderboard's ranking metric.
    pub score: Option<f64>,
    /// Share of tasks whose tests pass, 0..1.
    pub pass_rate: Option<f64>,
    /// Share of runs zeroed for consulting solution-bearing sources, 0..1.
    pub flag_rate: Option<f64>,
    /// USD per rollout.
    pub cost: Option<f64>,
    /// Output tokens per rollout.
    pub tokens: Option<f64>,
    pub minutes: Option<f64>,
    pub steps: Option<f64>,
    /// Every evaluated effort with its score, in effort order.
    pub efforts: Vec<(String, Option<f64>)>,
}

impl Row {
    pub fn display_name(&self) -> String {
        match &self.effort {
            Some(effort) => format!("{} [{effort}]", self.model),
            None => self.model.clone(),
        }
    }

    pub fn lab(&self) -> &str {
        self.lab.as_deref().unwrap_or("?")
    }
}

#[derive(Debug, Deserialize)]
struct Revision {
    #[serde(default)]
    models: Vec<String>,
    #[serde(default)]
    colors: HashMap<String, String>,
    #[serde(default)]
    lab_colors: HashMap<String, String>,
    #[serde(default)]
    harness: HashMap<String, String>,
    #[serde(default)]
    data: HashMap<String, HashMap<String, HashMap<String, Metrics>>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Metrics {
    #[serde(default)]
    correct: Option<f64>,
    #[serde(default)]
    new_score: Option<f64>,
    #[serde(default)]
    tokens: Option<f64>,
    #[serde(default)]
    cost: Option<f64>,
    #[serde(default)]
    duration_min: Option<f64>,
    #[serde(default)]
    steps: Option<f64>,
    #[serde(default)]
    flagged_rate: Option<f64>,
}

pub fn fetch() -> Result<Vec<Row>> {
    let json = rsc::fetch_text_retry(DATA_URL)?;
    parse(&json)
}

pub fn parse(json: &str) -> Result<Vec<Row>> {
    let revisions: HashMap<String, Revision> = serde_json::from_str(json)
        .map_err(|e| anyhow!("failed to parse FrontierCode leaderboard JSON: {e}"))?;
    let (_, current) = revisions
        .into_iter()
        .filter_map(|(key, rev)| Some((revision_number(&key)?, rev)))
        .max_by(|a, b| a.0.cmp(&b.0))
        .ok_or_else(|| anyhow!("no FrontierCode revision found"))?;

    let lab_by_color: HashMap<String, String> = current
        .lab_colors
        .iter()
        .map(|(lab, color)| (color.to_lowercase(), lab.clone()))
        .collect();

    let rows: Vec<Row> = current
        .models
        .iter()
        .filter_map(|model| {
            let by_effort = current.data.get(model)?;
            let color = current.colors.get(model).cloned();
            let lab = color
                .as_ref()
                .and_then(|c| lab_by_color.get(&c.to_lowercase()).cloned());
            best_row(
                model,
                by_effort,
                lab,
                color,
                current.harness.get(model).cloned(),
            )
        })
        .collect();
    if rows.is_empty() {
        return Err(anyhow!("FrontierCode leaderboard has no models"));
    }
    Ok(rows)
}

/// `v1` -> (1, 0), `v1_1` -> (1, 1). Revisions are named like versions.
fn revision_number(key: &str) -> Option<(u32, u32)> {
    let rest = key.strip_prefix('v')?;
    let mut parts = rest.split('_');
    let major = parts.next()?.parse().ok()?;
    let minor = match parts.next() {
        Some(p) => p.parse().ok()?,
        None => 0,
    };
    Some((major, minor))
}

fn effort_rank(effort: &str) -> usize {
    EFFORT_ORDER
        .iter()
        .position(|e| *e == effort)
        .unwrap_or(EFFORT_ORDER.len())
}

fn best_row(
    model: &str,
    by_effort: &HashMap<String, HashMap<String, Metrics>>,
    lab: Option<String>,
    color: Option<String>,
    harness: Option<String>,
) -> Option<Row> {
    let mut efforts: Vec<(&String, Option<&Metrics>)> = by_effort
        .iter()
        .map(|(effort, subsets)| (effort, subsets.get(SUBSET)))
        .collect();
    efforts.sort_by_key(|(effort, _)| (effort_rank(effort), (*effort).clone()));

    let (best_effort, best) = efforts
        .iter()
        .filter_map(|(effort, m)| Some((*effort, (*m)?)))
        .filter(|(_, m)| m.new_score.is_some())
        .max_by(|a, b| {
            a.1.new_score
                .partial_cmp(&b.1.new_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

    Some(Row {
        model: model.to_string(),
        lab,
        color,
        harness,
        // "none" marks a model without effort levels.
        effort: Some(best_effort.clone()).filter(|e| e != "none"),
        score: best.new_score,
        pass_rate: best.correct,
        flag_rate: best.flagged_rate,
        cost: best.cost,
        tokens: best.tokens,
        minutes: best.duration_min,
        steps: best.steps,
        efforts: efforts
            .iter()
            .map(|(effort, m)| ((*effort).clone(), m.and_then(|m| m.new_score)))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/frontiercode_data.json");

    fn find<'a>(rows: &'a [Row], model: &str) -> &'a Row {
        rows.iter()
            .find(|r| r.model == model)
            .unwrap_or_else(|| panic!("fixture should include {model}"))
    }

    #[test]
    fn live_fetch_when_enabled() {
        if std::env::var("LLMPK_LIVE").is_err() {
            return;
        }
        let rows = fetch().expect("live FrontierCode fetch");
        assert!(rows.len() > 10, "got {} rows", rows.len());
        assert!(rows.iter().all(|r| r.score.is_some()));
    }

    #[test]
    fn uses_the_newest_revision() {
        let rows = parse(FIXTURE).expect("parse committed fixture");
        // v1 only has Claude Fable 5; v1_1 has these three.
        let mut models: Vec<&str> = rows.iter().map(|r| r.model.as_str()).collect();
        models.sort_unstable();
        assert_eq!(models, ["Claude Opus 5.5", "Inkling", "Kimi K3"]);
    }

    #[test]
    fn keeps_the_best_scoring_effort_like_the_site() {
        let rows = parse(FIXTURE).expect("parse committed fixture");
        let opus = find(&rows, "Claude Opus 5.5");
        assert_eq!(opus.effort.as_deref(), Some("medium"));
        assert_eq!(opus.display_name(), "Claude Opus 5.5 [medium]");
        assert_eq!(opus.score, Some(0.5464));
        assert_eq!(opus.pass_rate, Some(0.596));
        assert_eq!(opus.cost, Some(0.8016));
        assert_eq!(opus.tokens, Some(18519.0));
        assert_eq!(opus.minutes, Some(6.34));
        assert_eq!(opus.flag_rate, Some(0.0));
        assert_eq!(opus.lab(), "Anthropic");
        assert_eq!(opus.harness.as_deref(), Some("claude-code"));

        let order: Vec<&str> = opus.efforts.iter().map(|(e, _)| e.as_str()).collect();
        assert_eq!(order, ["low", "medium", "high", "xhigh", "max"]);
    }

    #[test]
    fn models_without_effort_levels_have_no_effort() {
        let rows = parse(FIXTURE).expect("parse committed fixture");
        let kimi = find(&rows, "Kimi K3");
        assert_eq!(kimi.effort, None);
        assert_eq!(kimi.display_name(), "Kimi K3");
        assert_eq!(kimi.lab(), "Moonshot");
        assert_eq!(kimi.minutes, None);

        // An unusual effort key is kept as the site names it.
        let inkling = find(&rows, "Inkling");
        assert_eq!(inkling.effort.as_deref(), Some("0.99"));
        assert_eq!(inkling.lab(), "Thinking Machines");
    }

    #[test]
    fn rejects_unusable_payloads() {
        assert!(parse("{}").is_err());
        assert!(parse(r#"{"v1":{"models":[]}}"#).is_err());
        assert!(parse("not json").is_err());
    }

    #[test]
    fn orders_revisions_numerically() {
        assert!(revision_number("v1_10") > revision_number("v1_9"));
        assert!(revision_number("v2") > revision_number("v1_1"));
        assert_eq!(revision_number("latest"), None);
    }
}
