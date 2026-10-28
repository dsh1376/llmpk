import { BOARDS } from "../model.ts";

export interface HelpSection {
  title: string;
  /** Key and action pairs. */
  keys?: [string, string][];
  notes?: string[];
}

export const HELP: HelpSection[] = [
  {
    title: "Navigation",
    keys: [
      ["1-7", "Select board"],
      ["[ ]  Tab", "Previous / next board"],
      ["j k  ↓ ↑", "Select row"],
      ["PgUp PgDn", "Move 10 rows"],
      ["g G  Home End", "First / last row"],
      ["Esc", "Close help/details/search, otherwise quit"],
      ["q  Ctrl-C", "Quit"],
    ],
  },
  {
    title: "Search and display",
    keys: [
      ["/", "Edit this board's filter"],
      ["Enter  Esc", "Finish editing"],
      ["Ctrl-U", "Clear filter"],
      ["m", "Table / chart, or calendar"],
      ["z", "Compact columns"],
      ["Enter", "Full details, at every terminal size"],
      ["r", "Reload current board"],
      ["y", "Copy label (reset: post URL)"],
      ["Y", "Copy source URL"],
      ["Mouse", "Tabs, headers, rows, wheel and scrollbar drag"],
    ],
    notes: ["Search matches every word across names, providers and details."],
  },
  {
    title: "Sorting",
    keys: [
      ["o", "Reverse the current sort"],
      ...BOARDS.map((board, i): [string, string] => [
        `${i + 1} ${board.short}`,
        board.columns
          .filter((column) => column.hotkey)
          .map((column) => `${column.hotkey} ${column.label}`)
          .join(" · "),
      ]),
    ],
    notes: [
      "A metric key selects its sort; repeat it to reverse.",
      "Missing values remain last. Cost/time/token sorts start ascending.",
      "Selection stays with the same row across sort and refresh.",
    ],
  },
  {
    title: "Metrics",
    notes: [
      "AA price: USD per million tokens, blend 7:2:1 cache/input/output.",
      "No-cache blend: 3:1 input/output. Cache: 1 - hit/input.",
      "Slopalytics: cost/time/output tokens per intelligence-index task.",
      "AA estimate (~) is identified in details.",
      "FrontierCode 2.0: main subset, best score effort.",
      "Tracker: release and leak evidence, sorted by last sighting.",
      "AIHOT: Chinese-only titles, summaries and daily sections.",
      "Resets: calendar uses confirmedAt, else createdAt, in UTC.",
      "● Usage reset   ◆ Reset credit   + Multiple events on a day",
      "Status: confirmed / likely_completed / announced / unknown.",
      "Resets history cache lasts 10 minutes, including manual reloads.",
      "On 429, retain cached history and respect the server retry time.",
      "Without full history, recent events are labeled; 90-day stats are hidden.",
      "All dates and times are UTC. A dash means unavailable data.",
    ],
  },
  {
    title: "Sources",
    keys: BOARDS.map((board, i): [string, string] => [`${i + 1} ${board.short}`, board.source]),
  },
];
