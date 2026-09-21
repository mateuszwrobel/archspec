#!/bin/sh
# Install an Arch package the way a user would and verify what pacman records.
#
# Run this inside a stock Arch environment (the release job runs it in the
# official image, the local gate runs it in a container): it needs no network and
# no keyring, because a local file is not signature-checked.
#
# usage: install-check-arch-package.sh <package>
set -eu

pkg=${1:?usage: install-check-arch-package.sh <package>}

fail() { printf 'install-check-arch-package: %s\n' "$1" >&2; exit 1; }

[ -f "$pkg" ] || fail "no package at $pkg"
command -v pacman >/dev/null 2>&1 || fail 'pacman is required'

# pacman reports its own metadata complaints as "warning: <pkgname>: ..." and as
# "error:" lines. Warnings about databases are the state of the container, not of
# the package, so they are not failure conditions here.
install_log=$(pacman -U --noconfirm "$pkg" 2>&1) || {
	printf '%s\n' "$install_log"
	fail 'the install transaction failed'
}
printf '%s\n' "$install_log"
if printf '%s\n' "$install_log" | grep -Eq '^error:' ||
	printf '%s\n' "$install_log" | grep -Eq 'warning: archspec(:|$)'; then
	fail 'pacman misread the package metadata'
fi

# -Qkk compares every installed file against the .MTREE hashes: a missing file, a
# permission mismatch or altered content all make it exit non-zero.
pacman -Qkk archspec
archspec --help >/dev/null

printf 'install-check-arch-package: %s installs, verifies and runs\n' "$pkg"
