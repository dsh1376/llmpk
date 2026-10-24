use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::rsc;

const SITE: &str = "https://live.aitracker.bot";
/// Documented on https://live.aitracker.bot/developers as the JSON API for the
/// tracked-models list.
const MODEL_PAGES_URL: &str = "https://live.aitracker.bot/api/model-pages";

/// One model AI Tracker has posted about: a release or a leak.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPage {
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub maker: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    /// ISO 8601 UTC timestamp, for example `2026-10-22T15:43:41.669Z`.
    #[serde(default)]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub post_ids: Vec<String>,
    #[serde(default)]
    pub event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Response {
    pages: Vec<ModelPage>,
}

impl ModelPage {
    pub fn maker(&self) -> Option<&str> {
        self.maker.as_deref().filter(|m| !m.trim().is_empty())
    }

    pub fn stage(&self) -> Option<&str> {
        self.stage.as_deref().filter(|s| !s.trim().is_empty())
    }

    /// Seconds since the Unix epoch, or `None` for a missing or unreadable
    /// timestamp.
    pub fn last_seen_unix(&self) -> Option<i64> {
        self.last_seen_at.as_deref().and_then(parse_utc_timestamp)
    }

    pub fn url(&self) -> Option<String> {
        let path = self.path.as_deref().filter(|p| p.starts_with('/'))?;
        Some(format!("{SITE}{path}"))
    }
}

pub fn fetch() -> Result<Vec<ModelPage>> {
    let json = rsc::fetch_text_retry(MODEL_PAGES_URL)?;
    parse(&json)
}

pub fn parse(json: &str) -> Result<Vec<ModelPage>> {
    let resp: Response = serde_json::from_str(json)
        .map_err(|e| anyhow!("failed to parse AI Tracker model pages JSON: {e}"))?;
    let pages: Vec<ModelPage> = resp
        .pages
        .into_iter()
        .filter(|p| !p.model.trim().is_empty())
        .collect();
    if pages.is_empty() {
        return Err(anyhow!("AI Tracker model pages list empty"));
    }
    Ok(pages)
}

/// Parse `YYYY-MM-DDTHH:MM:SS[.fff]Z` to Unix seconds. Other offsets are not
/// accepted; AI Tracker only emits UTC.
pub fn parse_utc_timestamp(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return None;
    }
    if b[13] != b':' || b[16] != b':' || *b.last()? != b'Z' {
        return None;
    }
    let num = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = s.get(range)?;
        if !part.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hour, minute, second) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let fraction = &s[19..s.len() - 1];
    if !(fraction.is_empty()
        || (fraction.starts_with('.') && fraction[1..].bytes().all(|c| c.is_ascii_digit())))
    {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    if second > 60 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/aitracker_model_pages.json");

    #[test]
    fn live_fetch_when_enabled() {
        if std::env::var("LLMPK_LIVE").is_err() {
            return;
        }
        let pages = fetch().expect("live AI Tracker fetch");
        assert!(pages.len() > 20, "got {} pages", pages.len());
        assert!(pages.iter().all(|p| p.last_seen_unix().is_some()));
    }

    #[test]
    fn parses_committed_fixture() {
        let pages = parse(FIXTURE).expect("parse committed fixture");
        assert_eq!(pages.len(), 4);

        let leak = &pages[0];
        assert_eq!(leak.slug, "minicpm-ocr-1.0");
        assert_eq!(leak.model, "MiniCPM-OCR 1.0");
        assert_eq!(leak.maker(), Some("OpenBMB"));
        assert_eq!(leak.stage(), Some("leak"));
        assert_eq!(leak.last_seen_unix(), Some(1_792_683_821));
        assert_eq!(
            leak.url().as_deref(),
            Some("https://live.aitracker.bot/minicpm-ocr-1.0")
        );
        assert_eq!(leak.post_ids.len(), 1);

        let release = &pages[1];
        assert_eq!(release.stage(), Some("release"));
        assert_eq!(release.event_ids.len(), 1);
    }

    #[test]
    fn empty_maker_reads_as_unknown() {
        let pages = parse(FIXTURE).expect("parse committed fixture");
        let anon = pages
            .iter()
            .find(|p| p.slug == "ll-alpha")
            .expect("fixture should include ll-alpha");
        assert_eq!(anon.maker(), None);
    }

    #[test]
    fn rejects_empty_or_reshaped_payload() {
        assert!(parse(r#"{"pages":[]}"#).is_err());
        assert!(parse(r#"{"items":[]}"#).is_err());
        assert!(parse("not json").is_err());
    }

    #[test]
    fn parses_utc_timestamps() {
        assert_eq!(parse_utc_timestamp("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_utc_timestamp("2024-02-29T23:59:59Z"),
            Some(1_709_251_199)
        );
        assert_eq!(
            parse_utc_timestamp("2026-07-21T06:32:54.291Z"),
            Some(1_784_615_574)
        );
    }

    #[test]
    fn rejects_malformed_timestamps() {
        for s in [
            "",
            "2026-10-22",
            "2026-10-22T15:43:41",
            "2026-10-22T15:43:41+02:00",
            "2026-13-22T15:43:41Z",
            "2026-10-22T25:43:41Z",
            "2026-10-22T15:43:41.abcZ",
            "20x6-10-22T15:43:41Z",
        ] {
            assert_eq!(parse_utc_timestamp(s), None, "{s:?} should not parse");
        }
    }
}
