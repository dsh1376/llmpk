import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({ options: { target: { type: "string" } }, strict: true });
const target = values.target;
if (target && !["bun-linux-x64", "bun-darwin-arm64"].includes(target)) {
  throw new Error("supported build targets: bun-linux-x64, bun-darwin-arm64");
}
await mkdir("dist", { recursive: true });
const output = resolve("dist/llmpk");
// Keep Ink's optional devtools module behind its runtime availability check.
const args = [
  process.execPath,
  "build",
  "--compile",
  "--minify",
  "--no-compile-autoload-dotenv",
  "--no-compile-autoload-bunfig",
  "--external",
  "*/devtools.js",
  "src/cli.tsx",
  "--outfile",
  output,
];
if (target) args.push(`--target=${target}`);
const build = Bun.spawn(args, { stdout: "inherit", stderr: "inherit" });
if ((await build.exited) !== 0) process.exit(1);
if (process.platform === "darwin" && (!target || target === "bun-darwin-arm64")) {
  // Bun 1.4 can leave an invalid Mach-O signature after appending the bundle.
  const sign = Bun.spawn(
    ["codesign", "--force", "--sign", "-", "--preserve-metadata=entitlements", output],
    { stdout: "inherit", stderr: "inherit" },
  );
  if ((await sign.exited) !== 0) process.exit(1);
}
