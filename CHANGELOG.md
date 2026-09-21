# Changelog

All notable changes to archspec are documented here. The format follows
Keep a Changelog; versions are release tags. Entries describe the tool
only.

## [Unreleased]

## [0.4.159] - 2026-09-21

### Fixed
- Publishing a tag that already has a release page edits that page instead of
  failing. The release job runs again whenever a tag is re-pointed or a run is
  retried, and there creating the page answers 422 ("Release.tag_name already
  exists"); because the asset upload follows it, a failed create left a release
  that no later run could repair. The step is now a script in this tree
  (`release-assets.sh`) so the same logic runs in CI and in local checks.

## [0.4.158] - 2026-09-21

### Fixed
- The release page now always carries the version's own notes. The release job
  read them from the tag through `git`, and on a shallow checkout a tag ref can
  resolve to the commit it peels to, where `git` reports that commit's message as
  the tag contents — so a release could publish whatever the last commit happened
  to say. The job reads the tag object's message through the API instead and falls
  back to a one-line title rather than to a commit message.

## [0.4.157] - 2026-09-21

### Fixed
- The previous entry described the aarch64 Arch package as carrying the same
  payload as the aarch64 tarball and deb. It does not: the tarball and deb are
  the glibc build (`aarch64-unknown-linux-gnu`), the Arch package is the static
  musl build (`aarch64-unknown-linux-musl`), which is exactly why the package
  may declare no dependencies. The entry now says which build each asset
  carries.

## [0.4.156] - 2026-09-21

### Added
- Release artifacts for every published version: Linux (tarball, deb, Arch
  package; x86_64 and aarch64), macOS (x86_64 and aarch64) and Windows
  (x86_64) binaries with checksums, attached to the release page by the
  repository's own release workflow.
- Arch Linux packages that install. `pacman -U` accepts them, `pacman -Qkk`
  verifies every installed file against the package manifest, and the x86_64
  asset is installed by a stock Arch container as part of the build. Both
  packages carry the static musl build rather than the glibc one, so their
  empty dependency list is true rather than optimistic.
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
- The published tags before this version (every one up to and including the
  release-pipeline experiments) produced no release page at all: they carried
  runner labels, container images, shell options and quoting fixes rather than
  binaries. Their entries are collapsed into this one, so the gap in version
  numbers is documented instead of hidden.

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
