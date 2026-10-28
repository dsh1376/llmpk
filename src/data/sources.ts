import { type BoardId, formatValue, type Row } from "../model.ts";
import { parseModelsLiteral } from "./literal.ts";
import { afterKey, extractStream, objectsWith } from "./rsc.ts";

type ObjectData = Record<string, unknown>;
const obj = (value: unknown): ObjectData => {
  if (value == null) return {};
  if (typeof value !== "object" || Array.isArray(value)) throw new Error("expected data object");
  return value as ObjectData;
};
const arr = (value: unknown): unknown[] => {
  if (value == null) return [];
  if (!Array.isArray(value)) throw new Error("expected data array");
  return value;
};
const str = (value: unknown): string => {
  if (value == null) return "";
  if (typeof value !== "string") throw new Error("expected string field");
  return value;
};
const num = (value: unknown): number | null => {
  if (value == null) return null;
  if (typeof value !== "number" || !Number.isFinite(value))
    throw new Error("expected finite numeric field");
  return value;
};
const bool = (value: unknown): boolean | null => {
  if (value == null) return null;
  if (typeof value !== "boolean") throw new Error("expected boolean field");
  return value;
};
const first = (...values: unknown[]): string => values.map(str).find((value) => value.trim()) ?? "";
const date = (value: unknown, allowOffset = true): number | null => {
  const text = str(value);
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(text))
    return null;
  if (!allowOffset && !text.endsWith("Z")) return null;
  const epoch = Date.parse(text);
  return Number.isFinite(epoch) ? epoch : null;
};
const utc = (value: unknown) => {
  const epoch = date(value);
  return epoch === null ? "-" : `${formatValue(epoch, "date")} UTC`;
};
const money = (value: number | null) => formatValue(value, "money");
const count = (value: number | null) => formatValue(value, "tokens");
const text = (value: unknown) => str(value) || "-";
const color = (value: unknown) => (/^#[\da-f]{6}$/i.test(str(value)) ? str(value) : undefined);
const metrics = (data: ObjectData, keys: Record<string, string>) =>
  Object.fromEntries(Object.entries(keys).map(([key, source]) => [key, num(data[source])]));

function aa(body: string): Row[] {
  const stream = extractStream(body);
  const dates = new Map<string, string>();
  try {
    const table = obj(JSON.parse(afterKey(stream, "modelsAndReleases", "{")));
    const releases = new Map(
      arr(table.releases).map((v) => {
        const r = obj(v);
        return [str(r.slug), str(r.releaseDate)];
      }),
    );
    for (const value of arr(table.models)) {
      const model = obj(value);
      const released = releases.get(str(model.releaseSlug));
      if (released) dates.set(str(model.slug), released);
    }
  } catch {
    /* Releases are optional metadata; leaderboard fields remain strict. */
  }
  const seen = new Set<string>();
  return objectsWith(stream, "contextWindowTokens").flatMap((value) => {
    const r = obj(value);
    const id = str(r.slug);
    const input = num(r.price1mInputTokens);
    const output = num(r.price1mOutputTokens);
    const hit = num(r.cacheHitPrice);
    const write = num(r.cacheWritePrice);
    const blend =
      hit !== null && input !== null && output !== null
        ? (7 * hit + 2 * input + output) / 10
        : null;
    const uncached = input !== null && output !== null ? (3 * input + output) / 4 : null;
    const discount = hit !== null && input !== null && input > 0 ? 1 - hit / input : null;
    const weights = bool(r.isOpenWeights);
    const row: Row = {
      id,
      name: text(r.name),
      provider: str(r.modelCreatorName) || "?",
      color: color(r.modelCreatorColor),
      metrics: {
        ...metrics(r, {
          intelligence: "intelligenceIndex",
          speed: "medianOutputTokensPerSecond",
          context: "contextWindowTokens",
        }),
        price: blend,
        cache: discount,
      },
      fields: {
        "Input /M": money(input),
        "Output /M": money(output),
        "Cache hit /M": money(hit),
        "Cache write /M": money(write),
        "No cache 3:1 /M": money(uncached),
        Released: dates.get(id) || "-",
        Weights: weights === null ? "-" : weights ? "Open" : "Closed",
      },
      url: `https://artificialanalysis.ai/models/${encodeURIComponent(id)}`,
    };
    if (!id || seen.has(id)) return [];
    seen.add(id);
    return [row];
  });
}

function slop(body: string): Row[] {
  return arr(parseModelsLiteral(body)).flatMap((value) => {
    const r = obj(value);
    const id = str(r.id);
    if (bool(r.deprecated) || !id) return [];
    const effort = str(r.effortLabel).trim();
    const suffix = effort && effort.toLowerCase() !== "non-reasoning" ? ` [${effort}]` : "";
    return [
      {
        id,
        name: `${text(r.name)}${suffix}`,
        provider: str(r.labName).trim() || "?",
        metrics: metrics(r, {
          intelligence: "intelligence",
          price: "cost",
          time: "time",
          tokens: "tokens",
          speed: "speed",
          context: "context",
        }),
        fields: {
          Effort: suffix ? effort : "-",
          Intelligence: bool(r.estimated) ? "AA estimate (~)" : "Measured",
          "Full run cost": money(num(r.costTotal)),
          "Full run tokens": count(num(r.tokensTotal)),
          "Input /M": money(num(r.inputPrice)),
          "Output /M": money(num(r.outputPrice)),
          Released: text(r.released),
        },
        url: "https://slopalytics.com/",
      },
    ];
  });
}

// The site's leaderboard config hides these models from its table.
const FRONTIER_HIDDEN = new Set(["Obsidian 3"]);
// lab_colors shares one color across several labs, so labs come from name prefixes.
const FRONTIER_LABS: Record<string, string> = {
  Claude: "Anthropic",
  DeepSeek: "DeepSeek",
  GLM: "Zhipu",
  GPT: "OpenAI",
  Gemini: "Google",
  Grok: "xAI",
  Inkling: "Thinking Machines",
  Kimi: "Moonshot",
  Mistral: "Mistral",
  SWE: "Cognition",
};
// Harness display names from the site's leaderboard bundle.
const FRONTIER_HARNESSES: Record<string, string> = {
  devin: "Devin CLI",
  chisel: "Devin CLI",
  chisel_swe1: "Devin CLI",
  "claude-code": "Claude Code",
  codex: "Codex",
  "grok-build": "Grok Build",
  muse: "Muse Code",
  "gemini-cli": "Gemini CLI",
  vibe: "Mistral Vibe",
  "mini-swe-agent": "mini-swe-agent",
};

function frontier(body: string): Row[] {
  const r = obj(JSON.parse(body));
  const data = obj(r.data);
  const colors = obj(r.colors);
  const efforts = obj(r.efforts);
  return arr(r.models).flatMap((value) => {
    const name = str(value);
    if (FRONTIER_HIDDEN.has(name)) return [];
    const scores = obj(data[name]);
    const entries = arr(efforts[name]).map((v) => {
      const effort = str(v);
      return { effort, data: obj(obj(scores[effort]).main) };
    });
    // Ties keep the earlier effort, as the site does.
    let best: (typeof entries)[number] | undefined;
    for (const entry of entries) {
      const score = num(entry.data.new_score);
      if (score !== null && (!best || score > (num(best.data.new_score) ?? -Infinity)))
        best = entry;
    }
    if (!best) return [];
    const hex = /^#[\da-f]{6}(?:[\da-f]{2})?$/i.test(str(colors[name]))
      ? str(colors[name]).slice(0, 7)
      : undefined;
    const provider = FRONTIER_LABS[name.split(/[\s-]/)[0] ?? ""] ?? "?";
    const effort = best.effort === "none" || best.effort === "na" ? "" : best.effort;
    const harness = str(obj(r.harness)[name]);
    const minutes = num(best.data.duration_min);
    return [
      {
        id: name,
        name: effort ? `${name} [${effort}]` : name,
        provider,
        color: hex,
        metrics: {
          intelligence: num(best.data.new_score),
          pass: num(best.data.correct),
          price: num(best.data.cost),
          flag: num(best.data.flagged_rate),
          tokens: num(best.data.tokens),
        },
        fields: {
          Effort: effort || "-",
          Harness: FRONTIER_HARNESSES[harness] ?? text(harness),
          "Time/run": formatValue(minutes === null ? null : minutes * 60, "duration"),
          ...Object.fromEntries(
            entries.map((entry) => [
              `Effort ${entry.effort}`,
              formatValue(num(entry.data.new_score), "percent"),
            ]),
          ),
        },
        url: "https://cognition.com/frontiercode",
      },
    ];
  });
}

function tracker(body: string): Row[] {
  return arr(obj(JSON.parse(body)).pages).flatMap((value, index) => {
    const r = obj(value);
    const name = str(r.model);
    if (!name.trim()) return [];
    const path = str(r.path);
    return [
      {
        id: str(r.slug) || `tracker-${index}`,
        name,
        provider: str(r.maker).trim() || "unknown",
        metrics: {
          published: date(r.lastSeenAt, false),
          posts: arr(r.postIds).length,
          events: arr(r.eventIds).length,
        },
        fields: { Stage: str(r.stage).trim() || "unknown", "Last seen": utc(r.lastSeenAt) },
        url:
          path.startsWith("/") && !path.startsWith("//")
            ? `https://live.aitracker.bot${path}`
            : undefined,
      },
    ];
  });
}

function newsRow(value: unknown, index: number, section?: string, reportDate?: string): Row[] {
  const r = obj(value);
  const title = str(r.title);
  if (!title.trim()) return [];
  const links = obj(r.links);
  return [
    {
      id: str(r.id) || `news-${index}`,
      name: title,
      provider: str(obj(r.source).name) || "AIHOT",
      metrics: {
        rank: num(r.rank),
        order: index + 1,
        sources: num(r.sourceCount),
        signals: num(r.signalCount),
        participants: num(r.participantCount),
        published: date(section !== undefined ? r.publishedAt : r.latestAt),
      },
      fields: {
        ...(section !== undefined
          ? { Section: section || "-", "Report date": reportDate || "-" }
          : {}),
        "Source names": arr(r.sourceNames).map(str).join(", ") || "-",
        "Original URL": str(links.original) || "-",
        "AIHOT URL": str(links.aihot) || "-",
      },
      summary: str(r.summary) || undefined,
      url: first(links.aihot, links.original) || undefined,
    },
  ];
}

function resets(body: string): Row[] {
  const root = obj(JSON.parse(body));
  if (!str(root.checkedAt) || !str(root.historyFrom))
    throw new Error("missing reset history metadata");
  return arr(root.events).map((value, index) => {
    const r = obj(value);
    const presentation = obj(r.presentation);
    const posts = arr(r.posts).map(obj);
    const post = posts[0] ?? {};
    const schedule = obj(r.schedule);
    const status = first(presentation.status, r.status) || "unknown";
    const timestamp = date(r.confirmedAt) ?? date(r.createdAt);
    const from = date(schedule.from);
    const through = date(schedule.through);
    const expected =
      schedule.precision === "deadline" && through !== null
        ? `by ${utc(schedule.through)}`
        : schedule.precision === "approximate" && from !== null
          ? `~${utc(schedule.from)}`
          : from !== null && through !== null
            ? from === through
              ? utc(schedule.from)
              : `${utc(schedule.from)} - ${utc(schedule.through)}`
            : from !== null
              ? `after ${utc(schedule.from)}`
              : through !== null
                ? `by ${utc(schedule.through)}`
                : "-";
    return {
      id: str(r.id) || `reset-${index}`,
      name:
        str(r.type) === "reset_credit"
          ? "Reset credit"
          : str(r.type) === "direct_reset"
            ? "Usage reset"
            : text(r.type),
      provider: status,
      metrics: { published: date(r.createdAt) },
      fields: {
        Status: status,
        Scope: str(presentation.scopeLabel) || "Unknown",
        Announced: utc(r.createdAt),
        Confirmed: utc(r.confirmedAt),
        Expected: expected,
        Stage:
          (
            {
              预告: "announced",
              发卡预告: "credit announced",
              进展: "update",
              补充说明: "note",
              确认完成: "confirmed",
              确认发卡: "credit confirmed",
            } as Record<string, string>
          )[str(post.stage)] || text(post.stage),
        "Checked at": utc(root.checkedAt),
        "History from": utc(root.historyFrom),
      },
      summary: first(...posts.map((post) => post.originalText)) || undefined,
      url: first(...posts.map((post) => post.url)) || undefined,
      event: {
        day: timestamp === null ? "" : new Date(timestamp).toISOString().slice(0, 10),
        kind: str(r.type),
        status,
        timestamp,
      },
    };
  });
}

export function parseSource(id: BoardId, body: string): Row[] {
  let rows: Row[];
  switch (id) {
    case "aa":
      rows = aa(body);
      break;
    case "slop":
      rows = slop(body);
      break;
    case "frontier":
      rows = frontier(body);
      break;
    case "tracker":
      rows = tracker(body);
      break;
    case "hot":
      rows = arr(obj(JSON.parse(body)).items)
        .flatMap((value, i) => newsRow(value, i))
        .sort((a, b) => (a.metrics.rank ?? Infinity) - (b.metrics.rank ?? Infinity))
        .slice(0, 10);
      break;
    case "daily": {
      const report = obj(obj(JSON.parse(body)).report);
      rows = [];
      for (const value of arr(report.sections)) {
        const section = obj(value);
        for (const item of arr(section.items))
          rows.push(...newsRow(item, rows.length, str(section.label), str(report.date)));
      }
      for (const value of arr(report.flashes))
        rows.push(...newsRow(value, rows.length, "Flash", str(report.date)));
      break;
    }
    case "resets":
      rows = resets(body);
      break;
  }
  if (!rows.length) throw new Error(`no recognizable rows in ${id} source`);
  return rows;
}
