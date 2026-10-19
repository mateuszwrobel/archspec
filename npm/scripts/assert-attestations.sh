#!/bin/sh
# post-publish leg: provenance assert. Trusted-publishing config typos (a
# stage-only default, an expired registration) can leave a GREEN publish step
# with NO provenance — so the run asserts dist.attestations is non-empty
# after publish. --npm names the npm command (tests inject a stub). Retries
# a bounded window for registry propagation, then fails red.
#
# usage: sh assert-attestations.sh --pkg <name> --version <ver> [--npm <cmd>]
#        [--attempts <n>] [--sleep <seconds>]

set -eu

pkg=""
version=""
npm_bin=npm
attempts=20
wait_s=15
while [ "$#" -gt 0 ]; do
  case "$1" in
    --pkg) pkg="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --npm) npm_bin="$2"; shift 2 ;;
    --attempts) attempts="$2"; shift 2 ;;
    --sleep) wait_s="$2"; shift 2 ;;
    *) echo "assert-attestations: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "${pkg}" ] && [ -n "${version}" ] || {
  echo "usage: assert-attestations.sh --pkg <name> --version <ver> [--npm <cmd>] [--attempts <n>] [--sleep <s>]" >&2
  exit 2
}

i=1
while [ "${i}" -le "${attempts}" ]; do
  out="$("${npm_bin}" view "${pkg}@${version}" dist.attestations 2>/dev/null || true)"
  if [ -n "${out}" ]; then
    echo "assert-attestations: ${pkg}@${version} carries provenance attestations"
    exit 0
  fi
  i=$(( i + 1 ))
  [ "${i}" -le "${attempts}" ] && sleep "${wait_s}"
done

echo "assert-attestations: ${pkg}@${version} shows EMPTY dist.attestations after ${attempts} attempts — the trusted-publisher config is missing, expired, or stage-only; the run is red by design" >&2
exit 1
