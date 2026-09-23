# `architecture.spec.toml` — Spec Format Contract

The spec is the source of truth for verification (ADR-008). Declarative TOML, versioned schema, git-diffable, reviewable by humans and coding agents. It declares **boundaries** and **relations** — nothing about the "how" of implementation, and nothing about the physical shape of the tree (how many crates, whether they nest modules). The same spec works for a single crate, a multi-crate workspace, and any hybrid of the two.

Created by `archspec init` (minimal base spec) and `archspec update` (snapshot of the current model). Both scaffold one global `no_cycles` guard — `type = "no_cycles"` with no `modules` list — so new projects start cycle-clean; specs without the constraint keep verifying cycles as unmonitored. Consumed by `verify` and `diagram`. A JSON Schema (draft-07) covering the whole format is available via `archspec spec --schema`, and an annotated reference via `archspec spec`.

## Core idea: boundaries and relations, not shapes

The user's spec never says "this project is one crate" or "this project is a workspace". It declares:

- **boundaries** — named sets of matched things (units and/or modules),
- **relations** — the exact dependency sets between boundaries: the declared set is a ceiling (a real crossing outside it violates) and a floor (a declared edge absent from code violates), plus named constraint checks.

`verify` resolves the declared boundaries against the *actual* extracted model — whatever units and modules the tree has — and judges the relations on the resolved edges. Because resolution is against reality, the declaration format is identical whether the tree is one crate, several crates, or several internally-modular crates.

## Two levels of the model

The extracted model has two independent levels. The spec's `matches` keys address them explicitly:

| Match key | Resolves against | Meaning | Examples |
|---|---|---|---|
| `units` | extracted **units** (crates / packages / projects) | hard build-level boundaries | `"Billing"`, `"Billing.*"`, `"core"` |
| `modules` | extracted **module paths** *inside* units | soft source-level boundaries | `"Billing::domain"`, `"auth::ports"` |

A `matches.modules` pattern matches the named module path **and every module path beneath it** (its entire subtree of descendants). So `"commands"` covers `commands`, `commands::start`, `commands::init`, `commands::project_resolution`, and so on. A pattern may instead be the bare last segment of a module path (e.g. `"start"`), which resolves the same way against that segment's subtree. This subtree semantics is additive: a pattern always matches the module it names plus everything under it, never less.

**Prefix grouping.** An entry ending in `*` is a path-prefix claim: `"commands::*"` folds the whole `commands` subtree into one boundary with a single entry (`"commands*"` is a raw character prefix — it also claims `commands_old::*`). Ownership of a module path resolves by **specificity**, not declaration order: an entry naming the path exactly always wins; otherwise the entry pinning the longest prefix wins; a boundary reached through an ancestor claims the ancestor's subtree. When two boundaries claim the same module path with equal specificity, `verify` reports an `ambiguous module match` (naming the path and both entries) instead of guessing. Grouped view: soft module edges aggregate onto the owning boundaries (deduplicated, intra-boundary edges dropped), so a layered spec reads as `commands -> engines -> foundations`. Module paths claimed by no entry are never silently dropped from the edge analysis: the edge report lists each such endpoint as a warning-severity `unowned module edge endpoint` (`--strict` fails it) alongside the unit-level unexpected/unassigned reporting; `update` always emits concrete module names, never prefix patterns.

Edges are judged on the **union** of both levels:
- hard unit edges (crate depends on crate),
- soft module edges (`module_edges`: module depends on module, attributed to the using unit; the target module may live in another reference-reachable unit, in which case the manifest's unit edge carries the build fact).

A declared module boundary may match on `units`, on `modules`, or on both. The two keys are independent — you declare whichever tiers your architecture cares about.

**C# naming.** The dual-spelling sentence is stated once, in the csharp section of `archspec help languages`; ADR-018 records how allowances are owned across the tiers it names.

## Structure

```toml
[project]
language = "rust"            # rust | csharp | go

[[module]]
name = "domain"
matches = { units = ["core"], modules = ["Billing::domain", "Payments::domain"] }
contract = { forbid = ["entity"] }   # expose is reserved: declaring it today is a schema error

[module.allowed]
depend_on = ["domain", "ports"]
forbidden = ["infrastructure"]

[[constraint]]
type = "no_cycles"
modules = ["domain", "ports", "adapters"]
severity = "error"
```

### Constraint types

| `type` | Operates on | Keys |
|---|---|---|
| `no_cycles` | resolved module graph (unit and/or module edges) | `modules`, `severity` — no `modules` list means ALL declared modules; this global form is what `init`/`update` scaffold by default |
| `public_api_allowlist` | public exports of a unit's root (resolvable glob re-exports enumerated, unresolvable ones reported) | `allowed` |
| `forbid_external_crates` | module paths (dotted) → external packages (crate names on rust, NuGet package ids on C#, module paths on Go) | `from` + `forbid` |
| `manifest_integrity` | root + per-member manifest facts, each manifest named individually | `require_publish`, `forbidden_dependencies`, `required_features` |
| `feature_boundary` | cfg-gated module declarations | `feature`, `gated_modules`, `allowed_from` |
| `forbid_submodule_dependency` | module edges within a parent | `parent`, `from`, `forbid` |
| `external_free` | modules/units matching `from` must be attributed zero external packages — engaged by presence (a pure match passes for real), contamination fails naming module + packages, a pattern matching nothing present is vacuous; belongs on leaf layers, not composition roots; the monitored tier is the tier the driver serializes (module subtree where the model carries a module tier, the whole unit otherwise — `commands/verify/acceptance.md` rows 87–92) | `from` |

`severity` (default `error`; `warning` promoted by `--strict`) applies to all types.

On C# casing is asymmetric inside `forbid_external_crates`: scan attributes packages to usings case-insensitively (id `xunit` matches `using Xunit;`), while `forbid` patterns are compared case-sensitively against the attributed package ids, so a `forbid` entry must match the referenced package id casing exactly.

## Sections

| Section | Meaning |
|---|---|
| `[project]` | Language, project-level settings |
| `[[module]]` | A declared boundary: `matches` (one or both of `units`/`modules`), optional `contract` (`forbid` enforced on the module and — recursively — on its submodules, where it applies to the surface crossing the submodule's boundary path; `expose` reserved — declaring it is a schema error at every depth), `[module.allowed]` exact `depend_on` set (ceiling and floor — see Behavioral rules) and `forbidden` bans |
| `[[constraint]]` | Named dependency checks over the resolved module graph (see table above) |

## Behavioral rules

- **Shape-agnostic.** The spec never asserts tree shape. `matches.units` and `matches.modules` resolve against whatever the tree has; a single-crate, multi-crate, and hybrid tree all accept the same declaration format.
- **Boundaries are sets.** One module is one-or-more matched units and/or matched module paths. `verify` gathers all matches and resolves edges across them.
- **Complete for what it declares.** Verification compares the extracted model against the declared spec structurally; an extracted unit or module matched by no declared boundary is reported, not silently dropped.
- **Two keys are distinct.** `units` addresses crates/packages; `modules` addresses module paths inside units. The user's responsibility is to declare each boundary at the level(s) it actually spans.
- **`update` seeds coarse module boundaries.** To keep the seed round-trippable, `update` emits one `matches.modules` boundary per TOP-LEVEL module of each unit (one segment below the unit); descendant submodules fold into their top-level boundary via subtree matching, and a test-gated module (`#[cfg(test)]` or an equivalent test-only `cfg(all(..., test, ...))`, resolved by ancestry) is scaffolding that seeds none — a module merely *named* `tests` without a test cfg is production and seeds a boundary. The test-gating fact is provenance of the rust driver (`cfg(test)` modules, marked in the model), the go driver's file tier (`*_test.go` files, excluded at scan — go carries no serialized test-gating mark, so its test files seed no dependency but no module is flagged), and the C# driver's project tier (a project whose resolved package set names a test runner — an xunit/nunit-family id or `Microsoft.NET.Test.Sdk` — is excluded at the scan exactly like a go test file: whole projects are the C# test unit, so it seeds no boundary and flags no module; names and `IsPackable` play no part — a runner-free project named `LegacyTests` seeds a boundary like any production unit). An acyclic generated seed verifies clean against itself, and every seed carries the global `no_cycles` guard, so a cyclic tree fails `verify` instead of being seeded away.
- **Crate-root public exports are owned by the unit.** Public exports from a unit's root (`lib.rs`/`main.rs`) belong to the unit (crate) itself, not to a `matches.modules` boundary — even when the spec declares only module boundaries.
- **Glob re-exports are enumerated, never skipped.** A `pub use path::*;` at a unit root naming same-crate modules is resolved at scan time and its public items are checked against `allowed` like named exports (nested chains included, with cycle protection). A glob that cannot be resolved from source — an external crate, an unknown path, or a `cfg` on the glob declaration, the target module, or any chain link — is reported as unverifiable, and a glob resolving to an empty public surface fails closed; neither passes silently. `cfg` on a plain item *inside* a resolved module does not block enumeration: the checked surface is the union across configurations. The enumeration machinery is proven by the rust driver; the C#/Go drivers populate no root public-export facts, so there a `public_api_allowlist` constraint engages nothing and surfaces as a `vacuous constraint` warning — never a silent pass.
- **`manifest_integrity` checks each manifest.** The root manifest and every workspace member manifest are checked individually; a violation names the manifest that fails it, and a member missing a required feature is not masked by a sibling having it.
- **Units are compilation targets, not packages.** A package that builds multiple targets (a lib crate plus `main.rs` and/or `src/bin/*.rs` binaries) yields one unit per target. A bin's reference to its own lib is a unit edge, never an external.
- **Module files resolve like rustc.** A file-backed `mod x;` resolves to `<dir>/x.rs`, `<dir>/x/mod.rs`, its `#[path]` target, or the first existing target of its `#[cfg_attr(predicate, path = "...")]` attributes (the conventional variants remain candidates behind them, as rustc falls back to them when no predicate holds); the attributed file is analyzed as the module's source (its imports feed `forbid_external_crates`). A declaration that resolves to no parsed file is reported as warning-severity `unresolved module file` instead of silently skipped; `--strict` fails it.
- **Edges are all dependencies.** `scan` records edges from `use` imports AND qualified-path references (`audio::fn()`, `crate::x::run()`), so `module_edges` reflects every real dependency, not just import statements.
- **External is only true external crates.** `external` never contains the project's own modules, its own crate name, the standard library (`std`/`core`/`alloc`), or dev-dependencies. Crate names are normalized (dash↔underscore) so one crate appears once.
- **Feature gating names the feature.** A `feature_boundary` constraint cross-checks the declared feature against the module's actual `cfg(feature = "...")`; gated modules are matched as subtrees (submodules fold into the gated boundary).
- **Boundary references resolve by exact name.** `allowed.depend_on` and `allowed.forbidden` address declared top-level module names; `contract.forbid` addresses declared stereotype names. A reference that names neither (a typo, a removed module, a nested/submodule path) can never engage a rule, so `verify` reports it as a warning-severity `dead reference` — classified by source existence, with a did-you-mean candidate where feasible — instead of passing silently; `--strict` fails it.
- **`allowed.depend_on` is an exact set.** It serves two roles at once. Ceiling: a real boundary-crossing edge whose target is not listed violates as `disallowed cross-component dependency` (error). Floor: a declared target that is a **declared top-level module** (exact-name match) with no matching real edge violates as `missing edge` — error-level and unconditional, not `--strict`-gated (verify acceptance #15 in `commands/verify/acceptance.md`). The pair graph is the deduplicated cross-boundary edges of both tiers (unit and module) minus test-gated code, so an edge that exists only in test code does not satisfy the floor. A target that is not a declared top-level module engages no floor — it can match no edge — and is reported as a warning-severity `dead reference` instead (`--strict` fails it).
- **Submodule contracts are enforced.** A `contract.forbid` on a submodule is not decoration: it applies to the submodule's boundary path (`parent::...::submodule`) — module paths under that boundary and units claimed by the submodule's `matches.units` — with stereotype patterns matched by the same full-path/bare-name rule as other path references; a leak is reported under the submodule's full path (`contract leak: a::b exposes entity (forbidden)`). Dead-reference checking of `contract.forbid` targets recurses into submodules identically. Submodule enforcement needs the module tier: it is populated by the rust and C# drivers and, for Go, per the derivation rule of `archspec help languages`, taken natively from `go.work` members, or — on a tree whose packages record no references — supplied by spec modules claiming the tree's packages (see *Go modularity* below), so Go submodule contracts engage in each of these trees. On Go the surface a contract reads is **claimed membership over full import paths**: the parent module must claim the submodule's packages (a declared module that claims no package verifies as `missing component` and contributes no surface), and a stereotype pattern must glob the whole import path — the bare-segment concession below addresses `::`-separated module paths, which Go import paths never contain, so a bare `match = { names = ["entity"] }` matches no Go unit and the contract guards nothing — verify passes as if clean, with no leak and no warning — while `paths = ["example.com/demo/**/entity"]` fires. A `contract.expose` remains a schema error at every depth: it stays reserved and enforced by no check, top-level included.
- **Forbidden bans are transitive on the boundary graph.** A `forbidden` pair with no direct edge is a warning-severity `laundered forbidden edge` naming the chain when some route from source to target rides at least one hop whose ownership resolved through the **unit fallback** (a catch-all `units` boundary, a units boundary used as a layer, or modules no `matches.modules` entry claims): routing a ban through territory the spec never declares as a boundary violates the same declaration. Hop ownership resolves through the module tier, populated by the rust and C# drivers and, for Go, derived from the tree's own package references or taken natively from `go.work` members (see *Go modularity* below); a Go tree whose packages record no references has no hops to trace and laundering is not checked there. A hop owned by a boundary carrying a path the model's roles map marks `composition` is sanctioned wiring — the composition root gluing ports to bindings is never counted as fallback territory, whatever the hop's ownership route. A ban bridged only through explicitly claimed boundaries is legal layering (facade, composition root) and never reported; a coexisting purely declared route does not mask a conduit route. The trace runs over the boundary pair graph, so the intermediate's territory must be **claimed** — by some boundary's `matches.modules` or through its unit's `matches.units` fallback — for a bypass to be visible at all: an intermediate claimed by neither contributes no pairs (its edges surface as `unowned module edge endpoint` warnings, the unclaimed unit itself as `unexpected component`), and a ban routed through such territory is invisible to this check — mapping the conduit's territory into a boundary is the precondition for the bypass to surface. `--strict` fails laundering, direct violations stay error-severity.
- **A crate root that defines nothing is publication-only.** A rust crate root whose root file contains no item definitions — only `mod` declarations and `use`/`pub use` re-exports (`#[cfg(test)]` definitions do not count; tests are not production surface) — is a publication-only facade, activated automatically with nothing to declare. An internal module depending on that root module (e.g. importing an item through a root re-export instead of its canonical path) is an error-severity `facade dependency` violation: the umbrella is not a dependency target, and every internal→root edge is a conduit that launders any ban routed through the facade (see the laundering rule above; the two checks compose — the ban and the conduit are both reported). Listing the root in `allowed.depend_on` legalizes the pair for the boundary checks but never for this one; the fix is canonicalizing the import in code. Root→internal edges (the `mod` declarations and re-exports the facade exists to publish) stay legal, cross-crate unit edges (a bin linking its lib) are out of scope, and the rule is inert the moment the root file defines an item — nothing changes for roots that own real surface. The rule reads the model's `roles` map, not rust naming: the rust driver populates it from the root-definition fact above, and the C# driver marks a root module whose only facts are `using` directives (a using-only umbrella) with the same role, so the rule activates on both trees. The Go driver derives no facade role — no derivable go fact states it — so the rule stays inert on go trees, stated once per run as a role-scoped verify note, and a facade root there goes unchecked, not verified-clean.
- No stereotype/profiling vocabulary is built into the core, and there is no profile-resolution engine at runtime (ADR-013). Starter specs in `examples.md` are purely illustrative.

## Cross-language vocabulary

`units` vs `modules` is the universal vocabulary; each language's scanner maps its native concepts onto it:

| Language | `units` | `modules` |
|---|---|---|
| Rust | crates (`Cargo.toml`) | modules inside a crate (`mod`, `src/` tree) |
| C#/.NET | projects / packages (`.csproj`, nuget) | namespaces / services within a project |
| Go | the `go.mod` module path's packages — Go packages are **units** | derived by the driver per the `archspec help languages` rule: cross-package imports become module edges; in a `go.work` workspace the groups are its member modules (native fact); a tree recording no cross-package references records no module tier and the spec's `[[module]]` declarations supply the grouping `verify` compares over (declared derivation) |
| JS/TS | workspace packages (pnpm workspaces) | modules / aliases within a package |

The user writes the same `matches.units` / `matches.modules` keys for every language; the scanner supplies the concrete meaning.

### Go modularity

Go packages are units; the module tier above them is a derived fact — from the tree's own package references, natively from workspace membership, or from spec declarations where a tree records no references — and the layout rules below say how to use each.

- **Workspace members are modules (native).** A root `go.work` naming two or more member modules is a module fact the scan records: each member module becomes a module-tier group, the packages under its directory are its members, and every real import between packages of different members becomes a module edge. A package outside every member directory belongs to no module and contributes nothing (it is unbuildable from the workspace either way). A `go.work` with a single member is the single-module path below.
- **A flat tree derives the tier from its packages (derived).** In a single-`go.mod` tree, real imports between its packages become module edges and the referenced packages are module-tier-addressable at their import paths — the fact `archspec capability matrix` reports as `go module-tier granular`.
- **Otherwise every production package must be assigned to exactly one declared module.** On a tree whose packages record no references to each other (a single package being the limit) the model records no module tier; `verify` then derives the grouping for the comparison from the spec's `[[module]]` declarations that claim the packages (through `matches.units` names or `matches.modules` patterns over import paths). Every declared module must resolve to at least one package — one that claims none verifies as a `missing component` — and a production package claimed by no module surfaces through the component categories (`unexpected component` / `unassigned unit`), never silence; mapping it into a module clears the finding.
- **Native facts override declared ones.** When a `go.work` supplies the tier, declarations cannot reshape or override it; the scanned workspace is what `verify` enforces over.
- **Module-granularity rules enforce only over assigned packages.** Laundered-edge hop ownership, submodule contracts, and module-edge checks see the model's derived tier — membership from the driver's derivation or from the declarations, edges from real code — so they engage on claimed territory and nothing else.
- **Matching over go units is full-import-path globbing.** A go unit's name is the whole import path, and the bare-last-segment concession (see *Two levels of the model*) addresses `::`-separated module paths, which go import paths never contain — so stereotype and unit patterns must glob the full path (`example.com/demo/**/entity`), and a bare-name pattern (`entity`) matches nothing. Combined with the rule above this is where inert contracts come from: a bare-name stereotype, or a parent module whose declarations claim no package (reported `missing component`), leaves a `contract.forbid` guarding nothing while verify stays green.
- **Declared mode audits less than native mode, in one sentence:** declared membership is itself a declaration, so a package deliberately left out of every module escapes the module-tier checks a native `go.work` tier would place it under.
- **`update` seeds whatever derivation the tree offers**: a workspace seeds one module per member (`matches.units` listing the member's package import paths, `allowed.depend_on` from the real cross-member imports); a flat tree seeds the general per-package shape (its model tier rides on the package references regardless of declarations), and on a tree with no references `verify` derives the grouping from those declarations.

## Conventions

- File name: `architecture.spec.toml` (repo convention is TOML).
- Versioned schema — breaking changes bump the schema version.
