use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::rsc;

/// AA's models leaderboard is the only page that ships every model with its
/// prices, speed, and context window. The homepage carries a curated subset
/// with no prices.
const MODELS_LEADERBOARD: &str = "https://artificialanalysis.ai/leaderboards/models";

/// One row of the models leaderboard. AA ships raw input, output, and cache
/// prices; the blends and the cache discount are derived from them.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "modelCreatorName")]
    pub creator_name: Option<String>,
    #[serde(default, rename = "modelCreatorColor")]
    pub creator_color: Option<String>,
    #[serde(default)]
    pub intelligence_index: Option<f64>,
    #[serde(default, rename = "medianOutputTokensPerSecond")]
    pub median_output_speed: Option<f64>,
    #[serde(default, rename = "price1mInputTokens")]
    pub price_1m_input_tokens: Option<f64>,
    #[serde(default, rename = "price1mOutputTokens")]
    pub price_1m_output_tokens: Option<f64>,
    #[serde(default)]
    pub cache_hit_price: Option<f64>,
    /// Price per 1M tokens to write to the provider's prompt cache. AA only
    /// publishes it for providers that charge for writes.
    #[serde(default)]
    pub cache_write_price: Option<f64>,
    #[serde(default)]
    pub context_window_tokens: Option<u64>,
    /// Joined in from the page's release table; not part of the row itself.
    #[serde(skip)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub is_open_weights: Option<bool>,
}

impl Model {
    pub fn provider(&self) -> &str {
        self.creator_name.as_deref().unwrap_or("?")
    }

    pub fn provider_color(&self) -> Option<&str> {
        self.creator_color.as_deref()
    }

    pub fn speed(&self) -> Option<f64> {
        self.median_output_speed
    }

    /// AA's headline blended price: 7:2:1 cache-hit:input:output, USD per 1M
    /// tokens. Unknown when the model has no cache-hit price.
    pub fn price_1m_blended(&self) -> Option<f64> {
        let hit = self.cache_hit_price?;
        let input = self.price_1m_input_tokens?;
        let output = self.price_1m_output_tokens?;
        Some((7.0 * hit + 2.0 * input + output) / 10.0)
    }

    /// The blend with no cache hits: 3:1 input:output, USD per 1M tokens.
    pub fn price_1m_blended_no_cache(&self) -> Option<f64> {
        let input = self.price_1m_input_tokens?;
        let output = self.price_1m_output_tokens?;
        Some((3.0 * input + output) / 4.0)
    }

    /// Share of the input price saved on cache hits (0.9 = 90% cheaper).
    pub fn cache_hit_discount(&self) -> Option<f64> {
        let hit = self.cache_hit_price?;
        let input = self.price_1m_input_tokens.filter(|p| *p > 0.0)?;
        Some(1.0 - hit / input)
    }
}

/// The page's `modelsAndReleases` table: model slug -> release slug, and
/// release slug -> release date.
#[derive(Debug, Default, Deserialize)]
struct ReleaseTable {
    #[serde(default)]
    releases: Vec<Release>,
    #[serde(default)]
    models: Vec<ModelRelease>,
}

#[derive(Debug, Deserialize)]
struct Release {
    slug: String,
    #[serde(default, rename = "releaseDate")]
    release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelRelease {
    slug: String,
    #[serde(default, rename = "releaseSlug")]
    release_slug: Option<String>,
}

impl ReleaseTable {
    fn release_dates(self) -> HashMap<String, String> {
        let dates: HashMap<String, String> = self
            .releases
            .into_iter()
            .filter_map(|r| Some((r.slug, r.release_date?)))
            .collect();
        self.models
            .into_iter()
            .filter_map(|m| {
                let date = dates.get(m.release_slug.as_deref()?)?;
                Some((m.slug, date.clone()))
            })
            .collect()
    }
}

pub fn fetch() -> Result<Vec<Model>> {
    let html = rsc::fetch_text_retry(MODELS_LEADERBOARD)?;
    parse(&html)
}

pub fn parse(html: &str) -> Result<Vec<Model>> {
    let stream = rsc::extract_stream(html)?;
    let mut seen = HashSet::new();
    let mut out: Vec<Model> = rsc::innermost_objects_with(&stream, "\"contextWindowTokens\":")
        .into_iter()
        .filter_map(|span| serde_json::from_str::<Model>(span).ok())
        .filter(|m| !m.slug.is_empty() && seen.insert(m.slug.clone()))
        .collect();
    if out.is_empty() {
        return Err(anyhow!("no model rows found in artificialanalysis.ai"));
    }

    // Release dates are best-effort: a missing or reshaped table leaves `-`.
    let dates = rsc::first_object_after(&stream, "modelsAndReleases")
        .and_then(|span| serde_json::from_str::<ReleaseTable>(span).ok())
        .unwrap_or_default()
        .release_dates();
    for model in &mut out {
        model.release_date = dates.get(&model.slug).cloned();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/aa_leaderboard.html");

    fn find<'a>(models: &'a [Model], slug: &str) -> &'a Model {
        models
            .iter()
            .find(|m| m.slug == slug)
            .unwrap_or_else(|| panic!("fixture should include {slug}"))
    }

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("value should be present");
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn live_fetch_when_enabled() {
        if std::env::var("LLMPK_LIVE").is_err() {
            return;
        }
        let models = fetch().expect("live artificialanalysis fetch");
        assert!(models.len() > 100, "got {} models", models.len());
        let count = |f: fn(&Model) -> bool| models.iter().filter(|m| f(m)).count();
        assert!(count(|m| m.price_1m_blended().is_some()) > 50);
        assert!(count(|m| m.speed().is_some()) > 50);
        assert!(count(|m| m.context_window_tokens.is_some()) > 100);
        assert!(count(|m| m.release_date.is_some()) > 100);
    }

    #[test]
    fn parses_committed_fixture() {
        let models = parse(FIXTURE).expect("parse committed fixture");
        assert_eq!(models.len(), 4);

        let opus = find(&models, "claude-opus-5-5");
        assert_eq!(opus.name, "Claude Opus 5.5 (Max, Default Fallback)");
        assert_eq!(opus.provider(), "Anthropic");
        assert_eq!(opus.provider_color(), Some("#cc785c"));
        assert_close(opus.intelligence_index, 57.6223698102963);
        assert_close(opus.speed(), 96.0159691985147);
        assert_close(opus.price_1m_input_tokens, 4.0);
        assert_close(opus.price_1m_output_tokens, 20.0);
        assert_close(opus.cache_hit_price, 0.2);
        assert_close(opus.cache_write_price, 5.0);
        assert_eq!(opus.context_window_tokens, Some(1_000_000));
        assert_eq!(opus.is_open_weights, Some(false));
        assert_eq!(opus.release_date.as_deref(), Some("2026-09-22"));
    }

    #[test]
    fn derives_aa_price_blends_and_cache_discount() {
        let models = parse(FIXTURE).expect("parse committed fixture");

        // (7 * 0.2 + 2 * 4 + 20) / 10, (3 * 4 + 20) / 4, 1 - 0.2 / 4
        let opus = find(&models, "claude-opus-5-5");
        assert_close(opus.price_1m_blended(), 2.94);
        assert_close(opus.price_1m_blended_no_cache(), 8.0);
        assert_close(opus.cache_hit_discount(), 0.95);

        // No cache-hit price: the 7:2:1 blend and the discount are unknown,
        // the 3:1 blend is not.
        let qwen = find(&models, "qwen3-5-omni-plus");
        assert_eq!(qwen.price_1m_blended(), None);
        assert_close(qwen.price_1m_blended_no_cache(), 1.5);
        assert_eq!(qwen.cache_hit_discount(), None);

        let jt = find(&models, "jt-35b-flash");
        assert_eq!(jt.price_1m_blended(), None);
        assert_eq!(jt.price_1m_blended_no_cache(), None);
        assert_eq!(jt.speed(), None);
    }

    #[test]
    fn release_date_joins_through_release_slug() {
        let models = parse(FIXTURE).expect("parse committed fixture");
        let high = find(&models, "claude-opus-5-5-high");
        assert_eq!(high.release_date.as_deref(), Some("2026-09-22"));
    }

    #[test]
    fn missing_release_table_leaves_dates_empty() {
        let html = r#"<script>self.__next_f.push([1,"{\"slug\":\"m\",\"name\":\"M\",\"contextWindowTokens\":8000}"])</script>"#;
        let models = parse(html).expect("parse row without release table");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].release_date, None);
        assert_eq!(models[0].provider(), "?");
    }

    #[test]
    fn rejects_page_without_model_rows() {
        let html = r#"<script>self.__next_f.push([1,"{\"other\":1}"])</script>"#;
        assert!(parse(html).is_err());
    }

    #[test]
    fn parses_fixture_when_provided() {
        let Ok(path) = std::env::var("LLMPK_LEADERBOARD_FIXTURE") else {
            return;
        };
        let html = std::fs::read_to_string(&path).expect("fixture read");
        let models = parse(&html).expect("parse");
        assert!(
            models.len() >= 100,
            "expected >=100 models, got {}",
            models.len()
        );
    }
}
