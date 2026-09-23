# archspec `depgraph` — Design

Current-state dependency views of a project, straight from the extracted model. Part of the archspec design (see `../../../archspec-design.md`).

Where `inspect` maps files and `diagram` renders a spec, `depgraph` answers the three questions an auto-generated dependency document asks: which top-level modules depend on which, which public APIs each module actually uses, and how one module decomposes into its submodules. It projects the extracted model — no spec, no rules, nothing rendered but current state.

## Design decision: model-faithful, not old-scanner parity

This command exists to let a project generate its dependency doc from **archspec's model** instead of the older library graph API. Two consequences are intentional and are non-goals for byte-matching the previous `dependency-graph.md`:

- **Node set is archspec's own.** Modules come from archspec's module discovery (unit roots fold to `root`; every declared module and edge endpoint appears). Synthetic nodes the old graph builder invented are not reproduced.
- **Edges are import edges.** An edge means one module `use`s another module — module-tier edges exactly as the model records them, unit-tier edges as the cross product of the two endpoint units' top-nodes. Edges are **not** converted into "declared in the crate root" hub edges just to reproduce the old diagram's dense `module --> root` fan-out.

A regenerated doc therefore legitimately differs from an old one in node list and edge semantics. That divergence reflects a more accurate model, not a regression.

## Views

| View | Command | Output |
|---|---|---|
| top-level modules | `depgraph modules` | Mermaid (or PlantUML) graph |
| API usage by target | `depgraph api-usage` | Markdown table |
| one module's submodules | `depgraph submodules --parent <m>` | Mermaid (or PlantUML) graph |

The `modules` and `submodules` graphs mark the role carriers their projection names — the ` [facade]` / ` [composition]` label suffix or `<<facade>>` / `<<composition>>` stereotype, silent where a fold mixes roles (`output.md`, *Role markers*). `api-usage` carries no role column by decision: a role is not a usage fact.

## Quick usage

```bash
# top-level module graph of the current directory
archspec depgraph modules

# API-usage table to a file
archspec depgraph api-usage ./crates/auth --output docs/api-usage.md

# expand one parent module into its children
archspec depgraph submodules --parent orchestration
```

## Language support

Every view projects the model's **module tier** (soft module paths and module
edges) and nothing language-specific beyond it. rust and csharp always populate
it (modules / namespaces), so both render today. A Go tree renders exactly when
its scanned model carries the tier: a `go.work` workspace with several member
modules does (members are the modules, their packages the members), and a
single-`go.mod` tree does whenever its packages reference each other (the
grammar derives the tier from those references). Where such a tree's packages
all sit under the one module path, the modules view renders their **package
graph** rather than the single folded node that projection would otherwise be
(the trivial-fold rule in `output.md`). A tree whose model records no
package reference is refused on every view with this exact sentence:

> depgraph needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of

The guard keys on the model's module tier, not the detected language — so rust
and csharp runs are untouched, and a tier-carrying Go tree clears the guard and
renders automatically, with no change to `depgraph`. Note the asymmetry the
sentence hints at: `depgraph` scans only and never reads a spec, so for THIS
command only the native derivation (workspace members, or the tree's package
references) ever renders; the spec-declared derivation gives `verify` and
`update` their tier (the derivation needs the spec) but leaves `depgraph` on
the refusal path for trees that carry none.

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: views, flags, defaults, usage examples |
| `input.md` | Input contract: path and `--parent` semantics, accepted/rejected inputs |
| `output.md` | Output contract: destinations, projections, formats, table columns, determinism |
| `errors.md` | Failure contract: error conditions, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |
