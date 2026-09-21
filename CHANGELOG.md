# Changelog

All notable changes to archspec are documented here. The format follows
Keep a Changelog; versions are release tags. Entries describe the tool
only.

## [Unreleased]

## [0.4.156] - 2026-09-21

### Added
- Release artifacts for every published version: Linux (tarball, deb, Arch
  package; x86_64 and aarch64), macOS (x86_64 and aarch64) and Windows
  (x86_64) binaries with checksums, attached to the release page by the
  repository's own release workflow.
- Arch Linux packages that install. `pacman -U` accepts them, `pacman -Qkk`
  verifies every installed file against the package manifest, and the x86_64
  asset is installed by a stock Arch container as part of the build. The
  aarch64 payload is the same dependency-free static binary as the tarballs,
  so its empty dependency list is true rather than optimistic.
- `package-arch.sh` builds a package from a finished binary,
  `check-arch-package.sh` reads a package back the way `pacman` does (archive
  members, `.PKGINFO` fields, a manifest that actually hashes the binary,
  payload architecture, staticness against the dependency list) and
  `install-check-arch-package.sh` proves the install in a stock Arch
  environment. Packaging is therefore inspectable without a release run.
- This changelog: every change to the published source now records an entry
  here, and release notes are generated from it plus the commit record.

### Changed
- Publication appends one release commit with aggregated notes to the
  repository history instead of rewriting it.
- Release packaging is produced by the scripts above rather than by a
  containerized `makepkg` run, so the built asset can be checked before it is
  uploaded instead of after.
- The tags between 0.4.149 and this version carried release-pipeline
  experiments (runner labels, container images, shell options, quoting) and
  produced no release page at all. Their entries are collapsed into this one,
  because none of them ever shipped a binary.

## [0.4.146]

### Added
- File-level import map for Go in `inspect`: package files become nodes and
  own-module imports become edges, so Go trees get the same discovery surface
  as rust and c# trees.
- `external_free` constraint: purity guards that report on presence instead
  of passing vacuously.
- Behavioral language-conformance matrix: each guard rule is exercised per
  language with clean counterparts and non-vacuity checks.
- Model-facing audit workflow in `help workflow`, including inspect-edge
  mining as the discovery step before writing rules.

### Changed
- Cross-unit `using` edges in the c# driver: reference-reachable namespace
  usings now produce module-tier edges instead of staying invisible.
- Production imports never target test-tier files in `inspect` edge maps, in
  any language; test files stay visible as nodes and import sources. The rule
  is pinned by a shared cross-language corpus scenario.
- Structural inspect modes render what the model carries; tree-mode refusal
  now names the missing module tier instead of the previous blanket message.

Earlier releases predate this changelog; their notes lived only in release
annotations.
