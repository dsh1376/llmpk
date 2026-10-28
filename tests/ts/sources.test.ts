import { describe, expect, test } from "bun:test";
import { parseSource } from "../../src/data/sources.ts";

const fixture = (name: string) => Bun.file(new URL(`../fixtures/${name}`, import.meta.url)).text();

describe("source parity", () => {
  test("timestamps require explicit time and zone", () => {
    const tracker = parseSource(
      "tracker",
      JSON.stringify({ pages: [{ slug: "x", model: "X", lastSeenAt: "2026-10-22" }] }),
    );
    expect(tracker[0]?.metrics.published).toBeNull();
    const resets = parseSource(
      "resets",
      JSON.stringify({
        checkedAt: "x",
        historyFrom: "x",
        events: [{ createdAt: "2026-10-23T12:47:34" }],
      }),
    );
    expect(resets[0]?.event?.day).toBe("");
  });

  test("daily preserves metadata without a section label", () => {
    const rows = parseSource(
      "daily",
      JSON.stringify({
        report: {
          date: "2026-10-24",
          sections: [{ items: [{ title: "X", publishedAt: "2026-10-24T12:00:00Z" }] }],
        },
      }),
    );
    expect(rows[0]?.fields["Report date"]).toBe("2026-10-24");
    expect(rows[0]?.metrics.published).toBe(Date.parse("2026-10-24T12:00:00Z"));
  });

  test("resets finds English text and URL in separate posts", () => {
    const rows = parseSource(
      "resets",
      JSON.stringify({
        checkedAt: "x",
        historyFrom: "x",
        events: [
          {
            posts: [
              { originalText: null, url: null },
              { originalText: "Confirmed", url: "https://x.com/example" },
            ],
          },
        ],
      }),
    );
    expect(rows[0]?.summary).toBe("Confirmed");
    expect(rows[0]?.url).toBe("https://x.com/example");
  });
  test("AA joins releases and derives prices without inventing missing values", async () => {
    const rows = parseSource("aa", await fixture("aa_leaderboard.html"));
    expect(rows.length).toBeGreaterThanOrEqual(2);
    expect(rows.some((row) => row.metrics.intelligence !== null)).toBe(true);
    expect(rows.some((row) => row.fields.Released !== "-")).toBe(true);
    const opus = rows.find((row) => row.id === "claude-opus-5-5");
    expect(opus?.metrics.price).toBeCloseTo(2.94);
    expect(opus?.metrics.cache).toBeCloseTo(0.95);
    expect(opus?.fields["No cache 3:1 /M"]).toBe("$8.00");
    expect(opus?.fields.Released).toBe("2026-09-22");
    expect(rows.find((row) => row.name.includes("Qwen"))?.metrics.price).toBeNull();
  });

  test("AA preserves null names but rejects supplied invalid numeric types", () => {
    const html = (row: unknown) =>
      `<script>self.__next_f.push([1,${JSON.stringify(JSON.stringify(row))}])</script>`;
    expect(
      parseSource("aa", html({ slug: "x", name: null, contextWindowTokens: 10 }))[0]?.name,
    ).toBe("-");
    expect(() => parseSource("aa", html({ slug: "x", contextWindowTokens: "bad" }))).toThrow(
      "numeric",
    );
  });

  test("Slopalytics parses literals safely and drops deprecated models", async () => {
    const rows = parseSource("slop", await fixture("slopalytics_bundle.js"));
    expect(rows.length).toBeGreaterThanOrEqual(2);
    expect(rows.some((row) => row.name.includes("["))).toBe(true);
    expect(() => parseSource("slop", "models:[{id:`x`,name:evil()}]")).toThrow();
  });

  test("Frontier 2.0 uses main subset, best score effort and site-hidden models", async () => {
    const rows = parseSource("frontier", await fixture("frontiercode_data.json"));
    expect(rows.map((row) => row.id)).toEqual([
      "SWE-2",
      "GPT-6 Astra",
      "Muse Wonder 1.0",
      "Claude Opus 5.5",
      "SWE-1.7",
    ]);
    const swe = rows.find((row) => row.id === "SWE-1.7");
    expect(swe?.name).toBe("SWE-1.7");
    expect(swe?.fields.Effort).toBe("-");
    const opus = rows.find((row) => row.id === "Claude Opus 5.5");
    expect(opus?.name).toBe("Claude Opus 5.5 [max]");
    expect(opus?.metrics.intelligence).toBeCloseTo(0.4545);
    expect(opus?.metrics.pass).toBeCloseTo(0.497);
    expect(opus?.metrics.flag).toBeCloseTo(0.0288);
    expect(opus?.provider).toBe("Anthropic");
    expect(rows.map((row) => row.fields.Harness)).toEqual([
      "Devin CLI",
      "Codex",
      "Muse Code",
      "Claude Code",
      "Devin CLI",
    ]);
    expect(opus?.color).toBe("#FF5833");
    expect(opus?.fields["Effort low"]).toBe("14.7%");
    expect(rows.map((row) => row.provider)).toEqual([
      "Cognition",
      "OpenAI",
      "?",
      "Anthropic",
      "Cognition",
    ]);
  });

  test("Tracker keeps release/leak evidence", async () => {
    const rows = parseSource("tracker", await fixture("aitracker_model_pages.json"));
    expect(rows).toHaveLength(4);
    expect(rows[0]?.metrics.posts).toBe(1);
    expect(rows[0]?.url).toBe("https://live.aitracker.bot/minicpm-ocr-1.0");
  });

  test("Hot ranks and Daily flattens sections then flashes without repeating lead", async () => {
    expect(parseSource("hot", await fixture("aihot_hot_topics.json"))).toHaveLength(3);
    const rows = parseSource("daily", await fixture("aihot_daily.json"));
    expect(rows).toHaveLength(8);
    expect(rows[0]?.fields.Section).toBe("模型发布/更新");
    expect(rows[6]?.fields.Section).toBe("Flash");
    expect(rows[0]?.metrics.published).toBeNull();
  });

  test("Resets uses presentation status and UTC event dates", async () => {
    const rows = parseSource("resets", await fixture("aihot_codex_resets.json"));
    expect(rows).toHaveLength(4);
    expect(rows[0]?.fields.Status).toBe("likely_completed");
    expect(rows[2]?.event?.day).toBe("2026-10-08");
    expect(rows[2]?.event?.kind).toBe("reset_credit");
    expect(rows[0]?.summary).toContain("Landing tomorrow");
  });

  test.each(["aa", "frontier", "slop", "tracker", "hot", "daily", "resets"] as const)(
    "%s reports unrecognized data",
    (id) => {
      expect(() => parseSource(id, "{}")).toThrow();
    },
  );
});
