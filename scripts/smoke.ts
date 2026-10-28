import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { BOARDS, VERSION } from "../src/model.ts";

const { values } = parseArgs({ options: { binary: { type: "string" } }, strict: true });
const binary = resolve(values.binary ?? "dist/llmpk");
const directory = await mkdtemp(join(tmpdir(), "llmpk-smoke-"));
try {
  const run = (args: string[]) => {
    const result = Bun.spawnSync([binary, ...args], {
      cwd: directory,
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        TERM: "dumb",
        NO_COLOR: "1",
        XDG_CACHE_HOME: join(directory, "cache"),
      },
    });
    if (result.exitCode !== 0)
      throw new Error(`standalone ${args.join(" ")} failed: ${result.stderr.toString().trim()}`);
    return result.stdout.toString().trimEnd();
  };
  const version = run(["--version"]);
  if (version !== VERSION)
    throw new Error(`binary version ${version} does not match package version ${VERSION}`);
  const sizes = [
    { width: 80, height: 24 },
    { width: 36, height: 8 },
    { width: 160, height: 40 },
  ];
  let count = 0;
  for (const board of BOARDS) {
    for (const { width, height } of sizes) {
      const output = run([
        "--demo",
        "--snapshot",
        "--board",
        board.id,
        "--width",
        `${width}`,
        "--height",
        `${height}`,
      ]);
      const lines = output.split("\n");
      if (
        !output.includes(board.label) ||
        !output.includes("FIXTURE PREVIEW") ||
        lines.length > height ||
        lines.some((line) => Bun.stringWidth(line) > width)
      )
        throw new Error(`invalid ${board.id} snapshot at ${width}x${height}`);
      count++;
    }
  }
  process.stdout.write(
    `pass: standalone version ${version}, ${count} offline snapshots outside the checkout\n`,
  );
} finally {
  await rm(directory, { recursive: true, force: true });
}
