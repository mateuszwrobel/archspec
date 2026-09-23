# `depgraph` — Output Contract

## Destination

- Default: body written to **stdout**, exit code `0`.
- `--output <path>`: body written to the file, **stdout stays empty**, exit code `0`. `--output -` prints the body on stdout and creates no file.

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
- **Trivial fold**: when that projection yields exactly ONE node and the module tier holds at least two distinct paths below it, the view renders **one level deeper** — each path below the fold becomes a node, labelled by its own final segment, and the edges reproject at that level. Keyed on the shape of the projection, not on a language: a single-`go.mod` tree whose packages address themselves below host/organisation/repository segments shows its **package graph** instead of one folded label, and a nested-parent tree of any driver shows its children. A one-node projection with nothing distinct below it keeps its single node; a projection with more than one node is untouched by this rule.

Output is a `graph TD` (Mermaid) or a PlantUML component list — nodes first, then edges, in canonical order.

**Role markers.** Each rendered node carries at most one role marker: every key of the model's `roles` map is projected through this view's own fold — the same separator lift and the same granularity decision that project the edges — and the node is marked iff the roles landing on it are exactly one distinct role. The marker is the roles label syntax of `inspect tree`: a ` [facade]` / ` [composition]` suffix inside the quoted label in Mermaid, a `<<facade>>` / `<<composition>>` stereotype on the declaration in PlantUML, node ids and edge lines otherwise byte-identical. A node onto which two different roles fold carries no marker at all — the fold is too coarse to state one role, and silence beats an aggregate claim — and a key whose projection names a node the view does not render (a unit-root path folding to `root`) contributes nothing: the rule never invents a node to carry a role. A model with an empty `roles` map renders the pre-marker graph byte for byte.

### `submodules` — parent-scoped graph

- Only paths under `unit::parent` are kept; the parent's own module folds to `mod`, a deeper path folds to the child segment directly under the parent.
- A dependency leaving the parent's subtree is excluded.
- `parent` resolves by full module name (`unit::first-segment`, either separator) first, with the bare first segment as a compatibility fallback. A name matching no module is an error listing the module names the model knows.
- Role markers follow the same projection rule: each roles key projects through the parent-scoped fold and a child node marks only when the roles folding onto it are exactly one distinct role — keys below or outside the parent's subtree contribute nothing.

### `api-usage` — Markdown table

Grouped by target module, then by using module, at the granularity the `modules` graph renders: top-level modules, or the paths below the fold when the trivial-fold rule reads one level deeper (the same rule, one decision — the table names the nodes the graph shows). A pair contributes a row only when at least one symbol is recorded on its edges; edges that project to one node contribute nothing.

| Target module | Used by module | APIs used |
| --- | --- | --- |
| `agent_auth` | `config` | `AuthConfig`, `CliAuthEntry` |

Fixed columns: `Target module | Used by module | APIs used`. Symbols are backticked, comma-separated, sorted; module and symbol cells are Markdown-escaped. When the grouping is empty the body is `No internal API usage details found.`, extended with a reason sentence when the emptiness is a fact of where the model records usage rather than an absence of usage; the routing is `api_usage_empty_reason`'s arm order — csharp arm, rust unit-tier arm, the structural symbol-carrying arm, the bare statement — first match wins, and no model with symbol facts on its module edges prints the bare statement (the csharp arm preceding the symbol check makes the csharp sentence a decision rule of that order, not a guarantee about one tree's symbol facts; the state matrix with its pin per state is the api-usage section of this command's `acceptance.md`). For csharp the body is exactly `No internal API usage details found. No symbol facts were emitted for this tree.` (symbol facts are emitted wherever the tree carries cross-boundary references, so the sentence names this tree, not the driver); for rust a tree that has cross-unit links gets exactly `No internal API usage details found. Module edges record usage between modules of a single unit; cross-target links are recorded at unit granularity.` (rust module edges are intra-unit facts; `bin -> lib` and cross-crate usage lands on the unit tier — fact placement, not an absence of symbols); for any other model that does carry symbols on module edges while the grouping still came out empty — every symbol-carrying edge lands on one rendered node, one level too deep for the fold rule to help — the body is exactly `No internal API usage details found. Module edges carry symbols, but every symbol-carrying edge stays inside one rendered module; the usage sits between modules below the granularity this view renders.` The bare sentence stands only for a model with no symbol facts on its module edges at all — genuinely no recorded cross-module usage — whatever the language.

Roles are not usage facts, so this view marks nothing: the three fixed columns and the rows are unchanged by roles and the emptiness statements are untouched (acceptance row 38 records the same decision) — the roles answer for module-level questions is the `modules` graph, the report's Roles section and the `roles` map in the `scan` model.

## Divergence from the previous generator

Because `depgraph` projects archspec's own model rather than the old library graph API:

- The node set can differ (archspec's module discovery may include unit-root `main`/`tests` nodes, or omit synthetic nodes the old builder invented).
- Edge semantics are pure import edges, so the dense `module --> root` hub fan-out of the old diagram is not reproduced.

This is a documented non-goal — see `README.md`. A regenerated dependency doc is expected to reflect archspec's model.

## Determinism

The same model always produces **byte-identical output**. Nodes, edges, table groups, and symbol lists are canonically ordered via ordered maps/sets. Output is safe to commit.
