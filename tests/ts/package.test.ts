import { expect, test } from "bun:test";
import {
  chmod,
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  stat,
  unlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { packageBinary, RELEASE_TARGETS, verifyAssets } from "../../scripts/package.ts";

test("release archives retain the executable and verify every expected asset", async () => {
  const directory = await mkdtemp(join(tmpdir(), "llmpk-package-test-"));
  try {
    const binary = join(directory, "llmpk");
    await writeFile(binary, "fixture executable\n");
    await chmod(binary, 0o755);
    const release = join(directory, "release");
    await mkdir(release);
    await copyFile("install.sh", join(release, "install.sh"));
    for (const target of RELEASE_TARGETS) {
      const archive = await packageBinary(target, directory);
      await copyFile(archive, join(release, basename(archive)));
      await copyFile(`${archive}.sha256`, join(release, `${basename(archive)}.sha256`));
    }
    expect(await verifyAssets(release)).toHaveLength(5);
    const archive = join(release, `llmpk-${RELEASE_TARGETS[0]}.tar.gz`);
    const listing = Bun.spawnSync(["tar", "-tzf", archive]);
    expect(listing.exitCode).toBe(0);
    expect(listing.stdout.toString().trim()).toBe("llmpk");
    const content = Bun.spawnSync(["tar", "-xOzf", archive, "llmpk"]);
    expect(content.stdout.toString()).toBe("fixture executable\n");
    const extracted = join(directory, "extracted");
    await mkdir(extracted);
    expect(Bun.spawnSync(["tar", "-xzf", archive, "-C", extracted]).exitCode).toBe(0);
    expect((await stat(join(extracted, "llmpk"))).mode & 0o111).toBe(0o111);
    const checksum = `${archive}.sha256`;
    const original = await readFile(checksum, "utf8");
    await writeFile(checksum, original.replace(/^[a-f0-9]{64}/, "0".repeat(64)));
    await expect(verifyAssets(release)).rejects.toThrow("checksum");
    await writeFile(checksum, original);
    await writeFile(join(release, "unexpected.txt"), "not a release asset");
    await expect(verifyAssets(release)).rejects.toThrow("unexpected");
    await unlink(join(release, "unexpected.txt"));
    await unlink(checksum);
    await expect(verifyAssets(release)).rejects.toThrow("missing");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
