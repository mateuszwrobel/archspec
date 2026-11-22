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
use common::{
    exclusion_verdict, is_excluded, candidates, relative, strip_query,
    TS_SOURCE_EXTS, Exclusions, Resolved,
};

use crate::language;
use crate::model::{LiteralCandidate, ManifestInfo, Model, ModuleDeclaration, ModuleEdge, Unit};
use crate::model::DeclaredFacts;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

pub fn extract(root: &Path, facts: &DeclaredFacts) -> Result<Model, String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    // The declared exclusions and framework ride the DeclaredFacts seam: the
    // spec at the scanned root is loaded by the caller (verbs, inspect arm)
    // and mapped here without the library owning the loader (D14(4)).
    let declared = Exclusions::new(&facts.exclusions);
    let files = collect_files(&root, Some(&declared))?;
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
    // The unresolvable-import census entries (visibility phase 2, scenario
    // 01): the class that no longer aborts. The model lands; the lines and
    // the counts ride the transient model field to the verbs. Config faults
    // (the seam's findings below) keep the fail-closed abort byte-identical.
    let mut unresolvables: Vec<crate::model::UnresolvableImport> = Vec::new();
    // The warn-mode exclusion findings (issue-8 phase 2): deduped and sorted
    // (a BTreeSet) so the warning bytes are run-order free, like every other
    // collected report channel. They never join the findings error below —
    // a warn hit never aborts (BLOCKER-2) — and never enter the machine body:
    // they ride the transient warnings channel on the model.
    let mut warnings: BTreeSet<String> = BTreeSet::new();
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
        Some(&declared),
        facts.framework.as_deref(),
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
    // The literal-candidate feed (workplan events_value_join scenario 03,
    // decision-check F8): collected ONLY when a declared pattern side names
    // this driver — the declaration gate is the byte identity (4.2 arm 1),
    // so a non-declaring run reads and parses nothing twice and appends
    // nothing. The sites are vocabulary-free (A6): names ride as spelled,
    // the engine matcher verbs decide equality at the resolution site.
    let collect_candidates = facts
        .patterns
        .iter()
        .any(|pattern| pattern.language == language::Language::Ts.as_str());
    let mut literal_candidates: Vec<LiteralCandidate> = Vec::new();
    let all_facts: Vec<Result<syntax::FileFacts, String>> = crate::profile::phase(
        "scan-facts",
        || {
            if threads > 1 {
                super::leg::parallel_fact_leg(files.len(), threads, |index| {
                    file_facts(&root, &files[index])
                })
            } else {
                files.iter().map(|file| file_facts(&root, file)).collect()
            }
        },
    );

    for (file, facts) in files.iter().zip(all_facts) {
        let facts = facts?;
        if collect_candidates {
            let path = root.join(file);
            if let Ok(source) = std::fs::read_to_string(&path) {
                for (anchor, value) in syntax::literal_candidates(&path, &source) {
                    literal_candidates.push(LiteralCandidate {
                        path: file.clone(),
                        language: language::Language::Ts.as_str().to_string(),
                        anchor,
                        value,
                    });
                }
            }
        }
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
            match resolve(&root, file, edge, Some(&declared), &mut engine) {
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
                    // The census entry (scenario 01): the pre-fix finding
                    // bytes, plus the structured pair the census counts.
                    let line =
                        format!("unresolvable import: {file} -> \"{}\"{why}", edge.specifier);
                    unresolvables.push(crate::model::UnresolvableImport {
                        file: file.clone(),
                        specifier: edge.specifier.clone(),
                        line,
                    });
                }
                // The warn-mode dial (issue-8 phase 2): the hit is stated,
                // never aborted. The finding is pre-rendered in the warnings
                // channel's `<category>: <detail>` form, naming entry AND
                // target (BLOCKER-2): the importing file, the specifier and
                // the specificity-resolved entry. verify folds these into
                // the diff's warnings (--strict fails, else green); scan and
                // the report verbs print them on the report surface.
                Resolved::Warned(detail) => {
                    warnings.insert(format!(
                        "excluded source: {file} -> \"{}\" — {detail}",
                        edge.specifier
                    ));
                }
                Resolved::Reported => {
                    // The virtual-specifier law: the line was recorded by
                    // the seam; the model stays untouched — no edge, no
                    // external-tier entry, no finding.
                }
            }
        }
    }

    // The seam's category lines ride the same channel as the findings
    // above: config faults (tsconfig/manifest homes, unclassified resolver
    // errors) as generated, deduped by the engine. Scenario 01 split: the
    // unresolvable-import class left this abort for the census field —
    // what still aborts here is a RUN PRECONDITION (a config nothing
    // adjudicated against), not a dead edge. The seam's own unresolvable
    // lines (the relative-class misses it renders) join the CENSUS here,
    // ahead of the per-edge entries: every finding of this channel either
    // aborts or becomes a census entry, never a silent drop. The report-only
    // lines are taken here too: a virtual-specifier line is a STATED outcome
    // (round 4 C: the law never reddens a run), so it must also satisfy the
    // silence tripwire below — every other report-only class already rides
    // a fact, so this changes behavior for the `Reported` arm only.
    for finding in engine.take_findings() {
        if let Some(rest) = finding.strip_prefix("unresolvable import: ") {
            let (file, spec) = match rest.split_once(" -> ") {
                Some((file, spec)) => (file.to_string(), spec.to_string()),
                None => continue,
            };
            unresolvables.push(crate::model::UnresolvableImport {
                file,
                specifier: spec.trim_matches('"').to_string(),
                line: finding,
            });
        } else {
            findings.push(finding);
        }
    }
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
        !warnings.is_empty(),
    ) {
        return Err(tripwire);
    }

    // The unresolvable-import census (visibility phase 2, scenario 01):
    // computed STRICTLY AFTER the tripwire, and never itself a tripwire
    // arm (MINOR-2, the D7/B1 ordering law the orphan census above owns) —
    // a findings-only extractor stays red, the census launders nothing
    // green. The entries ride the transient channel like `warnings`: the
    // lines print at the verbs, the counts ride the same list, and no
    // serialized byte ever changes.
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
    // The owner index (perf-hotspots record 2026-11-20, H1): the member dirs
    // key a lookup that walks the file's OWN ancestors and stops at the first
    // member dir it meets. That is the deepest-first law byte-for-byte — two
    // members that both own a path sit on one ancestor chain, so the walk
    // meets the deeper one first, exactly the row the descending scan of
    // every member returned — and a path no member owns still falls back to
    // the root unit. `members_under_root` dedups member dirs, so no key is
    // ever rewritten; where a duplicate could reach here the FIRST member row
    // keeps the dir, which is what the stable sort used to resolve to. Cost
    // per row is the path's depth in lookups, never `members.len()`.
    let mut owner_by_dir: HashMap<&Path, &str> = HashMap::with_capacity(members.len());
    for (dir, name) in &members {
        owner_by_dir
            .entry(dir.as_path())
            .or_insert_with(|| name.as_str());
        units.push(Unit {
            name: name.clone(),
            kind: "package".to_string(),
            path: relative(dir, &root).unwrap_or_else(|| name.clone()),
            root: String::new(),
            crate_ids: BTreeSet::from([name.clone()]),
        });
    }
    let file_unit = |file: &str| -> String {
        let path = root.join(file);
        path.ancestors()
            .find_map(|dir| owner_by_dir.get(dir))
            .map_or_else(|| unit.name.clone(), |name| (*name).to_string())
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
    let mut roles: BTreeMap<String, crate::model::Role> = BTreeMap::new();
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
                crate::model::Role::Facade,
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
    let edges: Vec<crate::model::Edge> = package_edges
        .iter()
        .map(|(from, to)| crate::model::Edge {
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
        literal_candidates,
        // The `external not installed` lines ride the transient channel
        // (R7b): stderr via the report renderer, never the machine body.
        report_only_lines: report_only,
        // The warn-mode exclusion findings ride their own transient channel
        // (issue-8 phase 2): warnings-severity, never the machine body,
        // never the findings error — the record forbids the report-only
        // route precisely because that route can never go red under
        // `--strict`.
        warnings: warnings.into_iter().collect(),
        // The unresolvable-import census entries (scenario 01): transient,
        // never serialized, read by `scan` and `verify` only.
        unresolvable_imports: unresolvables,
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
    has_warnings: bool,
) -> Option<String> {
    if files > 0
        && import_statements > 0
        && !has_edges
        && !has_wildcards
        && !has_external
        && !has_report_only
        && !has_warnings
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
    collect_files_recorded(root, declared, None)
}

/// The collection walk with an optional visit ledger (the test seam the
/// pruning cell reads): every path the exclusion filter evaluates rides
/// `visits`, repo-relative. `None` — every shipped path — carries no ledger.
fn collect_files_recorded(
    root: &Path,
    declared: Option<&Exclusions>,
    mut visits: Option<&mut Vec<String>>,
) -> Result<Vec<String>, String> {
    // The H6 cure (probe record 2026-11-20): the declared table is indexed
    // ONCE per walk, so each visited path pays one O(depth) structural
    // descent instead of one comparison per declared entry per path.
    let index = declared.map(|exclusions| DeclaredIndex::new(&exclusions.declared));
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            if let Some(ledger) = visits.as_deref_mut()
                && let Some(rel) = relative(entry.path(), root)
            {
                ledger.push(rel);
            }
            !excluded_walk(entry.path(), root, index.as_ref())
        })
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

/// One component-trie node: `terminal` flags an entry path ending here;
/// `edges` keys the next component by exact OsStr bytes — the
/// `Path::starts_with` equality law (case-sensitive, never prefix-text).
#[derive(Default)]
struct TrieNode<'a> {
    terminal: bool,
    edges: HashMap<&'a OsStr, u32>,
}

/// The declared `excluded_sources` table seen as a component trie (the H6
/// cure). [`DeclaredIndex::matches`] answers "some entry path is a
/// component-wise prefix of this path" in one O(depth) descent — the same
/// verdict the linear `rel.starts_with(&entry.path)` scan gave: the entry
/// `web` matches `web` and `web/app.ts`, never `webapp/app.ts`, and a
/// matched directory is pruned by the walker before its subtree is read.
struct DeclaredIndex<'a> {
    nodes: Vec<TrieNode<'a>>,
    /// Entries carrying a non-Normal component (the loader refuses them at
    /// load; the linear law stays their home so no verdict can drift).
    literal: Vec<&'a Path>,
}

impl<'a> DeclaredIndex<'a> {
    fn new(entries: &'a [crate::model::ExclusionEntry]) -> Self {
        let mut index = Self {
            nodes: vec![TrieNode::default()],
            literal: Vec::new(),
        };
        for entry in entries {
            let mut node = 0usize;
            let mut trie_home = true;
            for component in entry.path.components() {
                let Component::Normal(name) = component else {
                    trie_home = false;
                    break;
                };
                let known = index.nodes[node].edges.get(name).copied();
                node = match known {
                    Some(next) => next as usize,
                    None => {
                        index.nodes.push(TrieNode::default());
                        let next = index.nodes.len() as u32 - 1;
                        index.nodes[node].edges.insert(name, next);
                        next as usize
                    }
                };
            }
            if trie_home {
                index.nodes[node].terminal = true;
            } else {
                index.literal.push(&entry.path);
            }
        }
        index
    }

    /// True when one entry path component-prefixes `rel` — byte-identical
    /// to `entries.iter().any(|entry| rel.starts_with(&entry.path))`.
    fn matches(&self, rel: &Path) -> bool {
        if self.nodes[0].terminal {
            // An empty entry path prefixes every path (the starts_with law).
            return true;
        }
        let mut node = 0usize;
        for component in rel.components() {
            let Component::Normal(name) = component else {
                return self.literal.iter().any(|entry| rel.starts_with(*entry));
            };
            let Some(&next) = self.nodes[node].edges.get(name) else {
                return self.literal.iter().any(|entry| rel.starts_with(*entry));
            };
            node = next as usize;
            if self.nodes[node].terminal {
                return true;
            }
        }
        false
    }
}

fn excluded_walk(path: &Path, root: &Path, declared: Option<&DeclaredIndex<'_>>) -> bool {
    // The absolute classes carry over the whole path (segments ABOVE the
    // scan root exclude too); the VENDORED class narrows to nothing —
    // it never excludes in the ts scan (R11, one law). A relative path's
    // components are a suffix of this path's components, so this one scan
    // covers the mechanical set for both spellings — the old second
    // `mechanically_excluded(rel)` pass (with its per-component String
    // allocations) decided nothing the scan above did not (the probe
    // record's rider).
    if path
        .components()
        .any(|component| matches!(component, Component::Normal(name)
            if language::is_excluded_dir(&name.to_string_lossy())))
    {
        return true;
    }
    // The declared table answers through the component trie (H6 cure).
    relative(path, root)
        .is_some_and(|rel| declared.is_some_and(|index| index.matches(Path::new(&rel))))
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
    // The H2 resolve-replay memo (decision-check 2026-11-22, §b): the lookup
    // sits AT this call site — after the exclusion-filtered file walk has
    // produced the fold's file list, in walk order — there is NO
    // pre-population pass, so files past the first parse error still owe
    // nothing (F14) and excluded trees never feed the memo (the H7 phantom
    // class). The key is (importing dir, spelled specifier): the dir is the
    // value `joined` reads, and the SPELLED specifier rides because the R12
    // report lines embed it (B1 law). A hit replays the verdict with zero
    // further debt: the candidate walk, the exclusion verdict and the R12
    // probe are functions of the key over the committed tree, and every
    // channel the body writes is a deduped set (`bare` memo law).
    let dir = Path::new(from).parent().unwrap_or(Path::new(""));
    if let Some(verdict) = engine.replay_relative(dir, specifier) {
        return verdict;
    }
    let verdict = resolve_relative(root, Path::new(from), specifier, declared, engine);
    engine.store_relative(dir, specifier, &verdict);
    verdict
}

/// The relative-arm body, computed exactly once per distinct key: ordinary
/// resolution first, the excluded-hit dial second, the R12 generated/NAPI
/// probe last. Bytes unchanged from the pre-memo code — the memo only calls
/// it less often.
fn resolve_relative(
    root: &Path,
    from: &Path,
    specifier: &str,
    declared: Option<&Exclusions>,
    engine: &mut resolution::Engine,
) -> Resolved {
    let base = match joined(from, strip_query(specifier)) {
        Some(base) => base,
        None => return Resolved::Unresolvable(String::new()),
    };
    // Ordinary resolution first: a candidate outside the exclusions wins.
    let mut excluded_hit: Option<(String, &'static str)> = None;
    for candidate in candidates(&base) {
        if !root.join(&candidate).is_file() {
            continue;
        }
        if !is_excluded(Path::new(&candidate), declared) {
            return Resolved::Edge(candidate);
        }
        excluded_hit.get_or_insert_with(|| exclusion_verdict(Path::new(&candidate), declared));
    }
    match excluded_hit {
        // The dial (issue-8 phase 2): `warn` states the hit, never aborts;
        // `abort` — the default — keeps the shipped line byte-identical.
        Some((exclusion, "warn")) => {
            Resolved::Warned(format!("resolves only into excluded path {exclusion}"))
        }
        Some((exclusion, _)) => Resolved::Unresolvable(format!(
            " — resolves only into excluded path {exclusion}"
        )),
        None => engine
            .reported_generated_relative(&base, specifier)
            .unwrap_or_else(|| Resolved::Unresolvable(String::new())),
    }
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

/// The driver-port seam entry (workplan events_value_join scenario 10):
/// a unit struct implementing `crate::driver::Driver` by calling this
/// module's own [`extract`] — body logic moves zero lines.
pub(crate) struct TsDriver;

impl crate::driver::Driver for TsDriver {
    fn extract(&self, root: &Path, declared: &DeclaredFacts) -> Result<Model, String> {
        extract(root, declared)
    }
}

#[cfg(test)]
mod tests {
    use super::common::exclusion_hit;
    use super::common::package_identity;
    use super::*;

    #[test]
    fn tripwire_fires_only_on_total_silence() {
        // Files with imports, no stated fact of any kind — the outlawed
        // silent parser (D2).
        assert!(tripwire_error(3, 5, false, false, false, false, false).is_some());
        // Every stated fact keeps the run green: edges, wildcards (R2-6),
        // externals (an honest npm-only tree states a fact) or report-only
        // lines (round 4 C: the virtual arm states an outcome, not silence).
        assert!(tripwire_error(3, 5, true, false, false, false, false).is_none());
        assert!(tripwire_error(3, 5, false, true, false, false, false).is_none());
        assert!(tripwire_error(3, 5, false, false, true, false, false).is_none());
        assert!(tripwire_error(3, 5, false, false, false, true, false).is_none());
        // A warn-mode exclusion finding is a STATED outcome too (issue-8
        // phase 2): the seam recorded the hit, so silence did not happen.
        assert!(tripwire_error(3, 5, false, false, false, false, true).is_none());
        // No imports at all: nothing to state, nothing to trip.
        assert!(tripwire_error(3, 0, false, false, false, false, false).is_none());
        assert!(tripwire_error(0, 0, false, false, false, false, false).is_none());
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
        let trip = tripwire_error(1, facts.import_statements, false, false, false, false, false)
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
        let trip = tripwire_error(2, 2, false, false, false, false, false)
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
            declared: vec![crate::model::ExclusionEntry {
                path: PathBuf::from("src/vendor"),
                mode: "abort",
            }],
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

    /// The walk-side twin of language.rs's pinned census law: the declared
    /// component trie matches by components only, never by prefix text.
    #[test]
    fn declared_index_matches_componentwise_never_by_prefix_text() {
        let entries: Vec<crate::model::ExclusionEntry> = ["web", "gen/r000/deep", "a/b.ts"]
            .iter()
            .map(|path| crate::model::ExclusionEntry {
                path: PathBuf::from(path),
                mode: "abort",
            })
            .collect();
        let index = DeclaredIndex::new(&entries);
        // The entry `web` matches its own root and its subtree…
        assert!(index.matches(Path::new("web")));
        assert!(index.matches(Path::new("web/app.ts")));
        // …and NEVER the look-alike sibling (component-wise, not prefix-text).
        assert!(!index.matches(Path::new("webapp/app.ts")));
        // A declared root only prefixes: mid-path occurrences stay in scope.
        assert!(!index.matches(Path::new("src/web/app.ts")));
        // Deep entries: the subtree matches, the parent and the sibling do not.
        assert!(index.matches(Path::new("gen/r000/deep/a/b/c.ts")));
        assert!(!index.matches(Path::new("gen/r000")));
        assert!(!index.matches(Path::new("gen/r000x/deep/a.ts")));
        // A file-level entry matches itself, never a look-alike file.
        assert!(index.matches(Path::new("a/b.ts")));
        assert!(!index.matches(Path::new("a/b.tsx")));
        assert!(!index.matches(Path::new("")));
    }

    /// The H6 pruning cell (probe record 2026-11-20): with a large declared
    /// table the walk must PRUNE every declared root at the root itself —
    /// no path strictly inside a declared subtree is ever evaluated. The
    /// visit ledger decides; no wall clock is read, so no flake is possible.
    #[test]
    fn excluded_walk_prunes_declared_subtrees_without_visiting_them() {
        let scratch = tempfile::TempDir::new().expect("scratch tree");
        let root = scratch.path();
        let mut entries = Vec::new();
        for n in 0..300usize {
            let dir = root.join(format!("gen/r{n:03}/deep/a/b"));
            std::fs::create_dir_all(&dir).expect("deep dir");
            std::fs::write(dir.join("c.ts"), "export const x = 1;\n").expect("deep file");
            entries.push(crate::model::ExclusionEntry {
                path: PathBuf::from(format!("gen/r{n:03}")),
                mode: "abort",
            });
        }
        // Component-wise look-alikes of two declared roots stay whole:
        // `gen/r300` never reaches `gen/r300x`, `web` never reaches `webapp`.
        std::fs::create_dir_all(root.join("gen/r300x/keep")).expect("look-alike dir");
        std::fs::write(root.join("gen/r300x/keep/k.ts"), "export const k = 1;\n")
            .expect("look-alike file");
        std::fs::create_dir_all(root.join("webapp")).expect("webapp dir");
        std::fs::write(root.join("webapp/keep.ts"), "export const w = 1;\n").expect("webapp file");
        std::fs::create_dir_all(root.join("src")).expect("src dir");
        std::fs::write(root.join("src/keep.ts"), "export const s = 1;\n").expect("src file");
        entries.push(crate::model::ExclusionEntry {
            path: PathBuf::from("gen/r300"),
            mode: "abort",
        });
        entries.push(crate::model::ExclusionEntry {
            path: PathBuf::from("web"),
            mode: "abort",
        });
        let declared = Exclusions { declared: entries };
        let mut visits: Vec<String> = Vec::new();
        let files =
            collect_files_recorded(root, Some(&declared), Some(&mut visits)).expect("collection");
        assert_eq!(
            files,
            vec![
                "gen/r300x/keep/k.ts".to_string(),
                "src/keep.ts".to_string(),
                "webapp/keep.ts".to_string(),
            ],
            "the 301 declared subtrees contribute no files; the look-alikes stay whole"
        );
        // THE PRUNING LAW: the filter never evaluated a path strictly INSIDE
        // a declared root — the walk met each declared directory itself (the
        // prune point) and never descended.
        let inside: Vec<&String> = visits
            .iter()
            .filter(|visit| {
                let visited = Path::new(visit.as_str());
                declared
                    .declared
                    .iter()
                    .any(|entry| visited.starts_with(&entry.path) && visited != entry.path)
            })
            .collect();
        assert!(
            inside.is_empty(),
            "the walk descended into a declared subtree: {inside:?}"
        );
        // Structural scale, timing-free: the scan root + `gen` + 301
        // declared root dirs + the 3-path look-alike chain + `webapp` +
        // its file + `src` + its file = one evaluation per surviving path.
        assert_eq!(
            visits.len(),
            309,
            "every surviving path evaluated exactly once, nothing pruned was read"
        );
    }
}

