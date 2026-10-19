// Finding M1 (the sidecar format law): every SHIPPED sidecar passes
// `sha256sum -c`. The legacy windows format (hash + CRLF, no filename) does
// not — that is the defect the release.yml fix closes; this suite pins the
// target format and proves the old one fails, so the canonical verify
// sentence is true on every platform.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { mkTmp, sha256Hex } from "./helpers.mjs";

function sha256sumAvailable() {
  const probe = spawnSync("sha256sum", ["--version"], { encoding: "utf8" });
  return probe.status === 0;
}

test("the portable 'hash  filename' LF format passes sha256sum -c; the legacy windows format fails", (t) => {
  if (!sha256sumAvailable()) t.skip("sha256sum not on this host");
  const dir = mkTmp("npm-sidecar-");
  const asset = path.join(dir, "archspec-9.9.9-x86_64-pc-windows-msvc.zip");
  fs.writeFileSync(asset, "zip-bytes");
  const hash = sha256Hex(fs.readFileSync(asset));

  // Portable format (what every asset — linux, macos, and windows from this
  // change on — ships): hash, two spaces, filename, LF.
  const portable = path.join(dir, `${path.basename(asset)}.sha256`);
  fs.writeFileSync(portable, `${hash}  ${path.basename(asset)}\n`);
  const ok = spawnSync("sha256sum", ["-c", portable], { cwd: dir, encoding: "utf8" });
  assert.equal(ok.status, 0, `portable sidecar must verify: ${ok.stdout}${ok.stderr}`);

  // Legacy windows format (hash alone + CRLF, no filename): fails -c.
  const legacy = path.join(dir, "legacy-shape.sha256");
  const legacyAsset = path.join(dir, "legacy-asset.bin");
  fs.writeFileSync(legacyAsset, "legacy-bytes");
  fs.writeFileSync(legacy, `${sha256Hex(fs.readFileSync(legacyAsset))}\r\n`);
  const bad = spawnSync("sha256sum", ["-c", legacy], { cwd: dir, encoding: "utf8" });
  assert.notEqual(bad.status, 0, "the legacy no-filename CRLF shape must fail sha256sum -c — the fix's reason to exist");

  // And the linux "hash  filename" LF format, unchanged, stays green.
  const linuxAsset = path.join(dir, "archspec-9.9.9-x86_64-unknown-linux-musl.tar.gz");
  fs.writeFileSync(linuxAsset, "tar-bytes");
  fs.writeFileSync(path.join(dir, `${linuxAsset.split("/").pop()}.sha256`), `${sha256Hex(fs.readFileSync(linuxAsset))}  ${path.basename(linuxAsset)}\n`);
  const lin = spawnSync("sha256sum", ["-c", `${path.basename(linuxAsset)}.sha256`], { cwd: dir, encoding: "utf8" });
  assert.equal(lin.status, 0, `linux sidecar stays green: ${lin.stdout}${lin.stderr}`);
});
