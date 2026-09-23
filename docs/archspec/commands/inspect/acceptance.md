# `inspect` — Acceptance Tests (Phase 1)

CLI-level, behavioral only. Each scenario runs the real command against a fixture crate and asserts on observable output (exit code, stdout/stderr, written files). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `inspect` on a fixture Rust crate renders a byte-stable diagram (Mermaid by default, PlantUML via `--format plantuml`) whose nodes are the crate's source files grouped into folder subgraphs and whose visible edges exactly match the resolved internal imports plus `mod` declarations — and each error condition produces a clear message and a non-zero exit.

## Happy path

| # | Given | When | Then |
|---|---|---|---|
| 1 | crate with `src/lib.rs`, `src/a.rs`, `src/b.rs`; `a.rs` imports `crate::b::Thing` | run `inspect` on crate root | exit 0; diagram has a node per file; edge `src/a.rs -> src/b.rs` |
| 2 | crate with `src/orchestration/common.rs`, `src/orchestration/state.rs`; `state.rs` imports `crate::orchestration::common::Util` | run `inspect` on crate root | exit 0; edge `src/orchestration/state.rs -> src/orchestration/common.rs`; `orchestration` subgraph contains its files |

## Resolution behavior

| # | Given | When | Then |
|---|---|---|---|
| 3 | `src/parent/mod.rs` declares submodules `common`, `pipeline`; `src/parent/pipeline.rs` uses `super::common::X` | run `inspect` | exit 0; edge `src/parent/pipeline.rs -> src/parent/common.rs` |
| 4 | `src/a.rs` declares `mod inner` inline; `src/c.rs` imports `crate::a::inner::Type` | run `inspect` | exit 0; edge `src/c.rs -> src/a.rs` (module `a::inner` lives in `src/a.rs`) |
| 5 | files use `self::` and `crate::` references within their own module | run `inspect` | exit 0; no spurious self-edges; imports resolve to the owning files |

## Rendering and determinism

| # | Given | When | Then |
|---|---|---|---|
| 6 | crate with imports | run `inspect --format plantuml` | exit 0; PlantUML output contains nodes and the resolved edges |
| 7 | crate with imports | run `inspect --output out.mmd` | exit 0; `out.mmd` contains the diagram; stdout is `wrote out.mmd` |
| 8 | same crate, unchanged | run `inspect` twice | both runs byte-identical output |

## Excluded paths

Generated, vendored, and hidden directories are never scanned — their `.rs` files do not appear as nodes, and imports resolved into them are dropped. The exclusion set matches `scan`'s so both commands see the same source tree.

| # | Given | When | Then |
|---|---|---|---|
| 15 | crate with `target/debug/build/<crate>/out/generated.rs` (a build artifact) | run `inspect` on crate root | exit 0; `generated.rs` does NOT appear as a node; no edge into it |
| 16 | crate with a `vendor/` directory containing vendored `.rs` files | run `inspect` on crate root | exit 0; the vendored files do NOT appear as nodes |
| 17 | crate with hidden directories (`.git/`, `.cargo/`) containing `.rs` files | run `inspect` on crate root | exit 0; hidden-dir files do NOT appear as nodes |
| 18 | crate where every `.rs` file sits under excluded paths (e.g. only `target/` remains) | run `inspect` on crate root | non-zero exit; message says `no Rust sources found under:` (language detected, but the file scanner found every source excluded) |
| 19 | crate with a `target/` dir plus real `src/` sources | run `inspect` on crate root | exit 0; nodes are exactly the real sources; node count excludes `target/` |

## Structural views

`inspect` exposes subcommands that render the extracted model (units + modules + edges) rather than the raw file tree. These read the same model `scan` extracts, so the diagram and the model never disagree (rows 28–29 pin it on the csharp and go trees whose boundary sets are richest in addressing facts).

| # | Given | When | Then |
|---|---|---|---|
| 20 | crate with modules and module edges | run `inspect tree` on crate root | exit 0; diagram groups nodes by unit and module (not raw files), edges match the extracted module edges |
| 21 | crate with modules and module edges | run `inspect scanner` on crate root | exit 0; diagram reflects the scan-phase model (units + module boundaries + edges) |

## Model-view agreement with `scan`

For the trees whose boundaries a naive using-endpoint reading would mis-state, the agreement above is asserted on the real model: the model view must draw exactly the boundaries `scan` records — never a mixture, none dropped, none invented.

| # | Given | When | Then |
|---|---|---|---|
| 28 | c# tree with a type-targeted `using` plus a type-position reference (the boundary sets differ from the naive using-endpoint reading) | run `inspect tree` (and `inspect scanner`) and `scan` on the same tree | exit 0; the model view draws exactly the `scan` module boundaries — same from/to endpoints, none dropped, none invented — the owner-addressed edges including the type-position crossing |
| 29 | go tree: single module with a sibling-package import, and a `go.work` workspace tree (cross-member plus within-member imports) | run `inspect tree` (and `inspect scanner`) and `scan` on the same tree | exit 0; the model-view boundaries equal the `scan` model — the intra-module package edge of `scan` row 56 and, for the workspace, the cross-member plus within-member edges of row 58 |
| 30 | any supported tree (rust, csharp or go) | run `inspect` file-level map | exit 0; the map is file-granular discovery produced without model extraction, so it reflects the parsed files regardless of what the model carries |
| 31 | go tree whose packages record no reference to each other (single-package module) | run `inspect tree` | exit 1; stderr exactly `inspect tree needs the module tier, which this model has none of; the module tier is derived from the tree's package references, which this tree records none of, 'inspect scanner' renders the unit-tier model`; on this same tree `inspect scanner` renders the unit-tier model and exits 0 |

The refusal sentence of row 31 is one of the module-tier refusal family:
`depgraph` refuses the same model state (`commands/depgraph/acceptance.md`,
row 31 — see it) with its own sentence — one family, per-view wording. The two
share the clause "needs the module tier, which this model has none of; the
module tier is derived from package references, which this tree records none
of" — here with the references addressed as "the tree's", the same fact either
way — and each view states its own remedy: this sentence names `inspect
scanner` as the unit-tier fallback, the depgraph sentence adds none beyond the
fact. The sentences stay per-view by decision, one decision per view —
`worklog/done/workplan_depgraph_message_real_remedies.md`.

## Roles in the tree

The tree marks the special nodes with the model's own words (roles US 06): a node whose model path carries a roles entry gets that role as a label marker — a pure lookup, so a node is marked exactly where the model speaks, on every driver.

| # | Given | When | Then |
|---|---|---|---|
| 32 | canonical probe tree per language (rust: `facade` unit roots; c#: `composition` entrypoint root; go: `composition` main-package unit) | run `inspect tree` | exit 0; every roles entry's path renders with its marker — the node's visible label carries ` [<role>]` — and a model without roles renders byte-identical to the pre-marker era |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 9 | path does not exist | run `inspect <missing>` | non-zero exit; message identifies the invalid path |
| 10 | path is a regular file | run `inspect <file.rs>` | non-zero exit; message identifies the invalid path |
| 11 | directory with no supported-language sources | run `inspect <dir>` | non-zero exit; message says no supported-language sources found |
| 12 | crate containing a file with invalid Rust syntax | run `inspect` | non-zero exit; message names the failing file; no partial diagram |
| 13 | unsupported format value | run `inspect --format bmp` | non-zero exit; message names the unsupported value |
| 14 | two positional paths given | run `inspect <a> <b>` | non-zero exit; message says at most one path accepted |

## Freshness (`--check`)

| # | Given | When | Then |
|---|---|---|---|
| 22 | `--output` destination already holds the generated diagram | run `inspect --output <f> --check` | exit 0; file untouched; stdout is `ok: <f> up to date` |
| 23 | `--output` destination differs from the generated diagram | run `inspect --output <f> --check` | non-zero exit; message says it differs and names the regenerate command; file untouched |
| 24 | no `[output] inspect` configured and no `--output` | run `inspect --check` | non-zero exit; message says `--check` requires an output destination |
