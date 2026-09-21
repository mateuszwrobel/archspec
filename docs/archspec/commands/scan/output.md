# `scan` — Output Contract

## Destination

- Default: canonical model written to **stdout**, exit code `0`.
- `--output <path>`: canonical model written to the file, **stdout stays empty**, exit code `0`.

## Artefact

`scan` emits the extracted architecture model — the canonical IR: **units, hard edges, usage, and soft structure** (per `../../../archspec-design.md` §4). It is:

- **deterministic text**, byte-identical for the same input;
- **JSON** on stdout by default;
- the **single canonical artefact** of the run — no diagram, no diff, no verification report.

The model is the shared artefact the spec pipeline compares against. It is directly consumable by `archspec update` (as a seed spec) and `archspec diagram --source scan` (as a render source) before any spec exists.

## Two tiers map to the spec vocabulary

The model's two levels correspond directly to the spec's `matches` keys (see `../../spec.md`):

- **`units` + `edges`** — the hard tier: `matches.units` resolves against these.
- **`soft_structure` + `module_edges` + `module_external`** — the soft tier: `matches.modules` resolves against these.

So `scan` exposes exactly what the spec needs to declare boundaries at either level, and a single-crate tree (one unit, many modules), a multi-crate workspace (many units), and a hybrid (units each with internal modules) are all visible — nothing is flattened away.

## Structure

The model always represents:

1. **Units** — the hard-boundary compilation/import units of the detected language (Rust crates, C# projects, Go packages).
2. **Hard edges** — dependency edges between units, from manifest/project/import ground truth.
3. **Usage** — which units reference which exposed surfaces of others.
4. **Soft structure** — namespace/folder groupings above and below the units (Rust `src/` module tree, C# namespaces/folders; Go: a `go.work` workspace groups each member module with its packages, while a single-`go.mod` tree carries no soft structure in the model).
5. **Module-level edges** — dependency edges *between modules*, attributed to the unit whose source declares the using (Rust `use crate::X` / `super::X`, C# `using` between namespaces, Go imports between packages of different `go.work` members). Recorded separately from hard unit edges: the unit edge says the build allows it, the module edge says the code uses it. An edge may cross the unit boundary when the target module lives in another reference-reachable unit (C# `using` of a namespace owned by a referenced project) — it then rides the manifest's unit edge without duplicating it. Each edge is attributed to its using unit, so internal-module boundaries inside every crate are resolvable.
6. **Module-level external** — dotted module path → external packages referenced by that module (feeds `forbid_external_crates`). Values are package identity in the driver's vocabulary: crate names on rust, module paths on go, NuGet package ids on C#. On C# a package is attributed when the project references it and one of its namespaces is visible in the module's `using` directives (longest-prefix match of namespace against referenced package ids — a documented approximation; namespaces alone never reach this tier). C# files that declare no namespace (top-level-statements composition roots like `Program.cs`) attribute their usings to the project's root module — the first two segments of the unit's namespace root, e.g. `HomeBudget.Api` → `HomeBudget::Api` — which then appears in the soft structure alongside the declared namespaces.
7. **External** — dependencies on crates/projects/packages outside the project (third-party crates, NuGet packages, external modules). Never includes workspace members or declared units.
8. **Root public exports** — per unit, the public items exported from that crate's root file (`lib.rs`/`main.rs`), including names enumerated from resolvable root glob re-exports (`pub use module::*`). Feeds `public_api_allowlist`.
9. **Root glob exports** — per unit, glob re-export tokens at the root that cannot be resolved from source (external crate, unknown path, cfg-gated or poisoned chain), so their exported set is unverifiable rather than silently empty.
10. **Root module declarations** — per unit, the top-level `mod` declarations found in the crate root, each with its source file and whether a `#[cfg(feature = "...")]` gates it (and which feature). Feeds `feature_boundary`.
11. **Manifest facts** — `manifest` carries the top-level `Cargo.toml` facts (publish state, dependency crates, declared features); `unit_manifests` carries the same facts per unit, so a workspace scan makes manifest checks meaningful for members, not only for a `[package]` root.

The exact field layout of the JSON is the versioned IR schema — the plugin contract between the core and the language drivers (see `../../../archspec-design.md` §7, ADR-012) — pinned with the schema itself, not in this behavioral contract.

## Determinism

The same input always produces **byte-identical output**. Unit, edge, and soft-structure ordering is canonical, independent of filesystem traversal order. Output is safe to commit to version control and to diff across runs.

## Example

Given a Rust workspace with crates `auth` and `billing`, where `billing` depends on `auth`, the output is JSON of the following shape (representative):

```json
{
  "schema_version": 1,
  "language": "rust",
  "units": [
    { "name": "auth", "kind": "crate", "path": "crates/auth" },
    { "name": "billing", "kind": "crate", "path": "crates/billing" }
  ],
  "edges": [
    { "from": "billing", "to": "auth" }
  ],
  "usage": {
    "billing": ["auth::storage::Session", "auth::config::Config"]
  },
  "soft_structure": {
    "auth": ["auth::storage", "auth::config", "auth::http"]
  },
  "manifest": {
    "publish": null,
    "dependencies": [],
    "features": []
  },
  "root_public_exports": {
    "auth": ["Config", "Session"]
  },
  "root_glob_exports": {
    "billing": ["serde::*"]
  },
  "root_module_declarations": {
    "auth": [
      { "name": "config", "gated": false, "feature": null, "file": "config.rs" },
      { "name": "storage", "gated": false, "feature": null, "file": "storage.rs" },
      { "name": "http", "gated": true, "feature": "http", "file": "http.rs" }
    ]
  },
  "unit_manifests": {
    "auth": { "publish": null, "dependencies": [], "features": [] },
    "billing": { "publish": null, "dependencies": ["auth"], "features": [] }
  }
}
```

The model carries the units and their hard edges, the usage of `auth` surfaces by `billing`, and the soft module structure of the `auth` crate. `manifest` carries the root `Cargo.toml` facts — empty here because a `[workspace]`-only root has no `[package]` — while `unit_manifests` gives the same facts per crate (so `billing` lists its `auth` dependency), and `root_public_exports` / `root_glob_exports` / `root_module_declarations` describe each crate's root surface (including `billing`'s one unverifiable glob token).
