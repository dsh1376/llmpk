import manifest from "../package.json" with { type: "json" };

export const VERSION = manifest.version;

export type BoardId = "aa" | "frontier" | "slop" | "tracker" | "hot" | "daily" | "resets";
export type MetricFormat = "number" | "money" | "percent" | "tokens" | "duration" | "date";
export interface Column {
  key: string;
  label: string;
  hotkey: string;
  format: MetricFormat;
  ascending?: boolean;
  width: number;
}
export interface Board {
  id: BoardId;
  label: string;
  short: string;
  source: string;
  url: string;
  description: string;
  chart: boolean;
  columns: Column[];
}
export interface Row {
  id: string;
  name: string;
  provider: string;
  color?: string;
  metrics: Record<string, number | null>;
  fields: Record<string, string>;
  summary?: string;
  url?: string;
  event?: { day: string; kind: string; status: string; timestamp: number | null };
}
const col = (
  key: string,
  label: string,
  hotkey: string,
  format: MetricFormat = "number",
  ascending = false,
  width = 9,
): Column => ({ key, label, hotkey, format, ascending, width });
export const BOARDS: Board[] = [
  {
    id: "aa",
    label: "AA",
    short: "AA",
    source: "artificialanalysis.ai",
    url: "https://artificialanalysis.ai/leaderboards/models",
    description: "Model intelligence, speed and token economics",
    chart: true,
    columns: [
      col("intelligence", "Intelligence", "i", "number", false, 13),
      col("price", "$/M", "p", "money", true),
      col("speed", "t/s", "s"),
      col("cache", "Cache", "d", "percent"),
      col("context", "Context", "c", "tokens"),
    ],
  },
  {
    id: "frontier",
    label: "FrontierCode",
    short: "Frontier",
    source: "cognition.com",
    url: "https://cognition.com/data/frontier-code-2.0/data.json",
    description: "FrontierCode 2.0 Leaderboard · Best reasoning mode",
    chart: true,
    columns: [
      col("intelligence", "Score", "i", "percent"),
      col("pass", "Pass rate", "a", "percent", false, 11),
      col("flag", "Flag rate", "f", "percent", true, 11),
      col("price", "Cost / rollout", "p", "money", true, 16),
      col("tokens", "Output tokens", "u", "tokens", true, 15),
    ],
  },
  {
    id: "slop",
    label: "Slopalytics",
    short: "Slop",
    source: "slopalytics.com",
    url: "https://slopalytics.com/",
    description: "Intelligence and compute per index task",
    chart: true,
    columns: [
      col("intelligence", "Intelligence", "i", "number", false, 13),
      col("price", "$/task", "p", "money", true),
      col("time", "Time/task", "t", "duration", true, 11),
      col("tokens", "Tokens", "u", "tokens", true),
      col("speed", "t/s", "s"),
    ],
  },
  {
    id: "tracker",
    label: "AI Tracker",
    short: "Tracker",
    source: "live.aitracker.bot",
    url: "https://live.aitracker.bot/api/model-pages",
    description: "New releases and leaks, with their evidence",
    chart: false,
    columns: [
      col("published", "Last seen UTC", "t", "date", false, 18),
      col("posts", "Posts", "", "number", false, 7),
      col("events", "Events", "", "number", false, 8),
    ],
  },
  {
    id: "hot",
    label: "AIHOT Hot",
    short: "Hot",
    source: "aihot.news",
    url: "https://aihot.news/api/v1/hot-topics",
    description: "AIHOT · 热门 AI 事件 · Top 10",
    chart: false,
    columns: [
      col("rank", "Rank", "i", "number", true, 7),
      col("sources", "Sources", "", "number", false, 9),
      col("signals", "Signals", ""),
      col("participants", "People", ""),
      col("published", "Latest UTC", "", "date", false, 18),
    ],
  },
  {
    id: "daily",
    label: "AIHOT Daily",
    short: "Daily",
    source: "aihot.news",
    url: "https://aihot.news/api/v1/dailies/latest",
    description: "AIHOT · 最新日报 · Sections then flashes",
    chart: false,
    columns: [
      col("order", "Order", "i", "number", true, 8),
      col("published", "Published UTC", "", "date", false, 18),
    ],
  },
  {
    id: "resets",
    label: "Codex Resets",
    short: "Resets",
    source: "aihot.news",
    url: "https://aihot.news/api/v1/codex-resets",
    description: "Usage resets and credits · Calendar dates in UTC",
    chart: true,
    columns: [col("published", "Announced UTC", "t", "date", false, 18)],
  },
];

export function formatValue(value: number | null | undefined, format: MetricFormat): string {
  if (value == null || !Number.isFinite(value)) return "-";
  switch (format) {
    case "money":
      return value > 0 && value < 0.01 ? "<$0.01" : `$${value.toFixed(2)}`;
    case "percent":
      return `${(value * 100).toFixed(1)}%`;
    case "tokens":
      return value >= 1e6
        ? `${+(value / 1e6).toFixed(1)}M`
        : value >= 1e3
          ? `${+(value / 1e3).toFixed(1)}k`
          : `${Math.round(value)}`;
    case "duration":
      return value >= 3600
        ? `${(value / 3600).toFixed(1)}h`
        : value >= 60
          ? `${(value / 60).toFixed(1)}m`
          : `${value.toFixed(1)}s`;
    case "date":
      return new Date(value).toISOString().slice(0, 16).replace("T", " ");
    default:
      return Number.isInteger(value) ? `${value}` : value.toFixed(1);
  }
}

export function visibleRows(rows: Row[], filter: string, sort: string, ascending: boolean): Row[] {
  const terms = filter.toLocaleLowerCase().split(/\s+/).filter(Boolean);
  return rows
    .filter((row) => {
      const text =
        `${row.name} ${row.provider} ${Object.values(row.fields).join(" ")} ${row.summary ?? ""}`.toLocaleLowerCase();
      return terms.every((term) => text.includes(term));
    })
    .sort((a, b) => {
      const av = a.metrics[sort];
      const bv = b.metrics[sort];
      if (av == null) return bv == null ? 0 : 1;
      if (bv == null) return -1;
      return ascending ? av - bv : bv - av;
    });
}
