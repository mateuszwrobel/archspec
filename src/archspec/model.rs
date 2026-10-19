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
    /// Dotted module path -> public type names (class/interface/struct/enum/
    /// record carrying an explicit `public` modifier, partial declarations
    /// deduped, nested types named `Outer.Nested`) declared by that module's
    /// production-tier sources. The C# public API surface:
    /// `public_api_allowlist` matches its `allowed` patterns against these
    /// module paths on drivers whose public API is module-attributed rather
    /// than crate-root (the rust root-export family above feeds the same
    /// constraint there, unchanged). Absent from JSON when empty — absence
    /// states "no such facts", never "no public API" (the `roles`
    /// convention) — so rust and go scan bytes stay untouched.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub module_public_types: BTreeMap<String, Vec<String>>,
    /// Dotted module path -> external crate names that module (or a `use`
    /// inside it) references. Feeds `forbid_external_crates`.
    #[serde(default)]
    pub module_external: BTreeMap<String, Vec<String>>,
    /// ts-only fact site (ts-driver design D2/R2-6): importing file ->
    /// verbatim spellings of the non-literal `import()` and computed
    /// `require()` specifiers it
    /// carries, the `dynamic import wildcard` facts. Report-only, never
    /// edges, never red. Absent from JSON when empty — absence states "no
    /// such facts", so the rust, go and csharp model bytes are untouched
    /// (the same optional-collection law as above).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dynamic_import_wildcards: BTreeMap<String, Vec<String>>,
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
    /// File-grained dependency facts: rows keyed by file identity, serialized
    /// by drivers whose `file-grained-dependency-facts` capability row reads
    /// granular in the field their file identity lives in. rust fills this
    /// field — its `module_edges` stay module-granular dotted paths; the ts
    /// driver keys its `module_edges` rows by file identity directly (ts-driver
    /// design D10) and leaves this field empty. Absent from JSON when empty
    /// (the `roles` convention), so the ts, go and csharp model bytes stay
    /// untouched. Consumers of the file tier read these rows through the
    /// facts-keyed adjacency accessor, never a language guess.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_edges: Vec<FileEdge>,
    /// Structural roles the driver derives from its own facts, keyed by model
    /// path with values from the closed vocabulary `Role` (`facade`,
    /// `composition`). A publication-only crate root carries `facade` at its
    /// unit path; a composition root (rust: a bin unit's wiring `main` root)
    /// carries `composition` at that module path. A path with no derivable
    /// role — or a driver that derives no role at all — carries no entry, and
    /// the key itself is absent from JSON when the map is empty: absence means
    /// "no role stated", never "role denied". `verify`'s `facade dependency`
    /// rule and the role-aware consumers read this map; it replaces the
    /// serde-skipped in-house `facade_roots` field.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub roles: BTreeMap<String, Role>,
    /// ts-only fact site (ts-resolution record R7b law): the rendered
    /// `external not installed` lines — packages a scanned tree declares in
    /// its nearest manifest but the disk does not carry. Report-only like
    /// `dynamic_import_wildcards`: never red, never part of the machine
    /// body — and never persisted: the field is transient like
    /// `unresolved_module_files`, skipped in both serde directions, so no
    /// serialized model ever carries it.
    #[serde(skip, default)]
    pub report_only_lines: Vec<String>,
}

/// The closed structural-role vocabulary of the model's `roles` map (ADR-017
/// territory: exactly two roles — richer taxonomies were rejected because no
/// rule consumes them). Serialization is lowercase (`facade`, `composition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// The addressed module is a publication-only facade: its root file
    /// defines nothing but declarations and re-exports, so internal→root
    /// edges through it launder bans (`verify`'s `facade dependency` rule).
    Facade,
    /// The addressed module is a composition root: its own file wires other
    /// modules together (a `main` root importing the modules it glues), the
    /// sanctioned place where cross-part wiring lives.
    Composition,
}

impl Role {
    /// The closed-vocabulary word for this role — the same lowercase text the
    /// serialized model uses, and the text the report's Roles section and the
    /// inspect markers print (US 06: a role nobody sees in output reproduces
    /// the "why is this node special" mystery). Presentation layers format
    /// the syntax around it (` [facade]` labels, `<<facade>>` stereotypes);
    /// the word itself has exactly one source.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Facade => "facade",
            Role::Composition => "composition",
        }
    }
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
    /// The ts-only edge-kind fact (ts-driver design D3/R3-9): a row keyed
    /// `(from, to, kind)` — an `import type` crossing serializes
    /// `type_only: true`, a value crossing omits the key, and a pair
    /// carrying both serializes two rows in deterministic key order.
    /// rust, go and csharp never set it, and the serde default with
    /// skip-when-false keeps their serialized bytes exactly as shipped.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub type_only: bool,
}

/// A file-grained dependency fact: one adjacency between two source files
/// of one unit, keyed by file identity (`from`/`to` are repo-relative file
/// paths, spelled like the diagram's file vertices). `kind` states HOW the
/// adjacency arises in rust's lexical vocabulary — a `mod` declaration, a
/// resolved `use` import, or a resolved qualified path reference. Rows
/// connect two distinct files; a reference a file makes to itself is not an
/// adjacency fact. rust serializes one row per distinct `(from, to, kind)`;
/// determinism is the same law as every other model collection (sorted).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEdge {
    pub unit: String,
    pub from: String,
    pub to: String,
    pub kind: FileFactKind,
}

/// The closed kind vocabulary of rust's file-grained facts. It states only
/// what the rust driver can see lexically; what rust does not carry stays
/// absent from the vocabulary rather than fabricated: there is no type-only
/// split (a rust `use` does not lexically distinguish type from value), no
/// dynamic import (no such construct), and no asset edge. A file adjacency
/// the scanner cannot derive — a module tree built by macros, an `include!`
/// body — serializes no row; the capability docs state that limit, and a
/// file-backed `mod` that resolves to no file is named by the
/// unresolved-declaration warning instead of silently dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFactKind {
    /// A file-backed `mod` declaration: the declaring file → the declared
    /// module's file. An inline `mod x { ... }` has no target file, so it
    /// contributes no row (its items live in the declaring file).
    Mod,
    /// A `use` import resolved to an item of another file's module: the
    /// importing file → the target module's defining file.
    Use,
    /// A qualified path reference (`other::func()`, `Type::method` paths)
    /// resolved to another file's module: the referring file → the target
    /// module's defining file.
    Path,
}

impl FileFactKind {
    /// The closed-vocabulary word for this kind — the same lowercase text the
    /// serialized model uses (the `Role::as_str` law).
    pub fn as_str(self) -> &'static str {
        match self {
            FileFactKind::Mod => "mod",
            FileFactKind::Use => "use",
            FileFactKind::Path => "path",
        }
    }
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
    /// The scaffold every scan driver builds its model on (self-audit F3):
    /// schema version 1, the scanned language, and no facts anywhere else —
    /// every collection empty, every optional absent, the transient channels
    /// empty. A new model fact lands empty HERE, at the one place, not at
    /// each driver literal; call sites state only the fields they actually
    /// populate (`..Model::new(language)` update syntax). The serialized
    /// bytes of a scaffold are the empty model of the language: `schema_version`
    /// is the version every serialized model carries (a derived `Default`
    /// would state version 0, which no model ever has — hence a constructor,
    /// not `Default`).
    pub fn new(language: &str) -> Self {
        Self {
            schema_version: 1,
            language: language.to_string(),
            units: Vec::new(),
            edges: Vec::new(),
            usage: BTreeMap::new(),
            soft_structure: BTreeMap::new(),
            external: Vec::new(),
            module_edges: Vec::new(),
            manifest: None,
            root_public_exports: BTreeMap::new(),
            root_glob_exports: BTreeMap::new(),
            root_empty_glob_exports: BTreeMap::new(),
            module_public_types: BTreeMap::new(),
            module_external: BTreeMap::new(),
            dynamic_import_wildcards: BTreeMap::new(),
            root_module_declarations: BTreeMap::new(),
            unresolved_module_files: BTreeMap::new(),
            unit_manifests: BTreeMap::new(),
            test_gated_modules: BTreeSet::new(),
            file_edges: Vec::new(),
            roles: BTreeMap::new(),
            report_only_lines: Vec::new(),
        }
    }

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

    /// The same facts indented for readers and diffs (`--pretty`, blind4
    /// D03): a different layout of one model, never a different model.
    pub fn to_json_pretty(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|err| format!("failed to serialize model: {err}"))
    }

    pub fn from_json(json: &str) -> Result<Model, String> {
        serde_json::from_str(json).map_err(|err| format!("failed to deserialize model: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_model() -> Model {
        Model::new("rust")
    }

    /// The roles map is a serialized model fact: model path -> closed-vocabulary
    /// role, serialized with lowercase values and restored exactly by a
    /// round-trip.
    #[test]
    fn roles_map_serde_round_trip_pins_the_serialized_shape() {
        let mut model = empty_model();
        model.roles.insert("app".to_string(), Role::Facade);
        model.roles.insert("app-bin::main".to_string(), Role::Composition);
        let json = model.to_json().expect("model serializes");
        assert!(
            json.contains("\"roles\":{\"app\":\"facade\",\"app-bin::main\":\"composition\"}"),
            "roles must serialize as the last model key with lowercase values:\n{json}"
        );
        let back = Model::from_json(&json).expect("model deserializes");
        assert_eq!(model, back);
    }

    /// Absence means "no role stated", never "role denied": an empty map emits
    /// no `roles` key at all (the pre-roles JSON shape is untouched), and JSON
    /// without the key loads as an empty map.
    #[test]
    fn empty_roles_emits_no_key_and_absent_key_loads_empty() {
        let json = empty_model().to_json().expect("model serializes");
        assert!(!json.contains("\"roles\""), "no role => no key:\n{json}");
        let back = Model::from_json(
            r#"{"schema_version":1,"language":"rust","units":[],"edges":[]}"#,
        )
        .expect("a roles-less model still loads");
        assert!(back.roles.is_empty());
    }

    /// The vocabulary words `as_str` hands to the report and inspect surfaces
    /// are exactly the serialized values — the prose can never drift from the
    /// model JSON it states (roles US 06).
    #[test]
    fn role_as_str_matches_the_serialized_vocabulary() {
        for (role, word) in [(Role::Facade, "facade"), (Role::Composition, "composition")] {
            assert_eq!(role.as_str(), word);
            let json = serde_json::to_string(&role).expect("role serializes");
            assert_eq!(json, format!("\"{word}\""));
        }
    }

    /// The file-grained rows are serialized model facts: `(from, to, kind)`
    /// with the lowercase kind vocabulary, restored exactly by a round-trip.
    #[test]
    fn file_edges_serde_round_trip_pins_the_serialized_shape() {
        let mut model = empty_model();
        model.file_edges.push(FileEdge {
            unit: "app".to_string(),
            from: "src/lib.rs".to_string(),
            to: "src/core.rs".to_string(),
            kind: FileFactKind::Mod,
        });
        model.file_edges.push(FileEdge {
            unit: "app".to_string(),
            from: "src/ui.rs".to_string(),
            to: "src/core.rs".to_string(),
            kind: FileFactKind::Use,
        });
        let json = model.to_json().expect("model serializes");
        assert!(
            json.contains(
                "\"file_edges\":[{\
                 \"unit\":\"app\",\"from\":\"src/lib.rs\",\"to\":\"src/core.rs\",\"kind\":\"mod\"},\
                 {\"unit\":\"app\",\"from\":\"src/ui.rs\",\"to\":\"src/core.rs\",\"kind\":\"use\"}]"
            ),
            "file edges must serialize unit/from/to/kind with lowercase kinds:\n{json}"
        );
        let back = Model::from_json(&json).expect("model deserializes");
        assert_eq!(model, back);
    }

    /// Absence means "the driver serialized no file facts", never an empty
    /// claim of them: an empty vector emits no `file_edges` key at all (the
    /// pre-facts JSON shape of every driver stays byte-identical), and JSON
    /// without the key loads as an empty vector (the additive-field law).
    #[test]
    fn empty_file_edges_emits_no_key_and_absent_key_loads_empty() {
        let json = empty_model().to_json().expect("model serializes");
        assert!(!json.contains("\"file_edges\""), "no facts => no key:\n{json}");
        let back = Model::from_json(
            r#"{"schema_version":1,"language":"rust","units":[],"edges":[]}"#,
        )
        .expect("a file_edges-less model still loads");
        assert!(back.file_edges.is_empty());
    }

    /// The kind vocabulary words `as_str` hands to prose are exactly the
    /// serialized values — the docs can never drift from the model JSON.
    #[test]
    fn file_fact_kind_as_str_matches_the_serialized_vocabulary() {
        for (kind, word) in [
            (FileFactKind::Mod, "mod"),
            (FileFactKind::Use, "use"),
            (FileFactKind::Path, "path"),
        ] {
            assert_eq!(kind.as_str(), word);
            let json = serde_json::to_string(&kind).expect("kind serializes");
            assert_eq!(json, format!("\"{word}\""));
        }
    }
}
