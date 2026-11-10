#!/usr/bin/env python3
"""Generate a deterministic synthetic TypeScript workspace corpus.

This is the corpus from issue #33's repro, committed so every parity sweep
and every CI perf-probe run benches the SAME bytes. Shapes:

- root package.json with workspaces: ["packages/*"]
- <packages> member packages (default 800), each with <files> sources
  (default 25) — `gen_corpus.py tree 800 25` produces 20,000 .ts files
- each file imports one sibling file (relative), one neighbour-package
  file (relative, next package modulo the count) and one external
  package name (never resolvable — no node_modules is generated)
- every file is padded to ~4 KB with index-derived comment lines so the
  total is ~80 MB and no two files are byte-identical

The generator is pure: same arguments, same bytes, on every platform.
No timestamps, no randomness. Guards that consume it (bench_sweep.py and
the perf-probe workflow) assert the file count and the aggregate hash in
scripts/bench/corpus-manifest.txt BEFORE timing — a stub tree must never
produce ratios (decision-check 2026-11-09, finding M3).

Usage: gen_corpus.py <outdir> [packages=800] [files=25]
"""
import json
import os
import sys

EXTERNALS = ["tslib", "clsx", "zod"]


def file_source(pkg: int, idx: int, files: int, packages: int) -> str:
    sibling = (idx + 1) % files
    neighbor_pkg = (pkg + 1) % packages
    external = EXTERNALS[(pkg + idx) % len(EXTERNALS)]
    lines = [
        f"// synthetic corpus file pkg-{pkg} f{idx} — deterministic, do not edit",
        f'import {{ S{pkg}_{sibling} }} from "./f{sibling}";',
        f'import {{ N{neighbor_pkg}_{idx} }} from "../../pkg{neighbor_pkg}/src/f{idx}";',
        f'import {{ E }} from "{external}";',
        "",
        f"export const V{pkg}_{idx} = {{",
        f'  name: "pkg{pkg}-f{idx}",',
        f"  sibling: S{pkg}_{sibling},",
        f"  neighbor: N{neighbor_pkg}_{idx},",
        f"  external: E,",
        "};",
        "",
    ]
    # Pad with index-derived comment lines to ~4 KB. The padding text is a
    # pure function of (pkg, idx, pad line number): deterministic everywhere.
    pad_no = 0
    while sum(len(x) + 1 for x in lines) < 4096:
        lines.append(
            f"// pad {pkg:04d}-{idx:02d}-{pad_no:04d} "
            + "x" * ((pkg * 31 + idx * 7 + pad_no * 13) % 40)
            + " "
        )
        pad_no += 1
    return "\n".join(lines)


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[1] in ("-h", "--help"):
        print(__doc__.strip(), file=sys.stderr)
        return 2
    outdir = argv[1]
    packages = int(argv[2]) if len(argv) > 2 else 800
    files = int(argv[3]) if len(argv) > 3 else 25

    os.makedirs(os.path.join(outdir, "packages"), exist_ok=True)
    root_manifest = {
        "name": "archspec-synth-corpus",
        "private": True,
        "workspaces": ["packages/*"],
    }
    # newline="\n" everywhere: the corpus bytes must be identical on every
    # platform, or the pinned aggregate hash would reject a windows/darwin
    # generation for newline reasons instead of content ones.
    with open(os.path.join(outdir, "package.json"), "w", newline="\n") as fh:
        json.dump(root_manifest, fh, indent=2)
        fh.write("\n")

    for pkg in range(packages):
        pkg_dir = os.path.join(outdir, "packages", f"pkg{pkg}")
        src_dir = os.path.join(pkg_dir, "src")
        os.makedirs(src_dir, exist_ok=True)
        pkg_manifest = {
            "name": f"@syn/pkg{pkg}",
            "version": "0.0.0",
            "main": "src/f0.ts",
        }
        with open(os.path.join(pkg_dir, "package.json"), "w", newline="\n") as fh:
            json.dump(pkg_manifest, fh, indent=2)
            fh.write("\n")
        for idx in range(files):
            with open(os.path.join(src_dir, f"f{idx}.ts"), "w", newline="\n") as fh:
                fh.write(file_source(pkg, idx, files, packages))

    print(f"{packages * files} .ts files under {outdir}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
