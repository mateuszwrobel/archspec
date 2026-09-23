use crate::archspec::inspect::FileGraph;
use crate::archspec::scan::csharp as cs_scan;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// File-level import graph for a C# tree. Mirrors the Rust file scanner
/// (`inspect/scanner.rs`): nodes are source files, edges are resolved
/// `using`-to-file edges. A `using A.B.C;` in file F resolves to the files
/// that declare namespace `A.B.C` (or its deepest declared namespace prefix);
/// unresolved (external) namespaces produce no edge, matching the Rust
/// scanner's treatment of external crates. C# has no file-level test marker,
/// so the target side mirrors the scan driver's project classification: a
/// file whose nearest-ancestor csproj is a test project per the scan
/// driver's existing `cs_scan::is_test_project` predicate owns no
/// namespace — production `using`s never fan into test-project files, while
/// those files stay nodes and import sources. Files are grouped into folder
/// subgraphs by the shared renderers.
pub fn scan(root: &Path) -> Result<FileGraph, String> {
    let files = collect_cs_files(root)?;
    if files.is_empty() {
        return Err(format!("no C# sources found under: {}", root.display()));
    }

    // Rel path -> (declared namespaces, using targets in that file).
    let mut file_data: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    // Namespace -> production files declaring it (for resolving `using` to a
    // file). Test-project files are filtered out at registration: they own
    // no namespace as an import target.
    let mut ns_files: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    // Dir -> test-tier classification of its nearest csproj, computed lazily.
    let mut project_tiers: BTreeMap<PathBuf, bool> = BTreeMap::new();

    for rel in &files {
        let full = root.join(rel);
        let source = std::fs::read_to_string(&full)
            .map_err(|err| format!("failed to read {}: {err}", full.display()))?;
        let mut namespaces = BTreeSet::new();
        let mut usings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        cs_scan::syntax::scan_cs_source(&source, &mut namespaces, &mut usings, &mut cs_scan::syntax::TypeFacts::default())
            .map_err(|err| format!("failed to parse {}: {err}", full.display()))?;
        // A file's own declared namespaces: the file owns them — unless the
        // file sits in a test project, which then contributes no target.
        let own_ns: BTreeSet<String> = namespaces.iter().cloned().collect();
        if !in_test_project(rel, root, &mut project_tiers) {
            for ns in &own_ns {
                ns_files
                    .entry(ns.clone())
                    .or_default()
                    .insert(rel.to_string_lossy().into_owned());
            }
        }
        // The file's using targets: all usings in the file, regardless of which
        // namespace block they appeared in.
        let mut targets: BTreeSet<String> = BTreeSet::new();
        for set in usings.values() {
            targets.extend(set.iter().cloned());
        }
        file_data.insert(rel.to_string_lossy().into_owned(), (own_ns, targets));
    }

    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for (from, (_namespaces, targets)) in &file_data {
        for target in targets {
            // Deepest declared namespace prefix wins — the shared longest-prefix
            // rule of the scan driver, so the two cannot drift.
            let resolved_ns = cs_scan::namespace_prefixes(target)
                .find(|candidate| ns_files.contains_key(*candidate));
            if let Some(resolved_ns) = resolved_ns {
                if let Some(owners) = ns_files.get(resolved_ns) {
                    for to in owners {
                        if to != from {
                            edges.insert((from.clone(), to.clone()));
                        }
                    }
                }
            }
        }
    }

    let nodes: BTreeSet<String> = files
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    Ok(FileGraph { nodes, edges })
}

/// Collect `.cs` files under `root`, excluding hidden, `bin`, `obj`, and
/// `node_modules` paths (matches `scan`'s exclusion set).
fn collect_cs_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
        if entry.depth() == 0 {
            return true;
        }
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        !cs_scan::is_excluded_cs(rel)
    });
    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("cs") {
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

/// The test-tier membership of a collected file: nearest-ancestor csproj
/// ownership — walking from the file's directory up to the root, the first
/// directory holding a `.csproj` owns the file (the first in sort order when
/// a directory holds several) — classified with the scan driver's existing
/// test-project predicate, so inspect and `scan` never disagree on what the
/// test tier is. A file with no csproj above it is production; trees whose
/// projects are all production classify identically and render unchanged.
fn in_test_project(
    rel: &Path,
    root: &Path,
    tiers: &mut BTreeMap<PathBuf, bool>,
) -> bool {
    let mut above: Vec<PathBuf> = Vec::new();
    let mut dir = rel.parent().unwrap_or(Path::new("")).to_path_buf();
    loop {
        if let Some(tier) = tiers.get(&dir) {
            let tier = *tier;
            for visited in &above {
                tiers.insert(visited.clone(), tier);
            }
            return tier;
        }
        let project_dir = if dir.as_os_str().is_empty() {
            root.to_path_buf()
        } else {
            root.join(&dir)
        };
        if let Some(csproj) = nearest_csproj(&project_dir) {
            let manifest = cs_scan::manifest_info(&project_dir, root, &csproj);
            let tier = cs_scan::is_test_project(&manifest);
            tiers.insert(dir, tier);
            for visited in &above {
                tiers.insert(visited.clone(), tier);
            }
            return tier;
        }
        above.push(dir.clone());
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => {
                for visited in &above {
                    tiers.insert(visited.clone(), false);
                }
                return false;
            }
        }
    }
}

/// The first `.csproj` file (sorted) directly in `dir`, if any.
fn nearest_csproj(dir: &Path) -> Option<PathBuf> {
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("csproj")
            {
                names.push(path.file_name()?.to_string_lossy().into_owned());
            }
        }
    }
    names.sort();
    names.first().map(|name| dir.join(name))
}
