import { BOARDS, type BoardId, type Row, visibleRows } from "../model.ts";

export interface BoardState {
  rows: Row[];
  loading: boolean;
  error?: string;
  warning?: string;
  updated?: number;
  cached?: boolean;
  partial?: boolean;
  filter: string;
  sort: string;
  ascending: boolean;
  selectedId?: string;
  view: "table" | "chart";
}
export type BoardStates = Record<BoardId, BoardState>;
export function initialStates(): BoardStates {
  return Object.fromEntries(
    BOARDS.map((board) => [
      board.id,
      {
        rows: [],
        loading: true,
        filter: "",
        sort: board.columns[0]?.key ?? "published",
        ascending: board.columns[0]?.ascending ?? false,
        view: board.id === "resets" ? "chart" : "table",
      },
    ]),
  ) as unknown as BoardStates;
}
export function rowsFor(state: BoardState): Row[] {
  return visibleRows(state.rows, state.filter, state.sort, state.ascending);
}
/** 1-based position of each row under the current sort, ignoring the filter. */
export function ranksFor(state: BoardState): Map<string, number> {
  return new Map(
    visibleRows(state.rows, "", state.sort, state.ascending).map((row, i) => [row.id, i + 1]),
  );
}
export function selectedIndex(state: BoardState, rows = rowsFor(state)): number {
  return Math.max(
    0,
    rows.findIndex((row) => row.id === state.selectedId),
  );
}
export function selectRow(state: BoardState, index: number): BoardState {
  const rows = rowsFor(state);
  return { ...state, selectedId: rows[Math.max(0, Math.min(rows.length - 1, index))]?.id };
}
export function changeSort(state: BoardState, key: string, ascending: boolean): BoardState {
  const selectedId = state.selectedId ?? rowsFor(state)[0]?.id;
  return {
    ...state,
    sort: key,
    ascending: state.sort === key ? !state.ascending : ascending,
    selectedId,
  };
}
export function windowStart(selected: number, length: number, capacity: number): number {
  return Math.max(0, Math.min(Math.max(0, length - capacity), selected - Math.floor(capacity / 2)));
}

export function resetStats(rows: Row[], now = Date.now()) {
  const recent = rows.filter(
    (row) =>
      row.event?.timestamp != null &&
      row.event.timestamp <= now &&
      row.event.timestamp >= now - 90 * 86_400_000,
  );
  const resets = recent
    .filter((row) => row.event?.kind === "direct_reset")
    .map((row) => row.event?.timestamp ?? 0)
    .sort((a, b) => a - b);
  const gaps = resets
    .slice(1)
    .map((time, i) => (time - (resets[i] ?? time)) / 86_400_000)
    .sort((a, b) => a - b);
  const medianDays = gaps.length
    ? ((gaps[Math.floor((gaps.length - 1) / 2)] ?? 0) + (gaps[Math.floor(gaps.length / 2)] ?? 0)) /
      2
    : null;
  return {
    resets: resets.length,
    credits: recent.filter((row) => row.event?.kind === "reset_credit").length,
    medianDays,
    last: resets.at(-1),
  };
}

export interface MouseEvent {
  button: number;
  x: number;
  y: number;
  released: boolean;
}
export function mouseDecoder(): (chunk: string) => MouseEvent[] {
  let pending = "";
  return (chunk) => {
    pending += chunk;
    const events: MouseEvent[] = [];
    for (;;) {
      const start = pending.indexOf("\x1b[<");
      if (start < 0) {
        pending = pending.endsWith("\x1b[") ? "\x1b[" : pending.endsWith("\x1b") ? "\x1b" : "";
        break;
      }
      pending = pending.slice(start);
      const match = /^(\d+);(\d+);(\d+)([Mm])/.exec(pending.slice(3));
      if (!match) {
        if (pending.length > 64) pending = "";
        break;
      }
      events.push({
        button: Number(match[1]),
        x: Number(match[2]) - 1,
        y: Number(match[3]) - 1,
        released: match[4] === "m",
      });
      pending = pending.slice(3 + match[0].length);
    }
    return events;
  };
}
