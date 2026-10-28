#!/usr/bin/env bun
import { parseArgs } from "node:util";
import { render, renderToString } from "ink";
import { loadBoard } from "./data/client.ts";
import { demoStates } from "./data/demo.ts";
import { BOARDS, VERSION } from "./model.ts";
import { App } from "./tui/app.tsx";
import { Screen } from "./tui/screen.tsx";
import { initialStates } from "./tui/state.ts";

const HELP = `llmpk ${VERSION} · LLM leaderboards in your terminal

Usage: llmpk [options]
  --board <1-7|id>  Initial board (aa, frontier, slop, tracker, hot, daily, resets)
  --demo            Preview the committed, trimmed fixtures without network or cache writes
  --snapshot        Print one frame and exit (fetches only the selected board)
  --width <cells>   Snapshot width (default 120)
  --height <rows>   Snapshot height (default 30)
  --no-mouse        Keep normal terminal text selection
  --version         Print version
  --help            Print this help

In app: 1-7 boards · j/k rows · / search · m chart · Enter details · ? help · q quit
`;

async function main() {
  const { values } = parseArgs({
    args: process.argv.slice(2),
    options: {
      board: { type: "string" },
      demo: { type: "boolean" },
      snapshot: { type: "boolean" },
      width: { type: "string" },
      height: { type: "string" },
      "no-mouse": { type: "boolean" },
      version: { type: "boolean" },
      help: { type: "boolean", short: "h" },
    },
    strict: true,
  });
  if (values.help) {
    process.stdout.write(HELP);
    return;
  }
  if (values.version) {
    process.stdout.write(`${VERSION}\n`);
    return;
  }
  const active = values.board
    ? BOARDS.findIndex(
        (board, index) => board.id === values.board || `${index + 1}` === values.board,
      )
    : 0;
  if (active < 0) throw new Error("--board must be 1-7 or a board id; see --help");
  const initialData = values.demo ? demoStates() : undefined;
  if (values.snapshot) {
    const width = Number(values.width ?? 120);
    const height = Number(values.height ?? 30);
    if (
      !Number.isInteger(width) ||
      width < 36 ||
      width > 500 ||
      !Number.isInteger(height) ||
      height < 8 ||
      height > 200
    )
      throw new Error("snapshot dimensions must be width 36-500 and height 8-200");
    const states = initialData ?? initialStates();
    const board = BOARDS[active];
    if (!board) throw new Error("board unavailable");
    if (!values.demo) {
      const result = await loadBoard(board.id);
      states[board.id] = { ...states[board.id], ...result, loading: false, updated: Date.now() };
    }
    process.stdout.write(
      `${renderToString(<Screen width={width} height={height} active={active} states={states} demo={values.demo} />, { columns: width })}\n`,
    );
    return;
  }
  if (!process.stdin.isTTY || !process.stdout.isTTY)
    throw new Error("interactive terminal required; use --snapshot or --demo --snapshot");
  const app = render(
    <App
      initialData={initialData}
      initialBoard={active}
      mouse={!values["no-mouse"]}
      demo={values.demo}
    />,
    // Full-frame redraws replace rows displaced by terminal resize.
    { alternateScreen: true, incrementalRendering: false, exitOnCtrlC: false, patchConsole: false },
  );
  const stop = () => app.unmount();
  process.once("SIGTERM", stop);
  process.once("SIGINT", stop);
  process.once("SIGHUP", stop);
  try {
    await app.waitUntilExit();
  } finally {
    process.off("SIGTERM", stop);
    process.off("SIGINT", stop);
    process.off("SIGHUP", stop);
  }
}
try {
  await main();
} catch (error) {
  process.stderr.write(`llmpk: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
