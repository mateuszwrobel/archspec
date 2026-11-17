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
    /// single-line spellings of the non-literal `import()` and computed
    /// `require()` specifiers it
    /// carries, the `dynamic import wildcard` facts. Report-only, never
    /// edges, never red. The load arity is one argument: the assert-style
    /// two-argument `require(cond, msg)` states no such fact (issue #16).
    /// Absent from JSON when empty — absence states "no
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
    /// Asynchronous (event) dependency rows of the `[[async_depends]]` feature
    /// (workplan archspec_async_depends, B1): one row per event-identity edge
    /// from a publishing unit to a subscribing unit. These rows NEVER enter
    /// `edges` — the edge comparison, the exempt ledger and the `update` writer
    /// adjudicate every `edges` row, and a pub/sub pair has no manifest basis,
    /// so riding `edges` would redden lawful pub/sub and inject phantom
    /// `allowed.depend_on` entries (B1). Consumers read the rows here, never
    /// through `edges`. Absent from JSON when empty — the `roles` convention —
    /// so every non-declaring repo's model bytes stay untouched (D8). Rows are
    /// keyed and ordered by `(from, to, event)` before any consumer reads them
    /// (N6); the collection's own rows are the only serialized carriers of the
    /// `kind` key (B2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub async_edges: Vec<AsyncEdge>,
    /// Driver-rendered report-only lines (ts-resolution record R7b law,
    /// GitHub issue #39 for the csharp producer): the rendered
    /// `external not installed` lines — packages a scanned tree declares in
    /// its nearest manifest but the disk does not carry — and the csharp
    /// `test-gated unit` census lines naming every project the scan dropped
    /// as test tier. Report-only like `dynamic_import_wildcards`: never red,
    /// never part of the machine body — and never persisted: the field is
    /// transient like `unresolved_module_files`, skipped in both serde
    /// directions, so no serialized model ever carries it.
    #[serde(skip, default)]
    pub report_only_lines: Vec<String>,
    /// ts-only fact site (GitHub issue 8 phase 2, decision-check BLOCKER-2):
    /// the warning-severity findings of `mode = "warn"` excluded-source
    /// hits, pre-rendered in the `<category>: <detail>` form the diff's
    /// warnings channel consumes. Never red without `--strict`, never part
    /// of the machine body — and never persisted: transient like
    /// `report_only_lines`, skipped in both serde directions, and NEVER
    /// routed through it: a report-only line can never go red, a warning
    /// must.
    #[serde(skip, default)]
    pub warnings: Vec<String>,
    /// The event payload-shape facts the derived drift census diffs
    /// (workplan archspec_async_depends, D2/M5, scenario 04): TWO derived
    /// facts — the publisher's record-declaration shape and the consumer's
    /// bound-field shape — never declared anywhere (payload SHAPE is
    /// derived from source, a declared shape would rot silently). Kept off
    /// the serialized model (JSON shape unchanged), like
    /// `test_gated_modules`: only `verify`'s drift arm reads it and
    /// `verify` reads the model straight from its own scan, so a stale or
    /// hand-authored model never opens a drift warning (fail closed, the
    /// same law). A driver without the shape derivations leaves every map
    /// empty and the census stays silent there (M5).
    #[serde(skip, default)]
    pub event_shapes: EventShapes,
}

/// Container of the two event-shape derivations the drift census diffs
/// (workplan archspec_async_depends, D2/M5). Determinism follows the
/// shipped collection law: BTree ordering everywhere, no source order
/// reaches a consumer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventShapes {
    /// Dotted module path -> record type name -> its members sorted by
    /// member name: the PUBLISHER-side derivation, read from parsed record
    /// declarations (positional parameters and field/property declarations
    /// of the record body). A type that is not a record carries no entry —
    /// its payload shape derives NOTHING, and the census stays silent on
    /// such a side (M5 silence law).
    pub records: BTreeMap<String, BTreeMap<String, Vec<EventRecordMember>>>,
    /// Dotted module path -> record type name -> the simple names the
    /// declaration lists after `:` (its base list): the base chain the drift
    /// census follows when it resolves the publisher's members (scenario 07,
    /// the base-blind cure). Non-serialized with the rest of EventShapes, so
    /// a non-declaring repo's `scan.json` bytes stay unchanged (D8). A
    /// record with no base list carries no entry.
    pub bases: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// Dotted module path -> the type names that module declares as enums.
    /// The projection class reads this census to tell an enum-typed
    /// publisher member from any other member.
    pub enums: BTreeMap<String, BTreeSet<String>>,
    /// Unit name -> event type name (the declaration's simple name) ->
    /// the fields that unit's code binds on variables of that type, and
    /// which of those bind as a string projection: the CONSUMER-side
    /// derivation (M5, the second fact). A handler binds a field by naming
    /// it on a variable of the record type (`order.Total`,
    /// `order.Status.ToString()`, `order.Status == "…"`); a dynamic,
    /// `JsonDocument` or map binding binds no variable of the event type
    /// and derives NOTHING — the census stays silent there, never a guess.
    pub bounds: BTreeMap<String, BTreeMap<String, EventBoundShape>>,
}

/// One member of a derived record shape: the member name and its declared
/// type as written (the drift projection class resolves the type name's
/// tail against the enum census).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventRecordMember {
    pub name: String,
    pub field_type: String,
}

/// What one unit binds on variables of one event type: the field names the
/// code references directly, and the set bound as a string projection
/// (`.ToString()` on the field, a comparison against a string literal, or
/// a `.ToString()`/literal comparison one member down a chain — scenario
/// 07). `projected ⊆ fields ∪ nested`: every direct projection is a bound
/// field, while a field projected HOP-FURTHER (`Data.Status.ToString()`)
/// joins `projected` alone — the drift census reads it as a projection
/// candidate and never as a direct reference the record must declare.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventBoundShape {
    pub fields: BTreeSet<String>,
    pub projected: BTreeSet<String>,
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

/// The closed edge-kind vocabulary of the async feature (workplan
/// archspec_async_depends, D5/B2): the `direct|async` label. `kind` is NOT a
/// serialized field of `Edge` — a direct edge serializes as `{from, to}`
/// exactly as shipped, and an absent `kind` means `direct` (the
/// `ModuleEdge::type_only` skip-when-false and `roles` absence precedent). The
/// label rides the renderers and the async collection's own rows only:
/// `AsyncEdge` below is the single serialized carrier of a `kind` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    /// Absent kind means direct: the default a row without a `kind` key
    /// states, so the shipped `edges` bytes (no key at all) are exactly the
    /// `direct` statement (B2 — a serialized `kind` on every edge would move
    /// every committed `scan.json`).
    #[default]
    Direct,
    /// The row is an asynchronous (event) dependency — the state only a row
    /// of the `async_edges` collection carries.
    Async,
}

impl EdgeKind {
    /// The closed-vocabulary word for this kind — the same lowercase text the
    /// serialized model uses (the `Role::as_str` law). Renderers print the
    /// label through this one source, never a local string.
    pub fn as_str(self) -> &'static str {
        match self {
            EdgeKind::Direct => "direct",
            EdgeKind::Async => "async",
        }
    }
}

/// One asynchronous (event) dependency row of the `async_edges` collection
/// (workplan archspec_async_depends, B1): the event-identity edge from the
/// publishing unit `from` to the subscribing unit `to` over the event type
/// `event` (the fully-qualified dotted name — identity only, payload shape
/// derives from source, never declared here). Rows never reach `edges`, the
/// edge comparison, the exempt ledger or the `update` writer (B1); they ride
/// the async renderer and the async census alone. Determinism follows the
/// shipped law for model collections (the `FileEdge` sort law above): rows
/// order by `(from, to, event)` — see `Model::sort_async_edges` (N6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsyncEdge {
    pub from: String,
    pub to: String,
    pub event: String,
    /// The kind label — the ONLY serialized `kind` key in the model (B2).
    /// Rows of this collection state `async`; a direct edge carries no key.
    pub kind: EdgeKind,
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
            async_edges: Vec::new(),
            report_only_lines: Vec::new(),
            warnings: Vec::new(),
            event_shapes: EventShapes::default(),
        }
    }

    /// Order the async collection by `(from, to, event)` — the determinism
    /// law of N6: derived async rows and rendered async edges sort before any
    /// renderer reads them (the `FileEdge` sort law, same shape). Producers
    /// build rows in whatever order their facts arrive; every consumer calls
    /// this once after building the collection, so a run's async bytes depend
    /// on the facts, never on the scan order.
    pub fn sort_async_edges(&mut self) {
        self.async_edges.sort_by(|a, b| {
            (&a.from, &a.to, &a.event).cmp(&(&b.from, &b.to, &b.event))
        });
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

/// One consumed exclusion entry: the trimmed root plus the resolved mode.
/// The spec loader (the binary's `spec` module) constructs entries; the
/// scan drivers adjudicate per hit through [`matching_exclusion`].
#[derive(Debug, Clone)]
pub struct ExclusionEntry {
    pub path: std::path::PathBuf,
    pub mode: &'static str,
}

/// The specificity of one entry's match on a repo-relative path (decision-
/// check MAJOR-1): `Some((exact, components))` when the entry matches —
/// the entry path equal to the file path is the `exact` match, the entry
/// path a component-wise ancestor of the file path is the prefix match
/// (`Path::starts_with`, so the entry `web` matches `web` and `web/app.ts`,
/// never `webapp/app.ts`). Ordering follows the tuple: an exact match beats
/// any prefix match, and among prefix matches the entry with the most
/// components — the most specific prefix — wins. Declaration order
/// contributes nothing.
#[must_use]
pub fn exclusion_specificity(entry: &std::path::Path, rel: &std::path::Path) -> Option<(bool, usize)> {
    if entry == rel {
        Some((true, entry.components().count()))
    } else if rel.starts_with(entry) {
        Some((false, entry.components().count()))
    } else {
        None
    }
}

/// The entry a path hits, resolved by specificity over all entries (never
/// the first in spec order). A tie — two entries of equal specificity on
/// one path — cannot exist for distinct entries: any two different prefixes
/// of the same path differ in component count, and equal paths are refused
/// at load by the duplicate-entry arm of the spec validator. The string
/// tie-break keeps the choice deterministic even for callers that skipped
/// validation.
#[must_use]
pub fn matching_exclusion<'a>(
    entries: &'a [ExclusionEntry],
    rel: &std::path::Path,
) -> Option<&'a ExclusionEntry> {
    entries
        .iter()
        .filter_map(|entry| {
            exclusion_specificity(&entry.path, rel).map(|score| (score, entry.path.as_path(), entry))
        })
        .max_by(|(left_score, left_path, _), (right_score, right_path, _)| {
            left_score
                .cmp(right_score)
                .then_with(|| left_path.cmp(right_path))
        })
        .map(|(_, _, entry)| entry)
}

/// Glob match over a unit/component name or path. Supports `*` (any run of
/// characters, including path separators) and `**` (same as `*` here). Mirrors
/// the spec.md glob conventions (`*Entity`, `**/domain/**`).
/// The single source of the shipped pattern semantics: the verify comparators
/// and the scan drivers match through this one function (the verify-side
/// `pattern` module re-exports it, so no second matcher can drift).
#[must_use]
pub fn matches_pattern(value: &str, pattern: &str) -> bool {
    glob_match(value.as_bytes(), pattern.as_bytes())
}

fn glob_match(value: &[u8], pattern: &[u8]) -> bool {
    let mut v = 0;
    let mut p = 0;
    let mut star: Option<usize> = None;
    let mut star_v = 0;

    while v < value.len() {
        if p < pattern.len() && (pattern[p] == b'*') {
            star = Some(p);
            star_v = v;
            p += 1;
        } else if p < pattern.len() && pattern[p] == value[v] {
            p += 1;
            v += 1;
        } else if let Some(star) = star {
            star_v += 1;
            v = star_v;
            p = star + 1;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

/// The declared facts a scan extraction may consume: the root-spec entries the
/// drivers adjudicate while walking (exclusions, kotlin zone claims, the
/// ts framework pin). The facts library never reads `architecture.spec.toml`
/// itself — loading stays with the consumer that owns the spec schema (the
/// binary's CLI glue). An empty `DeclaredFacts` is the spec-free fresh walk:
/// mechanical exclusions only, byte-identical to a root without a spec.
#[derive(Debug, Clone, Default)]
pub struct DeclaredFacts {
    /// The `excluded_sources` entries (trimmed root + resolved mode).
    pub exclusions: Vec<ExclusionEntry>,
    /// The kotlin zone-claim advisory input: one entry per top-level
    /// declared module, its name and its match patterns.
    pub zones: Vec<DeclaredZone>,
    /// The declared framework pin the ts resolution law names in
    /// unresolvable findings.
    pub framework: Option<String>,
}

/// One declared module's zone claim over the tree (name + match patterns).
#[derive(Debug, Clone)]
pub struct DeclaredZone {
    pub name: String,
    pub units: Vec<String>,
    pub paths: Vec<String>,
    pub modules: Vec<String>,
}

impl DeclaredFacts {
    /// The declared exclusion roots, in entry order (the same list the
    /// spec's `exclusion_paths` answers).
    #[must_use]
    pub fn exclusion_paths(&self) -> Vec<std::path::PathBuf> {
        self.exclusions.iter().map(|entry| entry.path.clone()).collect()
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
