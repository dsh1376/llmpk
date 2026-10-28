// biome-ignore-all lint/suspicious/noArrayIndexKey: Text lines, parts and calendar cells are stateless grid positions.
import { Box, Text } from "ink";
import { BOARDS, type Board, type Column, formatValue, type Row } from "../model.ts";
import { HELP } from "./help.ts";
import {
  BRAND,
  bodyHeightFor,
  clean,
  detailPaneWidth,
  EMPTY_SQUARE,
  ellipsis,
  fit,
  fitItems,
  listGeometry,
  meterCells,
  metricPositions,
  RANK_WIDTH,
  SQUARE,
  tabBar,
  tableLayout,
  wholeItems,
  widthOf,
  wrap,
} from "./layout.ts";
import {
  type BoardState,
  type BoardStates,
  ranksFor,
  resetStats,
  rowsFor,
  selectedIndex,
  windowStart,
} from "./state.ts";

export const ACCENT = "#d4ad6a";
const MUTED = "gray";
const RULE = "#4b5059";
const SELECTED = "#30343b";
/** Dark text on the accent pill. */
const ON_ACCENT = "#1c1e22";
export const SPINNER = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const statusColor = (status: string) =>
  status === "confirmed" || status === "release"
    ? "green"
    : status === "likely_completed" || status === "leak"
      ? "yellow"
      : status === "announced"
        ? "magenta"
        : MUTED;

/** One styled run of text. A line is a sequence of runs that fits its width. */
export interface Part {
  text: string;
  color?: string;
  bold?: boolean;
  underline?: boolean;
  bg?: string;
}
export type Line = Part[];

function clip(line: Line, width: number): Line {
  const result: Line = [];
  let used = 0;
  for (const part of line) {
    const text = clean(part.text);
    if (!text) continue;
    const kept =
      widthOf(text) <= width - used ? text : fit(text, Math.max(0, width - used)).trimEnd();
    if (!kept) break;
    result.push({ ...part, text: kept });
    used += widthOf(kept);
  }
  return result;
}
function Runs({ line, width, bg }: { line: Line; width: number; bg?: string }) {
  const parts = clip(line, width);
  const used = parts.reduce((sum, part) => sum + widthOf(part.text), 0);
  return (
    <Text backgroundColor={bg}>
      {parts.map((part, i) => (
        <Text
          key={i}
          color={part.color}
          bold={part.bold}
          underline={part.underline}
          backgroundColor={part.bg ?? bg}
        >
          {part.text}
        </Text>
      ))}
      {" ".repeat(Math.max(0, width - used))}
    </Text>
  );
}

/** Thumb cells for a scrollbar of `size` lines, proportional to the visible share. */
function thumbCells(total: number, visible: number, size: number, ratio: number): Set<number> {
  if (total <= visible || size < 1) return new Set();
  const length = Math.max(1, Math.round((size * visible) / total));
  const top = Math.round(Math.max(0, Math.min(1, ratio)) * (size - length));
  return new Set(Array.from({ length }, (_, i) => top + i));
}
function Scrollbar({
  total,
  visible,
  size,
  ratio,
}: {
  total: number;
  visible: number;
  size: number;
  ratio: number;
}) {
  const thumb = thumbCells(total, visible, size, ratio);
  return (
    <Box width={1} height={size} flexDirection="column">
      {total > visible
        ? Array.from({ length: size }, (_, i) => (
            <Text key={i} color={thumb.has(i) ? MUTED : RULE}>
              {thumb.has(i) ? "┃" : "│"}
            </Text>
          ))
        : null}
    </Box>
  );
}

/** Meter of small squares: filled cells in `color`, the rest as a faint track. */
function meter(ratio: number, width: number, color: string): Line {
  const filled = meterCells(ratio, width);
  return [
    { text: SQUARE.repeat(filled), color },
    { text: EMPTY_SQUARE.repeat(Math.max(0, width - filled)), color: RULE },
  ];
}

const heading = (text: string): Line => [{ text: text.toUpperCase(), color: MUTED, bold: true }];

/** Label/value rows with a muted label column and hanging indent for wrapped values. */
function pairs(
  items: { label: string; value: Line; plain: string }[],
  width: number,
  labelWidth: number,
): Line[] {
  const lines: Line[] = [];
  for (const { label, value, plain } of items) {
    if (width - labelWidth < 12) {
      lines.push([{ text: label, color: MUTED }]);
      lines.push(...(value.length > 1 ? [value] : wrapValue(plain, width, value[0])));
      continue;
    }
    const valueLines = value.length > 1 ? [value] : wrapValue(plain, width - labelWidth, value[0]);
    valueLines.forEach((line, i) => {
      lines.push([
        { text: i ? " ".repeat(labelWidth) : fit(label, labelWidth - 1), color: MUTED },
        ...(i ? [] : [{ text: " " }]),
        ...line,
      ]);
    });
  }
  return lines;
}
function wrapValue(text: string, width: number, style: Part | undefined): Line[] {
  return wrap(text, width).map((line) => [{ ...style, text: line }]);
}

const metricLabel = (board: Board, column: Column) =>
  board.id === "aa" && column.key === "price"
    ? "Blend 7:2:1 /M"
    : board.id === "aa" && column.key === "cache"
      ? "Cache discount"
      : column.label;
const metricMax = (state: BoardState, key: string) =>
  Math.max(0, ...state.rows.map((row) => row.metrics[key] ?? 0));

export function detailLines(board: Board, state: BoardState, width: number): Line[] {
  const rows = rowsFor(state);
  const row = rows[selectedIndex(state, rows)];
  if (!row) return [[{ text: "No row selected", color: MUTED }]];
  const sort = board.columns.find((column) => column.key === state.sort);
  const rank = ranksFor(state).get(row.id);
  const lines: Line[] = wrap(row.name, width).map((text) => [{ text, color: ACCENT, bold: true }]);
  lines.push(
    ...wrap(
      [row.provider, rank && sort ? `#${rank} of ${state.rows.length} by ${sort.label}` : ""]
        .filter(Boolean)
        .join(" · "),
      width,
    ).map((text): Line => [{ text, color: MUTED }]),
  );
  const labels = [
    ...board.columns.map((column) => metricLabel(board, column)),
    ...Object.keys(row.fields),
  ];
  const labelWidth = Math.min(17, Math.max(8, ...labels.map((label) => widthOf(label) + 2)));
  if (board.columns.length) {
    lines.push([], heading("Metrics"));
    lines.push(
      ...pairs(
        board.columns.map((column) => {
          const value = row.metrics[column.key];
          const text = formatValue(value, column.format);
          const active = column.key === state.sort;
          const style: Part = {
            text,
            color: value == null ? MUTED : active ? ACCENT : undefined,
            bold: active,
          };
          const room = width - labelWidth - widthOf(text) - 1;
          const max = metricMax(state, column.key);
          const line: Line =
            active && board.chart && column.format !== "date" && value != null && room >= 4 && max
              ? [
                  style,
                  { text: " " },
                  ...meter(value / max, Math.min(14, room), row.color || ACCENT),
                ]
              : [style];
          return { label: metricLabel(board, column), value: line, plain: text };
        }),
        width,
        labelWidth,
      ),
    );
  }
  const fields = Object.entries(row.fields);
  if (fields.length) {
    lines.push([], heading("Details"));
    lines.push(
      ...pairs(
        fields.map(([label, value]) => ({
          label,
          value: [
            {
              text: value || "-",
              color:
                label === "Status" || label === "Stage"
                  ? statusColor(value)
                  : value
                    ? undefined
                    : MUTED,
            },
          ],
          plain: value || "-",
        })),
        width,
        labelWidth,
      ),
    );
  }
  if (row.summary)
    lines.push([], heading("Summary"), ...wrap(row.summary, width).map((text) => [{ text }]));
  if (row.url)
    lines.push(
      [],
      heading("Link"),
      ...wrap(row.url, width).map((text): Line => [{ text, color: ACCENT, underline: true }]),
    );
  return lines;
}

export function helpLines(width: number): Line[] {
  const keyWidth = 15;
  const lines: Line[] = [];
  for (const [i, section] of HELP.entries()) {
    if (i) lines.push([]);
    lines.push([{ text: section.title, color: ACCENT, bold: true }]);
    for (const [key, action] of section.keys ?? []) {
      if (width - keyWidth < 16) {
        lines.push([{ text: key, color: ACCENT }]);
        lines.push(...wrap(action, width - 2).map((text) => [{ text: `  ${text}` }]));
        continue;
      }
      wrap(action, width - keyWidth).forEach((text, j) => {
        lines.push([{ text: fit(j ? "" : key, keyWidth), color: ACCENT }, { text }]);
      });
    }
    for (const note of section.notes ?? [])
      lines.push(...wrap(note, width).map((text): Line => [{ text, color: MUTED }]));
  }
  return lines;
}

function Lines({
  lines,
  width,
  height,
  offset = 0,
}: {
  lines: Line[];
  width: number;
  height: number;
  offset?: number;
}) {
  return (
    <Box flexDirection="column" width={width} height={height} overflow="hidden">
      {lines.slice(offset, offset + height).map((line, i) => (
        <Runs key={offset + i} line={line} width={width} />
      ))}
    </Box>
  );
}

function Table({
  board,
  state,
  width,
  height,
  compact,
}: {
  board: Board;
  state: BoardState;
  width: number;
  height: number;
  compact: boolean;
}) {
  const rows = rowsFor(state);
  const ranks = ranksFor(state);
  const selected = selectedIndex(state, rows);
  const { header, capacity } = listGeometry(height);
  const start = windowStart(selected, rows.length, capacity);
  const layout = tableLayout(board, width - 3, state.sort, compact);
  const gap = " ".repeat(layout.gap);
  const lead = layout.columns[0];
  const max = lead && layout.bar ? metricMax(state, lead.key) : 0;
  const metricCells = (row: Row | undefined): Line =>
    metricPositions(layout).flatMap(({ column }, j): Line => {
      const active = column.key === state.sort;
      let text = column.label === "Intelligence" ? "Intel" : column.label;
      let color: string | undefined = active ? ACCENT : MUTED;
      if (row) {
        const value = row.metrics[column.key];
        text =
          column.format === "date" && column.width < 18
            ? formatValue(value, "date").slice(0, 10)
            : formatValue(value, column.format);
        color = value == null ? MUTED : active ? ACCENT : undefined;
      } else if (active) text += state.ascending ? " ↑" : " ↓";
      const cells: Line = [
        ...(j ? [{ text: gap }] : []),
        { text: fit(text, column.width, true), color, bold: active && !row },
      ];
      if (j === 0 && layout.bar) {
        const value = row?.metrics[column.key];
        cells.push(
          { text: "  " },
          ...(value != null && max
            ? meter(value / max, layout.bar - 2, row?.color || ACCENT)
            : [{ text: " ".repeat(layout.bar - 2) }]),
        );
      }
      return cells;
    });
  const textCell = (row: Row | undefined): Line => {
    if (!layout.textField) return [];
    const value = row?.fields[layout.textField.key] ?? "";
    return [
      {
        text: fit(
          row ? value.replace("likely_completed", "likely") || "-" : layout.textField.key,
          layout.textField.width,
        ),
        color: row ? statusColor(value) : MUTED,
      },
      { text: gap },
    ];
  };
  const providerCell = (row: Row | undefined, chosen: boolean): Line =>
    layout.providerWidth
      ? [
          {
            text: fit(row ? row.provider : "Provider", layout.providerWidth),
            color: row ? (chosen ? "white" : row.color || MUTED) : MUTED,
          },
          { text: gap },
        ]
      : [];
  return (
    <Box width={width} height={height} paddingLeft={1} flexDirection="column" overflow="hidden">
      {header ? (
        <Runs
          width={width - 1}
          line={[
            { text: fit(fit("#", RANK_WIDTH - 1, true), RANK_WIDTH), color: MUTED },
            { text: fit(layout.nameLabel, layout.nameWidth), color: MUTED },
            { text: gap },
            ...textCell(undefined),
            ...providerCell(undefined, false),
            ...metricCells(undefined),
          ]}
        />
      ) : null}
      <Box height={capacity}>
        <Box width={width - 3} flexDirection="column">
          {rows.slice(start, start + capacity).flatMap((row, i) => {
            const chosen = i + start === selected;
            const rank = ranks.get(row.id) ?? i + start + 1;
            return [
              <Runs
                key={row.id}
                width={width - 3}
                bg={chosen ? SELECTED : undefined}
                line={[
                  { text: chosen ? "▌" : " ", color: ACCENT },
                  {
                    text: `${fit(`${rank}`, RANK_WIDTH - 2, true)} `,
                    color: chosen ? ACCENT : MUTED,
                  },
                  {
                    text: fit(row.name, layout.nameWidth),
                    color: chosen ? "white" : row.color,
                    bold: chosen,
                  },
                  { text: gap },
                  ...textCell(row),
                  ...providerCell(row, chosen),
                  ...metricCells(row),
                ]}
              />,
            ];
          })}
        </Box>
        <Scrollbar
          total={rows.length}
          visible={capacity}
          size={capacity}
          ratio={selected / Math.max(1, rows.length - 1)}
        />
      </Box>
    </Box>
  );
}

function Chart({
  board,
  state,
  width,
  height,
}: {
  board: Board;
  state: BoardState;
  width: number;
  height: number;
}) {
  const rows = rowsFor(state);
  const ranks = ranksFor(state);
  const selected = selectedIndex(state, rows);
  const { header, capacity } = listGeometry(height);
  const start = windowStart(selected, rows.length, capacity);
  const column = board.columns.find((column) => column.key === state.sort) ?? board.columns[0];
  if (!column) return null;
  const max = Math.max(1e-9, metricMax(state, column.key));
  const inner = width - 2;
  const valueWidth = 10;
  const labelWidth = Math.max(8, Math.min(32, Math.floor(inner * 0.34)));
  const barWidth = Math.max(0, inner - RANK_WIDTH - labelWidth - valueWidth - 2);
  const scale = `max ${formatValue(metricMax(state, column.key), column.format)}`;
  const title = `${column.label} · ${column.ascending ? "lower" : "higher"} is better`;
  return (
    <Box flexDirection="column" width={width} height={height} paddingX={1} overflow="hidden">
      {header ? (
        <Runs
          width={inner}
          line={[
            { text: title, color: ACCENT, bold: true },
            ...(widthOf(title) + widthOf(scale) + 2 <= inner
              ? [{ text: fit(scale, inner - widthOf(title), true), color: MUTED }]
              : []),
          ]}
        />
      ) : null}
      {rows.slice(start, start + capacity).flatMap((row, i) => {
        const value = row.metrics[column.key];
        const chosen = start + i === selected;
        return [
          <Runs
            key={row.id}
            width={inner}
            bg={chosen ? SELECTED : undefined}
            line={[
              { text: chosen ? "▌" : " ", color: ACCENT },
              {
                text: `${fit(`${ranks.get(row.id) ?? start + i + 1}`, RANK_WIDTH - 2, true)} `,
                color: chosen ? ACCENT : MUTED,
              },
              {
                text: fit(row.name, labelWidth),
                color: chosen ? "white" : row.color,
                bold: chosen,
              },
              { text: " " },
              ...meter(value == null ? 0 : value / max, barWidth, row.color || ACCENT),
              { text: " " },
              {
                text: fit(formatValue(value, column.format), valueWidth, true),
                color: value == null ? MUTED : chosen ? ACCENT : undefined,
                bold: chosen,
              },
            ]}
          />,
        ];
      })}
    </Box>
  );
}

function Calendar({
  state,
  width,
  height,
  now,
}: {
  state: BoardState;
  width: number;
  height: number;
  now: number;
}) {
  const rows = rowsFor(state);
  const selected = rows[selectedIndex(state, rows)];
  const today = new Date(now).toISOString().slice(0, 10);
  const selectedDay = selected?.event?.day || today;
  const month = selectedDay.slice(0, 7);
  const firstDay = new Date(`${month}-01T00:00:00Z`);
  const firstWeekday = (firstDay.getUTCDay() + 6) % 7;
  const days = new Date(
    Date.UTC(firstDay.getUTCFullYear(), firstDay.getUTCMonth() + 1, 0),
  ).getUTCDate();
  const weeks = Math.ceil((days + firstWeekday) / 7);
  const inner = width - 2;
  const cell = inner >= 42 ? 6 : inner >= 35 ? 5 : 4;
  const gridWidth = cell * 7;
  const sideWidth = inner - gridWidth - 3 >= 28 ? inner - gridWidth - 3 : 0;
  const gridLines = 2 + weeks;
  const stats = resetStats(state.rows, now);
  const statusLine: Line = [
    {
      text: selected?.fields.Status || selected?.event?.status || "unknown",
      color: statusColor(selected?.event?.status || ""),
    },
  ];
  if (height < gridLines + (sideWidth ? 2 : 1))
    return (
      <Box width={width} height={height} paddingX={1} flexDirection="column">
        {(
          [
            [
              { text: selectedDay, color: ACCENT, bold: true },
              { text: ` ${selected?.name || "No events"}` },
            ],
            statusLine,
            [{ text: "Enter details · m table", color: MUTED }],
          ] as Line[]
        )
          .slice(0, height)
          .map((line, i) => (
            <Runs key={i} line={line} width={inner} />
          ))}
      </Box>
    );
  const grid: Line[] = [
    [
      {
        text: firstDay.toLocaleString("en-US", { month: "long", year: "numeric", timeZone: "UTC" }),
        color: ACCENT,
        bold: true,
      },
    ],
    ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(
      (day): Part => ({ text: fit(`${" ".repeat(cell - 4)}${day}`, cell), color: MUTED }),
    ),
    ...Array.from(
      { length: weeks },
      (_, week): Line =>
        Array.from({ length: 7 }, (_, weekday): Part => {
          const day = week * 7 + weekday - firstWeekday + 1;
          if (day < 1 || day > days) return { text: " ".repeat(cell) };
          const key = `${month}-${`${day}`.padStart(2, "0")}`;
          const events = rows.filter((row) => row.event?.day === key);
          const event = events.find((row) => row.id === selected?.id) ?? events[0];
          const mark = event ? (event.event?.kind === "reset_credit" ? "◆" : "●") : " ";
          return {
            text: fit(
              `${" ".repeat(cell - 4)}${`${day}`.padStart(2)}${mark}${events.length > 1 ? "+" : " "}`,
              cell,
            ),
            color: event ? statusColor(event.event?.status || "") : key === today ? "white" : MUTED,
            bold: Boolean(event) || key === today,
            underline: key === today,
            bg: key === selectedDay ? SELECTED : undefined,
          };
        }),
    ),
  ];
  const infoWidth = sideWidth || inner;
  const stat = (text: string): Line[] =>
    wrap(text, infoWidth).map((line): Line => [{ text: line, color: MUTED }]);
  const info: Line[] = [
    ...(sideWidth ? [heading(state.partial ? "Recent history" : "Last 90 days")] : []),
    state.partial
      ? [{ text: "Recent events only; history incomplete", color: "yellow" }]
      : [{ text: `90d: ${stats.resets} resets · ${stats.credits} credits` }],
    ...(state.partial
      ? stat("90-day statistics unavailable")
      : [
          ...stat(
            `Median gap: ${stats.medianDays === null ? "-" : `${stats.medianDays.toFixed(1)}d`}`,
          ),
          ...stat(
            `Last reset: ${stats.last === undefined ? "-" : new Date(stats.last).toISOString().slice(0, 10)}`,
          ),
        ]),
    [],
    ...(sideWidth ? [heading("Selected")] : []),
    [
      { text: selectedDay, color: ACCENT, bold: true },
      { text: ` ${selected?.name || "No events"}` },
    ],
    statusLine,
    ...wrap(selected?.summary || "", infoWidth).map((text): Line => [{ text }]),
  ];
  const legend: Line = [
    { text: "●", color: MUTED },
    { text: " Reset  ", color: MUTED },
    { text: "◆", color: MUTED },
    { text: " Credit  + Multiple  ", color: MUTED },
    { text: "UTC", color: MUTED, bold: true },
  ];
  if (sideWidth)
    return (
      <Box width={width} height={height} paddingX={1} overflow="hidden">
        <Box width={gridWidth} flexDirection="column">
          {[...grid, [], legend].map((line, i) => (
            <Runs key={i} line={line} width={gridWidth} />
          ))}
        </Box>
        <Box width={3} />
        <Lines lines={info} width={sideWidth} height={height} />
      </Box>
    );
  return (
    <Box width={width} height={height} paddingX={1} overflow="hidden">
      <Lines lines={[...grid, legend, ...info]} width={inner} height={height} />
    </Box>
  );
}

export interface ScreenProps {
  width: number;
  height: number;
  active: number;
  states: BoardStates;
  compact?: boolean;
  editing?: boolean;
  panel?: "help" | "details";
  panelOffset?: number;
  message?: string;
  demo?: boolean;
  /** Spinner frame for loading indicators. */
  frame?: number;
  /** Clock for the calendar's current-day mark. */
  now?: number;
}
export function Screen({
  width,
  height,
  active,
  states,
  compact = false,
  editing = false,
  panel,
  panelOffset = 0,
  message,
  demo,
  frame = 0,
  now = Date.now(),
}: ScreenProps) {
  const board = BOARDS[active] ?? BOARDS[0];
  if (!board) return null;
  if (width < 36 || height < 8)
    return (
      <Box width={width} height={height}>
        <Text>{fit("llmpk needs 36 columns × 8 rows", width)}</Text>
      </Box>
    );
  const state = states[board.id];
  const rows = rowsFor(state);
  const index = selectedIndex(state, rows);
  const bodyHeight = bodyHeightFor(height);
  const detailWidth = panel ? 0 : detailPaneWidth(width, height);
  const mainWidth = width - detailWidth;
  const column = board.columns.find((column) => column.key === state.sort);
  const spinner = SPINNER[frame % SPINNER.length] ?? "·";
  const mode =
    panel || (state.view === "chart" ? (board.id === "resets" ? "calendar" : "chart") : "table");

  // Tab bar: brand, numbered tabs and source health.
  const loading = BOARDS.filter((b) => states[b.id].loading).length;
  const failed = BOARDS.filter((b) => states[b.id].error).length;
  const health: Part = loading
    ? { text: `${spinner} ${loading} loading`, color: ACCENT }
    : failed
      ? { text: `✕ ${failed} failed`, color: "red" }
      : { text: `● ${BOARDS.length}/${BOARDS.length} ready`, color: MUTED };
  const tabs = tabBar(width, active, BOARDS, widthOf(health.text) + 1);
  const tabsEnd = tabs.items.at(-1);
  const used = tabsEnd ? tabsEnd.x + widthOf(tabsEnd.text) : 0;
  const tabLine: Line = [
    ...(tabs.brand ? [{ text: BRAND, color: ACCENT, bold: true }, { text: " " }] : []),
    ...tabs.items.map((item): Part => {
      const state = states[BOARDS[item.index]?.id ?? "aa"];
      return item.index === active
        ? { text: item.text, color: ON_ACCENT, bg: ACCENT, bold: true }
        : { text: item.text, color: state.error ? "red" : state.loading ? RULE : MUTED };
    }),
    ...(width - used >= widthOf(health.text) + 2
      ? [{ ...health, text: fit(`${health.text} `, width - used, true) }]
      : []),
  ];

  // Board title rule: label, description, then status, count, sort and view.
  const status = state.loading
    ? { text: `${spinner} ${state.rows.length ? "Refreshing" : "Loading"}`, color: ACCENT }
    : state.error
      ? { text: "✕ Error", color: "red" }
      : state.partial
        ? { text: "◐ Recent", color: "yellow" }
        : state.cached
          ? { text: "● Cached", color: "cyan" }
          : { text: "● Ready", color: "green" };
  const label = ` ${board.label} `;
  const facts = fitItems<Part>(
    [
      status,
      { text: `${rows.length}/${state.rows.length}` },
      { text: column ? `${column.label} ${state.ascending ? "↑" : "↓"}` : "" },
      { text: mode },
    ].filter((part) => part.text),
    width - widthOf(label) - 4,
    3,
    (part) => widthOf(part.text),
  );
  const factsWidth = facts.reduce((sum, part, i) => sum + widthOf(part.text) + (i ? 3 : 0), 0);
  const descWidth = width - widthOf(label) - factsWidth - 6;
  const description = descWidth >= 12 ? ` ${ellipsis(board.description, descWidth)} ` : "";
  const titleLine: Line = [
    { text: label, color: ACCENT, bold: true },
    { text: description, color: MUTED },
    {
      text: "─".repeat(Math.max(1, width - widthOf(label) - widthOf(description) - factsWidth - 2)),
      color: RULE,
    },
    { text: " " },
    ...facts.flatMap(
      (part, i): Line => [
        ...(i ? [{ text: " · ", color: RULE }] : []),
        { ...part, color: part.color ?? MUTED },
      ],
    ),
  ];

  // Key hints and status line.
  const hints: [string, string][] = panel
    ? [
        ["j/k", "scroll"],
        ["Esc", "close"],
        ["q", "quit"],
        ["PgUp/PgDn", "page"],
        ["y", "copy"],
      ]
    : width < 65
      ? [
          ["/", "find"],
          ["Enter", "info"],
          ["?", "help"],
          ["q", "quit"],
        ]
      : [
          ["j/k", "row"],
          ["[ ]", "board"],
          ["/", "search"],
          ["?", "help"],
          ["q", "quit"],
          ...(board.chart
            ? [["m", board.id === "resets" ? "calendar" : "chart"] as [string, string]]
            : []),
          ["Enter", "details"],
          ["r", "reload"],
          [
            board.columns
              .map((column) => column.hotkey)
              .filter(Boolean)
              .join("/"),
            "sort",
          ],
        ];
  const hintLine: Line = editing
    ? [
        { text: " / ", color: ACCENT, bold: true },
        { text: state.filter },
        { text: "▏", color: ACCENT },
        { text: "  Enter done · Ctrl-U clear", color: MUTED },
      ]
    : [
        { text: " " },
        ...fitItems(
          hints.filter(([key]) => key),
          width - 2,
          2,
          ([key, action]) => widthOf(`${key} ${action}`),
        ).flatMap(
          ([key, action], i): Line => [
            ...(i ? [{ text: "  " }] : []),
            { text: key, color: ACCENT, bold: true },
            { text: ` ${action}`, color: MUTED },
          ],
        ),
      ];
  const right = demo
    ? "FIXTURE PREVIEW"
    : state.updated
      ? `updated ${new Date(state.updated).toISOString().slice(11, 19)} UTC`
      : "";
  const leftWidth = width - 2 - (right ? widthOf(right) + 2 : 0);
  const notice = message || state.error || state.warning;
  const left = notice
    ? ellipsis(notice, leftWidth)
    : wholeItems(
        [
          `${rows.length ? index + 1 : 0}/${rows.length}`,
          board.source,
          state.filter ? `Filter: ${state.filter}` : "",
        ],
        leftWidth,
      );
  const statusBar: Line = [
    {
      text: ` ${left}`,
      color: message ? ACCENT : state.error ? "red" : state.warning ? "yellow" : MUTED,
    },
    ...(right
      ? [
          {
            text: fit(right, width - 1 - widthOf(left) - 1, true),
            color: demo ? ACCENT : MUTED,
            bold: demo,
          },
        ]
      : []),
  ];

  const panelLines =
    panel === "help"
      ? helpLines(width - 4)
      : panel === "details"
        ? detailLines(board, state, width - 4)
        : [];
  const offset = Math.max(0, Math.min(panelOffset, panelLines.length - bodyHeight));
  const empty = !state.rows.length;
  return (
    <Box width={width} height={height} flexDirection="column" overflow="hidden">
      <Runs line={tabLine} width={width} />
      <Runs line={titleLine} width={width} />
      <Box height={bodyHeight} width={width} overflow="hidden">
        {panel ? (
          <Box width={width} height={bodyHeight} paddingX={1}>
            <Box width={width - 3} paddingLeft={1}>
              <Lines lines={panelLines} width={width - 4} height={bodyHeight} offset={offset} />
            </Box>
            <Scrollbar
              total={panelLines.length}
              visible={bodyHeight}
              size={bodyHeight}
              ratio={offset / Math.max(1, panelLines.length - bodyHeight)}
            />
          </Box>
        ) : (
          <>
            <Box width={mainWidth} height={bodyHeight} flexDirection="column" overflow="hidden">
              {empty ? (
                <Box paddingX={2} paddingTop={bodyHeight >= 8 ? 2 : 0} flexDirection="column">
                  <Runs
                    width={mainWidth - 4}
                    line={
                      state.error
                        ? [{ text: "✕ Cannot load this board", color: "red", bold: true }]
                        : [{ text: `${spinner} Fetching source data`, color: ACCENT, bold: true }]
                    }
                  />
                  {wrap(state.error || board.description, mainWidth - 4)
                    .slice(0, Math.max(0, bodyHeight - 2))
                    .map((line, i) => (
                      <Text key={i}>{line}</Text>
                    ))}
                  <Runs
                    width={mainWidth - 4}
                    line={[
                      { text: board.source, color: MUTED },
                      { text: "  r", color: ACCENT, bold: true },
                      { text: " retry", color: MUTED },
                      { text: "  [ ]", color: ACCENT, bold: true },
                      { text: " switch", color: MUTED },
                    ]}
                  />
                </Box>
              ) : !rows.length ? (
                <Box paddingX={2} flexDirection="column">
                  <Runs
                    width={mainWidth - 4}
                    line={[
                      { text: "No rows match " },
                      { text: `“${ellipsis(state.filter, mainWidth - 22)}”`, color: ACCENT },
                    ]}
                  />
                  <Runs
                    width={mainWidth - 4}
                    line={[
                      { text: "/", color: ACCENT, bold: true },
                      { text: " edit filter  ", color: MUTED },
                      { text: "Ctrl-U", color: ACCENT, bold: true },
                      { text: " clear", color: MUTED },
                    ]}
                  />
                </Box>
              ) : state.view === "chart" && board.id === "resets" ? (
                <Calendar state={state} width={mainWidth} height={bodyHeight} now={now} />
              ) : state.view === "chart" && board.chart ? (
                <Chart board={board} state={state} width={mainWidth} height={bodyHeight} />
              ) : (
                <Table
                  board={board}
                  state={state}
                  width={mainWidth}
                  height={bodyHeight}
                  compact={compact}
                />
              )}
            </Box>
            {detailWidth > 0 ? (
              <Box
                width={detailWidth}
                height={bodyHeight}
                borderStyle="single"
                borderTop={false}
                borderBottom={false}
                borderRight={false}
                borderColor={RULE}
                paddingX={1}
              >
                <Lines
                  lines={detailLines(board, state, detailWidth - 3)}
                  width={detailWidth - 3}
                  height={bodyHeight}
                />
              </Box>
            ) : null}
          </>
        )}
      </Box>
      <Runs line={hintLine} width={width} />
      <Runs line={statusBar} width={width} />
    </Box>
  );
}
