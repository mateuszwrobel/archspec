---
name: archspec
description: Use when auditing a codebase's architecture — module boundaries, dependency direction, public API surface, spec gaps — with the archspec CLI. Also use when asked whether a module's public API leaks across layers.
---

# archspec — architecture audit skill

`archspec` extracts an architecture model from source (no build, no network),
compares it against an optional `architecture.spec.toml`, and prints dependency
graphs and per-symbol crossing data. This skill is the machine-relevant protocol
for auditing with it. Reinstall after upgrading the binary
(`archspec skill install --force`); re-print any time with `archspec skill`.

## Rule zero

Every extraction command (`scan`, `verify`, `report`, `depgraph`, `inspect`,
`update`) runs the syntax-tree extractor on Rust, C#, and Go trees: module
edges carry **which public names cross them**, and crossings at type positions
(base types, casts, generics, injected fields, attributes) are visible. Check
what a driver emits with `archspec capability matrix`.

## The model, in three tiers of trust

- **Units and unit edges** — ground truth (build files, manifests).
- **References and root exports** — ground truth (imports, `pub`/`public` decls).
- **Module edges and their `symbols`** — heuristic but *named*: extraction from
  declarations and import positions. Addressing: an edge endpoint is the deepest
  declared namespace path; tail segments appear as symbols on the edge.

**C# naming.** The dual-spelling sentence is stated once, in the csharp section of `archspec help languages`.

Read the raw model with `archspec scan .` (JSON on stdout) and
the graphs with `archspec depgraph modules|api-usage .`.

The same JSON carries a `roles` map — model path → role, drawn from the
closed vocabulary {`facade`, `composition`} — when any role is derivable.
Each driver derives from its own facts: rust marks a root file that defines
nothing (only `mod` declarations and re-exports) `facade` and a bin root
whose `main` wires modules `composition`; csharp marks a using-only umbrella
root `facade` and an entrypoint project running DI-registration-family calls
`composition`; go marks the `package main` module `composition` and derives
no facade role (derivability investigated, absence recorded in ADR-017). No
entry means "no role stated", never "role denied". Which views mark roles and
which stay silent by decision is stated by `archspec help roles` — behind it
sits a computed views × markers matrix, and the matrix outranks this
document; read the special nodes off the output instead of guessing.
`verify` consumes the map
the other way: the `facade dependency` rule fires on `facade` paths, and a
laundering-trace hop owned by a `composition` path is sanctioned wiring,
never fallback territory.

## Four finding classes that work

1. **Named crossings.** An edge `Api -> Persistence` with symbol
   `BudgetDbContext`-style names says *which* type crossed. Grep that name to
   get file:line evidence, then judge by the model's stated role: an edge out
   of a path the `roles` map names `composition` is sanctioned wiring; a
   business-layer file importing the same name is a leak. The edge alone (no
   symbols) cannot tell you these apart.
2. **Absence of facts.** A declared project/module dependency with *no* symbol
   or usage facts anywhere is a stale reference. A service registered but
   consumed by no edge is a dead seam. Before trusting an absence, confirm the
   driver records that fact for the language — `archspec capability matrix`.
3. **Type-position-only boundaries.** Base types, generic arguments,
   `new T()`, casts, attributes, injected constructor fields cross boundaries
   without appearing in any import list. A coupling no import list explains
   still shows up as an edge — read it with `archspec depgraph api-usage`.
4. **Spec-vs-model gaps.** `verify` failing with
   `disallowed cross-component dependency` on every edge usually means the
   spec's namespace-tier modules carry no `allowed.depend_on` (default deny),
   not that the code is broken. Seed from the real graph (`archspec update`),
   then tighten by hand; never "fix" such a wall by allowing everything.

## Report format

Every finding you report back follows `archspec help diagnostics`:
`finding` (the category), `evidence` (command + output excerpt), `resolution`
(`code-fix | spec-fix | architecture-rework`), `follow-up` (one command).
Do not invent categories; read the finding text and look it up there.

## Honest blind spots

- **Go facade rule inert.** The go driver derives no facade role — the
  alias-umbrella, public-package and root delegation idioms were investigated
  and found not derivable from go facts, a decision recorded in ADR-017 — so
  the `facade dependency` rule never fires there and a go facade root goes
  unchecked, not verified-clean; verify states this once per run as a
  role-scoped note. Composition is
  stated for go (the main package), so sanctioned go wiring stays readable
  by role, not by eye.
- **Selector blind spots.** Go dot-imports yield an edge with no symbols;
  capitalized selectors only on some paths. Cross-check with grep before
  calling an edge clean.
- **Transitive build references** (project/dep graphs resolved transitively)
  are not extraction edges; verify each declared reference separately.
- **Excluded trees** (tests, vendor) are out of the model by config; absence
  there is not absence in the project.
- Parser recovery errors do not surface as findings; a surprising tree should
  be sanity-checked with `archspec inspect`.

## Workflow in one line each

| Task | Command |
|---|---|
| First look, no spec | `archspec scan .` |
| Which nodes are special | `archspec scan .` → `roles` map; `archspec report` prints a Roles section; which views mark roles, and which stay silent by decision, is stated by `archspec help roles` — the computed matrix behind it outranks this table |
| Glob rules, before authoring any constraint | `archspec help glob` |
| Real dependency graph | `archspec depgraph modules .` |
| Who uses which public name | `archspec depgraph api-usage .` |
| Compare to declared architecture | `archspec verify .` |
| CI gate (warnings fail) | `archspec verify --strict .` |
| Seed a spec from the tree | `archspec update .` — writes `architecture.spec.toml` in the scanned directory; review and trim the seed (it mirrors the graph, not the intent) before enforcing |
| Driver/toolchain health | `archspec doctor` |
| Which binary am I holding | `archspec --version` |
| File-level import mining | `archspec inspect .` |
