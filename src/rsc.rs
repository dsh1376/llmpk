use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
                  AppleWebKit/537.36 (KHTML, like Gecko) \
                  Chrome/124.0.0.0 Safari/537.36";

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(20))
        .build()
        .expect("building reqwest client")
});

/// A non-2xx response. Kept as a type so retry logic can read the status
/// instead of parsing the message.
#[derive(Debug)]
pub struct HttpStatusError {
    pub status: reqwest::StatusCode,
    pub url: String,
}

impl std::fmt::Display for HttpStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Status first: the error view may only have room for one line.
        write!(f, "HTTP {} from {}", self.status, self.url)
    }
}

impl std::error::Error for HttpStatusError {}

pub fn fetch_text(url: &str) -> Result<String> {
    let resp = CLIENT
        .get(url)
        .send()
        .with_context(|| format!("requesting {url}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(HttpStatusError {
            status,
            url: url.to_string(),
        }
        .into());
    }
    resp.text().context("reading response body")
}

/// Fetch with retry for transient failures (see `is_transient`).
/// Retries up to 2 times with 1s/2s backoff.
pub fn fetch_text_retry(url: &str) -> Result<String> {
    let delays = [Duration::from_secs(1), Duration::from_secs(2)];
    let mut last_err = None;
    for attempt in 0..=delays.len() {
        match fetch_text(url) {
            Ok(body) => return Ok(body),
            Err(e) => {
                if attempt < delays.len() && is_transient(&e) {
                    std::thread::sleep(delays[attempt]);
                    last_err = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("fetch failed after retries")))
}

/// Server errors (any 5xx, including Cloudflare's 52x) and network faults are
/// worth a retry; client errors and parse failures are not.
fn is_transient(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        if let Some(http) = cause.downcast_ref::<HttpStatusError>() {
            return http.status.is_server_error();
        }
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|e| e.is_timeout() || e.is_connect() || e.is_request() || e.is_body())
    })
}

static RSC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"self\.__next_f\.push\(\[1,"((?:\\.|[^"\\])*)"\]\)"#).expect("compiling RSC regex")
});

pub fn extract_stream(html: &str) -> Result<String> {
    let mut joined = String::new();
    for cap in RSC_RE.captures_iter(html) {
        decode_into(&cap[1], &mut joined);
    }
    if joined.is_empty() {
        return Err(anyhow!(
            "could not find leaderboard data — the site may have changed its structure"
        ));
    }
    Ok(joined)
}

fn decode_into(src: &str, out: &mut String) {
    let mut chars = src.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('\\') => out.push('\\'),
            Some('/') => out.push('/'),
            Some('b') => out.push('\u{0008}'),
            Some('f') => out.push('\u{000c}'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Ok(n) = u32::from_str_radix(&hex, 16) {
                    // Handle UTF-16 surrogate pairs
                    if (0xD800..=0xDBFF).contains(&n) {
                        // High surrogate — look for \u low surrogate
                        if chars.next() == Some('\\') && chars.next() == Some('u') {
                            let low_hex: String = chars.by_ref().take(4).collect();
                            if let Ok(low) = u32::from_str_radix(&low_hex, 16) {
                                if (0xDC00..=0xDFFF).contains(&low) {
                                    let cp = 0x10000 + (n - 0xD800) * 0x400 + (low - 0xDC00);
                                    if let Some(ch) = char::from_u32(cp) {
                                        out.push(ch);
                                        continue;
                                    }
                                }
                            }
                        }
                        out.push('\u{FFFD}');
                        continue;
                    }
                    if let Some(ch) = char::from_u32(n) {
                        out.push(ch);
                        continue;
                    }
                }
                out.push('\u{FFFD}');
            }
            Some(other) => out.push(other),
            None => break,
        }
    }
}

/// Return the smallest balanced `{...}` substrings that contain `needle`.
pub fn innermost_objects_with<'a>(stream: &'a str, needle: &str) -> Vec<&'a str> {
    let bytes = stream.as_bytes();
    let mut starts: Vec<usize> = Vec::new();
    let mut closed: Vec<(usize, usize)> = Vec::new();
    let mut in_str = false;
    let mut escape = false;

    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => starts.push(i),
            b'}' => {
                if let Some(s) = starts.pop() {
                    closed.push((s, i));
                }
            }
            _ => {}
        }
    }

    // Filter to only those containing the needle, sorted smallest first.
    let mut matches: Vec<(usize, usize)> = closed
        .into_iter()
        .filter(|(s, e)| stream[*s..=*e].contains(needle))
        .collect();
    matches.sort_unstable_by_key(|(s, e)| e - s);

    // Keep only innermost: skip entries that contain a smaller already-chosen match.
    let mut chosen: Vec<(usize, usize)> = Vec::new();
    for (s, e) in matches {
        if chosen.iter().any(|&(cs, ce)| cs >= s && ce <= e) {
            continue;
        }
        chosen.push((s, e));
    }
    chosen.sort_unstable_by_key(|(s, _)| *s);
    chosen.iter().map(|(s, e)| &stream[*s..=*e]).collect()
}

/// Fetch HTML using a fresh client per call (old behavior).
#[cfg(test)]
pub fn fetch_text_new_client(url: &str) -> Result<String> {
    Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(20))
        .build()?
        .get(url)
        .send()
        .with_context(|| format!("requesting {url}"))?
        .error_for_status()
        .with_context(|| format!("non-2xx from {url}"))?
        .text()
        .context("reading response body")
}

/// How many references a resolved value may chain through before the rest
/// is left unresolved, so a reference cycle cannot loop forever.
const MAX_REFERENCE_DEPTH: usize = 8;

/// Replace React Server Components path references in `value` with the
/// values they point at in `stream`. A reference is a string such as
/// `"$32:props:children:props:rows:0:agentCreator"`: stream row `32`, then
/// that path, where `props`, `type`, and `key` step into a
/// `["$", type, key, props]` element. A reference that does not resolve
/// stays a string, so the typed parse that follows reports it.
pub fn resolve_references(value: &mut Value, stream: &str) {
    let mut rows = HashMap::new();
    resolve_in(value, stream, &mut rows, 0);
}

fn resolve_in(
    value: &mut Value,
    stream: &str,
    rows: &mut HashMap<String, Option<Value>>,
    depth: usize,
) {
    match value {
        Value::String(s) if depth < MAX_REFERENCE_DEPTH => {
            if let Some(mut target) = lookup_reference(s, stream, rows) {
                resolve_in(&mut target, stream, rows, depth + 1);
                *value = target;
            }
        }
        Value::Array(items) => {
            for item in items {
                resolve_in(item, stream, rows, depth);
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                resolve_in(item, stream, rows, depth);
            }
        }
        _ => {}
    }
}

fn lookup_reference(
    reference: &str,
    stream: &str,
    rows: &mut HashMap<String, Option<Value>>,
) -> Option<Value> {
    let (id, path) = reference.strip_prefix('$')?.split_once(':')?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let row = rows
        .entry(id.to_string())
        .or_insert_with(|| parse_stream_row(stream, id))
        .as_ref()?;
    let mut current = row;
    for segment in path.split(':') {
        current = step(current, segment)?;
    }
    Some(current.clone())
}

/// The JSON value of stream row `<id>:`, which starts the stream or a line.
fn parse_stream_row(stream: &str, id: &str) -> Option<Value> {
    let prefix = format!("{id}:");
    let start = if stream.starts_with(&prefix) {
        prefix.len()
    } else {
        stream.find(&format!("\n{prefix}"))? + prefix.len() + 1
    };
    serde_json::Deserializer::from_str(&stream[start..])
        .into_iter::<Value>()
        .next()?
        .ok()
}

fn step<'a>(value: &'a Value, segment: &str) -> Option<&'a Value> {
    match value {
        Value::Array(items) => {
            let element_field = match segment {
                "type" => Some(1),
                "key" => Some(2),
                "props" => Some(3),
                _ => None,
            };
            match element_field {
                Some(i) if items.first().and_then(Value::as_str) == Some("$") => items.get(i),
                _ => items.get(segment.parse::<usize>().ok()?),
            }
        }
        Value::Object(map) => map.get(segment),
        _ => None,
    }
}

/// Find the first balanced `[...]` array following the literal `"<key>":` in the stream.
pub fn first_array_after<'a>(stream: &'a str, key: &str) -> Option<&'a str> {
    balanced_after(stream, key, b'[', b']')
}

/// Find the first balanced `{...}` object following the literal `"<key>":` in the stream.
pub fn first_object_after<'a>(stream: &'a str, key: &str) -> Option<&'a str> {
    balanced_after(stream, key, b'{', b'}')
}

fn balanced_after<'a>(stream: &'a str, key: &str, open: u8, close: u8) -> Option<&'a str> {
    let needle = format!("\"{key}\":");
    let i = stream.find(&needle)?;
    let bytes = stream.as_bytes();
    let mut start = i + needle.len();
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    if start >= bytes.len() || bytes[start] != open {
        return None;
    }

    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut escape = false;
    for (j, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        if b == b'"' {
            in_str = true;
        } else if b == open {
            depth += 1;
        } else if b == close {
            depth -= 1;
            if depth == 0 {
                return Some(&stream[start..=j]);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_element_paths_and_chained_references() {
        let stream = "0:{}\n5:[\"$\",\"div\",null,{\"rows\":[{\"c\":{\"n\":\"A\"},\
                      \"refs\":[\"$5:props:rows:0:c\",\"$7:x\"]}]}]\n7:{\"x\":\"$5:props:rows:0:c:n\"}\n";
        let mut rows: Value = serde_json::from_str(
            r#"["$5:props:rows:0:refs", "$undefined", "$9:missing", "$5:nope"]"#,
        )
        .unwrap();
        resolve_references(&mut rows, stream);
        assert_eq!(
            rows,
            serde_json::json!([[{"n": "A"}, "A"], "$undefined", "$9:missing", "$5:nope"])
        );
    }

    #[test]
    fn reference_cycles_stop() {
        let stream = "1:{\"a\":\"$1:a\"}\n";
        let mut value = Value::String("$1:a".into());
        resolve_references(&mut value, stream);
        assert_eq!(value, Value::String("$1:a".into()));
    }

    #[test]
    fn decode_basic_escapes() {
        let mut out = String::new();
        decode_into(r#"hello\nworld\t!"#, &mut out);
        assert_eq!(out, "hello\nworld\t!");
    }

    #[test]
    fn decode_quote_and_backslash() {
        let mut out = String::new();
        decode_into(r#"say \"hi\" and \\"#, &mut out);
        assert_eq!(out, r#"say "hi" and \"#);
    }

    #[test]
    fn decode_unicode_bmp() {
        let mut out = String::new();
        decode_into(r#"A\u00e9"#, &mut out);
        assert_eq!(out, "Aé");
    }

    #[test]
    fn decode_utf16_surrogate_pair() {
        let mut out = String::new();
        // U+1F600 = 😀
        decode_into(r#"\uD83D\uDE00"#, &mut out);
        assert_eq!(out, "😀");
    }

    #[test]
    fn decode_high_surrogate_without_low() {
        let mut out = String::new();
        decode_into(r#"\uD800"#, &mut out);
        assert_eq!(out, "\u{FFFD}");
    }

    #[test]
    fn decode_invalid_hex() {
        let mut out = String::new();
        decode_into(r#"\uZZZZ"#, &mut out);
        assert_eq!(out, "\u{FFFD}");
    }

    #[test]
    fn decode_trailing_backslash() {
        let mut out = String::new();
        decode_into("hello\\", &mut out);
        assert_eq!(out, "hello");
    }

    #[test]
    fn innermost_finds_nested_object() {
        let stream = r#"{"outer":{"inner":{"target":1}}}"#;
        let results = innermost_objects_with(stream, "\"target\":");
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("\"target\":1"));
    }

    #[test]
    fn innermost_skips_containing_if_inner_exists() {
        let stream = r#"{"a":{"x":1},"b":{"x":2}}"#;
        let results = innermost_objects_with(stream, "\"x\":");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn innermost_handles_strings_with_braces() {
        let stream = r#"{"a":"{not a brace}","b":1}"#;
        let results = innermost_objects_with(stream, "\"b\":");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "{\"a\":\"{not a brace}\",\"b\":1}");
    }

    #[test]
    fn innermost_no_match() {
        let stream = r#"{"a":1}"#;
        let results = innermost_objects_with(stream, "\"missing\":");
        assert!(results.is_empty());
    }

    #[test]
    fn first_array_finds_key() {
        let stream = r#"garbage"rows":[{"id":1},{"id":2}]"#;
        let result = first_array_after(stream, "rows");
        assert_eq!(result, Some(r#"[{"id":1},{"id":2}]"#));
    }

    #[test]
    fn first_array_skips_whitespace() {
        let stream = r#"key: "rows":   [1, 2, 3] more"#;
        let result = first_array_after(stream, "rows");
        assert_eq!(result, Some("[1, 2, 3]"));
    }

    #[test]
    fn first_array_missing_key() {
        let stream = r#"{"data":[1]}"#;
        let result = first_array_after(stream, "rows");
        assert_eq!(result, None);
    }

    #[test]
    fn first_array_no_bracket_after_key() {
        let stream = r#"key: "rows": null"#;
        let result = first_array_after(stream, "rows");
        assert_eq!(result, None);
    }

    #[test]
    fn first_array_nested_arrays() {
        let stream = r#"data: "rows": [[1,2],[3,4]]"#;
        let result = first_array_after(stream, "rows");
        assert_eq!(result, Some("[[1,2],[3,4]]"));
    }

    #[test]
    fn first_object_ignores_braces_in_strings() {
        let stream = r#"x "meta": {"a":"}","b":{"c":1}} tail"#;
        let result = first_object_after(stream, "meta");
        assert_eq!(result, Some(r#"{"a":"}","b":{"c":1}}"#));
    }

    #[test]
    fn extract_stream_no_rsc_chunks() {
        let result = extract_stream("<html>no data here</html>");
        assert!(result.is_err());
    }

    #[test]
    fn extract_stream_decodes_chunks() {
        let html = r#"self.__next_f.push([1,"hello\nworld"])"#;
        let result = extract_stream(html).unwrap();
        assert_eq!(result, "hello\nworld");
    }

    const AA_URL: &str = "https://artificialanalysis.ai/leaderboards/models";

    /// Benchmark: fresh client per request (old) vs shared client (new).
    /// Run with: LLMPK_BENCH=1 cargo test bench_fetch -- --nocapture
    #[test]
    fn bench_fetch() {
        if std::env::var("LLMPK_BENCH").is_err() {
            return;
        }

        let urls = [AA_URL];
        let rounds = 3;

        println!("\n=== Fetch Benchmark ({rounds} rounds per URL) ===\n");

        for url in &urls {
            // Warm up the shared client
            let _ = fetch_text(url);

            // --- Shared client (new) ---
            let mut shared_times = Vec::new();
            for _ in 0..rounds {
                let t = std::time::Instant::now();
                let _ = fetch_text(url);
                shared_times.push(t.elapsed());
            }

            // --- Fresh client per request (old) ---
            let mut fresh_times = Vec::new();
            for _ in 0..rounds {
                let t = std::time::Instant::now();
                let _ = fetch_text_new_client(url);
                fresh_times.push(t.elapsed());
            }

            let shared_avg: Duration = shared_times.iter().sum::<Duration>() / rounds;
            let fresh_avg: Duration = fresh_times.iter().sum::<Duration>() / rounds;
            let speedup = fresh_avg.as_secs_f64() / shared_avg.as_secs_f64();

            println!("URL: {url}");
            println!("  Fresh client : {fresh_times:?}  avg={fresh_avg:.0?}");
            println!("  Shared client: {shared_times:?}  avg={shared_avg:.0?}");
            println!("  Speedup      : {speedup:.2}x");
            println!();
        }
    }

    /// Benchmark: regex compiled per call (old) vs static regex (new).
    /// Run with: LLMPK_BENCH=1 cargo test bench_regex -- --nocapture
    #[test]
    #[allow(clippy::regex_creation_in_loops)]
    fn bench_regex() {
        if std::env::var("LLMPK_BENCH").is_err() {
            return;
        }

        let html = match std::env::var("LLMPK_LEADERBOARD_FIXTURE") {
            Ok(path) => std::fs::read_to_string(&path).expect("fixture read"),
            Err(_) => {
                println!("skipping bench_regex (set LLMPK_LEADERBOARD_FIXTURE)");
                return;
            }
        };

        let rounds = 10;

        // Static regex (new)
        let mut static_times = Vec::new();
        for _ in 0..rounds {
            let t = std::time::Instant::now();
            let _ = extract_stream(&html);
            static_times.push(t.elapsed());
        }

        // Regex compiled per call (old)
        let re = Regex::new(r#"self\.__next_f\.push\(\[1,"((?:\\.|[^"\\])*)"\]\)"#).unwrap();
        let mut compiled_times = Vec::new();
        for _ in 0..rounds {
            let t = std::time::Instant::now();
            let mut joined = String::new();
            for cap in re.captures_iter(&html) {
                decode_into(&cap[1], &mut joined);
            }
            let _ = joined;
            compiled_times.push(t.elapsed());
        }

        // Regex recompiled per call (old)
        let mut recompile_times = Vec::new();
        for _ in 0..rounds {
            let re = Regex::new(r#"self\.__next_f\.push\(\[1,"((?:\\.|[^"\\])*)"\]\)"#).unwrap();
            let t = std::time::Instant::now();
            let mut joined = String::new();
            for cap in re.captures_iter(&html) {
                decode_into(&cap[1], &mut joined);
            }
            let _ = joined;
            recompile_times.push(t.elapsed());
        }

        let static_avg: Duration = static_times.iter().sum::<Duration>() / rounds;
        let compiled_avg: Duration = compiled_times.iter().sum::<Duration>() / rounds;
        let recompile_avg: Duration = recompile_times.iter().sum::<Duration>() / rounds;

        println!("\n=== Regex Benchmark ({rounds} rounds) ===\n");
        println!("  Recompile each call: {recompile_times:?}  avg={recompile_avg:.0?}");
        println!("  Pre-compiled       : {compiled_times:?}  avg={compiled_avg:.0?}");
        println!("  Static (LazyLock)  : {static_times:?}  avg={static_avg:.0?}");
        println!(
            "  Compile overhead   : {:.2}x vs static",
            recompile_avg.as_secs_f64() / static_avg.as_secs_f64()
        );
    }

    /// Serve one canned HTTP response on a loopback port, then close.
    fn serve_once(response: &'static str) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{addr}/")
    }

    fn status_response(status: &'static str) -> &'static str {
        match status {
            "503" => {
                "HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
            }
            "530" => {
                "HTTP/1.1 530 Origin Unreachable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
            }
            "404" => "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            _ => unreachable!(),
        }
    }

    #[test]
    fn server_errors_are_transient() {
        for code in ["503", "530"] {
            let err = fetch_text(&serve_once(status_response(code))).unwrap_err();
            assert!(is_transient(&err), "{code}: {err:#}");
            assert!(format!("{err:#}").starts_with(&format!("HTTP {code}")));
        }
    }

    #[test]
    fn client_errors_are_not_transient() {
        let err = fetch_text(&serve_once(status_response("404"))).unwrap_err();
        assert!(!is_transient(&err), "{err:#}");
    }

    #[test]
    fn dropped_connections_are_transient() {
        // The server closes without answering.
        let err = fetch_text(&serve_once("")).unwrap_err();
        assert!(is_transient(&err), "{err:#}");
    }

    #[test]
    fn parse_errors_are_not_transient() {
        assert!(!is_transient(&anyhow!(
            "no model rows found (timed out 503)"
        )));
    }
}
