# `verify` — Acceptance Tests

CLI-level, behavioral only. Each scenario runs the real command against a fixture project and asserts on observable output (exit code, stdout/stderr, written files). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `verify` on a project whose code satisfies its `architecture.spec.toml` prints a short confirmation and exits `0`; on any divergence it prints a full diff report covering **every** difference and exits non-zero; `--strict` promotes warnings to errors; warning-level differences are listed but do not fail without `--strict`; a root glob re-export that resolves to same-crate modules is enumerated and checked name by name, and one that cannot resolve or resolves to nothing is reported, never silently ignored; manifest-integrity violations name the specific manifest that fails (a workspace member is never masked by a sibling); every operational error produces a clear message and a non-zero exit; and the same input always produces byte-identical output.

## Happy path

| # | Given | When | Then |
|---|---|---|---|
| 1 | project with `architecture.spec.toml` matching the code | run `verify` on the project root | exit 0; confirmation on stdout |
| 2 | project with spec; no `architecture.spec.toml` divergences | run `verify` twice, unchanged | both runs exit 0; byte-identical stdout |

## Shape-agnostic boundaries

The same spec declaration works regardless of tree shape (see `../../spec.md`). Boundaries resolve against whatever the tree has; a `matches.units` boundary matches crates/packages, a `matches.modules` boundary matches module paths inside units. A `matches.modules` pattern is a subtree match: it covers the named module path and every module path beneath it (all descendants), so a boundary declared at a directory/module level covers its submodules. This is additive — a pattern always matches the module it names plus everything under it.

| # | Given | When | Then |
|---|---|---|---|
| 3 | single-crate project; spec declares a boundary with `matches.modules` over internal modules; code satisfies it | run `verify` | exit 0; confirmation; the one unit is not reported as unexpected |
| 4 | multi-crate workspace; spec declares boundaries with `matches.units`; code satisfies them | run `verify` | exit 0; confirmation |
| 5 | hybrid project (units each with internal modules); spec declares unit boundaries and module boundaries together; code satisfies them | run `verify` | exit 0; confirmation; no spurious added/missing components |
| 6 | spec uses `matches.modules`; an extracted unit with no matching boundary is present | run `verify` | non-zero exit; report names the unexpected unit (a unit is still a boundary to account for) |
| 6a | single crate; spec declares `matches = { modules = ["commands"] }`; code has `commands::start`, `commands::init`, `commands::project_resolution` under it and satisfies the boundary | run `verify` | exit 0; confirmation; each descendant submodule is covered by the boundary, none reported as unexpected |
| 6b | spec declares `matches = { modules = ["commands"] }` with `forbidden = ["config"]`; code has a real `commands::start -> config` edge | run `verify` | non-zero exit; report names the forbidden edge from the descendant submodule |
| 6c | spec declares `matches = { modules = ["commands::start"] }`; code has only the leaf `commands::start` and satisfies the boundary | run `verify` | exit 0; confirmation; the leaf matches exactly, subtree semantics unchanged for a leaf pattern |

## Full-diff reporting

| # | Given | When | Then |
|---|---|---|---|
| 7 | spec declares components; code adds an extra component | run `verify` | non-zero exit; report names the added component |
| 8 | spec declares a component; code no longer contains it | run `verify` | non-zero exit; report names the missing component |
| 9 | spec forbids an edge; code contains it | run `verify` | non-zero exit; report names the forbidden edge |
| 10 | spec forbids cross-component dependency; code has it | run `verify` | non-zero exit; report names the disallowed dependency |
| 11 | spec declares a component; extracted unit matches no component | run `verify` | non-zero exit; report names the unassigned unit |
| 12 | component contract forbids a stereotype; component exposes it | run `verify` | non-zero exit; report names the contract leak |
| 12a | top-level module declares a submodule (`a::b`) whose `contract.forbid` names a stereotype; source exposes that stereotype under the submodule's boundary path (`a::b::c`) | run `verify` | non-zero exit; report names the contract leak under the submodule's **full path** (`contract leak: a::b exposes c (forbidden)`), not under the parent |
| 13 | spec has `no_cycles` over two modules; code has a cycle between them | run `verify` | non-zero exit; report names the cycle |
| 14 | project with several divergences (added, missing, forbidden edge, unassigned unit) | run `verify` | non-zero exit; report lists **all** divergences in one run |
| 15 | spec declares a required dependency edge; code lacks it | run `verify` | non-zero exit; report names the missing edge |

Scenario 15 is the floor side of `allowed.depend_on`'s exact-set semantics (floor and ceiling stated in [`../../spec.md`](../../spec.md)); it fires unconditionally, without `--strict`.

## `--strict`

| # | Given | When | Then |
|---|---|---|---|
| 16 | spec has `no_cycles` with `severity = "warning"`; code has the cycle | run `verify` | exit 0; report lists the cycle as a warning |
| 17 | same project as #16 | run `verify --strict` | non-zero exit; report lists the cycle as an error |
| 18 | project with only warning-level divergences | run `verify --strict` | non-zero exit; report lists each promoted warning |

## Determinism

| # | Given | When | Then |
|---|---|---|---|
| 19 | project with divergences, unchanged | run `verify` twice | both runs non-zero; byte-identical diff report |

## Constraint types

Additional constraint `type` values supported by the spec, in addition to `no_cycles` (behaviors 13, 16–17). Each is declared as a `[[constraint]]` with `type`, a required set of module/structure references, and optional `severity` (default `error`, `warning` promoted by `--strict` exactly as in 16–18).

### `public_api_allowlist`

| # | Given | When | Then |
|---|---|---|---|
| 20 | spec has `public_api_allowlist` with an `allowed` list; every module exported from the crate root is in it | run `verify` | exit 0; confirmation |
| 21 | spec has `public_api_allowlist`; a public item (module/fn) is exported from the crate root that is NOT in `allowed` | run `verify` | non-zero exit; report names the offending public export and the rule |
| 22 | spec has `public_api_allowlist`; the crate root's reserved infra (`lib.rs`/`main.rs` plumbing) is not allowlisted | run `verify` | exit 0; reserved root segments do not trigger a violation |
| 22a | single-crate project; spec declares only `matches.modules` boundaries plus `public_api_allowlist`; the crate root exports are allowlisted | run `verify` | exit 0; the crate-root exports are attributed to the unit root, NOT reported as leaks from a specific module boundary |
| 22b | single-crate project; spec declares only `matches.modules` boundaries; a crate-root export is NOT in `allowed` | run `verify` | non-zero exit; report names the offending export as a leak from the unit (crate) itself |
| 51 | spec has `public_api_allowlist`; the crate root contains an **unresolvable** glob re-export (`pub use ghost::*;` — no such module) | run `verify` | non-zero exit; report names the glob re-export and the rule — an unresolvable glob's exported set cannot be proven against `allowed`, so it is reported, never silently ignored; no names are attributed to it |
| 52 | spec has `public_api_allowlist`; the crate root contains named exports (all allowlisted) plus an unresolvable glob re-export (`pub use serde::*;`) | run `verify` | non-zero exit; the allowlisted named exports are not reported; the unresolvable glob alone is reported as unverifiable |
| 71 | spec has `public_api_allowlist`; the crate root contains `pub use types::*;` naming a same-crate module exporting public items, and every derived name is in `allowed` | run `verify` | exit 0; confirmation; no glob finding is reported for the resolved glob |
| 72 | same crate and glob; `allowed` lists one derived name but not the other | run `verify` | non-zero exit; the report names the missing name as a public api leak attributed to the unit, at the constraint's severity |
| 73 | crate root with allowlisted named exports plus one resolvable root glob | run `verify` | exit 0 when the glob's names are allowlisted too; each glob-derived name is checked individually, named exports stay silent |
| 74 | crate root with several root globs whose resolved sets overlap each other and a named export | run `verify` | each distinct name is checked once (duplicates collapse); an unchanged source reruns byte-identically |
| 75 | root glob whose chain cycles (`a` re-exports `b`, `b` re-exports `a`) | run `verify` | resolution terminates (visited-set cycle protection); the union of public items reachable through the chain is checked against `allowed` |
| 76 | root glob whose chain passes through a module re-exporting from an external crate | run `verify` | non-zero exit; the root glob is unverifiable — a chain is only as resolvable as its weakest link |
| 77 | `cfg` attribute on the glob re-export declaration, any predicate | run `verify` | non-zero exit; the glob is reported as unverifiable rather than a resolved set being guessed |
| 78 | `cfg` attribute on the target module's declaration | run `verify` | non-zero exit; unverifiable glob finding |
| 79 | `cfg` on a plain item inside a reachable target module (does not block), versus `cfg` on a `pub use`/`mod` declaration along the chain (blocks) | run `verify` | item-level cfg enumerates the name union and passes; a cfg-gated chain link is unverifiable |
| 80 | root spells the same glob three ways (`self::types::*`, `crate::types::*`, `app::types::*`) | run `verify` | all three normalize to the same local module path; one deduplicated resolved set is checked |
| 80a | crate-name prefix spelled with underscores while the package name uses dashes (`voice_app_lib::types::*` in `voice-app-lib`) | run `verify` | the prefix normalizes and the glob resolves |
| 81 | root glob whose first segment matches neither a root module nor a `self`/`crate`/crate-name prefix, but a declared external dependency | run `verify` | non-zero exit; unverifiable glob finding — external contents are never guessed |
| 86 | glob's first segment names BOTH a local root module and a declared dependency | run `verify` | the local module wins — precedence is `self`/`crate`/crate-name prefix, then root module, then dependency, then unknown |
| 82 | root glob resolving to a same-crate module exporting zero public items | run `verify` (with or without `--strict`) | non-zero exit; the report names an empty glob finding; an empty derived set never passes silently and is never called a vacuous constraint |
| 83 | root glob whose chain resolves only to modules exporting nothing public | run `verify` | non-zero exit; empty glob finding for the root glob |
| 84 | target module whose items carry item-level `cfg` | run `verify` | enumeration is not blocked; the names are the union across configurations |
| 85 | `pub use core::types::*;` through a nested same-crate module path (the facade shape) | run `verify` | the concrete items are enumerated and checked against `allowed` |

### `forbid_external_crates`

| # | Given | When | Then |
|---|---|---|---|
| 23 | spec has `forbid_external_crates` with `from` modules and `forbid` crate patterns; no listed module imports a forbidden crate | run `verify` | exit 0; confirmation |
| 24 | spec has `forbid_external_crates`; a listed module imports a crate matching `forbid` | run `verify` | non-zero exit; report names the module, the external crate, and the rule |
| 25 | spec has `forbid_external_crates`; an unlisted module imports a forbidden crate | run `verify` | exit 0; rule only applies to `from` modules |

### `manifest_integrity`

| # | Given | When | Then |
|---|---|---|---|
| 26 | spec has `manifest_integrity` with `require_publish` / `forbidden_dependencies` / `required_features`; manifest matches all | run `verify` | exit 0; confirmation |
| 27 | spec has `manifest_integrity` with `require_publish = true`; manifest has `publish = false` (or absent) | run `verify` | non-zero exit; report names the manifest and the expected/resolved publish state |
| 28 | spec has `manifest_integrity` with `forbidden_dependencies = ["clap"]`; manifest lists `clap` | run `verify` | non-zero exit; report names the manifest and the forbidden dependency |
| 29 | spec has `manifest_integrity` with `required_features = ["telemetry"]`; manifest lacks that feature | run `verify` | non-zero exit; report names the manifest and the missing feature |
| 53 | workspace with members; a member's manifest lists a dependency forbidden by `manifest_integrity` | run `verify` | non-zero exit; finding names the member's manifest (e.g. `crates/billing/Cargo.toml`), not the workspace root |
| 54 | workspace with members; a member's manifest is missing a `required_features` entry present in another member | run `verify` | non-zero exit; finding names the member manifest missing the feature (a sibling having it does not mask the violation) |
| 55 | workspace with members; every manifest (root + members) satisfies the `manifest_integrity` constraint | run `verify` | exit 0; confirmation |

### `feature_boundary`

| # | Given | When | Then |
|---|---|---|---|
| 30 | spec has `feature_boundary` with a `feature`, `gated_modules`, and optional `allowed_from`; only allowed modules depend on gated modules, and each gated module is declared under `cfg(feature = "...")` | run `verify` | exit 0; confirmation |
| 31 | spec has `feature_boundary`; a non-allowed module depends on a gated module | run `verify` | non-zero exit; report names the depending module, the gated module, and the feature |
| 32 | spec has `feature_boundary`; a gated module is declared without `cfg(feature = "...")` | run `verify` | non-zero exit; report names the module file and the missing cfg gate |
| 33 | spec has `feature_boundary` referencing a `gated_modules` pattern that matches no declared module | run `verify` | on a driver emitting the root-module-declarations fact (rust): non-zero exit, report says the pattern is unknown; on a driver whose table marks that fact not-emitted (csharp, go): exit 0 with one capability-reason vacuous constraint and no per-pattern errors |
| 47 | spec has `feature_boundary` with `feature = "tauri"`; the source module is actually gated under `#[cfg(feature = "uniffi")]` | run `verify` | non-zero exit; report names the mismatch between the declared feature and the module's actual gating feature |
| 48 | spec has `feature_boundary` whose `gated_modules` includes a parent module with submodules; a non-allowed module depends on one of the submodules | run `verify` | non-zero exit; report treats the submodule as part of the gated boundary (subtree match) |
| 49 | spec declares a unit boundary via `matches.units` and module boundaries via `matches.modules`; an internal edge's endpoint is the crate root | run `verify` | exit 0 when satisfied; the crate-root endpoint resolves to the unit boundary, edge counted (not silently dropped) |
| 50 | spec declares module boundaries; report `edges_internal` counts resolved-boundary edges consistently with the module edges actually extracted | run `verify` | exit 0; no edges are silently dropped from metrics when an endpoint is the crate root |

### `forbid_submodule_dependency`

| # | Given | When | Then |
|---|---|---|---|
| 34 | spec has `forbid_submodule_dependency` with `parent`, `from`, and `forbid` submodule lists; no listed submodule depends on a forbidden sibling submodule | run `verify` | exit 0; confirmation |
| 35 | spec has `forbid_submodule_dependency`; a `from` submodule depends on a `forbid` submodule within `parent` | run `verify` | non-zero exit; report names the parent, both submodules, and the rule |
| 36 | spec has `forbid_submodule_dependency`; a submodule NOT in `from` depends on a `forbid` submodule | run `verify` | exit 0; rule only applies to `from` submodules |

### `external_free`

| # | Given | When | Then |
|---|---|---|---|
| 87 | spec has `external_free` whose `from` matches a module present in the model with no attributed externals | run `verify` (with or without `--strict`) | exit 0; confirmation; the constraint counts as checked; NO vacuous warning — purity is the monitored state, not an empty domain |
| 88 | same spec; the matched module is attributed external packages | run `verify` | non-zero exit; finding `not external free:` names the matched module and the offending packages |
| 89 | spec has `external_free` whose `from` pattern matches no module or unit present in the model | run `verify` | exit 0; report lists the vacuous constraint naming the dead pattern ("matches no module or unit present in the model") — a diagnostic distinct from the pure pass of 87 |
| 90 | same project as 89 | run `verify --strict` | non-zero exit; vacuous line promoted (no `warning: ` prefix) |
| 91 | `external_free` with `severity = "warning"` and a contamination present | run `verify` / `verify --strict` | without `--strict`: exit 0, finding listed as `warning: not external free: …`; with `--strict`: non-zero exit, promoted |
| 92 | single-module go tree (no module tier); `from` names a package unit present in the model with no externals | run `verify` | exit 0; confirmation; engaged at the unit tier, no vacuous warning |
| 93 | module matched by both `external_free` and a `forbid_external_crates` rule; an external attributed to it | run `verify` | non-zero exit; both findings reported (`forbidden external crate:` and `not external free:`) without duplication of cause |
| 94 | `external_free` on a composition root that legitimately owns externals | run `verify` | non-zero exit; immediate `not external free:` failure — the guard is misplaced |

### Vacuous constraints

A constraint whose effective domain is empty (nothing to check) is reported as a warning-severity **vacuous constraint** rather than silently passing; a run whose only findings are vacuous never claims a match, and `--strict` promotes the finding to an error.

| # | Given | When | Then |
|---|---|---|---|
| 56 | `forbid_external_crates` whose `from` pattern matches no module with external dependencies | run `verify` | exit 0; report lists the vacuous constraint naming the `from` pattern; no "matches source model" |
| 57 | same project as 56 | run `verify --strict` | non-zero exit; report lists the vacuous constraint as an error (no `warning: ` prefix) |
| 58 | `forbid_external_crates` whose `from` pattern is misspelled (matches no module) | run `verify` | exit 0; report names the offending (unmatched) pattern |
| 59 | `no_cycles` over modules with no dependency edges among them | run `verify` | exit 0; report lists the vacuous constraint; no "matches source model" |
| 60 | `manifest_integrity` with no manifests to check | run `verify` | exit 0; report lists the vacuous constraint ("no manifests to check") |
| 61 | `public_api_allowlist` with no crate-root public exports | run `verify` | exit 0; report lists the vacuous constraint ("no crate-root public exports") |
| 62 | `forbid_submodule_dependency` with no engaging intra-parent submodule edge | run `verify` | exit 0; report lists the vacuous constraint |
| 63 | engaged `forbid_external_crates` whose `from` modules have external deps but none forbidden | run `verify` | exit 0; confirmation ("matches source model"); no vacuous warning |
| 95 | `external_free` whose `from` pattern matches no module or unit present in the model (typo, planned module) | run `verify` | exit 0; report lists the vacuous constraint naming the dead pattern; a matched pure element instead passes for real (row 87) |
| 64 | project with a real error AND a vacuous constraint | run `verify` | non-zero exit; report lists both; vacuous line keeps `warning: ` prefix |
| 65 | same project as 64 | run `verify --strict` | non-zero exit; report lists both; vacuous line promoted (no `warning: ` prefix) |
| 66 | engaging `manifest_integrity` with a manifest that matches | run `verify` | exit 0; confirmation ("matches source model"); no vacuous warning |
| 67 | project with two vacuous constraints, no other findings | run `verify` | exit 0; header names each vacuous guard, not "does not match source model" |
| 68 | project with a vacuous constraint | run `report` | exit 0; report lists the vacuous constraint as content |
| 69 | project with a vacuous constraint, unchanged | run `verify` twice | both runs exit 0; byte-identical stdout |
| 70 | `forbid_external_crates` whose `from` module HAS an external import (not forbidden) | run `verify` | exit 0; confirmation; never a false vacuous warning |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 37 | path does not exist | run `verify <missing>` | non-zero exit; message identifies the invalid path |
| 38 | path is a regular file | run `verify <file.toml>` | non-zero exit; message identifies the invalid path |
| 39 | directory with no `architecture.spec.toml` | run `verify <dir>` | non-zero exit; message names the expected file and where |
| 40 | spec file is not valid TOML | run `verify` | non-zero exit; message names the file and the TOML error |
| 41 | spec violates the schema (e.g. unknown constraint type) | run `verify` | non-zero exit; message names the file, the offending field, and why |
| 42 | directory with no supported-language sources | run `verify <dir>` | non-zero exit; message says no sources found and where |
| 43 | project containing a file with invalid syntax | run `verify` | non-zero exit; message names the failing file; no partial diff |
| 44 | project in a language whose driver/toolchain is missing | run `verify` | non-zero exit; message names the language and suggests `archspec doctor` |
| 45 | unknown flag given | run `verify --format mermaid` | non-zero exit; message names the unknown flag |
| 46 | two positional paths given | run `verify <a> <b>` | non-zero exit; message says at most one path accepted |
