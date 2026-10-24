# llmpk

[![CI](https://github.com/dsh1376/llmpk/actions/workflows/ci.yml/badge.svg)](https://github.com/dsh1376/llmpk/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/dsh1376/llmpk)](https://github.com/dsh1376/llmpk/releases/latest)

A terminal TUI for browsing LLM and coding-agent leaderboards from one keyboard-driven interface.

> **Note:** This is a personal vibe coding project using [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Built for my own use to quickly compare LLM models and coding agents without opening a browser. Expect rough edges.

No API keys. No headless browser. No JavaScript runtime. Just HTTP and regex.

```
┌ llmpk v0.6.1 ────────────────────────────────────────────────────────────────────────────────────┐
│ 1 AA   2 AA Agents   3 Slopalytics   4 FrontierCode   5 AI Tracker                               │
│artificialanalysis.ai/leaderboards/models  |  272 models  |  view: table  |  sort: Intelligence ↓ │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ AA (272) ────────────────────────────────────────────────────────────────────────────────────────┐
│  #   Model                                 Provider    Intel  t/s    $/M     Cache  Ctx     Open │
│>  1  Claude Opus 5.5 (Max, Default Fallbac Anthropic   57.6   96     $2.94   95%    1M           │
│   2  Claude Sonnet 5.5 (Max, Default Fallb Anthropic   56.0   133    $1.47   95%    1M           │
│   3  Claude Opus 5.5 (Xhigh, Default Fallb Anthropic   56.0   84     $2.94   95%    1M           │
│   4  Muse Wonder Full (Max)                Meta        55.8   -      -       -      1M           │
│   5  Claude Opus 5.5 (High, Default Fallba Anthropic   53.6   78     $2.94   95%    1M           │
│   6  Claude Fable 5.1 (Max, Default Fallba Anthropic   53.4   68     $7.17   98%    1M           │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
? / h help  q quit  [ ] board  j/k row  g/G top/bot  r reload  y copy  1/272
i/s/p/d/c sort  o asc/desc  m chart  / filter
```

## Features

- **8 boards** — Artificial Analysis model rankings, Artificial Analysis coding-agent rankings, Slopalytics cost and time per task by reasoning effort, the FrontierCode mergeability benchmark, the AI Tracker feed of new model releases and leaks, and three AIHOT boards: the hot-events list, the latest daily report, and a calendar of Codex usage resets
- **Background fetching** — every board starts loading on its own thread at launch; `r` refetches the current board without blocking the UI, and quitting never waits on a slow source
- **AA price economics** — input / output / cache-hit / cache-write prices from AA, plus the 7:2:1 cache-hit:input:output blend, the no-cache 3:1 blend, and the cache discount derived from them; prices carry banded color and sub-cent values render as `<$0.01/M` instead of rounding to `$0.00`
- **Release and leak feed** — the AI Tracker board lists each tracked model with its maker, stage (release in green, leak in yellow), age since last sighting, and a link to its evidence page
- **AIHOT news and Codex resets** — the AIHOT Hot and Daily boards show AIHOT's Chinese titles and summaries with source counts and links; the Codex Resets board opens on a month calendar of every usage reset (`●`) and reset credit (`◆`) that Tibo announced on X, colored by status (confirmed in green, likely landed in yellow, announced in magenta), with 90-day counts, the median gap between resets, and the selected event's scope, expected time in UTC, and English post text; `j`/`k` move between events and `m` switches to the table. The full history is cached for 10 minutes in `~/.cache/llmpk/` (or `$XDG_CACHE_HOME/llmpk/`), because the API allows one full read per 10 minutes
- **Table and full-width horizontal chart views** — toggle with `m` (the AI Tracker, AIHOT Hot, and AIHOT Daily boards are table-only)
- **Provider-aware colors** — AA provider colors and agent creator/provider accents carry into tables and charts
- **Per-board filtering** — type `/` to filter; filters are cached and scoped to the active board
- **Responsive layout** — columns appear as the terminal widens (provider from 62 columns, cache from 70, context from 74, speed from 84, license from 98, release date from 110); the selected row gets a side detail pane once the terminal is wide enough for that board
- **Metric sorting** — missing values stay last; cost-like metrics default to ascending when selected
- **Compact mode** — `z` removes column spacing for narrow terminals
- **Mouse support** — click a tab to switch boards, a column header to sort by it (click again to flip the direction), or a row to select it; the wheel scrolls the table
- **Scrollbars** — long tables and the help overlay show a scrollbar on their right border; click or drag it to jump through the table
- **Row position indicator** — table view footer shows selected row / visible row count
- **OSC 52 copy** — press `y` to copy the selected model or agent label, news title, or reset post URL through terminal clipboard support

## Installation

### Prebuilt binaries

```sh
curl -fsSL https://github.com/dsh1376/llmpk/releases/latest/download/install.sh | bash
```

Prebuilt for macOS Apple Silicon (`aarch64-apple-darwin`) and Linux x86_64 (`x86_64-unknown-linux-gnu`). Set `LLMPK_INSTALL_DIR` to override the destination directory.

### From source

```sh
cargo install --git https://github.com/dsh1376/llmpk.git
```

Requires Rust 1.88+ (`rust-version` in `Cargo.toml`; CI checks the floor).

## Usage

### Navigation

| Key | Action |
|-----|--------|
| `q`, `Esc`, `Ctrl-C` | Quit |
| `[` / `]` | Previous / next board |
| `1` - `8` | Jump to AA / AA Agents / Slopalytics / FrontierCode / AI Tracker / AIHOT Hot / AIHOT Daily / Codex Resets |
| `r` | Reload current board |
| `y` | Copy selected row label to clipboard (OSC 52) |
| `↑` / `↓` (or `k` / `j`) | Move selected row |
| `PgUp` / `PgDn` | Move by 10 rows |
| `Home` / `End` (or `g` / `G`) | Jump to first / last row |
| `?` or `h` | Toggle in-app help (lists every keybinding and explains each metric; scroll it with `j`/`k`, `PgUp`/`PgDn`, or the mouse wheel) |
| Mouse click | Tab: switch board. Column header: sort by it. Row: select it. Scrollbar: jump there (drag to scrub) |
| Mouse scroll | Scroll the table |

### Sorting

| Key | AA | AA Agents | Slopalytics | FrontierCode | AI Tracker | AIHOT Hot | AIHOT Daily | Codex Resets |
|-----|----|-----------|-------------|--------------|------------|-----------|-------------|--------------|
| `i` | Intelligence | Index | Intelligence | Score | — | Rank | Report order | — |
| `a` | — | Pass@1 | — | Pass rate | — | — | — | — |
| `s` | Speed | Turns | Speed | — | — | — | — | — |
| `p` | Price | Cost | Cost per task | Cost per run | — | — | — | — |
| `t` | — | Time | Time per task | Time per run | Last seen | — | — | Announced |
| `u` | — | Tokens | Tokens per task | Tokens per run | — | — | — | — |
| `c` | Context | — | — | — | — | — | — | — |
| `d` | Cache discount | — | — | — | — | — | — | — |
| `o` | Toggle sort direction | Toggle sort direction | Toggle sort direction | Toggle sort direction | Toggle sort direction | Toggle sort direction | Toggle sort direction | Toggle sort direction |

Clicking a column header does the same as pressing its key.

### View and filter

| Key | Action |
|-----|--------|
| `m` | Toggle table / chart view (boards 1-4), or calendar / table on Codex Resets |
| `z` | Toggle compact mode (tighter column spacing) |
| `/` | Edit filter for current board |
| `Backspace` | Delete filter character |
| `Enter` / `Esc` | Finish editing filter |
| `Ctrl-U` | Clear filter |

## Data sources

| Board | Source | Metrics |
|-------|--------|---------|
| AA | [artificialanalysis.ai/leaderboards/models](https://artificialanalysis.ai/leaderboards/models) | Intelligence index, output speed (t/s), input / output / cache hit / cache write prices, context window, release date; derived blended price ($/M tokens at 7:2:1 cache-hit:input:output, plus the no-cache 3:1 blend) and cache discount (%) |
| AA Agents | [artificialanalysis.ai/agents/coding-agents](https://artificialanalysis.ai/agents/coding-agents) | Coding Agent Index, Pass@1, mean cost, mean execution time, mean tokens, mean turns |
| Slopalytics | [slopalytics.com](https://slopalytics.com/) | One row per model and reasoning effort: AA intelligence index (`~` marks AA estimates), cost, time, and output tokens per index task, speed, full index-run cost and tokens, prices, context |
| FrontierCode | [cognition.com/frontiercode](https://cognition.com/frontiercode) | Mergeability score, pass rate, flag rate, cost, output tokens, and time per run at each model's best effort, plus the score at every evaluated effort |
| AI Tracker | [live.aitracker.bot](https://live.aitracker.bot/) ([JSON API](https://live.aitracker.bot/developers)) | Tracked new models: maker, stage (release / leak), last-seen time, evidence post and event counts, page link |
| AIHOT Hot | [aihot.news/hot](https://aihot.news/hot) ([JSON API](https://aihot.news/api/v1/hot-topics)) | Top 10 AI events of the last 48 hours: rank, title, source / signal / participant counts, latest report time, sample of covering sources, links |
| AIHOT Daily | [aihot.news/daily](https://aihot.news/daily) ([JSON API](https://aihot.news/api/v1/dailies/latest)) | Latest daily report: section, title, summary, source, links; flashes with their publish time |
| Codex Resets | [aihot.news/codex-reset](https://aihot.news/codex-reset) ([JSON API](https://aihot.news/api/v1/codex-resets)) | Full history of Codex usage resets and reset credits, as a calendar: status, scope, announced / expected / confirmed time (UTC), English X posts |

## How it works

Artificial Analysis is a Next.js application. It embeds leaderboard data in the HTML stream via React Server Components (`self.__next_f.push([1, "..."])` calls).

llmpk:

1. Fetches each page with a shared `reqwest` client (connection pooling, TLS session reuse)
2. Extracts all RSC push chunks with a regex
3. Decodes JS string escapes and concatenates into a single stream
4. Parses AA model rows from `/leaderboards/models` by scanning for balanced JSON objects containing `contextWindowTokens`, one row per slug; the homepage carries only a curated subset with no prices
5. Derives AA's blended prices and cache discount from the raw input, output, and cache-hit prices, and joins release dates through the page's `modelsAndReleases` table
6. Parses AA Agents rows by scanning for the first balanced array after `rows`
7. Reads the Slopalytics model table from the site's JavaScript bundle (found through the hashed `/assets/index-*.js` link) and converts its object literals to JSON
8. Fetches FrontierCode, AI Tracker, and AIHOT data directly from their JSON endpoints; for FrontierCode it keeps the newest benchmark revision and each model's best-scoring effort, as the site does
9. Preserves provider metadata where available so table and chart colors stay tied to the underlying model or agent creator

This is inherently fragile: if a site changes its markup, the scraper breaks. That is deliberate — breakage shows up as an error banner on the board instead of being papered over. Retries cover transient network faults only (timeout, reset, 5xx), never parsing.

## Building

```sh
cargo build --release
```

The release profile uses thin LTO, single codegen unit, and symbol stripping for a small, fast binary.

Run from source with:

```sh
cargo run --release
```

### Testing

`cargo test` covers the parsers against the trimmed captures in `tests/fixtures/` — no network, no environment variables:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Tests gated behind environment variables run against a **full** captured page. Point them at one saved from the live site; the committed fixtures are trimmed to a handful of rows and intentionally do not satisfy their row-count assertions.

```sh
# Fixture-based tests (expect a complete page, not the trimmed fixture)
LLMPK_LEADERBOARD_FIXTURE=path/to/leaderboards-models.html cargo test
LLMPK_CODING_AGENTS_FIXTURE=path/to/coding-agents.html cargo test

# Live network tests
LLMPK_LIVE=1 cargo test live_fetch

# Benchmarks
LLMPK_BENCH=1 cargo test bench_fetch -- --nocapture
LLMPK_LEADERBOARD_FIXTURE=path/to/leaderboards-models.html LLMPK_BENCH=1 cargo test bench_regex -- --nocapture
```

## Project structure

```
src/
  main.rs           Entry point, terminal setup/teardown, event loop, fetch dispatch
  rsc.rs            HTTP client, RSC stream extraction, brace/bracket scanners
  aa.rs             artificialanalysis.ai parser
  coding_agents.rs  artificialanalysis.ai coding agents parser
  slopalytics.rs    slopalytics.com parser (JS bundle object literals to JSON)
  frontiercode.rs   cognition.com FrontierCode parser (JSON, best effort per model)
  aitracker.rs      live.aitracker.bot model-pages parser (JSON API, UTC timestamps)
  aihot.rs          aihot.news hot topics, daily report, and Codex reset parsers (JSON API)
  board.rs          Board enum, Data/Status wrappers, fetch dispatch
  ui.rs             AppState, sort/filter state, render dispatch, shared helpers
  ui/aa_board.rs    AA table/detail/chart rendering
  ui/agents.rs      Agents table/chart rendering, brand colors
  ui/slop_board.rs     Slopalytics table/detail/chart rendering, lab colors
  ui/frontier_board.rs FrontierCode table/detail/chart rendering, per-effort scores
  ui/tracker_board.rs  AI Tracker table/detail rendering, stage colors, ages
  ui/hot_board.rs      AIHOT Hot table/detail rendering
  ui/daily_board.rs    AIHOT Daily table/detail rendering, section colors
  ui/reset_board.rs    Codex Resets calendar/table/detail rendering, status colors
  ui/chart.rs       Shared horizontal chart infrastructure
  ui/filter.rs      Per-board filter matching and filter cache helpers
  ui/sort.rs        Sort keys, sort direction, and missing-value ordering
  ui/state.rs       Runtime app state, selection, filters, views, and sorting
  ui/chrome.rs      Tabs, header, footer, help overlay, loading/error screens
  ui/test_helpers.rs   Test-only row constructors
tests/fixtures/     Trimmed captures for every board used by parser tests
```

## License

MIT © Dsh
