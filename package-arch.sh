#!/bin/sh
# Build an Arch package (pkg.tar.zst) from a prebuilt binary, without makepkg.
#
# A .pkg.tar.zst is a zstd tarball holding the install tree plus two metadata
# members that pacman reads: .PKGINFO (package identity, install size, licensed
# and targeted arches) and .MTREE (an mtree manifest that `pacman -Qk` verifies
# installed files against). Both are produced here with bsdtar/libarchive, the
# same toolchain makepkg drives, so integrity checks see exactly what a
# makepkg-built package would show. Ownership is forced to root inside both the
# payload and the manifest, which is what an installed system expects.
#
# usage: package-arch.sh <binary> <arch> <version> [pkgrel]
# env:   PKG_URL    package homepage recorded in .PKGINFO
#        PKG_DESC   one-line description recorded in .PKGINFO
#        PKG_REL    release number, default 1
#        PKG_ZSTD   zstd binary to use (default: zstd on PATH)
#        PKG_GZIP   gzip binary to use for the manifest (default: pigz, else gzip)
#
# Portable shell only: a runner's /bin/sh may be dash, so no bashisms.
#
# The binary must be self-contained: this records no runtime dependencies, so a
# dynamically linked binary would install with unsatisfied deps.
set -eu

bin=${1:?usage: package-arch.sh <binary> <arch> <version> [pkgrel]}
arch=${2:?usage: package-arch.sh <binary> <arch> <version> [pkgrel]}
version=${3:?usage: package-arch.sh <binary> <arch> <version> [pkgrel]}
pkgrel=${4:-${PKG_REL:-1}}

pkgname=archspec
pkgdesc=${PKG_DESC:-Architecture conformance toolkit}
url=${PKG_URL:-https://github.com/archspec/archspec}
pkgver=$(printf '%s' "$version" | sed -E 's/^v//')

case $pkgver in
	'' | *[!0-9.]*) printf 'package-arch: invalid version %s\n' "$version" >&2; exit 1 ;;
esac
case $pkgrel in
	'' | *[!0-9]*) printf 'package-arch: invalid pkgrel %s\n' "$pkgrel" >&2; exit 1 ;;
esac
case $arch in
	x86_64 | aarch64) ;;
	*) printf 'package-arch: unsupported arch %s\n' "$arch" >&2; exit 1 ;;
esac
[ -f "$bin" ] || { printf 'package-arch: no binary at %s\n' "$bin" >&2; exit 1; }
command -v bsdtar >/dev/null 2>&1 || { printf 'package-arch: bsdtar (libarchive) is required\n' >&2; exit 1; }
zstd=${PKG_ZSTD:-$(command -v zstd || true)}
[ -n "$zstd" ] || { printf 'package-arch: zstd is required\n' >&2; exit 1; }
gzip=${PKG_GZIP:-$(command -v pigz || command -v gzip || true)}
[ -n "$gzip" ] || { printf 'package-arch: gzip (or pigz) is required\n' >&2; exit 1; }

out=${pkgname}-${pkgver}-${pkgrel}-${arch}.pkg.tar.zst
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT HUP INT TERM

install -d -m 755 "$stage/pkg/usr/bin"
install -m 755 "$bin" "$stage/pkg/usr/bin/$pkgname"
chmod 755 "$stage/pkg/usr" "$stage/pkg/usr/bin"

# mtime shared by every member and by the manifest, so the package is
# reproducible from the same inputs.
builddate=$(date -u -r "$stage/pkg/usr/bin/$pkgname" +%s 2>/dev/null || date -u +%s)
size=$(du -sb "$stage/pkg" | cut -f1)
outabs=$PWD/$out

cat >"$stage/pkg/.PKGINFO" <<EOF
pkgname = $pkgname
pkgbase = $pkgname
pkgver = $pkgver-$pkgrel
pkgdesc = $pkgdesc
url = $url
builddate = $builddate
packager = $pkgname release automation <$pkgname@invalid>
size = $size
license = Apache-2.0
arch = $arch
EOF

# Ownership normalised before the manifest is written, so its entries match the
# root-owned files pacman will install. The manifest describes every member of
# the archive except itself, which is what makepkg records too. Compression runs
# through the standalone tools rather than libarchive's filters: those tools are
# always present, support for the zstd filter inside bsdtar is not guaranteed.
# Stages write to files and redirect into the compressor instead of piping,
# because /bin/sh on a runner may be dash, which has neither pipelines' exit
# status nor pipefail, so a dead compressor could go unnoticed.
bsdtar -C "$stage/pkg" --format=mtree \
	--options='!all,use-set,type,uid,gid,mode,time,size,md5,sha256,link' \
	--uid 0 --gid 0 --uname root --gname root \
	--exclude .MTREE -cf "$stage/pkg.mtree" .
"$gzip" -9n -c "$stage/pkg.mtree" >"$stage/pkg/.MTREE"

# Metadata members first, install tree after, root-owned throughout: the same
# member order and ownership a makepkg build produces.
bsdtar -C "$stage/pkg" --format=pax \
	--uid 0 --gid 0 --uname root --gname root \
	-cf "$stage/pkg.tar" .PKGINFO .MTREE usr
"$zstd" -q -T2 -19 -c "$stage/pkg.tar" >"$outabs"

# Read the finished package back: a compressor or archiver that died part-way
# leaves an archive pacman cannot open, and that must never be handed on.
bsdtar -tf "$outabs" >/dev/null

printf '%s\n' "$outabs"
