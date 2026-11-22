# archspec performance — measured phase budgets

## Measured phase budgets (verify pipeline, GitHub issue #34)

The `verify` pipeline is timed phase by phase by the dev-only `phase-probe`
harness: a cargo feature (`profile-phases`) plus an example target. hotpath is
a dev-dependency only — the shipped dependency graph has no hotpath edge in
any feature shape (`cargo tree -e normal | grep -c hotpath` answers 0 with the
feature on and off; `tests/profile_gate.rs` proves it from the manifest and
the metadata graph, and the binary target refuses the feature at compile
time). With the feature off — every gate run and every shipped build — each
phase seam inlines to the wrapped call alone: no clock, no record, no label
string.

Machine class: linux x86_64 (kernel 6.18, glibc 2.44), AMD Ryzen 9 9950X
16-core (32 threads), 60 GB RAM, Rust 1.99.0, release profile with fat LTO.
Corpus: the committed synthetic tree of `scripts/bench/gen_corpus.py` — 800
workspace packages, 25 files each, 20 000 `.ts` files (~80 MB), byte-pinned
by `scripts/bench/corpus-manifest.txt`; the guard runs before timing. The
corpus verifies with findings (the file-tier rings are its point), so the
timed runs carry the full compare cost. Page cache warm after the discarded
warmup run. Date of measurement: 2026-11-11.

Commands (from the crate root; pipeline stdout is discarded, the table
prints on stderr):

```console
$ python3 scripts/bench/gen_corpus.py /var/tmp/hot34/corpus 800 25
$ python3 scripts/bench/bench_sweep.py guard --corpus /var/tmp/hot34/corpus \
      --manifest scripts/bench/corpus-manifest.txt
$ cargo run --release --features profile-phases --example phase-probe -- \
      /var/tmp/hot34/corpus 5 hotpath_report.json >/dev/null
```

9 runs, median of the per-run spans; `share` is the phase median over the sum
of the top-level phase medians (7 537 ms), and nested phases state their share
of the enclosing phase:

| phase | median ms | share |
|---|---:|---|
| `spec-load` | 3.3 | < 0.1 % |
| `scan` | 2 398.1 | 31.8 % |
| — `scan-facts` (inside `scan`) | 102.8 | 4.3 % of `scan` |
| `mapping` | 1.6 | < 0.1 % |
| `compare` | 4 108.0 | 54.5 % |
| — `cycle-detect` (inside `compare`) | 94.8 | 2.3 % of `compare`, 1.3 % of the total |
| `report` | 1 026.2 | 13.6 % |

Whole-run wall for the same runs: 7 600 ms mean (hotpath `verify-total`
histogram) — the ~1 % above the phase sum is dispatch plumbing outside the
declared boundaries (argument parsing, language selection, the agreement
precondition).

## The cycle-detect number in context

CONTEXT, superseded by the table above: an earlier fleet figure put a
"cold-report" cycle-detect share at ~59 %. It measured a different shape
(cold filesystem state, different tree) and has no record in this repository.
The 2026-11-09 table measured 2 966.6 ms on this corpus — 38.7 % of `compare`,
25.0 % of the verify total — on the machine class above. The phase then
rescanned the whole edge list per node inside its graph walks. The walks are
indexed now: the SCC walk reads per-node edge lists, and the file-tier
attribution legs memoize their per-file claim scans. Verdicts are unchanged —
findings and their order are byte-identical on the ring corpus and on the
first-party specs. The same corpus now measures 94.8 ms — 2.3 % of `compare`,
1.3 % of the total. Treat only the table as data.

## The scan row in context

CONTEXT, superseded for one phase by a later change: the `scan` row above was
measured 2026-11-11, before the ts multi-unit owner lookup was indexed (perf
record 2026-11-20 H1). The owner of each source file used to be found by
scanning every workspace member dir per row, so that phase cost
`rows × member dirs`. It now walks the file's own ancestor chain, so the cost
per row is the path's depth. On the same committed corpus the whole-tree
`scan` command wall moved from 3 089 ms to 382 ms (median of three warm runs
per binary, same machine class). The table keeps the 2026-11-11 numbers: the
verify pipeline was not re-probed for this change, so read `scan` as an upper
bound — the phase it names now costs less.
The shape stays pinned by `tests/ts_owner_shape.rs`, which times two generated
trees of equal file and module-edge-row counts over 2 048 and 64 member dirs.

## What the harness is not

The numbers are data for agents and maintainers, not a shipped behavior
contract: no gate compares any runtime against this table. The harness never
enters a published artifact — the example is dev-only (`required-features`),
and the licensing surface excludes dev-dependencies by construction, so the
credits table states the shipped closure only.
