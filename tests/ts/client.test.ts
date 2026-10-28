import { describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm, stat, utimes } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fetchText, loadBoard } from "../../src/data/client.ts";

describe("network boundary", () => {
  test("429 retains full history and persists the server cooldown across reloads", async () => {
    const directory = await mkdtemp(join(tmpdir(), "llmpk-429-test-"));
    try {
      const body = await Bun.file(
        new URL("../fixtures/aihot_codex_resets.json", import.meta.url),
      ).text();
      const path = join(directory, "codex-resets.json");
      await Bun.write(path, body);
      const now = Date.now();
      await utimes(path, new Date(now - 900_000), new Date(now - 900_000));
      let requests = 0;
      const options = {
        cacheDirectory: directory,
        now: () => now,
        request: async () => {
          requests++;
          return new Response("limited", { status: 429, headers: { "Retry-After": "120" } });
        },
      };
      const result = await loadBoard("resets", undefined, options);
      expect(result.rows).toHaveLength(4);
      expect(result.cached).toBe(true);
      expect(result.partial).not.toBe(true);
      expect(result.warning).toContain("rate limited");
      expect(result.warning).toContain(new Date(now + 120_000).toISOString().slice(11, 19));
      const again = await loadBoard("resets", undefined, {
        ...options,
        request: async () => {
          throw new Error("cooldown must survive a new caller");
        },
      });
      expect(again.rows).toHaveLength(4);
      expect(await readFile(path, "utf8")).toBe(body);
      expect(requests).toBe(1);
      await expect(
        loadBoard("resets", undefined, { ...options, now: () => now + 120_001 }),
      ).resolves.toMatchObject({ cached: true });
      expect(requests).toBe(2);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("cold-cache 429 uses the recent API, labels partial history and caches it separately", async () => {
    const directory = await mkdtemp(join(tmpdir(), "llmpk-recent-test-"));
    try {
      const body = await Bun.file(
        new URL("../fixtures/aihot_codex_resets.json", import.meta.url),
      ).text();
      const urls: string[] = [];
      const request = async (url: string) => {
        urls.push(url);
        return url.endsWith("/recent")
          ? new Response(body)
          : new Response("limited", { status: 429, headers: { "Retry-After": "600" } });
      };
      const options = { cacheDirectory: directory, request };
      const result = await loadBoard("resets", undefined, options);
      expect(result.rows).toHaveLength(4);
      expect(result.partial).toBe(true);
      expect(result.warning).toContain("Recent events only");
      expect(urls).toEqual([
        "https://aihot.news/api/v1/codex-resets",
        "https://aihot.news/api/v1/codex-resets/recent",
      ]);
      expect((await loadBoard("resets", undefined, options)).partial).toBe(true);
      expect(urls).toHaveLength(2);
      expect(await Bun.file(join(directory, "codex-resets.json")).exists()).toBe(false);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("concurrent reset loads share one upstream request", async () => {
    const directory = await mkdtemp(join(tmpdir(), "llmpk-concurrent-test-"));
    try {
      const body = await Bun.file(
        new URL("../fixtures/aihot_codex_resets.json", import.meta.url),
      ).text();
      let requests = 0;
      const options = {
        cacheDirectory: directory,
        request: async () => {
          requests++;
          await Bun.sleep(10);
          return new Response(body);
        },
      };
      const results = await Promise.all([
        loadBoard("resets", undefined, options),
        loadBoard("resets", undefined, options),
      ]);
      expect(results.every((result) => result.rows.length === 4)).toBe(true);
      expect(requests).toBe(1);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("cancelling one reset caller does not abort another caller", async () => {
    const directory = await mkdtemp(join(tmpdir(), "llmpk-cancel-test-"));
    try {
      const body = await Bun.file(
        new URL("../fixtures/aihot_codex_resets.json", import.meta.url),
      ).text();
      const started = Promise.withResolvers<void>();
      const finish = Promise.withResolvers<void>();
      let reads = 0;
      const options = {
        cacheDirectory: directory,
        request: async (_url: string, init?: RequestInit) => {
          reads++;
          started.resolve();
          await finish.promise;
          expect(init?.signal?.aborted).toBe(false);
          return new Response(body);
        },
      };
      const controller = new AbortController();
      const first = loadBoard("resets", controller.signal, options);
      const second = loadBoard("resets", undefined, options);
      await started.promise;
      controller.abort(new Error("caller cancelled"));
      await expect(first).rejects.toThrow("caller cancelled");
      finish.resolve();
      expect((await second).rows).toHaveLength(4);
      expect(reads).toBe(1);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test.each(["date", "missing", "invalid"])(
    "429 honors %s Retry-After without cached history",
    async (format) => {
      const directory = await mkdtemp(join(tmpdir(), "llmpk-retry-after-test-"));
      try {
        const now = Math.floor(Date.now() / 1000) * 1000;
        const headers =
          format === "date"
            ? { "Retry-After": new Date(now + 180_000).toUTCString() }
            : format === "invalid"
              ? { "Retry-After": "invalid" }
              : undefined;
        const urls: string[] = [];
        const options = {
          cacheDirectory: directory,
          now: () => now,
          request: async (url: string) => {
            urls.push(url);
            return new Response("limited", { status: 429, headers });
          },
        };
        await expect(loadBoard("resets", undefined, options)).rejects.toThrow("retry at");
        await expect(loadBoard("resets", undefined, options)).rejects.toThrow("retry at");
        expect(urls).toHaveLength(2);
        const metadata = JSON.parse(
          await readFile(join(directory, "codex-resets-cooldown.json"), "utf8"),
        );
        expect(metadata.full).toBe(now + (format === "date" ? 180_000 : 600_000));
        expect(metadata.recent).toBe(metadata.full);
      } finally {
        await rm(directory, { recursive: true, force: true });
      }
    },
  );

  test("parser failures stay errors even when an expired full snapshot exists", async () => {
    const directory = await mkdtemp(join(tmpdir(), "llmpk-shape-test-"));
    try {
      const path = join(directory, "codex-resets.json");
      await Bun.write(
        path,
        await Bun.file(new URL("../fixtures/aihot_codex_resets.json", import.meta.url)).text(),
      );
      await utimes(path, new Date(0), new Date(0));
      let requests = 0;
      await expect(
        loadBoard("resets", undefined, {
          cacheDirectory: directory,
          request: async () => {
            requests++;
            return new Response("{}");
          },
        }),
      ).rejects.toThrow("metadata");
      expect(requests).toBe(1);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  test("invalid persisted cooldown dates do not prevent loading full history", async () => {
    const directory = await mkdtemp(join(tmpdir(), "llmpk-invalid-cooldown-test-"));
    try {
      await Bun.write(
        join(directory, "codex-resets-cooldown.json"),
        JSON.stringify({ full: 1e20, recent: 0 }),
      );
      const body = await Bun.file(
        new URL("../fixtures/aihot_codex_resets.json", import.meta.url),
      ).text();
      const result = await loadBoard("resets", undefined, {
        cacheDirectory: directory,
        request: async () => new Response(body),
      });
      expect(result.partial).toBe(false);
      expect(result.rows).toHaveLength(4);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
  test("retries 5xx, never 4xx or aborted requests", async () => {
    let attempts = 0;
    const request = async () => new Response("busy", { status: ++attempts < 3 ? 503 : 200 });
    const delays: number[] = [];
    expect(
      await fetchText("https://example.test", undefined, request, async (ms) => {
        delays.push(ms);
      }),
    ).toBe("busy");
    expect(attempts).toBe(3);
    expect(delays).toEqual([1000, 2000]);
    attempts = 0;
    await expect(
      fetchText("https://example.test", undefined, async () => {
        attempts++;
        return new Response("limited", { status: 429 });
      }),
    ).rejects.toThrow("429");
    expect(attempts).toBe(1);
    const controller = new AbortController();
    controller.abort(new Error("request cancelled"));
    attempts = 0;
    await expect(fetchText("https://example.test", controller.signal, request)).rejects.toThrow(
      "cancelled",
    );
    expect(attempts).toBe(0);
  });

  test("resets cache includes reloads, expires at ten minutes, ignores invalid data", async () => {
    const directory = await mkdtemp(join(tmpdir(), "llmpk-cache-test-"));
    try {
      const body = await Bun.file(
        new URL("../fixtures/aihot_codex_resets.json", import.meta.url),
      ).text();
      let reads = 0;
      const request = async () => {
        reads++;
        return new Response(body);
      };
      const options = { cacheDirectory: directory, request };
      const first = await loadBoard("resets", undefined, options);
      const second = await loadBoard("resets", undefined, options);
      expect(first.rows).toHaveLength(4);
      expect(second.cached).toBe(true);
      expect(reads).toBe(1);
      const mtime = (await stat(join(directory, "codex-resets.json"))).mtimeMs;
      expect(
        (await loadBoard("resets", undefined, { ...options, now: () => mtime - 500 })).cached,
      ).toBe(true);
      expect(reads).toBe(1);
      await loadBoard("resets", undefined, { ...options, now: () => mtime + 600_000 });
      expect(reads).toBe(2);
      await Bun.write(join(directory, "codex-resets.json"), "invalid");
      await loadBoard("resets", undefined, options);
      expect(reads).toBe(3);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
});
