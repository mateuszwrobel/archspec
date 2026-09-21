#!/bin/sh
# Publish one release: notes plus built assets.
#
# Usage: sh release-assets.sh <tag> <asset-dir>
#
# Requires the gh CLI and GITHUB_REPOSITORY. Notes are the tag object's own
# message, read through the API: on a shallow checkout a tag ref can resolve to
# the commit it peels to, where git reports that commit's message as the tag's
# contents, so reading the annotation from the working tree can silently publish
# whatever the last commit happened to say.
#
# An existing release page is edited rather than created. The job runs again
# whenever a tag is re-pointed or a run is retried, and there gh release create
# answers 422 ("Release.tag_name already exists") — which, with the upload as the
# next command, would leave a page that CI can never repair.
set -eu

tag="${1:?usage: release-assets.sh <tag> <asset-dir>}"
dir="${2:?usage: release-assets.sh <tag> <asset-dir>}"
repo="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY must name owner/repo}"
title="archspec ${tag#v}"

notes="$(mktemp)"
trap 'rm -f "${notes}"' EXIT

tagobj="$(gh api "repos/${repo}/git/ref/tags/${tag}" --jq .object.sha || true)"
raw=""
if [ -n "${tagobj}" ]; then
  raw="$(gh api "repos/${repo}/git/tags/${tagobj}" --jq .message || true)"
fi
printf '%s\n' "${raw}" | awk 'p { print } /^[[:space:]]*$/ { p = 1 }' >"${notes}"
if [ ! -s "${notes}" ]; then
  printf '%s\n' "${title}" >"${notes}"
fi

if gh api "repos/${repo}/releases/tags/${tag}" >/dev/null 2>&1; then
  gh release edit "${tag}" --repo "${repo}" --title "${title}" --notes-file "${notes}"
else
  gh release create "${tag}" --repo "${repo}" --title "${title}" --notes-file "${notes}"
fi

gh release upload "${tag}" --repo "${repo}" "${dir}"/* --clobber
gh release view "${tag}" --repo "${repo}" --json assets -q '.assets[].name'
