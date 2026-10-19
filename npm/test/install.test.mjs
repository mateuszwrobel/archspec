// Findings m1 (zero api.github.com calls, one download per first-run) and
// m2 (cache atomicity under concurrent first runs), exercised on the shared
// install path with an injected fetcher.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { obtain, cacheFile, platformKey } from "../lib/install.mjs";
import { mkTmp, fixturePackage, fixtureAssets, fakeBinaryScript } from "./helpers.mjs";

const VER = "8.8.8";

function setup(dir, { downloadBaseUrl = "https://release.example/releases/download" } = {}) {
  const assets = fixtureAssets(dir, VER);
  const pkg = fixturePackage({ dir, version: VER, downloadBaseUrl, assets });
  const home = path.join(dir, "home");
  fs.mkdirSync(home);
  const key = platformKey();
  // The fetcher reads the archive bytes from the fixture dir but records the
  // URL the product asked for — so the URL contract is observable.
  const urls = [];
  const fetch = async (url) => {
    urls.push(url);
    const name = url.slice(url.lastIndexOf("/") + 1);
    return fs.readFileSync(path.join(dir, name));
  };
  return { pkg, home, key, assets, urls, fetch };
}

test("first run downloads exactly the release-asset URL — zero API calls", async () => {
  const dir = mkTmp("npm-url-");
  const { pkg, home, key, fetch, urls } = setup(dir);
  const res = await obtain({ pkgRoot: pkg, env: { HOME: home, USERPROFILE: home }, fetch });
  assert.equal(urls.length, 1, `one download, got ${urls.length}`);
  assert.equal(
    urls[0],
    `https://release.example/releases/download/v${VER}/${JSON.parse(fs.readFileSync(path.join(pkg, "assets.json"))).assets[key].name}`,
    "the download URL is <base>/<tag>/<asset>",
  );
  for (const url of urls) assert.ok(!url.includes("api.github.com"), `no API calls: ${url}`);
  assert.equal(res.binaryPath, cacheFile(VER, key, { HOME: home }));
  const cached = fs.readFileSync(res.binaryPath);
  assert.deepEqual(
    cached,
    Buffer.from(fakeBinaryScript(VER), "utf8"),
    "the cached binary is the extracted fixture member (the archive pin was verified on the archive bytes before extraction)",
  );
  const leftovers = fs.readdirSync(path.dirname(res.binaryPath)).filter((n) => n.startsWith(".dl-") || n.startsWith(".stage-") || n === ".lock");
  assert.deepEqual(leftovers, [], `no temp or lock residue: ${leftovers}`);
});

test("downloaded-bytes mismatch hard-fails: nothing cached, nonzero, named", async () => {
  const dir = mkTmp("npm-mismatch-");
  const { pkg, home, key, urls } = setup(dir);
  const fetch = async (url) => {
    urls.push(url);
    return Buffer.from("tampered bytes, not the release asset");
  };
  await assert.rejects(
    obtain({ pkgRoot: pkg, env: { HOME: home, USERPROFILE: home }, fetch }),
    (err) => {
      assert.equal(err.kind, "mismatch", `kind must be mismatch, got ${err.kind}`);
      assert.match(err.lines.join("\n"), /does not match its pinned hash/);
      return true;
    },
  );
  assert.equal(urls.length, 1, "one download attempt");
  assert.ok(!fs.existsSync(cacheFile(VER, key, { HOME: home })), "mismatch writes nothing to the cache");
  const cacheDir = path.dirname(cacheFile(VER, key, { HOME: home }));
  const leftovers = fs.readdirSync(cacheDir).filter((n) => n.includes("tampered") || n.startsWith(".dl-") || n.startsWith(".stage-"));
  assert.deepEqual(leftovers, [], "no residue from the failed download");
});

test("20 concurrent first runs share exactly one atomic cache entry", async () => {
  const dir = mkTmp("npm-concurrent-");
  const { pkg, home, key, fetch, urls } = setup(dir);
  // Slow the fetcher so all 20 tasks race through the same first-run window.
  const slowFetch = async (url) => {
    await new Promise((r) => setTimeout(r, 40));
    return fetch(url);
  };
  const results = await Promise.all(
    Array.from({ length: 20 }, () => obtain({ pkgRoot: pkg, env: { HOME: home, USERPROFILE: home }, fetch: slowFetch })),
  );
  assert.equal(urls.length, 1, `the lock collapses the race to one download, got ${urls.length}`);
  const finalPath = cacheFile(VER, key, { HOME: home });
  for (const r of results) assert.equal(r.binaryPath, finalPath, "every task resolves to the one cache file");
  assert.deepEqual(
    fs.readFileSync(finalPath),
    Buffer.from(fakeBinaryScript(VER), "utf8"),
    "one complete verified binary",
  );
  const entries = fs.readdirSync(path.dirname(finalPath)).sort();
  assert.deepEqual(
    entries,
    [path.basename(finalPath), `${path.basename(finalPath)}.sha256`].sort(),
    `exactly one cache entry plus its digest sidecar: ${entries}`,
  );
});
