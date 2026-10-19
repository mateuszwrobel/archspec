#!/usr/bin/env bash
# npm-channel test suite entry: node's own test runner over this directory.
# Hermetic — no network (reserved .invalid hosts and injected fetchers only),
# fixtures under the system temp dir. Called from run-unit-tests.sh (native
# hosts) and from the git hooks' npm legs; the containerized cargo legs have
# no node, where the runner prints a named skip line instead.
set -euo pipefail

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX \
      GIT_COMMON_DIR GIT_NAMESPACE GIT_OBJECT_DIRECTORY \
      GIT_ALTERNATE_OBJECT_DIRECTORIES 2>/dev/null || true

TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

command -v node >/dev/null 2>&1 || {
  echo "run-npm-tests: node not found — this script must run on a host with node" >&2
  exit 1
}

echo "Running npm-channel tests (node $(node --version))..."
node --test "$TEST_DIR"/*.test.mjs
