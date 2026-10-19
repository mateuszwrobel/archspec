#!/bin/sh
# publish-gate leg: registry-existence check (republish-freeze law, ADR-051).
# If the npm registry already carries <pkg>@<ver>, this run MUST fail loud
# BEFORE any build or upload touches the release assets — npm pins assume the
# tag's assets are frozen. --npm names the npm command (tests inject a stub).
#
# usage: sh gate-registry.sh --pkg <name> --version <ver> [--npm <cmd>]

set -eu

pkg=""
version=""
npm_bin=npm
while [ "$#" -gt 0 ]; do
  case "$1" in
    --pkg) pkg="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --npm) npm_bin="$2"; shift 2 ;;
    *) echo "gate-registry: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "${pkg}" ] && [ -n "${version}" ] || {
  echo "usage: gate-registry.sh --pkg <name> --version <ver> [--npm <cmd>]" >&2
  exit 2
}

# exit 0 = version exists (refuse), 1 = not found (allow), anything else =
# registry unreadable (refuse — fail closed, an unverifiable registry is
# not a green light to clobber).
"${npm_bin}" view "${pkg}@${version}" version >/dev/null 2>&1 && code=0 || code=$?
if [ "${code}" -eq 0 ]; then
  echo "publish-gate: ${pkg}@${version} already exists on the npm registry — the republish-freeze law refuses this run; recovery is a version bump, never a tag re-push" >&2
  exit 1
fi
if [ "${code}" -ne 1 ]; then
  echo "publish-gate: cannot read ${pkg}@${version} from the npm registry (npm view exited ${code}) — refusing to run blind" >&2
  exit 1
fi
echo "publish-gate: ${pkg}@${version} is not on the npm registry — proceeding"
