#!/bin/sh
# publish-gate leg: NuGet registry-existence check (republish-freeze law,
# decision-check record 2026-11-06 finding M7 — the npm twin is
# npm/scripts/gate-registry.sh and the two shapes stay twins). A published
# NuGet version is permanent (unlisting is not deleting), so an existing
# version refuses the run BEFORE any build or upload touches the release
# page — a named refusal, not a 409 error and not a silent success.
#
# The flatcontainer version index answers one exact version: 200 means
# published, 404 means the version is free. --curl names the fetch command
# (tests inject a stub).
#
# usage: sh gate-nuget-registry.sh --pkg <id> --version <ver> [--curl <cmd>]

set -eu

pkg=""
version=""
curl_bin=curl
while [ "$#" -gt 0 ]; do
  case "$1" in
    --pkg) pkg="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --curl) curl_bin="$2"; shift 2 ;;
    *) echo "gate-nuget-registry: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "${pkg}" ] && [ -n "${version}" ] || {
  echo "usage: gate-nuget-registry.sh --pkg <name> --version <ver> [--curl <cmd>]" >&2
  exit 2
}

# The flatcontainer stores the shipped nupkg lower-cased at
# <id>/<version>/<id>.<version>.nupkg — reading that blob path is the exact
# existence probe (HEAD-style: status only). A wrong path shape would make
# a PUBLISHED version read as free: the silent-pass class, so the shape is
# welded by the live-rehearsal arms and the stubbed table in the guard test.
pkg_lc="$(printf '%s' "${pkg}" | tr '[:upper:]' '[:lower:]')"
ver_lc="$(printf '%s' "${version}" | tr '[:upper:]' '[:lower:]')"
url="https://api.nuget.org/v3-flatcontainer/${pkg_lc}/${ver_lc}/${pkg_lc}.${ver_lc}.nupkg"

# Exit classes mirror gate-registry.sh: 200 = version exists (refuse),
# 404 = not found (allow), anything else = registry unreadable (refuse —
# fail closed, an unverifiable registry is not a green light to publish).
if ! code="$("${curl_bin}" -sS -o /dev/null -w '%{http_code}' "${url}" 2>/dev/null)"; then
  echo "publish-gate: cannot read ${pkg} ${version} from the NuGet registry (${url}) — refusing to run blind" >&2
  exit 1
fi
case "${code}" in
  200)
    echo "publish-gate: ${pkg} ${version} is already published on NuGet.org — the republish-freeze law refuses this run; recovery is a version bump, never a tag re-push" >&2
    exit 1
    ;;
  404)
    echo "publish-gate: ${pkg} ${version} is not published on NuGet.org — proceeding"
    ;;
  *)
    echo "publish-gate: NuGet registry answered ${code} for ${pkg} ${version} — refusing to run blind" >&2
    exit 1
    ;;
esac
