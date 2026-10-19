// Shared fixture plumbing for the adversarial npm test suite. Everything is
// hermetic: temp dirs via fs.mkdtempSync, a fake tar archive built with the
// host tar, offline paths aimed at the RFC-2606 reserved .invalid domain.

import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { matrix } from "../lib/matrix.mjs";

export const NPM_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export function sha256Hex(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function mkTmp(prefix = "npm-test-") {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

/// A fake archspec binary: a node script echoing argv and exiting with the
/// code in ARCHSPEC_FAKE_EXIT (default 0).
export function fakeBinaryScript(version = "9.9.9") {
  return [
    "#!/usr/bin/env node",
    `if (process.argv.includes("--version")) { console.log("${version}"); process.exit(0); }`,
    'console.log("args:" + process.argv.slice(2).join(","));',
    "process.exit(Number(process.env.ARCHSPEC_FAKE_EXIT || 0));",
    "",
  ].join("\n");
}

/// Build a .tar.gz containing the fake binary; returns { file, content }
/// where content is the binary bytes the hash of the archive must match.
export function fakeTarball(dir, name, version) {
  const binDir = path.join(dir, ".build");
  fs.mkdirSync(binDir, { recursive: true });
  const bin = path.join(binDir, "archspec");
  fs.writeFileSync(bin, fakeBinaryScript(version), { mode: 0o755 });
  const file = path.join(dir, name);
  const res = spawnSync("tar", ["-czf", file, "-C", binDir, "archspec"], { stdio: "inherit" });
  if (res.status !== 0) throw new Error(`fixture tar failed: ${res.status}`);
  return { file, bytes: fs.readFileSync(file) };
}

/// Assemble a fixture package root: manifest + assets.json + the real
/// payload bin/lib, so tests exercise the shipping shim.
export function fixturePackage({
  dir,
  version = "9.9.9",
  downloadBaseUrl = "https://example.invalid/releases/download",
  assets,
  releaseTag,
}) {
  fs.mkdirSync(path.join(dir, "bin"), { recursive: true });
  fs.mkdirSync(path.join(dir, "lib"), { recursive: true });
  fs.mkdirSync(path.join(dir, "scripts"), { recursive: true });
  fs.copyFileSync(path.join(NPM_DIR, "bin/archspec.mjs"), path.join(dir, "bin/archspec.mjs"));
  fs.copyFileSync(path.join(NPM_DIR, "lib/install.mjs"), path.join(dir, "lib/install.mjs"));
  fs.copyFileSync(path.join(NPM_DIR, "lib/matrix.mjs"), path.join(dir, "lib/matrix.mjs"));
  fs.copyFileSync(path.join(NPM_DIR, "scripts/postinstall.mjs"), path.join(dir, "scripts/postinstall.mjs"));
  fs.writeFileSync(
    path.join(dir, "package.json"),
    JSON.stringify(
      {
        name: "@fixture/archspec",
        version,
        bin: { archspec: "bin/archspec.mjs" },
        config: { downloadBaseUrl },
      },
      null,
      2,
    ),
  );
  fs.writeFileSync(
    path.join(dir, "assets.json"),
    JSON.stringify({ version, releaseTag: releaseTag ?? `v${version}`, assets }, null, 2),
  );
  return dir;
}

/// The full five-key asset map built from fake tarballs in dir.
export function fixtureAssets(dir, version, { omitKey = null } = {}) {
  const expected = matrix(version);
  const assets = {};
  for (const [key, name] of Object.entries(expected)) {
    if (key === omitKey) continue;
    const { bytes } = fakeTarball(dir, name, version);
    assets[key] = { name, sha256: sha256Hex(bytes) };
  }
  return assets;
}

export function runShim(pkgDir, args = [], env = {}) {
  return runBin(path.join(pkgDir, "bin/archspec.mjs"), args, env);
}

export function runBin(binPath, args = [], env = {}) {
  return spawnSync(process.execPath, [binPath, ...args], {
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
}
