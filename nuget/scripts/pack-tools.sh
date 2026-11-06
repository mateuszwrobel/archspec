#!/bin/sh
# Release-transaction leg: assemble the NuGet tools package from the five
# platform archives this version's matrix already built (decision-check
# record 2026-11-06, finding B2: the pack rides the release job's single
# gh release upload, so the release page materializes complete exactly once
# and is never edited again — the republish-freeze law, ADR-051).
#
# Layout (finding M4): runtimes/<rid>/native/archspec[.exe] — the documented
# native-files storage — plus build/archspec.props, the props selector that
# exposes the matching binary as $(ArchspecPath). The nuspec comes out of
# dotnet pack against the stub project (finding B1), with the version and
# repository URL injected from the command line (repo-identity injection law,
# the same shape npm/scripts/prepare-package.mjs follows): no payload
# template names the repository.
#
# The RID map below is the finding R4/M3 table: one row per archive the build
# matrix ships, each RID the props file names. linux-x64 carries the static
# musl x64 archive — it runs on glibc and musl x64 hosts alike. No asset
# means no RID: win-arm64, win-x86, linux-x86 and linux-musl-arm64 (the
# Alpine arm64 gap the props file and README name) pack nothing.
#
# usage: sh pack-tools.sh --dist <asset-dir> --version <ver> \
#          --repository <owner/name> [--out <dir>] [--dotnet <cmd>]

set -eu

script_dir="$(cd "$(dirname "$0")" && pwd)"
dist=""
version=""
repository=""
out=""
dotnet_bin=dotnet

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dist) dist="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    --repository) repository="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    --dotnet) dotnet_bin="$2"; shift 2 ;;
    *) echo "pack-tools: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "${dist}" ] && [ -n "${version}" ] && [ -n "${repository}" ] || {
  echo "usage: pack-tools.sh --dist <asset-dir> --version <ver> --repository <owner/name> [--out <dir>] [--dotnet <cmd>]" >&2
  exit 2
}
[ -d "${dist}" ] || { echo "pack-tools: asset dir ${dist} does not exist" >&2; exit 1; }
out="${out:-${dist}}"
mkdir -p "${out}"

nupkg="archspec-${version}.nupkg"
if [ -e "${out}/${nupkg}" ]; then
  echo "pack-tools: ${out}/${nupkg} already exists — refusing to overwrite an asset the release transaction may have packed already" >&2
  exit 1
fi

stage="$(mktemp -d)"
trap 'rm -rf "${stage}"' EXIT

# rid | release archive | staged member name — one row per matrix asset.
printf '%s\n' \
  "linux-x64|archspec-${version}-x86_64-unknown-linux-musl.tar.gz|archspec" \
  "linux-arm64|archspec-${version}-aarch64-unknown-linux-gnu.tar.gz|archspec" \
  "osx-x64|archspec-${version}-x86_64-apple-darwin.tar.gz|archspec" \
  "osx-arm64|archspec-${version}-aarch64-apple-darwin.tar.gz|archspec" \
  "win-x64|archspec-${version}-x86_64-pc-windows-msvc.zip|archspec.exe" \
  | while IFS='|' read -r rid asset member; do
    [ -f "${dist}/${asset}" ] || {
      echo "pack-tools: the asset set carries no ${asset} for RID ${rid} — refusing to pack an incomplete package" >&2
      exit 1
    }
    dest="${stage}/runtimes/${rid}/native"
    mkdir -p "${dest}"
    case "${asset}" in
      *.tar.gz) tar -xzf "${dist}/${asset}" -C "${dest}" "${member}" ;;
      *.zip)    unzip -p "${dist}/${asset}" "${member}" > "${dest}/${member}" ;;
      *) echo "pack-tools: ${asset} has an archive type this packer does not read" >&2; exit 1 ;;
    esac
    [ -f "${dest}/${member}" ] || {
      echo "pack-tools: ${asset} carries no ${member} member — refusing" >&2
      exit 1
    }
    case "${rid}" in
      win-*) : ;;
      *) chmod 755 "${dest}/${member}" ;;
    esac
  done

mkdir -p "${stage}/build"
cp "${script_dir}/../pkg/archspec.props" "${stage}/build/archspec.props"
cp "${script_dir}/../pkg/archspec.csproj" "${stage}/archspec.csproj"

"${dotnet_bin}" pack "${stage}/archspec.csproj" --configuration Release \
  --output "${stage}/out" \
  -p:ArchspecPackageId=archspec \
  -p:ArchspecVersion="${version}" \
  -p:ArchspecRepositoryUrl="https://github.com/${repository}"

packed="${stage}/out/archspec.${version}.nupkg"
[ -f "${packed}" ] || {
  echo "pack-tools: dotnet pack produced no ${packed}" >&2
  exit 1
}

# Fail closed on the members the consumer contract needs — a pack that
# silently dropped a staging item is the silent-pass class (finding B1's
# adversarial test: a missing asset member makes the pack refuse).
for want in \
  "build/archspec.props" \
  "runtimes/linux-x64/native/archspec" \
  "runtimes/linux-arm64/native/archspec" \
  "runtimes/osx-x64/native/archspec" \
  "runtimes/osx-arm64/native/archspec" \
  "runtimes/win-x64/native/archspec.exe"; do
  unzip -Z1 "${packed}" | grep -qxF "${want}" || {
    echo "pack-tools: the packed nupkg carries no ${want} member — refusing" >&2
    exit 1
  }
done

mv "${packed}" "${out}/${nupkg}"
# Portable sidecar shape — hash, two spaces, filename, LF — so the nupkg
# passes `sha256sum -c` like every other installable (finding M8: a hash of
# the shipped bytes, no determinism claim).
( cd "${out}" && sha256sum "${nupkg}" > "${nupkg}.sha256" )
echo "pack-tools: packed ${out}/${nupkg} (five runtimes RIDs + build/archspec.props)"
