// Findings M2 (shim missing-key message, mismatch hard fail) and M5 (skipped
// scripts + offline first run), plus windows spawn exit-code forwarding (m2):
// the shipping bin shim's user-visible failure contract, exercised end to end.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { platformKey, cacheFile, sha256Hex } from "../lib/install.mjs";
import { matrix } from "../lib/matrix.mjs";
import { mkTmp, fixturePackage, fixtureAssets, runShim, runBin, fakeBinaryScript } from "./helpers.mjs";

const VER = "9.9.9";

test("missing-key path prints the exact line and exits nonzero — never ENOENT", (t) => {
  const dir = mkTmp("npm-missing-");
  const pkg = fixturePackage({ dir, version: VER, assets: fixtureAssets(dir, VER, { omitKey: platformKey() }) });
  const home = path.join(dir, "home");
  fs.mkdirSync(home);
  const res = runShim(pkg, ["--version"], { HOME: home, USERPROFILE: home });
  assert.notEqual(res.status, 0, "must exit nonzero");
  assert.ok(
    res.stderr.includes(`archspec@${VER}: no release asset for ${platformKey()}`),
    `exact missing-key line required, got: ${res.stderr}`,
  );
  assert.ok(!res.stderr.includes("ENOENT"), `never a cryptic ENOENT: ${res.stderr}`);
});

test("cached binary: stdio forwarded, exit code forwarded, hash verified", (t) => {
  const dir = mkTmp("npm-cache-");
  const key = platformKey();
  const name = matrix(VER)[key];
  const assets = fixtureAssets(dir, VER);
  const pkg = fixturePackage({ dir, version: VER, assets });
  const home = path.join(dir, "home");
  fs.mkdirSync(home);
  // Seed the cache with the real archive's extracted binary — the same bytes
  // the pinned hash covers.
  const extractDir = path.join(dir, "extract");
  fs.mkdirSync(extractDir, { recursive: true });
  const archive = path.join(dir, name);
  const tar = spawnSync("tar", ["-xf", archive, "-C", extractDir]);
  assert.equal(tar.status, 0, tar.stderr?.toString());
  const finalPath = cacheFile(VER, key, { HOME: home });
  fs.mkdirSync(path.dirname(finalPath), { recursive: true });
  fs.copyFileSync(path.join(extractDir, key === "win32-x64" ? "archspec.exe" : "archspec"), finalPath);
  fs.chmodSync(finalPath, 0o755);
  fs.writeFileSync(`${finalPath}.sha256`, `${sha256Hex(fs.readFileSync(finalPath))}\n`);

  const res = runShim(pkg, ["--version"], { HOME: home, USERPROFILE: home });
  assert.equal(res.status, 0, `stdout=${res.stdout} stderr=${res.stderr}`);
  assert.equal(res.stdout.trim(), VER, "the binary's stdout reaches the caller");

  // Exit-code forwarding (the windows contract holds on every platform):
  const code = runShim(pkg, ["run"], { HOME: home, USERPROFILE: home, ARCHSPEC_FAKE_EXIT: "7" });
  assert.equal(code.status, 7, "the binary's exit code is the shim's exit code");

  // A corrupted cache is a hard fail naming the mismatch, and nothing is
  // re-fetched or executed.
  fs.appendFileSync(finalPath, "torn");
  const bad = runShim(pkg, ["--version"], { HOME: home, USERPROFILE: home });
  assert.notEqual(bad.status, 0, "a mismatching cache never runs");
  assert.match(bad.stderr, /does not match its recorded hash/);
  assert.ok(!bad.stderr.includes("ENOENT"), `never a cryptic ENOENT: ${bad.stderr}`);
});

test("offline first run prints the loud-failure contract, never ENOENT", () => {
  const dir = mkTmp("npm-offline-");
  const pkg = fixturePackage({
    dir,
    version: VER,
    downloadBaseUrl: "https://example.invalid/releases/download", // RFC 2606: cannot resolve
    assets: fixtureAssets(dir, VER),
  });
  const home = path.join(dir, "home");
  fs.mkdirSync(home);
  const res = runShim(pkg, ["--version"], { HOME: home, USERPROFILE: home });
  assert.notEqual(res.status, 0);
  const err = res.stderr;
  assert.ok(err.includes(platformKey()), "names platform-arch");
  assert.ok(err.includes(VER), "names the version");
  const key = platformKey();
  assert.ok(
    err.includes(`download url: https://example.invalid/releases/download/v${VER}/${matrix(VER)[key]}`),
    `names the exact download URL: ${err}`,
  );
  assert.ok(err.includes("--ignore-scripts"), "names the --ignore-scripts hint");
  assert.ok(!err.includes("ENOENT"), `never a cryptic ENOENT: ${err}`);
  // Nothing was written to the cache by the failed run.
  assert.ok(!fs.existsSync(cacheFile(VER, key, { HOME: home })), "failed download writes no cache entry");
});

test("runBinary forwards codes and never surfaces a raw ENOENT", async (t) => {
  const { runBinary } = await import("../lib/install.mjs");
  // Windows-shape simulation: a stub spawner receives the exact call shape.
  let seen = null;
  const code = runBinary("C:\\cache\\archspec-9.9.9-win32-x64.exe", ["verify", "."], {
    spawn: (cmd, args, opts) => {
      seen = { cmd, args, opts };
      return { status: 3, error: undefined };
    },
  });
  assert.equal(code, 3, "the spawned exit code is the shim's exit code");
  assert.deepEqual(seen.args, ["verify", "."], "arguments are forwarded verbatim");
  assert.equal(seen.opts.stdio, "inherit", "stdio is inherited");
  // A spawn that fails to start is the loud contract, not a raw ENOENT.
  assert.throws(
    () => runBinary("/no/such/archspec", [], { spawn: () => ({ status: null, error: Object.assign(new Error("spawn /no/such/archspec ENOENT"), { code: "ENOENT" }) }) }),
    (err) => err.kind === "unavailable" && !String(err.message).match(/^spawn .*ENOENT$/),
  );
});
