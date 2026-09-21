use crate::archspec::inspect::FileGraph;
use crate::archspec::scan::go as go_scan;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// File-level import graph for a Go tree. Mirrors the C# file scanner
/// (`inspect/csharp.rs`): nodes are source files, edges are resolved
/// import-to-file edges. A `import "example.com/demo/store"` in file F
/// resolves to every PRODUCTION file declaring package `store` — the package
/// clause, not the directory, declares ownership, and test files
/// (`*_test.go`, both `package foo` and `package foo_test` forms) are
/// filtered from the target list with the scan driver's own predicate: no
/// production import can compile against a test file, so a test file is
/// never an import target. Imports that resolve under no member module path
/// (stdlib, third-party, the `import "C"` cgo marker) produce no edge,
/// matching the rust/c# treatment of external targets; `//go:embed` is a
/// comment the tokenizer skips. Test files are nodes and import sources like
/// any other file — discovery deliberately exceeds the guard model, which
/// drops `*_test.go` at scan.
pub fn scan(root: &Path) -> Result<FileGraph, String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let members = go_scan::module_members(&root)?;
    let files = collect_go_files(&root)?;
    if files.is_empty() {
        return Err(format!("no Go sources found under: {}", root.display()));
    }

    // A dir belongs to the member with the longest matching directory prefix
    // (a `.` member matches whatever no other claims) — the workspace
    // resolution of `scan::go::extract_workspace`, so both agree on internal.
    let member_of_dir = |rel_dir: &str| -> Option<usize> {
        let mut best: Option<(usize, usize)> = None;
        for (index, member) in members.iter().enumerate() {
            let dir: &str = &member.0;
            let matches = if dir == "." {
                true
            } else {
                rel_dir == dir || rel_dir.starts_with(&format!("{dir}/"))
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

    let mut dirs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut clauses: BTreeMap<String, String> = BTreeMap::new();
    let mut imports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for rel in &files {
        let rel_str = rel.to_string_lossy().into_owned();
        let dir = match rel.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().into_owned(),
            _ => ".".to_string(),
        };
        dirs.entry(dir).or_default().push(rel_str.clone());
        let full = root.join(rel);
        let source = std::fs::read_to_string(&full)
            .map_err(|err| format!("failed to read {}: {err}", full.display()))?;
        clauses.insert(rel_str.clone(), package_clause(&source));
        imports.insert(rel_str, go_scan::imports_in_file(&full)?);
    }

    // Import path -> production files declaring that package. The target list
    // applies the scan driver's test-file predicate: a test file owns no
    // import path (no production import can compile against it), in either
    // package form. A directory holding only test files owns no import path.
    let mut owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (rel_dir, dir_files) in &dirs {
        let Some(index) = member_of_dir(rel_dir) else {
            continue;
        };
        let (dir, module) = &members[index];
        let import_path = go_scan::package_name(module, &rel_in_member(rel_dir, dir));
        let declared: BTreeSet<&str> = dir_files
            .iter()
            .filter(|file| !go_scan::is_test_file(Path::new(file)))
            .filter_map(|file| clauses.get(file).map(String::as_str))
            .filter(|clause| !clause.is_empty())
            .collect();
        for package in declared {
            for file in dir_files {
                if go_scan::is_test_file(Path::new(file)) {
                    continue;
                }
                let clause = clauses.get(file).map(String::as_str).unwrap_or("");
                if clause == package {
                    owners.entry(import_path.clone()).or_default().insert(file.clone());
                }
            }
        }
    }

    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for (from, file_imports) in &imports {
        for import in file_imports {
            let internal = members
                .iter()
                .any(|(_, module)| import == module || import.starts_with(&format!("{module}/")));
            if !internal {
                continue;
            }
            if let Some(declared_by) = owners.get(import) {
                for to in declared_by {
                    if to != from {
                        edges.insert((from.clone(), to.clone()));
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

/// The package clause of a Go source file: the name on the first `package`
/// line (the clause precedes all code and its comments start with `/`, so a
/// line scan cannot mistake a comment for the clause). Empty when absent.
fn package_clause(source: &str) -> String {
    for line in source.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("package") else {
            continue;
        };
        if rest.chars().next().is_some_and(|c| {
            c.is_alphanumeric() || c == '_'
        }) {
            continue;
        }
        let rest = rest.trim();
        let rest = rest.split("//").next().unwrap_or(rest).trim();
        if let Some(name) = rest.split_whitespace().next() {
            return name.to_string();
        }
    }
    String::new()
}

/// Collect `.go` files under `root` with the shared go exclusion predicate
/// (`scan::go::is_excluded_go`): hidden, `vendor`, and `testdata` paths are
/// invisible. Unlike the scan collectors, `*_test.go` files stay — inspect
/// discovers what the guard model deliberately drops.
fn collect_go_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
        if entry.depth() == 0 {
            return true;
        }
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        !go_scan::is_excluded_go(rel)
    });
    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("go") {
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
