import { createHash } from "node:crypto";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

export const RELEASE_TARGETS = ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"] as const;
type ReleaseTarget = (typeof RELEASE_TARGETS)[number];
const archiveName = (target: string) => `llmpk-${target}.tar.gz`;
const digest = async (path: string) =>
  createHash("sha256")
    .update(await readFile(path))
    .digest("hex");

export async function packageBinary(target: ReleaseTarget, directory = "dist"): Promise<string> {
  if (!RELEASE_TARGETS.includes(target)) throw new Error(`unsupported package target: ${target}`);
  await mkdir(directory, { recursive: true });
  const asset = archiveName(target);
  const archive = resolve(directory, asset);
  const tar = Bun.spawnSync(["tar", "-czf", archive, "-C", resolve(directory), "llmpk"], {
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  if (tar.exitCode !== 0)
    throw new Error(`cannot package ${target}: ${tar.stderr.toString().trim()}`);
  await writeFile(`${archive}.sha256`, `${await digest(archive)}  ${asset}\n`);
  return archive;
}

export async function verifyAssets(directory = "dist"): Promise<string[]> {
  const expected = [
    "install.sh",
    ...RELEASE_TARGETS.flatMap((target) => [archiveName(target), `${archiveName(target)}.sha256`]),
  ];
  const actual = await readdir(directory);
  const unexpected = actual.filter((name) => !expected.includes(name));
  const missing = expected.filter((name) => !actual.includes(name));
  if (unexpected.length) throw new Error(`unexpected release assets: ${unexpected.join(", ")}`);
  if (missing.length) throw new Error(`missing release assets: ${missing.join(", ")}`);
  for (const target of RELEASE_TARGETS) {
    const asset = archiveName(target);
    const checksum = (await readFile(join(directory, `${asset}.sha256`), "utf8")).trim();
    if (checksum !== `${await digest(join(directory, asset))}  ${asset}`)
      throw new Error(`checksum mismatch: ${asset}`);
    const listing = Bun.spawnSync(["tar", "-tzf", join(directory, asset)], {
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    });
    if (listing.exitCode !== 0 || listing.stdout.toString().trim() !== "llmpk")
      throw new Error(`invalid archive contents: ${asset}`);
  }
  return expected.map((name) => join(directory, name));
}

async function main() {
  const { values } = parseArgs({
    options: {
      target: { type: "string" },
      verify: { type: "boolean" },
      directory: { type: "string" },
    },
    strict: true,
  });
  const directory = values.directory ?? "dist";
  if (values.verify) {
    const assets = await verifyAssets(directory);
    process.stdout.write(`pass: ${assets.length} release assets, checksums and archive contents\n`);
  } else {
    if (!RELEASE_TARGETS.includes(values.target as ReleaseTarget))
      throw new Error(`--target must be ${RELEASE_TARGETS.join(" or ")}`);
    process.stdout.write(`${await packageBinary(values.target as ReleaseTarget, directory)}\n`);
  }
}

if (import.meta.main) {
  try {
    await main();
  } catch (error) {
    process.stderr.write(`package: ${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
