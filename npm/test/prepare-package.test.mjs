// Finding B2 (publish-time identity injection): the pre-injection staged tree
// is neutral (sentinels only), and the post-injection packed manifest carries
// the exact repository.url and download base URL. Leftovers fail closed.
// The grep of the staged tree against the publish guard's identifier list
// runs outside the payload, in the project's internal test suite — the list
// belongs to the publish workflow and a payload that names it leaks it.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { mkTmp, NPM_DIR } from "./helpers.mjs";

const PREPARE = path.join(NPM_DIR, "scripts/prepare-package.mjs");

function walk(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) files.push(...walk(p));
    else files.push(p);
  }
  return files;
}

test("pre-injection staging is structurally neutral and packs without tests", () => {
  const dir = mkTmp("npm-prepare-");
  const out = path.join(dir, "neutral");
  const res = spawnSync(process.execPath, [PREPARE, "--out", out], { encoding: "utf8" });
  assert.equal(res.status, 0, res.stderr);
  // Structural neutrality of the staged tree: public release host or a
  // reserved/synthetic test sentinel — nothing else may be named.
  const allowedHosts = ["github.com", "example.invalid", "release.example"];
  for (const file of walk(out)) {
    if (file.endsWith("LICENSE")) continue; // the license text is the crate's, not npm-tree prose
    const bytes = fs.readFileSync(file, "utf8");
    let rest = bytes;
    for (;;) {
      const at = rest.indexOf("://");
      if (at === -1) break;
      const tail = rest.slice(at + 3);
      const host = /^[A-Za-z0-9.-]+/.exec(tail)?.[0] ?? "";
      if (host === "") {
        rest = tail; // a bare "://" literal (a scanner's own needle), not a URL
        continue;
      }
      assert.ok(allowedHosts.includes(host), `host ${host} in ${file} is outside the public allow-list`);
      rest = tail;
    }
  }
  const manifest = JSON.parse(fs.readFileSync(path.join(out, "package.json"), "utf8"));
  assert.equal(manifest.name, "@scope/archspec");
  assert.equal(manifest.version, "0.0.0");
  // The packed manifest (npm pack) must also stay clean pre-injection.
  const pack = spawnSync("npm", ["pack", "--pack-destination", dir, out], { encoding: "utf8" });
  assert.equal(pack.status, 0, pack.stderr);
  const tgz = path.join(dir, pack.stdout.trim().split("\n").pop());
  const list = spawnSync("tar", ["-tzf", tgz], { encoding: "utf8" });
  assert.equal(list.status, 0);
  assert.ok(list.stdout.split("\n").some((n) => n.endsWith("package/bin/archspec.mjs")), `packed tree: ${list.stdout}`);
  assert.ok(!list.stdout.includes("test/"), "tests are not packed");
});

test("post-injection manifest carries the exact identity values", () => {
  const dir = mkTmp("npm-prepare-");
  const assets = path.join(dir, "assets.json");
  fs.writeFileSync(assets, JSON.stringify({ version: "1.2.3", releaseTag: "v1.2.3", assets: {} }));
  const out = path.join(dir, "injected");
  const res = spawnSync(
    process.execPath,
    [PREPARE, "--out", out, "--scope", "fixture-scope", "--version", "1.2.3", "--repository", "octocat/archspec-demo", "--assets", assets],
    { encoding: "utf8" },
  );
  assert.equal(res.status, 0, res.stderr);
  const manifest = JSON.parse(fs.readFileSync(path.join(out, "package.json"), "utf8"));
  assert.equal(manifest.name, "@fixture-scope/archspec");
  assert.equal(manifest.version, "1.2.3");
  assert.equal(manifest.repository.url, "https://github.com/octocat/archspec-demo.git");
  assert.equal(manifest.config.downloadBaseUrl, "https://github.com/octocat/archspec-demo/releases/download");
  assert.ok(fs.existsSync(path.join(out, "assets.json")), "the generated assets.json ships");
  const bytes = fs.readFileSync(path.join(out, "package.json"), "utf8");
  assert.ok(!bytes.includes("example.invalid"), "no neutral URL sentinel survives");
  assert.ok(!bytes.includes('"0.0.0"'), "no neutral version survives");
  assert.ok(!bytes.includes("@scope/archspec"), "no neutral name survives");
});

test("injection fails closed on partial flags, bad scope, bad version, missing assets", () => {
  const dir = mkTmp("npm-prepare-");
  const out = path.join(dir, "out");
  const partial = spawnSync(process.execPath, [PREPARE, "--out", out, "--scope", "x"], { encoding: "utf8" });
  assert.equal(partial.status, 2, "partial identity flags are a usage error");
  const noAssets = spawnSync(
    process.execPath,
    [PREPARE, "--out", out, "--scope", "x", "--version", "1.0.0", "--repository", "octocat/archspec-demo"],
    { encoding: "utf8" },
  );
  assert.equal(noAssets.status, 2, "injection without a generated assets.json is a usage error");
  const badScope = spawnSync(
    process.execPath,
    [PREPARE, "--out", out, "--scope", "Bad Scope", "--version", "1.0.0", "--repository", "octocat/archspec-demo", "--assets", "/dev/null"],
    { encoding: "utf8" },
  );
  assert.equal(badScope.status, 1, "an uppercase/spaced scope is refused");
  const badVersion = spawnSync(
    process.execPath,
    [PREPARE, "--out", out, "--scope", "ok", "--version", "0.5.2-gap", "--repository", "octocat/archspec-demo", "--assets", "/dev/null"],
    { encoding: "utf8" },
  );
  assert.equal(badVersion.status, 1, "a suffix version (npm semver, outside the tag shape) is refused");
});
