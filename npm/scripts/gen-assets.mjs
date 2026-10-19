// Publish-time generator: builds assets.json from the exact five-key matrix
// over the release's asset NAME LIST with hashes computed from the DOWNLOADED
// ASSET BYTES (never by parsing a sidecar). Fails closed naming the missing
// key when any target's asset is absent. deb, pkg and zst names cannot appear
// in the map by construction — the matrix law names them nowhere.

// usage: node gen-assets.mjs --dir <assets-dir> --version <ver> [--out <file>]

import fs from "node:fs";
import path from "node:path";
import { matrix, MATRIX_KEYS } from "../lib/matrix.mjs";
import { sha256Hex } from "../lib/install.mjs";

const argv = process.argv.slice(2);
function flag(name) {
  const at = argv.indexOf(name);
  if (at === -1) return null;
  return argv[at + 1] ?? null;
}

const dir = flag("--dir");
const version = flag("--version");
const out = flag("--out");
if (!dir || !version) {
  console.error("usage: gen-assets.mjs --dir <assets-dir> --version <ver> [--out <file>]");
  process.exit(2);
}

const expected = matrix(version);
const assets = {};
const missing = [];
for (const key of MATRIX_KEYS) {
  const name = expected[key];
  const file = path.join(dir, name);
  if (!fs.existsSync(file)) {
    missing.push(`${key} → ${name}`);
    continue;
  }
  assets[key] = { name, sha256: sha256Hex(fs.readFileSync(file)) };
}
if (missing.length > 0) {
  console.error(`gen-assets: release assets missing — publish fails closed:`);
  for (const line of missing) console.error(`  ${line}`);
  process.exit(1);
}

// Defense-in-depth: the map must never carry a non-installable name.
for (const [key, entry] of Object.entries(assets)) {
  if (/\.(deb|pkg\.tar\.zst|deb\.sha256|pkg\.tar\.zst\.sha256)$/.test(entry.name)) {
    console.error(`gen-assets: ${key} maps to a non-installable asset ${entry.name}`);
    process.exit(1);
  }
}

const doc = { version, releaseTag: `v${version}`, assets };
const json = `${JSON.stringify(doc, null, 2)}\n`;
if (out) fs.writeFileSync(out, json);
else process.stdout.write(json);
