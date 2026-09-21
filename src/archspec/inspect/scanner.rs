use crate::archspec::inspect::FileGraph;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};
use syn::UseTree;
use walkdir::WalkDir;

pub fn scan(root: &Path) -> Result<FileGraph, String> {
    let files = collect_rs_files(root)?;
    if files.is_empty() {
        return Err(format!("no Rust sources found under: {}", root.display()));
    }

    let mut ownership = build_ownership(root, &files);
    let own = own_index(root, &files);

    let mut parsed = Vec::new();
    for rel in &files {
        let full = root.join(rel);
        let source = std::fs::read_to_string(&full)
            .map_err(|err| format!("failed to read {}: {err}", full.display()))?;
        let syntax = syn::parse_file(&source)
            .map_err(|_| format!("failed to parse source: {}", full.display()))?;

        let crate_root = crate_root_for(root, rel);
        let importing_module = module_path(rel, &crate_root, root);

        for module in inline_modules(&importing_module, &syntax) {
            let key = (crate_root.clone(), module);
            ownership.entry(key).or_insert_with(|| rel.clone());
        }

        let mut collector = ImportCollector::new(importing_module, &own);
        collector.visit_file(&syntax);
        parsed.push((rel.clone(), crate_root, collector.targets));
    }

    let mut edges = BTreeSet::new();
    for (rel, crate_root, targets) in parsed {
        let from = rel.to_string_lossy().into_owned();
        for target in targets {
            let resolved = match &target {
                Target::Relative(path) => resolve_target(&crate_root, path, &ownership),
                Target::Own(candidates) => {
                    candidates.iter().find_map(|(target_root, path)| {
                        resolve_target(target_root, path, &ownership)
                    })
                }
                Target::Declared(path) => {
                    ownership.get(&(crate_root.clone(), path.clone())).cloned()
                }
            };
            if let Some(target) = resolved {
                let to = target.to_string_lossy().into_owned();
                if from != to {
                    edges.insert((from.clone(), to));
                }
            }
        }
    }

    let nodes = files
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    Ok(FileGraph { nodes, edges })
}

/// True when any path component (relative to `root`) is `target`, `vendor`, or
/// starts with `.`. Matches `scan`'s exclusion rule so both commands see the
/// same source tree.
fn is_excluded(root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name.starts_with('.') || name == "target" || name == "vendor"
    })
}

fn collect_rs_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let walker = WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_excluded(root, entry.path()));
    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let rel = path.strip_prefix(root).map_err(|err| {
            format!(
                "failed to resolve relative path for {}: {err}",
                path.display()
            )
        })?;
        files.push(rel.to_path_buf());
    }
    files.sort();
    Ok(files)
}

fn build_ownership(root: &Path, files: &[PathBuf]) -> BTreeMap<(PathBuf, Vec<String>), PathBuf> {
    let mut map = BTreeMap::new();
    for rel in files {
        let crate_root = crate_root_for(root, rel);
        let key = (crate_root.clone(), module_path(rel, &crate_root, root));
        map.insert(key, rel.clone());
    }
    map
}

fn crate_root_for(_root: &Path, rel: &Path) -> PathBuf {
    let components: Vec<_> = rel.components().collect();
    if components.len() > 1 && components[0].as_os_str() == "src" {
        return PathBuf::from("src");
    }
    PathBuf::new()
}

fn module_path(rel: &Path, crate_root: &Path, root: &Path) -> Vec<String> {
    let rel_from_root = rel.strip_prefix(root).unwrap_or(rel);
    let rel_from_crate = rel_from_root
        .strip_prefix(crate_root)
        .unwrap_or(rel_from_root);
    let comps: Vec<String> = rel_from_crate
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();

    if comps.is_empty() {
        return Vec::new();
    }

    let stem = Path::new(comps.last().unwrap())
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    let dirs = &comps[..comps.len() - 1];

    if stem == "mod" {
        return dirs.to_vec();
    }
    if dirs.is_empty() && (stem == "lib" || stem == "main") {
        return Vec::new();
    }

    let mut out = dirs.to_vec();
    out.push(stem);
    out
}

fn inline_modules(module_path: &[String], syntax: &syn::File) -> Vec<Vec<String>> {
    let mut modules = Vec::new();
    let mut path = module_path.to_vec();
    collect_inline_mods(&syntax.items, &mut path, &mut modules);
    modules
}

fn collect_inline_mods(items: &[syn::Item], path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let Some((_, items)) = &module.content else {
            continue;
        };
        path.push(module.ident.to_string());
        out.push(path.clone());
        collect_inline_mods(items, path, out);
        path.pop();
    }
}

fn resolve_target(
    crate_root: &Path,
    module_path: &[String],
    ownership: &BTreeMap<(PathBuf, Vec<String>), PathBuf>,
) -> Option<PathBuf> {
    let mut len = module_path.len();
    while len > 0 {
        if let Some(file) = ownership.get(&(crate_root.to_path_buf(), module_path[..len].to_vec()))
        {
            return Some(file.clone());
        }
        len -= 1;
    }
    None
}

/// A collected import target: either a module path relative to the importing
/// file's crate root (`crate::`, `self::`, `super::`), or a set of candidate
/// `(crate root, module path)` pairs reached through one of the package's own
/// cargo target names. Own-name candidates are ordered lib-first so root-name
/// collisions resolve toward the library deterministically. `Declared` is a
/// `mod x;` declaration's child module path: it resolves by exact ownership
/// lookup (file vs dir/mod.rs form), never by prefix fallback, so unresolved
/// declarations emit nothing.
#[derive(Eq, PartialEq, PartialOrd, Ord)]
enum Target {
    Relative(Vec<String>),
    Own(Vec<(PathBuf, Vec<String>)>),
    Declared(Vec<String>),
}

/// Own-crate-name index: normalized cargo target name → candidate module roots
/// in priority order, each a crate root directory plus the module path of the
/// target's root file. Names come only from the package manifest and cargo's
/// target-discovery layout, never from directory names.
type OwnIndex = BTreeMap<String, Vec<(PathBuf, Vec<String>)>>;

fn normalize_target_name(name: &str) -> String {
    name.replace('-', "_")
}

fn own_index(root: &Path, files: &[PathBuf]) -> OwnIndex {
    let mut index = OwnIndex::new();
    let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        return index;
    };
    let Ok(manifest) = text.parse::<toml::Value>() else {
        return index;
    };
    let Some(package_name) = manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
    else {
        return index;
    };

    // Cargo targets in resolution priority order: the lib first, so an
    // own-name import resolves into the library root when names collide.
    let mut targets: Vec<(BTreeSet<String>, PathBuf)> = Vec::new();
    if files.iter().any(|rel| rel == Path::new("src/lib.rs")) {
        let mut aliases = BTreeSet::new();
        aliases.insert(normalize_target_name(package_name));
        if let Some(lib_name) = manifest
            .get("lib")
            .and_then(|lib| lib.get("name"))
            .and_then(toml::Value::as_str)
        {
            aliases.insert(normalize_target_name(lib_name));
        }
        targets.push((aliases, PathBuf::from("src/lib.rs")));
    }

    // Bin targets: declared `[[bin]]` entries plus cargo's standard discovery
    // (`src/bin/*.rs` stems and the implicit `src/main.rs` bin under the
    // package name).
    let mut bins: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut declared_paths: BTreeSet<PathBuf> = BTreeSet::new();
    if let Some(declared) = manifest.get("bin").and_then(toml::Value::as_array) {
        let mut entries = Vec::new();
        for bin in declared {
            let Some(name) = bin.get("name").and_then(toml::Value::as_str) else {
                continue;
            };
            let default = format!("src/bin/{name}.rs");
            let path = bin
                .get("path")
                .and_then(toml::Value::as_str)
                .unwrap_or(default.as_str());
            entries.push((normalize_target_name(name), PathBuf::from(path)));
        }
        for (alias, path) in &entries {
            declared_paths.insert(path.clone());
            bins.entry(alias.clone()).or_insert_with(|| path.clone());
        }
    }
    for rel in files {
        let Some(in_bin) = rel.strip_prefix("src/bin").ok() else {
            continue;
        };
        if in_bin.components().count() != 1 || declared_paths.contains(rel) {
            continue;
        }
        if let Some(stem) = in_bin.file_stem().and_then(|stem| stem.to_str()) {
            bins.entry(normalize_target_name(stem))
                .or_insert_with(|| rel.clone());
        }
    }
    if files.iter().any(|rel| rel == Path::new("src/main.rs")) {
        bins.entry(normalize_target_name(package_name))
            .or_insert_with(|| PathBuf::from("src/main.rs"));
    }
    targets.extend(bins.into_iter().map(|(alias, file)| {
        let mut aliases = BTreeSet::new();
        aliases.insert(alias);
        (aliases, file)
    }));

    for (aliases, root_file) in targets {
        let crate_root = crate_root_for(root, &root_file);
        let prefix = module_path(&root_file, &crate_root, root);
        for alias in aliases {
            let candidates = index.entry(alias).or_default();
            let candidate = (crate_root.clone(), prefix.clone());
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    index
}

struct ImportCollector<'a> {
    /// Module path of the current scope: the file's own module path plus the
    /// chain of enclosing inline `mod` declarations. Used to resolve `super::`
    /// and `self::` relative to the inline module the import lives in.
    scope: Vec<String>,
    /// Cargo target names of the scanned package, for own-name resolution.
    own: &'a OwnIndex,
    targets: BTreeSet<Target>,
}

impl<'a> ImportCollector<'a> {
    fn new(importing_module: Vec<String>, own: &'a OwnIndex) -> Self {
        Self {
            scope: importing_module,
            own,
            targets: BTreeSet::new(),
        }
    }

    fn current_module(&self) -> &[String] {
        &self.scope
    }

    fn record_path_segments(&mut self, segments: &[String]) {
        let scope = self.current_module();
        match segments.first().map(String::as_str) {
            Some("crate") if segments.len() > 1 => {
                self.targets
                    .insert(Target::Relative(segments[1..].to_vec()));
            }
            Some("super") => {
                let levels = segments
                    .iter()
                    .take_while(|segment| segment.as_str() == "super")
                    .count();
                if levels > scope.len() {
                    return;
                }
                if levels < segments.len() {
                    let mut target = scope[..scope.len() - levels].to_vec();
                    target.extend_from_slice(&segments[levels..]);
                    self.targets.insert(Target::Relative(target));
                }
            }
            Some("self") if segments.len() > 1 => {
                let mut target = scope.to_vec();
                target.extend_from_slice(&segments[1..]);
                self.targets.insert(Target::Relative(target));
            }
            Some(first) if segments.len() > 1 => {
                // An import written through one of the package's own cargo
                // target names refers in-crate; resolve it through that
                // target's module root instead of dropping it as external.
                if let Some(candidates) = self.own.get(first) {
                    let paths = candidates
                        .iter()
                        .map(|(target_root, prefix)| {
                            let mut path = prefix.clone();
                            path.extend_from_slice(&segments[1..]);
                            (target_root.clone(), path)
                        })
                        .collect();
                    self.targets.insert(Target::Own(paths));
                }
            }
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for ImportCollector<'_> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        self.record_path_segments(&rust_arch_test_kit::collector::path_segments(path));
        visit::visit_path(self, path);
    }

    fn visit_use_tree(&mut self, use_tree: &'ast UseTree) {
        let mut paths = Vec::new();
        rust_arch_test_kit::collector::flatten_use_tree(use_tree, Vec::new(), &mut paths);
        for segments in paths {
            self.record_path_segments(&segments);
        }
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let Some((_, items)) = &module.content else {
            let mut target = self.scope.clone();
            target.push(module.ident.to_string());
            self.targets.insert(Target::Declared(target));
            return;
        };
        self.scope.push(module.ident.to_string());
        for item in items {
            self.visit_item(item);
        }
        self.scope.pop();
    }
}
