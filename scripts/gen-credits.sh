#!/usr/bin/env bash
# Generate CREDITS.md — the third-party license listing for the archspec
# binary — from the lockfile-resolved dependency graph.
#
# Modes:
#   gen-credits.sh          write CREDITS.md next to the crate manifest
#   gen-credits.sh --check  regenerate into a scratch dir, compare against the
#                           committed CREDITS.md, print the actionable diff and
#                           exit 1 when stale (freshness gate leg)
#
# The listing covers the normal (runtime) dependency closure of the root
# package: dev-dependencies and build-host-only crates are excluded, so the
# table states exactly the licenses of the code linked into the binary.
# Works from a clean checkout of this crate; needs cargo (with the committed
# lockfile) and jq.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/CREDITS.md"

command -v cargo >/dev/null 2>&1 || { echo "gen-credits: cargo not found" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "gen-credits: jq not found" >&2; exit 1; }

check=0
case "${1:-}" in
  --check) check=1 ;;
  "")      ;;
  *)       echo "gen-credits: unknown mode '$1' (expected '' or --check)" >&2; exit 2 ;;
esac

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# --locked: the listing must describe the committed lockfile, never a newer
# resolution. Any resolution drift fails closed here.
cargo metadata --format-version 1 --locked --manifest-path "$ROOT/Cargo.toml" >"$tmp/meta.json"

# Transitive closure from the root over normal edges only (in metadata v1 a
# dependency kind serialized as null is the normal kind), minus the root.
jq -r '
  (.resolve.root) as $root
  | (.packages | map({key: .id, value: .}) | from_entries) as $p
  | (.resolve.nodes
     | map({key: .id,
            value: [ (.deps // [])[]
                     | select((.dep_kinds // [{}]) | any(.kind == null))
                     | .pkg ] })
     | from_entries) as $adj
  | { seen: { ($root): true }, frontier: [ $root ] }
  | until(.frontier | length == 0;
      . as $s
      | ([ $s.frontier[] as $f | ($adj[$f] // [])[] ] | unique) as $cand
      | ($cand | map(select(. as $n | $s.seen[$n] | not))) as $new
      | { seen: ($s.seen | reduce $new[] as $n (.; .[$n] = true)), frontier: $new })
  | [ .seen | keys_unsorted[] | select(. != $root) ]
  | map($p[.] | [ .name, .version,
                  (.license // "UNKNOWN"),
                  (.repository // .homepage // "UNKNOWN") ])
  | sort_by(.[0], .[1])
  | .[] | @tsv
' "$tmp/meta.json" >"$tmp/rows.tsv"

{
  cat <<'EOF'
# Credits

The `archspec` tool (crate `rust-arch-test-kit`) is licensed under the
Apache License 2.0 — the full text lives in [LICENSE](LICENSE).

This listing credits every third-party crate whose code is linked into the
`archspec` binary, as resolved from the committed lockfile. It is
**generated** — do not edit it by hand. Regenerate it from a checkout of
this crate with:

    ./scripts/gen-credits.sh

The listing covers the normal (runtime) dependency closure of the root
package: crates used only for development, or only on the build host as
build-script tooling, are not part of the distributed binary and are not
listed here. Grammar crates embed their upstream grammar sources from the
linked repository; the license stated in a grammar crate's row is also the
license of the grammar it vendors.

## Dependencies

| Crate | Version | License | Repository |
|---|---|---|---|
EOF
  while IFS=$'\t' read -r name version license url; do
    printf '| `%s` | %s | %s | %s |\n' "$name" "$version" "$license" "$url"
  done <"$tmp/rows.tsv"
  printf '\n**%s crates.**\n' "$(wc -l <"$tmp/rows.tsv" | tr -d '[:space:]')"
} >"$tmp/CREDITS.md"

if [ "$check" = 1 ]; then
  if [ ! -f "$OUT" ]; then
    echo "gen-credits: CREDITS.md is missing — run ./scripts/gen-credits.sh" >&2
    exit 1
  fi
  if ! cmp -s "$OUT" "$tmp/CREDITS.md"; then
    echo "gen-credits: CREDITS.md is stale versus the lockfile-resolved dependency set —" >&2
    echo "gen-credits: regenerate with ./scripts/gen-credits.sh and commit the result" >&2
    diff -u "$OUT" "$tmp/CREDITS.md" || true
    exit 1
  fi
  echo "gen-credits: CREDITS.md up to date"
else
  cat "$tmp/CREDITS.md" >"$OUT"
  echo "gen-credits: wrote $OUT"
fi
