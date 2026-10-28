import { afterEach, expect, spyOn, test } from "bun:test";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { cleanup, render } from "ink-testing-library";
import { demoStates } from "../../src/data/demo.ts";
import { App } from "../../src/tui/app.tsx";

afterEach(cleanup);
async function frameUntil(app: ReturnType<typeof render>, text: string) {
  for (let i = 0; i < 40; i++) {
    if (app.lastFrame()?.includes(text)) return;
    await Bun.sleep(10);
  }
  throw new Error(`frame did not contain ${text}: ${app.lastFrame()}`);
}

test("keyboard state, board-local search and complete details", async () => {
  const app = render(<App demo mouse={false} initialData={demoStates()} />);
  await frameUntil(app, "● Ready · 4/4");
  await Bun.sleep(20);
  app.stdin.write("j");
  await frameUntil(app, "2/4 · artificialanalysis.ai");
  app.stdin.write("p");
  await frameUntil(app, "$/M ↑");
  expect(app.lastFrame()).toContain("2/4 · artificialanalysis.ai");
  app.stdin.write("/");
  await frameUntil(app, " / ▏");
  app.stdin.write("qwen");
  await frameUntil(app, "1/1 · artificialanalysis.ai");
  app.stdin.write("\r");
  await frameUntil(app, "Filter: qwen");
  app.stdin.write("\r");
  await frameUntil(app, "details");
  expect(app.lastFrame()).toContain("Input /M");
  app.stdin.write("\x1b");
  await frameUntil(app, "table");
  app.stdin.write("4");
  await frameUntil(app, "Last seen UTC ↓ · table");
  expect(app.lastFrame()).toContain("release");
  app.stdin.write("\x1b[D");
  await Bun.sleep(30);
  expect(app.lastFrame()).toContain("Last seen UTC ↓");
  app.stdin.write("1");
  await frameUntil(app, "● Ready · 1/4 · $/M ↑");
  expect(app.lastFrame()).toContain("Qwen3.5");
});

test("reset calendar toggles to a status table", async () => {
  const app = render(<App demo mouse={false} initialBoard={6} initialData={demoStates()} />);
  await frameUntil(app, "calendar");
  await Bun.sleep(20);
  app.stdin.write("m");
  await frameUntil(app, "table");
  expect(app.lastFrame()).toContain("likely");
});

test("a fresh full snapshot clears the earlier rate-limit and partial-history warning", async () => {
  const directory = await mkdtemp(join(tmpdir(), "llmpk-warning-test-"));
  const originalCache = process.env.XDG_CACHE_HOME;
  const blockedFetch = Object.assign(
    async () => new Response("blocked test request", { status: 403 }),
    { preconnect: globalThis.fetch.preconnect },
  );
  const request = spyOn(globalThis, "fetch").mockImplementation(blockedFetch);
  try {
    process.env.XDG_CACHE_HOME = directory;
    await mkdir(join(directory, "llmpk"));
    await Bun.write(
      join(directory, "llmpk", "codex-resets.json"),
      await Bun.file(new URL("../fixtures/aihot_codex_resets.json", import.meta.url)).text(),
    );
    const states = demoStates();
    states.resets.partial = true;
    states.resets.warning = "Recent events only; full history rate limited";
    const app = render(<App mouse={false} initialBoard={6} initialData={states} />);
    await frameUntil(app, "● Cached");
    expect(app.lastFrame()).not.toContain("rate limited");
    expect(app.lastFrame()).toContain("90d:");
  } finally {
    cleanup();
    request.mockRestore();
    if (originalCache === undefined) delete process.env.XDG_CACHE_HOME;
    else process.env.XDG_CACHE_HOME = originalCache;
    await rm(directory, { recursive: true, force: true });
  }
});
