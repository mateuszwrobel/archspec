# `update` — Acceptance Tests

CLI-level, behavioral only. Each scenario runs the real command against a fixture project and asserts on observable output (exit code, stdout/stderr, written files). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `update` on a fixture project writes a byte-stable `architecture.spec.toml` that snapshots the extracted model (language, boundaries across both the unit and module tiers, dependency edges) in the declared spec format, refuses to overwrite an existing spec unless `--force` is given, and each error condition produces a clear message and a non-zero exit.

## Happy path

| # | Given | When | Then |
|---|---|---|---|
| 1 | project with supported-language sources and no `architecture.spec.toml` | run `update` on the project root | exit 0; `architecture.spec.toml` created; stdout names the generated file and suggests review and `verify` |
| 2 | project with sources under a subdirectory | run `update ./crates/auth` | exit 0; `architecture.spec.toml` created in `./crates/auth` |

## Spec content

| # | Given | When | Then |
|---|---|---|---|
| 3 | project whose units form known dependency edges | run `update` | exit 0; spec captures the language, one module per boundary, and the extracted edges as allowed constraints |
| 4 | single-crate project with internal modules | run `update` | exit 0; spec declares module-path boundaries (`matches.modules`) for the internal modules |
| 5 | project with units that have no inter-unit dependencies | run `update` | exit 0; spec declares the boundaries without invented edges |
| 16 | single-crate project with nested internal modules (e.g. `commands` with `commands::start`, `commands::init`, `commands::tests`) | run `update` | exit 0; spec declares ONE boundary per top-level module (`commands`, `config`, ...); NO boundary for `commands::start` or `commands::tests` |
| 17 | single crate where `commands::start` reaches `config` | run `update` | exit 0; the `commands` boundary's `allowed.depend_on` includes `config` |
| 19 | crate whose `lib.rs` declares a module gated under `#[cfg(feature = "tauri")]` | run `update` | exit 0; the seed declares a `feature_boundary` constraint with `feature = "tauri"` and the gated module; a `matches.modules` boundary still names the gated module |
| 22 | project with sources | run `update` | exit 0; the seed declares one global `no_cycles` constraint — `type = "no_cycles"` with **no** `modules` list (global = all declared modules) — exactly once |
| 23 | workspace whose crates form a dependency cycle | run `update` then `verify` | verify exits non-zero; the report names the cycle — the seeded guard is global, so seeded projects start cycle-clean |
| 20 | workspace with members that inherit a dependency via `{ workspace = true }` where a real member matches the dependency name | run `scan` on the workspace root | exit 0; the inherited dependency resolves to the workspace member unit (a unit edge), NOT listed in `external` |
| 21 | workspace root whose `Cargo.toml` has no `[package]` (pure workspace); member manifests carry publish/dependency facts | run `scan` on the workspace root | exit 0; `manifest_integrity` can still assert publish/dependency facts — the model surfaces per-member manifest facts, not an empty root manifest |

## Seed self-consistency

| # | Given | When | Then |
|---|---|---|---|
| 18 | same fixture as #16 | run `update` then `verify` | exit 0 on verify; the generated seed is self-consistent, no `missing edge` findings |

The seed declares no observed-shape constraints beyond `feature_boundary` and no test-gated-module boundaries (a module declared `#[cfg(test)]` or an equivalent test-only `cfg(all(..., test, ...))`, resolved by ancestry), but it always carries the global `no_cycles` guard. Descendant submodules fold into their top-level boundary via subtree matching, so an acyclic generated seed always verifies clean against itself (a tree with no edges among its boundaries reports the guard as a vacuous warning, exit 0). A module merely *named* `tests` without a test cfg is production and seeds a boundary like any other top-level module. The test-gating fact is a rust (`cfg(test)` modules), go (`*_test.go` files) and C# (projects whose resolved package set names a test runner — an xunit/nunit-family id or `Microsoft.NET.Test.Sdk`) provenance: go carries no serialized test-gating mark, so its test files simply seed no dependency and mark no module; the C# driver excludes test projects at the scan, so a test project contributes no unit boundary or dependency to the seed while a runner-free project merely named `*.Tests` (or `IsPackable=false`) seeds a boundary like any production unit. Go seeds follow the tree's derivation: a `go.work` workspace seeds one module per member (packages claimed through `matches.units`, `allowed.depend_on` from the real cross-member imports), a single-`go.mod` tree seeds the general per-package shape that `verify` groups via its declarations.

## Determinism

| # | Given | When | Then |
|---|---|---|---|
| 6 | project with sources; run once, then run again unchanged | run `update` twice | exit 0 both times; `architecture.spec.toml` byte-identical across runs |
| 7 | project with sources, `--force` | run `update --force` | exit 0; spec regenerated and valid |

## Existing spec

| # | Given | When | Then |
|---|---|---|---|
| 8 | `architecture.spec.toml` exists, contents known | run `update` | non-zero exit; message names the existing spec and `--force`; file byte-identical to before |
| 9 | `architecture.spec.toml` exists | run `update --force` | exit 0; spec regenerated from the current tree |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 10 | path does not exist | run `update <missing>` | non-zero exit; message identifies the invalid path |
| 11 | path is a regular file | run `update <file>` | non-zero exit; message identifies the invalid path |
| 12 | directory with no supported-language sources | run `update <dir>` | non-zero exit; message says no sources found |
| 13 | project containing a file with invalid syntax | run `update` | non-zero exit; message names the failing file; no spec written |
| 14 | language driver/toolchain unavailable | run `update` | non-zero exit; message names the missing driver and suggests `archspec doctor` |
| 15 | two positional paths given | run `update <a> <b>` | non-zero exit; message says at most one path accepted |
