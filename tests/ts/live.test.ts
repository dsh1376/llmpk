import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fetchText, loadBoard } from "../../src/data/client.ts";
import { parseSource } from "../../src/data/sources.ts";
import { BOARDS } from "../../src/model.ts";

const minimums = {
  aa: 100,
  frontier: 10,
  slop: 100,
  tracker: 20,
  hot: 5,
  daily: 5,
  resets: 1,
};
for (const board of BOARDS) {
  test.skipIf(!process.env.LLMPK_LIVE).concurrent(
    `live_fetch ${board.id}`,
    async () => {
      const directory = await mkdtemp(join(tmpdir(), "llmpk-live-"));
      try {
        // Full-history reads consume the user's ten-minute quota even with a test cache.
        const result =
          board.id === "resets"
            ? { rows: parseSource("resets", await fetchText(`${board.url}/recent`)) }
            : await loadBoard(board.id, undefined, { cacheDirectory: directory });
        expect(result.rows.length).toBeGreaterThanOrEqual(minimums[board.id]);
        expect(result.rows.every((row) => row.id && row.name)).toBe(true);
        if (board.id === "aa") {
          expect(result.rows.filter((row) => row.metrics.price != null).length).toBeGreaterThan(50);
          expect(result.rows.filter((row) => row.metrics.speed != null).length).toBeGreaterThan(50);
          expect(result.rows.filter((row) => row.fields.Released !== "-").length).toBeGreaterThan(
            100,
          );
        } else if (board.id === "tracker")
          expect(result.rows.every((row) => row.metrics.published !== null)).toBe(true);
        else if (board.id === "resets")
          expect(result.rows.every((row) => row.event?.day && row.event.status !== "unknown")).toBe(
            true,
          );
        else if (board.id === "slop")
          expect(result.rows.filter((row) => row.metrics.price !== null).length).toBeGreaterThan(
            50,
          );
        else if (board.id === "frontier")
          expect(result.rows.some((row) => row.metrics.intelligence !== null)).toBe(true);
      } finally {
        await rm(directory, { recursive: true, force: true });
      }
    },
    75_000,
  );
}
