use crate::archspec::model::{
    Edge, ManifestInfo, Model, ModuleDeclaration, ModuleEdge, Role, Unit,
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
/// `use` trees, and the dotted paths referenced in its item bodies.
type ModuleRefs = (Vec<String>, Vec<syn::UseTree>, BTreeSet<Vec<String>>);

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

    let manifest = manifest_info(&root_manifest, workspace_deps);

    Ok(Model {
        schema_version: 1,
        language: "rust".to_string(),
        units,
        edges,
        usage: Default::default(),
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
        roles,
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
            &root_ast.items,
            &mut state,
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
        state.soft.insert(main_path);
        for entry in state.uses.iter_mut() {
            if entry.0.len() == 1 && entry.0[0] == unit.name {
                entry.0.push("main".to_string());
            }
        }
    }

    // Second pass: resolve `use` trees and qualified paths against the module set.
    let mut edge_map: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut external: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut unit_edges: BTreeSet<(String, String)> = BTreeSet::new();
    for (current, trees, paths) in &state.uses {
        let source_dotted = current.join("::");
        for tree in trees {
            let mut paths = Vec::new();
            flatten_use_tree(tree, Vec::new(), &mut paths);
            for segments in paths {
                let first = segments[0].as_str();
                if is_reserved_segment(first) {
                    let full = resolve_module_path(&segments, current, &crate_name);
                    if let Some((target, symbols)) = target_module(&full, &state.known) {
                        if target != source_dotted {
                            edge_map
                                .entry((source_dotted.clone(), target))
                                .or_default()
                                .extend(symbols);
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
                        // the unit's OWN crate name stays internal.
                        if *real != crate_name {
                            unit_edges.insert((crate_name.clone(), real.clone()));
                        }
                        continue;
                    }
                    if let Some(own_path) =
                        own_module_prefix(first, current, &crate_name, &state.known)
                    {
                        // A bare reference to the crate's own module.
                        let full = resolve_bare_module(&segments, &own_path);
                        if let Some((target, symbols)) = target_module(&full, &state.known) {
                            if target != source_dotted {
                                edge_map
                                    .entry((source_dotted.clone(), target))
                                    .or_default()
                                    .extend(symbols);
                            }
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
                            .entry((source_dotted.clone(), target))
                            .or_default();
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
                        unit_edges.insert((crate_name.clone(), real.clone()));
                    }
                } else if let Some(own_path) =
                    own_module_prefix(first, current, &crate_name, &state.known)
                {
                    let target = own_path;
                    if target != source_dotted {
                        edge_map.entry((source_dotted.clone(), target)).or_default();
                    }
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
    }

    let mut edges: Vec<ModuleEdge> = edge_map
        .into_iter()
        .map(|((from, to), symbols)| {
            let mut symbols: Vec<String> = symbols.into_iter().collect();
            symbols.sort();
            ModuleEdge {
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
        root_defines_items,
    })
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
/// current module; a bare first segment resolves against the current module and
/// its ancestors, exactly like a plain path in that module.
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
            let owner = own_module_prefix(first, current, ctx.crate_name, ctx.known)?;
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
}

/// Recursively walk a module's items: record each declared sub-module (inline or
/// file-backed) in `state.soft`/`state.known`, collect its `use` trees and
/// qualified-path references for later resolution, and record file-backed
/// declarations that resolve to no parsed file in `state.unresolved`.
fn walk_module(
    path: &[String],
    mod_dir: &Path,
    file_dir: &Path,
    items: &[syn::Item],
    state: &mut WalkState,
) {
    let mut use_trees = Vec::new();
    let mut paths = BTreeSet::new();
    for item in items {
        if let syn::Item::Use(use_item) = item {
            use_trees.push(use_item.tree.clone());
        }
    }
    // Walk every item body (fn/impl/type positions) for path references.
    let mut visitor = PathVisitor { out: &mut paths };
    for item in items {
        visitor.visit_item(item);
    }
    state.uses.push((path.to_vec(), use_trees, paths));

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
            // against the current file's module directory.
            walk_module(&child_path, mod_dir, file_dir, inline_items, state);
            continue;
        }
        // File-backed `mod X;` — resolve an attributed target when declared,
        // else the conventional `<dir>/X.rs` / `<dir>/X/mod.rs` variants.
        // A declaration resolving to no parsed file is recorded in
        // `unresolved`: silently skipping it would hide its contents from
        // every check.
        let mut resolved = false;
        for (candidate, next_dir, next_file_dir) in
            module_file_candidates(&module.attrs, mod_dir, file_dir, &name)
        {
            if let Some(ast) = state.files.get(&candidate) {
                walk_module(&child_path, &next_dir, &next_file_dir, &ast.items, state);
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
        if attr.path().is_ident("path") {
            if let Some(target) = path_literal(&attr.meta) {
                return Some(AttributeTarget::Fixed(target));
            }
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

/// Resolve a bare (non `crate`/`self`/`super`) path whose first segment is an own
/// module of the crate into the crate-prefixed dotted space. `prefix` is the
/// dotted path of the own module resolved by `own_module_prefix` (crate-name
/// included), e.g. `acme-lib::core::client`.
fn resolve_bare_module(segments: &[String], prefix: &str) -> Vec<String> {
    let mut out: Vec<String> = prefix.split("::").map(String::from).collect();
    out.extend_from_slice(segments);
    out
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
