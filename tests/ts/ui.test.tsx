import { expect, test } from "bun:test";
import { renderToString } from "ink";
import { demoStates } from "../../src/data/demo.ts";
import { parseSource } from "../../src/data/sources.ts";
import { BOARDS, visibleRows } from "../../src/model.ts";
import {
  BODY_TOP,
  detailPaneWidth,
  metricPositions,
  tabBar,
  tableLayout,
} from "../../src/tui/layout.ts";
import { Screen } from "../../src/tui/screen.tsx";
import { initialStates, mouseDecoder, resetStats } from "../../src/tui/state.ts";

const fixtureNames = [
  "aa_leaderboard.html",
  "frontiercode_data.json",
  "slopalytics_bundle.js",
  "aitracker_model_pages.json",
  "aihot_hot_topics.json",
  "aihot_daily.json",
  "aihot_codex_resets.json",
];
test("smallest supported terminal shows the selected data row", () => {
  const output = renderToString(<Screen width={36} height={8} active={0} states={demoStates()} />, {
    columns: 36,
  });
  expect(output).toContain("Claude");
  expect(output).toContain("q quit");
});

test("narrow tab bar keeps every board number and the active label", () => {
  const tabs = tabBar(36, 3, BOARDS);
  expect(tabs.items.map((item) => item.index)).toEqual([0, 1, 2, 3, 4, 5, 6]);
  const output = renderToString(<Screen width={36} height={8} active={3} states={demoStates()} />, {
    columns: 36,
  });
  expect(output.split("\n")[0]).toContain(" 4 Tracker ");
});

test("header sort hit regions match the rendered metric columns", () => {
  const states = demoStates();
  for (const width of [80, 160, 220]) {
    const board = BOARDS[0];
    if (!board) throw new Error("missing board");
    const output = renderToString(<Screen width={width} height={24} active={0} states={states} />, {
      columns: width,
    });
    const header = output.split("\n")[BODY_TOP] ?? "";
    const mainWidth = width - detailPaneWidth(width, 24);
    const layout = tableLayout(board, mainWidth - 3, states.aa.sort, false);
    for (const { column, x } of metricPositions(layout))
      expect(header.slice(1 + x, 1 + x + column.width).trim()).toStartWith(
        column.label === "Intelligence" ? "Intel" : column.label,
      );
  }
});

test("tracker exposes release/leak stage in the table", () => {
  const output = renderToString(
    <Screen width={60} height={24} active={3} states={demoStates()} />,
    { columns: 60 },
  );
  expect(output).toContain("release");
  expect(output).toContain("leak");
});

test("FrontierCode uses the site's leaderboard header", () => {
  const output = renderToString(
    <Screen width={120} height={12} active={1} states={demoStates()} />,
    { columns: 120 },
  );
  expect(output).toMatch(
    /#\s+Model\s+Harness\s+Score ↓\s+Pass rate\s+Flag rate\s+Cost \/ rollout\s+Output tokens/,
  );
  expect(output).toMatch(
    /Claude Opus 5\.5 \[max\]\s+Claude Code\s+45\.5%\s+49\.7%\s+2\.9%\s+\$12\.82/,
  );
  expect(output).not.toContain("Provider");
  const wide = renderToString(<Screen width={160} height={24} active={1} states={demoStates()} />, {
    columns: 160,
  });
  expect(wide).toMatch(/│ Claude Opus 5\.5 \[max\]/);
  expect(wide).not.toContain("SELECTED");
});

test("search keeps each row's rank from the unfiltered sort", () => {
  const states = demoStates();
  states.aa = { ...states.aa, filter: "qwen" };
  const output = renderToString(<Screen width={120} height={20} active={0} states={states} />, {
    columns: 120,
  });
  expect(output).toMatch(/▌\s+3 Qwen/);
});

test("reset statistics use effective timestamps within the past 90 days", () => {
  const stats = resetStats(demoStates().resets.rows, Date.parse("2026-10-24T15:00:00Z"));
  expect(stats.resets).toBe(2);
  expect(stats.credits).toBe(2);
  expect(stats.medianDays?.toFixed(1)).toBe("2.9");
});

test("partial reset history is labeled and does not claim complete 90-day statistics", () => {
  const states = demoStates();
  states.resets.partial = true;
  states.resets.warning = "Recent events only; full history rate limited until 12:30:00 UTC";
  const output = renderToString(<Screen width={80} height={24} active={6} states={states} />, {
    columns: 80,
  });
  expect(output).toContain("Recent events only");
  expect(output).toContain("90-day statistics unavailable");
  expect(output).not.toContain("90d:");
  expect(output).not.toContain("Median gap:");
});
test("all boards fit narrow and wide terminals in both views", async () => {
  const states = initialStates();
  for (const [i, board] of BOARDS.entries())
    states[board.id] = {
      ...states[board.id],
      loading: false,
      rows: parseSource(
        board.id,
        await Bun.file(new URL(`../fixtures/${fixtureNames[i]}`, import.meta.url)).text(),
      ),
    };
  for (const width of [36, 60, 80, 120, 160]) {
    for (const height of [8, 24, 40]) {
      for (const [active, board] of BOARDS.entries()) {
        for (const view of ["table", "chart"] as const) {
          const state = { ...states[board.id], view: board.chart ? view : ("table" as const) };
          const output = renderToString(
            <Screen
              width={width}
              height={height}
              active={active}
              states={{ ...states, [board.id]: state }}
            />,
            { columns: width },
          );
          expect(output.split("\n").length).toBeLessThanOrEqual(height);
          expect(output.split("\n").every((line) => Bun.stringWidth(line) <= width)).toBe(true);
          expect(output).toContain(board.label);
          expect(output).not.toContain("undefined");
        }
      }
    }
  }
});

test("AND search and null-last sort preserve source data", () => {
  const rows = [
    { id: "1", name: "Model Alpha", provider: "Lab", fields: {}, metrics: { price: null } },
    { id: "2", name: "Model Beta", provider: "Lab", fields: {}, metrics: { price: 1 } },
  ];
  expect(visibleRows(rows, "lab beta", "price", true).map((r) => r.id)).toEqual(["2"]);
  expect(visibleRows(rows, "", "price", false).map((r) => r.id)).toEqual(["2", "1"]);
  expect(rows[0]?.id).toBe("1");
});

test("mouse parser handles split and combined SGR reports", () => {
  const decode = mouseDecoder();
  expect(decode("\x1b[<0;12;")).toEqual([]);
  expect(decode("7M\x1b[<64;12;7M")).toEqual([
    { button: 0, x: 11, y: 6, released: false },
    { button: 64, x: 11, y: 6, released: false },
  ]);
});
