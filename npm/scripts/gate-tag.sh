#!/bin/sh
# publish-gate leg: tag shape enforcement (republish-freeze law, ADR-051).
# Suffix tags (v0.5.2-gap class) are valid semver to npm but outside the
# release tag shape — refuse them at the workflow edge, before any build or
# upload can move an npm dist-tag.
#
# usage: sh gate-tag.sh <tag>

set -eu

tag="${1:?usage: gate-tag.sh <tag>}"
SHAPE='^v[0-9]+\.[0-9]+\.[0-9]+$'

if ! printf '%s' "${tag}" | grep -Eq "${SHAPE}"; then
  echo "publish-gate: tag '${tag}' violates the release tag shape ${SHAPE} — refusing (suffix tags must not reach the npm channel)" >&2
  exit 1
fi
echo "publish-gate: tag '${tag}' matches ${SHAPE}"
