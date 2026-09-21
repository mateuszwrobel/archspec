#!/usr/bin/env bash
# Run rust-arch-test-kit full test suite + clippy gate + self-dogfood gate.
#
# Safe to run from inside a git hook: we strip inherited GIT_* env so any
# `git` subprocess spawned during the run uses `current_dir`, not the
# enclosing repo's GIT_DIR.
set -euo pipefail

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX \
      GIT_COMMON_DIR GIT_NAMESPACE GIT_OBJECT_DIRECTORY \
      GIT_ALTERNATE_OBJECT_DIRECTORIES

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Running rust-arch-test-kit tests..."
cargo test --manifest-path "$SCRIPT_DIR/Cargo.toml" "$@"

echo "Running rust-arch-test-kit clippy gate..."
cargo clippy --manifest-path "$SCRIPT_DIR/Cargo.toml" --all-targets -- -D warnings

# Self-dogfood gate: archspec checks its own architecture spec and the
# architecture artefacts it generates for this crate.
#
# Binary resolution: honor $ARCHSPEC if set, else the debug binary produced by
# the build below (target/debug/archspec).
echo "Building archspec binary for the self-dogfood gate..."
cargo build --manifest-path "$SCRIPT_DIR/Cargo.toml" --bin archspec
ARCHSPEC_BIN="${ARCHSPEC:-$SCRIPT_DIR/target/debug/archspec}"
if [[ ! -x "$ARCHSPEC_BIN" ]]; then
  echo "archspec binary not found/executable at: $ARCHSPEC_BIN" >&2
  echo "set ARCHSPEC=/path/to/archspec to override" >&2
  exit 1
fi

# 1. Verify this crate against its own architecture.spec.toml.
echo "Self verify: archspec verify --strict"
"$ARCHSPEC_BIN" verify "$SCRIPT_DIR" --strict

# 2. Freshness of the committed architecture artefacts that archspec generates.
#    Destinations come from archspec.toml [output]:
#      scan    -> docs/archspec/scan.json
#      diagram -> docs/archspec/diagram.mmd
#      inspect -> docs/archspec/inspect.mmd
#      report  -> docs/archspec/report.md
#    Hand-authored and test-regenerated artefacts are intentionally NOT checked:
#      * docs/**\/*.md prose (README, cli/errors/acceptance/..., docs/archspec/*.md)
#        are written by hand, not emitted by archspec.
#      * feature-matrix.{json,md} are rewritten by the `feature_matrix` cargo test
#        on every run (not an `archspec` output command), so they are excluded here.
#      * depgraph has no committed generated artefact and no [output] default.
echo "Self freshness: archspec artefact --check"
for cmd in scan diagram inspect report; do
  echo "  --check $cmd"
  "$ARCHSPEC_BIN" "$cmd" "$SCRIPT_DIR" --check
done

echo "All rust-arch-test-kit tests, clippy, self verify and doc freshness checks passed."
