import { execFileSync } from "node:child_process";
import { appendFile, readFile } from "node:fs/promises";

function validateTag(tag: string): void {
  if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(tag)) {
    throw new Error(
      "release tag must have the form vX.Y.Z, with an optional prerelease or build suffix",
    );
  }
}

export function validateReleaseVersion(tag: string, version: unknown) {
  validateTag(tag);
  if (typeof version !== "string" || tag !== `v${version}`) {
    throw new Error(`release tag ${tag} does not match package.json version ${String(version)}`);
  }
  return { version, prerelease: version.split("+")[0]?.includes("-") ?? false };
}

function command(program: string, ...args: string[]): string {
  try {
    return execFileSync(program, args, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  } catch (error) {
    throw new Error(
      `cannot read release source: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

export function resolveRelease(tag: string) {
  validateTag(tag);
  const sha = command("git", "rev-parse", "--verify", `refs/tags/${tag}^{commit}`);
  if (!/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(sha))
    throw new Error(`invalid commit SHA for tag ${tag}`);
  let manifest: { version?: unknown };
  try {
    manifest = JSON.parse(command("git", "show", `${sha}:package.json`));
  } catch (error) {
    throw new Error(
      `tag ${tag} must contain a readable package.json: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  return { tag, sha, ...validateReleaseVersion(tag, manifest.version) };
}

function remoteCommit(tag: string): string {
  const repository = process.env.GITHUB_REPOSITORY;
  if (!repository || !/^[\w.-]+\/[\w.-]+$/.test(repository))
    throw new Error("GITHUB_REPOSITORY is required to verify the remote tag");
  const api = (path: string) => JSON.parse(command("gh", "api", `repos/${repository}/git/${path}`));
  let object = api(`ref/tags/${encodeURIComponent(tag)}`).object;
  for (let depth = 0; depth < 8; depth++) {
    if (!object || !/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(object.sha))
      throw new Error(`invalid GitHub tag object for ${tag}`);
    if (object.type === "commit") return object.sha;
    if (object.type !== "tag") throw new Error(`tag ${tag} does not point to a commit`);
    object = api(`tags/${object.sha}`).object;
  }
  throw new Error(`too many annotated tag references for ${tag}`);
}

async function main() {
  const [command, tag, expectedSha] = process.argv.slice(2);
  if (!tag || !["resolve", "verify", "version"].includes(command ?? "")) {
    throw new Error("usage: release.ts resolve <tag> | verify <tag> <sha> | version <tag>");
  }
  if (command === "version") {
    const manifest = JSON.parse(await readFile("package.json", "utf8"));
    validateReleaseVersion(tag, manifest.version);
    process.stdout.write(`pass: ${tag} matches package.json\n`);
    return;
  }
  const release = resolveRelease(tag);
  if (command === "verify") {
    if (release.sha !== expectedSha)
      throw new Error(`tag ${tag} moved: expected ${expectedSha}, got ${release.sha}`);
    const remote = remoteCommit(tag);
    if (remote !== expectedSha)
      throw new Error(`remote tag ${tag} moved: expected ${expectedSha}, got ${remote}`);
    process.stdout.write(`pass: remote ${tag} still points to ${release.sha}\n`);
    return;
  }
  if (process.env.GITHUB_OUTPUT) {
    await appendFile(
      process.env.GITHUB_OUTPUT,
      Object.entries(release)
        .map(([key, value]) => `${key}=${value}\n`)
        .join(""),
    );
  }
  process.stdout.write(`${JSON.stringify(release)}\n`);
}

if (import.meta.main) {
  try {
    await main();
  } catch (error) {
    process.stderr.write(`release: ${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
