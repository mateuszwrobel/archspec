// postinstall: fetch the host binary at install time when scripts run.
// Integrity failures are hard failures — a hash mismatch never defers.
// Availability failures (offline, blocked network) exit 0 with the
// loud-failure contract printed: the first run of the bin repeats the
// contract and fails there, so nothing is ever silently missing.

import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { obtain, contractLines, platformKey, ObtainError } from "../lib/install.mjs";

const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

try {
  const { binaryPath, manifest } = await obtain({ pkgRoot });
  console.log(`archspec@${manifest.version}: host binary ready at ${binaryPath}`);
} catch (err) {
  if (!(err instanceof ObtainError)) throw err;
  const detail = err.lines.join("\n");
  if (err.kind === "mismatch") {
    console.error(detail);
    console.error(
      contractLines({
        version: err.version ?? "unknown",
        key: platformKey(),
        url: err.url || null,
      }).join("\n"),
    );
    process.exit(1);
  }
  if (err.kind === "missing-key") {
    console.error(detail);
    process.exit(0); // first run prints the same line and fails there
  }
  console.warn(
    contractLines({
      version: err.version ?? "unknown",
      key: platformKey(),
      url: err.url || null,
    }).join("\n"),
  );
  console.warn(detail);
  console.warn("install continuing without the binary; the first run will fetch it.");
}
