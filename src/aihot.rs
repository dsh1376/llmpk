//! AIHOT (aihot.news): the hot-topics list, the latest daily report, and the
//! Codex usage-reset monitor. All three come from the anonymous read-only
//! `/api/v1` JSON API listed in https://aihot.news/llms.txt and
//! https://aihot.news/openapi-v1.json.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::aitracker::parse_utc_timestamp;
use crate::rsc;

const HOT_URL: &str = "https://aihot.news/api/v1/hot-topics";
const DAILY_URL: &str = "https://aihot.news/api/v1/dailies/latest";
/// The full history, for the calendar. Each client may read it once every
/// 10 minutes; a second read answers 429, so reads go through a disk cache.
const RESETS_URL: &str = "https://aihot.news/api/v1/codex-resets";
const RESETS_CACHE_TTL: Duration = Duration::from_secs(10 * 60);
const RESETS_CACHE_FILE: &str = "codex-resets.json";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Source {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Links {
    #[serde(default)]
    pub aihot: Option<String>,
    #[serde(default)]
    pub original: Option<String>,
    #[serde(default)]
    pub story: Option<String>,
}

/// One event on the AIHOT hot list: what several independent sources
/// discussed in the last 48 hours.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotTopic {
    #[serde(default)]
    pub rank: Option<u32>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub links: Links,
    /// Independent sources that covered the event.
    #[serde(default)]
    pub source_count: Option<u32>,
    #[serde(default)]
    pub signal_count: Option<u32>,
    #[serde(default)]
    pub participant_count: Option<u32>,
    /// A sample of the covering sources, not all of them.
    #[serde(default)]
    pub source_names: Vec<String>,
    /// ISO 8601 UTC timestamp of the newest report.
    #[serde(default)]
    pub latest_at: Option<String>,
}

impl HotTopic {
    pub fn latest_unix(&self) -> Option<i64> {
        self.latest_at.as_deref().and_then(parse_timestamp)
    }
}

#[derive(Debug, Deserialize)]
struct HotResponse {
    items: Vec<HotTopic>,
}

/// One entry of the latest AIHOT daily report: a section item or a flash.
#[derive(Debug, Clone)]
pub struct DailyItem {
    /// Order in the report, from 0.
    pub position: usize,
    /// Report date, `YYYY-MM-DD` (Beijing time).
    pub date: String,
    /// Section label as the site shows it, or `FLASH_SECTION`.
    pub section: String,
    pub title: String,
    pub summary: Option<String>,
    pub source: Option<String>,
    pub links: Links,
    /// Only flashes carry a publish time.
    pub published_at: Option<String>,
}

pub const FLASH_SECTION: &str = "Flash";

#[derive(Debug, Deserialize)]
struct DailyResponse {
    report: DailyReport,
}

#[derive(Debug, Deserialize)]
struct DailyReport {
    #[serde(default)]
    date: String,
    #[serde(default)]
    sections: Vec<DailySection>,
    #[serde(default)]
    flashes: Vec<RawDailyItem>,
}

#[derive(Debug, Deserialize)]
struct DailySection {
    #[serde(default)]
    label: String,
    #[serde(default)]
    items: Vec<RawDailyItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDailyItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    source: Source,
    #[serde(default)]
    links: Links,
    #[serde(default)]
    published_at: Option<String>,
}

/// One Codex usage reset (or reset-credit grant) that Tibo (OpenAI's Codex
/// lead) announced on X, as AIHOT tracks it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetEvent {
    #[serde(default)]
    pub id: String,
    /// `direct_reset` or `reset_credit`.
    #[serde(default, rename = "type")]
    pub kind: String,
    /// Workflow status: `announced` or `confirmed`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub presentation: Presentation,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub confirmed_at: Option<String>,
    #[serde(default)]
    pub schedule: Option<Schedule>,
    /// Newest first.
    #[serde(default)]
    pub posts: Vec<ResetPost>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Presentation {
    /// What the site shows: `confirmed` or `likely_completed`. An announced
    /// reset whose deadline passed reads as `likely_completed` here.
    #[serde(default)]
    pub status: Option<String>,
    /// Who the reset covers, in the post's words (mostly English).
    #[serde(default)]
    pub scope_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Schedule {
    /// `deadline`, `window`, `date`, or `approximate`.
    #[serde(default)]
    pub precision: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub through: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPost {
    /// Chinese stage label such as `预告` (announced) or `确认完成` (confirmed).
    #[serde(default)]
    pub stage: Option<String>,
    /// The X post in English.
    #[serde(default)]
    pub original_text: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

impl ResetPost {
    /// The stage in English; unknown stages pass through.
    pub fn stage_label(&self) -> Option<&str> {
        let stage = self.stage.as_deref().filter(|s| !s.is_empty())?;
        Some(match stage {
            "预告" => "announced",
            "发卡预告" => "credit announced",
            "进展" => "update",
            "补充说明" => "note",
            "确认完成" => "confirmed",
            "确认发卡" => "credit confirmed",
            other => other,
        })
    }
}

/// `checkedAt` and `historyFrom` are required as a shape check.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct ResetsResponse {
    checked_at: String,
    history_from: String,
    events: Vec<ResetEvent>,
}

impl ResetEvent {
    /// `reset` for a direct reset, `credit` for a reset-credit grant.
    pub fn kind_label(&self) -> &str {
        match self.kind.as_str() {
            "direct_reset" => "reset",
            "reset_credit" => "credit",
            other => other,
        }
    }

    /// The status the site shows, falling back to the workflow status.
    pub fn status(&self) -> Option<&str> {
        self.presentation
            .status
            .as_deref()
            .or(self.status.as_deref())
            .filter(|s| !s.is_empty())
    }

    pub fn status_label(&self) -> &str {
        match self.status() {
            Some("likely_completed") => "likely",
            Some(other) => other,
            None => "-",
        }
    }

    pub fn scope(&self) -> Option<&str> {
        self.presentation
            .scope_label
            .as_deref()
            .filter(|s| !s.trim().is_empty())
    }

    pub fn created_unix(&self) -> Option<i64> {
        self.created_at.as_deref().and_then(parse_timestamp)
    }

    /// The calendar day as Unix days (UTC): the confirmation post if there
    /// is one, else the announcement. aihot.news places events the same
    /// way, in Beijing time.
    pub fn calendar_day(&self) -> Option<i64> {
        self.effective_unix().map(|t| t.div_euclid(86_400))
    }

    /// The confirmation post time, else the announcement time.
    pub fn effective_unix(&self) -> Option<i64> {
        self.confirmed_at
            .as_deref()
            .and_then(parse_timestamp)
            .or_else(|| self.created_unix())
    }

    /// The newest post's English text.
    pub fn latest_text(&self) -> Option<&str> {
        self.posts
            .iter()
            .find_map(|p| p.original_text.as_deref())
            .filter(|t| !t.trim().is_empty())
    }

    /// The newest post on X.
    pub fn post_url(&self) -> Option<&str> {
        self.posts.iter().find_map(|p| p.url.as_deref())
    }

    /// Expected landing time in UTC, such as `by 2026-10-23 17:00 UTC`.
    pub fn schedule_label(&self) -> Option<String> {
        let schedule = self.schedule.as_ref()?;
        let from = schedule.from.as_deref().and_then(fmt_utc);
        let through = schedule.through.as_deref().and_then(fmt_utc);
        match (schedule.precision.as_deref(), from, through) {
            (Some("deadline"), _, Some(t)) => Some(format!("by {t}")),
            (Some("approximate"), Some(f), _) => Some(format!("~{f}")),
            (_, Some(f), Some(t)) if f == t => Some(f),
            (_, Some(f), Some(t)) => Some(format!("{f} - {t}")),
            (_, Some(f), None) => Some(format!("after {f}")),
            (_, None, Some(t)) => Some(format!("by {t}")),
            (_, None, None) => None,
        }
    }
}

pub fn fetch_hot() -> Result<Vec<HotTopic>> {
    parse_hot(&rsc::fetch_text_retry(HOT_URL)?)
}

pub fn fetch_daily() -> Result<Vec<DailyItem>> {
    parse_daily(&rsc::fetch_text_retry(DAILY_URL)?)
}

pub fn fetch_resets() -> Result<Vec<ResetEvent>> {
    load_resets(resets_cache_path().as_deref(), SystemTime::now(), || {
        rsc::fetch_text_retry(RESETS_URL)
    })
}

/// `$XDG_CACHE_HOME/llmpk`, else `~/.cache/llmpk`; `None` without either.
fn resets_cache_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("llmpk").join(RESETS_CACHE_FILE))
}

/// Use the cached body while it is younger than the API's read interval,
/// else fetch, parse, and replace the cache. Only a body that parses is
/// cached.
fn load_resets(
    cache: Option<&Path>,
    now: SystemTime,
    fetch: impl FnOnce() -> Result<String>,
) -> Result<Vec<ResetEvent>> {
    if let Some(events) = cache
        .and_then(|path| read_fresh(path, now))
        .and_then(|body| parse_resets(&body).ok())
    {
        return Ok(events);
    }
    let body = fetch()?;
    let events = parse_resets(&body)?;
    if let Some(path) = cache {
        // Best effort: without the cache, the next read within 10 minutes
        // gets a 429, and the board shows that error.
        let _ = write_cache(path, &body);
    }
    Ok(events)
}

fn read_fresh(path: &Path, now: SystemTime) -> Option<String> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    // A write just after `now`, or clock skew, puts the mtime ahead.
    let age = now
        .duration_since(modified)
        .unwrap_or_else(|e| e.duration());
    if age >= RESETS_CACHE_TTL {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Write through a temporary file so a reader never sees a partial body.
fn write_cache(path: &Path, body: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, path)
}

pub fn parse_hot(json: &str) -> Result<Vec<HotTopic>> {
    let resp: HotResponse = serde_json::from_str(json)
        .map_err(|e| anyhow!("failed to parse AIHOT hot-topics JSON: {e}"))?;
    let items: Vec<HotTopic> = resp
        .items
        .into_iter()
        .filter(|t| !t.title.trim().is_empty())
        .collect();
    if items.is_empty() {
        return Err(anyhow!("AIHOT hot-topics list empty"));
    }
    Ok(items)
}

/// Flatten the report into its section items, in site order, then the flashes.
pub fn parse_daily(json: &str) -> Result<Vec<DailyItem>> {
    let resp: DailyResponse =
        serde_json::from_str(json).map_err(|e| anyhow!("failed to parse AIHOT daily JSON: {e}"))?;
    let report = resp.report;
    let date = report.date;
    let sections = report
        .sections
        .into_iter()
        .flat_map(|s| {
            let label = s.label;
            s.items.into_iter().map(move |item| (label.clone(), item))
        })
        .chain(
            report
                .flashes
                .into_iter()
                .map(|item| (FLASH_SECTION.to_string(), item)),
        );
    let items: Vec<DailyItem> = sections
        .filter(|(_, item)| !item.title.trim().is_empty())
        .enumerate()
        .map(|(position, (section, item))| DailyItem {
            position,
            date: date.clone(),
            section,
            title: item.title,
            summary: item.summary.filter(|s| !s.trim().is_empty()),
            source: item.source.name,
            links: item.links,
            published_at: item.published_at,
        })
        .collect();
    if items.is_empty() {
        return Err(anyhow!("AIHOT daily report has no items"));
    }
    Ok(items)
}

pub fn parse_resets(json: &str) -> Result<Vec<ResetEvent>> {
    let resp: ResetsResponse = serde_json::from_str(json)
        .map_err(|e| anyhow!("failed to parse AIHOT Codex reset JSON: {e}"))?;
    if resp.events.is_empty() {
        return Err(anyhow!("AIHOT Codex reset history empty"));
    }
    Ok(resp.events)
}

/// Parse `YYYY-MM-DDTHH:MM:SS[.fff]` with a `Z` or `±HH:MM` offset to Unix
/// seconds. AIHOT emits both: UTC for news, Beijing time for resets.
pub fn parse_timestamp(s: &str) -> Option<i64> {
    if s.ends_with('Z') {
        return parse_utc_timestamp(s);
    }
    let split = s.len().checked_sub(6)?;
    let (local, offset) = (s.get(..split)?, s.get(split..)?);
    let ob = offset.as_bytes();
    let sign = match ob[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    if ob[3] != b':' || !ob[1..3].iter().chain(&ob[4..]).all(u8::is_ascii_digit) {
        return None;
    }
    let hours: i64 = offset[1..3].parse().ok()?;
    let minutes: i64 = offset[4..].parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    let as_utc = parse_utc_timestamp(&format!("{local}Z"))?;
    Some(as_utc - sign * (hours * 3_600 + minutes * 60))
}

/// Any accepted timestamp as `YYYY-MM-DD HH:MM UTC`.
pub fn fmt_utc(s: &str) -> Option<String> {
    let t = parse_timestamp(s)?;
    let (year, month, day) = civil_from_days(t.div_euclid(86_400));
    let secs = t.rem_euclid(86_400);
    Some(format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        secs / 3_600,
        secs % 3_600 / 60
    ))
}

/// Proleptic Gregorian date for days since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOT: &str = include_str!("../tests/fixtures/aihot_hot_topics.json");
    const DAILY: &str = include_str!("../tests/fixtures/aihot_daily.json");
    const RESETS: &str = include_str!("../tests/fixtures/aihot_codex_resets.json");

    #[test]
    fn live_fetch_when_enabled() {
        if std::env::var("LLMPK_LIVE").is_err() {
            return;
        }
        let hot = fetch_hot().expect("live AIHOT hot-topics fetch");
        assert!(hot.len() >= 5, "got {} hot topics", hot.len());
        assert!(hot.iter().all(|t| t.rank.is_some()));
        let daily = fetch_daily().expect("live AIHOT daily fetch");
        assert!(daily.len() >= 5, "got {} daily items", daily.len());
        let resets = fetch_resets().expect("live AIHOT Codex reset fetch");
        assert!(resets.len() > 10, "got {} resets", resets.len());
        assert!(resets.iter().all(|e| e.calendar_day().is_some()));
        assert!(resets.iter().all(|e| e.status().is_some()));
    }

    #[test]
    fn parses_hot_fixture() {
        let hot = parse_hot(HOT).expect("parse hot fixture");
        assert_eq!(hot.len(), 3);
        let top = &hot[0];
        assert_eq!(top.rank, Some(1));
        assert_eq!(top.source.name.as_deref(), Some("Moonshot AI：Kimi Blog"));
        assert_eq!(top.source_count, Some(20));
        assert_eq!(top.participant_count, Some(76));
        assert_eq!(top.source_names.len(), 3);
        assert_eq!(
            top.links.original.as_deref(),
            Some("https://www.kimi.com/blog/kimi-k3-1-pro")
        );
        assert_eq!(
            top.latest_unix(),
            parse_utc_timestamp("2026-10-24T04:19:28Z")
        );
    }

    #[test]
    fn daily_flattens_sections_then_flashes() {
        let items = parse_daily(DAILY).expect("parse daily fixture");
        assert_eq!(items.len(), 8);
        assert!(items.iter().all(|i| i.date == "2026-10-24"));
        assert_eq!(items[0].section, "模型发布/更新");
        assert!(items[0].summary.is_some());
        assert!(items[0].published_at.is_none());
        let flashes: Vec<_> = items
            .iter()
            .filter(|i| i.section == FLASH_SECTION)
            .collect();
        assert_eq!(flashes.len(), 2);
        assert!(flashes.iter().all(|i| i.published_at.is_some()));
        assert_eq!(items.last().unwrap().section, FLASH_SECTION);
    }

    #[test]
    fn resets_read_presentation_status_and_english_text() {
        let events = parse_resets(RESETS).expect("parse reset fixture");
        assert_eq!(events.len(), 4);

        let landed = &events[0];
        assert_eq!(landed.status.as_deref(), Some("announced"));
        assert_eq!(landed.status_label(), "likely");
        assert_eq!(landed.kind_label(), "reset");
        assert_eq!(
            landed.latest_text(),
            Some("Landing tomorrow before 10am PST together with a small ship.")
        );
        assert_eq!(
            landed.schedule_label().as_deref(),
            Some("by 2026-10-23 17:00 UTC")
        );

        let unscheduled = &events[1];
        assert_eq!(unscheduled.status_label(), "confirmed");
        assert_eq!(unscheduled.schedule_label(), None);
        assert_eq!(unscheduled.scope(), None);

        let credit = &events[2];
        assert_eq!(credit.kind_label(), "credit");
        assert_eq!(credit.scope(), Some("everyone's paid accounts"));
    }

    #[test]
    fn resets_cache_is_reused_within_the_read_interval() {
        let dir = std::env::temp_dir().join(format!("llmpk-cache-test-{}", std::process::id()));
        let path = dir.join(RESETS_CACHE_FILE);
        let _ = std::fs::remove_dir_all(&dir);
        let now = SystemTime::now();

        let first = load_resets(Some(&path), now, || Ok(RESETS.to_string())).expect("fetch");
        assert_eq!(first.len(), 4);
        assert!(path.exists(), "a parsed body should be cached");

        let cached = load_resets(Some(&path), now, || panic!("fresh cache must not refetch"))
            .expect("cached");
        assert_eq!(cached.len(), 4);

        let later = now + RESETS_CACHE_TTL + Duration::from_secs(1);
        let err = load_resets(Some(&path), later, || Err(anyhow!("HTTP 429")))
            .expect_err("a stale cache should refetch");
        assert!(err.to_string().contains("429"));

        std::fs::remove_dir_all(&dir).expect("remove test cache dir");
    }

    #[test]
    fn unparsable_body_is_not_cached() {
        let dir = std::env::temp_dir().join(format!("llmpk-cache-bad-{}", std::process::id()));
        let path = dir.join(RESETS_CACHE_FILE);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load_resets(Some(&path), SystemTime::now(), || Ok("<html>".into())).is_err());
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn calendar_day_prefers_the_confirmation() {
        let events = parse_resets(RESETS).expect("parse reset fixture");
        // Announced 2026-10-23 04:47 UTC, not confirmed.
        assert_eq!(
            events[0].calendar_day(),
            parse_utc_timestamp("2026-10-23T00:00:00Z").map(|t| t / 86_400)
        );
        // Announced 2026-10-07 19:19 UTC, confirmed 2026-10-08 03:44 UTC.
        assert_eq!(
            events[2].calendar_day(),
            parse_utc_timestamp("2026-10-08T00:00:00Z").map(|t| t / 86_400)
        );
    }

    #[test]
    fn schedule_label_follows_precision() {
        let event = |precision: &str, from: &str, through: &str| ResetEvent {
            id: String::new(),
            kind: "direct_reset".into(),
            status: None,
            presentation: Presentation::default(),
            created_at: None,
            confirmed_at: None,
            schedule: Some(Schedule {
                precision: Some(precision.into()),
                from: Some(from.into()),
                through: Some(through.into()),
            }),
            posts: Vec::new(),
        };
        let a = "2026-10-03T01:00:00.000+08:00";
        let b = "2026-10-03T02:00:00.000+08:00";
        assert_eq!(
            event("window", a, b).schedule_label().as_deref(),
            Some("2026-10-02 17:00 UTC - 2026-10-02 18:00 UTC")
        );
        assert_eq!(
            event("approximate", a, a).schedule_label().as_deref(),
            Some("~2026-10-02 17:00 UTC")
        );
    }

    #[test]
    fn rejects_empty_or_reshaped_payloads() {
        assert!(parse_hot(r#"{"items":[]}"#).is_err());
        assert!(parse_hot(r#"{"topics":[]}"#).is_err());
        assert!(parse_daily(r#"{"report":{"date":"2026-10-24","sections":[]}}"#).is_err());
        assert!(parse_daily(r#"{"sections":[]}"#).is_err());
        assert!(parse_resets(r#"{"events":[]}"#).is_err());
        assert!(parse_resets(r#"{"checkedAt":"x","historyFrom":"y","events":[]}"#).is_err());
        assert!(parse_resets(r#"{"checkedAt":"x","historyFrom":"y","items":[]}"#).is_err());
        assert!(parse_resets("not json").is_err());
    }

    #[test]
    fn parses_offset_timestamps() {
        assert_eq!(
            parse_timestamp("2026-10-23T12:47:34.000+08:00"),
            parse_utc_timestamp("2026-10-23T04:47:34Z")
        );
        assert_eq!(parse_timestamp("1970-01-01T00:00:00-01:30"), Some(5_400));
        for s in [
            "",
            "2026-10-23T12:47:34",
            "2026-10-23T12:47:34+0800",
            "2026-10-23T12:47:34+24:00",
            "2026-10-23T12:47:34*08:00",
        ] {
            assert_eq!(parse_timestamp(s), None, "{s:?} should not parse");
        }
    }

    #[test]
    fn formats_utc_across_day_boundaries() {
        assert_eq!(
            fmt_utc("2026-10-24T01:00:00.000+08:00").as_deref(),
            Some("2026-10-23 17:00 UTC")
        );
        assert_eq!(
            fmt_utc("2024-02-29T23:59:00Z").as_deref(),
            Some("2024-02-29 23:59 UTC")
        );
        assert_eq!(
            fmt_utc("1970-01-01T00:00:00Z").as_deref(),
            Some("1970-01-01 00:00 UTC")
        );
    }
}
