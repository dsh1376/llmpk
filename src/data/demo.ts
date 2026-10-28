import aa from "../../tests/fixtures/aa_leaderboard.html" with { type: "text" };
import resets from "../../tests/fixtures/aihot_codex_resets.json" with { type: "text" };
import daily from "../../tests/fixtures/aihot_daily.json" with { type: "text" };
import hot from "../../tests/fixtures/aihot_hot_topics.json" with { type: "text" };
import tracker from "../../tests/fixtures/aitracker_model_pages.json" with { type: "text" };
import frontier from "../../tests/fixtures/frontiercode_data.json" with { type: "text" };
import slop from "../../tests/fixtures/slopalytics_bundle.js" with { type: "text" };
import { BOARDS } from "../model.ts";
import { initialStates } from "../tui/state.ts";
import { parseSource } from "./sources.ts";

export function demoStates() {
  const states = initialStates();
  const bodies = { aa, frontier, slop, tracker, hot, daily, resets };
  for (const board of BOARDS) {
    const body: unknown = bodies[board.id];
    if (typeof body !== "string") throw new Error(`fixture text unavailable: ${board.id}`);
    states[board.id] = { ...states[board.id], loading: false, rows: parseSource(board.id, body) };
  }
  return states;
}
