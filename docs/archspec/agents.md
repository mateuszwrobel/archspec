# archspec — Agent Guide

This guide is for **LLM agents** and model-driven / automated users driving the `archspec` CLI. If you are an agent asked to author, tighten, or verify an `architecture.spec.toml`, this is your working protocol: the recipes, the exact commands, and the output shapes to expect. Everything below was captured from the real binary on a demo fixture, so the snippets are exact — do not "improve" the output text.

The spec is the source of truth for verification (see `spec.md`). Your job as an agent: extract the real model from source, declare boundaries that match what actually exists, and never invent structure the tree does not have.

## The canonical recipe

`archspec` follows a fixed workflow order — use it in this sequence:

| Step | Command | What it does |
|---|---|---|
| 1 | `archspec scan` | Extract the architecture model only (no spec needed). JSON on stdout. |
| 2 | `archspec report` | Text/markdown diff + metric output. |
| 3 | `archspec diagram` | Render the model (extracted or declared) to an artefact. |
| 4 | `archspec verify` | Extract + compare vs spec, full diff, exit code. |

Supporting commands: `archspec init` scaffolds a minimal base spec; `archspec update` snapshots the current model as a seed spec; `archspec doctor` diagnoses which language drivers/toolchains are present. The recipe's CI form is `archspec verify --strict` (all warnings promoted to errors).

## The built-in manual

Instead of guessing flags, read the manual embedded in the binary:

- `archspec help spec` — the `architecture.spec.toml` annotated reference
- `archspec help glob` — glob matching rules for units and module paths
- `archspec help constraints` — the constraint types, keys, and severity contract
- `archspec help languages` — the language-tier matrix (which model tiers each scanner populates)
- `archspec help workflow` — the `scan → report → diagram → verify` recipe and `--strict` gate
- `archspec help diagnostics` — the catalog of every finding category (see next section)

## Reading diagnostics

When `verify` fails, do not guess whether to touch code or the spec. Read the
finding category, then run **`archspec help diagnostics`** for the authoritative
entry per category: exact message pattern, meaning, origin, severity +
`--strict` behaviour, first follow-up command, and the decision rule. The quick
map below is a pointer, not the full catalog.

| Finding (category) | Usually means | Resolution |
|---|---|---|
| `forbidden edge` | code imports something its boundary bans | code-fix |
| `disallowed cross-component dependency` | real cross-boundary dep, undeclared | spec-fix (declare `depend_on`) or code-fix |
| `missing edge` | declared `depend_on` with no code edge | spec-fix (drop stale edge) |
| `facade dependency` | internal module imports through a root facade (f21); rust only — csharp/go emit no root-facade fact | code-fix (canonicalize import) |
| `contract leak` | a forbidden stereotype is exposed (submodule enforcement: rust, csharp, and go when the tree carries a module tier — go.work members or spec-declared grouping; on go the stereotype must glob the full import path — a bare name matches no go unit — and the parent must claim the packages, else the contract is inert and verify green) | code-fix or spec-fix |
| `cycle` / `no_cycles` | strongly connected components | code-fix or architecture-rework |
| `public api leak` | public export not in the allowlist | code-fix (hide) or spec-fix (allowlist) |
| `unverifiable glob export` / `empty glob export` | root glob cannot resolve / is empty | spec-fix |
| `forbidden external crate` | a `forbid` crate is imported | code-fix |
| `laundered forbidden edge` | a ban routed through territory some boundary claims (directly or via its unit); an intermediate claimed by no boundary leaves the pair graph — surfaces as `unexpected component` / `unowned module edge endpoint` with no laundered line; rust, csharp, and go with a module tier (go.work members or spec-declared grouping) | spec-fix (claim the conduit, then declare boundary) or code-fix |
| `dead reference` / dead contract target | a reference names no boundary/stereotype | spec-fix |
| `vacuous constraint` | constraint matches nothing, checks nothing | spec-fix |
| `unresolved module file` / `unowned module edge endpoint` | a `mod` has no file (rust) / edge owned by no boundary (rust, csharp, and go once a module tier exists) | code-fix / spec-fix |

Exit-code contract: any error-level finding exits `1`; warnings-only exits `0`
until `--strict` promotes them to errors (the CI gate). Every finding you report
back must follow the REPORT FORMAT in `archspec help diagnostics`: `finding`,
`evidence` (command + output excerpt), `resolution`
(`code-fix | spec-fix | architecture-rework`), and `follow-up`.

## Reference outputs (real, from the binary)

These were captured on a fixture crate. Keep these shapes in mind; every prompt below expects exactly these strings.

The fixture: `Cargo.toml` with name `demo`, dependency `serde = "1"`; `src/lib.rs` with `pub mod core; pub mod ui;`; `src/core.rs` with `use serde::Serialize; pub fn c() {}`; `src/ui.rs` with `use crate::core; pub fn u() {}`. The spec has `[project] language="rust"`, a `[[module]]` named `demo` matching units `["demo"]`, and a constraint `forbid_external_crates` from `["app::ui"]` forbid `["serde"]`.

`archspec scan` prints a single-line JSON model. Real top-level keys, in order:

```json
{"schema_version":1,"language":"rust","units":[{"name":"demo","kind":"crate","path":"."}],"edges":[],"usage":{},"soft_structure":{"demo":["demo::core","demo::ui"]},"external":["serde"],"module_edges":[{"unit":"demo","from":"demo::ui","to":"demo::core","symbols":[]}],"manifest":{"publish":null,"dependencies":["serde"],"features":[]},"root_public_exports":{"demo":["core","ui"]},"root_glob_exports":{},"module_external":{"demo::core":["serde"]},"root_module_declarations":{"demo":[{"name":"core","gated":false,"feature":null,"file":"core.rs"},{"name":"ui","gated":false,"feature":null,"file":"ui.rs"}]},"unit_manifests":{"demo":{"publish":null,"dependencies":["serde"],"features":[]}}}
```

A clean `archspec verify` pass (exit 0):

```
ok: architecture.spec.toml matches source model (1 modules, 0 constraints checked)
```

## Example prompts

### 1. Author the spec from `archspec scan` output

Use when no spec exists yet and you must write `architecture.spec.toml` from the real tree:

> Run `archspec scan` in the project root and read the JSON model from stdout. List the declared `[[module]]` boundaries you will write, one per real unit/tier that appears in the model's `units` array and `soft_structure` map. Match each boundary to actual units or module paths — never invent an edge, unit, or module that the scan JSON does not contain. Then author `architecture.spec.toml` with `[project]`, each `[[module]]`, and any `[[constraint]]`, and run `archspec verify` to confirm a clean pass.

Expected scan shape to build from (abridged `...` where values are truncated):

```json
{"schema_version":1,"language":"rust","units":[{"name":"demo","kind":"crate","path":"."}],"edges":[],"usage":{},"soft_structure":{"demo":["demo::core","demo::ui"]},"external":["serde"],"module_edges":[{"unit":"demo","from":"demo::ui","to":"demo::core","symbols":[]}],"manifest":{...},"root_public_exports":{"demo":["core","ui"]},"root_glob_exports":{},"module_external":{"demo::core":["serde"]},"root_module_declarations":{"demo":[{"name":"core","gated":false,"feature":null,"file":"core.rs"},{"name":"ui","gated":false,"feature":null,"file":"ui.rs"}]},"unit_manifests":{...}}
```

Declared boundaries must resolve against these keys: `units` and `edges` (hard tier), `soft_structure` + `module_edges` + `module_external` (soft tier). If `soft_structure` lists `demo` with `["demo::core","demo::ui"]`, then a `[[module]]` matching `modules = ["demo::core", "demo::ui"]` (or the subtree `demo` patterns) is grounded in the model; a boundary for a module that does not appear is not.

### 2. Tighten boundaries and gate CI

Use for hardening a spec and making `verify` your CI gate:

> Run `archspec verify --strict` as the CI gate. First run `archspec verify` (non-strict) to see spec-quality findings: warn about any `vacuous constraint` lines, because a vacuous constraint matches no module or edge and therefore checks nothing. Fix or delete vacuous constraints so every constraint is grounded in a real dependency. Then gate on `archspec verify --strict`, which promotes all `warning` findings to errors — relying on non-strict warnings lets vacuous constraints slip through in CI. Do not ship until `verify --strict` exits 0.

Spec-quality finding on a run that still exits 0 (non-strict):

```
architecture.spec.toml has vacuous constraints: [constraint #1] forbid_external_crates
  warning: vacuous constraint: [constraint #1] forbid_external_crates: 'from' pattern "app::ui" matches no module with external dependencies
```

Same fixture under `--strict`: the warning is promoted and the gate fails (exit 1):

```
architecture.spec.toml has vacuous constraints: [constraint #1] forbid_external_crates
  vacuous constraint: [constraint #1] forbid_external_crates: 'from' pattern "app::ui" matches no module with external dependencies
```

The `'from' pattern "app::ui" matches no module with external dependencies` tells you the boundary itself is misdeclared — `app::ui` is not a real module tier in this tree, so the constraint cannot fire against `serde`.

### 3. The iterate loop

Use whenever `verify` fails and you must reach a clean pass:

> Run `archspec verify`. Read the full diff: every `unexpected component`, missing component, or vacuous constraint is a distinct finding — fix them all in one pass. Do not "fix" the output text; change the spec (and only if warranted, the code) to match the model. Re-run `archspec verify` and confirm a clean pass before stopping. Accept only the exact pass line as success.

Real violation — an undeclared unit on the simplified spec (no `[[module]]` block for it):

```
architecture.spec.toml does not match source model
  unexpected component: demo
  warning: vacuous constraint: [constraint #1] forbid_external_crates: 'from' pattern "app::ui" matches no module with external dependencies
```

`unexpected component: demo` means the scanned tree contains a unit the spec declares no boundary for — `verify` reports it, never silently drops it. Fix by declaring a `[[module]]` whose `matches` resolve to `demo`. Success looks like:

```
ok: architecture.spec.toml matches source model (1 modules, 0 constraints checked)
```

Ever make the loop's goal explicit: the pass line is the definition of done. If `verify` keeps printing `architecture.spec.toml does not match source model`, you are diverging from the real model — rescan with `archspec scan` and re-ground your boundaries.

### 4. Go modularity adoption (playbook)

Advisory for agents only — archspec's runtime output never gives this advice; it enforces and reports. Compose the proposal yourself from `archspec scan` output plus this playbook, and present it to the human as a recommendation.

Use when `archspec scan` on a Go repo shows a flat single-`go.mod` tree: `units` are packages, `soft_structure` and `module_edges` are empty, so laundered-edge detection, submodule contracts, and module-granularity rules have nothing to enforce over.

**Step 1 — derive candidate boundaries from the package graph.** Run `archspec scan`; read `units` and `edges` (real imports between packages). Group the packages into cohesive clusters — dense internal imports, sparse edges between clusters, and layering direction (who imports whom) — and propose one boundary per cluster (typical shapes: api/service, store/persistence, shell/util conduits). Benefits: the boundary proposal is grounded in real edges, and `verify` gets a complete component map — every package lands in exactly one declared module, so nothing hides as `unexpected component`.

**Step 2 — pick a derivation.**
- *Split into `go.work` members* (preferred when the clusters can become modules): move each cluster to its own directory with its own `go.mod`, add a root `go.work` with a `use` entry per member. The scan model now carries the module tier natively (members are modules, their packages are members, cross-member imports are `module_edges`), `depgraph` renders the tree, and laundering checks run over filesystem facts no spec can reshape.
- *Declare modules over the existing packages* (when a split is not yet possible): one `[[module]]` per cluster with `matches.units` naming the cluster's package import paths. `verify` derives the tier from the declarations for the comparison; the model itself stays tier-free, so `depgraph` keeps refusing until step 2's workspace variant lands.
State the trade-off honestly in your proposal: declared membership is itself a declaration, so a package deliberately left out of every module escapes module-tier checks that native `go.work` membership would place it under.

**Step 3 — seed and map.** Run `archspec update` (a workspace seeds one module per member from the real cross-member imports; a declared tree seeds the per-package shape), then trim. Map every production package into exactly one module: a package claimed by none surfaces as `unexpected component` and must be mapped, not silenced.

**Step 4 — tighten step by step, gate with `--strict`.** Each step must keep `archspec verify` green before the next:
1. boundaries + `allowed.depend_on` exact for every real cross-module import → every cross-module dependency is declared; drift fails as `disallowed cross-component dependency` / `missing edge`;
2. `allowed.forbidden` bans between modules that must not couple → a ban routed through a conduit package now fails as `laundered forbidden edge` (warning; under `--strict` an error) — this power exists only because steps 1–3 created a tier; a conduit left outside every module contributes no pairs, so a ban routed through it stays invisible to the laundering check (the package surfaces as `unexpected component` — map it, then the bypass shows up);
3. submodule boundaries with `contract.forbid` stereotypes → exposed-surface leaks fail as `contract leak` under the submodule path; on go, stereotype patterns must glob the full import path (`**/entity`-style — a bare `entity` matches no go unit) and the parent module must claim the submodule's packages, or the contract guards nothing while verify stays green;
4. `archspec verify --strict` as the CI gate so all warnings are errors.