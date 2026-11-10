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
warmup run. Date of measurement: 2026-11-09.

Commands (from the crate root; pipeline stdout is discarded, the table
prints on stderr):

```console
$ python3 scripts/bench/gen_corpus.py /var/tmp/hot34/corpus 800 25
$ python3 scripts/bench/bench_sweep.py guard --corpus /var/tmp/hot34/corpus \
      --manifest scripts/bench/corpus-manifest.txt
$ cargo run --release --features profile-phases --example phase-probe -- \
      /var/tmp/hot34/corpus 5 hotpath_report.json >/dev/null
```

5 runs, median of the per-run spans; `share` is the phase median over the sum
of the top-level phase medians (11 876 ms), and nested phases state their
share of the enclosing phase:

| phase | median ms | share |
|---|---:|---|
| `spec-load` | 3.7 | < 0.1 % |
| `scan` | 2 758.6 | 23.2 % |
| — `scan-facts` (inside `scan`) | 171.3 | 6.2 % of `scan` |
| `mapping` | 1.8 | < 0.1 % |
| `compare` | 7 661.0 | 64.5 % |
| — `cycle-detect` (inside `compare`) | 2 966.6 | 38.7 % of `compare`, 25.0 % of the total |
| `report` | 1 451.0 | 12.2 % |

Whole-run wall for the same runs: 12 130 ms mean (hotpath `verify-total`
histogram) — the ~2 % above the phase sum is dispatch plumbing outside the
declared boundaries (argument parsing, language selection, the agreement
precondition).

## The cycle-detect number in context

CONTEXT, superseded by the table above: an earlier fleet figure put a
"cold-report" cycle-detect share at ~59 %. It measured a different shape
(cold filesystem state, different tree) and has no record in this repository.
The measured share on the committed corpus is 38.7 % of `compare` and 25.0 %
of the verify total on the machine class above. Treat only the table as
data.

## What the harness is not

The numbers are data for agents and maintainers, not a shipped behavior
contract: no gate compares any runtime against this table. The harness never
enters a published artifact — the example is dev-only (`required-features`),
and the licensing surface excludes dev-dependencies by construction, so the
credits table states the shipped closure only.
