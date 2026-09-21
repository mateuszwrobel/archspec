# `verify` — Output Contract

## Destination

- **Pass:** a short confirmation written to **stdout**, exit code `0`.
- **Fail:** the full diff report written to **stdout**, exit code non-zero.
- **Warning findings** print on **stdout**, as part of the diff report (each `warning:`-prefixed unless `--strict` promotes it) — a warning-only run lists them on stdout and exits `0`.
- **stderr** carries **operational errors only** (see `errors.md`); warnings never go there.

## Boundaries across both tiers

`verify` resolves the spec's declared boundaries (see `../../spec.md`) against whatever the tree has. A boundary matched by `matches.units` resolves against crates/packages; one matched by `matches.modules` resolves against module paths inside units; a boundary may span both. Edges are judged on the union of the two tiers, so the same spec is verified identically whether the tree is a single crate, a multi-crate workspace, or a hybrid — no shape selection. Report lines name resolved boundaries, and module-level items appear as dotted paths (e.g. `orchestration::common`).

Go trees carry a module tier in two derivations, and every module-granularity check runs on whichever exists. A `go.work` with two or more member modules is the native fact: the scan model carries the tier (member modules as groups, their packages as members) and `verify` enforces over it. A single-`go.mod` tree carries no tier of its own; when the spec declares modules that claim its packages, `verify` derives the tier for the comparison — membership from the declarations, edges from real cross-package imports, presented in the same import-path shape (`example.com/demo/auth`) the native tier normalizes to. Module-granularity rules therefore enforce over claimed packages only: a production package claimed by no module is loud through the component categories (`unexpected component` / `unassigned unit`) rather than silently skipped, and mapping it into a module clears it. The derivation is visible by construction: `scan` output stays tier-free for a single-`go.mod` tree (the derivation needs the spec), a native tier is never overridden, and the emitted-by notes below state which language/derivation each module-tier check can fire on.

## Pass output

A short confirmation naming the verified spec and what was checked. Representative example:

```
ok: architecture.spec.toml matches source model (4 modules, 3 constraints checked)
```

The exact wording is not a contract; that a short confirmation appears on stdout and the exit code is `0` is.

## Capability notes

After the confirmation or the report, `verify` may print short informational
lines stating which capability-table-listed rules can never fire for the
language (or, for a conditional fact such as the go module tier, ran this run
with no native scan fact to ride on). A rule whose own finding appears in the
run is demonstrably not inert — its note is suppressed and the finding is the
announcement. Notes are **output, not findings**: they never enter the
diff, never change the verdict or the exit status, and rust runs stay
byte-identical (rust emits every rule fact). Representative examples:

```
note: facade dependency rule inert for csharp: driver emits no root-facade fact
note: laundered forbidden edge rule: no native module tier this run (grouping derived from spec declarations)
```

The single source is the in-code capability table (`archspec::capability`);
prose here follows the table, never the other way around.

## Fail output: the full diff report

On failure, the report lists **every** difference across all checks — not the first hit (ADR-010). Each reported item is a distinct check result. Report categories:

| Category | Meaning | Example line |
|---|---|---|
| missing component | declared in the spec, absent from the source model | `missing component: Billing` |
| added component | present in the source model, not declared in the spec | `unexpected component: Portal` |
| unassigned unit | extracted unit matched by no declared component | `unassigned unit: Billing.Data.Orders` |
| ambiguous module match | a module path claimed by two declared boundaries with equal specificity (prefix-grouping overlap) | `ambiguous module match: app::commands::start claimed by both commands entry 'commands::*' and start entry 'commands::*' (equal specificity)` |
| forbidden edge | dependency present in the source but banned by the spec | `forbidden edge: Billing -> Portal` |
| missing edge | dependency declared by the spec, absent from the source | `missing edge: Billing -> Shared` |
| disallowed cross-component dep | dependency crossing a boundary the spec forbids | `disallowed cross-component dependency: Portal -> Billing.Data` |
| facade dependency | structural (no constraint declares it): an internal module depends on its own crate root whose file defines nothing — a publication-only facade of `mod` declarations and re-exports; declaring the root in `depend_on` does not legalize it, the fix is canonicalizing the import; emitted by rust only — the csharp/go drivers populate no root-facade fact, so the rule is inert there | `facade dependency: app::engine -> app` |
| contract leak | exposed surface bleeds outside the declared contract; on a submodule the contract is enforced over the surface crossing the submodule's boundary path (submodule form emitted by rust, csharp, and go — needs the module tier; go acquires one from go.work members or from spec modules claiming its packages); on go the surface is claimed membership over full import paths — the parent module must claim the submodule's packages (a module claiming none is a `missing component`) and a stereotype pattern must glob the whole import path (`example.com/demo/**/entity`), because the bare-segment concession addresses `::` paths only: a bare-name stereotype matches no go unit and the contract leaks nothing, verify green, no warning | `contract leak: Billing exposes entity (forbidden)` |
| cycle | `no_cycles` constraint violated over the declared groups | `cycle: Billing -> Portal -> Billing` |
| public api leak | public export not allowlisted by `public_api_allowlist` (named exports and names enumerated from resolvable root globs) | `public api leak: Billing exposes X (not allowlisted)` |
| unverifiable glob export | root glob re-export the scan cannot resolve from source (external crate, unknown path, cfg-gated declaration or chain link, poisoned chain), so `public_api_allowlist` cannot prove its set against `allowed` | `unverifiable glob export: Billing exposes core::types::*` |
| empty glob export | root glob re-export that resolves to a same-crate module exporting zero public items — a checked surface that would silently vanish | `empty glob export: Billing exposes empty_mod::* (resolves to no public items)` |
| forbidden external crate | a module imports a crate banned by `forbid_external_crates`; on csharp `forbid` patterns are compared case-sensitively while scan attribution is case-insensitive, so a pattern must match the referenced package id casing exactly | `forbidden external crate: Billing -> clap` |
| not external free | an `external_free` purity guard matched this module/unit and the model attributes external packages to it — the zero-dependency claim fired; a module-tier match monitors the subtree, a unit-naming pattern (or a tier-less go tree) every external attributed anywhere to the unit | `not external free: domain imports EF Core` |
| manifest integrity | a manifest (root or workspace member) fails `manifest_integrity` (publish/features/forbidden deps) | `manifest integrity: crates/billing/Cargo.toml has forbidden dependency clap` |
| feature boundary | module outside `allowed_from` depends on a gated module, or a gated module lacks its cfg; a gate naming no declared module is an unknown-pattern error on drivers emitting the root-module-declarations fact (rust) — where the driver emits no such fact at all (csharp, go) the constraint cannot engage and is reported once as a vacuous constraint citing the capability, never as per-pattern errors | `feature boundary: Portal depends on gated module X (feature: f)` |
| forbidden submodule dep | a `from` submodule depends on a `forbid` submodule within `parent`; needs module content below the unit (rust, csharp) — go packages are units, so no tree shape engages it there | `forbidden submodule dependency: orchestration::common -> orchestration::control_loop` |
| dead reference | an `allowed.depend_on`/`allowed.forbidden` boundary reference or `contract.forbid` stereotype reference that can never engage (names no declared top-level module / stereotype), classified by source existence where the driver's soft-visibility facts allow and by capability honesty where they do not, with a did-you-mean where feasible | `dead reference: module 'model' allowed.forbidden target "fold" does not exist — create it or fix the reference` |
| vacuous constraint | a constraint whose effective domain is empty (no module, edge, manifest, or export to check) | `vacuous constraint: [constraint #2] forbid_external_crates: 'from' pattern "app::persistnce" matches no module with external dependencies` |
| unresolved module file | a file-backed `mod x;` resolves to no parsed file (conventional variants, any `#[path]` target, and any `#[cfg_attr(.., path = "..")]` candidate missed), so its contents would be invisible to every check; emitted by rust only — mod-file resolution is a rust-driver fact | `unresolved module file: app::ghost` |
| unowned module edge endpoint | a soft module-edge endpoint claimed by no boundary (`matches.modules` and unit fallback both miss), so it cannot appear on the boundary graph; emitted by rust, csharp, and go when a module tier exists (go.work members, or spec modules deriving the tier over packages) — a tier-less tree has no soft edges to own | `unowned module edge endpoint: app::hidden` |
| laundered forbidden edge | a declared `allowed.forbidden` pair shows no direct boundary edge, yet some route from source to target rides at least one hop owned only via the unit fallback (catch-all `units` boundary, units-as-layer boundary, or modules no boundary claims); a ban bridged only through explicitly claimed boundaries is legal layering and stays silent; the trace sees only routes on the pair graph — an intermediate claimed by no boundary (neither `matches.modules` nor its unit's `matches.units`) contributes no pairs, so a ban routed through unclaimed territory surfaces here not at all but as `unexpected component` / `unowned module edge endpoint`, and laundering analysis engages only once the conduit's territory is claimed; emitted by rust, csharp, and go (hop ownership resolves through the module tier — go carries one natively from go.work members or acquires one when spec modules claim its packages) | `laundered forbidden edge: a -> b via shell` |

Every item carries the severity the spec assigns its constraint; a warning-level item is listed but does not fail the run unless `--strict` promotes it. The report always covers all failing checks, in canonical order.

## Vacuous constraints

A constraint that verifies nothing — e.g. `forbid_external_crates` whose `from` patterns match no module with external dependencies, a `no_cycles` over modules with no dependency edges, a `manifest_integrity` with no manifests to check, a `public_api_allowlist` with no crate-root exports, a `forbid_submodule_dependency` with no engaging intra-parent edge, an `external_free` whose `from` patterns match no module or unit present in the model (a matched pure element is the passing case and prints no diagnostic at all — only a pattern addressing nothing present is vacuous), or a `feature_boundary` whose root-module-declarations fact the driver's capability table marks not-emitted (the reason names the capability, not a source deficiency) — is reported as a warning-severity **vacuous constraint** instead of silently passing, so an empty check is never mistaken for a real one. Vacuous findings never count toward the pass confirmation, and a run whose *only* findings are vacuous never claims the source model matches; its header names the vacuous guard(s) instead:

```
architecture.spec.toml has vacuous constraints: [constraint #1] forbid_external_crates, [constraint #2] forbid_external_crates
  warning: vacuous constraint: [constraint #1] forbid_external_crates: 'from' pattern "app::ui" matches no module with external dependencies
```

Like every warning-level finding, `--strict` promotes a vacuous constraint to an error: the `warning: ` prefix is dropped and the run exits `1`. A genuinely clean run — where every constraint's domain is non-empty and no finding is produced — still prints the `ok: … matches source model` confirmation.

## Dead references

An `allowed.depend_on`/`allowed.forbidden` target is resolved by exact declared top-level module name, and a `contract.forbid` target by exact declared stereotype name. A reference that names neither is inert — the rule that carries it verifies nothing. `contract.forbid` targets are collected recursively from submodules too — submodule contracts are enforced, so their targets are checked and reported under the submodule's full path, never skipped. Such references are reported as warning-severity **dead reference** entries, classified by source existence where the driver's visibility facts allow:

- a name that exists in the source model (a unit or module path) but is declared nowhere: `exists in source but undeclared — references resolve to declared top-level module names only`;
- a name absent everywhere, on a driver whose module-tier fact is granular: `does not exist — create it or fix the reference`, with a `(did you mean "…"?` candidate when a declared name is within a small edit distance;
- a name whose existence the driver cannot judge — its capability table row for the module tier is not granular for this tree (a go tree whose scan carries no native tier: a `go.work` with two or more members, since a one-member work is the single-module path; grouping derived from spec declarations counts for the boundary checks but proves nothing about unclaimed source): `not verifiable from source — driver emits no module-tier fact for this tree`, never a claim of non-existence.

A reference naming a declared module or stereotype is engaged and never reported here. `constraint.modules` references are exempt: an undeclared name there is a schema error raised at load, not a warning. Like every warning-level finding, `--strict` promotes a dead reference to an error and the run exits `1`.

## Determinism

The same input always produces **byte-identical output**. Report items are canonically ordered, independent of filesystem traversal order and of edge ordering in the source. Output is safe to commit to version control and safe to diff in CI.

## Example

Representative fail output — canonical order, not exhaustive of the categories:

```
architecture.spec.toml does not match source model
  missing component: Billing
  unexpected component: Portal
  unassigned unit: Billing.Data.Orders
  forbidden edge: Billing -> Portal
  cycle: Billing -> Portal -> Billing
```

Pass and fail outputs are both deterministic: the same input, unchanged, always yields the same bytes.
