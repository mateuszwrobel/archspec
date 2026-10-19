// Publish-time staging builder (repo-identity injection law): assembles the
// publishable tree from the payload's npm/ directory, then — only when the
// identity flags are given — injects the scope, version, repository.url and
// download base URL into the manifest. No payload file carries these strings;
// they arrive from github.repository plus the scope repo variable at publish
// time. Without the identity flags the staging tree is the pre-injection
// pack (neutral sentinels), which the leak-parity test greps under the exact
// publish guard's pattern list.

// usage: node prepare-package.mjs --out <dir> [--scope <s> --version <ver>
//        --repository <owner/name>] [--assets <assets.json>]

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const npmDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

// The neutral sentinels committed in the template. After injection none may
// survive; before injection they are what makes the tree leak-clean AND
// pack-valid (both URLs are syntactically valid, both names npm accepts).
export const NEUTRAL = Object.freeze({
  name: "@scope/archspec",
  version: "0.0.0",
  repositoryUrl: "https://example.invalid/archspec.git",
  downloadBaseUrl: "https://example.invalid/releases/download",
});

const argv = process.argv.slice(2);
function flag(name) {
  const at = argv.indexOf(name);
  if (at === -1) return null;
  return argv[at + 1] ?? null;
}

const outDir = flag("--out");
const scope = flag("--scope");
const version = flag("--version");
const repository = flag("--repository");
const assetsFile = flag("--assets");
if (!outDir) {
  console.error("usage: prepare-package.mjs --out <dir> [--scope <s> --version <ver> --repository <owner/name>] [--assets <assets.json>]");
  process.exit(2);
}
const injecting = Boolean(scope || version || repository);
if (injecting && !(scope && version && repository)) {
  console.error("prepare-package: --scope, --version and --repository must be given together");
  process.exit(2);
}
if (injecting && !assetsFile) {
  console.error("prepare-package: an injected package must carry a generated assets.json (--assets)");
  process.exit(2);
}
if (!/^[a-z0-9]([a-z0-9._-]*[a-z0-9])?$/.test(scope ?? "placeholder")) {
  console.error(`prepare-package: scope ${JSON.stringify(scope)} is not a valid lowercase npm scope fragment`);
  process.exit(1);
}
if (!/^[0-9]+\.[0-9]+\.[0-9]+$/.test(version ?? "0.0.0")) {
  console.error(`prepare-package: version ${JSON.stringify(version)} is not a plain x.y.z release version`);
  process.exit(1);
}
if (!/^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/.test(repository ?? "octocat/archspec")) {
  console.error(`prepare-package: repository ${JSON.stringify(repository)} is not an owner/name pair`);
  process.exit(1);
}

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(path.join(outDir, "bin"), { recursive: true });
fs.mkdirSync(path.join(outDir, "lib"), { recursive: true });
fs.mkdirSync(path.join(outDir, "scripts"), { recursive: true });

for (const rel of ["package.json", "README.md", "bin/archspec.mjs", "lib/install.mjs", "lib/matrix.mjs", "scripts/postinstall.mjs"]) {
  fs.copyFileSync(path.join(npmDir, rel), path.join(outDir, rel));
}
// The crate's license file lives at the payload root; the package mirrors it.
fs.copyFileSync(path.join(npmDir, "..", "LICENSE"), path.join(outDir, "LICENSE"));
if (assetsFile) fs.copyFileSync(assetsFile, path.join(outDir, "assets.json"));

const manifestPath = path.join(outDir, "package.json");
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
if (injecting) {
  manifest.name = `@${scope}/archspec`;
  manifest.version = version;
  manifest.repository = { type: "git", url: `https://github.com/${repository}.git` };
  manifest.config = { ...manifest.config, downloadBaseUrl: `https://github.com/${repository}/releases/download` };
  const bytes = JSON.stringify(manifest, null, 2);
  for (const [field, sentinel] of [
    ["name", NEUTRAL.name],
    ["version", NEUTRAL.version],
    ["repository.url", NEUTRAL.repositoryUrl],
    ["config.downloadBaseUrl", NEUTRAL.downloadBaseUrl],
  ]) {
    if (bytes.includes(sentinel)) {
      console.error(`prepare-package: neutral sentinel for ${field} survived injection: ${sentinel}`);
      process.exit(1);
    }
  }
  fs.writeFileSync(manifestPath, `${bytes}\n`);
}

console.log(`prepare-package: staged ${outDir} (${injecting ? "injected" : "neutral"})`);
