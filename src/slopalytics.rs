use std::sync::LazyLock;

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::Deserialize;

use crate::rsc;

const SITE: &str = "https://slopalytics.com";

/// One model at one reasoning effort, as charted on slopalytics.com. The
/// numbers come from Artificial Analysis's intelligence-index runs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub lab_name: Option<String>,
    #[serde(default)]
    pub released: Option<String>,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub effort_label: Option<String>,
    /// AA estimated the intelligence index instead of measuring it.
    #[serde(default)]
    pub estimated: bool,
    #[serde(default)]
    pub intelligence: Option<f64>,
    /// Mean USD per index task.
    #[serde(default)]
    pub cost: Option<f64>,
    /// USD for the full intelligence-index run.
    #[serde(default)]
    pub cost_total: Option<f64>,
    /// Mean output tokens per index task.
    #[serde(default)]
    pub tokens: Option<f64>,
    /// Output tokens for the full intelligence-index run.
    #[serde(default)]
    pub tokens_total: Option<f64>,
    /// Output tokens per second.
    #[serde(default)]
    pub speed: Option<f64>,
    /// Mean seconds per index task.
    #[serde(default)]
    pub time: Option<f64>,
    #[serde(default)]
    pub input_price: Option<f64>,
    #[serde(default)]
    pub output_price: Option<f64>,
    #[serde(default)]
    pub context: Option<f64>,
}

impl Row {
    pub fn lab(&self) -> &str {
        self.lab_name
            .as_deref()
            .filter(|l| !l.trim().is_empty())
            .unwrap_or("?")
    }

    /// Effort as the site labels it ("max", "low", "Default"), or `None` for
    /// models without reasoning modes.
    pub fn effort(&self) -> Option<&str> {
        self.effort_label
            .as_deref()
            .filter(|e| !e.trim().is_empty() && !e.eq_ignore_ascii_case("non-reasoning"))
    }

    pub fn display_name(&self) -> String {
        match self.effort() {
            Some(effort) => format!("{} [{effort}]", self.name),
            None => self.name.clone(),
        }
    }
}

pub fn fetch() -> Result<Vec<Row>> {
    let html = rsc::fetch_text_retry(&format!("{SITE}/"))?;
    let path =
        bundle_path(&html).ok_or_else(|| anyhow!("no app bundle linked from slopalytics.com"))?;
    let bundle = rsc::fetch_text_retry(&format!("{SITE}{path}"))?;
    parse(&bundle)
}

static BUNDLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"src="(/assets/index-[A-Za-z0-9_-]+\.js)""#).expect("compiling bundle regex")
});

/// The Vite entry script, for example `/assets/index-7H6vCIWV.js`. The hash
/// changes on every deploy, so it is read from the page each time.
fn bundle_path(html: &str) -> Option<&str> {
    BUNDLE_RE
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
}

/// Parse the model table the site compiles into its JS bundle. Deprecated
/// models are dropped, as on the AA board.
pub fn parse(bundle: &str) -> Result<Vec<Row>> {
    let literal = models_array(bundle)
        .ok_or_else(|| anyhow!("no model table found in the slopalytics.com bundle"))?;
    let json = js_literal_to_json(literal).context("reading the slopalytics.com model table")?;
    let rows: Vec<Row> = serde_json::from_str(&json)
        .map_err(|e| anyhow!("failed to parse the slopalytics.com model table: {e}"))?;
    let rows: Vec<Row> = rows
        .into_iter()
        .filter(|r| !r.deprecated && !r.id.is_empty())
        .collect();
    if rows.is_empty() {
        return Err(anyhow!("slopalytics.com model table is empty"));
    }
    Ok(rows)
}

/// The balanced `[...]` after `models:[{id:` in the bundle. Strings in the
/// bundle are template literals, so backticks delimit them.
fn models_array(bundle: &str) -> Option<&str> {
    let start = bundle.find("models:[{id:")? + "models:".len();
    let bytes = bundle.as_bytes();
    let mut depth = 0usize;
    let mut quote: Option<u8> = None;
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(_) if b == b'\\' => i += 1,
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None => match b {
                b'`' | b'"' | b'\'' => quote = Some(b),
                b'[' | b'{' => depth += 1,
                b']' | b'}' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(&bundle[start..=i]);
                    }
                }
                _ => {}
            },
        }
        i += 1;
    }
    None
}

/// Convert the minified object-literal subset the bundle uses into JSON:
/// bare keys, template-literal and quoted strings, `!0`/`!1` booleans,
/// `void 0`, and numbers such as `.55`, `-.5`, or `2e5`. Anything else, such
/// as an interpolated `${...}` template, is an error rather than a guess.
fn js_literal_to_json(src: &str) -> Result<String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len() + src.len() / 4);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '`' | '"' | '\'' => {
                let (text, next) = read_string(&chars, i)?;
                out.push_str(&serde_json::to_string(&text)?);
                i = next;
            }
            '!' if matches!(chars.get(i + 1), Some('0' | '1')) => {
                out.push_str(if chars[i + 1] == '0' { "true" } else { "false" });
                i += 2;
            }
            c if c.is_ascii_alphabetic() || c == '_' || c == '$' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '_' | '$'))
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let is_key = chars[i..].iter().find(|c| !c.is_whitespace()) == Some(&':');
                if is_key {
                    out.push_str(&serde_json::to_string(&word)?);
                } else if word == "void" {
                    // `void 0` is `undefined`; the operand is skipped below.
                    out.push_str("null");
                    while i < chars.len() && (chars[i].is_whitespace() || chars[i] == '0') {
                        i += 1;
                    }
                } else if matches!(word.as_str(), "null" | "true" | "false") {
                    out.push_str(&word);
                } else {
                    return Err(anyhow!("unsupported identifier `{word}` in model table"));
                }
            }
            '-' | '.' | '0'..='9' => {
                let start = i;
                if chars[i] == '-' {
                    out.push('-');
                    i += 1;
                }
                if chars.get(i) == Some(&'.') {
                    out.push('0');
                }
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || matches!(chars[i], '.' | 'e' | 'E' | '+' | '-'))
                {
                    // A '-' only continues a number right after an exponent.
                    if chars[i] == '-' && !matches!(chars[i - 1], 'e' | 'E') {
                        break;
                    }
                    out.push(chars[i]);
                    i += 1;
                }
                if i == start + 1 && chars[start] == '-' {
                    return Err(anyhow!("stray '-' in model table"));
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Ok(out)
}

/// Read a string literal starting at `chars[start]`; return its text and the
/// index after the closing quote.
fn read_string(chars: &[char], start: usize) -> Result<(String, usize)> {
    let quote = chars[start];
    let mut text = String::new();
    let mut i = start + 1;
    while i < chars.len() {
        let c = chars[i];
        if c == quote {
            return Ok((text, i + 1));
        }
        if quote == '`' && c == '$' && chars.get(i + 1) == Some(&'{') {
            return Err(anyhow!("interpolated template string in model table"));
        }
        if c == '\\' {
            let escaped = *chars
                .get(i + 1)
                .ok_or_else(|| anyhow!("unterminated escape in model table"))?;
            text.push(match escaped {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                other => other,
            });
            i += 2;
            continue;
        }
        text.push(c);
        i += 1;
    }
    Err(anyhow!("unterminated string in model table"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/slopalytics_bundle.js");

    #[test]
    fn live_fetch_when_enabled() {
        if std::env::var("LLMPK_LIVE").is_err() {
            return;
        }
        let rows = fetch().expect("live slopalytics fetch");
        assert!(rows.len() > 100, "got {} rows", rows.len());
        assert!(rows.iter().filter(|r| r.cost.is_some()).count() > 50);
    }

    #[test]
    fn finds_the_hashed_bundle() {
        let html = r#"<script type="module" crossorigin src="/assets/index-7H6vCIWV.js"></script>"#;
        assert_eq!(bundle_path(html), Some("/assets/index-7H6vCIWV.js"));
        assert_eq!(bundle_path("<html></html>"), None);
    }

    #[test]
    fn parses_committed_fixture_and_drops_deprecated_rows() {
        let rows = parse(FIXTURE).expect("parse committed fixture");
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "claude-opus-5-5",
                "claude-opus-5-5-low",
                "apertus-70b-instruct"
            ]
        );

        let max = &rows[0];
        assert_eq!(max.display_name(), "Claude Opus 5.5 [max]");
        assert_eq!(max.lab(), "Anthropic");
        assert_eq!(max.released.as_deref(), Some("2026-09-22"));
        assert!(!max.estimated);
        assert_eq!(max.intelligence, Some(57.6223698102963));
        assert_eq!(max.cost, Some(5.982012019521066));
        assert_eq!(max.tokens_total, Some(259_888_331.0));
        assert_eq!(max.time, Some(803.9885317972685));
        assert_eq!(max.context, Some(1e6));

        // Leading-dot numbers.
        assert_eq!(rows[1].cost, Some(0.551180473909146));

        // Non-reasoning model with no index-run numbers.
        let apertus = &rows[2];
        assert_eq!(apertus.effort(), None);
        assert_eq!(apertus.display_name(), "Apertus 70B Instruct");
        assert!(apertus.estimated);
        assert_eq!(apertus.cost, None);
        assert_eq!(apertus.input_price, Some(0.82));
    }

    #[test]
    fn converts_the_bundle_literal_subset() {
        let json = js_literal_to_json(
            "[{a:`x \"y\"`,b:!0,c:!1,d:.5,e:-.25,f:2e5,g:1e-3,h:null,i:void 0,j:-3}]",
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v[0]["a"], "x \"y\"");
        assert_eq!(v[0]["b"], true);
        assert_eq!(v[0]["c"], false);
        assert_eq!(v[0]["d"], 0.5);
        assert_eq!(v[0]["e"], -0.25);
        assert_eq!(v[0]["f"], 200000.0);
        assert_eq!(v[0]["g"], 0.001);
        assert!(v[0]["h"].is_null());
        assert!(v[0]["i"].is_null());
        assert_eq!(v[0]["j"], -3);
    }

    #[test]
    fn rejects_code_it_cannot_read_safely() {
        assert!(js_literal_to_json("[{a:`${x}`}]").is_err());
        assert!(js_literal_to_json("[{a:someVar}]").is_err());
        assert!(js_literal_to_json("[{a:`open").is_err());
        assert!(parse("var x={models:[]};").is_err());
        assert!(parse("no table here").is_err());
    }
}
