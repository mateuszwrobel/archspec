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

# git-env preamble (ADR-065): when the enclosing checkout's scrub preamble is
# reachable, source it so the canonical scrub set (git's own
# `--local-env-vars` list plus GIT_NAMESPACE) applies before any git call
# below. Absent it — the published standalone payload shape — the
# hand-written unset block above stays the scrub.
if [[ -f "$SCRIPT_DIR/../../../scripts/lib/git-env.sh" ]]; then
  # shellcheck source=/dev/null
  source "$SCRIPT_DIR/../../../scripts/lib/git-env.sh"
fi

# Stage timing (optional): when the repository-level stage helper is reachable
# from this checkout, every leg below runs under a named stage and prints a
# wall line; when STAGE_LOG is set, one JSONL row per stage rides with it.
# Absent the helper (published standalone payload) the commands run exactly
# as before — behavior identical, output unchanged.
STAGE_TIMER="${STAGE_TIMER:-$SCRIPT_DIR/../../../scripts/stage-timer.sh}"
if [ -f "$STAGE_TIMER" ]; then
  # shellcheck source=/dev/null
  . "$STAGE_TIMER"
else
  stage() { shift 3; "$@"; }
fi

echo "Running rust-arch-test-kit tests..."
# nextest (hardening lane A): per-test timeouts (see .config/nextest.toml).
# Doctest parity: doc tests stay on cargo test (nextest does not run them).
stage unit nextest -- cargo nextest run --no-tests=pass --config-file "$SCRIPT_DIR/.config/nextest.toml" --manifest-path "$SCRIPT_DIR/Cargo.toml" "$@"
stage unit doctest -- cargo test --doc --manifest-path "$SCRIPT_DIR/Cargo.toml"

echo "Running rust-arch-test-kit clippy gate..."
stage lint clippy -- cargo clippy --manifest-path "$SCRIPT_DIR/Cargo.toml" --all-targets -- -D warnings

# Self-dogfood gate: archspec checks its own architecture spec and the
# architecture artefacts it generates for this crate.
#
# Binary resolution: honor $ARCHSPEC if set, else the debug binary produced by
# the build below — $CARGO_TARGET_DIR/debug/archspec when the caller preset a
# target dir externally (containerized legs run with one), else the manifest
# -relative target/debug/archspec.
echo "Building archspec binary for the self-dogfood gate..."
stage build build-archspec -- cargo build --manifest-path "$SCRIPT_DIR/Cargo.toml" --bin archspec
ARCHSPEC_BIN="${ARCHSPEC:-${CARGO_TARGET_DIR:-$SCRIPT_DIR/target}/debug/archspec}"
if [[ ! -x "$ARCHSPEC_BIN" ]]; then
  echo "archspec binary not found/executable at: $ARCHSPEC_BIN" >&2
  echo "set ARCHSPEC=/path/to/archspec to override" >&2
  exit 1
fi

# 1. Verify this crate against its own architecture.spec.toml.
echo "Self verify: archspec verify --strict"
stage lint archspec-verify -- "$ARCHSPEC_BIN" verify "$SCRIPT_DIR" --strict

# 1b. Portfolio arm — root cure of the red-base origin class (2026-11-08).
#     A verify-semantics change lands green on this self-verify alone, then
#     surfaces later as a red base when a sibling spec meets the new edge
#     semantics at the next binary move. This arm adjudicates the whole
#     first-party portfolio with the SAME fresh binary: every
#     architecture.spec.toml found in the enclosing checkout — a structural
#     find, no named crate (the published payload names no other project).
#     Fail-closed: every found spec must pass. Trunk census: every first-party
#     spec passes verify --strict with the fresh binary and resolves from
#     sources alone (no build), so a FAIL here means the delta changed what
#     the specs adjudicate, not that a spec cannot resolve.
#
#     Semantics-path trigger, derived from the semantics-change commit
#     63471aa14 (GitHub issue #30): its payload src/ path (src/archspec/scan/)
#     plus the scan/ dir as a floor — generalized to every payload src/ path,
#     since verify semantics ship in src/. Delta sources: the committed delta
#     vs upstream (the pushed range; '@{u}', else the merge-base with
#     origin/main) plus the staged delta (the commit-hook shape). A delta that
#     is derivable and provably misses those paths pays zero added cost: one
#     SKIP line. A delta that cannot be derived (git unreadable — the
#     read-only container-mount class) RUNS the portfolio: unknown assumes
#     semantics (fail-safe). ARCHSPEC_PORTFOLIO=force runs the arm always
#     (probe seam).
PORTFOLIO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$PORTFOLIO_ROOT" ]; then
  PORTFOLIO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
fi
portfolio_base=""
if portfolio_up="$(git -C "$PORTFOLIO_ROOT" rev-parse --verify -q '@{u}' 2>/dev/null)"; then
  portfolio_base="$portfolio_up"
elif portfolio_up="$(git -C "$PORTFOLIO_ROOT" rev-parse --verify -q 'origin/main' 2>/dev/null)"; then
  portfolio_base="$(git -C "$PORTFOLIO_ROOT" merge-base HEAD "$portfolio_up" 2>/dev/null || true)"
fi
portfolio_delta=""
portfolio_known=0
if [ -n "$portfolio_base" ]; then
  if portfolio_committed="$(git -C "$PORTFOLIO_ROOT" diff --name-only \
      "${portfolio_base}...HEAD" 2>/dev/null)"; then
    portfolio_known=1
    portfolio_delta+="${portfolio_committed}"$'\n'
  fi
fi
if portfolio_staged="$(git -C "$PORTFOLIO_ROOT" diff --cached --name-only HEAD 2>/dev/null)"; then
  portfolio_known=1
  portfolio_delta+="${portfolio_staged}"$'\n'
fi
portfolio_run=0
portfolio_reason=""
if [[ "${ARCHSPEC_PORTFOLIO:-}" == "force" ]]; then
  portfolio_run=1
  portfolio_reason="forced via ARCHSPEC_PORTFOLIO=force"
elif [ "$portfolio_known" -ne 1 ]; then
  portfolio_run=1
  portfolio_reason="delta not derivable — assumes semantics (fail-safe)"
elif printf '%s\n' "$portfolio_delta" \
    | grep -q '^rust-arch-test-kit/public/rust-arch-test-kit/src/'; then
  portfolio_run=1
  portfolio_reason="delta touches payload src/ (verify semantics paths)"
fi
if [ "$portfolio_run" -eq 1 ]; then
  echo "Portfolio verify: every first-party architecture.spec.toml against $ARCHSPEC_BIN ($portfolio_reason)"
  portfolio_pass=0
  portfolio_fail=0
  while IFS= read -r portfolio_spec; do
    portfolio_dir="${portfolio_spec%/architecture.spec.toml}"
    portfolio_rel="${portfolio_dir#"$PORTFOLIO_ROOT"/}"
    portfolio_slug="${portfolio_rel//[^A-Za-z0-9._-]/-}"
    if stage lint "portfolio-verify-${portfolio_slug}" \
        -- "$ARCHSPEC_BIN" verify "$portfolio_dir" --strict; then
      portfolio_pass=$((portfolio_pass + 1))
      echo "PORTFOLIO-VERIFY PASS $portfolio_rel"
    else
      portfolio_fail=$((portfolio_fail + 1))
      echo "PORTFOLIO-VERIFY FAIL $portfolio_rel"
    fi
  done < <(find "$PORTFOLIO_ROOT" \
      \( -path '*/target' -o -path '*/.git' -o -path '*/node_modules' \) -prune -o \
      -name architecture.spec.toml -print | LC_ALL=C sort)
  portfolio_specs=$((portfolio_pass + portfolio_fail))
  echo "PORTFOLIO-VERIFY tally pass=$portfolio_pass fail=$portfolio_fail specs=$portfolio_specs"
  if [ "$portfolio_specs" -eq 0 ] || [ "$portfolio_fail" -gt 0 ]; then
    echo "PORTFOLIO-VERIFY FAILED — the portfolio disagrees with this binary's semantics (or no spec was found)" >&2
    exit 1
  fi
else
  echo "PORTFOLIO-VERIFY SKIP delta does not touch archspec semantics paths (zero added cost)"
fi

# 2. Freshness of the committed architecture artefacts that archspec generates.
#    Destinations come from archspec.toml [output]:
#      scan    -> docs/archspec/scan.json
#      diagram -> docs/archspec/diagram.mmd
#      inspect -> docs/archspec/inspect.mmd
#      report  -> docs/archspec/report.md
#    Hand-authored and test-regenerated artefacts are intentionally NOT checked:
#      * docs/**\/*.md prose (README, cli/errors/acceptance/..., docs/archspec/*.md)
#        are written by hand, not emitted by archspec.
#      * feature-matrix.{json,md} are regenerated into the target dir and
#        byte-compared against the committed copies by the `feature_matrix`
#        cargo test (not an `archspec` output command), so they are excluded
#        here.
#      * depgraph has no committed generated artefact and no [output] default.
echo "Self freshness: archspec artefact --check"
for cmd in scan diagram inspect report; do
  echo "  --check $cmd"
  stage artifact "artefact-check-$cmd" -- "$ARCHSPEC_BIN" "$cmd" "$SCRIPT_DIR" --check
done

# 3. Freshness of the generated third-party credits listing (CREDITS.md).
#    Cargo-driven like the artefact checks above: it resolves the committed
#    lockfile and must match the committed listing byte for byte. Fails
#    closed — a missing cargo/jq or a stale listing is a gate failure.
echo "Credits freshness: gen-credits --check"
stage artifact credits-check -- "$SCRIPT_DIR/scripts/gen-credits.sh" --check

# 4. npm-channel legs (ADR-051). The node suite exercises the package tree
#    (shim, install path, gates, staging). The containerized build image has
#    no node — there the leg prints a named skip instead of passing silently;
#    hook legs run on hosts with node and always run the suite. The
#    leak-parity script lives in the project's internal/ dir (outside the
#    payload) and is likewise skipped by name where absent.
if command -v node >/dev/null 2>&1; then
  echo "npm channel: node suite (node $(node --version))"
  stage other npm-suite -- bash "$SCRIPT_DIR/npm/test/run-npm-tests.sh"
else
  echo "npm channel: node absent (containerized leg) — node suite skipped by name"
fi
if [[ -f "$SCRIPT_DIR/../../internal/test_npm_leak_parity.sh" ]]; then
  echo "npm channel: leak parity against the publish guard's list"
  stage other npm-leak-parity -- bash "$SCRIPT_DIR/../../internal/test_npm_leak_parity.sh"
else
  echo "npm channel: internal/test_npm_leak_parity.sh absent — parity leg skipped by name"
fi

# 5. Bench tooling self-test (issue #33 corpus guard, decision-check M3):
#    prove the corpus guard refuses a stub tree BEFORE timing, nonzero —
#    the adversarial form, so a ratio guard can never pass on zero load.
#    Needs python3; a leg without python3 names its skip (npm-leg class).
if command -v python3 >/dev/null 2>&1; then
  echo "Bench tooling: corpus-guard self-test"
  stage other bench-guard-selftest -- python3 "$SCRIPT_DIR/scripts/bench/bench_sweep.py" self-test
else
  echo "Bench tooling: python3 absent — corpus-guard self-test skipped by name"
fi

echo "All rust-arch-test-kit tests, clippy, self verify and doc freshness checks passed."
