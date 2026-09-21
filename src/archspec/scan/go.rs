use crate::archspec::model::{Edge, Model, ModuleEdge, Unit};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    // path, module tier absent exactly as before go.work support.
    let module = module_path(&root)?;

    let packages = collect_packages(&root);

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
    let mut external_set: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let module_prefix = format!("{module}/");
    for (rel_dir, files) in &packages {
        let from = package_name(&module, rel_dir);
        for file in files {
            for import in imports_in_file(file)? {
                if import == module || import.starts_with(&module_prefix) {
                    // Internal to the module: an edge only when the target
                    // package is a discovered unit (empty/undiscovered dirs
                    // contribute nothing).
                    if unit_names.contains(import.as_str()) && import != from {
                        edge_set.insert((from.clone(), import));
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
        .into_iter()
        .map(|(from, to)| Edge { from, to })
        .collect();
    let external: Vec<String> = external_set.into_iter().collect();
    let module_external: BTreeMap<String, Vec<String>> = module_external
        .into_iter()
        .map(|(unit, crates)| (unit, crates.into_iter().collect()))
        .collect();

    Ok(Model {
        schema_version: 1,
        language: "go".to_string(),
        units,
        edges,
        usage: Default::default(),
        soft_structure: Default::default(),
        external,
        module_edges: Default::default(),
        manifest: Default::default(),
        root_public_exports: Default::default(),
        root_glob_exports: Default::default(),
        root_empty_glob_exports: Default::default(),
        module_external,
        root_module_declarations: Default::default(),
        unit_manifests: Default::default(),
        unresolved_module_files: Default::default(),
        test_gated_modules: Default::default(),
        facade_roots: Default::default(),
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
fn extract_workspace(root: &Path, members: &[(String, String)]) -> Result<Model, String> {
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
    let mut external_set: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (rel_dir, files) in &packages {
        let Some(from) = unit_for_dir.get(rel_dir) else {
            continue;
        };
        for file in files {
            for import in imports_in_file(file)? {
                let internal = members
                    .iter()
                    .any(|(_, module)| import == *module || import.starts_with(&format!("{module}/")));
                if internal {
                    if unit_names.contains(import.as_str()) && import != *from {
                        edge_set.insert((from.clone(), import));
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

    // Cross-member imports project onto the module tier; within-member edges
    // stay unit-tier only (one module, nothing to project between).
    let mut module_edges: Vec<ModuleEdge> = Vec::new();
    for (from, to) in &edge_set {
        let (Some(from_member), Some(to_member)) = (module_owner(from), module_owner(to)) else {
            continue;
        };
        if from_member != to_member {
            module_edges.push(ModuleEdge {
                unit: from_member.to_string(),
                from: dotted_module(from),
                to: dotted_module(to),
                symbols: Vec::new(),
            });
        }
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
        facade_roots: Default::default(),
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

/// Collect `import` paths from a .go file (source-level, no toolchain).
/// Walks the source with a tokenizer that skips strings and comments so an
/// `import` keyword inside a comment or string is never matched. `import "C"`
/// (the cgo marker) is dropped — it states no dependency fact.
pub(crate) fn imports_in_file(path: &Path) -> Result<BTreeSet<String>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let bytes = raw.as_bytes();
    let mut i = 0;
    let mut imports = BTreeSet::new();

    while i < bytes.len() {
        let c = bytes[i];
        if c == b'"' {
            i = skip_quoted(bytes, i);
            continue;
        }
        if c == b'`' {
            i = skip_raw_string(bytes, i);
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'/' {
                i = skip_line_comment(bytes, i);
                continue;
            }
            if bytes[i + 1] == b'*' {
                i = skip_block_comment(bytes, i);
                continue;
            }
        }
        if c.is_ascii_alphanumeric() && matches_identifier(bytes, i, b"import") {
            // `import` keyword: parse the following import spec(s).
            i += "import".len();
            i = skip_ws_and_comments(bytes, i);
            if i < bytes.len() && bytes[i] == b'(' {
                i += 1;
                i = parse_block_imports(bytes, i, &mut imports);
            } else {
                i = parse_single_import(bytes, i, &mut imports);
            }
            continue;
        }
        i += 1;
    }
    Ok(imports)
}

/// True if `bytes[i..]` starts with `word` at an identifier boundary.
fn matches_identifier(bytes: &[u8], i: usize, word: &[u8]) -> bool {
    if bytes.len() < i + word.len() {
        return false;
    }
    let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_';
    let after = i + word.len();
    let after_ok =
        after >= bytes.len() || !bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_';
    before_ok && after_ok && &bytes[i..after] == word
}

fn skip_quoted(bytes: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

fn skip_raw_string(bytes: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < bytes.len() && bytes[i] != b'`' {
        i += 1;
    }
    if i < bytes.len() {
        i + 1
    } else {
        bytes.len()
    }
}

fn skip_line_comment(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn skip_block_comment(bytes: &[u8], mut i: usize) -> usize {
    i += 2;
    while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
        i += 1;
    }
    if i + 1 < bytes.len() {
        i + 2
    } else {
        bytes.len()
    }
}

fn skip_ws_and_comments(bytes: &[u8], mut i: usize) -> usize {
    loop {
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n') {
            i += 1;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            i = skip_line_comment(bytes, i);
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i = skip_block_comment(bytes, i);
            continue;
        }
        return i;
    }
}

/// In block form: repeatedly skip an optional alias token (identifier, `_`, or
/// `.`) then consume a string literal, until `)`.
fn parse_block_imports(bytes: &[u8], mut i: usize, imports: &mut BTreeSet<String>) -> usize {
    loop {
        i = skip_ws_and_comments(bytes, i);
        if i >= bytes.len() || bytes[i] == b')' {
            return if i < bytes.len() { i + 1 } else { bytes.len() };
        }
        // Skip alias: identifier, `_`, or `.`
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
        {
            i += 1;
        }
        i = skip_ws_and_comments(bytes, i);
        if i < bytes.len() && bytes[i] == b'"' {
            i = record_import(bytes, i, imports);
        } else if i < bytes.len() && bytes[i] == b'`' {
            i = record_raw_import(bytes, i, imports);
        } else {
            // Malformed spec; step one byte to avoid a hang.
            return i + 1;
        }
    }
}

/// Single form: skip one optional alias token, then consume the string.
fn parse_single_import(bytes: &[u8], mut i: usize, imports: &mut BTreeSet<String>) -> usize {
    // Optional alias: identifier, `_`, or `.`
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
    {
        i += 1;
    }
    i = skip_ws_and_comments(bytes, i);
    if i < bytes.len() && bytes[i] == b'"' {
        record_import(bytes, i, imports)
    } else if i < bytes.len() && bytes[i] == b'`' {
        record_raw_import(bytes, i, imports)
    } else {
        i
    }
}

fn record_import(bytes: &[u8], start: usize, imports: &mut BTreeSet<String>) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            let path = String::from_utf8_lossy(&bytes[start + 1..i]).into_owned();
            if path != "C" {
                imports.insert(path);
            }
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

fn record_raw_import(bytes: &[u8], start: usize, imports: &mut BTreeSet<String>) -> usize {
    let mut i = start + 1;
    while i < bytes.len() && bytes[i] != b'`' {
        i += 1;
    }
    if i < bytes.len() {
        let path = String::from_utf8_lossy(&bytes[start + 1..i]).into_owned();
        imports.insert(path);
        i + 1
    } else {
        bytes.len()
    }
}
