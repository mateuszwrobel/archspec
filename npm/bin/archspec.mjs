#!/usr/bin/env node
// archspec bin shim: resolve, verify, cache and run the release binary for
// this host. Arguments are forwarded verbatim; the shim defines no flags of
// its own. Every failure is the loud-failure contract — platform, arch,
// version, URL and an --ignore-scripts hint — never a cryptic ENOENT.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { obtain, runBinary, contractLines, platformKey, ObtainError } from "../lib/install.mjs";

const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function manifestVersion() {
  try {
    return JSON.parse(fs.readFileSync(path.join(pkgRoot, "package.json"), "utf8")).version;
  } catch {
    return "unknown";
  }
}

function fail(err) {
  if (err.kind === "missing-key") {
    console.error(err.lines.join("\n"));
    process.exit(1);
  }
  console.error(
    contractLines({
      version: manifestVersion(),
      key: platformKey(),
      url: err.url || null,
    }).join("\n"),
  );
  console.error(err.lines.join("\n"));
  process.exit(1);
}

const version = manifestVersion();
try {
  const { binaryPath } = await obtain({ pkgRoot });
  try {
    process.exit(runBinary(binaryPath, process.argv.slice(2)));
  } catch (err) {
    if (err instanceof ObtainError) fail(err);
    throw err;
  }
} catch (err) {
  if (err instanceof ObtainError) fail(err);
  else if (version === "unknown") throw err;
  else {
    console.error(
      contractLines({ version, key: platformKey(), url: null }).join("\n"),
    );
    console.error(String(err && err.stack ? err.stack : err));
    process.exit(1);
  }
}
