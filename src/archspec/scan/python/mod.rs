//! python driver — the extraction arm of the python driver design record
//! (`workplan_archspec_python_driver.md`, laws P1-P5).
//!
//! Files are the vertices (the ts shape): a scanned tree's module identity is
//! the file path exactly as git spells it (D4 lineage), and one pyproject is
//! ONE package unit (P4). Resolution runs with NO interpreter, NO venv and
//! NO version modeling (P3): OWN → STDLIB → EXTERNAL, own keyed on directory
//! existence under the resolution roots — never on the distribution name
//! (the named sys.path deviation, PY-OWN-SHADOW). The stdlib table is the
//! static CPython 3.9-3.13 union ([`stdlib`]); an unlisted stdlib name stays
//! an EXTERNAL fact, so attribution drift never moves a verdict. Dynamic
//! imports ride the report-only line `dynamic import not modelled:` — no
//! edge, never red, never persisted (the ts wildcard precedent, P2).
//! `from a.b import c` is submodule-or-attribute ambiguous — the FILESYSTEM
//! decides: when `a/b/c.py` or `a/b/c/__init__.py` exists the edge ALSO
//! lands on module `a/b/c` (P2).
//!
//! The test tier is the C2 law ported (P5): pytest-family evidence in the
//! scanned-root manifest AND the `test_*.py` / `*_test.py` / `conftest.py`
//! convention, co-occurrence or nothing; the files stay nodes and import
//! sources, marked through the serialized `is_test_gated` channel.
//! The mechanical exclusion set (`language::DETECT_EXCLUDE_DIRS`) and the
//! spec's `[project] excluded_sources` roots contribute no files and no
//! edges; the report lines for declared exclusions print at the verbs.

pub(crate) mod stdlib;
pub(crate) mod syntax;

use crate::archspec::language;
use crate::archspec::model::{ManifestInfo, Model, ModuleDeclaration, ModuleEdge, Unit};
use crate::archspec::scan::python::syntax::ImportForm;
use crate::archspec::spec;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

/// One resolved first-party referent: a module FILE (regular package
/// `__init__.py` included) or a bare NAMESPACE DIRECTORY carrying no file of
/// its own (P3 — module-grain edge onto the directory, never red).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    File(String),
    Dir(String),
}

pub fn extract(root: &Path) -> Result<Model, String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let declared = spec::load_scan(&root)
        .ok()
        .flatten()
        .map(|loaded| spec::exclusion_paths(&loaded));
    let files = collect_files(&root, declared.as_ref())?;
    if files.is_empty() {
        return Err(format!("no python sources found under: {}", root.display()));
    }

    let resolution = Resolution::derive(&files);
    let manifest = root_manifest(&root);
    let unit_name = pyproject_name(&root).unwrap_or_else(|| ".".to_string());
    let unit = Unit {
        name: unit_name.clone(),
        kind: "package".to_string(),
        path: ".".to_string(),
        root: String::new(),
        crate_ids: BTreeSet::from([unit_name.clone()]),
    };

    let mut rows: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut external: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut findings: Vec<String> = Vec::new();
    let mut report_only: Vec<String> = Vec::new();
    let mut import_statements = 0usize;

    // The PARALLEL FACT LEG (round 9, P-PAR1/P-PAR2): ONLY the pure
    // per-file leg (`scan_python_file`: read + parse, no resolution
    // contact) runs across workers; the slot vector keeps every file's
    // result at its walk index and the fold replays in walk order, the
    // `?` on the FIRST Err reproducing today's early abort (F13/F14).
    // Below the floor the lever returns 1 and this is literally today's
    // serial loop (no threads).
    let threads = super::leg::fact_leg_threads(files.len());
    let all_facts: Vec<Result<syntax::FileFacts, String>> = if threads > 1 {
        super::leg::parallel_fact_leg(files.len(), threads, |index| {
            let file = &files[index];
            syntax::scan_python_file(&root.join(file))
                .map_err(|err| format!("{file}: {err}"))
        })
    } else {
        files.iter().map(|file| {
            syntax::scan_python_file(&root.join(file))
                .map_err(|err| format!("{file}: {err}"))
        }).collect()
    };

    for (file, facts) in files.iter().zip(all_facts) {
        let facts = facts?;
        import_statements += facts.import_statements;
        for (line, _spelling) in &facts.dynamic_sites {
            // The report-only wildcard line (P2/P3): the site is NAMED,
            // nothing is gated on it; never an edge, never the machine body.
            report_only.push(format!("dynamic import not modelled: {file}:{line} — importlib"));
        }
        for import in &facts.imports {
            match import {
                ImportForm::Absolute { specifier, names } => {
                    let parts: Vec<&str> = specifier.split('.').collect();
                    match resolution.resolve_absolute(&parts) {
                        Some(target) => {
                            add_edge(&mut rows, file, &target, names.iter().cloned());
                            // Submodule-or-attribute arm (P2): the FILESYSTEM
                            // decides — a name that exists as a module under
                            // the resolved module lands an edge of its own.
                            for name in names {
                                let mut deeper = parts.clone();
                                deeper.push(name.as_str());
                                if let Some(extra) = resolution.resolve_dotted_submodule(&deeper) {
                                    add_edge(&mut rows, file, &extra, std::iter::once(name.clone()));
                                }
                            }
                        }
                        None => {
                            // STDLIB and third-party are BOTH external facts
                            // (P3 tier order: OWN → STDLIB → EXTERNAL). The
                            // static union names the tier; both arms state
                            // the same serialized EXTERNAL fact, so the
                            // table can only drift the attribution label,
                            // never the verdict — a stdlib import is an
                            // EXTERNAL fact, never unresolvable, and a list
                            // miss keeps the name an EXTERNAL fact too
                            // (decision-check C2). The table itself rides
                            // `python::stdlib` as data: no legal branch
                            // exists here (attribution is never a gate).
                            let top = parts[0].to_string();
                            external.insert(top.clone());
                            module_external.entry(file.clone()).or_default().insert(top);
                        }
                    }
                }
                ImportForm::Relative { level, module, names } => {
                    let Some(anchor) = climb(file, *level) else {
                        // The NAMED python variant clause (P3, reused
                        // category): depth exceeding the package root.
                        findings.push(format!(
                            "unresolvable import: {file} -> \"{}\" — relative depth beyond package root",
                            relative_spelling(*level, module.as_deref())
                        ));
                        continue;
                    };
                    if !resolution.anchor_in_package(&anchor) {
                        findings.push(format!(
                            "unresolvable import: {file} -> \"{}\" — relative depth beyond package root",
                            relative_spelling(*level, module.as_deref())
                        ));
                        continue;
                    }
                    let base = match module {
                        Some(module) => join(&anchor, &module.replace('.', "/")),
                        None => anchor.clone(),
                    };
                    if !base.is_empty() {
                        match resolution.resolve_path(&base) {
                            Some(target) => add_edge(&mut rows, file, &target, names.iter().cloned()),
                            None => {
                                // A relative import names first-party code by
                                // construction; landing nowhere (not even a
                                // deeper existing prefix) is the reused
                                // unresolvable category with its shipped bytes.
                                findings.push(format!(
                                    "unresolvable import: {file} -> \"{}\"",
                                    relative_spelling(*level, module.as_deref())
                                ));
                                continue;
                            }
                        }
                    }
                    for name in names {
                        let deeper = join(&base, name);
                        if let Some(extra) = resolution.resolve_submodule(&deeper) {
                            add_edge(&mut rows, file, &extra, std::iter::once(name.clone()));
                        }
                    }
                }
            }
        }
    }
    if !findings.is_empty() {
        return Err(findings.join("\n"));
    }

    // The tripwire law (the ts D2 precedent, G1 vacuity law): scannable
    // files with parsed imports whose model states no edge or external fact
    // means extraction saw nothing. Report-only lines state an outcome too.
    if let Some(tripwire) = tripwire_error(
        files.len(),
        import_statements,
        !rows.is_empty(),
        !external.is_empty(),
        !report_only.is_empty(),
    ) {
        return Err(tripwire);
    }

    // P5: the test tier co-occurs — naming convention AND pytest-family
    // evidence in the scanned-root manifest. The vacuity law mirrors the ts
    // engine (the carrier rides a MISSING manifest, never an empty deps
    // list): a root with no manifest at all must not scan to a silent
    // green; a manifest stating an empty dependency set states a fact.
    let dependencies: Vec<String> = manifest
        .as_ref()
        .map(|info| info.dependencies.clone())
        .unwrap_or_default();
    if manifest.is_none() {
        report_only.push(
            "vacuous constraint: scan.unit_manifests: no package manifest states dependencies"
                .to_string(),
        );
    }
    // A declared distribution is an EXTERNAL fact beside the imports that
    // mention it (the ts package.json / Cargo.toml parity carried by
    // `lists_manifest_declared_packages_as_external`): the model states
    // every name the root manifest declares, no version, no finding.
    for dependency in &dependencies {
        external.insert(dependency.clone());
    }
    let test_gated_modules: BTreeSet<String> = files
        .iter()
        .filter(|file| test_tier_convention(file))
        .filter(|_| declares_pytest(&dependencies))
        .cloned()
        .collect();

    let mut unit_manifests: BTreeMap<String, ManifestInfo> = BTreeMap::new();
    unit_manifests.insert(
        unit_name.clone(),
        manifest.clone().unwrap_or(ManifestInfo {
            publish: None,
            dependencies: Vec::new(),
            features: Vec::new(),
        }),
    );

    let module_edges: Vec<ModuleEdge> = rows
        .iter()
        .map(|((from, to), symbols)| ModuleEdge {
            unit: unit_name.clone(),
            from: from.clone(),
            to: to.clone(),
            symbols: symbols.iter().cloned().collect(),
            type_only: false,
        })
        .collect();
    let mut declarations: BTreeMap<String, Vec<ModuleDeclaration>> = BTreeMap::new();
    for file in &files {
        declarations
            .entry(unit_name.clone())
            .or_default()
            .push(ModuleDeclaration {
                name: file.clone(),
                gated: false,
                feature: None,
                file: file.clone(),
            });
    }

    Ok(Model {
        units: vec![unit],
        external: external.into_iter().collect(),
        module_edges,
        manifest,
        module_external: module_external
            .into_iter()
            .map(|(file, names)| (file, names.into_iter().collect()))
            .collect(),
        root_module_declarations: declarations,
        unit_manifests,
        test_gated_modules,
        // The wildcard line and the vacuity line ride the transient
        // channel (the ts R7b lineage): stderr via the report renderer,
        // never the machine body.
        report_only_lines: report_only,
        ..Model::new("python")
    })
}

/// The tripwire law (the ts D2 precedent ported): files with parsed imports
/// whose model states no edge and no external fact means extraction saw
/// nothing — a parser that silently states nothing cannot pass green.
/// Pure for the unit test; the assembly calls it after the walk.
fn tripwire_error(
    files: usize,
    import_statements: usize,
    has_edges: bool,
    has_external: bool,
    has_report_only: bool,
) -> Option<String> {
    if files > 0 && import_statements > 0 && !has_edges && !has_external && !has_report_only {
        return Some(format!(
            "python driver tripwire: {files} scanned files carry {import_statements} import statements, yet the model states no edge or external fact"
        ));
    }
    None
}

/// The P5 naming convention half: `test_*.py`, `*_test.py` and
/// `conftest.py` files. The convention alone carries no tier — it is ANDed
/// with the manifest's pytest-family evidence below (the ts C2 law ported).
fn test_tier_convention(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    name == "conftest.py" || (name.starts_with("test_") || name.ends_with("_test.py"))
}

/// The P5 evidence half: the `pytest*` FAMILY predicate (round-1 fix E1 —
/// the pilot declares `pytest-asyncio` only, and the family is the honest
/// signal that the tree runs pytest). Case-insensitive by distribution-name
/// law.
fn declares_pytest(dependencies: &[String]) -> bool {
    dependencies
        .iter()
        .any(|dependency| dependency.to_lowercase().starts_with("pytest"))
}

/// One edge row: same-file rows state nothing (the R2-1 lineage) and are
/// dropped at entry; symbol lists merge into the (from, to) key.
fn add_edge(
    rows: &mut BTreeMap<(String, String), BTreeSet<String>>,
    from: &str,
    target: &Target,
    symbols: impl Iterator<Item = String>,
) {
    let to = match target {
        Target::File(path) => path.clone(),
        Target::Dir(dir) => dir.clone(),
    };
    if to == from {
        return;
    }
    rows.entry((from.to_string(), to)).or_default().extend(symbols);
}

/// The file's directory climbed `level − 1` levels (P3: `depth = dots − 1`);
/// `None` when the walk leaves the tree — the beyond-root variant.
fn climb(file: &str, level: usize) -> Option<String> {
    let mut path = PathBuf::from(file);
    if !path.pop() {
        path = PathBuf::new();
    }
    for _ in 1..level {
        if !path.pop() {
            return None;
        }
    }
    Some(spell(&path))
}

/// The `from` clause re-spelled for the finding line: the dots plus the
/// module tail, exactly the specifier the source carries.
fn relative_spelling(level: usize, module: Option<&str>) -> String {
    let dots = ".".repeat(level);
    match module {
        Some(module) => format!("{dots}{module}"),
        None => dots,
    }
}

fn join(base: &str, tail: &str) -> String {
    if base.is_empty() {
        tail.to_string()
    } else if tail.is_empty() {
        base.to_string()
    } else {
        format!("{base}/{tail}")
    }
}

fn spell(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The P3 package-root derivation (decision-check B2) and the resolution
/// law: first-party maps the scanned FILE SET — every directory question is
/// a question about collected sources, so the resolution roots, the
/// traversability set and the edges share one ground (a mechanical or
/// declared exclusion removes files, never resolution law).
struct Resolution {
    /// Case 1 (the scan root holds sources or `__init__.py`): dotted names
    /// map onto root-relative paths directly.
    root_maps_names: bool,
    /// Cases 2/3: resolution roots a name must start with (the package root
    /// of a single-child layout; the union of every top-level source dir
    /// otherwise). The first segment must equal the root dir's name.
    name_roots: Vec<String>,
    /// Every ancestor directory of every scanned `.py` file — the
    /// traversable set: a directory holding `__init__.py` or any `.py` below
    /// it is first-party-traversable, regular or namespace, ONE referent
    /// rule (P3, closing decision-check B1).
    traversable: BTreeSet<String>,
    /// The scanned files themselves.
    files: BTreeSet<String>,
}

impl Resolution {
    fn derive(files: &[String]) -> Self {
        let mut traversable: BTreeSet<String> = BTreeSet::new();
        let mut tops: BTreeSet<String> = BTreeSet::new();
        let mut root_maps_names = false;
        for file in files {
            let mut dir = Path::new(file)
                .parent()
                .map(spell)
                .unwrap_or_default();
            loop {
                if dir.is_empty() {
                    root_maps_names = true;
                    break;
                }
                if !traversable.insert(dir.clone()) {
                    break;
                }
                if let Some(tail) = dir.split_once('/') {
                    dir = tail.0.to_string();
                } else {
                    tops.insert(dir.clone());
                    break;
                }
            }
        }
        // Case 1 maps names onto the root directly. Cases 2/3 share one
        // shape: every top-level source dir is a name root (single-child
        // layouts have exactly one; case 3 has many).
        let name_roots = if root_maps_names {
            Vec::new()
        } else {
            tops.iter().cloned().collect()
        };
        Self {
            root_maps_names,
            name_roots,
            traversable,
            files: files.iter().cloned().collect(),
        }
    }

    /// The candidate scan-root-relative paths a dotted name maps onto:
    /// case 1 maps the joined path onto the scan root directly; cases 2/3
    /// require the first segment to equal the root dir's name (the
    /// open-webui `backend/` → `open_webui/` shape), one candidate per
    /// matching root.
    fn candidate_paths(&self, parts: &[&str]) -> Vec<String> {
        if parts.is_empty() {
            return Vec::new();
        }
        let mut candidates = Vec::new();
        if self.root_maps_names {
            candidates.push(parts.join("/"));
        }
        for root in &self.name_roots {
            let name = root.rsplit('/').next().unwrap_or(root);
            if name != parts[0] {
                continue;
            }
            candidates.push(if parts.len() == 1 {
                root.clone()
            } else {
                join(root, &parts[1..].join("/"))
            });
        }
        candidates
    }

    /// An absolute dotted name: OWN by directory existence (the shadow
    /// clause — never by the distribution name). Case 1 maps the path onto
    /// the scan root; cases 2/3 require the first segment to equal the root
    /// dir's name (the open-webui `backend/` → `open_webui/` shape).
    fn resolve_absolute(&self, parts: &[&str]) -> Option<Target> {
        for rel in self.candidate_paths(parts) {
            if let Some(target) = self.resolve_path(&rel) {
                return Some(target);
            }
        }
        None
    }

    /// The dotted name of the submodule-or-attribute arm, resolved by
    /// MODULE existence alone (see `resolve_submodule`).
    fn resolve_dotted_submodule(&self, parts: &[&str]) -> Option<Target> {
        for rel in self.candidate_paths(parts) {
            if let Some(target) = self.resolve_submodule(&rel) {
                return Some(target);
            }
        }
        None
    }

    /// The longest-EXISTING-prefix referent law (P3): the full dotted path
    /// maps to exactly one referent — its file, its package `__init__.py`,
    /// the bare namespace directory, or the deepest existing directory
    /// prefix below it (a bare or partial path landing on a directory is a
    /// module-grain edge, never red, never a file-grain phantom).
    fn resolve_path(&self, rel: &str) -> Option<Target> {
        if rel.is_empty() {
            return if self.traversable.contains(".") || self.files.iter().any(|f| !f.contains('/'))
            {
                Some(Target::Dir(".".to_string()))
            } else {
                None
            };
        }
        if self.files.contains(&format!("{rel}.py")) {
            return Some(Target::File(format!("{rel}.py")));
        }
        if self.traversable.contains(rel) {
            return Some(if self.files.contains(&format!("{rel}/__init__.py")) {
                Target::File(format!("{rel}/__init__.py"))
            } else {
                Target::Dir(rel.to_string())
            });
        }
        let segments: Vec<&str> = rel.split('/').collect();
        for end in (1..segments.len()).rev() {
            let prefix = segments[..end].join("/");
            if self.traversable.contains(&prefix) {
                return Some(Target::Dir(prefix));
            }
        }
        None
    }

    /// The submodule-or-attribute arm (P2): the FILESYSTEM decides, and only
    /// MODULE existence lands an edge — `c.py`, `c/__init__.py`, or the
    /// namespace directory `c` under the ONE referent rule (P3). An
    /// attribute is no module and rides no edge: the longest-prefix
    /// fallback of `resolve_path` is illegal here (it would steer every
    /// attribute onto the base package directory — the shape P2 excludes).
    fn resolve_submodule(&self, rel: &str) -> Option<Target> {
        if rel.is_empty() {
            return None;
        }
        if self.files.contains(&format!("{rel}.py")) {
            return Some(Target::File(format!("{rel}.py")));
        }
        if self.files.contains(&format!("{rel}/__init__.py")) {
            return Some(Target::File(format!("{rel}/__init__.py")));
        }
        if self.traversable.contains(rel) {
            return Some(Target::Dir(rel.to_string()));
        }
        None
    }

    /// True while a relative anchor still sits at or below a package root
    /// (the P3 relative law: exceeding it is the named unresolvable clause).
    fn anchor_in_package(&self, anchor: &str) -> bool {
        if self.root_maps_names {
            return true;
        }
        self.name_roots.iter().any(|root| {
            anchor == root || anchor.starts_with(&format!("{root}/"))
        })
    }
}

/// The scanned `.py` files, repo-relative, git spelling, sorted. Excluded
/// trees contribute no files (the ts exclusion law; `.venv` and
/// `__pycache__` ride the mechanical set, the P3 no-venv law).
fn collect_files(root: &Path, declared: Option<&Vec<PathBuf>>) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !excluded_walk(entry.path(), root, declared))
    {
        let entry = entry.map_err(|err| format!("cannot scan tree: {err}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !language::is_python_source(&name) {
            continue;
        }
        if let Some(rel) = entry.path().strip_prefix(root).ok().map(spell) {
            files.push(rel);
        }
    }
    files.sort();
    Ok(files)
}

fn excluded_walk(path: &Path, root: &Path, declared: Option<&Vec<PathBuf>>) -> bool {
    if path
        .components()
        .any(|component| matches!(component, Component::Normal(name)
            if language::is_excluded_dir(&name.to_string_lossy())))
    {
        return true;
    }
    path.strip_prefix(root).ok().is_some_and(|rel| {
        declared.is_some_and(|exclusions| {
            exclusions.iter().any(|entry| rel.starts_with(entry))
        })
    })
}

/// The scanned-root manifest facts (P1 decision (a), round-1 fix A1/A2):
/// `pyproject.toml` IS the declared-name set — `[project].dependencies`,
/// `[project.optional-dependencies]` and `[dependency-groups]` — and a
/// `requirements.txt` at the SAME root ADDS names, never replaces. A
/// requirements-only root uses its own list. No walk-up (the scan-root
/// bounding law); `setup.py`/`setup.cfg` are not read (measured absent).
pub(crate) fn root_manifest(root: &Path) -> Option<ManifestInfo> {
    let mut dependencies: BTreeSet<String> = BTreeSet::new();
    let pyproject = std::fs::read_to_string(root.join("pyproject.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Value>().ok());
    if let Some(value) = &pyproject {
        if let Some(entries) = value
            .get("project")
            .and_then(|project| project.get("dependencies"))
            .and_then(toml::Value::as_array)
        {
            dependencies.extend(entries.iter().filter_map(dependency_name));
        }
        for key in ["optional-dependencies", "dependency-groups"] {
            if let Some(groups) = value
                .get("project")
                .and_then(|project| project.get(key))
                .or_else(|| value.get("dependency-groups"))
                .and_then(toml::Value::as_table)
            {
                for entries in groups.values().filter_map(toml::Value::as_array) {
                    dependencies.extend(entries.iter().filter_map(dependency_name));
                }
            }
        }
    }
    if let Ok(raw) = std::fs::read_to_string(root.join("requirements.txt")) {
        dependencies.extend(requirements_names(&raw));
    }
    if pyproject.is_none() && dependencies.is_empty() {
        return None;
    }
    Some(ManifestInfo {
        publish: None,
        dependencies: dependencies.into_iter().collect(),
        features: Vec::new(),
    })
}

/// The distribution name of a PEP 508 clause (`sqlalchemy>=2.0`,
/// `pkg[extra]==1; marker`, an inline-table `{ name = "pytest" }`): the
/// leading name run, version specifiers never parsed (P3).
fn dependency_name(entry: &toml::Value) -> Option<String> {
    let text = match entry {
        toml::Value::String(text) => Some(text.as_str()),
        toml::Value::Table(table) => table.get("name").and_then(toml::Value::as_str),
        _ => None,
    }?;
    let name = text
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect::<String>();
    (!name.is_empty()).then_some(name)
}

/// A requirements.txt name list: name runs, comments, option lines and
/// empty lines skipped (no `-r` chase — the ADDITIVE list is this file).
fn requirements_names(raw: &str) -> BTreeSet<String> {
    raw.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('-'))
        .filter_map(|line| {
            let name = line
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                .collect::<String>();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

fn pyproject_name(root: &Path) -> Option<String> {
    std::fs::read_to_string(root.join("pyproject.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Value>().ok())
        .and_then(|value| {
            value
                .get("project")
                .and_then(|project| project.get("name"))
                .and_then(toml::Value::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
}

#[cfg(test)]
mod tests {
    use super::{
        Resolution, declares_pytest, relative_spelling, test_tier_convention, tripwire_error,
    };

    #[test]
    fn tripwire_fires_only_on_total_silence() {
        assert!(tripwire_error(3, 5, false, false, false).is_some());
        assert!(tripwire_error(3, 5, true, false, false).is_none());
        assert!(tripwire_error(3, 5, false, true, false).is_none());
        assert!(tripwire_error(3, 5, false, false, true).is_none());
        assert!(tripwire_error(3, 0, false, false, false).is_none());
        assert!(tripwire_error(0, 0, false, false, false).is_none());
    }

    #[test]
    fn test_convention_is_the_pytest_family_shape() {
        assert!(test_tier_convention("test_login.py"));
        assert!(test_tier_convention("pkg/test_deep.py"));
        assert!(test_tier_convention("api_test.py"));
        assert!(test_tier_convention("pkg/conftest.py"));
        assert!(!test_tier_convention("tests.py"));
        assert!(!test_tier_convention("helper.py"));
    }

    #[test]
    fn pytest_evidence_is_the_family_not_the_bare_name() {
        assert!(declares_pytest(&["pytest".to_string()]));
        assert!(declares_pytest(&["pytest-asyncio".to_string()]));
        assert!(declares_pytest(&["Pytest-Cov".to_string()]));
        assert!(!declares_pytest(&["nose".to_string(), "unittest2".to_string()]));
    }

    #[test]
    fn relative_spelling_reconstructs_the_specifier() {
        assert_eq!(relative_spelling(1, None), ".");
        assert_eq!(relative_spelling(3, Some("beyond.root")), "...beyond.root");
    }

    fn resolution(files: &[&str]) -> Resolution {
        Resolution::derive(&files.iter().map(|file| file.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn root_holding_sources_maps_names_directly() {
        let resolution = resolution(&["app.py", "utils/helper.py"]);
        assert!(resolution.root_maps_names);
        let target = resolution
            .resolve_absolute(&["utils", "helper"])
            .expect("root-relative name resolves");
        assert!(matches!(target, super::Target::File(path) if path == "utils/helper.py"));
        let bare = resolution.resolve_absolute(&["app"]).expect("root module");
        assert!(matches!(bare, super::Target::File(path) if path == "app.py"));
    }

    #[test]
    fn single_child_layout_requires_the_package_name() {
        // The open-webui `backend/` shape: no root sources, ONE child dir
        // holding everything — the child IS the package root.
        let resolution = resolution(&["open_webui/main.py", "open_webui/models/user.py"]);
        assert!(!resolution.root_maps_names);
        let file = resolution
            .resolve_absolute(&["open_webui", "models", "user"])
            .expect("leaf file resolves");
        assert!(matches!(file, super::Target::File(path) if path == "open_webui/models/user.py"));
        // The package itself (regular package: `__init__.py` in the set).
        let bare = resolution.resolve_absolute(&["open_webui"]).expect("root package");
        assert!(matches!(bare, super::Target::Dir(dir) if dir == "open_webui"));
        // A name skipping the package root is NOT first-party here.
        assert!(resolution.resolve_absolute(&["models", "user"]).is_none());
    }

    #[test]
    fn namespace_directory_lands_module_grain_edges() {
        let resolution = resolution(&["pkg/nsdir/leaf.py"]);
        let target = resolution
            .resolve_absolute(&["pkg", "nsdir"])
            .expect("namespace dir");
        assert!(
            matches!(target, super::Target::Dir(dir) if dir == "pkg/nsdir"),
            "a bare namespace-dir import is a first-party module-grain edge, never red (P3)"
        );
    }

    #[test]
    fn the_longest_existing_prefix_decides_a_partial_path() {
        let resolution = resolution(&["pkg/sub/mod.py"]);
        let target = resolution
            .resolve_absolute(&["pkg", "sub", "missing"])
            .expect("prefix");
        assert!(matches!(target, super::Target::Dir(dir) if dir == "pkg/sub"));
        // A name whose FIRST segment is not a first-party dir is no referent.
        assert!(resolution.resolve_absolute(&["other", "deep"]).is_none());
    }

    #[test]
    fn shadow_clause_directory_existence_claims_first_party() {
        // A package-root directory named like a PyPI import name claims
        // first-party (P3 — the named sys.path deviation, PY-OWN-SHADOW).
        let resolution = resolution(&["jwt/decode.py"]);
        let target = resolution
            .resolve_absolute(&["jwt", "api"])
            .expect("shadowed");
        assert!(matches!(target, super::Target::Dir(dir) if dir == "jwt"));
    }

    #[test]
    fn anchors_outside_a_name_root_are_beyond_the_package_root() {
        let tree = resolution(&["open_webui/routers/x.py"]);
        assert!(tree.anchor_in_package("open_webui"));
        assert!(tree.anchor_in_package("open_webui/routers"));
        assert!(!tree.anchor_in_package(""));
        assert!(!tree.anchor_in_package("other"));
        let flat = resolution(&["app.py"]);
        assert!(flat.anchor_in_package(""));
    }
}
