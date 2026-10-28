import { expect, test } from "bun:test";
import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { validateReleaseVersion } from "../../scripts/release.ts";

test("release tags must match the manifest version", () => {
  expect(validateReleaseVersion("v1.2.3", "1.2.3")).toEqual({
    version: "1.2.3",
    prerelease: false,
  });
  expect(validateReleaseVersion("v1.2.3-rc.1", "1.2.3-rc.1").prerelease).toBe(true);
  expect(validateReleaseVersion("v1.2.3+build-linux", "1.2.3+build-linux").prerelease).toBe(false);
  expect(() => validateReleaseVersion("v1.2.4", "1.2.3")).toThrow("does not match");
  for (const tag of ["main", "v1", "v1.2.3\nsha=bad", "refs/tags/v1.2.3", "v1.2.3;exit 0"]) {
    expect(() => validateReleaseVersion(tag, "1.2.3")).toThrow("tag");
  }
});

test("release preparation reads the tagged manifest and rejects a moved tag before publish", async () => {
  const directory = await mkdtemp(join(tmpdir(), "llmpk-release-test-"));
  try {
    // Git is the external boundary. The fixture never creates or changes a repository.
    const git = join(directory, "git");
    await writeFile(
      git,
      `#!/bin/sh
set -eu
case "$1" in
  rev-parse)
    [ "$3" = 'refs/tags/v1.2.3^{commit}' ]
    printf '%s\\n' "$LLMPK_TEST_SHA"
    ;;
  show)
    [ "$2" = "$LLMPK_TEST_SHA:package.json" ]
    printf '{"version":"%s"}\\n' "$LLMPK_TEST_VERSION"
    ;;
  *) exit 2 ;;
esac
`,
    );
    await chmod(git, 0o755);
    const gh = join(directory, "gh");
    await writeFile(
      gh,
      `#!/bin/sh
set -eu
[ "$1" = api ]
if [ "$LLMPK_TEST_REMOTE_SHA" = error ]; then
  printf '%s\\n' 'fixture API unavailable' >&2
  exit 1
fi
case "$2" in
  repos/owner/repo/git/ref/tags/v1.2.3)
    printf '{"object":{"type":"%s","sha":"%s"}}\\n' "$LLMPK_TEST_REF_TYPE" "$LLMPK_TEST_REF_SHA"
    ;;
  repos/owner/repo/git/tags/*)
    printf '{"object":{"type":"commit","sha":"%s"}}\\n' "$LLMPK_TEST_REMOTE_SHA"
    ;;
  *) exit 2 ;;
esac
`,
    );
    await chmod(gh, 0o755);
    const sha = "a".repeat(40);
    const output = join(directory, "outputs");
    const run = (
      action: string,
      version: string,
      expected?: string,
      remote = sha,
      annotated = false,
    ) =>
      Bun.spawnSync(
        [
          process.execPath,
          "run",
          "scripts/release.ts",
          action,
          "v1.2.3",
          ...(expected ? [expected] : []),
        ],
        {
          stdin: "ignore",
          stdout: "pipe",
          stderr: "pipe",
          env: {
            ...process.env,
            PATH: `${directory}:${process.env.PATH}`,
            GITHUB_OUTPUT: output,
            LLMPK_TEST_SHA: sha,
            LLMPK_TEST_VERSION: version,
            GITHUB_REPOSITORY: "owner/repo",
            LLMPK_TEST_REMOTE_SHA: remote,
            LLMPK_TEST_REF_SHA: annotated ? "c".repeat(40) : remote,
            LLMPK_TEST_REF_TYPE: annotated ? "tag" : "commit",
          },
        },
      );
    const prepared = run("resolve", "1.2.3");
    expect(prepared.exitCode).toBe(0);
    expect(JSON.parse(prepared.stdout.toString())).toMatchObject({
      tag: "v1.2.3",
      sha,
      version: "1.2.3",
    });
    expect(await readFile(output, "utf8")).toContain(`sha=${sha}\n`);
    expect(run("resolve", "1.2.4").stderr.toString()).toContain("does not match");
    expect(run("verify", "1.2.3", "b".repeat(40)).exitCode).toBe(1);
    expect(run("verify", "1.2.3", sha).exitCode).toBe(0);
    expect(run("verify", "1.2.3", sha, "b".repeat(40)).exitCode).toBe(1);
    expect(run("verify", "1.2.3", sha, sha, true).exitCode).toBe(0);
    expect(run("verify", "1.2.3", sha, "error").stderr.toString()).toContain(
      "fixture API unavailable",
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
