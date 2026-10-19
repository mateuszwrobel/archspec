// Shared install path for the bin shim and the postinstall script: resolve
// the host platform-arch key against the pinned assets.json, download the
// matching GitHub release asset, verify the pinned sha256 from the DOWNLOADED
// BYTES (never a sidecar), cache atomically (temp dir + rename under a lock),
// and return the cached binary path. A cache entry also carries a binary-digest
// sidecar (written under the same lock right after the archive pin checked),
// so a cache hit verifies its bytes without re-downloading the archive.
// Every failure that means "cannot be obtained" carries the loud-failure
// contract: platform, arch, version, the exact download URL and an
// --ignore-scripts hint — never a cryptic ENOENT.

import { createHash } from "node:crypto";
import { randomUUID } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import https from "node:https";
import { spawnSync } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";

const LOCK_STALE_MS = 15 * 60 * 1000;
const LOCK_DEADLINE_MS = 120 * 1000;
const MAX_REDIRECTS = 5;

export class ObtainError extends Error {
  constructor(kind, lines) {
    super(lines.join("\n"));
    this.kind = kind; // "missing-key" | "mismatch" | "unavailable" | "lock" | "extract"
    this.lines = lines;
  }
}

export function platformKey(platform = process.platform, arch = process.arch) {
  return `${platform}-${arch}`;
}

export function assetUrl(baseUrl, tag, name) {
  return `${baseUrl}/${tag}/${name}`;
}

export function sha256Hex(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

/// Cache root: under the user's home, named by version and platform-arch at
/// the file level. Tests and hermetic runs override HOME (or USERPROFILE on
/// windows), which os.homedir() honors — no product-visible env var.
export function cacheFile(manifestVersion, key, env = process.env) {
  const home = env.HOME || os.homedir();
  const exe = key === "win32-x64" ? ".exe" : "";
  return path.join(
    home,
    ".cache",
    "archspec-npm",
    `archspec-${manifestVersion}-${key}${exe}`,
  );
}

/// Binary-digest sidecar of a cache entry. Written last under the lock, so a
/// present sidecar means the binary next to it was extracted from an archive
/// whose pinned hash checked out.
function digestFile(finalPath) {
  return `${finalPath}.sha256`;
}

/// "ok" — cache hit, bytes match the recorded digest (return the entry);
/// "absent" — no complete entry (provision it); throws "mismatch" — the
/// binary disagrees with its recorded digest (integrity failure, never a
/// silent redo, never a run).
function cacheState(finalPath, manifestVersion, key) {
  if (!fs.existsSync(finalPath) || !fs.existsSync(digestFile(finalPath))) {
    return "absent";
  }
  const actual = sha256Hex(fs.readFileSync(finalPath));
  const recorded = fs.readFileSync(digestFile(finalPath), "utf8").trim();
  if (actual !== recorded) {
    throw new ObtainError("mismatch", [
      `archspec@${manifestVersion}: cached binary for ${key} does not match its recorded hash`,
      `  file: ${finalPath}`,
      `  recorded sha256: ${recorded}`,
      `  actual sha256:   ${actual}`,
    ]);
  }
  return "ok";
}

/// Default fetcher: plain release-download URL (a CDN redirect, no API and
/// therefore no rate wall), never api.github.com.
export function httpsFetch(url) {
  return new Promise((resolve, reject) => {
    const get = (target, redirectsLeft) => {
      const req = https.get(target, (res) => {
        const location = res.headers.location;
        if (res.statusCode >= 300 && res.statusCode < 400 && location && redirectsLeft > 0) {
          res.resume();
          get(new URL(location, target), redirectsLeft - 1);
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new ObtainError("unavailable", [`download failed: HTTP ${res.statusCode} for ${target}`]));
          return;
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", (err) => reject(new ObtainError("unavailable", [`download failed: ${err.message} for ${target}`])));
      });
      req.on("error", (err) => reject(new ObtainError("unavailable", [`download failed: ${err.message} for ${target}`])));
    };
    get(new URL(url), MAX_REDIRECTS);
  });
}

function readManifest(pkgRoot) {
  const manifestPath = path.join(pkgRoot, "package.json");
  const assetsPath = path.join(pkgRoot, "assets.json");
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  let assets;
  try {
    assets = JSON.parse(fs.readFileSync(assetsPath, "utf8"));
  } catch {
    throw new ObtainError("unavailable", [
      `archspec@${manifest.version}: the package ships no assets.json — the publish step did not generate it`,
    ]);
  }
  return { manifest, assets };
}

/// Loud-failure contract: names platform, arch, version, URL and the
/// --ignore-scripts hint. `null` parts are omitted on purpose (missing-key
/// has no URL; that path uses its own one-line message instead).
export function contractLines({ version, key, url }) {
  const lines = [`archspec@${version}: could not obtain the binary for ${key}.`];
  if (url) lines.push(`  download url: ${url}`);
  lines.push(
    `  if this install skipped lifecycle scripts (npm --ignore-scripts or a package-manager script policy),`,
    `  the binary is fetched on first run — rerun once with network access, or fetch the url above by hand.`,
  );
  return lines;
}

function missingKeyLines(version, key) {
  return [`archspec@${version}: no release asset for ${key}`];
}

async function acquireLock(lockDir) {
  const deadline = Date.now() + LOCK_DEADLINE_MS;
  for (;;) {
    try {
      fs.mkdirSync(lockDir, { recursive: false });
      return true;
    } catch (err) {
      if (err.code !== "EEXIST") throw err;
      let age = 0;
      try {
        age = Date.now() - fs.statSync(lockDir).mtimeMs;
      } catch {
        continue; // vanished between EEXIST and stat — retry the mkdir
      }
      if (age > LOCK_STALE_MS) {
        try {
          fs.rmSync(lockDir, { recursive: true, force: true });
        } catch {}
        continue;
      }
      if (Date.now() >= deadline) {
        throw new ObtainError("lock", [`timed out waiting for the cache lock at ${lockDir}`]);
      }
      // Awaited sleep, never Atomics.wait: a blocked thread would starve the
      // holder's own download and deadlock concurrent in-process first runs.
      await sleep(100);
    }
  }
}

function extractArchive(archivePath, archiveName, stageDir, key) {
  // tarballs are gzipped tar; the windows asset is a zip, which bsdtar —
  // present on every supported windows host since Windows 10 — extracts.
  const args =
    key === "win32-x64"
      ? ["-xf", archivePath, "-C", stageDir]
      : ["-xzf", archivePath, "-C", stageDir];
  const res = spawnSync("tar", args, { stdio: "inherit" });
  if (res.error) {
    throw new ObtainError("extract", [`extraction failed to start: ${res.error.message} (is tar available?)`]);
  }
  if (res.status !== 0) {
    throw new ObtainError("extract", [`extraction failed: tar exited ${res.status} for ${archiveName}`]);
  }
}

/// Obtain the binary for the host. Returns { binaryPath, url, manifest }.
/// `fetch` and `now` are injectable seams for the adversarial tests; the
/// product path uses httpsFetch and Date.now.
export async function obtain({
  pkgRoot,
  env = process.env,
  fetch = httpsFetch,
  platform = process.platform,
  arch = process.arch,
} = {}) {
  const { manifest, assets } = readManifest(pkgRoot);
  const key = platformKey(platform, arch);
  const entry = (assets.assets || {})[key];
  if (!entry || !entry.name || !entry.sha256) {
    throw new ObtainError("missing-key", missingKeyLines(manifest.version, key));
  }
  const baseUrl = (manifest.config && manifest.config.downloadBaseUrl) || "";
  const tag = assets.releaseTag || `v${manifest.version}`;
  const url = assetUrl(baseUrl, tag, entry.name);
  const finalPath = cacheFile(manifest.version, key, env);

  try {
    return await obtainInner({ manifest, entry, key, url, finalPath, env, fetch });
  } catch (err) {
    if (err instanceof ObtainError) {
      if (!err.url) err.url = url;
      if (!err.version) err.version = manifest.version;
    }
    throw err;
  }
}

async function obtainInner({ manifest, entry, key, url, finalPath, env, fetch }) {
  if (cacheState(finalPath, manifest.version, key) === "ok") {
    return { binaryPath: finalPath, url, manifest };
  }

  const cacheDir = path.dirname(finalPath);
  fs.mkdirSync(cacheDir, { recursive: true });
  const lockDir = path.join(cacheDir, ".lock");
  const held = await acquireLock(lockDir);
  try {
    if (cacheState(finalPath, manifest.version, key) === "ok") {
      // Another first-run finished while we waited for the lock. A corrupted
      // binary against its recorded digest already threw — never a silent redo.
      return { binaryPath: finalPath, url, manifest };
    }

    let bytes;
    try {
      bytes = await fetch(url);
    } catch (err) {
      if (err instanceof ObtainError) throw err;
      throw new ObtainError("unavailable", [`download failed: ${err.message} for ${url}`]);
    }
    const actual = sha256Hex(bytes);
    if (actual !== entry.sha256) {
      // Hard fail by design: no warn-and-continue path exists, nothing is
      // written to the cache, and the mismatch is named.
      throw new ObtainError("mismatch", [
        `archspec@${manifest.version}: downloaded asset for ${key} does not match its pinned hash`,
        `  url: ${url}`,
        `  expected sha256: ${entry.sha256}`,
        `  actual sha256:   ${actual}`,
      ]);
    }

    const archivePath = path.join(cacheDir, `.dl-${process.pid}-${randomUUID()}-${entry.name}`);
    const stageDir = path.join(cacheDir, `.stage-${process.pid}-${randomUUID()}`);
    const sidecarTmp = path.join(cacheDir, `.sidecar-${process.pid}-${randomUUID()}`);
    try {
      fs.writeFileSync(archivePath, bytes);
      fs.mkdirSync(stageDir, { recursive: true });
      extractArchive(archivePath, entry.name, stageDir, key);
      const stagedBinary = path.join(stageDir, key === "win32-x64" ? "archspec.exe" : "archspec");
      if (!fs.existsSync(stagedBinary)) {
        throw new ObtainError("extract", [
          `extraction produced no ${key === "win32-x64" ? "archspec.exe" : "archspec"} member in ${entry.name}`,
        ]);
      }
      if (key !== "win32-x64") fs.chmodSync(stagedBinary, 0o755);
      // Same-directory rename keeps the publish atomic (no EXDEV, no torn
      // file); concurrent first runs land as complete binaries either way.
      fs.renameSync(stagedBinary, finalPath);
      // The digest sidecar is written last: an entry without one is
      // incomplete and gets re-provisioned, never trusted.
      fs.writeFileSync(sidecarTmp, `${sha256Hex(fs.readFileSync(finalPath))}\n`);
      fs.renameSync(sidecarTmp, digestFile(finalPath));
    } finally {
      fs.rmSync(archivePath, { force: true });
      fs.rmSync(stageDir, { recursive: true, force: true });
      fs.rmSync(sidecarTmp, { force: true });
    }
    return { binaryPath: finalPath, url, manifest };
  } finally {
    if (held) fs.rmSync(lockDir, { recursive: true, force: true });
  }
}

/// Launch the binary with stdio inherit; the child's exit code is the shim's
/// exit code. injectable spawn keeps the windows-path contract testable on
/// any host.
export function runBinary(binaryPath, args, { spawn = spawnSync } = {}) {
  let res;
  try {
    res = spawn(binaryPath, args, { stdio: "inherit", windowsHide: true });
  } catch (err) {
    throw new ObtainError("unavailable", [`could not start ${binaryPath}: ${err.message}`]);
  }
  if (res.error) {
    if (res.error.code === "ENOENT") {
      // The ENOENT class is design-forbidden as a user-visible failure.
      throw new ObtainError("unavailable", [`the cached binary is not executable: ${binaryPath}`]);
    }
    throw new ObtainError("unavailable", [`could not start ${binaryPath}: ${res.error.message}`]);
  }
  return res.status === null ? 1 : res.status;
}
