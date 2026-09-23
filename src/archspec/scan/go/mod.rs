pub(crate) mod syntax;

use crate::archspec::model::{Edge, Model, ModuleEdge, Role, Unit};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// go extraction (grammar source facts). go.mod discovery, module-path parsing,
/// package-directory discovery, the `*_test.go` exclusion and the
/// stdlib/external attribution rules are the shared graph layer; per file the
/// grammar `import_spec` walk in [`syntax`] provides the import facts, import
/// qualifiers and selector symbols that feed the edge assembly below.
///
/// The single-module tree projects every intra-module package import onto the
/// module tier using the same `dotted_module` addressing the go.work edges
/// emit (decision "Go visibility: every package of a module is a module").
///
/// A `go.work` naming 2+ members takes [`extract_workspace`]: member discovery,
/// unit naming and unit-tier edges as above, selector symbols onto the
/// cross-member edges, and within-member package imports projected exactly
/// like the single-module path.
pub fn extract(root: &Path) -> Result<Model, String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let members = workspace_members(&root)?;
    if members.len() >= 2 {
        // A go.work naming several member modules is a module fact the model
        // can state: the module tier mirrors the csharp shape (group ->
        // members) with member modules as the groups and their packages as
        // the members.
        return extract_workspace(&root, &members);
    }
    // No go.work (or a degenerate one with a single member): the single-module
    // path.
    extract_single_module(&root)
}

/// The single-module path: units are the discovered packages, hard edges are
/// imports matching a unit name, and every such intra-module import
/// additionally projects onto the module tier (see [`extract`]).
fn extract_single_module(root: &Path) -> Result<Model, String> {
    let module = module_path(root)?;

    let packages = collect_packages(root);

    if packages.is_empty() {
        return Err(format!("no Go packages found under: {}", root.display()));
    }

    let mut units = Vec::new();
    for rel_dir in packages.keys() {
        let name = package_name(&module, rel_dir);
        units.push(Unit {
            name: name.clone(),
            kind: "package".to_string(),
            path: rel_dir.clone(),
            root: String::new(),
            crate_ids: BTreeSet::from([name]),
        });
    }
    units.sort_by(|left, right| left.name.cmp(&right.name));

    // Only intra-module imports can be edges: an import matching a unit name.
    let unit_names: BTreeSet<&str> = units.iter().map(|unit| unit.name.as_str()).collect();

    let mut edge_set: BTreeSet<(String, String)> = BTreeSet::new();
    let mut edge_symbols: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut external_set: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut roles: BTreeMap<String, Role> = BTreeMap::new();
    let module_prefix = format!("{module}/");
    for (rel_dir, files) in &packages {
        let from = package_name(&module, rel_dir);
        for file in files {
            // The grammar pass yields the import paths, the alias map and the
            // file's exported selector facts (US 12) in one walk
            // ([`syntax::scan_go_file`]).
            let facts = syntax::scan_go_file(file)?;
            let (imports, qualifiers, selectors) =
                (facts.imports, facts.qualifiers, facts.selectors);
            // Roles from this driver's own facts: a unit whose `package_clause`
            // is `main` is the tree's composition root — the place where
            // cross-part wiring legally lives — and gains `composition` keyed
            // at its unit path (the directory-derived import path, the same
            // identity the edges and the update seed use). The packages the
            // main binary links gain nothing beyond that. The facade role has
            // no derivable go predicate, so it is stated nowhere: absence in
            // the roles map is the honest truth, not an oversight.
            if facts.package.as_deref() == Some("main") {
                roles.insert(from.clone(), Role::Composition);
            }
            for import in imports {
                if import == module || import.starts_with(&module_prefix) {
                    // Internal to the module: an edge only when the target
                    // package is a discovered unit (empty/undiscovered dirs
                    // contribute nothing). The `import != from` clause also
                    // rules out the self-import module edge (impossible in
                    // valid Go, guarded anyway).
                    if unit_names.contains(import.as_str()) && import != from {
                        let key = (from.clone(), import.clone());
                        edge_set.insert(key.clone());
                        // Selector symbols join the edge THIS import created
                        // (workplan decision "Symbol vocabulary"): the file's
                        // qualifier for this path selects which recorded
                        // selector facts count; `_`/`.` qualifiers are not
                        // referenceable (see `syntax` docs). Multiple files
                        // and call sites merge into one deduplicated set per
                        // (from, to) — the edge itself still exists once.
                        if let Some(symbols) = edge_selector_symbols(&qualifiers, &selectors, &key.1)
                        {
                            edge_symbols.entry(key).or_default().extend(symbols.iter().cloned());
                        }
                    }
                    continue;
                }
                if is_stdlib(&import) {
                    continue;
                }
                // External tier stays purely import-driven:
                // selectors qualified by external or stdlib packages add
                // nothing module-side.
                external_set.insert(import.clone());
                module_external
                    .entry(from.clone())
                    .or_default()
                    .insert(import.clone());
            }
        }
    }
    let edges: Vec<Edge> = edge_set
        .iter()
        .map(|(from, to)| Edge {
            from: from.clone(),
            to: to.clone(),
        })
        .collect();
    let external: Vec<String> = external_set.into_iter().collect();
    let module_external: BTreeMap<String, Vec<String>> = module_external
        .into_iter()
        .map(|(unit, crates)| (unit, crates.into_iter().collect()))
        .collect();

    // Module tier (decision "Go visibility: every package of a module is a
    // module"): `edge_set` already holds exactly the discovered,
    // non-self sibling-package imports, so each unit edge projects 1:1
    // onto a module edge — endpoints through `dotted_module` (the go.work
    // cross-member vocabulary), `unit` = the importing module, symbols = the
    // exported selector names merged onto that (from, to) key (US 12; the
    // BTreeSet order is sorted and deduplicated, the edge exists once
    // regardless of how many selector call sites fed it). BTreeSet iteration
    // is sorted by (from, to) and the unit is constant, so the (unit, from,
    // to) order the workspace path uses holds here too.
    let module_edges: Vec<ModuleEdge> = edge_set
        .iter()
        .map(|(from, to)| ModuleEdge {
            unit: module.clone(),
            from: dotted_module(from),
            to: dotted_module(to),
            symbols: edge_symbols
                .get(&(from.clone(), to.clone()))
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default(),
        })
        .collect();

    Ok(Model {
        schema_version: 1,
        language: "go".to_string(),
        units,
        edges,
        usage: Default::default(),
        soft_structure: Default::default(),
        external,
        module_edges,
        manifest: Default::default(),
        root_public_exports: Default::default(),
        root_glob_exports: Default::default(),
        root_empty_glob_exports: Default::default(),
        module_external,
        root_module_declarations: Default::default(),
        unit_manifests: Default::default(),
        unresolved_module_files: Default::default(),
        test_gated_modules: Default::default(),
        roles,
    })
}

/// The module-tier members a root `go.work` declares: every `use` entry
/// resolved to its member directory and module path (sorted, deduplicated by
/// directory). An absent go.work contributes no members — every other code
/// path behaves exactly as a single-module tree.
pub(crate) fn workspace_members(root: &Path) -> Result<Vec<(String, String)>, String> {
    let work_file = root.join("go.work");
    if !work_file.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&work_file)
        .map_err(|err| format!("failed to read {}: {err}", work_file.display()))?;
    let mut members: BTreeMap<String, String> = BTreeMap::new();
    for dir in work_use_entries(&raw) {
        let module = module_path(&root.join(&dir))?;
        members.insert(dir, module);
    }
    Ok(members.into_iter().collect())
}

/// The `use` entries of a go.work file (source-level, no toolchain): block
/// form `use (\n\t./dir\n)` with one entry per line and one-line form
/// `use ./dir`; `//` comments stripped. Pure line parse mirroring
/// `module_path`'s style.
fn work_use_entries(raw: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut in_block = false;
    for line in raw.lines() {
        let line = line.trim();
        let line = line.split("//").next().unwrap_or(line).trim();
        if in_block {
            if line == ")" {
                in_block = false;
            } else if !line.is_empty() {
                entries.push(normalize_use_dir(line));
            }
            continue;
        }
        if matches_word(line, "use") {
            let rest = line["use".len()..].trim();
            if rest.starts_with('(') {
                in_block = true;
            } else if !rest.is_empty() {
                entries.push(normalize_use_dir(rest));
            }
        }
    }
    entries
}

/// True when `line` starts with `word` at a word boundary.
fn matches_word(line: &str, word: &str) -> bool {
    let Some(after) = line.strip_prefix(word) else {
        return false;
    };
    after
        .chars()
        .next()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_')
}

/// Normalize a go.work `use` path relative to the workspace root: quotes and
/// a trailing slash dropped, `./x` collapsed to `x`, `.` kept as `.`.
fn normalize_use_dir(entry: &str) -> String {
    let entry = entry.trim().trim_matches('"').trim_end_matches('/');
    let stripped = entry.strip_prefix("./").unwrap_or(entry);
    if stripped.is_empty() {
        ".".to_string()
    } else {
        stripped.to_string()
    }
}

/// Extraction for a multi-member `go.work` workspace. Units are every member
/// package named through its own member module path (hard edges between units
/// work exactly as in a single module, across members included). The module
/// tier is filled the way csharp fills projects and namespaces: each member
/// module maps under `soft_structure` to its packages as `::` module paths,
/// and every real import between packages of different members emits a module
/// edge owned by the importing member. Packages outside every member dir
/// belong to no module and contribute nothing — they are unbuildable from the
/// workspace either.
///
/// The per-file import facts come from the grammar — the paths plus the
/// qualifier map and selector facts — so cross-member module edges carry the
/// exported selector symbols, and within-member package imports project module
/// edges exactly like the single-module path (decision "Go visibility: every
/// package of a module is a module" applies per member here). The unit-tier
/// edges, units, external tier and `soft_structure` are graph-layer facts.
fn extract_workspace(
    root: &Path,
    members: &[(String, String)],
) -> Result<Model, String> {
    let packages = collect_packages(root);
    if packages.is_empty() {
        return Err(format!("no Go packages found under: {}", root.display()));
    }

    // A dir belongs to the member with the longest matching directory prefix;
    // a `.` member (the root module itself) matches whatever no other claims.
    let owner_of_dir = |rel_dir: &str| -> Option<usize> {
        let mut best: Option<(usize, usize)> = None;
        for (index, (dir, _)) in members.iter().enumerate() {
            let matches = match dir.as_str() {
                "." => true,
                dir => rel_dir == dir || rel_dir.starts_with(&format!("{dir}/")),
            };
            if matches && best.is_none_or(|(_, len)| dir.len() > len) {
                best = Some((index, dir.len()));
            }
        }
        best.map(|(index, _)| index)
    };
    let rel_in_member = |rel_dir: &str, dir: &str| -> String {
        if dir == "." {
            rel_dir.to_string()
        } else if rel_dir == dir {
            ".".to_string()
        } else {
            rel_dir[dir.len() + 1..].to_string()
        }
    };

    let mut units = Vec::new();
    let mut unit_for_dir: BTreeMap<String, String> = BTreeMap::new();
    for rel_dir in packages.keys() {
        let Some(index) = owner_of_dir(rel_dir) else {
            continue;
        };
        let (dir, module) = &members[index];
        let name = package_name(module, &rel_in_member(rel_dir, dir));
        unit_for_dir.insert(rel_dir.clone(), name.clone());
        units.push(Unit {
            name: name.clone(),
            kind: "package".to_string(),
            path: rel_dir.clone(),
            root: String::new(),
            crate_ids: BTreeSet::from([name]),
        });
    }
    units.sort_by(|left, right| left.name.cmp(&right.name));

    // The member module owning a unit name: longest module-path prefix wins
    // (every unit was built from a member, so an owner always exists).
    let module_owner = |unit: &str| -> Option<&str> {
        members
            .iter()
            .filter(|(_, module)| {
                unit == module.as_str() || unit.starts_with(&format!("{module}/"))
            })
            .max_by_key(|(_, module)| module.len())
            .map(|(_, module)| module.as_str())
    };

    let unit_names: BTreeSet<&str> = units.iter().map(|unit| unit.name.as_str()).collect();

    let mut edge_set: BTreeSet<(String, String)> = BTreeSet::new();
    let mut edge_symbols: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut external_set: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut roles: BTreeMap<String, Role> = BTreeMap::new();
    for (rel_dir, files) in &packages {
        let Some(from) = unit_for_dir.get(rel_dir) else {
            continue;
        };
        for file in files {
            // Same per-file fact source as the single-module path: one
            // grammar walk yields paths plus the qualifier map and selector
            // facts (US 13).
            let facts = syntax::scan_go_file(file)?;
            let (imports, qualifiers, selectors) =
                (facts.imports, facts.qualifiers, facts.selectors);
            // Roles derive per member exactly like the single-module path:
            // every main package of every member gains `composition` at its
            // own module path; members without a main package state no role,
            // and the facade role stays underived (no go predicate exists).
            if facts.package.as_deref() == Some("main") {
                roles.insert(from.clone(), Role::Composition);
            }
            for import in imports {
                let internal = members
                    .iter()
                    .any(|(_, module)| import == *module || import.starts_with(&format!("{module}/")));
                if internal {
                    if unit_names.contains(import.as_str()) && import != *from {
                        // Selector symbols are keyed on the (from, to) pair
                        // regardless of member boundaries — the module-tier
                        // arm below reads them for cross-member AND
                        // within-member edges.
                        let key = (from.clone(), import);
                        edge_set.insert(key.clone());
                        if let Some(symbols) =
                            edge_selector_symbols(&qualifiers, &selectors, &key.1)
                        {
                            edge_symbols
                                .entry(key)
                                .or_default()
                                .extend(symbols.iter().cloned());
                        }
                    }
                    continue;
                }
                if is_stdlib(&import) {
                    continue;
                }
                external_set.insert(import.clone());
                module_external
                    .entry(from.clone())
                    .or_default()
                    .insert(import.clone());
            }
        }
    }
    let edges: Vec<Edge> = edge_set
        .iter()
        .cloned()
        .map(|(from, to)| Edge { from, to })
        .collect();
    let external: Vec<String> = external_set.into_iter().collect();
    let module_external: BTreeMap<String, Vec<String>> = module_external
        .into_iter()
        .map(|(unit, crates)| (unit, crates.into_iter().collect()))
        .collect();

    // Module tier: member module -> its packages as `::` module paths.
    let mut soft_structure: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_dir, module) in members {
        let mut paths: Vec<String> = units
            .iter()
            .filter(|unit| module_owner(&unit.name) == Some(module.as_str()))
            .map(|unit| dotted_module(&unit.name))
            .collect();
        paths.sort();
        if !paths.is_empty() {
            soft_structure.insert(module.clone(), paths);
        }
    }

    // Module tier projection: cross-member AND within-member imports project
    // (decision "Go visibility: every package of a module is a module" applies
    // per member). Endpoints use the `dotted_module` vocabulary and
    // `unit` = importing member; symbols ride the cross-member AND
    // within-member edges (same merge rule as the single-module path).
    let mut module_edges: Vec<ModuleEdge> = Vec::new();
    for (from, to) in &edge_set {
        let (Some(from_member), Some(_)) = (module_owner(from), module_owner(to)) else {
            // Both endpoints must belong to a member module: packages outside
            // every member dir contribute no module edge.
            continue;
        };
        module_edges.push(ModuleEdge {
            unit: from_member.to_string(),
            from: dotted_module(from),
            to: dotted_module(to),
            symbols: edge_symbols
                .get(&(from.clone(), to.clone()))
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default(),
        });
    }
    module_edges.sort_by(|left, right| {
        (&left.unit, &left.from, &left.to).cmp(&(&right.unit, &right.from, &right.to))
    });

    Ok(Model {
        schema_version: 1,
        language: "go".to_string(),
        units,
        edges,
        usage: Default::default(),
        soft_structure,
        external,
        module_edges,
        manifest: Default::default(),
        root_public_exports: Default::default(),
        root_glob_exports: Default::default(),
        root_empty_glob_exports: Default::default(),
        module_external,
        root_module_declarations: Default::default(),
        unit_manifests: Default::default(),
        unresolved_module_files: Default::default(),
        test_gated_modules: Default::default(),
        roles,
    })
}

/// The `::` module path of a member package in the module tier: the package's
/// import path with `/` -> `::`. The first segment group identifies the unit
/// (host + member module), the second is the member's name, so the shared
/// `::` projections fold member packages onto member nodes — the same
/// unit-prefixed shape rust crates and csharp namespaces use.
fn dotted_module(unit: &str) -> String {
    unit.replace('/', "::")
}

/// A Go standard-library import: its first path segment has no dot. Third-party
/// paths always carry a dotted first segment (`modernc.org/x`, `gopkg.in/y`).
fn is_stdlib(import: &str) -> bool {
    match import.split('/').next() {
        Some(first) => first.is_empty() || !first.contains('.'),
        None => true,
    }
}

/// The module path from `go.mod` (e.g. `module example.com/demo`).
pub(crate) fn module_path(root: &Path) -> Result<String, String> {
    let mod_file = root.join("go.mod");
    let raw = std::fs::read_to_string(&mod_file)
        .map_err(|err| format!("failed to read {}: {err}", mod_file.display()))?;
    for line in raw.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("module") else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }
        // Strip a trailing `//` comment.
        let module = rest.split("//").next().unwrap_or(rest).trim();
        if !module.is_empty() {
            return Ok(module.to_string());
        }
    }
    Err(format!(
        "no module declaration found in {}",
        mod_file.display()
    ))
}

/// Map package dir (rel to root) -> sorted production .go files in it. Skips
/// dirs with no production .go files and skips `vendor/` and hidden dirs.
/// `*_test.go` files are the Go test tier and contribute no production
/// evidence (see `is_test_file`).
fn collect_packages(root: &Path) -> BTreeMap<String, Vec<PathBuf>> {
    let mut packages: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("go") {
            continue;
        }
        if is_test_file(path) {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        if is_excluded_go(rel) {
            continue;
        }
        let dir = match rel.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().into_owned(),
            _ => ".".to_string(),
        };
        packages.entry(dir).or_default().push(path.to_path_buf());
    }
    for files in packages.values_mut() {
        files.sort();
    }
    packages
}

/// The Go driver's exclusion predicate: a path is excluded when any component
/// is hidden (dot-prefixed), `vendor`, or `testdata`. Shared by every walk of
/// a Go tree (the scan collectors and the file-level inspector) so the node
/// sets cannot drift — the csharp `is_excluded_cs` precedent.
pub(crate) fn is_excluded_go(rel: &Path) -> bool {
    rel.components().filter_map(|component| component.as_os_str().to_str()).any(
        |component| component.starts_with('.') || component == "vendor" || component == "testdata",
    )
}

/// The symbols one file's selector uses contribute to the edge created by
/// `import`: the file's qualifier for that path resolves into the file's raw
/// selector facts (US 12). The blank (`_`) and dot (`.`) qualifiers are not
/// referenceable — a blank import binds no name, a dot import's exported
/// names are bare identifiers indistinguishable from local ones — so imports
/// under those qualifiers keep their edges symbolless.
fn edge_selector_symbols<'a>(
    qualifiers: &BTreeMap<String, String>,
    selectors: &'a BTreeMap<String, BTreeSet<String>>,
    import: &str,
) -> Option<&'a BTreeSet<String>> {
    let qualifier = qualifiers.get(import)?;
    if qualifier == "_" || qualifier == "." {
        return None;
    }
    selectors.get(qualifier)
}

/// The module paths a tree's imports resolve against — the internal/external
/// contract every Go consumer (scan and inspect alike) must agree on: the
/// `go.work` members when a root go.work names two or more, otherwise the
/// single module from the root `go.mod` (mirrored as the `.` member).
pub(crate) fn module_members(root: &Path) -> Result<Vec<(String, String)>, String> {
    let members = workspace_members(root)?;
    if members.len() >= 2 {
        return Ok(members);
    }
    Ok(vec![(".".to_string(), module_path(root)?)])
}

/// True for Go test files: any `*_test.go` file, in either the `package foo`
/// or `package foo_test` form — Go's own definition of the test tier. Such
/// files compile only for `go test`, so their imports are not production
/// dependencies: dropping them here keeps them out of units, edges, external
/// and `module_external` entirely, the file-tier mirror of the rust driver's
/// `cfg(test)` exclusion. The rust contract's model fact (`test_gated_modules`)
/// is module-path data kept off the serialized model (`#[serde(skip)]`,
/// consumed by `verify`/`update` through `Model::is_test_gated`); Go has no
/// module tier and its units list no files, so the honest mirror is exclusion
/// at the scan with no serialized fact — a test-only import simply does not
/// exist in the model, exactly like a rust integration test under `tests/`,
/// and the JSON shape is unchanged for every tree.
pub(crate) fn is_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("_test.go"))
}

/// Full package import path for a rel dir: `module/rel` (or `module` for `.`).
pub(crate) fn package_name(module: &str, rel_dir: &str) -> String {
    if rel_dir == "." {
        module.to_string()
    } else {
        format!("{module}/{rel_dir}")
    }
}
