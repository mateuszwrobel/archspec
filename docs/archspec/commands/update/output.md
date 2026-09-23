# `update` — Output Contract

## Destination

- **File:** writes `architecture.spec.toml` in the scanned project directory (the directory given as `path`, or the current working directory when omitted).
- **Stdout:** a short summary of the run — the generated file path, and guidance to review/trim the seed spec and then run `archspec verify`.
- Exit code `0` on success.

The spec file is written in the declared spec format (see `../../spec.md`).

## What the file captures

`architecture.spec.toml` snapshots the model extracted from the source tree — what the code does today:

- the project language;
- one module declaration per detected boundary across **both** tiers the tree has: `matches.units` for crates/packages and `matches.modules` for module paths inside units (see `../../spec.md`);
- module boundaries are seeded **coarse**: one `matches.modules` boundary per TOP-LEVEL module of each unit (exactly one segment below the unit, e.g. `auth::config`, `auth::storage`). Descendant submodules fold into their top-level boundary via subtree matching — never a separate boundary per leaf. A test-gated module (declared `#[cfg(test)]` or an equivalent test-only `cfg(all(..., test, ...))`, resolved by ancestry so its subtree folds in too) is scaffolding and seeds **no** boundary; a module merely *named* `tests` without a test cfg is production and seeds one like any other top-level module. The test-gating fact comes from rust `cfg(test)` modules, go `*_test.go` files — go carries no serialized test-gating mark, so its test files simply feed no seeded dependency and mark no module — and C# projects whose resolved package set names a test runner (an xunit/nunit-family id or `Microsoft.NET.Test.Sdk`); the C# driver excludes those projects at the scan, so they seed no dependency, mark no module and no unit boundary (a runner-free project named `LegacyTests` seeds a boundary like any production unit);
- **Go seeds per derivation** (see `../../spec.md` § Go modularity): a `go.work` tree carries the module tier natively, so the seed declares **one module per workspace member** — `matches.units` listing the member's package import paths and `allowed.depend_on` from the real cross-member imports — instead of the unit + top-level-module fold, which would name dotted renderings no verify pattern can claim; a single-`go.mod` tree keeps the general per-package shape — its model tier derives from the packages' references when there are any, and on a tree recording none `verify` derives the grouping from the declarations;
- the dependency edges between those boundaries, declared as the allowed set, aggregated to the same coarse granularity: each boundary's `allowed.depend_on` lists the top-level target modules its subtree actually reaches (deduped, sorted).

Because boundaries are coarse (no nested boundaries to shadow one another), a generated seed adds no violations of its own and `verify` on an **acyclic** seed exits `0`. The seed additionally carries one global `no_cycles` guard (a `[[constraint]]` with `type = "no_cycles"` and no `modules` list, which already means "all declared modules"), so **seeded projects start cycle-clean**: a cyclic tree is reported as a cycle by `verify` instead of being blessed as allowed edges. Where the tree has no dependency edges among its declared boundaries, the guard trivially passes and `verify` flags it as a vacuous warning (exit `0`, no violations). The seed is a starting point, not a verdict: it is still intended to be reviewed, trimmed, and strengthened before it is enforced. It captures whatever structure exists, so the same command seeds a single-crate tree, a multi-crate workspace, and a hybrid tree without any shape selection.

## Structure

The file follows the spec format's sections in canonical order (see `../../spec.md`):

| Section | Content in the seed |
|---|---|
| `[project]` | the detected language |
| `[[module]]` | one declaration per detected boundary — matched via `units` and/or `modules`; module boundaries are seeded one per top-level module, descendants folded in (see `../../spec.md`) |
| inline `allowed` | the dependency edges realized from the current model at the same coarse granularity, rendered inside the `[[module]]` table as `allowed = { depend_on = [...] }` (no `[module.allowed]` section header is emitted) |
| `[[constraint]]` | one global `no_cycles` guard (no `modules` list = all declared modules), plus a `feature_boundary` constraint per gated top-level module |

The seed declares only the observed shape — no test-gated-module boundaries, no invented edges, no observed-shape constraints beyond `feature_boundary` — plus the one global `no_cycles` guard so a seeded project starts cycle-clean. Nothing the acyclic code does today is reported as a violation by the seed.

## Determinism

The same source tree always produces **byte-identical** `architecture.spec.toml`. Ordering is canonical, independent of filesystem traversal order. The output is safe to commit to version control: a second `update` against an unchanged tree produces the same bytes.

That guarantee holds **modulo the rewrite**: `update` replaces the destination file wholesale with the rendered seed, it does not edit it in place. Hand-written comments, custom headers, and any other content not derivable from the model do not survive a re-run — the next `update` overwrites them with the canonical seed rendering. Seed idempotence (unchanged tree ⇒ identical bytes) therefore applies to the seed's own output; a manually annotated spec is not a fixed point.

## Example

Source tree:

```
crates/auth/src/
├── lib.rs
├── config.rs
└── storage/
    ├── mod.rs
    ├── session.rs
    └── vault.rs
```

With extracted unit `auth`, top-level modules `auth::config` and `auth::storage` (its nested `storage::session`, `storage::vault` folded in), and the edge `auth::config -> auth::storage`, the generated spec is (representative):

```toml
[project]
language = "rust"

[[module]]
name = "auth"
matches = { units = ["auth"] }

[[module]]
name = "auth::config"
matches = { modules = ["auth::config"] }
allowed = { depend_on = ["auth::storage"] }

[[module]]
name = "auth::storage"
matches = { modules = ["auth::storage"] }

[[constraint]]
type = "no_cycles"
```

The file carries the observed coarse boundaries (unit `auth`, one boundary per top-level module), edges aggregated to them, and the global cycle guard. No test-gated-module or nested leaf boundaries are emitted. An acyclic seed verifies clean; the developer trims and strengthens it into the architecture they want to keep.
