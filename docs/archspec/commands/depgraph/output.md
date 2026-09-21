# `depgraph` — Output Contract

## Destination

- Default: body written to **stdout**, exit code `0`.
- `--output <path>`: body written to the file, **stdout stays empty**, exit code `0`.

## Formats

| View | `--format` values | Default |
|---|---|---|
| `modules` | `mermaid`, `plantuml` | `mermaid` |
| `submodules` | `mermaid`, `plantuml` | `mermaid` |
| `api-usage` | `markdown` | `markdown` |

## Projections

The model stores module edges at full dotted granularity (`unit::parent::child`). Each view collapses those paths onto one vocabulary:

### `modules` — top-level graph

- A dotted path projects to the segment right after the unit; a unit-root path folds to `root`.
- Every top-level module present in the soft tier becomes a node; every module edge contributes its two endpoints.
- An edge whose endpoints project to the **same** top-level module emits no edge (it collapses to a node). Edges that stay inside one top-level module never appear.
- Unit (dependency) edges are projected through module membership: each endpoint unit contributes the SET of top-level nodes its soft paths fold to, and the unit edge becomes the cross product of the two sets with self-edges suppressed. A unit whose soft paths all fold to one top-level node (a c# project per module) *is* that module, so unit edges between two such units appear as module edges; a unit folding to several nodes (a rust bin sharing its package's module tree, a crate with several top modules) still contributes its cross-unit edges, projected onto the top nodes of both ends. Units with no soft paths contribute no edges — one membership rule for every driver, no language branch: the modules view projects unit edges onto module membership for every driver, so a rust workspace whose crates each expose one top module shows those projected edges, and a bin+lib crate shows its `bin -> lib` dependency instead of two edgeless islands (a `src/bin/*.rs` binary that declares no `mod`s carries no soft paths, so it folds to an empty set and stays an island).

Output is a `graph TD` (Mermaid) or a PlantUML component list — nodes first, then edges, in canonical order.

### `submodules` — parent-scoped graph

- Only paths under `unit::parent` are kept; the parent's own module folds to `mod`, a deeper path folds to the child segment directly under the parent.
- A dependency leaving the parent's subtree is excluded.
- `parent` resolves by full module name (`unit::first-segment`, either separator) first, with the bare first segment as a compatibility fallback. A name matching no module is an error listing the module names the model knows.

### `api-usage` — Markdown table

Grouped by target top-level module, then by using top-level module. A pair contributes a row only when at least one symbol is recorded on its edges; edges within one top-level module contribute nothing.

| Target module | Used by module | APIs used |
| --- | --- | --- |
| `agent_auth` | `config` | `AuthConfig`, `CliAuthEntry` |

Fixed columns: `Target module | Used by module | APIs used`. Symbols are backticked, comma-separated, sorted; module and symbol cells are Markdown-escaped. When the grouping is empty — no edge between distinct top-level modules carries a symbol — the body is `No internal API usage details found.`, extended with a reason sentence when the emptiness is a fact of where usage is recorded rather than an absence of usage: for csharp the body is exactly `No internal API usage details found. Symbol-level facts are not emitted by the csharp driver.` (the c# driver records no symbols on module edges), and for rust a tree that has cross-unit links gets exactly `No internal API usage details found. Module edges record usage between modules of a single unit; cross-target links are recorded at unit granularity.` (rust module edges are intra-unit facts; `bin -> lib` and cross-crate usage lands on the unit tier — fact placement, not an absence of symbols). A tree with genuinely no cross-module usage keeps the bare sentence.

## Divergence from the previous generator

Because `depgraph` projects archspec's own model rather than the old library graph API:

- The node set can differ (archspec's module discovery may include unit-root `main`/`tests` nodes, or omit synthetic nodes the old builder invented).
- Edge semantics are pure import edges, so the dense `module --> root` hub fan-out of the old diagram is not reproduced.

This is a documented non-goal — see `README.md`. A regenerated dependency doc is expected to reflect archspec's model.

## Determinism

The same model always produces **byte-identical output**. Nodes, edges, table groups, and symbol lists are canonically ordered via ordered maps/sets. Output is safe to commit.
