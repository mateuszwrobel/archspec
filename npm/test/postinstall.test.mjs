// Finding M5 (script-blocking package managers): the postinstall leg must be
// best-effort on availability and hard on integrity. Offline install succeeds
// (the bin repeats the contract at first run); a pinned-hash mismatch is a
// red install; a missing key for the host prints its line and defers.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { cacheFile, platformKey, sha256Hex } from "../lib/install.mjs";
import { mkTmp, fixturePackage, fixtureAssets, runBin, fakeBinaryScript } from "./helpers.mjs";

const VER = "6.6.6";

function extractHostArchive(dir, name, into) {
  fs.mkdirSync(into, { recursive: true });
  const tar = spawnSync("tar", ["-xf", path.join(dir, name), "-C", into], { encoding: "utf8" });
  assert.equal(tar.status, 0, tar.stderr);
  return path.join(into, platformKey() === "win32-x64" ? "archspec.exe" : "archspec");
}

test("postinstall offline: exits 0 with the contract, first run stays loud", () => {
  const dir = mkTmp("npm-postinstall-");
  const pkg = fixturePackage({
    dir,
    version: VER,
    downloadBaseUrl: "https://example.invalid/releases/download",
    assets: fixtureAssets(dir, VER),
  });
  const home = path.join(dir, "home");
  fs.mkdirSync(home);
  const res = runBin(path.join(pkg, "scripts/postinstall.mjs"), [], { HOME: home, USERPROFILE: home });
  assert.equal(res.status, 0, `offline install must not fail: ${res.stderr}`);
  assert.ok(res.stderr.includes("--ignore-scripts"), "the contract hint is printed");
  assert.ok(res.stderr.toLowerCase().includes("first run"), "the deferral to first run is stated");
  // The first run then fails loudly.
  const run = runBin(path.join(pkg, "bin/archspec.mjs"), ["--version"], { HOME: home, USERPROFILE: home });
  assert.notEqual(run.status, 0);
  assert.ok(run.stderr.includes(platformKey()));
});

test("postinstall with a valid cache exits 0 and reports the path", () => {
  const dir = mkTmp("npm-postinstall-");
  const key = platformKey();
  const assets = fixtureAssets(dir, VER);
  const pkg = fixturePackage({ dir, version: VER, assets });
  const home = path.join(dir, "home");
  fs.mkdirSync(home);
  const staged = extractHostArchive(dir, assets[key].name, path.join(dir, "extract"));
  const finalPath = cacheFile(VER, key, { HOME: home });
  fs.mkdirSync(path.dirname(finalPath), { recursive: true });
  fs.copyFileSync(staged, finalPath);
  fs.chmodSync(finalPath, 0o755);
  fs.writeFileSync(`${finalPath}.sha256`, `${sha256Hex(fs.readFileSync(finalPath))}\n`);
  const res = runBin(path.join(pkg, "scripts/postinstall.mjs"), [], { HOME: home, USERPROFILE: home });
  assert.equal(res.status, 0, res.stderr);
  assert.ok(res.stdout.includes(finalPath), `reports the binary path: ${res.stdout}`);
});

test("postinstall on a hash mismatch is a red install naming the mismatch", () => {
  const dir = mkTmp("npm-postinstall-");
  const key = platformKey();
  const assets = fixtureAssets(dir, VER);
  const pkg = fixturePackage({ dir, version: VER, assets });
  const home = path.join(dir, "home");
  fs.mkdirSync(home);
  // Integrity failures never defer: seed an entry whose bytes disagree with
  // the digest sidecar recorded at provisioning time.
  const finalPath = cacheFile(VER, key, { HOME: home });
  fs.mkdirSync(path.dirname(finalPath), { recursive: true });
  fs.writeFileSync(finalPath, "wrong bytes", { mode: 0o755 });
  fs.writeFileSync(`${finalPath}.sha256`, `${sha256Hex(Buffer.from(fakeBinaryScript(VER), "utf8"))}\n`);
  const res = runBin(path.join(pkg, "scripts/postinstall.mjs"), [], { HOME: home, USERPROFILE: home });
  assert.notEqual(res.status, 0, "a mismatch must fail the install, not defer it");
  assert.match(res.stderr, /does not match its recorded hash/);
});
