# llmpk

[![CI](https://github.com/dsh1376/llmpk/actions/workflows/ci.yml/badge.svg)](https://github.com/dsh1376/llmpk/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/dsh1376/llmpk)](https://github.com/dsh1376/llmpk/releases/latest)

Browse LLM leaderboards and AI news in your terminal. Written in TypeScript with Bun, React and Ink. Ink provides Yoga for terminal flexbox layout.

> **Note:** This is a personal vibe coding project using [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Built for my own use to quickly compare LLM models without opening a browser. Expect rough edges.

No API keys or headless browser. All seven sources load concurrently over HTTP.

## Features

- Seven boards with separate filters, sort order, selection and view state.
- Adaptive tables, provider colors, small-square meter charts and a UTC reset calendar with a 90-day summary. The active sort metric stays visible as columns narrow; wide tables add an inline meter beside it. Tracker stages, daily sections and reset statuses have colored text columns.
- Selected-row details beside the table at 116 columns and 16 rows, in metric, detail, summary and link sections with the row's rank. `Enter` opens scrollable full details at any supported size, including summaries, omitted metrics and source URLs.
- Search matches every word across row names, providers and details. Sorting keeps missing values last and preserves the selected row. Charts scroll with the selection.
- Reloads retain existing rows while fetching. Per-board errors stay visible; quitting aborts outstanding requests.
- Mouse tabs, sortable headers, row selection, wheel scrolling and scrollbar click/drag. `--no-mouse` keeps normal terminal text selection.
- OSC 52 clipboard support, scrollable help, compact columns and terminal resize handling down to 36 columns by 8 rows.
- Offline fixture previews and single-frame snapshots for inspection without an interactive terminal.

## Installation

### From source

Requires [Bun 1.4.0 or later](https://bun.sh/docs/installation).

```sh
git clone https://github.com/dsh1376/llmpk.git
cd llmpk
bun install --frozen-lockfile
bun run start
```

Build a standalone executable, which includes Bun and Yoga:

```sh
bun run build
./dist/llmpk
```

On macOS, the build script applies an ad hoc signature to the generated executable. No certificate or keychain access is required.

### Prebuilt binaries

For a published release:

```sh
curl -fsSL https://github.com/dsh1376/llmpk/releases/latest/download/install.sh | bash
```

The release workflow builds macOS Apple Silicon (`aarch64-apple-darwin`) and Linux x86_64 (`x86_64-unknown-linux-gnu`) archives. Set `LLMPK_INSTALL_DIR` to choose the install directory, or `LLMPK_VERSION` to select a release tag. Existing published releases can still contain an earlier implementation; use a source checkout to run this TypeScript rewrite before its release.

## Usage

```sh
bun run start --board frontier
bun run start --no-mouse

# Offline preview using the committed, trimmed source captures
bun run start --demo

# Print one frame and exit; no network or cache writes with --demo
bun run start --demo --snapshot --board resets --width 80 --height 24

# Fetch and print only the selected board
bun run start --snapshot --board aa
```

The standalone executable accepts the same options. `--help` lists CLI options; `?` opens help inside the TUI. Dates and times use UTC. A dash means the source did not provide a value.

### Navigation

| Key | Action |
|-----|--------|
| `q`, `Ctrl-C` | Quit |
| `Esc` | Close details/help or finish search; otherwise quit |
| `[` / `]`, `Shift-Tab` / `Tab` | Previous / next board |
| `1` through `7` | Select a board in the order below |
| `j` / `k`, `↓` / `↑` | Select a row; scroll when details/help is open |
| `PgUp` / `PgDn` | Move 10 rows or scroll 10 lines |
| `g` / `G`, `Home` / `End` | First / last row, or start / end of an open panel |
| `Enter` | Open full details; press again to close |
| `r` | Reload the current board |
| `?`, `h` | Toggle help |
| `y` | Copy the selected label; on Resets, copy the post URL |
| `Y` | Copy the selected row's source URL |
| Mouse | Click tabs, headers and rows; wheel to scroll; click or drag scrollbars |

### Sorting

Press a metric key to sort. Press it again to reverse. Cost, time, tokens and flag rate start ascending; quality, speed, context and cache discount start descending. `o` reverses the current sort. Clicking a numeric column header also sorts it, including columns without a shortcut.

| Key | AA | FrontierCode | Slopalytics | AI Tracker | AIHOT Hot | AIHOT Daily | Codex Resets |
|-----|----|--------------|-------------|------------|-----------|-------------|--------------|
| `i` | Intelligence | Score | Intelligence | - | Rank | Report order | - |
| `a` | - | Pass rate | - | - | - | - | - |
| `s` | Speed | - | Speed | - | - | - | - |
| `p` | Price | Cost / rollout | Cost/task | - | - | - | - |
| `t` | - | - | Time/task | Last seen | - | - | Announced |
| `u` | - | Output tokens | Tokens/task | - | - | - | - |
| `c` | Context | - | - | - | - | - | - |
| `d` | Cache discount | - | - | - | - | - | - |
| `f` | - | Flag rate | - | - | - | - | - |

### View and filter

| Key | Action |
|-----|--------|
| `m` | Table / chart on boards 1-3; calendar / table on Resets |
| `z` | Toggle compact column spacing |
| `/` | Edit the active board's filter |
| `Backspace` | Delete a filter character |
| `Enter`, `Esc` | Finish editing and keep the filter |
| `Ctrl-U` | Clear the filter while editing |

Tracker, Hot and Daily use tables. On narrow terminals, inactive tabs shorten to their numbers so all seven stay visible. The tab bar shows source health, and the board title line shows status, row count, sort and view. Full details remain available with `Enter` when the side pane is hidden.

## Data sources

| # | Board | Source | Data |
|---|-------|--------|------|
| 1 | AA | [Artificial Analysis models](https://artificialanalysis.ai/leaderboards/models) | Intelligence, speed, input/output/cache prices, context, release date and weights |
| 2 | FrontierCode | [cognition.com/frontiercode](https://cognition.com/frontiercode) | FrontierCode 2.0 leaderboard columns (harness, score, pass rate, flag rate, cost per rollout, output tokens) at the best reasoning mode, plus every evaluated effort |
| 3 | Slopalytics | [slopalytics.com](https://slopalytics.com/) | Intelligence and cost/time/output tokens per task, reasoning effort, full-run totals, token prices and context |
| 4 | AI Tracker | [live.aitracker.bot](https://live.aitracker.bot/) ([API](https://live.aitracker.bot/developers)) | Releases and leaks, maker, last sighting, evidence counts and page URL |
| 5 | AIHOT Hot | [aihot.news/hot](https://aihot.news/hot) ([API](https://aihot.news/api/v1/hot-topics)) | Top 10 events by rank, Chinese titles, source/signal/participant counts and links |
| 6 | AIHOT Daily | [aihot.news/daily](https://aihot.news/daily) ([API](https://aihot.news/api/v1/dailies/latest)) | Sections in report order, then flashes; Chinese titles, summaries, sources and links |
| 7 | Codex Resets | [aihot.news/codex-reset](https://aihot.news/codex-reset) ([API](https://aihot.news/api/v1/codex-resets)) | Usage resets and credits, presentation status, scope, UTC times and English X posts |

AA's cached price is `(7 × cache hit + 2 × input + output) / 10`, in USD per million tokens. The uncached blend is `(3 × input + output) / 4`; cache discount is `1 - cache hit / input`. Missing component prices leave the result unavailable. Positive prices below one cent display as `<$0.01`.

Codex Resets opens on a calendar. `●` marks a reset, `◆` a credit, and `+` multiple events that day. Green means confirmed, yellow likely completed, and magenta announced. Calendar days use `confirmedAt`, otherwise `createdAt`, in UTC. Counts and median reset gaps use effective timestamps in the past 90 days. AIHOT's own calendar uses Beijing time, so a day can differ.

The reset API allows one full-history read per 10 minutes. Successful parsed responses are cached at `$XDG_CACHE_HOME/llmpk/codex-resets.json`, otherwise `~/.cache/llmpk/codex-resets.json`. Reload also respects this cache. On HTTP 429, llmpk saves the server's `Retry-After` deadline in `codex-resets-cooldown.json` and shows existing history with a rate-limit warning. Without valid full history, it reads the official [recent-events API](https://aihot.news/api/v1/codex-resets/recent), caches it separately in `codex-resets-recent.json`, and labels the calendar incomplete. Partial history does not show 90-day counts or gap statistics. Parse failures and other HTTP errors remain errors. Fixture previews never write the cache.

## How it works

`src/data/client.ts` fetches all boards asynchronously with Bun's native `fetch`. Each request has a 20-second timeout. Network faults and 5xx responses get up to two retries, after one and two seconds. Parse errors and 4xx responses are reported without retrying.

The AA page embeds React Server Component chunks in `self.__next_f.push` calls. The parser joins those chunks, scans balanced JSON values, and joins model release dates. Slopalytics supplies a hashed JavaScript bundle; llmpk parses only its restricted model-data literal syntax and never executes the downloaded bundle. Other sources use JSON.

React manages view state; Ink renders terminal components and Yoga handles layout. Only the visible table/chart rows render. Reloads keep the selected row by ID. Source-shape errors appear on the affected board.

## Building

```sh
bun run build
./dist/llmpk --version
bun run smoke

# Release targets, built on their matching operating system
bun run build --target bun-darwin-arm64
bun run build --target bun-linux-x64

# Package the native executable, using the matching release target
bun run package --target aarch64-apple-darwin
```

### CI and releases

CI runs on pull requests, pushes to `main`, and manual dispatch. CI and Release
share `.github/workflows/build.yml`, pinned to Ubuntu 24.04 and macOS 15. Bun is
pinned in `.bun-version`. Each platform restores the package download cache and
runs a frozen install, offline tests, a native build, 24 standalone fixture
snapshots outside the checkout, and archive packaging. Linux also runs TypeScript,
Biome and installer syntax checks. Build archives and SHA256 files remain in
Actions artifacts for seven days.

A `v*` tag push starts Release. Manual dispatch requires an existing tag. The
preparation job reads `package.json` from that tag and requires its version to
match. Builds and the installer use the resolved commit SHA. Before publication,
the workflow reads the remote tag through GitHub's API, checks that it still
points to that SHA, and verifies the exact
five-file asset inventory, checksums and archive contents. Stable versions and
prereleases are identified from the manifest version. Existing releases are never
overwritten; use a new version and tag for a new release.

To verify the complete downloaded release bundle locally, including `install.sh`:

```sh
bun run package:verify --directory path/to/downloaded-assets
```

### Testing

```sh
bun run check       # TypeScript, Biome and offline tests
bun run typecheck
bun run lint
bun run test
bun run format      # Format TypeScript and project JSON

# Explicit live network checks, using temporary isolated caches
LLMPK_LIVE=1 bun test tests/ts/live.test.ts
```

Offline tests cover committed fixtures, malformed input, RSC references, prices, timestamps, filtering, sorting, cache expiry, rate-limit cooldowns, rendered layouts, release source validation and release archives. Live tests are skipped unless `LLMPK_LIVE` is set; the reset check uses the recent-events API so it does not consume the full-history quota. CI explicitly disables live tests. `bun run dev` watches source changes.

## Project structure

```text
src/cli.tsx          CLI options, snapshots and terminal lifecycle
src/model.ts         Board registry, columns, row types, formatting and filtering
src/data/client.ts   HTTP retries, cancellation and reset disk cache
src/data/rsc.ts      RSC stream extraction, balanced scanners and reference resolution
src/data/literal.ts  Restricted Slopalytics object-literal parser
src/data/sources.ts  Parsers and row normalization for all seven boards
src/data/demo.ts     Offline fixture preview data
src/tui/app.tsx      Async loading, keyboard/mouse input and view state
src/tui/screen.tsx   Tables, charts, calendar, details, help and chrome
src/tui/layout.ts    Unicode cell fitting, wrapping and responsive layouts
src/tui/state.ts     Selection, sorting, scrolling, calendar stats and mouse reports
src/tui/help.ts      In-app key and metric reference
scripts/build.ts     Standalone compilation and macOS signing
scripts/smoke.ts     Version and offline snapshots in an isolated working directory
scripts/package.ts   Release archives, checksums and complete asset verification
scripts/release.ts   Release tag/version validation and commit resolution
tests/ts/            Offline and opt-in live tests
tests/fixtures/      Trimmed source captures for offline tests
```

## License

MIT © Dsh
