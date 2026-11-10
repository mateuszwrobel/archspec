#!/usr/bin/env python3
"""archspec parity bench — deterministic medians + corpus guards.

Protocol (the 2026-11-09 issue #33 perf decision-check, findings M3/M4):
- Corpus guards BEFORE any timing: file count and aggregate content hash
  must equal scripts/bench/corpus-manifest.txt; a short or mismatched tree
  refuses the run, nonzero, and times nothing (finding M3).
- One discarded warmup scan (page cache), then medians of N (default 3)
  parallel runs and N serial runs (ARCHSPEC_THREADS=1), legs sequential
  in this one process (finding M4 — no cross-job denominator noise).
- Peak RSS per leg from /proc VmHWM sampling (GNU time is not portable;
  where /proc is absent, RSS reports 0 and the ratio assertions stand).

Usage:
  bench_sweep.py bench --corpus DIR --manifest MANIFEST --reps 3 \
      --bin NAME=/path/to/archspec [--bin ...] --out metrics.json
  bench_sweep.py guard --corpus DIR --manifest MANIFEST
  bench_sweep.py manifest --corpus DIR --out MANIFEST
  bench_sweep.py self-test        # proves the corpus guard refuses a stub tree

Aggregate hash definition (portable across absolute paths): run from the
corpus root; for every `*.ts` file take sha256 of its bytes keyed by the
sorted relative posix path; the aggregate is the sha256 over the sorted
lines "<sha256>  <relpath>".
"""
import hashlib
import json
import os
import re
import statistics
import subprocess
import sys
import tempfile
import threading
import time

GUARD_FLOOR_RATIO = 0  # count must EQUAL the manifest count (M3: never short)


def aggregate_hash(corpus: str) -> tuple[int, str]:
    digests = []
    for dirpath, _dirnames, filenames in os.walk(corpus):
        for name in filenames:
            if not name.endswith(".ts"):
                continue
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, corpus).replace(os.sep, "/")
            h = hashlib.sha256()
            with open(full, "rb") as fh:
                for chunk in iter(lambda: fh.read(1 << 20), b""):
                    h.update(chunk)
            digests.append(f"{h.hexdigest()}  {rel}")
    digests.sort()
    blob = "\n".join(digests).encode()
    return len(digests), hashlib.sha256(blob).hexdigest()


def read_manifest(path: str) -> dict:
    data = {}
    with open(path) as fh:
        for line in fh:
            m = re.match(r"^(\w+)\s*=\s*(.+?)\s*$", line)
            if m:
                data[m.group(1)] = m.group(2)
    return data


def guard(corpus: str, manifest_path: str) -> int:
    manifest = read_manifest(manifest_path)
    count, digest = aggregate_hash(corpus)
    want_count = int(manifest["ts_files"])
    want_digest = manifest["aggregate_sha256"]
    if count != want_count:
        print(
            f"CORPUS-GUARD REFUSED: {count} .ts files, manifest says "
            f"{want_count} — tree is short or wrong, nothing was timed",
            file=sys.stderr,
        )
        return 1
    if digest != want_digest:
        print(
            f"CORPUS-GUARD REFUSED: aggregate sha256 {digest} != manifest "
            f"{want_digest} — corpus bytes drifted, nothing was timed",
            file=sys.stderr,
        )
        return 1
    print(f"corpus guard green: {count} files, aggregate {digest}")
    return 0


class PeakRss(threading.Thread):
    """Polls VmHWM of a pid; keeps the max seen (KB)."""

    def __init__(self, pid: int):
        super().__init__(daemon=True)
        self.pid = pid
        self.kb = 0
        self._stop = threading.Event()
        self.path = f"/proc/{pid}/status"

    def run(self):
        while not self._stop.is_set():
            try:
                with open(self.path) as fh:
                    for line in fh:
                        if line.startswith("VmHWM:"):
                            self.kb = max(self.kb, int(line.split()[1]))
                            break
            except OSError:
                return
            time.sleep(0.02)

    def stop(self):
        self._stop.set()


def timed_run(binary: str, corpus: str, env: dict, deadline: float) -> tuple[float, int]:
    start = time.perf_counter()
    devnull = subprocess.DEVNULL
    proc = subprocess.Popen(
        [binary, "scan", corpus], stdout=devnull, stderr=devnull, env=env
    )
    mon = PeakRss(proc.pid)
    mon.start()
    try:
        proc.wait(timeout=deadline)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()
        raise RuntimeError(f"leg exceeded {deadline}s deadline — run VOID")
    mon.stop()
    mon.join(timeout=1)
    return time.perf_counter() - start, mon.kb


def bench(args: dict) -> int:
    if guard(args["corpus"], args["manifest"]) != 0:
        return 1
    reps = int(args.get("reps", "3"))
    deadline = float(args.get("deadline", "900"))
    bins = [b.split("=", 1) for b in args["bin"]]
    rows = []
    for name, path in bins:
        base_env = dict(os.environ)
        par_env = dict(base_env)
        par_env.pop("ARCHSPEC_THREADS", None)
        ser_env = dict(base_env)
        ser_env["ARCHSPEC_THREADS"] = "1"
        # one discarded warmup scan per binary (page cache), then measured legs
        timed_run(path, args["corpus"], par_env, deadline)
        par_walls, par_rss, ser_walls, ser_rss = [], [], [], []
        for _ in range(reps):
            w, r = timed_run(path, args["corpus"], par_env, deadline)
            par_walls.append(w)
            par_rss.append(r)
        for _ in range(reps):
            w, r = timed_run(path, args["corpus"], ser_env, deadline)
            ser_walls.append(w)
            ser_rss.append(r)
        row = {
            "binary": name,
            "path": path,
            "wall_p": statistics.median(par_walls),
            "wall_s": statistics.median(ser_walls),
            "ratio": statistics.median(par_walls) / statistics.median(ser_walls),
            "rss_p_kb": max(par_rss),
            "rss_s_kb": max(ser_rss),
            "runs_p": par_walls,
            "runs_s": ser_walls,
        }
        rows.append(row)
        print(
            f"{name}\tparallel {row['wall_p']:.2f}s\tpar {row['rss_p_kb']}KB\t"
            f"serial {row['wall_s']:.2f}s\tserial {row['rss_s_kb']}KB\t"
            f"ratio {row['ratio']:.2f}"
        )
    out = args.get("out")
    if out:
        with open(out, "w") as fh:
            json.dump({"corpus": os.path.abspath(args["corpus"]), "reps": reps,
                       "rows": rows}, fh, indent=2)
            fh.write("\n")
    return 0


def self_test() -> int:
    """Prove the corpus guard refuses a stub tree before timing (finding M3)."""
    payload = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    gen = os.path.join(payload, "scripts", "bench", "gen_corpus.py")
    with tempfile.TemporaryDirectory() as tmp:
        stub = os.path.join(tmp, "stub")
        subprocess.run([sys.executable, gen, stub, "1", "1"], check=True,
                       stdout=subprocess.DEVNULL)
        manifest = os.path.join(tmp, "manifest.txt")
        if manifest_cmd({"corpus": stub, "out": manifest}) != 0:
            print("SELF-TEST FAIL: manifest generation failed", file=sys.stderr)
            return 1
        # guard over the stub vs its own manifest must be GREEN (proves the
        # check function works at all) ...
        if guard(stub, manifest) != 0:
            print("SELF-TEST FAIL: guard rejected a tree matching its manifest",
                  file=sys.stderr)
            return 1
        # ... and against the real pinned manifest must be RED, nonzero,
        # before any timing (the adversarial form).
        real = os.path.join(payload, "scripts", "bench", "corpus-manifest.txt")
        if guard(stub, real) == 0:
            print("SELF-TEST FAIL: guard ACCEPTED a stub tree against the pinned "
                  "manifest — the ratio guard could pass on zero load",
                  file=sys.stderr)
            return 1
    print("self-test green: stub corpus refused before timing")
    return 0


def manifest_cmd(args: dict) -> int:
    count, digest = aggregate_hash(args["corpus"])
    gen = os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen_corpus.py")
    gen_sha = hashlib.sha256(open(gen, "rb").read()).hexdigest()
    with open(args["out"], "w") as fh:
        fh.write("# pinned corpus identity for the parity sweep and the CI perf probe\n")
        fh.write("# aggregate = sha256 over sorted \"<sha256>  <relpath>\" lines of every *.ts file,\n")
        fh.write("# computed from the corpus root — path-independent across machines.\n")
        fh.write(f"ts_files = {count}\n")
        fh.write(f"aggregate_sha256 = {digest}\n")
        fh.write(f"gen_corpus_sha256 = {gen_sha}\n")
    print(f"manifest written: {count} files, aggregate {digest}")
    return 0


def parse(argv: list[str]) -> dict:
    args = {}
    rest = list(argv)
    if not rest:
        print(__doc__.strip(), file=sys.stderr)
        sys.exit(2)
    args["cmd"] = rest.pop(0)
    while rest:
        key = rest.pop(0)
        if not key.startswith("--"):
            continue
        k = key[2:]
        if rest and not rest[0].startswith("--"):
            args.setdefault(k, [])
            args[k].append(rest.pop(0))
        else:
            args[k] = True
    return {k: (v[0] if isinstance(v, list) and len(v) == 1 else v) for k, v in args.items()}


def main(argv: list[str]) -> int:
    # argv arrives WITHOUT the program name (parse treats argv[0] as the verb)
    args = parse(argv)
    cmd = args["cmd"]
    if cmd == "bench":
        # --bin may appear multiple times; parse() stores lists for repeats
        bins = args.get("bin", [])
        if isinstance(bins, str):
            bins = [bins]
        args["bin"] = bins
        return bench(args)
    if cmd == "guard":
        return guard(args["corpus"], args["manifest"])
    if cmd == "manifest":
        return manifest_cmd(args)
    if cmd == "self-test":
        return self_test()
    print(__doc__.strip(), file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
