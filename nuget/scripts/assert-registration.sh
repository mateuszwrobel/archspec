#!/bin/sh
# publish-nuget leg: bounded registration poll (decision-check record
# 2026-11-06 finding M15 — NuGet validation and indexing are asynchronous;
# a push that exits 0 is NOT evidence of a published package, and validation
# failures never touch the push exit code). Green = the version appears at
# its registration leaf. Red = one loud TIMEOUT line at the deadline,
# pointing at the package page — never a "pushed, awaiting validation"
# standing state (long-drill every-wait-gets-a-deadline law).
#
# usage: sh assert-registration.sh --pkg <id> --version <ver> \
#          [--timeout-secs n] [--space-secs n] [--curl <cmd>]

set -eu

pkg=""
version=""
timeout_secs=1500
space_secs=30
curl_bin=curl
while [ "$#" -gt 0 ]; do
  case "$1" in
    --pkg) pkg="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --timeout-secs) timeout_secs="$2"; shift 2 ;;
    --space-secs) space_secs="$2"; shift 2 ;;
    --curl) curl_bin="$2"; shift 2 ;;
    *) echo "assert-registration: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "${pkg}" ] && [ -n "${version}" ] || {
  echo "usage: assert-registration.sh --pkg <name> --version <ver> [--timeout-secs n] [--space-secs n] [--curl <cmd>]" >&2
  exit 2
}

pkg_lc="$(printf '%s' "${pkg}" | tr '[:upper:]' '[:lower:]')"
ver_lc="$(printf '%s' "${version}" | tr '[:upper:]' '[:lower:]')"
# The registration leaf is the indexed-truth document: it answers 200 only
# once validation and indexing have placed the version (finding M15). The
# endpoint serves gzip; only the status code is read.
url="https://api.nuget.org/v3/registration5-gz-semver2/${pkg_lc}/${ver_lc}.json"
page="https://www.nuget.org/packages/${pkg}/${version}"

deadline="$(($(date +%s) + timeout_secs))"
last_code=000
while :; do
  last_code="$("${curl_bin}" -sS -o /dev/null -w '%{http_code}' "${url}" 2>/dev/null || echo 000)"
  if [ "${last_code}" = "200" ]; then
    echo "publish-nuget: ${pkg} ${version} is registered on NuGet.org — ${page}"
    exit 0
  fi
  now="$(date +%s)"
  if [ "${now}" -ge "${deadline}" ]; then
    echo "publish-nuget: TIMEOUT — ${pkg} ${version} still not registered after ${timeout_secs}s (last registry answer: ${last_code}) — NuGet validation or indexing may have failed; inspect ${page}" >&2
    exit 1
  fi
  sleep "${space_secs}"
done
