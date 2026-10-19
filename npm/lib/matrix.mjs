// The exact five-key matrix law (ADR-051): every npm package version pins
// exactly these five release-asset names. No globs: the release carries more
// installables per target than one (linux-x64 ships tarball, deb and zst), so
// name matching must be exact and the map is built from this table alone.
// deb, pkg and zst names are excluded by construction — they appear nowhere.

export const MATRIX_KEYS = Object.freeze([
  "linux-x64",
  "linux-arm64",
  "darwin-x64",
  "darwin-arm64",
  "win32-x64",
]);

/// version -> { "<platform>-<arch>": "<release asset name>" }
export function matrix(version) {
  return {
    "linux-x64": `archspec-${version}-x86_64-unknown-linux-musl.tar.gz`,
    "linux-arm64": `archspec-${version}-aarch64-unknown-linux-gnu.tar.gz`,
    "darwin-x64": `archspec-${version}-x86_64-apple-darwin.tar.gz`,
    "darwin-arm64": `archspec-${version}-aarch64-apple-darwin.tar.gz`,
    "win32-x64": `archspec-${version}-x86_64-pc-windows-msvc.zip`,
  };
}

/// The binary member name inside the archive for one platform-arch key.
export function binaryName(key) {
  return key === "win32-x64" ? "archspec.exe" : "archspec";
}
