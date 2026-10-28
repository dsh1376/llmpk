import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { stripVTControlCharacters } from "node:util";

function cli(...args: string[]) {
  const result = Bun.spawnSync([process.execPath, "run", "src/cli.tsx", ...args], {
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  return {
    code: result.exitCode,
    output: result.stdout.toString(),
    error: result.stderr.toString(),
  };
}

// Project Ink's cursor and erase sequences onto cells to check the visible frame.
class Viewport {
  x = 0;
  y = 0;
  pending = "";
  cells: string[][];
  constructor(
    public width: number,
    public height: number,
  ) {
    this.cells = Array.from({ length: height }, () => Array<string>(width).fill(" "));
  }
  resize(width: number, height: number) {
    this.cells = Array.from({ length: height }, (_, y) =>
      Array.from({ length: width }, (_, x) => this.cells[y]?.[x] ?? " "),
    );
    this.width = width;
    this.height = height;
    this.x = Math.min(this.x, width - 1);
    this.y = Math.min(this.y, height - 1);
  }
  linefeed() {
    if (++this.y === this.height) {
      this.cells.shift();
      this.cells.push(Array<string>(this.width).fill(" "));
      this.y--;
    }
  }
  write(chunk: string) {
    const input = this.pending + chunk;
    this.pending = "";
    for (let i = 0; i < input.length; ) {
      if (input[i] === "\x1b") {
        const match = /^\[([\d;?]*)([A-Za-z])/u.exec(input.slice(i + 1));
        if (!match) {
          this.pending = input.slice(i);
          break;
        }
        i += match[0].length + 1;
        const args = (match[1] ?? "").replace("?", "").split(";").map(Number);
        const n = args[0] || 1;
        const command = match[2] ?? "";
        if (!"ABCDEFGHfJKmhl".includes(command))
          throw new Error(`unsupported terminal sequence: ${JSON.stringify(match[0])}`);
        if (command === "A") this.y = Math.max(0, this.y - n);
        if (command === "B" || command === "E") this.y = Math.min(this.height - 1, this.y + n);
        if (command === "C") this.x = Math.min(this.width - 1, this.x + n);
        if (command === "D") this.x = Math.max(0, this.x - n);
        if (command === "E" || command === "F") this.x = 0;
        if (command === "F") this.y = Math.max(0, this.y - n);
        if (command === "G") this.x = Math.min(this.width - 1, n - 1);
        if (command === "H" || command === "f") {
          this.y = Math.min(this.height - 1, n - 1);
          this.x = Math.min(this.width - 1, (args[1] || 1) - 1);
        }
        if (command === "J" && args[0] === 2) for (const line of this.cells) line.fill(" ");
        if (command === "J" && args[0] === 0) {
          this.cells[this.y]?.fill(" ", Math.min(this.x, this.width - 1));
          for (let y = this.y + 1; y < this.height; y++) this.cells[y]?.fill(" ");
        }
        if (command === "K")
          this.cells[this.y]?.fill(" ", args[0] === 2 ? 0 : Math.min(this.x, this.width - 1));
        if (!"mhl".includes(command)) this.x = Math.min(this.x, this.width - 1);
        continue;
      }
      const character = String.fromCodePoint(input.codePointAt(i) ?? 0);
      i += character.length;
      if (character === "\r") {
        this.x = 0;
        continue;
      }
      if (character === "\n") {
        this.linefeed();
        continue;
      }
      const width = Bun.stringWidth(character);
      if (!width) continue;
      if (this.x + width > this.width) {
        this.x = 0;
        this.linefeed();
      }
      const line = this.cells[this.y];
      if (!line) throw new Error(`terminal row out of bounds: ${this.y}`);
      line[this.x] = character;
      for (let offset = 1; offset < width; offset++) line[this.x + offset] = "";
      this.x += width;
    }
  }
  lines() {
    return this.cells.map((line) => line.join("").trimEnd());
  }
}

test("interactive resize repaints displaced rows without keyboard input", async () => {
  const directory = await mkdtemp(join(tmpdir(), "llmpk-resize-test-"));
  const viewport = new Viewport(160, 24);
  const decoder = new TextDecoder();
  let received = 0;
  let lastData = 0;
  const terminal = new Bun.Terminal({
    cols: 160,
    rows: 24,
    data(_, data) {
      received += data.byteLength;
      lastData = performance.now();
      viewport.write(decoder.decode(data, { stream: true }));
    },
  });
  const child = Bun.spawn(
    [process.execPath, "run", resolve("src/cli.tsx"), "--demo", "--no-mouse"],
    {
      cwd: directory,
      terminal,
      env: {
        ...process.env,
        CI: "false",
        CONTINUOUS_INTEGRATION: "false",
        TERM: "xterm-256color",
        NO_COLOR: "1",
        FORCE_COLOR: "0",
        XDG_CACHE_HOME: join(directory, "cache"),
      },
    },
  );
  const settled = async (previous: number) => {
    for (let i = 0; i < 200; i++) {
      if (received > previous && performance.now() - lastData > 100) return;
      await Bun.sleep(10);
    }
    throw new Error(`terminal did not repaint: ${viewport.lines().join("\n")}`);
  };
  const checkFrame = (width: number, height: number) => {
    const snapshot = cli("--demo", "--snapshot", "--width", `${width}`, "--height", `${height}`);
    expect(snapshot.code).toBe(0);
    expect(viewport.pending).toBe("");
    expect(viewport.lines()).toEqual(
      stripVTControlCharacters(snapshot.output)
        .trimEnd()
        .split("\n")
        .map((line) => line.trimEnd()),
    );
  };
  try {
    await settled(0);
    checkFrame(160, 24);
    for (const [width, height] of [
      [80, 24],
      [36, 8],
      [160, 40],
      [160, 24],
    ] as const) {
      const previous = received;
      viewport.resize(width, height);
      terminal.resize(width, height);
      // The private PTY has no foreground process group to receive the resize signal.
      child.kill("SIGWINCH");
      await settled(previous);
      checkFrame(width, height);
    }
    terminal.write("q");
    expect(await child.exited).toBe(0);
  } finally {
    child.kill();
    await child.exited;
    terminal.close();
    await rm(directory, { recursive: true, force: true });
  }
});

test("CLI offers terminal-free help, version and fixture snapshots", () => {
  expect(cli("--version")).toEqual({ code: 0, output: "0.6.2\n", error: "" });
  expect(cli("--help").output).toContain("--no-mouse");
  const snapshot = cli(
    "--demo",
    "--snapshot",
    "--board",
    "resets",
    "--width",
    "36",
    "--height",
    "8",
  );
  expect(snapshot.code).toBe(0);
  expect(snapshot.output).toContain("FIXTURE PREVIEW");
  expect(snapshot.output).toContain("Usage");
  expect(snapshot.output.trimEnd().split("\n")).toHaveLength(8);
});
test("CLI rejects invalid options and noninteractive launch before fetching", () => {
  expect(cli("--board", "unknown").error).toContain("--board must be");
  expect(cli("--demo", "--snapshot", "--width", "0").code).toBe(1);
  expect(cli("--demo").error).toContain("interactive terminal required");
});
