import type { Board, Column } from "../model.ts";

// Bun.stringWidth accounts for CJK, combining marks and terminal graphemes.
export const widthOf = (value: string): number => Bun.stringWidth(value);
export function clean(value: string): string {
  return value.replace(/\p{Cc}/gu, " ");
}
export function fit(value: string, width: number, right = false): string {
  const safe = clean(value);
  let result = "";
  for (const { segment } of new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(
    safe,
  )) {
    if (widthOf(result + segment) > width) break;
    result += segment;
  }
  const padding = " ".repeat(Math.max(0, width - widthOf(result)));
  return right ? padding + result : result + padding;
}
/** Truncates to `width` cells with a trailing ellipsis when text is cut. No padding. */
export function ellipsis(value: string, width: number): string {
  if (width <= 0) return "";
  if (widthOf(clean(value)) <= width) return clean(value);
  return `${fit(value, width - 1).trimEnd()}…`;
}
export function wrap(value: string, width: number): string[] {
  const lines: string[] = [];
  for (const paragraph of value.split("\n")) {
    let line = "";
    for (const { segment } of new Intl.Segmenter(undefined, { granularity: "word" }).segment(
      clean(paragraph),
    )) {
      if (line && widthOf(line + segment) > width) {
        lines.push(line.trimEnd());
        line = "";
      }
      if (!line && !segment.trim()) continue;
      if (widthOf(segment) <= width) {
        line += segment;
        continue;
      }
      for (const { segment: part } of new Intl.Segmenter(undefined, {
        granularity: "grapheme",
      }).segment(segment)) {
        if (widthOf(line + part) > width) {
          lines.push(line);
          line = "";
        }
        line += part;
      }
    }
    lines.push(line);
  }
  return lines;
}

/** Small squares for filled and empty meter cells. */
export const SQUARE = "▪";
export const EMPTY_SQUARE = "▫";
/** Filled cells of a `width`-cell meter. Positive values fill at least one cell. */
export function meterCells(ratio: number, width: number): number {
  if (!Number.isFinite(ratio) || ratio <= 0 || width <= 0) return 0;
  return Math.max(1, Math.round(Math.min(1, ratio) * width));
}

/**
 * Shell rows: tab bar (0) and board title rule (1) above the body; key hints and
 * the status line below it. Renderers and mouse hit tests share these values.
 */
export const BODY_TOP = 2;
export const CHROME_ROWS = 4;
export const bodyHeightFor = (height: number): number => height - CHROME_ROWS;
export function detailPaneWidth(width: number, height: number): number {
  return width >= 116 && height >= 16 ? Math.max(36, Math.min(48, Math.floor(width * 0.3))) : 0;
}
/** Table and chart rows: one header line when the body has room for it. */
export function listGeometry(bodyHeight: number): { header: boolean; capacity: number } {
  const header = bodyHeight > 1;
  return { header, capacity: Math.max(1, bodyHeight - (header ? 1 : 0)) };
}
/** Cells before the row name: selection gutter, right-aligned rank and one space. */
export const RANK_WIDTH = 5;

export interface TableLayout {
  columns: Column[];
  nameLabel: string;
  nameWidth: number;
  providerWidth: number;
  gap: number;
  width: number;
  /** Inline bar cells after the active metric, including its leading space. */
  bar: number;
  textField?: { key: string; width: number };
}
export function tableLayout(
  board: Board,
  width: number,
  sort: string,
  compact: boolean,
): TableLayout {
  const gap = compact ? 0 : 1;
  const base = board.columns.find((column) => column.key === sort) ?? board.columns[0];
  const active =
    base?.format === "date" && width < 60 ? { ...base, label: "UTC date", width: 12 } : base;
  const columns: Column[] = active ? [active] : [];
  const textField =
    board.id === "tracker"
      ? { key: "Stage", width: 8 }
      : board.id === "resets"
        ? { key: "Status", width: 10 }
        : board.id === "daily" && width >= 56
          ? { key: "Section", width: 14 }
          : board.id === "frontier" && width >= 56
            ? { key: "Harness", width: 14 }
            : undefined;
  // FrontierCode mirrors the site's table, which has no provider column.
  const providerWidth = board.id !== "frontier" && width >= (textField ? 94 : 76) ? 14 : 0;
  let remaining =
    width -
    RANK_WIDTH -
    providerWidth -
    (providerWidth ? gap : 0) -
    (active ? active.width + gap : 0) -
    (textField ? textField.width + gap : 0);
  for (const column of board.columns) {
    if (
      columns.some((existing) => existing.key === column.key) ||
      remaining - column.width - gap < 26
    )
      continue;
    columns.push(column);
    remaining -= column.width + gap;
  }
  const barWidth = board.chart && active && active.format !== "date" && remaining >= 34 ? 8 : 0;
  remaining -= barWidth;
  return {
    columns,
    nameLabel: board.id === "frontier" ? "Model" : "Model / event",
    nameWidth: Math.max(5, remaining),
    providerWidth,
    gap,
    width,
    bar: barWidth,
    textField,
  };
}
/** Metric column x offsets, relative to the first cell of a table row. */
export function metricPositions(layout: TableLayout): { column: Column; x: number }[] {
  let x =
    RANK_WIDTH +
    layout.nameWidth +
    layout.gap +
    (layout.textField ? layout.textField.width + layout.gap : 0) +
    (layout.providerWidth ? layout.providerWidth + layout.gap : 0);
  return layout.columns.map((column, i) => {
    const result = { column, x };
    x += column.width + (i === 0 ? layout.bar : 0) + layout.gap;
    return result;
  });
}

/** Chosen items, in order, that fit `width` with `separator` between them. */
export function fitItems<T>(
  items: T[],
  width: number,
  separator: number,
  measure: (item: T) => number,
): T[] {
  const chosen: T[] = [];
  let used = 0;
  for (const item of items) {
    const next = used + (chosen.length ? separator : 0) + measure(item);
    if (next > width) continue;
    chosen.push(item);
    used = next;
  }
  return chosen;
}
export function wholeItems(items: string[], width: number, separator = " · "): string {
  return fitItems(items.filter(Boolean), width, widthOf(separator), widthOf).join(separator);
}

export interface TabItem {
  index: number;
  text: string;
  x: number;
}
export interface TabBar {
  brand: boolean;
  items: TabItem[];
}
export const BRAND = " llmpk ";
/**
 * Tabs prefer full labels, then short labels, then a short active label with
 * numbered neighbors. Distant tabs drop last. The brand appears only with spare room.
 */
export function tabBar(width: number, active: number, boards: Board[], reserve = 0): TabBar {
  const variants = [
    (board: Board, i: number) => `${i + 1} ${board.label}`,
    (board: Board, i: number) => `${i + 1} ${board.short}`,
    (board: Board, i: number) => (i === active ? `${i + 1} ${board.short}` : `${i + 1}`),
  ];
  const total = (items: { text: string }[]) =>
    items.reduce((sum, item) => sum + widthOf(item.text), 0);
  let visible = boards.map((board, index) => ({ index, text: ` ${variants[0]?.(board, index)} ` }));
  for (const variant of variants) {
    visible = boards.map((board, index) => ({ index, text: ` ${variant(board, index)} ` }));
    if (total(visible) <= width) break;
  }
  while (total(visible) > width && visible.length > 1) {
    const first = visible[0];
    const last = visible.at(-1);
    if (!first || !last) break;
    if (Math.abs(first.index - active) > Math.abs(last.index - active)) visible.shift();
    else visible.pop();
  }
  const brand = total(visible) + widthOf(BRAND) + 1 + reserve <= width;
  let x = brand ? widthOf(BRAND) + 1 : 0;
  return {
    brand,
    items: visible.map((item) => {
      const result = { ...item, x };
      x += widthOf(item.text);
      return result;
    }),
  };
}
