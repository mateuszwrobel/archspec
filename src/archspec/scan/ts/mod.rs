//! ts/js driver — the extraction arm of the ts-driver design (ADR-022).
//!
//! Files are the vertices: a scanned tree's module identity is the file path
//! exactly as git spells it, extension included (D4), and the package root
//! is the unit (D9). Edges come from import, export-from, literal dynamic
//! and asset specifiers — nothing else; call graphs and template references
//! are not dependency facts (D2). Resolution runs through extension-and-index
//! inference for relative specifiers; bare and `#` specifiers delegate
//! through the pinned resolver on the bounded owning tsconfig chain (ADR-024
//! resolution record R1) — a first-party alias hit contributes an edge, node
//! builtins and third-party packages land in the external tier, and a
//! declared-absent package adds a report-only `external not installed` line
//! (R7b). A specifier that resolves to nothing fails loud as an
//! `unresolvable import` finding, and one that resolves only into excluded
//! paths names the exclusion it hit (D2, D14) — the driver never drops an
//! edge silent. A non-literal dynamic import or computed require is a
//! named wildcard fact,
//! report-only (R2-6). `.svelte` components are first-class units: every
//! `<script>` block is parsed with the TypeScript grammar.
//!
//! The mechanical exclusion set (`language::DETECT_EXCLUDE_DIRS`) and the
//! spec's `[project] excluded_sources` roots contribute no files and no
//! edges; the scan report lines for the declared exclusions are printed by
//! the verbs (D14/R3-7).

mod config_chain;
mod config_preload;
mod gitignore;
mod resolution;
mod syntax;
mod workspace;

mod common;
use common::{Exclusions, Resolved, exclusion_hit, is_excluded, mechanically_excluded, relative, spell, strip_query};

use crate::archspec::language;
use crate::archspec::model::{ManifestInfo, Model, ModuleDeclaration, ModuleEdge, Unit};
use crate::archspec::spec;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

/// The source extensions the ts driver reads and the resolution law infers.
const TS_SOURCE_EXTS: &[&str] = &["ts", "tsx", "js", "jsx", "svelte", "mjs", "cjs", "mts", "cts"];

/// Extensions that mark an asset import: an ordinary edge to the exact file,
/// never extension-inferred (D2 asset law — the runtime edges the retired
/// textual guard counted stay visible).
const ASSET_EXTS: &[&str] = &[
    "css", "scss", "sass", "less", "styl", "svg", "png", "jpg", "jpeg", "gif", "webp", "avif",
    "woff", "woff2", "ttf", "eot", "mp3", "mp4", "webm", "wav", "ogg", "json", "txt", "wasm",
];

pub fn extract(root: &Path) -> Result<Model, String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    // The spec at the scanned root (root-only discovery, D14(4)) supplies the
    // declared exclusions the resolution law names in unresolvable findings.
    // An invalid spec failed the verb before extraction; its error is not
    // re-raised here.
    let loaded_spec = spec::load_scan(&root).ok().flatten();
    let declared = loaded_spec.as_ref().map(Exclusions::new);
    let files = collect_files(&root, declared.as_ref())?;
    if files.is_empty() {
        return Err(format!("no ts sources found under: {}", root.display()));
    }

    let unit = package_unit(&root);
    let mut rows: BTreeMap<(String, String, bool), BTreeSet<String>> = BTreeMap::new();
    let mut external: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut wildcards: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut exports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut findings: Vec<String> = Vec::new();
    // C4 root-barrel candidates: files whose every resolvable row is an
    // export-from republication and whose every exported name is backed by
    // such a row — the ts counterpart of the rust "root defines nothing but
    // mod declarations and re-exports" facade derivation. The identity set
    // alone decides nothing: the barrel position (a package's root
    // `index.ts`) is ANDed with this purity below.
    let mut pure_republication: BTreeSet<String> = BTreeSet::new();
    // The resolution seam (ADR-024 R1): one engine per extract, entries
    // cached per owning config, findings and report-only lines accumulate
    // for the join-and-Err below and the model's transient report field.
    let mut engine = resolution::Engine::new(
        &root,
        declared.as_ref(),
        loaded_spec
            .as_ref()
            .and_then(|loaded| loaded.project.framework.as_deref()),
    );
    // The workspace registry (R5): the membership map of a workspace tree.
    // An empty registry keeps every law below at the single-package
    // dialect, byte-identical (the R5 single-package-compat clause).
    let registries = workspace::scan_roots_ts(&root)?;
    let members = members_under_root(&registries, &root);
    if !members.is_empty() {
        engine.set_workspace(registries.clone());
    }
    let mut import_statements = 0usize;

    // The PARALLEL FACT LEG (round 9, P-PAR1/P-PAR2): ONLY the pure
    // per-file leg (read + parse, no engine contact) may run across
    // workers; the slot vector keeps every file's result at its walk
    // index, and the fold below replays strictly in walk order, the `?`
    // on the FIRST Err reproducing today's early abort — files after
    // the error never reach `resolve`, so their findings, rows and
    // report-only lines never exist (F14). Below the floor the lever
    // returns 1 and this is literally today's serial loop (no threads).
    let threads = super::leg::fact_leg_threads(files.len());
    let all_facts: Vec<Result<syntax::FileFacts, String>> = if threads > 1 {
        super::leg::parallel_fact_leg(files.len(), threads, |index| {
            file_facts(&root, &files[index])
        })
    } else {
        files.iter().map(|file| file_facts(&root, file)).collect()
    };

    for (file, facts) in files.iter().zip(all_facts) {
        let facts = facts?;
        import_statements += facts.import_statements;
        let defines_nothing = !facts.edges.is_empty()
            && facts.edges.iter().all(|edge| edge.export_from)
            && facts.exports.iter().all(|name| {
                facts.edges.iter().flat_map(|edge| {
                    edge.value_symbols
                        .iter()
                        .chain(edge.type_symbols.iter())
                })
                .any(|symbol| symbol == name || symbol == "*")
            });
        if defines_nothing {
            pure_republication.insert(file.clone());
        }
        if !facts.exports.is_empty() {
            exports
                .entry(file.clone())
                .or_default()
                .extend(facts.exports.iter().cloned());
        }
        for wildcard in &facts.wildcards {
            wildcards
                .entry(file.clone())
                .or_default()
                .insert(wildcard.clone());
        }
        for edge in &facts.edges {
            match resolve(&root, file, edge, declared.as_ref(), &mut engine) {
                Resolved::Edge(target) => {
                    // The (from, to, kind) row split (D3/R3-9): a value
                    // crossing contributes the type_only=false row, a
                    // type-only crossing the type_only=true row, and a
                    // mixed pair contributes exactly the two rows.
                    if !edge.type_only {
                        rows.entry((file.clone(), target.clone(), false))
                            .or_default()
                            .extend(edge.value_symbols.iter().cloned());
                    }
                    if edge.type_only || !edge.type_symbols.is_empty() {
                        let symbols = if edge.type_symbols.is_empty() {
                            edge.value_symbols.iter().cloned().collect::<BTreeSet<_>>()
                        } else {
                            edge.type_symbols.iter().cloned().collect()
                        };
                        rows.entry((file.clone(), target, true))
                            .or_default()
                            .extend(symbols);
                    }
                }
                Resolved::External(package) => {
                    external.insert(package.clone());
                    // File identity (D10): `module_external` keys the file
                    // exactly as edges and claims address it. The
                    // single-package law keeps every file under one unit,
                    // so the comparison's ts fallback attributes a unit
                    // pattern to all keys — the key itself stays undecorated.
                    module_external.entry(file.clone()).or_default().insert(package);
                }
                Resolved::Unresolvable(why) => {
                    findings.push(format!("unresolvable import: {file} -> \"{}\"{why}", edge.specifier));
                }
                Resolved::Reported => {
                    // The virtual-specifier law: the line was recorded by
                    // the seam; the model stays untouched — no edge, no
                    // external-tier entry, no finding.
                }
            }
        }
    }

    // The seam's category lines ride the same channel as the anchor lines
    // above: findings as generated, deduped by the engine. The report-only
    // lines are taken here too: a virtual-specifier line is a STATED outcome
    // (round 4 C: the law never reddens a run), so it must also satisfy the
    // silence tripwire below — every other report-only class already rides
    // a fact, so this changes behavior for the `Reported` arm only.
    findings.extend(engine.take_findings());
    if !findings.is_empty() {
        return Err(findings.join("\n"));
    }
    let mut report_only = engine.take_report_only();
    if let Some(tripwire) = tripwire_error(
        files.len(),
        import_statements,
        !rows.is_empty(),
        !wildcards.is_empty(),
        !external.is_empty(),
        !report_only.is_empty(),
    ) {
        return Err(tripwire);
    }

    // The orphan census (workplan workplan_archspec_unreferenced_census):
    // computed STRICTLY AFTER the tripwire above, and its lines never feed
    // the tripwire's stated-fact arms — a dead extractor cannot launder
    // itself green through the census (D7, the B1 ordering law: the census
    // is ABSENCE computed from the walk, not a fact). The lines ride the
    // transient channel like the R7b and vacuity classes, and the summary
    // never fires at zero (the java-census precedent, D9).
    let orphans = orphan_census_lines(&files, &rows, &module_external, &wildcards);
    if !orphans.is_empty() {
        report_only.extend(orphans.iter().cloned());
        report_only.push(format!(
            "orphan file census: {} of {} scanned files — the model lists no fact for any of them",
            orphans.len(),
            files.len()
        ));
    }

    // The manifest feeds the external tier exactly like the family law: the
    // tier names the packages the project declares and the packages its
    // sources import. The merge runs here — after the tripwire — so declared
    // dependencies can never mask an extraction that emitted nothing.
    // Multi-package trees keep workspace names out of the declared merge:
    // a member name rides the external tier only through the precedence
    // law's resolved-outside case (R5), never because a manifest named it.
    let manifest = package_manifest(&root);
    let member_names: BTreeSet<String> = members.iter().map(|(_, name)| name.clone()).collect();
    if let Some(manifest) = &manifest {
        external.extend(
            manifest
                .dependencies
                .iter()
                .filter(|name| !member_names.contains(name.as_str()))
                .cloned(),
        );
    }
    // Per-unit manifests: every member's own manifest facts (the
    // manifest-fact tier the R2 conformance fixture exercises) plus the
    // root package's row (C2): every materialized package has an entry, so
    // a package that declares no packages reports an empty dependency
    // list rather than no row at all.
    let mut unit_manifests: BTreeMap<String, ManifestInfo> = BTreeMap::new();
    if let Some(info) = &manifest {
        unit_manifests.insert(model_unit_name(&root), info.clone());
    }
    let mut member_manifests: BTreeMap<PathBuf, ManifestInfo> = BTreeMap::new();
    for (dir, name) in &members {
        if let Some(info) = package_manifest(dir) {
            external.extend(
                info.dependencies
                    .iter()
                    .filter(|dep| !member_names.contains(dep.as_str()))
                    .cloned(),
            );
            unit_manifests.insert(name.clone(), info.clone());
            member_manifests.insert(dir.clone(), info);
        }
    }
    // Declared package-level dependency edges (R5/R6 union, source (ii)):
    // a dependency naming a workspace member is a package edge beside the
    // file edges the resolved imports produce.
    let mut package_edges: BTreeSet<(String, String)> = BTreeSet::new();
    let root_name = model_unit_name(&root);
    if let Some(info) = &manifest {
        for dep in &info.dependencies {
            if member_names.contains(dep) {
                package_edges.insert((root_name.clone(), dep.clone()));
            }
        }
    }
    for (dir, name) in &members {
        let Some(info) = package_manifest(dir) else {
            continue;
        };
        for dep in &info.dependencies {
            if member_names.contains(dep) && dep != name {
                package_edges.insert((name.clone(), dep.clone()));
            }
        }
    }
    // Multi-package trees are MULTI-UNIT (R5): the root keeps its unit and
    // every member dir under the root is a unit of its own. A file
    // attributes to the deepest owning dir along its path.
    let mut units = vec![unit.clone()];
    let mut owners: Vec<(PathBuf, String)> = Vec::new();
    for (dir, name) in &members {
        units.push(Unit {
            name: name.clone(),
            kind: "package".to_string(),
            path: relative(dir, &root).unwrap_or_else(|| name.clone()),
            root: String::new(),
            crate_ids: BTreeSet::from([name.clone()]),
        });
        owners.push((dir.clone(), name.clone()));
    }
    owners.sort_by(|left, right| right.0.cmp(&left.0));
    let file_unit = |file: &str| -> String {
        let path = root.join(file);
        owners
            .iter()
            .find(|(dir, _)| path.starts_with(dir))
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| unit.name.clone())
    };
    // C2 test-tier facts: a file carries the tier only when framework
    // evidence and naming convention co-occur. The evidence is the owning
    // package's manifest declaring a named runner (the closed vocabulary,
    // csharp-`is_test_framework_package` parity); the convention is the
    // `*.test.*` / `*.spec.*` / `__tests__/` shape. The name alone never
    // carries the tier — a package without a runner keeps its
    // `LegacyTests.ts` in production. The files stay nodes and import
    // sources (unlike the go file-tier drop): the serialized model is
    // unchanged, and the verify graph excludes the tier through the same
    // `is_test_gated` channel rust's `cfg(test)` set uses.
    let mut manifests_by_unit: BTreeMap<&str, &ManifestInfo> = BTreeMap::new();
    if let Some(info) = &manifest {
        manifests_by_unit.insert(unit.name.as_str(), info);
    }
    for (dir, name) in &members {
        if let Some(info) = member_manifests.get(dir) {
            manifests_by_unit.insert(name.as_str(), info);
        }
    }
    let test_gated_modules: BTreeSet<String> = files
        .iter()
        .filter(|file| test_tier_convention(file))
        .filter(|file| {
            manifests_by_unit
                .get(file_unit(file).as_str())
                .is_some_and(|info| declares_test_framework(&info.dependencies))
        })
        .cloned()
        .collect();
    // C4 root-barrel roles: the package-root `index.ts` of every unit, when
    // its facts say pure republication (the loop above). The role key is
    // the file identity the module edges use, so `check_facade_roots`
    // addresses it exactly as it addresses rust root module paths.
    let mut roles: BTreeMap<String, crate::archspec::model::Role> = BTreeMap::new();
    for dir in std::iter::once(&root).chain(members.iter().map(|(dir, _)| dir)) {
        let Some(rel) = relative(dir, &root).or_else(|| {
            (dir == &root).then(String::new)
        }) else {
            continue;
        };
        let barrel = if rel.is_empty() {
            "index.ts".to_string()
        } else {
            format!("{rel}/index.ts")
        };
        if pure_republication.contains(&barrel) {
            roles.insert(
                barrel,
                crate::archspec::model::Role::Facade,
            );
        }
    }
    // C2/R8 scan-side vacuity carrier: the `scan.unit_manifests` row has no
    // constraint to carry its vacuity, so the empty-evidence verdict rides
    // the named non-silent report channel — the same channel as the R7b
    // lines, never a finding, never red, never persisted. A tree whose
    // packages state no manifest at all must not scan to a silent green.
    // (`report_only` was taken before the tripwire above; the engine adds
    // nothing after the walk.)
    if unit_manifests.is_empty() {
        report_only
            .push("vacuous constraint: scan.unit_manifests: no package manifest states dependencies".to_string());
    }

    let module_edges: Vec<ModuleEdge> = rows
        .iter()
        .map(
            |((from, to, type_only), symbols)| ModuleEdge {
                unit: file_unit(from),
                from: from.clone(),
                to: to.clone(),
                symbols: symbols.iter().cloned().collect(),
                type_only: *type_only,
            },
        )
        .collect();
    let edges: Vec<crate::archspec::model::Edge> = package_edges
        .iter()
        .map(|(from, to)| crate::archspec::model::Edge {
            from: from.clone(),
            to: to.clone(),
        })
        .collect();
    let mut declarations: BTreeMap<String, Vec<ModuleDeclaration>> = BTreeMap::new();
    for file in &files {
        let owner = file_unit(file);
        declarations
            .entry(owner)
            .or_default()
            .push(ModuleDeclaration {
                name: file.clone(),
                gated: false,
                feature: None,
                file: file.clone(),
            });
    }

    let model = Model {
        units,
        edges,
        external: external.iter().cloned().collect(),
        module_edges,
        manifest,
        module_public_types: exports
            .iter()
            .map(|(file, names)| (file.clone(), names.iter().cloned().collect()))
            .collect(),
        module_external: module_external
            .iter()
            .map(|(file, names)| (file.clone(), names.iter().cloned().collect()))
            .collect(),
        root_module_declarations: declarations,
        dynamic_import_wildcards: wildcards
            .iter()
            .map(|(file, expressions)| (file.clone(), expressions.iter().cloned().collect()))
            .collect(),
        unit_manifests,
        test_gated_modules,
        roles,
        // The `external not installed` lines ride the transient channel
        // (R7b): stderr via the report renderer, never the machine body.
        report_only_lines: report_only,
        ..Model::new("ts")
    };
    Ok(model)
}

/// The tripwire law (D2): scannable files with parsed import statements
/// whose model states no edge, wildcard OR external fact means extraction
/// saw nothing — a parser that silently states nothing cannot pass green.
/// External facts count as stated facts: a tree importing only npm packages
/// is honest, not silence (the wildcard-only-green law of R2-6 is its
/// sibling). Report-only lines state an outcome too (round 4 C virtual law:
/// never a verdict), so they satisfy the tripwire as well.
/// Pure for the unit test; the assembly calls it after the walk.
fn tripwire_error(
    files: usize,
    import_statements: usize,
    has_edges: bool,
    has_wildcards: bool,
    has_external: bool,
    has_report_only: bool,
) -> Option<String> {
    if files > 0
        && import_statements > 0
        && !has_edges
        && !has_wildcards
        && !has_external
        && !has_report_only
    {
        return Some(format!(
            "ts driver tripwire: {files} scanned files carry {import_statements} import statements, yet the model states no edge, wildcard or external fact"
        ));
    }
    None
}

/// The orphan census (D5: a pure helper beside the assembly; the scan
/// report surface renders the lines). The predicate is V2 model-
/// invisibility (PL2): a walk-set file the model lists NO fact for. A
/// fact is membership in exactly three sets — an endpoint (EITHER end)
/// of a module edge, a key of `module_external`, or a key of
/// `dynamic_import_wildcards`. The export-symbol tier
/// (`module_public_types`) is NOT a fact set: presence there neither
/// suppresses an orphan line nor is denied by one. Returned lines are
/// sorted and deduped (the D1 rendering law).
fn orphan_census_lines(
    files: &[String],
    rows: &BTreeMap<(String, String, bool), BTreeSet<String>>,
    module_external: &BTreeMap<String, BTreeSet<String>>,
    wildcards: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    let mut fact_files: BTreeSet<&str> = BTreeSet::new();
    for (from, to, _) in rows.keys() {
        fact_files.insert(from);
        fact_files.insert(to);
    }
    fact_files.extend(module_external.keys().map(String::as_str));
    fact_files.extend(wildcards.keys().map(String::as_str));
    let mut orphans: Vec<&str> = files
        .iter()
        .map(String::as_str)
        .filter(|file| !fact_files.contains(file))
        .collect();
    orphans.sort_unstable();
    orphans.dedup();
    orphans.into_iter().map(|file| format!("orphan file: {file}")).collect()
}


/// The scanned source files, repo-relative, git spelling, sorted. Excluded
/// trees contribute no files (D1/D2/D14).
fn collect_files(root: &Path, declared: Option<&Exclusions>) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !excluded_walk(entry.path(), root, declared))
    {
        let entry = entry.map_err(|err| format!("cannot scan tree: {err}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(rel) = relative(entry.path(), root) else {
            continue;
        };
        if source_extension(Path::new(&rel)).is_some() {
            files.push(rel);
        }
    }
    files.sort();
    Ok(files)
}

fn excluded_walk(path: &Path, root: &Path, declared: Option<&Exclusions>) -> bool {
    // The absolute classes carry over the whole path (segments ABOVE the
    // scan root exclude too); the VENDORED class narrows to nothing —
    // it never excludes in the ts scan (R11, one law).
    if path
        .components()
        .any(|component| matches!(component, Component::Normal(name)
            if language::is_excluded_dir(&name.to_string_lossy())))
    {
        return true;
    }
    relative(path, root)
        .map(|rel| {
            let rel = Path::new(&rel);
            mechanically_excluded(rel)
                || declared.is_some_and(|exclusions| {
                    exclusions.declared.iter().any(|entry| rel.starts_with(entry))
                })
        })
        .unwrap_or(false)
}

fn source_extension(path: &Path) -> Option<&'static str> {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .and_then(|extension| {
            TS_SOURCE_EXTS
                .iter()
                .find(|known| **known == extension.as_str())
                .copied()
        })
}

/// The facts of one file: source read, parse per shape — a `.svelte` file
/// parses every `<script>` block with the TypeScript grammar (ADR-022).
fn file_facts(root: &Path, rel: &str) -> Result<syntax::FileFacts, String> {
    let path = root.join(rel);
    let source = std::fs::read_to_string(&path)
        .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    let mut merged = syntax::FileFacts::default();
    if Path::new(rel).extension().is_some_and(|e| e.eq_ignore_ascii_case("svelte")) {
        for block in syntax::svelte_script_blocks(&source) {
            let facts = syntax::scan_file(&path, &block)
                .map_err(|err| format!("{rel}: {err}"))?;
            merged.edges.extend(facts.edges);
            merged.wildcards.extend(facts.wildcards);
            merged.exports.extend(facts.exports);
            merged.import_statements += facts.import_statements;
        }
    } else {
        merged = syntax::scan_file(&path, &source).map_err(|err| format!("{rel}: {err}"))?;
    }
    Ok(merged)
}

/// One specifier's resolution verdict.
fn resolve(
    root: &Path,
    from: &str,
    edge: &syntax::EdgeSpec,
    declared: Option<&Exclusions>,
    engine: &mut resolution::Engine,
) -> Resolved {
    let specifier = edge.specifier.trim();
    if specifier.is_empty() {
        return Resolved::Unresolvable(String::new());
    }
    let relative_spec =
        specifier.starts_with("./") || specifier.starts_with("../") || specifier == "." || specifier == "..";
    if !relative_spec {
        if specifier.starts_with('/') {
            // An absolute specifier names no first-party file in a tree.
            return Resolved::Unresolvable(String::new());
        }
        // The seam: builtins and `node:` stay external by fact, everything
        // else delegates through the entry's cached resolver (resolution.rs).
        return engine.classify_bare(&root.join(from), specifier);
    }
    let base = match joined(Path::new(from), strip_query(specifier)) {
        Some(base) => base,
        None => return Resolved::Unresolvable(String::new()),
    };
    // Ordinary resolution first: a candidate outside the exclusions wins.
    let mut excluded_hit: Option<String> = None;
    for candidate in candidates(&base) {
        if !root.join(&candidate).is_file() {
            continue;
        }
        if !is_excluded(Path::new(&candidate), declared) {
            return Resolved::Edge(candidate);
        }
        excluded_hit.get_or_insert_with(|| exclusion_hit(Path::new(&candidate), declared));
    }
    match excluded_hit {
        Some(exclusion) => Resolved::Unresolvable(format!(
            " — resolves only into excluded path {exclusion}"
        )),
        None => engine
            .reported_generated_relative(&base, specifier)
            .unwrap_or_else(|| Resolved::Unresolvable(String::new())),
    }
}

/// The candidate spellings: the exact path when it carries a known
/// extension (a source file or an asset), the NodeNext extension
/// substitution when the literal spelling ends in `.js`/`.mjs`/`.cjs`
/// (record R10, round 4 B — after the literal, never before it), extension
/// inference over the ts source set, and the index law (`dir/x` also tries
/// `dir/x/index.*`).
fn candidates(base: &Path) -> Vec<String> {
    let mut candidates = Vec::new();
    let has_known_extension = |path: &Path| -> bool {
        path.extension()
            .map(|extension| {
                let extension = extension.to_string_lossy().to_lowercase();
                TS_SOURCE_EXTS.contains(&extension.as_str()) || ASSET_EXTS.contains(&extension.as_str())
            })
            .unwrap_or(false)
    };
    if has_known_extension(base) {
        candidates.push(spell(base));
    }
    for substituted in nodext_substitutions(base) {
        if !candidates.contains(&substituted) {
            candidates.push(substituted);
        }
    }
    for extension in TS_SOURCE_EXTS {
        let mut with_extension = base.as_os_str().to_os_string();
        with_extension.push(".");
        with_extension.push(*extension);
        candidates.push(spell(Path::new(&with_extension)));
    }
    for extension in TS_SOURCE_EXTS {
        let mut index = base.as_os_str().to_os_string();
        index.push("/index.");
        index.push(*extension);
        candidates.push(spell(Path::new(&index)));
    }
    candidates
}

/// The NodeNext extension substitutions (record R10, round 4 B): a base
/// spelled `.js` also tries `.ts` and `.tsx`; `.mjs` tries `.mts`; `.cjs`
/// tries `.cts`. The literal spelling stays FIRST in every row — when both
/// `x.js` and `x.ts` exist, the runtime loads `x.js` and the edge lands
/// there (the mixed-tree law). `.mjs`/`.cjs` literals carry a known
/// extension since the indexing law, so the literal rides the exact-path
/// leg and the substitution rows repeat it after the dedup. No `.svelte`
/// row exists in any mapping. The oxc-side `extension_alias` table stays
/// DISABLED: it fires before the literal and errors on twin-less trees
/// (R10 Rejected, round-4 measurements).
fn nodext_substitutions(base: &Path) -> Vec<String> {
    let Some(extension) = base.extension() else {
        return Vec::new();
    };
    let extension = extension.to_string_lossy().to_lowercase();
    let row: &[&str] = match extension.as_str() {
        "js" => &["ts", "tsx"],
        "mjs" => &["mjs", "mts"],
        "cjs" => &["cjs", "cts"],
        _ => &[],
    };
    row.iter()
        .map(|target| {
            let substituted = base.with_extension(target);
            spell(&substituted)
        })
        .collect()
}

/// `from_dir + specifier` normalized component-wise; `..` steps that leave
/// the tree root resolve to nothing.
fn joined(from: &Path, specifier: &str) -> Option<PathBuf> {
    let mut path: PathBuf = from.parent().map(Path::to_path_buf).unwrap_or_default();
    for component in Path::new(specifier).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !path.pop() {
                    return None; // above the tree root: no first-party file.
                }
            }
            Component::Normal(part) => path.push(part),
            _ => return None,
        }
    }
    Some(path)
}

fn package_name(root: &Path) -> Option<String> {
    package_json(root).and_then(|value| {
        value
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
    })
}

/// The registered members that live INSIDE the scanned root (multi-unit
/// territory, R5): sorted by directory, deepest last. Members outside the
/// root (a scan rooted inside one member looking at the workspace above)
/// own no scanned file and contribute no unit.
fn members_under_root(
    registries: &[workspace::Registry],
    root: &Path,
) -> Vec<(PathBuf, String)> {
    let mut members: Vec<(PathBuf, String)> = Vec::new();
    for registry in registries {
        for member in &registry.members {
            if member.dir.starts_with(root) && member.dir != root {
                members.push((member.dir.clone(), member.name.clone()));
            }
        }
    }
    members.sort();
    members.dedup_by(|left, right| left.0 == right.0);
    members
}

fn package_unit(root: &Path) -> Unit {
    // D9: the unit root is the package root. The name is the package.json
    // name when stated; the fallback is the root path spelling "." — always
    // byte-stable across clones (D4), never a directory name from the host.
    let name = package_name(root).unwrap_or_else(|| ".".to_string());
    Unit {
        name: name.clone(),
        kind: "package".to_string(),
        path: ".".to_string(),
        root: String::new(),
        crate_ids: BTreeSet::from([name]),
    }
}

fn model_unit_name(root: &Path) -> String {
    package_name(root).unwrap_or_else(|| ".".to_string())
}

fn package_manifest(root: &Path) -> Option<ManifestInfo> {
    let value = package_json(root)?;
    let mut dependencies: BTreeSet<String> = BTreeSet::new();
    for field in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(entries) = value.get(field).and_then(Value::as_object) {
            dependencies.extend(entries.keys().cloned());
        }
    }
    Some(ManifestInfo {
        publish: None,
        dependencies: dependencies.into_iter().collect(),
        features: Vec::new(),
    })
}

fn package_json(root: &Path) -> Option<serde_json::Value> {
    std::fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

/// The C2 naming convention half of the test-tier predicate: `*.test.*`,
/// `*.spec.*` or a `__tests__/` segment. The convention alone carries no
/// tier — it is ANDed with the manifest's framework evidence below, the
/// csharp `is_test_project` shape transposed from project tier to file
/// tier (C2: name-only classification is the mistake the matrix cell
/// exists to forbid).
fn test_tier_convention(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    name.contains(".test.")
        || name.contains(".spec.")
        || file.split('/').any(|segment| segment == "__tests__")
}

/// The C2 evidence half: the closed runner vocabulary whose declaration in a
/// package's manifest states that a test runner is present. C# names its
/// runners the same way (`is_test_framework_package`); extending this list
/// is a capability-row change of its own, never a drive-by.
fn declares_test_framework(dependencies: &[String]) -> bool {
    dependencies
        .iter()
        .any(|dependency| dependency == "vitest" || dependency == "jest")
}

#[cfg(test)]
mod tests {
    use super::common::package_identity;
    use super::*;

    #[test]
    fn tripwire_fires_only_on_total_silence() {
        // Files with imports, no stated fact of any kind — the outlawed
        // silent parser (D2).
        assert!(tripwire_error(3, 5, false, false, false, false).is_some());
        // Every stated fact keeps the run green: edges, wildcards (R2-6),
        // externals (an honest npm-only tree states a fact) or report-only
        // lines (round 4 C: the virtual arm states an outcome, not silence).
        assert!(tripwire_error(3, 5, true, false, false, false).is_none());
        assert!(tripwire_error(3, 5, false, true, false, false).is_none());
        assert!(tripwire_error(3, 5, false, false, true, false).is_none());
        assert!(tripwire_error(3, 5, false, false, false, true).is_none());
        // No imports at all: nothing to state, nothing to trip.
        assert!(tripwire_error(3, 0, false, false, false, false).is_none());
        assert!(tripwire_error(0, 0, false, false, false, false).is_none());
    }

    /// D7 (the N1 adopted fix): require sites are import statements — the
    /// counter counts them, so a require-only tree whose extraction silently
    /// broke trips BY NAME instead of reading clean. This is the plan's
    /// "require-only tree with resolution stubbed to zero facts": the pure
    /// law receives the parsed tree's real counter and zero stated facts.
    #[test]
    fn require_sites_count_in_the_tripwire_counter() {
        let facts = syntax::scan_file(
            Path::new("only-require.cjs"),
            "const a = require('./a');\nrequire(workerData.modulePath);\n",
        )
        .expect("the grammar parses the fixture");
        assert_eq!(facts.import_statements, 2, "both require sites count (literal and computed)");
        let trip = tripwire_error(1, facts.import_statements, false, false, false, false)
            .expect("a broken extraction on a require-only tree must trip (N1/D7)");
        assert!(
            trip.contains("2 import statements"),
            "the named tripwire line states the require sites: {trip}"
        );
    }

    /// B1 (the D7 ordering law, the census decision-check's test gap 1):
    /// a dead extractor cannot launder itself green through the census.
    /// The same zero-facts tree the tripwire names now ALSO produces
    /// orphan lines — the tripwire's `has_report_only` arm is evaluated
    /// BEFORE the census runs, so the census lines never satisfy it and
    /// the tripwire fires exactly as before the census existed.
    #[test]
    fn census_lines_never_satisfy_the_tripwire_arms() {
        let files = vec!["a.ts".to_string(), "b.ts".to_string()];
        let rows = BTreeMap::new();
        let external = BTreeMap::new();
        let wildcards = BTreeMap::new();
        // The dead extractor: two files, parsed import statements, zero
        // stated facts — the census would list BOTH files, yet the
        // tripwire call site passes `has_report_only` from the ENGINE
        // lines only (empty here), so the run stays red by name.
        let orphans = orphan_census_lines(&files, &rows, &external, &wildcards);
        assert_eq!(
            orphans,
            vec!["orphan file: a.ts", "orphan file: b.ts"],
            "the census lists every zero-fact file, sorted (D1)"
        );
        let trip = tripwire_error(2, 2, false, false, false, false)
            .expect("census lines are not a stated fact: the tripwire still fires");
        assert!(trip.contains("ts driver tripwire"), "named tripwire: {trip}");
    }

    /// PL2: a fact is membership in one of the THREE fact sets — an
    /// endpoint (EITHER end) of an edge, an external key, or a wildcard
    /// key. The helper takes only those sets: the export-symbol tier is
    /// not an input and can never suppress an orphan line.
    #[test]
    fn the_census_predicate_is_the_three_fact_sets() {
        let files = vec![
            "edge.ts".to_string(),
            "ext.ts".to_string(),
            "wild.ts".to_string(),
            "none.ts".to_string(),
        ];
        let mut rows = BTreeMap::new();
        rows.insert(
            ("edge.ts".to_string(), "target.ts".to_string(), false),
            BTreeSet::from(["t".to_string()]),
        );
        let mut external = BTreeMap::new();
        external.insert("ext.ts".to_string(), BTreeSet::from(["lodash".to_string()]));
        let mut wildcards = BTreeMap::new();
        wildcards.insert("wild.ts".to_string(), BTreeSet::from(["expr".to_string()]));
        let orphans = orphan_census_lines(&files, &rows, &external, &wildcards);
        assert_eq!(
            orphans,
            vec!["orphan file: none.ts"],
            "edge endpoints (either end), external keys and wildcard keys all state facts"
        );
    }

    #[test]
    fn package_identity_folds_paths_keeps_scopes_names_builtins() {
        assert_eq!(package_identity("lodash"), "lodash");
        assert_eq!(package_identity("@org/pkg/sub/module"), "@org/pkg");
        assert_eq!(package_identity("node:fs"), "node:fs");
        assert_eq!(package_identity("node:fs/promises"), "node:fs");
        assert_eq!(package_identity("axios"), "axios");
    }

    #[test]
    fn svelte_blocks_open_on_tag_boundaries_only() {
        let source = "<script>import \"a\";</script><p>a &lt;script&gt; is markup</p>\n<script context=\"module\">import \"b\";</script>";
        let blocks = syntax::svelte_script_blocks(source);
        assert_eq!(blocks, vec!["import \"a\";", "import \"b\";"]);
    }

    #[test]
    fn exclusion_hit_names_the_declared_entry_before_the_mechanical_dir() {
        let declared = Exclusions {
            declared: vec![PathBuf::from("src/vendor")],
        };
        assert_eq!(
            exclusion_hit(Path::new("src/vendor/legacy.ts"), Some(&declared)),
            "src/vendor"
        );
        assert_eq!(
            exclusion_hit(Path::new("node_modules/pkg/index.ts"), Some(&declared)),
            "node_modules"
        );
    }
}
