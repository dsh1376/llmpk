import { mkdir, readFile, rename, stat, unlink, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { BOARDS, type BoardId, type Row, VERSION } from "../model.ts";
import { parseSource } from "./sources.ts";

class HttpError extends Error {
  constructor(
    readonly status: number,
    url: string,
    readonly retryAfter: string | null = null,
  ) {
    super(`HTTP ${status} from ${new URL(url).hostname}`);
  }
}
const wait = (ms: number, signal?: AbortSignal) =>
  new Promise<void>((resolve, reject) => {
    const abort = () => {
      clearTimeout(timer);
      reject(signal?.reason ?? new Error("request cancelled"));
    };
    const timer = setTimeout(() => {
      signal?.removeEventListener("abort", abort);
      resolve();
    }, ms);
    if (signal?.aborted) abort();
    else signal?.addEventListener("abort", abort, { once: true });
  });

export type Request = (url: string, init?: RequestInit) => Promise<Response>;
export async function fetchText(
  url: string,
  signal?: AbortSignal,
  request: Request = fetch,
  delay: (ms: number, signal?: AbortSignal) => Promise<void> = wait,
): Promise<string> {
  for (let attempt = 0; ; attempt++) {
    signal?.throwIfAborted();
    try {
      const response = await request(url, {
        signal: signal
          ? AbortSignal.any([signal, AbortSignal.timeout(20_000)])
          : AbortSignal.timeout(20_000),
        headers: {
          "User-Agent": `Mozilla/5.0 (compatible; llmpk/${VERSION})`,
          Accept: "text/html,application/json,*/*",
        },
      });
      if (!response.ok)
        throw new HttpError(response.status, url, response.headers.get("Retry-After"));
      return await response.text();
    } catch (error) {
      if (signal?.aborted || attempt >= 2 || (error instanceof HttpError && error.status < 500))
        throw error;
      await delay((attempt + 1) * 1000, signal);
    }
  }
}

export interface LoadResult {
  rows: Row[];
  cached: boolean;
  partial?: boolean;
  warning?: string;
}
interface LoadOptions {
  cacheDirectory?: string | null;
  request?: Request;
  now?: () => number;
}
export function cacheDirectory(): string {
  return join(process.env.XDG_CACHE_HOME || join(homedir(), ".cache"), "llmpk");
}

const RESET_TTL = 600_000;
interface Snapshot {
  rows: Row[];
  modified: number;
}
interface Cooldown {
  full: number;
  recent: number;
}
const cooldowns = new Map<string, Cooldown>();

async function readSnapshot(path: string | null): Promise<Snapshot | undefined> {
  if (!path) return undefined;
  try {
    const [body, metadata] = await Promise.all([readFile(path, "utf8"), stat(path)]);
    return { rows: parseSource("resets", body), modified: Math.floor(metadata.mtimeMs) };
  } catch {
    return undefined;
  }
}

async function writeCache(path: string | null, body: string): Promise<string | undefined> {
  if (!path) return undefined;
  const temporary = `${path}.${process.pid}.${crypto.randomUUID()}.tmp`;
  try {
    await mkdir(dirname(path), { recursive: true });
    await writeFile(temporary, body, { mode: 0o600 });
    await rename(temporary, path);
    return undefined;
  } catch (error) {
    await unlink(temporary).catch(() => undefined);
    return `Cache write failed: ${error instanceof Error ? error.message : String(error)}`;
  }
}

function retryAt(error: HttpError, now: number): number {
  if (error.retryAfter && /^\d+$/.test(error.retryAfter)) {
    const deadline = now + Number(error.retryAfter) * 1000;
    if (Number.isFinite(new Date(deadline).getTime())) return deadline;
  }
  const date = error.retryAfter ? Date.parse(error.retryAfter) : NaN;
  return Number.isFinite(date) ? Math.max(now, date) : now + RESET_TTL;
}
const retryTime = (time: number) => `${new Date(time).toISOString().slice(11, 19)} UTC`;

async function loadResets(
  url: string,
  signal: AbortSignal,
  options: LoadOptions,
): Promise<LoadResult> {
  const directory =
    options.cacheDirectory === undefined ? cacheDirectory() : options.cacheDirectory;
  const key = directory ?? "no-reset-cache";
  const fullPath = directory ? join(directory, "codex-resets.json") : null;
  const recentPath = directory ? join(directory, "codex-resets-recent.json") : null;
  const cooldownPath = directory ? join(directory, "codex-resets-cooldown.json") : null;
  const now = options.now ?? Date.now;
  const full = await readSnapshot(fullPath);
  if (full && Math.abs(now() - full.modified) < RESET_TTL)
    return { rows: full.rows, cached: true, partial: false };
  const cooldown = { ...(cooldowns.get(key) ?? { full: 0, recent: 0 }) };
  if (cooldownPath) {
    try {
      const saved = JSON.parse(await readFile(cooldownPath, "utf8"));
      for (const scope of ["full", "recent"] as const) {
        if (typeof saved[scope] === "number" && Number.isFinite(new Date(saved[scope]).getTime()))
          cooldown[scope] = Math.max(cooldown[scope], saved[scope]);
      }
    } catch {
      /* Missing or invalid cooldown metadata does not invalidate a snapshot. */
    }
  }
  let cacheWarning: string | undefined;
  const saveCooldown = async () => {
    cooldowns.set(key, cooldown);
    cacheWarning = await writeCache(cooldownPath, JSON.stringify(cooldown));
  };
  if (cooldown.full <= now()) {
    try {
      const body = await fetchText(url, signal, options.request);
      const rows = parseSource("resets", body);
      return { rows, cached: false, partial: false, warning: await writeCache(fullPath, body) };
    } catch (error) {
      if (!(error instanceof HttpError) || error.status !== 429) throw error;
      cooldown.full = retryAt(error, now());
      await saveCooldown();
    }
  }
  signal.throwIfAborted();
  const warning = `${full ? "Cached history;" : "Recent events only; full history"} rate limited until ${retryTime(cooldown.full)}${cacheWarning ? `; ${cacheWarning}` : ""}`;
  if (full) return { rows: full.rows, cached: true, partial: false, warning };
  const recent = await readSnapshot(recentPath);
  if (recent && Math.abs(now() - recent.modified) < RESET_TTL)
    return { rows: recent.rows, cached: true, partial: true, warning };
  if (cooldown.recent > now())
    throw new Error(`HTTP 429 from aihot.news; retry at ${retryTime(cooldown.recent)}`);
  try {
    const body = await fetchText(`${url}/recent`, signal, options.request);
    const rows = parseSource("resets", body);
    const recentWarning = await writeCache(recentPath, body);
    return {
      rows,
      cached: false,
      partial: true,
      warning: `${warning}${recentWarning ? `; ${recentWarning}` : ""}`,
    };
  } catch (error) {
    if (!(error instanceof HttpError) || error.status !== 429) throw error;
    cooldown.recent = retryAt(error, now());
    await saveCooldown();
    throw new Error(
      `HTTP 429 from aihot.news; retry at ${retryTime(cooldown.recent)}${cacheWarning ? `; ${cacheWarning}` : ""}`,
    );
  }
}

interface ResetRequest {
  controller: AbortController;
  promise: Promise<LoadResult>;
  users: number;
  settled: boolean;
}
const resetRequests = new Map<string, ResetRequest>();

function sharedResets(
  url: string,
  signal: AbortSignal | undefined,
  options: LoadOptions,
): Promise<LoadResult> {
  signal?.throwIfAborted();
  const key =
    options.cacheDirectory === undefined
      ? cacheDirectory()
      : (options.cacheDirectory ?? "no-reset-cache");
  let request = resetRequests.get(key);
  if (!request || request.controller.signal.aborted) {
    const controller = new AbortController();
    request = {
      controller,
      promise: loadResets(url, controller.signal, options),
      users: 0,
      settled: false,
    };
    resetRequests.set(key, request);
    const current = request;
    const settled = () => {
      current.settled = true;
      if (resetRequests.get(key) === current) resetRequests.delete(key);
    };
    void current.promise.then(settled, settled);
  }
  const current = request;
  current.users++;
  return new Promise((resolve, reject) => {
    let finished = false;
    const cleanup = () => {
      if (finished) return false;
      finished = true;
      signal?.removeEventListener("abort", abort);
      if (--current.users === 0 && !current.settled) current.controller.abort();
      return true;
    };
    const abort = () => {
      if (cleanup()) reject(signal?.reason ?? new Error("request cancelled"));
    };
    signal?.addEventListener("abort", abort, { once: true });
    void current.promise.then(
      (result) => {
        if (cleanup()) resolve(result);
      },
      (error) => {
        if (cleanup()) reject(error);
      },
    );
  });
}

export async function loadBoard(
  id: BoardId,
  signal?: AbortSignal,
  options: LoadOptions = {},
): Promise<LoadResult> {
  const board = BOARDS.find((board) => board.id === id);
  if (!board) throw new Error(`unknown board: ${id}`);
  if (id === "resets") return sharedResets(board.url, signal, options);
  let body = await fetchText(board.url, signal, options.request);
  if (id === "slop") {
    const path = /src=["'](\/assets\/index-[\w-]+\.js)["']/.exec(body)?.[1];
    if (!path) throw new Error("no app bundle linked from slopalytics.com");
    body = await fetchText(new URL(path, board.url).href, signal, options.request);
  }
  const rows = parseSource(id, body);
  return { rows, cached: false };
}
