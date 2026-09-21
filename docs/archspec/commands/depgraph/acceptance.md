# `depgraph` — Acceptance Tests

CLI-level, behavioral. Each scenario runs the real command against a fixture and asserts on observable output (exit code, stdout/stderr, written files). Cross-driver behaviors live in the shared feature matrix; language-specific exactness is pinned in the Rust integration tests (`tests/depgraph.rs`).

## Views

| # | Given | When | Then |
|---|---|---|---|
| 1 | crate where `billing` imports `auth::Token` | `depgraph modules` | exit 0; `graph TD`; edge `billing --> auth` |
| 2 | same crate | `depgraph modules --format plantuml` | exit 0; `@startuml`/`@enduml`; the same edge |
| 3 | crate with a module symbol used across top-level modules | `depgraph api-usage` | exit 0; header `Target module \| Used by module \| APIs used`; a row `` | `auth` | `billing` | `Token` | `` |
| 4 | crate with a parent module and two children that reference each other | `depgraph submodules --parent parent` | exit 0; `graph TD`; `mod` node; edge `a --> b` |

## Destinations and determinism

| # | Given | When | Then |
|---|---|---|---|
| 5 | crate with a module graph | `depgraph modules --output g.mmd` | exit 0; `g.mmd` contains the edge; stdout empty |
| 6 | same crate, unchanged | run `depgraph modules` twice | byte-identical output |

## Cross-driver (feature matrix)

| # | Behavior | Rust | C# | Go |
|---|---|---|---|---|
| 7 | top-level edge projected between two modules | implemented | implemented | implemented on `go.work` workspaces (member edge renders); refused on single-`go.mod` trees (no tier in the model) |
| 8 | api-usage emits table or the no-usage statement | implemented | implemented | refused on single-`go.mod` trees; no workspace api-usage fixture is pinned |
| 9 | nested submodules expand to children + `mod` | implemented | n/a (nested parents fold to a lone `mod`) | implemented on `go.work` workspaces (member children render); refused on single-`go.mod` trees |

A single-`go.mod` Go tree carries no module tier in its scanned model, so every view is refused with the module-tier sentence and the feature probe (which materializes that tree) reports not-implemented; the scenario is skipped rather than failed. `go.work` workspaces DO carry the tier and render here — pinned in `tests/depgraph.rs`, not by the probe.

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 10 | any crate | `depgraph bogus` | exit 1; `unknown depgraph view` |
| 11 | any crate | `depgraph submodules` | exit 1; `requires --parent` |
| 12 | any crate | `depgraph modules --format bmp` | exit 1; `unsupported format` |
| 13 | any crate | `depgraph api-usage --format mermaid` | exit 1; `unsupported format` |
| 14 | any crate | `depgraph modules --parent x` | exit 1; `only valid for the submodules view` |
| 15 | any crate | `depgraph submodules --parent ghost` | exit 1; `parent module not found`, error lists the known top-level module names |

## Freshness (`--check`)

| # | Given | When | Then |
|---|---|---|---|
| 16 | `--output` destination already holds the generated body | `depgraph modules --output <f> --check` | exit 0; file untouched; stdout empty |
| 17 | `--output` destination differs from the generated body | `depgraph modules --output <f> --check` | exit 1; message says it differs and names the regenerate command; file untouched |
| 18 | no `--output` (depgraph has no `[output]` default) | `depgraph modules --check` | exit 1; message says `--check` requires an output destination |

## Model-faithfulness (non-goal guard)

The suite asserts archspec's own projection: import edges between top-level modules and archspec's node set. It deliberately does **not** assert any equivalence to the previous library generator's diagram (hub `--> root` edges or synthetic nodes). Byte-parity with the old `dependency-graph.md` is a documented non-goal (see `README.md`).

## C# honesty

| # | Given | When | Then |
|---|---|---|---|
| 19 | c# solution whose projects reference each other (Api→Application→Domain via ProjectReference) | `depgraph modules` | exit 0; edges `Api --> Application` and `Application --> Domain` (unit edges projected through module membership) |
| 20 | c# tree with parent namespace `Demo.Parent` | `depgraph submodules --parent Demo.Parent` | exit 0; resolves and lists the child namespaces; the bare segment `Parent` still resolves (compatibility fallback) |
| 21 | c# tree whose module edges carry no symbols | `depgraph api-usage` | exit 0; empty body carries the reason `Symbol-level facts are not emitted by the csharp driver.` |
| 22 | same tree | `depgraph submodules --parent Demo.Ghost` | exit 1; error lists the known names (e.g. `Demo::Parent`) |
| 23 | rust bin+lib crate whose binary consumes the library through the crate name (the dependency exists only at unit tier) | `depgraph api-usage` | exit 0; empty body carries the placement reason `Module edges record usage between modules of a single unit; cross-target links are recorded at unit granularity.`; a rust tree without cross-unit links keeps the bare sentence |
