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
| 5 | crate with a module graph | `depgraph modules --output g.mmd` | exit 0; `g.mmd` contains the edge; stdout is `wrote g.mmd` |
| 6 | same crate, unchanged | run `depgraph modules` twice | byte-identical output |

## Cross-driver (feature matrix)

| # | Behavior | Rust | C# | Go |
|---|---|---|---|---|
| 7 | top-level edge projected between two modules | implemented | implemented | implemented — `go.work` member edges render, and a single-`go.mod` tree renders its package graph (the trivial fold renders one level deeper); refused where the tree records no package references |
| 8 | api-usage emits table or the no-usage statement | implemented | implemented | implemented — the statements and reasons are pinned in rows 27–29; refused where the tree records no package references |
| 9 | nested submodules expand to children + `mod` | implemented | n/a in the shared matrix (the harness materializes nested trees for rust only); the driver's own shape: the parent's own module folds to the lone `mod` node, the child namespaces render as nodes and the edges between children render (row 20; pinned in `tests/depgraph.rs` and `tests/depgraph_reasons.rs::csharp_submodules_fold_prints_child_nodes_mod_and_edge_verbatim`) | implemented — `go.work` members and single-`go.mod` trees whose packages reference each other render; refused where the tree records no package references |

A single-`go.mod` Go tree populates the module tier from its own package references, so every view renders it. Only a Go tree recording **no** package references at all lacks the tier; there every view is refused with the module-tier sentence (row 31). `go.work` workspaces carry the tier through their members and render here — pinned in `tests/depgraph.rs`, not by the probe.

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
| 16 | `--output` destination already holds the generated body | `depgraph modules --output <f> --check` | exit 0; file untouched; stdout is `ok: <f> up to date` |
| 17 | `--output` destination differs from the generated body | `depgraph modules --output <f> --check` | exit 1; message says it differs and names the regenerate command; file untouched |
| 18 | no `--output` (depgraph has no `[output]` default) | `depgraph modules --check` | exit 1; message says `--check` requires an output destination |

## Model-faithfulness (non-goal guard)

The suite asserts archspec's own projection: import edges between top-level modules and archspec's node set. It deliberately does **not** assert any equivalence to the previous library generator's diagram (hub `--> root` edges or synthetic nodes). Byte-parity with the old `dependency-graph.md` is a documented non-goal (see `README.md`).

## C# honesty

| # | Given | When | Then |
|---|---|---|---|
| 19 | c# solution whose projects reference each other (Api→Application→Domain via ProjectReference) | `depgraph modules` | exit 0; edges `Api --> Application` and `Application --> Domain` (unit edges projected through module membership) |
| 20 | c# tree with parent namespace `Demo.Parent` and child namespaces `Demo.Parent.A`, `Demo.Parent.B` | `depgraph submodules --parent Demo.Parent` | exit 0; renders nodes `A`, `B` and `mod` with edge `A --> B`: the parent's own module folds to the lone `mod` node, the child namespaces render as nodes and the edges between children render — the shape rust row 4 states; the fold is a projection choice shared across drivers (`worklog/done/workplan_depgraph_csharp_views_honest.md`), not a degradation; the bare segment `Parent` still resolves (compatibility fallback) |
| 21 | c# tree whose module edges carry no symbols | `depgraph api-usage` | exit 0; empty body carries exactly `No internal API usage details found. No symbol facts were emitted for this tree.` — symbol facts are emitted wherever the tree carries cross-boundary references, so an empty grouping states a fact of this tree, not of the driver |
| 22 | same tree | `depgraph submodules --parent Demo.Ghost` | exit 1; error lists the known names (e.g. `Demo::Parent`) |
| 23 | rust bin+lib crate whose binary consumes the library through the crate name (the dependency exists only at unit tier) | `depgraph api-usage` | exit 0; empty body carries the placement reason `Module edges record usage between modules of a single unit; cross-target links are recorded at unit granularity.`; the bare sentence survives only for symbol-free models — a rust tree whose module edges DO carry symbols renders rows or states the fold placement (rows 32, 33) |

## api-usage symbols

| # | Given | When | Then |
|---|---|---|---|
| 27 | a c# tree whose model carries module edges with symbols (type-targeted usings, type positions), or a go.work workspace whose cross-member edge carries an exported selector | `depgraph api-usage` | exit 0; the table prints the symbols the model carries, grouped per (target top-level module, using top-level module) — backticked, comma-separated, sorted, the rust shape (row 3); the printed rows equal the grouping of the `scan` model |
| 29 | c# tree that yields no symbols (external-only usings) | `depgraph api-usage` | exit 0; the empty body states the tree fact exactly — `No internal API usage details found. No symbol facts were emitted for this tree.` — and never claims the csharp driver emits no symbols |
| 31 | go tree whose packages record no reference to each other (single-package module) | `depgraph modules` | exit 1; stderr exactly `depgraph needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of` |
| 32 | any tree whose module edges carry symbols and whose top-level projection folds to one node with distinct paths below it (single-module go tree's sibling-package edge with an exported selector) | `depgraph api-usage` | exit 0; the table renders at the deeper granularity the modules graph shows — the edge's symbols appear in a row per (target, using) path below the fold — and the bare statement never prints on a symbol-bearing model |
| 33 | go tree whose symbol-carrying edges still land on one rendered node at every depth (member packages sharing a final segment under one address prefix) | `depgraph api-usage` | exit 0; the empty body carries exactly `No internal API usage details found. Module edges carry symbols, but every symbol-carrying edge stays inside one rendered module; the usage sits between modules below the granularity this view renders.` — symbols exist, placement is stated, nothing is unexplained |

Row 31's refusal sentence is one of the module-tier refusal family: `inspect
tree` refuses the same model state (`commands/inspect/acceptance.md`, row 31 —
see it) with its own sentence — one family, per-view wording. The two share
the clause "needs the module tier, which this model has none of; the module
tier is derived from package references, which this tree records none of" —
the inspect sentence addresses the references as "the tree's", the same fact
either way — and each view states its own remedy: here every module-tier view
(`modules`, `api-usage`, `submodules`) refuses with this one sentence, nothing
beyond the fact, while the inspect sentence adds `inspect scanner` as its
unit-tier fallback. The sentences stay per-view by decision, one decision per
view — `worklog/done/workplan_depgraph_message_real_remedies.md`.

## api-usage outcomes as a state matrix

The grouping granularity is one decision for every view: the api-usage table
reads the same projection the `modules` graph renders, top-level or one level
below a trivial fold (row 32), and an edge that projects onto one node never
opens a group. When the grouping is non-empty a table prints and no reason
arm is consulted; when it is empty, `api_usage_empty_reason` decides in arm
order, first match wins. The matrix mirrors that order cell for cell; the
arms are **decision rules, not per-state guarantees**: the csharp arm precedes
the symbol check, so an empty c# grouping prints the tree-fact sentence
whether or not the model's module edges carry symbols (the reachability probe
and its consequence — a misworded sentence on a symbol-bearing c# tree,
reported and not fixed here, no row promises the fold sentence for c# — are in
`worklog/workplan_archspec_cvd_precision/verification.md`, US 03).

| State | Example trees | Rendered outcome | Quoting row | Pinning test(s) |
|---|---|---|---|---|
| symbols on module edges project across rendered nodes (no arm consulted) | rust crate `billing`→`auth::Token`; c# solution `App.Api`→`App.Core`; go.work cross-member selector | table at the rendered granularity, one row per (target, using) pair | 3, 27 | `rust_api_usage_symbol_rows_come_from_the_model`, `csharp_api_usage_prints_model_grouped_symbols_per_pair`, `go_workspace_api_usage_prints_selector_symbol` (`tests/depgraph_reasons.rs`); `api_usage_locks_symbol_rows` (`tests/depgraph.rs`) |
| symbols one fold below a trivial projection (grouping non-empty at the deeper granularity) | single-`go.mod` tree's sibling-package edge with an exported selector; rust nested-parent siblings | table at the deeper granularity the modules graph shows — symbols in the folded paths' rows | 32 | `go_single_module_api_usage_renders_folded_package_rows` (`tests/depgraph_reasons.rs`); `rust_api_usage_renders_folded_child_module_rows` (`tests/depgraph.rs`) |
| empty grouping, arm 1: language is csharp, whatever else the model carries | c# tree with external-only usings; also a symbol-bearing c# tree whose symbol-carrying edges all land in one rendered node | empty body carries exactly the tree-fact sentence — by arm order; on the symbol-bearing variant the sentence's absence wording misstates the model (probe record; a reported wording defect, not a contract) | 21, 29 (arm); 32 (never-bare invariant) | `csharp_api_usage_without_symbol_facts_states_the_tree_not_a_driver_rule` (`tests/depgraph_reasons.rs`); `csharp_api_usage_empty_carries_reason_sentence` (`tests/depgraph.rs`) |
| empty grouping, arm 2: rust with cross-unit links on the unit tier | rust bin+lib crate consuming the library through the crate name | empty body carries exactly the unit-tier placement sentence | 23 | `api_usage_empty_with_unit_links_carries_placement_reason` (`tests/depgraph.rs`) |
| empty grouping, arm 3: module edges carry symbols, no earlier arm fired | member packages sharing a final segment under one address prefix | empty body carries exactly the fold-placement sentence — symbols exist, placement is stated | 33 | `symbols_inside_one_rendered_node_carry_the_fold_placement_reason` (`tests/depgraph_reasons.rs`) |
| empty grouping, arm 4: no symbol facts on module edges (not csharp, no rust unit links) — any language | symbol-free go tree that keeps the module tier | the bare statement, legal only here | 23 (tail) | `api_usage_symbol_free_go_model_keeps_bare_empty_statement` (`tests/depgraph.rs`); `api_usage_markdown_states_none_found_when_empty` (unit) |
| invariant over every empty arm: a symbol-bearing model never prints the bare statement | every symbol-bearing shape, all three drivers | a table or a reasoned sentence — never the bare one | 32 | `symbol_bearing_models_never_print_the_bare_empty_statement` (`tests/depgraph_reasons.rs`) |
| no module tier at all | single-package go module | **not a cell of this table**: every view refuses up front with the package-reference rule (row 31) | 31 | `go_tier_less_refusal_names_the_missing_fact` (`tests/depgraph_reasons.rs`); `depgraph_views_refuse_tier_less_single_package_go` (`tests/depgraph.rs`) |

Every outcome above is the pre-marker byte shape: rows 34–38 decide that
roles and fold markers touch neither the table nor the emptiness statements,
and a fold-shape node set (row 20) carries markers only where roles paths
project onto its nodes (row 37 states the no-node case).

## Role markers

| # | Given | When | Then |
|---|---|---|---|
| 34 | a tree whose `roles` map states a role at any granularity this view's projection renders (rust bin composition `tool::main`, go main package at either fold depth with its import-path key spelling, c# composition-root project key `Shop::Api`) | `depgraph modules`, `depgraph submodules --parent <m>`, `--format mermaid` and `plantuml` | exit 0; the node the role paths fold onto carries that role's marker (roles US 06 syntax: ` [facade]` / ` [composition]` label suffix, `<<facade>>` / `<<composition>>` stereotype on the bare declaration) while node ids and every edge line keep their exact pre-marker bytes |
| 35 | a node onto which several projected roles paths fold and state different roles (two projects `X::Api`, `Y::Api` folding onto one `Api` node) | `depgraph modules` | exit 0; the node carries no marker and no marker syntax appears anywhere — the fold is too coarse to state one role, silence beats an aggregate claim, and no node ever carries two markers; edges and the node itself are unmoved |
| 36 | several projected roles paths folding onto one node and agreeing on one role | `depgraph modules` | exit 0; the node carries that role's marker exactly once |
| 37 | a roles key whose projection names a node the view does not render (unit-tier rust/c# keys folding to `root`; keys below/outside a submodule view's parent) | `depgraph modules`, `depgraph submodules --parent <m>` | exit 0; the key contributes nothing — no node is invented to carry a role and bytes equal the pre-plan golden (the rust probe tree's golden is its own byte pin) |
| 38 | any tree with derived roles | `depgraph api-usage` | exit 0; the three fixed columns and the rows are unchanged by roles and the emptiness statements are untouched (rows 21, 23, 29, 33 keep their bytes) — a role is not a usage fact, and the roles answer for module-level questions is the modules graph plus the report's Roles section; this table stays silent by decision, not oversight |

Pins (rows 34–38 are roles-views US 02's append; row numbers are ids —
the gaps at 24–26, 28 and 30 are sibling plans' reserved space, and no number
collides): row 34 — `tests/depgraph.rs::modules_marks_rust_composition_root_on_the_main_node`,
`…::modules_rust_composition_mark_is_a_bare_stereotype_in_plantuml`,
`…::modules_marks_go_main_package_at_the_folded_granularity`,
`…::modules_marks_deeply_addressed_go_main_package_below_the_fold`,
`…::modules_marks_csharp_project_root_composition` and
`…::modules_marked_output_is_byte_identical_across_runs` (determinism arm);
row 35 — `…::modules_conflicting_fold_prints_no_marker`; row 36 —
`…::modules_agreeing_fold_marks_the_node_once`; row 37 —
`…::modules_rust_facade_key_folds_to_no_rendered_node_and_prints_no_marker`
and `…::modules_bytes_on_the_rust_probe_tree_match_the_golden_that_carries_no_marks`;
row 38 — no roles-specific test: the view reads no roles map, and rows 21, 23,
29 and 33's own pins already lock every body api-usage can print. The
bidirectional cross-driver leg is the shared scenario
`depgraph / modules_mark_folded_role_carriers`.
