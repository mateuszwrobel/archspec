use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub schema_version: u32,
    pub language: String,
    pub units: Vec<Unit>,
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub usage: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub soft_structure: BTreeMap<String, Vec<String>>,
    /// External dependencies (crates/projects/packages outside the project).
    /// Scan drivers may leave it empty; populating it is a scan-phase concern.
    #[serde(default)]
    pub external: Vec<String>,
    /// Soft module-level dependency edges *within* a unit, distinct from the
    /// hard unit `edges`. `from`/`to` are dotted module paths inside `unit`.
    #[serde(default)]
    pub module_edges: Vec<ModuleEdge>,
    /// The project's root manifest facts, extracted from the top-level
    /// `Cargo.toml` (publish state, dependency crates, declared features).
    /// Only rust populates it today; other drivers leave it absent.
    #[serde(default)]
    pub manifest: Option<ManifestInfo>,
    /// Unit crate name -> public items exported from the crate root
    /// (`lib.rs`/`main.rs`). Feeds `public_api_allowlist`.
    #[serde(default)]
    pub root_public_exports: BTreeMap<String, Vec<String>>,
    /// Unit crate name -> glob re-export tokens (e.g. `core::types::*`) at that
    /// unit's root that cannot be resolved from source (external crate, unknown
    /// path, cfg-gated declaration or chain link, poisoned chain). A glob's
    /// exported set cannot be proven against an `allowed` list, so it is
    /// reported as unverifiable. Globs that DO resolve contribute their
    /// enumerated names to `root_public_exports` instead.
    #[serde(default)]
    pub root_glob_exports: BTreeMap<String, Vec<String>>,
    /// Internal: unit crate name -> root glob re-export tokens that resolve to
    /// a same-crate module exporting zero public items. Kept off the serialized
    /// model (JSON shape unchanged) because only `verify`'s fail-closed check
    /// needs the empty/unresolvable split; `verify` reads the model straight
    /// from the scan it runs, so nothing else sees this field.
    #[serde(skip)]
    pub root_empty_glob_exports: BTreeMap<String, Vec<String>>,
    /// Dotted module path -> external crate names that module (or a `use`
    /// inside it) references. Feeds `forbid_external_crates`.
    #[serde(default)]
    pub module_external: BTreeMap<String, Vec<String>>,
    /// Unit crate name -> top-level module declarations found in the crate
    /// root, with their `cfg(feature = "...")` gating. Feeds
    /// `feature_boundary`.
    #[serde(default)]
    pub root_module_declarations: BTreeMap<String, Vec<ModuleDeclaration>>,
    /// Unit crate name -> dotted module paths declared as file-backed `mod`s
    /// that resolved to no known file variant (`name.rs`, `name/mod.rs`, nor a
    /// `#[path]` target). Kept off the serialized model (JSON shape unchanged),
    /// like `root_empty_glob_exports`: only `verify`'s fail-closed warning
    /// needs it, and `verify` reads the model straight from its own scan.
    #[serde(skip)]
    pub unresolved_module_files: BTreeMap<String, Vec<String>>,
    /// Unit crate name -> the manifest facts of the package that unit belongs to.
    /// Surfaced per-unit (instead of only the root `manifest`) so a workspace scan
    /// makes `manifest_integrity` meaningful: member publish/dependency/feature
    /// facts are no longer vacuous at a root without `[package]`. Populated for
    /// every unit, single-crate and workspace alike; the top-level `manifest`
    /// keeps the root facts for backward compatibility.
    #[serde(default)]
    pub unit_manifests: BTreeMap<String, ManifestInfo>,
    /// Internal: dotted module paths declared under a `#[cfg(test)]` (or an
    /// equivalent test-only `cfg(all(..., test, ...))`) gate. These modules —
    /// and everything beneath them — are architecture scaffolding and are
    /// excluded from boundary/cycle grouping, replacing the old rule that
    /// dropped every path merely *named* `tests` (which hid real production
    /// cycles when a `tests` module was not `cfg(test)` scaffolding). Kept off
    /// the serialized model (JSON shape unchanged), like
    /// `unresolved_module_files`: only `verify` and `update` need it and they
    /// read the model straight from their own scan, so a stale or hand-authored
    /// model never reopens a hidden cycle. Absent => treated as production
    /// (fail closed): a module is only excluded when proven test-gated.
    #[serde(skip)]
    pub test_gated_modules: BTreeSet<String>,
    /// Internal: unit crate names whose crate-root file DEFINES NOTHING —
    /// only `mod` declarations and `use`/`pub use` re-exports (a `#[cfg(test)]`
    /// definition does not count; tests are not production surface). Such a
    /// root is a publication-only facade, so `verify` rejects internal→root
    /// edges in that unit: routing through the umbrella launders any ban
    /// (`facade dependency` violations). Kept off the serialized model (JSON
    /// shape unchanged), like `unresolved_module_files`: only `verify` needs
    /// it and it reads the model straight from its own scan. A unit absent
    /// from the set is inactive — its root defines items, or the driver
    /// (csharp/go) does not implement the fact — so no existing model or
    /// language gains new findings from its absence.
    #[serde(skip)]
    pub facade_roots: BTreeSet<String>,
}

/// Root-manifest facts for `manifest_integrity`. `publish` is `Some(true)` when
/// the manifest allows publishing, `Some(false)` when it is disabled, and `None`
/// when the field is absent (resolved as not-published).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestInfo {
    pub publish: Option<bool>,
    pub dependencies: Vec<String>,
    pub features: Vec<String>,
}

/// A top-level `mod` declaration in a crate root. `gated` is true when the
/// declaration carries a `#[cfg(feature = "...")]` attribute, and `feature` is
/// the specific feature name gating it (if any). `file` is the module's source
/// file relative to `src/` (or `lib.rs`/`main.rs` for an inline module).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleDeclaration {
    pub name: String,
    pub gated: bool,
    #[serde(default)]
    pub feature: Option<String>,
    pub file: String,
}

/// A dependency edge a unit's sources declare at module level through their
/// usings/imports. `from` and `to` are `::`-separated module paths and
/// `unit` is the USING unit — normally the unit owning both modules, but
/// drivers also record edges whose `to` lives in a reference-reachable other
/// unit (C# `using` across projects, Go imports across `go.work` members);
/// the hard unit-level edge then comes from the manifest, this edge from the
/// code. `symbols` lists the path-imported items used along the edge
/// (e.g. `Token`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleEdge {
    pub unit: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub name: String,
    pub kind: String,
    pub path: String,
    /// Internal: the source root file of this unit relative to the unit's `src/`
    /// (`lib.rs`, `main.rs`, or `bin/<tool>.rs`). Used only during scan module
    /// analysis so each compilation target analyzes its own root. Not serialized.
    #[serde(skip)]
    pub root: String,
    /// Internal: every crate identifier this unit can be referenced by, including
    /// its primary name. A lib's `[lib] name` (which may differ from the package
    /// name) is added here so a bin referencing `voice_app_lib::run()` resolves to
    /// the lib's unit. Not serialized.
    #[serde(skip)]
    pub crate_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

impl Model {
    /// True when `module_path` is, or lives under, a module the scan proved
    /// test-gated (a `#[cfg(test)]` declaration or one of its descendants).
    /// Consulted by the boundary/cycle graph and the seed instead of a raw
    /// `::tests` name match. An empty set (an unscanned or older model) proves
    /// nothing, so every module is treated as production and participates.
    pub fn is_test_gated(&self, module_path: &str) -> bool {
        if self.test_gated_modules.is_empty() {
            return false;
        }
        if self.test_gated_modules.contains(module_path) {
            return true;
        }
        let mut current = module_path;
        while let Some((parent, _)) = current.rsplit_once("::") {
            if self.test_gated_modules.contains(parent) {
                return true;
            }
            current = parent;
        }
        false
    }

    /// True when the model carries any module-tier content: at least one soft
    /// module path or one module edge. Every `depgraph` view projects this tier
    /// and nothing language-specific beyond it, so a model without it has no
    /// module structure to render. The rust and csharp drivers always populate
    /// it (modules / namespaces); the go driver emits neither, so a Go tree
    /// reaches `depgraph` with the tier absent and must be refused by that
    /// structural fact — not by a hardcoded language check. A future Go tree
    /// that DOES emit module facts (workspace members / declared matches) gains
    /// a module tier here and renders automatically, with no guard change.
    pub fn has_module_tier(&self) -> bool {
        !self.module_edges.is_empty()
            || self.soft_structure.values().any(|paths| !paths.is_empty())
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|err| format!("failed to serialize model: {err}"))
    }

    pub fn from_json(json: &str) -> Result<Model, String> {
        serde_json::from_str(json).map_err(|err| format!("failed to deserialize model: {err}"))
    }
}
