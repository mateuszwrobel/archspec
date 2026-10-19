use crate::archspec::model::{
    Edge, FileEdge, FileFactKind, ManifestInfo, Model, ModuleDeclaration, ModuleEdge, Role, Unit,
};
use rust_arch_test_kit::collector::{
    cfg_feature_name, flatten_use_tree, glob_token, has_cfg, is_cfg_test, is_public,
    is_reserved_segment, path_segments, pub_use_glob_paths, pub_use_names,
};
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::Visit;
use std::path::{Path, PathBuf};
use toml::Value;
use walkdir::WalkDir;

/// Per-module references gathered during the first pass: the module path, its
/// `use` trees, the dotted paths referenced in its item bodies, the trait- and
/// type-position references collected by `TraitRefVisitor` (impl-header trait
/// paths, `dyn`-bound paths, type paths nested in generic arguments — resolved
/// by their own law, kept SEPARATE from `paths` so that set's single-segment
/// law and stdlib drops stay byte-stable), and the file.
type ModuleRefs = (
    Vec<String>,
    Vec<syn::UseTree>,
    BTreeSet<Vec<String>>,
    BTreeSet<Vec<String>>,
    PathBuf,
);

pub fn extract(root: &Path) -> Result<Model, String> {
    let root_manifest_path = root.join("Cargo.toml");
    if !root_manifest_path.exists() {
        return Err(format!(
            "no Cargo.toml found under: {}",
            display_relative(root, &root_manifest_path)
        ));
    }
    let root_manifest = read_manifest(&root_manifest_path)?;
    // The workspace root's `[workspace.dependencies]` table (when present), used
    // to resolve member deps declared via `{ workspace = true }` to their real
    // crate name. A single-crate root (no `[workspace]`) has none.
    let workspace_deps = root_manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"));

    let mut units = units_for(&root_manifest, root, &root_manifest_path)?;
    units.sort_by(|left, right| left.name.cmp(&right.name));

    let normalized_units: BTreeMap<String, String> = units
        .iter()
        .flat_map(|unit| {
            unit.crate_ids
                .iter()
                .map(move |id| (normalize_crate_name(id), unit.name.clone()))
        })
        .collect();

    // Runtime `[dependencies]` crate names (normalized, real names resolved) per
    // unit. Gates the fully-qualified-path external detection so only real
    // declared dependencies are recorded, never locals or own modules.
    let manifest_deps: BTreeMap<String, BTreeSet<String>> = units
        .iter()
        .map(|unit| {
            let manifest = read_manifest(&manifest_for_unit(root, &unit.path))?;
            let mut deps = BTreeSet::new();
            if let Some(dependencies) = manifest.get("dependencies").and_then(Value::as_table) {
                for (name, value) in dependencies {
                    let real = dependency_real_name(name, value, workspace_deps);
                    deps.insert(normalize_crate_name(&real));
                }
            }
            Ok((unit.name.clone(), deps))
        })
        .collect::<Result<_, String>>()?;

    // Per-unit manifest facts, surfaced for every unit so `manifest_integrity`
    // can assert publish/dependency/feature facts at a workspace root where the
    // root manifest itself has no `[package]`.
    let unit_manifests: BTreeMap<String, ManifestInfo> = units
        .iter()
        .map(|unit| {
            let manifest = read_manifest(&manifest_for_unit(root, &unit.path))?;
            let info = manifest_info(&manifest, workspace_deps);
            Ok((unit.name.clone(), info))
        })
        .collect::<Result<_, String>>()?;

    let mut soft_structure: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut module_edges: Vec<ModuleEdge> = Vec::new();
    let mut external_from_uses: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut root_public_exports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut root_glob_exports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut root_empty_glob_exports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut root_module_declarations: BTreeMap<String, Vec<ModuleDeclaration>> = BTreeMap::new();
    let mut unresolved_files: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut test_gated_modules: BTreeSet<String> = BTreeSet::new();
    let mut file_edges: Vec<FileEdge> = Vec::new();
    let mut roles: BTreeMap<String, Role> = BTreeMap::new();
    let mut edge_set: BTreeSet<(String, String)> = BTreeSet::new();

    for unit in &units {
        parse_unit_sources(root, unit)?;
        let info = analyze_unit_modules(
            root,
            unit,
            &normalized_units,
            manifest_deps.get(&unit.name),
        )?;
        if !info.soft.is_empty() {
            soft_structure.insert(unit.name.clone(), info.soft);
        }
        // Roles are derived from this driver's own facts and ride the model as
        // serialized entries (model path -> closed-vocabulary role).
        // `facade`: the unit's root file defines nothing but declarations and
        // re-exports — the same predicate that activated the in-house
        // `facade_roots` set; keyed at the root module path, which for rust is
        // the unit name. `composition`: a bin unit whose main root wires —
        // at least one module edge sourced from `<unit>::main`, the shape the
        // verify facade check exempts as root content. A unit whose root file
        // cannot be located proves nothing (see `UnitModuleInfo`) and gains no
        // role: absence states "no role", it never denies one.
        if !info.root_defines_items {
            roles.insert(unit.name.clone(), Role::Facade);
        }
        if unit.root == "main.rs" {
            let main_path = format!("{}::main", unit.name);
            if info.edges.iter().any(|edge| edge.from == main_path) {
                roles.insert(main_path, Role::Composition);
            }
        }
        module_edges.extend(info.edges);
        file_edges.extend(info.file_edges);
        external_from_uses.extend(info.external);
        for (module, crates) in info.module_external {
            module_external.insert(module, crates);
        }
        if !info.root_public_exports.is_empty() {
            root_public_exports.insert(unit.name.clone(), info.root_public_exports);
        }
        if !info.root_glob_exports.is_empty() {
            root_glob_exports.insert(unit.name.clone(), info.root_glob_exports);
        }
        if !info.root_empty_glob_exports.is_empty() {
            root_empty_glob_exports.insert(unit.name.clone(), info.root_empty_glob_exports);
        }
        if !info.root_module_declarations.is_empty() {
            root_module_declarations.insert(unit.name.clone(), info.root_module_declarations);
        }
        if !info.unresolved.is_empty() {
            unresolved_files.insert(unit.name.clone(), info.unresolved);
        }
        test_gated_modules.extend(info.test_gated);
        for (from, to) in info.unit_edges {
            edge_set.insert((from, to));
        }
    }

    let mut external_from_manifest: BTreeSet<String> = BTreeSet::new();
    for unit in &units {
        let manifest_path = manifest_for_unit(root, &unit.path);
        let manifest = read_manifest(&manifest_path)?;
        for section in ["dependencies"] {
            if let Some(dependencies) = manifest.get(section).and_then(Value::as_table) {
                for (name, value) in dependencies {
                    // A renamed dependency uses `package = "real-name"`; the key is the
                    // local alias. Match on the real crate name. A `{ workspace = true }`
                    // dep resolves its real name from the workspace root.
                    let real = dependency_real_name(name, value, workspace_deps);
                    let normalized = normalize_crate_name(&real);
                    if let Some(real_unit) = normalized_units.get(&normalized) {
                        if *real_unit != unit.name {
                            edge_set.insert((unit.name.clone(), real_unit.clone()));
                        }
                    } else {
                        external_from_manifest.insert(normalized);
                    }
                }
            }
        }
    }
    let edges: Vec<Edge> = edge_set
        .into_iter()
        .map(|(from, to)| Edge { from, to })
        .collect();

    let mut external: Vec<String> = external_from_uses
        .union(&external_from_manifest)
        .cloned()
        .collect();
    external.sort();

    // One deterministic order over the whole model's file-grained facts.
    file_edges.sort_by(|left, right| {
        (&left.unit, &left.from, &left.to, left.kind.as_str())
            .cmp(&(&right.unit, &right.from, &right.to, right.kind.as_str()))
    });

    let manifest = manifest_info(&root_manifest, workspace_deps);

    Ok(Model {
        units,
        edges,
        soft_structure,
        external,
        module_edges,
        manifest: Some(manifest),
        root_public_exports,
        root_glob_exports,
        root_empty_glob_exports,
        module_external,
        root_module_declarations,
        unit_manifests,
        unresolved_module_files: unresolved_files,
        test_gated_modules,
        file_edges,
        roles,
        ..Model::new("rust")
    })
}

struct UnitModuleInfo {
    soft: Vec<String>,
    edges: Vec<ModuleEdge>,
    external: Vec<String>,
    module_external: BTreeMap<String, Vec<String>>,
    root_public_exports: Vec<String>,
    root_glob_exports: Vec<String>,
    root_empty_glob_exports: Vec<String>,
    root_module_declarations: Vec<ModuleDeclaration>,
    unit_edges: Vec<(String, String)>,
    unresolved: Vec<String>,
    test_gated: Vec<String>,
    /// File-grained dependency facts of this unit, spelled repo-relative
    /// (the diagram's file-vertex identity), sorted by `(from, to, kind)`.
    file_edges: Vec<FileEdge>,
    /// True when the unit's root file defines at least one production item
    /// (top-level struct/enum/trait/fn/type/const/static, not `#[cfg(test)]`).
    /// False means the root is a publication-only facade, which gives the
    /// unit's root module path the `facade` role in the model's roles map.
    root_defines_items: bool,
}

/// Root-manifest facts for `manifest_integrity`: publish state, dependency
/// crates (real names, aliases resolved via `package = "..."`), and declared
/// feature names. A workspace root without `[package]` yields `publish: None`.
fn manifest_info(manifest: &Value, workspace_deps: Option<&Value>) -> ManifestInfo {
    let publish = manifest
        .get("package")
        .and_then(|package| package.get("publish"))
        .map(|value| match value {
            Value::Boolean(false) => false,
            // A registry list/string means "publishable" for our boolean purpose.
            _ => true,
        });

    let mut dependencies: BTreeSet<String> = BTreeSet::new();
    // Only runtime `[dependencies]` are runtime facts; dev- and build-deps are
    // kept out of `dependencies` and never surface in `external`.
    if let Some(table) = manifest.get("dependencies").and_then(Value::as_table) {
        for (name, value) in table {
            dependencies.insert(dependency_real_name(name, value, workspace_deps));
        }
    }

    let features: Vec<String> = manifest
        .get("features")
        .and_then(Value::as_table)
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default();

    ManifestInfo {
        publish,
        dependencies: dependencies.into_iter().collect(),
        features,
    }
}

/// Extract module-level structure for one unit from its `src/` tree: the soft
/// module hierarchy, intra-crate module edges (from `use crate::`/`super::` and
/// qualified-path calls), the symbols used along each edge, external crates
/// referenced from `use`, and the unit's cross-unit edges found in source.
fn analyze_unit_modules(
    root: &Path,
    unit: &Unit,
    normalized_units: &BTreeMap<String, String>,
    manifest_deps: Option<&BTreeSet<String>>,
) -> Result<UnitModuleInfo, String> {
    let empty = UnitModuleInfo {
        soft: Vec::new(),
        edges: Vec::new(),
        external: Vec::new(),
        module_external: BTreeMap::new(),
        root_public_exports: Vec::new(),
        root_glob_exports: Vec::new(),
        root_empty_glob_exports: Vec::new(),
        root_module_declarations: Vec::new(),
        unit_edges: Vec::new(),
        unresolved: Vec::new(),
        test_gated: Vec::new(),
        file_edges: Vec::new(),
        // A unit whose root file cannot be located proves nothing about its
        // definitions; keep the facade rule inactive rather than invent one.
        root_defines_items: true,
    };
    let unit_dir = unit_dir_for(root, unit);
    let src_dir = unit_dir.join("src");
    // Each compilation target analyzes its own root file only (B25/B26).
    let root_files: Vec<String> = vec![unit.root.clone()];
    if !src_dir.join(&unit.root).exists() {
        return Ok(empty);
    }

    // Parse every `.rs` under `src/`, keyed by its path relative to `src/`.
    let mut files: BTreeMap<PathBuf, syn::File> = BTreeMap::new();
    for path in collect_rs_files(root, &src_dir) {
        if let Ok(rel) = path.strip_prefix(&src_dir) {
            if files.contains_key(rel) {
                continue;
            }
            let source = std::fs::read_to_string(&path)
                .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            let ast = syn::parse_file(&source)
                .map_err(|_| format!("failed to parse source: {}", path.display()))?;
            files.insert(rel.to_path_buf(), ast);
        }
    }

    // First pass: discover the full module tree and gather `use` trees plus
    // qualified-path references per module.
    let mut state = WalkState {
        files: &files,
        known: BTreeSet::new(),
        soft: BTreeSet::new(),
        uses: Vec::new(),
        unresolved: BTreeSet::new(),
        test_gated: BTreeSet::new(),
        module_files: BTreeMap::new(),
        file_rows: BTreeSet::new(),
    };
    let crate_name = unit.name.clone();
    // The module tree is rooted at the unit itself for discovery; the bin root
    // (`main.rs`) renames its own module path to `<unit>::main` so edge sources
    // read as `voice-cli::main`.
    let root_path: Vec<String> = vec![unit.name.clone()];
    state.known.insert(unit.name.clone());
    let root_rel_dir = Path::new(unit.root.rsplit_once('/').map(|(d, _)| d).unwrap_or(""));
    for root_file in &root_files {
        let Some(root_ast) = files.get(&PathBuf::from(root_file)) else {
            continue;
        };
        walk_module(
            &root_path,
            root_rel_dir,
            root_rel_dir,
            Path::new(root_file),
            &root_ast.items,
            &mut state,
            false,
        );
    }
    // The bin root is a `main` module; it is a real soft node and its own source
    // path reads as `<unit>::main`. Record the node so edges sourced from it
    // resolve against the module set (otherwise `update` seeds a boundary that
    // `verify` reports as a missing component). Lib roots keep their unit-name
    // node — no `::main` is added.
    if unit.root == "main.rs" {
        let main_path = format!("{}::main", unit.name);
        state.known.insert(main_path.clone());
        state.soft.insert(main_path.clone());
        state
            .module_files
            .entry(main_path)
            .or_insert_with(|| PathBuf::from(unit.root.clone()));
        for entry in state.uses.iter_mut() {
            if entry.0.len() == 1 && entry.0[0] == unit.name {
                entry.0.push("main".to_string());
            }
        }
    }

    // Second pass: resolve `use` trees and qualified paths against the module set.
    // The collected references move out of the walk state so the same state's
    // `module_files` can map every resolved target to its file identity.
    let uses = std::mem::take(&mut state.uses);
    // Declaration index for in-scope resolution of bare `use` segments: the
    // mod-declaration pairs (declaring file, backed file) collected by the walk
    // tell which modules THIS file declares; `module_files` locates each
    // module's defining file (inline declarations map to the declaring file).
    let module_files = state.module_files.clone();
    let mod_pairs: BTreeSet<(PathBuf, PathBuf)> = state
        .file_rows
        .iter()
        .filter(|(_, _, kind)| matches!(kind, FileFactKind::Mod))
        .map(|(from, to, _)| (from.clone(), to.clone()))
        .collect();
    let mut edge_map: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut external: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut unit_edges: BTreeSet<(String, String)> = BTreeSet::new();
    for (current, trees, paths, trait_refs, file) in &uses {
        let source_dotted = current.join("::");
        for tree in trees {
            let mut paths_flat = Vec::new();
            flatten_use_tree(tree, Vec::new(), &mut paths_flat);
            for segments in paths_flat {
                let first = segments[0].as_str();
                if is_reserved_segment(first) {
                    let full = resolve_module_path(&segments, current, &crate_name);
                    if let Some((target, symbols)) = target_module(&full, &state.known) {
                        if target != source_dotted {
                            edge_map
                                .entry((source_dotted.clone(), target.clone()))
                                .or_default()
                                .extend(symbols);
                        }
                        // A CONTAINMENT hop is not a dependency: a path that
                        // lands on an ancestor module of the using module —
                        // spelled `super::`, `crate::`-absolute, or a bare
                        // name — points at a module that CONTAINS the using
                        // one. The module tier already edges that pair as a
                        // descendant→ancestor module edge, so the file tier
                        // may not row it either (the tier-comparability law
                        // of the D6 projection dedup: a file ring closing on
                        // an ancestor hop would re-report a fact the module
                        // tier owns, and ring clean containment trees —
                        // e.g. a sibling Rust workspace crate's freshness
                        // check or the sibling crate's config module).
                        // Sibling hops through a parent's item and
                        // descendant-target rows keep their row: the target
                        // is not an ancestor of the source.
                        let containment_hop = contains_module(&target, &source_dotted);
                        if !containment_hop {
                            record_file_row(&mut state, file, &target, FileFactKind::Use);
                        }
                    }
                } else {
                    let normalized = normalize_crate_name(first);
                    if is_stdlib(&normalized) {
                        continue;
                    }
                    if let Some(real) = normalized_units.get(&normalized) {
                        // A reference to a unit of the project (self or a member),
                        // never external. A cross-unit `use` (e.g. a bin importing
                        // its own lib by crate name) is a unit edge; a reference to
                        // the unit's OWN crate name stays internal. A cross-unit
                        // reference addresses another unit's source tree, never a
                        // file of THIS unit's, so it contributes no file row.
                        if *real != crate_name {
                            unit_edges.insert((crate_name.clone(), real.clone()));
                        }
                        continue;
                    }
                    // Resolution law for a bare first segment of a `use` path
                    // (flip lane 2026-10-16): the name is in scope only if the
                    // CURRENT FILE declares it (`mod name;` here, inline or
                    // file-backed). Rust resolution never walks ancestors or
                    // the crate root for a bare use segment, so the old
                    // `own_module_prefix` probe mis-attributed `use tauri::{...}`
                    // inside module `tauri` to the crate's own `tauri` module —
                    // a false parent back-edge and a phantom file-tier cycle
                    // (voice-lib, the fleet's last residual). Anything else is
                    // a crate reference (external here). A name imported into
                    // scope by an earlier `use` is not tracked — an
                    // over-approximation toward `external`, never toward a
                    // false internal edge.
                    if let Some(own_path) = module_declared_in_file(
                        first,
                        current,
                        file,
                        &state.known,
                        &module_files,
                        &mod_pairs,
                    ) {
                        let mut full: Vec<String> =
                            own_path.split("::").map(String::from).collect();
                        full.extend_from_slice(&segments[1..]);
                        if let Some((target, symbols)) = target_module(&full, &state.known) {
                            if target != source_dotted {
                                edge_map
                                    .entry((source_dotted.clone(), target.clone()))
                                    .or_default()
                                    .extend(symbols);
                            }
                            record_file_row(&mut state, file, &target, FileFactKind::Use);
                        }
                    } else {
                        external.insert(normalized.clone());
                        module_external
                            .entry(source_dotted.clone())
                            .or_default()
                            .insert(normalized);
                    }
                }
            }
        }

        // Qualified-path references: `audio::fn()`, `crate::x::run()`,
        // `Type::method()`, and cross-unit refs like `voice_app::run()`.
        // A single-segment path (`String`, `Vec`, `data`, `stream`) is a local
        // variable or prelude type, never a crate/module reference — skip it.
        for path in paths {
            if path.len() < 2 {
                continue;
            }
            let first = path[0].as_str();
            if is_reserved_segment(first) {
                let full = resolve_module_path(path, current, &crate_name);
                if let Some((target, _)) = target_module(&full, &state.known) {
                    if target != source_dotted {
                        edge_map
                            .entry((source_dotted.clone(), target.clone()))
                            .or_default();
                    }
                    // The containment-hop law of the `use` walk above: a
                    // path expression that lands on an ancestor module
                    // contributes no file row, whatever spelling it uses.
                    let containment_hop = contains_module(&target, &source_dotted);
                    if !containment_hop {
                        record_file_row(&mut state, file, &target, FileFactKind::Path);
                    }
                }
            } else {
                let normalized = normalize_crate_name(first);
                if is_stdlib(&normalized) {
                    continue;
                }
                if let Some(real) = normalized_units.get(&normalized) {
                    if *real != crate_name {
                        // Qualified call into another unit -> a unit edge (B24).
                        // A cross-unit call addresses another unit's tree, never
                        // a file of THIS unit's, so it contributes no file row.
                        unit_edges.insert((crate_name.clone(), real.clone()));
                    }
                } else if let Some(own_path) =
                    own_module_prefix(first, current, &crate_name, &state.known)
                {
                    let target = own_path;
                    if target != source_dotted {
                        edge_map
                            .entry((source_dotted.clone(), target.clone()))
                            .or_default();
                    }
                    record_file_row(&mut state, file, &target, FileFactKind::Path);
                } else if manifest_deps.is_some_and(|deps| deps.contains(&normalized)) {
                    // A qualified path whose first segment is a real declared
                    // runtime dependency (e.g. `toml::from_str` with no `use`) is
                    // an external crate. Gated on the manifest so locals and
                    // type names (`Arc::new`, `Foo::bar`) never match.
                    external.insert(normalized.clone());
                    module_external
                        .entry(source_dotted.clone())
                        .or_default()
                        .insert(normalized);
                }
                // A qualified path whose first segment is none of a unit, a known
                // module, or stdlib is a type/local (e.g. `Arc::new`, `Foo::bar`)
                // — never recorded as an external crate. External crates come from
                // `use` imports and the manifest, not from type/expr paths.
            }
        }

        // Trait- and type-position references (impl-header trait paths,
        // trait-object `dyn` bounds, type paths nested in generic arguments —
        // the accepted extraction arms of the dyn-edge adjudication §7 item 2,
        // invisible to the `paths` set because `PathVisitor`'s law prunes them).
        // Resolution mirrors the two cited laws: the unit tier resolves the
        // first segment against `normalized_units` like the cross-unit `use`
        // branch above, and the module tier resolves a reserved-segment path
        // against the module set with the containment-hop law like the
        // qualified-path walk above. Two absences are law, not omission: trait
        // positions never contribute an `external` row (external facts come
        // from `use` and the manifest), and a bare first segment is probed
        // against neither own modules nor `manifest_deps` here — non-unit
        // trait paths state nothing. Single-segment trait paths (a self impl
        // `impl Foo for Bar`, a generic self type, `dyn Fn()`) fall under the
        // same lenient single-segment law.
        for path in trait_refs {
            if path.len() < 2 {
                continue;
            }
            let first = path[0].as_str();
            if is_reserved_segment(first) {
                let full = resolve_module_path(path, current, &crate_name);
                if let Some((target, _)) = target_module(&full, &state.known) {
                    if target != source_dotted {
                        edge_map
                            .entry((source_dotted.clone(), target.clone()))
                            .or_default();
                    }
                    let containment_hop = contains_module(&target, &source_dotted);
                    if !containment_hop {
                        record_file_row(&mut state, file, &target, FileFactKind::Path);
                    }
                }
            } else {
                let normalized = normalize_crate_name(first);
                if is_stdlib(&normalized) {
                    continue;
                }
                if let Some(real) = normalized_units.get(&normalized) {
                    // Cross-unit trait reference -> a unit edge, exactly like a
                    // cross-unit `use` or qualified call. A reference through
                    // the unit's OWN crate name stays internal. Like the other
                    // cross-unit branches, it addresses another unit's source
                    // tree, so it contributes no file row.
                    if *real != crate_name {
                        unit_edges.insert((crate_name.clone(), real.clone()));
                    }
                }
            }
        }
    }

    let mut edges: Vec<ModuleEdge> = edge_map
        .into_iter()
        .map(|((from, to), symbols)| {
            let mut symbols: Vec<String> = symbols.into_iter().collect();
            symbols.sort();
            ModuleEdge {
                type_only: false,
                unit: unit.name.clone(),
                from,
                to,
                symbols,
            }
        })
        .collect();
    edges.sort_by(|left, right| {
        (&left.unit, &left.from, &left.to).cmp(&(&right.unit, &right.from, &right.to))
    });

    let mut soft_sorted: Vec<String> = state.soft.into_iter().collect();
    soft_sorted.sort();
    let mut external_sorted: Vec<String> = external.into_iter().collect();
    external_sorted.sort();
    let module_external: BTreeMap<String, Vec<String>> = module_external
        .into_iter()
        .map(|(module, crates)| {
            let mut sorted: Vec<String> = crates.into_iter().collect();
            sorted.sort();
            (module, sorted)
        })
        .collect();
    let mut unit_edges: Vec<(String, String)> = unit_edges.into_iter().collect();
    unit_edges.sort();

    let mut public_exports: BTreeSet<String> = BTreeSet::new();
    let mut glob_exports: BTreeSet<String> = BTreeSet::new();
    let mut empty_glob_exports: BTreeSet<String> = BTreeSet::new();
    let mut module_declarations: Vec<ModuleDeclaration> = Vec::new();
    // Every identifier this crate is referenced by, dash/underscore normalized,
    // so a glob spelled with the crate-name prefix (`voice_app_lib::x::*`) is
    // recognized as a local path.
    let crate_ids: BTreeSet<String> = unit
        .crate_ids
        .iter()
        .map(|id| normalize_crate_name(id))
        .collect();
    for root_file in &root_files {
        let Some(root_ast) = files.get(&PathBuf::from(root_file)) else {
            continue;
        };
        public_exports.extend(root_public_exports(&root_ast.items));
        let ctx = GlobCtx {
            files: &files,
            known: &state.known,
            crate_name: &crate_name,
            crate_ids: &crate_ids,
            root_file: root_file.as_str(),
            root_rel_dir,
            module_files: &module_files,
            mod_pairs: &mod_pairs,
        };
        classify_root_globs(
            &ctx,
            &root_ast.items,
            &mut public_exports,
            &mut glob_exports,
            &mut empty_glob_exports,
        );
        module_declarations.extend(root_module_declarations(
            &root_ast.items,
            &files,
            root_file,
        ));
    }
    let root_defines_items = root_files
        .iter()
        .filter_map(|file| files.get(&PathBuf::from(file)))
        .any(|ast| root_defines_production_items(&ast.items));
    let root_public_exports: Vec<String> = public_exports.into_iter().collect();
    let root_glob_exports: Vec<String> = glob_exports.into_iter().collect();
    let root_empty_glob_exports: Vec<String> = empty_glob_exports.into_iter().collect();
    module_declarations.sort_by(|left, right| left.name.cmp(&right.name));
    module_declarations.dedup_by(|left, right| left.name == right.name);
    let root_module_declarations = module_declarations;

    // File-grained facts serialize repo-relative — the same file identity the
    // diagram's file vertices and the file-tier findings name. A row whose
    // endpoints spell to the same repo-relative path (the same src file seen
    // twice) is not an adjacency fact and is dropped here, not at a consumer.
    let src_prefix = match unit_dir.strip_prefix(root) {
        Ok(prefix) if !prefix.as_os_str().is_empty() => prefix.join("src"),
        _ => PathBuf::from("src"),
    };
    let mut file_edges: Vec<FileEdge> = state
        .file_rows
        .iter()
        .filter(|(from, to, _)| from != to)
        .map(|(from, to, kind)| FileEdge {
            unit: unit.name.clone(),
            from: spell_repo_file(&src_prefix, from),
            to: spell_repo_file(&src_prefix, to),
            kind: *kind,
        })
        .collect();
    file_edges.sort_by(|left, right| {
        (&left.from, &left.to, left.kind.as_str())
            .cmp(&(&right.from, &right.to, right.kind.as_str()))
    });

    Ok(UnitModuleInfo {
        soft: soft_sorted,
        edges,
        external: external_sorted,
        module_external,
        root_public_exports,
        root_glob_exports,
        root_empty_glob_exports,
        root_module_declarations,
        unit_edges,
        unresolved: state.unresolved.into_iter().collect(),
        test_gated: state.test_gated.into_iter().collect(),
        file_edges,
        root_defines_items,
    })
}

/// Record a file-grained fact for a resolved module target: the referring
/// file -> the file DEFINING the target module, when those are two distinct
/// files. A target the walk never located (a cross-unit path, or a module
/// path assembled outside the walk) states no file identity and contributes
/// no row — the fact is absent, never invented.
fn record_file_row(state: &mut WalkState, from_file: &Path, target: &str, kind: FileFactKind) {
    if let Some(target_file) = state.module_files.get(target)
        && target_file != from_file
    {
        state
            .file_rows
            .insert((from_file.to_path_buf(), target_file.clone(), kind));
    }
}

/// Spell a `src/`-relative file as the repo-relative path the file identity
/// uses ("/" separators — the ts driver's `spell` law).
fn spell_repo_file(src_prefix: &Path, rel: &Path) -> String {
    src_prefix.join(rel).to_string_lossy().replace('\\', "/")
}

/// True when a crate-root file DEFINES anything at the top level: a
/// `struct`/`enum`/`trait`/`fn`/`type`/`const`/`static` (any visibility) that
/// is not `#[cfg(test)]`-gated — a test-only definition is not production
/// surface. `mod` declarations (inline or file-backed, content included),
/// `use`/`pub use` re-exports and other items do not count: a root holding
/// only those is a publication-only facade, so internal→root edges in that
/// unit violate the facade rule.
fn root_defines_production_items(items: &[syn::Item]) -> bool {
    items.iter().any(|item| {
        let attrs = match item {
            syn::Item::Struct(item) => &item.attrs,
            syn::Item::Enum(item) => &item.attrs,
            syn::Item::Trait(item) => &item.attrs,
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::Type(item) => &item.attrs,
            syn::Item::Const(item) => &item.attrs,
            syn::Item::Static(item) => &item.attrs,
            _ => return false,
        };
        !is_cfg_test(attrs)
    })
}

/// Crate-root public API surface: every item declared `pub` at the top level of
/// `lib.rs`/`main.rs` (`pub mod`, `pub fn`, `pub use`, `pub struct/enum/trait/
/// type/const/static`). Non-public items — the crate's internal plumbing, the
/// `mod` declarations for internal modules, `#[macro_export]` macro bodies,
/// `main`/`run` fns — are reserved and excluded here. Only a true `pub` (not
/// `pub(crate)`/`pub(super)`) counts as public API.
fn root_public_exports(items: &[syn::Item]) -> Vec<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    for item in items {
        match item {
            syn::Item::Use(use_item) => {
                if is_public(&use_item.vis) {
                    pub_use_names(&use_item.tree, &mut out);
                }
            }
            other => {
                if let Some(name) = public_item_name(other) {
                    out.insert(name);
                }
            }
        }
    }
    out.into_iter().collect()
}

/// The name a `pub` item contributes to its module's public surface (`pub mod`,
/// `pub fn`, `pub struct/enum/trait/type/const/static`), or `None` for a
/// non-public item, a `use` (its names come from `pub_use_names`), or any other
/// item kind. Shared by the crate root and by glob-target enumeration so both
/// count a public item the same way.
fn public_item_name(item: &syn::Item) -> Option<String> {
    let (ident, visibility) = match item {
        syn::Item::Mod(module) => (&module.ident, &module.vis),
        syn::Item::Fn(function) => (&function.sig.ident, &function.vis),
        syn::Item::Struct(strukt) => (&strukt.ident, &strukt.vis),
        syn::Item::Enum(en) => (&en.ident, &en.vis),
        syn::Item::Trait(trait_item) => (&trait_item.ident, &trait_item.vis),
        syn::Item::Type(type_item) => (&type_item.ident, &type_item.vis),
        syn::Item::Const(const_item) => (&const_item.ident, &const_item.vis),
        syn::Item::Static(static_item) => (&static_item.ident, &static_item.vis),
        _ => return None,
    };
    is_public(visibility).then(|| ident.to_string())
}

/// What a root glob re-export resolves to at scan time.
enum GlobOutcome {
    /// The target module (and every module reachable through nested globs in
    /// its public chain) was located from source; these are its public items.
    Resolved(BTreeSet<String>),
    /// The chain resolved, but the re-exported public surface is empty — a
    /// checked surface that would silently vanish, so it fails closed.
    Empty,
    /// The target is not the crate's own source (external crate, unknown path,
    /// a crate name that isn't this unit) or a cfg-blocked/poisoned link —
    /// reported as unverifiable, never guessed.
    Unresolvable,
}

/// Everything glob resolution needs from the unit: the parsed files, the module
/// tree already walked, the crate's own names, and where its root file sits.
struct GlobCtx<'a> {
    files: &'a BTreeMap<PathBuf, syn::File>,
    known: &'a BTreeSet<String>,
    crate_name: &'a str,
    crate_ids: &'a BTreeSet<String>,
    root_file: &'a str,
    root_rel_dir: &'a Path,
    module_files: &'a BTreeMap<String, PathBuf>,
    mod_pairs: &'a BTreeSet<(PathBuf, PathBuf)>,
}

/// A located module: its items and whether any `cfg` sat on a declaration along
/// the chain leading here (which blocks resolution).
struct Located<'a> {
    items: &'a [syn::Item],
    blocked: bool,
}

/// Classify every glob re-export at a crate root into three deterministic sets:
/// names enumerated from resolvable same-crate globs (checked against `allowed`
/// like named exports), glob tokens that cannot be resolved (unverifiable
/// findings), and glob tokens that resolve to an empty public surface (fail
/// closed). Only a true `pub` (not `pub(crate)`/`pub(super)`) counts.
fn classify_root_globs(
    ctx: &GlobCtx,
    items: &[syn::Item],
    enumerated: &mut BTreeSet<String>,
    unverifiable: &mut BTreeSet<String>,
    empty: &mut BTreeSet<String>,
) {
    for item in items {
        let syn::Item::Use(use_item) = item else {
            continue;
        };
        if !is_public(&use_item.vis) {
            continue;
        }
        for prefix in glob_prefixes(&use_item.tree, &[]) {
            let outcome = if has_cfg(&use_item.attrs) {
                // Any cfg on the re-export declaration makes the set
                // configuration-dependent — resolvable under no single config.
                GlobOutcome::Unresolvable
            } else {
                resolve_root_glob(&prefix, ctx)
            };
            match outcome {
                GlobOutcome::Resolved(names) => {
                    enumerated.extend(names);
                }
                GlobOutcome::Empty => {
                    empty.insert(glob_token(&prefix));
                }
                GlobOutcome::Unresolvable => {
                    unverifiable.insert(glob_token(&prefix));
                }
            }
        }
    }
}

/// The segment prefixes of every glob leaf in a `use` tree.
fn glob_prefixes(tree: &syn::UseTree, prefix: &[String]) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    pub_use_glob_paths(tree, prefix, &mut out);
    out
}

/// Resolve a root glob's prefix to the module it names, then enumerate that
/// module's public surface. Ownership precedence: `self::x::*`, `crate::x::*`
/// and the crate-name prefix `name::x::*` are one local path `x::*`; else a
/// first segment naming a root module of this crate is local; anything else
/// (external dependency, unknown path) is unresolvable.
fn resolve_root_glob(prefix: &[String], ctx: &GlobCtx) -> GlobOutcome {
    let Some(target) = root_glob_target(prefix, ctx) else {
        return GlobOutcome::Unresolvable;
    };
    match locate_module(&target, ctx) {
        Some(located) if located.blocked => GlobOutcome::Unresolvable,
        Some(located) => enumerate_surface(&located, &target, &mut BTreeSet::new(), ctx),
        None => GlobOutcome::Unresolvable,
    }
}

/// Map a root glob's prefix to the crate-prefixed dotted path of the module it
/// names, or `None` when it names no module of this crate.
fn root_glob_target(prefix: &[String], ctx: &GlobCtx) -> Option<Vec<String>> {
    let (first, rest) = prefix.split_first()?;
    match first.as_str() {
        "self" | "crate" => {
            let mut out = vec![ctx.crate_name.to_string()];
            out.extend(rest.iter().cloned());
            Some(out)
        }
        // No parent above a crate root; a bare `*` names no module.
        "super" => None,
        _ => {
            if ctx.crate_ids.contains(&normalize_crate_name(first)) {
                let mut out = vec![ctx.crate_name.to_string()];
                out.extend(rest.iter().cloned());
                return Some(out);
            }
            let candidate = format!("{}::{first}", ctx.crate_name);
            if ctx.known.contains(&candidate) {
                let mut out = vec![ctx.crate_name.to_string()];
                out.extend(prefix.iter().cloned());
                return Some(out);
            }
            None
        }
    }
}

/// Locate the module a crate-prefixed dotted path names: walk the `mod`
/// declarations down the path, resolving file-backed modules through the same
/// variants the module walk uses, and report whether any declaration on the way
/// carries a `cfg`.
fn locate_module<'a>(path: &[String], ctx: &GlobCtx<'a>) -> Option<Located<'a>> {
    let root = ctx.files.get(&PathBuf::from(ctx.root_file))?;
    let mut items = root.items.as_slice();
    let mut rel_dir = ctx.root_rel_dir.to_path_buf();
    // `#[path]` targets resolve against the directory of the file that carries
    // the declaration; for the root file that is the root directory itself.
    let mut file_dir = ctx.root_rel_dir.to_path_buf();
    let mut blocked = false;
    for segment in path.iter().skip(1) {
        let module = items
            .iter()
            .find_map(|item| match item {
                syn::Item::Mod(module) if module.ident == segment.as_str() => Some(module),
                _ => None,
            })?;
        if has_cfg(&module.attrs) {
            blocked = true;
        }
        let name = module.ident.to_string();
        match &module.content {
            Some((_, inline)) => items = inline.as_slice(),
            None => {
                let found = module_file_candidates(&module.attrs, &rel_dir, &file_dir, &name)
                    .into_iter()
                    .find_map(|(file, next_dir, next_file_dir)| {
                        ctx.files
                            .get(&file)
                            .map(|ast| (ast.items.as_slice(), next_dir, next_file_dir))
                    })?;
                items = found.0;
                rel_dir = found.1;
                file_dir = found.2;
            }
        }
    }
    Some(Located { items, blocked })
}

/// The public items a located module re-exports, following nested `pub use`
/// globs through the module tree. A visited set terminates chains that cycle
/// back into themselves; any cfg-blocked, unlocatable, or non-local link
/// poisons the whole chain (a chain is only as resolvable as its weakest
/// link). `cfg` on a plain *item* inside a reached module does not block: the
/// enumerated surface is the union across configurations, which is the stricter
/// direction for a name-based allowlist.
fn enumerate_surface<'a>(
    located: &Located<'a>,
    path: &[String],
    visited: &mut BTreeSet<String>,
    ctx: &GlobCtx<'a>,
) -> GlobOutcome {
    let mut names = BTreeSet::new();
    let mut poisoned = false;
    walk_surface(located, path, visited, &mut names, &mut poisoned, ctx);
    if poisoned {
        GlobOutcome::Unresolvable
    } else if names.is_empty() {
        GlobOutcome::Empty
    } else {
        GlobOutcome::Resolved(names)
    }
}

fn walk_surface<'a>(
    located: &Located<'a>,
    path: &[String],
    visited: &mut BTreeSet<String>,
    names: &mut BTreeSet<String>,
    poisoned: &mut bool,
    ctx: &GlobCtx<'a>,
) {
    if *poisoned || !visited.insert(path.join("::")) {
        return;
    }
    for item in located.items {
        match item {
            syn::Item::Use(use_item) => {
                if !is_public(&use_item.vis) {
                    continue;
                }
                if has_cfg(&use_item.attrs) {
                    *poisoned = true;
                    return;
                }
                pub_use_names(&use_item.tree, names);
                for prefix in glob_prefixes(&use_item.tree, &[]) {
                    let Some(target) = glob_target_path(&prefix, path, ctx) else {
                        *poisoned = true;
                        return;
                    };
                    match locate_module(&target, ctx) {
                        Some(located) if !located.blocked => {
                            walk_surface(&located, &target, visited, names, poisoned, ctx);
                            if *poisoned {
                                return;
                            }
                        }
                        _ => {
                            *poisoned = true;
                            return;
                        }
                    }
                }
            }
            other => {
                if let Some(name) = public_item_name(other) {
                    names.insert(name);
                }
            }
        }
    }
}

/// Resolve a glob prefix found in module `current` to a crate-prefixed module
/// path, or `None` when it does not name a module of this crate. `self`/`crate`
/// and the crate-name prefix behave as at the root; `super::` walks up from the
/// current module; a bare first segment names a CRATE (Rust 2018+ `use` law),
/// never an in-scope module, so it resolves only via the crate-name prefix.
fn glob_target_path(prefix: &[String], current: &[String], ctx: &GlobCtx) -> Option<Vec<String>> {
    let (first, rest) = prefix.split_first()?;
    match first.as_str() {
        "self" | "crate" => {
            let mut out = if first == "crate" {
                vec![ctx.crate_name.to_string()]
            } else {
                current.to_vec()
            };
            out.extend(rest.iter().cloned());
            Some(out)
        }
        "super" => {
            let mut out = current.to_vec();
            let mut i = 0;
            while i < prefix.len() && prefix[i] == "super" {
                out.pop()?;
                i += 1;
            }
            if out.is_empty() {
                return None;
            }
            out.extend(prefix[i..].iter().cloned());
            Some(out)
        }
        _ => {
            if ctx.crate_ids.contains(&normalize_crate_name(first)) {
                let mut out = vec![ctx.crate_name.to_string()];
                out.extend(rest.iter().cloned());
                return Some(out);
            }
            // In-scope law (mirrors the `use`-fact branch above): a bare
            // segment resolves only to a module the walking surface's file
            // declares — never an ancestor or crate-root module. Otherwise it
            // names a crate, not a module of this crate.
            let here = ctx
                .module_files
                .get(&current.join("::"))
                .cloned()
                .unwrap_or_default();
            let owner = module_declared_in_file(
                first,
                current,
                &here,
                ctx.known,
                ctx.module_files,
                ctx.mod_pairs,
            )?;
            let mut out: Vec<String> = owner.split("::").map(String::from).collect();
            out.extend(rest.iter().cloned());
            Some(out)
        }
    }
}

/// Top-level module declarations in a crate root, with their `cfg(feature =
/// "...")` gating and the module's source file relative to `src/`. Used by
/// `feature_boundary` to require gated modules to be cfg-gated and to name the
/// file of a module that is missing its gate.
fn root_module_declarations(
    items: &[syn::Item],
    files: &BTreeMap<PathBuf, syn::File>,
    root_file: &str,
) -> Vec<ModuleDeclaration> {
    let mut out = Vec::new();
    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let name = module.ident.to_string();
        // Capture which `cfg(feature = "...")` gate (if any) the module sits under.
        let feature = cfg_feature_name(&module.attrs);
        let gated = feature.is_some();
        let file = if module.content.is_some() {
            root_file.to_string()
        } else {
            let child_dir = Path::new("").join(&name);
            let variants = [
                Path::new("").join(format!("{name}.rs")),
                child_dir.join("mod.rs"),
            ];
            variants
                .iter()
                .find(|candidate| files.contains_key(*candidate))
                .map(|candidate| candidate.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("{name}.rs"))
        };
        out.push(ModuleDeclaration {
            name,
            gated,
            feature,
            file,
        });
    }
    out
}

/// Mutable state threaded through one unit's module-tree walk.
struct WalkState<'a> {
    files: &'a BTreeMap<PathBuf, syn::File>,
    known: BTreeSet<String>,
    soft: BTreeSet<String>,
    uses: Vec<ModuleRefs>,
    unresolved: BTreeSet<String>,
    test_gated: BTreeSet<String>,
    /// Dotted module path -> the source file DEFINING that module (relative to
    /// the unit's `src/`). A file-backed module maps to its resolved file; an
    /// inline `mod x { ... }` maps to the file carrying it — its items live
    /// there, so facts sourced from the inline module are facts of that file.
    module_files: BTreeMap<String, PathBuf>,
    /// File-grained adjacency rows collected lexically during the walk:
    /// `(declaring file, declared file)` mod-declaration pairs, relative to
    /// the unit's `src/`. Resolved `use`/path rows join later, in the second
    /// pass that resolves imports against `known`/`module_files`.
    file_rows: BTreeSet<(PathBuf, PathBuf, FileFactKind)>,
}

/// Recursively walk a module's items: record each declared sub-module (inline or
/// file-backed) in `state.soft`/`state.known`, collect its `use` trees and
/// qualified-path references for later resolution, and record file-backed
/// declarations that resolve to no parsed file in `state.unresolved`.
/// `file` is the parsed file whose items are walked (relative to the unit's
/// `src/`): every module defined here — file-backed or inline — is registered
/// in `state.module_files` against it, and a file-backed declaration that
/// resolves contributes a `mod` row to `state.file_rows`.
fn walk_module(
    path: &[String],
    mod_dir: &Path,
    file_dir: &Path,
    file: &Path,
    items: &[syn::Item],
    state: &mut WalkState,
    in_test_module: bool,
) {
    state
        .module_files
        .entry(path.join("::"))
        .or_insert_with(|| file.to_path_buf());
    let mut use_trees = Vec::new();
    let mut paths = BTreeSet::new();
    let mut trait_refs = BTreeSet::new();
    for item in items {
        if let syn::Item::Use(use_item) = item {
            // Test-gated imports are scaffolding, not production dependency:
            // a `use` inside a `#[cfg(test)]` module contributes no file row
            // and no reference (the module tier's one-case-one-verdict law,
            // carried to file grain — a production file whose `mod tests`
            // block imports its own module is a declaration pair, not a ring).
            // Nested items below an `Item::Mod` are walked by the recursive
            // call below, which re-enters this function with the gate known.
            if !in_test_module {
                use_trees.push(use_item.tree.clone());
            }
        }
    }
    // Walk every item body (fn/impl/type positions) for path references.
    // The visitor recurses into modules too, so test-gated subtrees are
    // excluded by the same law: their references are collected only when the
    // walk reaches them from outside a `#[cfg(test)]` gate — never here.
    let mut visitor = PathVisitor { out: &mut paths };
    let mut trait_visitor = TraitRefVisitor { out: &mut trait_refs };
    for item in items {
        if in_test_module && matches!(item, syn::Item::Mod(_)) {
            continue;
        }
        visitor.visit_item(item);
        trait_visitor.visit_item(item);
    }
    if !in_test_module {
        state.uses.push((
            path.to_vec(),
            use_trees,
            paths,
            trait_refs,
            file.to_path_buf(),
        ));
    }

    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let name = module.ident.to_string();
        let mut child_path = path.to_vec();
        child_path.push(name.clone());
        let child_dotted = child_path.join("::");
        state.known.insert(child_dotted.clone());
        state.soft.insert(child_dotted.clone());
        // A `#[cfg(test)]` declaration (or an equivalent test-only `cfg(all(...,
        // test, ...))`) marks this module — and everything beneath it — as
        // scaffolding. Record the declaration site; the consumer resolves
        // descendants by ancestry. A module merely *named* `tests` without a
        // test cfg is NOT recorded here, so it stays in the graph.
        if is_cfg_test(&module.attrs) {
            state.test_gated.insert(child_dotted.clone());
        }

        if let Some((_, inline_items)) = &module.content {
            // Inline `mod X { ... }` — no file; declarations keep resolving
            // against the current file's module directory. The module's items
            // live in the declaring file, so it contributes no `mod` row.
            let child_test = in_test_module || is_cfg_test(&module.attrs);
            walk_module(
                &child_path,
                mod_dir,
                file_dir,
                file,
                inline_items,
                state,
                child_test,
            );
            continue;
        }
        // File-backed `mod X;` — resolve an attributed target when declared,
        // else the conventional `<dir>/X.rs` / `<dir>/X/mod.rs` variants.
        // A declaration resolving to no parsed file is recorded in
        // `unresolved`: silently skipping it would hide its contents from
        // every check. A resolved declaration is a file-grained `mod` fact:
        // declaring file -> declared module's file.
        let mut resolved = false;
        for (candidate, next_dir, next_file_dir) in
            module_file_candidates(&module.attrs, mod_dir, file_dir, &name)
        {
            if let Some(ast) = state.files.get(&candidate) {
                state
                    .file_rows
                    .insert((file.to_path_buf(), candidate.clone(), FileFactKind::Mod));
                let child_test = in_test_module || is_cfg_test(&module.attrs);
                walk_module(
                    &child_path,
                    &next_dir,
                    &next_file_dir,
                    &candidate,
                    &ast.items,
                    state,
                    child_test,
                );
                resolved = true;
                break;
            }
        }
        if !resolved {
            state.unresolved.insert(child_dotted.clone());
        }
    }
}

/// The candidate source files of a file-backed `mod` declaration, in resolution
/// order, each paired with the two directories its own declarations would
/// resolve against: the module directory for plain `mod` items and the file
/// directory for `#[path]` targets. An unconditional `#[path]` names the one
/// file rustc requires; `cfg_attr` alternatives are candidates behind which the
/// conventional variants remain, because a predicate the scanner does not
/// evaluate may be false and then rustc compiles the conventional file.
/// Conventional layout keeps children in `<dir>/<name>/`; a `name.rs` file
/// carries plain children in `<dir>/<name>/` but `#[path]` targets in `<dir>`.
/// An attributed target hosts both in its own directory. `#[path]` targets
/// always join the directory of the file carrying the declaration, which
/// rustc's module-directory convention (`<dir>/<name>/`) differs from. Shared
/// by the module walk and the glob-chain locator so both apply rustc's
/// file-resolution rule identically.
fn module_file_candidates(
    attrs: &[syn::Attribute],
    mod_dir: &Path,
    file_dir: &Path,
    name: &str,
) -> Vec<(PathBuf, PathBuf, PathBuf)> {
    let child_dir = mod_dir.join(name);
    // Variants are `(file, attributed)` in resolution order.
    let mut file_variants: Vec<(PathBuf, bool)> = Vec::new();
    let mut attributed_only = false;
    match path_attribute(attrs) {
        Some(AttributeTarget::Fixed(target)) => {
            file_variants.push((file_dir.join(normalize_relative(&target)), true));
            attributed_only = true;
        }
        Some(AttributeTarget::Candidates(targets)) => {
            file_variants.extend(
                targets
                    .iter()
                    .map(|target| (file_dir.join(normalize_relative(target)), true)),
            );
        }
        None => {}
    }
    if !attributed_only {
        file_variants.push((mod_dir.join(format!("{name}.rs")), false));
        file_variants.push((child_dir.join("mod.rs"), false));
    }
    file_variants
        .into_iter()
        .map(|(file, from_path_attr)| {
            let own_dir = file
                .parent()
                .map_or_else(PathBuf::new, |parent| parent.to_path_buf());
            let next_dir = if from_path_attr || file.ends_with("mod.rs") {
                own_dir.clone()
            } else {
                child_dir.clone()
            };
            (file, next_dir, own_dir)
        })
        .collect()
}

/// How a declaration's attributes pin a file-backed module's source file.
enum AttributeTarget {
    /// An unconditional `#[path = "..."]`: rustc requires exactly this file.
    Fixed(PathBuf),
    /// The targets of `#[cfg_attr(predicate, path = "...")]` attributes, in
    /// declaration order. rustc applies the one whose predicate holds and the
    /// conventional file when none does, so every target is a candidate.
    Candidates(Vec<PathBuf>),
}

/// The file(s) the attributes of a `mod` declaration attribute to it.
fn path_attribute(attrs: &[syn::Attribute]) -> Option<AttributeTarget> {
    for attr in attrs {
        if attr.path().is_ident("path")
            && let Some(target) = path_literal(&attr.meta) {
                return Some(AttributeTarget::Fixed(target));
            }
    }
    let mut candidates = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("cfg_attr") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            continue;
        };
        let Ok(args) = syn::parse::Parser::parse2(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            list.tokens.clone(),
        ) else {
            continue;
        };
        // The first argument is the cfg predicate; the rest are attributes
        // applied when it holds.
        candidates.extend(args.iter().skip(1).filter_map(|meta| {
            if meta.path().is_ident("path") {
                path_literal(meta)
            } else {
                None
            }
        }));
    }
    (!candidates.is_empty()).then_some(AttributeTarget::Candidates(candidates))
}

/// The literal target of a `path = "..."` meta item, in that exact shape.
fn path_literal(meta: &syn::Meta) -> Option<PathBuf> {
    let syn::Meta::NameValue(syn::MetaNameValue {
        value:
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(target),
                ..
            }),
        ..
    }) = meta
    else {
        return None;
    };
    Some(PathBuf::from(target.value()))
}

/// Drop `.` components (`./x.rs` and `x.rs` name the same file) while keeping
/// parent-dir and normal components for `Path::join`.
fn normalize_relative(path: &Path) -> PathBuf {
    path.components()
        .filter(|component| !matches!(component, std::path::Component::CurDir))
        .collect()
}

/// Collects the segments of every `ExprPath` and `TypePath` reference in a
/// module's item bodies. Sub-modules (`syn::Item::Mod`) are deliberately not
/// descended into: their bodies belong to the child module and are walked
/// separately by `walk_module`.
struct PathVisitor<'a> {
    out: &'a mut BTreeSet<Vec<String>>,
}

impl<'a> Visit<'a> for PathVisitor<'a> {
    fn visit_item_mod(&mut self, _node: &'a syn::ItemMod) {}

    fn visit_expr_path(&mut self, node: &'a syn::ExprPath) {
        self.out.insert(path_segments(&node.path));
    }

    fn visit_type_path(&mut self, node: &'a syn::TypePath) {
        self.out.insert(path_segments(&node.path));
    }
}

/// Collects references from the positions `PathVisitor`'s law prunes away:
/// the trait path of an `impl Trait for Type` header, the trait bounds of a
/// `dyn` trait object, and type paths nested inside generic arguments (the
/// default `visit_type_path` recursion never runs in `PathVisitor`, so the
/// whole subtree below the outermost path segment is invisible there). This
/// feeds a set kept separate from `paths` — the existing set's single-segment
/// law and stdlib drops stay byte-stable, and resolution runs in its own
/// second-pass branch. Sub-modules (`syn::Item::Mod`) are deliberately not
/// descended into: their references belong to the child module and are walked
/// separately by `walk_module`, so test-gated subtrees stay behind the same
/// gate as every other reference.
struct TraitRefVisitor<'a> {
    out: &'a mut BTreeSet<Vec<String>>,
}

impl<'a> Visit<'a> for TraitRefVisitor<'a> {
    fn visit_item_mod(&mut self, _node: &'a syn::ItemMod) {}

    fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
        if let Some((_, path, _)) = &node.trait_ {
            self.out.insert(path_segments(path));
        }
        // Continue with the default recursion: the impl's self type, generic
        // arguments and item bodies hold further type-position references.
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_type_path(&mut self, node: &'a syn::TypePath) {
        self.out.insert(path_segments(&node.path));
        // Unlike `PathVisitor`, recursion continues into the path's generic
        // arguments — the nested type paths there are a fact of this visitor.
        syn::visit::visit_type_path(self, node);
    }

    fn visit_type_trait_object(&mut self, node: &'a syn::TypeTraitObject) {
        for bound in &node.bounds {
            if let syn::TypeParamBound::Trait(trait_bound) = bound {
                self.out.insert(path_segments(&trait_bound.path));
            }
        }
        syn::visit::visit_type_trait_object(self, node);
    }
}

/// Resolve a `crate`/`self`/`super`-led path to the unit's dotted module path
/// space (crate-name-prefixed).
fn resolve_module_path(segments: &[String], current: &[String], crate_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    match segments[0].as_str() {
        "crate" => {
            out.push(crate_name.to_string());
            i = 1;
        }
        "self" => {
            out = current.to_vec();
            i = 1;
        }
        "super" => {
            out = current.to_vec();
            while i < segments.len() && segments[i] == "super" {
                out.pop();
                i += 1;
            }
        }
        _ => {
            out.push(segments[0].clone());
            i = 1;
        }
    }
    out.extend_from_slice(&segments[i..]);
    out
}

/// Find the dotted path of the module named `name` DECLARED BY `file`: a child
/// `current::name` (the conventional layout — the path can only exist because
/// this module declares it), or any known module whose last segment is `name`
/// and whose defining file is `file` itself (an inline `mod name { ... }`) or
/// is backed by a mod-declaration pair from `file` (covers declarations whose
/// dotted path skips a segment, e.g. `mod store;` in a bin's `main.rs` yielding
/// `tool::store`). Rust resolution puts only the declaring file's own names in
/// scope for a bare `use`/path segment; ancestors and the crate root are NOT
/// walked. Returns `None` when no declaration in this file matches, so the
/// caller treats the segment as a crate reference.
/// True when `target` is a STRICT ANCESTOR module of `source` — it contains
/// `source` as a descendant, or `target` is the crate root module and `source`
/// sits below it. Containment is the `mod`-tree relation, not a dependency:
/// the containment-hop law that keeps ancestor-target rows out of the file
/// tier rests on this predicate.
fn contains_module(target: &str, source: &str) -> bool {
    if target.is_empty() {
        return !source.is_empty();
    }
    source
        .strip_prefix(&format!("{target}::"))
        .is_some_and(|rest| !rest.is_empty())
}

fn module_declared_in_file(
    name: &str,
    current: &[String],
    file: &Path,
    known: &BTreeSet<String>,
    module_files: &BTreeMap<String, PathBuf>,
    mod_pairs: &BTreeSet<(PathBuf, PathBuf)>,
) -> Option<String> {
    // A module that CONTAINS the using file is never a name-match candidate:
    // inside module `config` the bare name `config` resolves to the containing
    // module itself (an import from the parent module), not to a same-named
    // sibling elsewhere in the tree. The old find_map walked `known` by name
    // and hit a sibling first — a false back-edge from child to parent's
    // declaring file and a phantom parent cycle (the sibling crate's
    // config module).
    let current_joined = current.join("::");
    let contained = |path: &str| {
        path == current_joined
            || current_joined
                .strip_prefix(&format!("{path}::"))
                .is_some_and(|rest| !rest.is_empty())
    };
    let direct = format!("{}::{name}", current.join("::"));
    if known.contains(&direct) {
        return Some(direct);
    }
    known.iter().find_map(|x| {
        let last = x.rsplit("::").next()?;
        if last != name || contained(x) {
            return None;
        }
        let backing = module_files.get(x)?;
        if backing == file
            || mod_pairs.contains(&(file.to_path_buf(), backing.clone()))
        {
            Some(x.clone())
        } else {
            None
        }
    })
}

/// Given a bare first segment `name`, find the crate-prefixed dotted path of the
/// own module it resolves to, or `None`. Rust bare paths resolve relative to the
/// current module and up through its ancestors, so `client` in
/// `acme-lib::core` resolves to `acme-lib::core::client`; the same name in
/// `acme-lib` resolves to `acme-lib::client`. Checks the current module
/// path and each ancestor up to the crate root.
fn own_module_prefix(
    name: &str,
    current: &[String],
    crate_name: &str,
    known: &BTreeSet<String>,
) -> Option<String> {
    for depth in (1..=current.len()).rev() {
        let candidate = format!("{}::{name}", current[..depth].join("::"));
        if known.contains(&candidate) {
            return Some(candidate);
        }
    }
    let candidate = format!("{crate_name}::{name}");
    if known.contains(&candidate) {
        return Some(candidate);
    }
    None
}

/// Normalize a crate name for dash/underscore-insensitive comparison and dedup
/// (`tower-http` == `tower_http`).
fn normalize_crate_name(name: &str) -> String {
    name.replace('-', "_")
}

/// True for the Rust standard-library crates, which are never external.
fn is_stdlib(name: &str) -> bool {
    matches!(name, "std" | "core" | "alloc")
}

/// Find the deepest known module prefix of `full`; the segments after it are the
/// imported items (symbols). `*` (glob) is not a specific item and is dropped.
fn target_module(full: &[String], known: &BTreeSet<String>) -> Option<(String, Vec<String>)> {
    for split in (1..=full.len()).rev() {
        let candidate = full[..split].join("::");
        if known.contains(&candidate) {
            let symbols: Vec<String> = full[split..]
                .iter()
                .filter(|segment| segment.as_str() != "*")
                .cloned()
                .collect();
            return Some((candidate, symbols));
        }
    }
    None
}

/// Parse every `.rs` source under the unit's directory with syn, so a syntax
/// error aborts extraction naming the failing file (contract: hard fail, no
/// partial model). The unit path is relative to the workspace root (`.` for the
/// root crate).
fn parse_unit_sources(root: &Path, unit: &Unit) -> Result<(), String> {
    let unit_dir = unit_dir_for(root, unit);
    let files = collect_rs_files(root, &unit_dir);
    for path in files {
        let source = std::fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        syn::parse_file(&source)
            .map_err(|_| format!("failed to parse source: {}", path.display()))?;
    }
    Ok(())
}

/// Collect every `.rs` file under `dir`, excluding dot-entries, `target`, and
/// `vendor`, in deterministic (sorted) order.
fn collect_rs_files(root: &Path, dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        let excluded = rel.components().any(|component| {
            let name = component.as_os_str().to_string_lossy();
            name.starts_with('.') || name == "target" || name == "vendor"
        });
        if excluded {
            continue;
        }
        if entry.path().extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        files.push(entry.path().to_path_buf());
    }
    files.sort();
    files
}

fn unit_dir_for(root: &Path, unit: &Unit) -> PathBuf {
    if unit.path == "." {
        root.to_path_buf()
    } else {
        root.join(&unit.path)
    }
}

/// Resolve the real crate name of a dependency. For `name = { package = "real" }`
/// the key is an alias and the crate is `real`; otherwise the key is the crate.
/// A `{ workspace = true }` dep carries no `package`, so its real crate name is
/// resolved from the workspace root's `[workspace.dependencies]` entry for `key`
/// (which may itself rename via `package = "..."`). When the workspace table is
/// absent or lacks the key, the key is the crate name.
fn dependency_real_name(key: &str, value: &Value, workspace_deps: Option<&Value>) -> String {
    if value.get("workspace").and_then(Value::as_bool) == Some(true) {
        if let Some(entry) = workspace_deps.and_then(|deps| deps.get(key)) {
            return entry
                .get("package")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_else(|| key.to_string());
        }
        return key.to_string();
    }
    value
        .get("package")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| key.to_string())
}

fn units_for(
    root_manifest: &Value,
    root: &Path,
    _root_manifest_path: &Path,
) -> Result<Vec<Unit>, String> {
    let members: Vec<String> = if let Some(workspace) = root_manifest.get("workspace") {
        let declared = workspace
            .get("members")
            .and_then(Value::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut members = expand_globs(root, &declared);
        if root_manifest.get("package").is_some() {
            members.push(".".to_string());
        }
        members.sort();
        members.dedup();
        members
    } else {
        vec![".".to_string()]
    };

    let mut units = Vec::new();
    for member in members {
        let manifest_path = manifest_for_unit(root, &member);
        let manifest = read_manifest(&manifest_path)?;
        let name = package_name(&manifest, root, &manifest_path)?;
        units.extend(units_for_package(root, &member, &name, &manifest)?);
    }
    Ok(units)
}

/// Enumerate every compilation target of one package as a separate unit:
/// the lib (`lib.rs`), the bin root (`main.rs`), and each `src/bin/*.rs` or
/// declared `[[bin]]`. Naming: the lib keeps the package name; `main.rs` is the
/// package name unless a lib also exists, in which case it is `<pkg>-bin` so the
/// two stay distinct in the model; a `src/bin/<tool>.rs` is `<tool>`. Targets
/// are deduplicated by file path: a declared `[[bin]]` wins over the implicit
/// `main.rs` discovery, so `<pkg>-bin` naming applies only when no declaration
/// points at `main.rs`.
fn units_for_package(
    root: &Path,
    member: &str,
    package_name: &str,
    manifest: &Value,
) -> Result<Vec<Unit>, String> {
    let unit_dir = if member == "." {
        root.to_path_buf()
    } else {
        root.join(member)
    };
    let src_dir = unit_dir.join("src");
    let crate_unit = |kind: &str, name: String, crate_ids: BTreeSet<String>, root_file: String| Unit {
        name,
        kind: kind.to_string(),
        path: member.to_string(),
        root: root_file,
        crate_ids,
    };

    let mut targets: Vec<Unit> = Vec::new();
    let mut seen_names: BTreeSet<String> = BTreeSet::new();
    let mut seen_paths: BTreeSet<String> = BTreeSet::new();
    let has_lib = src_dir.join("lib.rs").exists();

    // Explicit `[[bin]]` declarations are authoritative cargo targets. Collect
    // them up front so implicit discovery yields to a declaration pointing at
    // the same file: discovery dedupes on target file path, so a `[[bin]]` at
    // `src/main.rs` replaces the implicit `<pkg>-bin` unit instead of doubling
    // it (one model unit per cargo target).
    let declared_bins: Vec<(String, String)> = manifest
        .get("bin")
        .and_then(Value::as_array)
        .map(|bins| {
            bins.iter()
                .filter_map(|bin| {
                    let bin_name = bin.get("name").and_then(Value::as_str)?;
                    let path = bin
                        .get("path")
                        .and_then(Value::as_str)
                        .map(String::from)
                        .unwrap_or_else(|| format!("src/bin/{bin_name}.rs"));
                    let root_file = path
                        .strip_prefix("src/")
                        .map(String::from)
                        .unwrap_or_else(|| format!("bin/{bin_name}.rs"));
                    Some((bin_name.to_string(), root_file))
                })
                .collect()
        })
        .unwrap_or_default();
    let declared_paths: BTreeSet<String> =
        declared_bins.iter().map(|(_, path)| path.clone()).collect();

    if has_lib {
        // A lib target's crate id is `[lib] name` when present, else the package
        // name. Both the package name and any lib name are reference aliases.
        let lib_name = manifest
            .get("lib")
            .and_then(|lib| lib.get("name"))
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| package_name.to_string());
        let mut lib_ids = BTreeSet::new();
        lib_ids.insert(package_name.to_string());
        lib_ids.insert(lib_name);
        targets.push(crate_unit(
            "crate",
            package_name.to_string(),
            lib_ids,
            "lib.rs".to_string(),
        ));
        seen_names.insert(package_name.to_string());
        seen_paths.insert("lib.rs".to_string());
    }
    if src_dir.join("main.rs").exists() && !declared_paths.contains("main.rs") {
        let bin_name = if has_lib {
            format!("{package_name}-bin")
        } else {
            package_name.to_string()
        };
        if !seen_names.contains(&bin_name) && !seen_paths.contains("main.rs") {
            let mut bin_ids = BTreeSet::new();
            bin_ids.insert(bin_name.clone());
            targets.push(crate_unit("bin", bin_name.clone(), bin_ids, "main.rs".to_string()));
            seen_names.insert(bin_name);
            seen_paths.insert("main.rs".to_string());
        }
    }

    // Auto-discovered `src/bin/*.rs` targets.
    let bin_dir = src_dir.join("bin");
    if bin_dir.is_dir() {
        let mut bins: Vec<PathBuf> = std::fs::read_dir(&bin_dir)
            .map_err(|err| format!("failed to read {}: {err}", bin_dir.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
            .collect();
        bins.sort();
        for path in bins {
            let tool = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let root_file = format!("bin/{tool}.rs");
            if !seen_names.contains(&tool)
                && !seen_paths.contains(&root_file)
                && !declared_paths.contains(&root_file)
            {
                let mut bin_ids = BTreeSet::new();
                bin_ids.insert(tool.clone());
                targets.push(crate_unit("bin", tool.clone(), bin_ids, root_file.clone()));
                seen_names.insert(tool);
                seen_paths.insert(root_file);
            }
        }
    }

    // Explicit `[[bin]]` declarations (name may override the auto name); they
    // were already consulted above so a declaration wins over any implicit
    // target sharing its file path.
    for (bin_name, root_file) in declared_bins {
        if !seen_names.contains(&bin_name) && !seen_paths.contains(&root_file) {
            let mut bin_ids = BTreeSet::new();
            bin_ids.insert(bin_name.clone());
            targets.push(crate_unit("bin", bin_name.clone(), bin_ids, root_file.clone()));
            seen_names.insert(bin_name);
            seen_paths.insert(root_file);
        }
    }

    Ok(targets)
}

/// Expand `members` entries containing `*` (e.g. `crates/*`) to the directories
/// that actually exist under the workspace root and contain a `Cargo.toml`.
/// Only single-star globs in the final path segment are supported (cargo's
/// common `crates/*` form); anything else is kept as-is.
fn expand_globs(root: &Path, members: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for member in members {
        if !member.contains('*') {
            out.push(member.clone());
            continue;
        }
        let dir = member.rsplit_once('*').map(|(d, _)| d).unwrap_or(member);
        let dir = dir.trim_end_matches('/');
        let dir_path = root.join(dir);
        let globbed: Vec<String> = WalkDir::new(&dir_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_dir())
            .filter(|entry| entry.path().join("Cargo.toml").exists())
            .map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                format!("{dir}/{name}")
            })
            .collect();
        out.extend(globbed);
    }
    out
}

fn manifest_for_unit(root: &Path, member: &str) -> PathBuf {
    if member == "." {
        root.join("Cargo.toml")
    } else {
        root.join(member).join("Cargo.toml")
    }
}

fn package_name(manifest: &Value, root: &Path, manifest_path: &Path) -> Result<String, String> {
    manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| {
            format!(
                "failed to parse source: {}",
                display_relative(root, manifest_path)
            )
        })
}

fn read_manifest(path: &Path) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to parse source: {} ({err})", path.display()))?;
    toml::from_str(&raw)
        .map_err(|err| format!("failed to parse source: {} ({err})", path.display()))
}

fn display_relative(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => {
            let rel = relative.to_string_lossy().into_owned();
            if rel.is_empty() || rel == "Cargo.toml" {
                // The path sits directly under the root; keep the root visible so
                // the message reads as a location, not a bare file name.
                path.display().to_string()
            } else {
                rel
            }
        }
        Err(_) => path.display().to_string(),
    }
}
