#!/bin/sh
# Verify an Arch package's contents against what the release promises, without
# installing it. Complements package-arch.sh: that one produces the package, this
# one reads it back as pacman would and refuses a package that would install
# wrongly or is not the build it claims to carry.
#
# usage: check-arch-package.sh <package> <arch> <binary>
#
# The version and release numbers are taken from the file name, which must be
# archspec-<version>-<pkgrel>-<arch>.pkg.tar.zst, so a package built from another
# tag cannot pass by being renamed here.
set -eu

pkg=${1:?usage: check-arch-package.sh <package> <arch> <binary>}
arch=${2:?usage: check-arch-package.sh <package> <arch> <binary>}
bin=${3:?usage: check-arch-package.sh <package> <arch> <binary>}

fail() { printf 'check-arch-package: %s\n' "$1" >&2; exit 1; }

[ -f "$pkg" ] || fail "no package at $pkg"
[ -f "$bin" ] || fail "no binary at $bin"
command -v bsdtar >/dev/null 2>&1 || fail 'bsdtar (libarchive) is required'
case $arch in
	x86_64) want_elf='x86-64' ;;
	aarch64) want_elf='ARM aarch64' ;;
	*) fail "unsupported arch $arch" ;;
esac

name=$(basename "$pkg" .pkg.tar.zst)
nvr=$(printf '%s' "$name" | sed -E "s/^archspec-([^-]+)-([0-9]+)-${arch}\$/\1-\2/")
[ "$nvr" != "$name" ] || fail "package name $name does not match archspec-<version>-<pkgrel>-${arch}.pkg.tar.zst"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT HUP INT TERM

bsdtar -tf "$pkg" >"$stage/members"
for want in .PKGINFO .MTREE usr/bin/archspec; do
	grep -qx -- "$want" "$stage/members" || fail "archive is missing $want"
done

bsdtar -xOf "$pkg" .PKGINFO >"$stage/pkginfo"
grep -qx 'pkgname = archspec' "$stage/pkginfo" || fail 'wrong pkgname'
grep -qx "pkgver = $nvr" "$stage/pkginfo" || fail "pkgver is not $nvr"
grep -qx "arch = $arch" "$stage/pkginfo" || fail "package is not for $arch"
grep -q '^pkgdesc = .\+' "$stage/pkginfo" || fail 'no description'
grep -q '^url = https\?://' "$stage/pkginfo" || fail 'no homepage url'
grep -q '^license = .\+' "$stage/pkginfo" || fail 'no license'
grep -q '^size = [1-9][0-9]*$' "$stage/pkginfo" || fail 'installed size is not a positive number'
# A static payload declares nothing; a dependency here would mean the payload
# stopped being self-contained.
! grep -q '^depend = ' "$stage/pkginfo" || fail 'package declares dependencies but ships a static binary'

# The manifest is what `pacman -Qk` checks installed files against, so it must
# hash the binary rather than merely list it.
bsdtar -xOf "$pkg" .MTREE >"$stage/mtree.gz"
gzip -dc "$stage/mtree.gz" >"$stage/mtree" || fail 'manifest is not gzip-compressed'
grep -q '\./usr/bin/archspec .*sha256digest=' "$stage/mtree" ||
	fail 'manifest does not hash the binary'

bsdtar -xOf "$pkg" usr/bin/archspec >"$stage/extracted"
cmp -s "$stage/extracted" "$bin" || fail 'packaged binary differs from the build'
file "$stage/extracted" | grep -q "$want_elf" || fail "payload is not an $arch binary"
! file "$stage/extracted" | grep -q 'dynamically linked' ||
	fail 'payload is dynamically linked while the package declares no dependencies'

printf 'check-arch-package: %s ok (pkgver %s)\n' "$pkg" "$nvr"

