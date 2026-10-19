//! kotlin driver — the extraction arm of the kotlin driver design record
//! (`workplan_archspec_kotlin_driver.md`, laws K-P1..K-P8).
//!
//! Files are the vertices and the PACKAGE DECLARATION is identity (K-P3):
//! a `.kt` file's identity is its `package` header, resolution keys on
//! `(package, basename)`, and a file without a package is the NAMED MISS
//! CLASS — never a directory guess. Edges come from import statements ONLY
//! (the K-P6 blind spot, stated verbatim on the five surfaces): a
//! same-package reference needs no import and therefore carries no edge.
//! Units come from the settings file's `include(":x")` literals (K-P2):
//! identity is the gradle path, a `projectDir` remap moves location only.
//! Resolution runs with NO JDK, NO gradle daemon and NO build state (the
//! python no-venv law, mirrored): the tiers are OWN (the package census) →
//! EXTERNAL — `android.*`, `androidx.*` and `uniffi.*` included as PLAIN
//! external facts (operator ruling 2026-10-17), no special arm. The test
//! tier is the C2 law ported (K-P4): the `*Test.kt` convention AND
//! junit-family evidence on the module's test configuration, co-occurrence
//! or nothing. The build exclusion is the DRIVER'S OWN set (K-P7): the
//! module map's `<moduleDir>/build` plus any statically-visible literal
//! relocation — the shared `DETECT_EXCLUDE_DIRS` list stays untouched and
//! a sibling component's authored `build/` tree stays indexed.
//! Same-package edges, wildcard (`*`) import expansion and the java model
//! are PARKED (K-P3/K-P6): `.java` files carry no vertices and no edges —
//! their count ships as the report-only census line.

pub(crate) mod gradle;
pub(crate) mod settings;
pub(crate) mod syntax;

use crate::archspec::language;
use crate::archspec::model::{
    FileEdge, FileFactKind, ManifestInfo, Model, ModuleEdge, ModuleDeclaration, Unit,
};
use crate::archspec::spec;
use crate::archspec::verify::compare::matches_pattern;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

use gradle::Dependency;
use settings::UnitSpec;

/// The model-honesty sentence (K-P6, VERBATIM — the five-surface weld reads
/// these bytes). The live surfaces carry wrapped or registry-side copies;
/// this const is the weld source the unit tests and the claim guard read,
/// so the bin target never consumes it (the `parse/mod.rs` precedent).
#[allow(dead_code)]
pub const MODEL_HONESTY: &str = "Same-package references need no import in Kotlin. The kotlin driver models edges only from import statements. A vertex can depend on same-package code with no edge in the model.";

pub fn extract(root: &Path) -> Result<Model, String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let loaded = spec::load_scan(&root).ok().flatten();
    let declared = loaded.as_ref().map(spec::exclusion_paths);

    // K-P2: the settings file IS the module list; the dual-settings class
    // is the fail-closed module-list parse (detection has already answered
    // kotlin — one owner decides: keep one file).
    let plan = settings::plan(&root)?;
    let mut report_only: Vec<String> = plan.lines.clone();
    if plan.no_settings {
        report_only.push(
            "no settings file: gradle module list unavailable — one unit at the scan root"
                .to_string(),
        );
    }

    // K-P7: the driver's OWN exclusion set — `<moduleDir>/build` for every
    // module map entry (root included), replaced by a statically-visible
    // literal relocation. Property-driven relocation is NOT here: that is
    // the `excluded_sources` config duty (the shipped escape hatch).
    let own_exclusions = own_exclusion_set(&plan, &root);

    let (files, java_count) = collect_files(&root, declared.as_ref(), &own_exclusions)?;
    if files.is_empty() {
        return Err(format!("no kotlin sources found under: {}", root.display()));
    }
    if java_count > 0 {
        // K-P6: a SHAPE census (count only) — no java driver is implied and
        // no detection changes; `.java` carries real code, so its absence
        // from the model is COUNTED, never buried in prose.
        report_only.push(format!(
            "java sources not indexed: {java_count} files — the kotlin driver indexes .kt only"
        ));
    }

    // The PARALLEL FACT LEG (the python P-PAR shape): ONLY the pure
    // per-file leg runs across workers; the fold replays in walk order and
    // the `?` on the FIRST Err reproduces the serial early abort.
    let threads = super::leg::fact_leg_threads(files.len());
    let all_facts: Vec<Result<syntax::FileFacts, String>> = if threads > 1 {
        super::leg::parallel_fact_leg(files.len(), threads, |index| {
            let file = &files[index];
            syntax::scan_kotlin_file(&root.join(file)).map_err(|err| format!("{file}: {err}"))
        })
    } else {
        files
            .iter()
            .map(|file| {
                syntax::scan_kotlin_file(&root.join(file)).map_err(|err| format!("{file}: {err}")
            )
            })
            .collect()
    };
    let mut facts_of: BTreeMap<String, syntax::FileFacts> = BTreeMap::new();
    let mut import_statements = 0usize;
    for (file, facts) in files.iter().zip(all_facts) {
        let facts = facts?;
        import_statements += facts.import_statements;
        facts_of.insert(file.clone(), facts);
    }

    // Units (K-P2): the settings-file modules plus an implicit ROOT unit
    // for sources that live outside every module directory. Identity is
    // the gradle path; a remap moved only the location.
    let mut units: Vec<UnitSpec> = plan.units.clone();
    let root_holds_files = files
        .iter()
        .any(|file| units_for(&units).iter().all(|dir| !under_dir(file, dir)));
    if root_holds_files {
        units.insert(
            0,
            UnitSpec {
                name: ".".to_string(),
                dir: ".".to_string(),
            },
        );
    }
    if plan.no_settings {
        // One root unit, stated once, whatever the file layout.
        units = vec![UnitSpec {
            name: ".".to_string(),
            dir: ".".to_string(),
        }];
    }
    let model_units: Vec<Unit> = units
        .iter()
        .map(|unit| Unit {
            name: unit.name.clone(),
            kind: "module".to_string(),
            path: unit.dir.clone(),
            root: String::new(),
            crate_ids: BTreeSet::from([unit.name.clone()]),
        })
        .collect();

    // Build-script and catalog evidence per unit (K-P4 evidence half).
    let catalog = match gradle::catalog(&root) {
        Ok(catalog) => catalog,
        Err(line) => {
            report_only.push(line);
            None
        }
    };
    let mut unit_dependencies: BTreeMap<String, Vec<Dependency>> = BTreeMap::new();
    // Whether ANY build script was found at all: the vacuity carrier rides
    // this missing-manifest fact (the ts/python C5 law verbatim — a manifest
    // stating an empty dependency set states a fact).
    let mut manifest_found = false;
    for unit in units.iter().filter(|unit| unit.name != ".") {
        let dir = root.join(&unit.dir);
        for name in ["build.gradle.kts", "build.gradle"] {
            let path = dir.join(name);
            if !path.is_file() {
                continue;
            }
            manifest_found = true;
            let facts = gradle::scan_build_file(&path)?;
            unit_dependencies
                .entry(unit.name.clone())
                .or_default()
                .extend(facts.dependencies);
        }
    }
    // Root-level build script evidence rides the root unit when one exists.
    if units.iter().any(|unit| unit.name == ".") {
        for name in ["build.gradle.kts", "build.gradle"] {
            let path = root.join(name);
            if path.is_file() {
                manifest_found = true;
                let facts = gradle::scan_build_file(&path)?;
                unit_dependencies
                    .entry(".".to_string())
                    .or_default()
                    .extend(facts.dependencies);
            }
        }
    } else {
        // Root-script fact leg without a root unit: the coordinate evidence
        // still reaches the EXTERNAL tier — a fact is never silently lost —
        // but a Project row states an edge and no source unit owns it, so
        // project rows are skipped. No unit_manifests entry is invented for
        // a unit the module list does not declare.
        for name in ["build.gradle.kts", "build.gradle"] {
            let path = root.join(name);
            if path.is_file() {
                manifest_found = true;
                let facts = gradle::scan_build_file(&path)?;
                let entry = unit_dependencies.entry(".".to_string()).or_default();
                for dependency in facts.dependencies {
                    if !matches!(dependency, Dependency::Project { .. }) {
                        entry.push(dependency);
                    }
                }
            }
        }
    }
    for dependencies in unit_dependencies.values_mut() {
        for dependency in dependencies.iter_mut() {
            if let Dependency::Catalog {
                alias,
                group,
                name,
                ..
            } = dependency
                && let Some(catalog) = catalog.as_ref()
                && let Some((resolved_group, resolved_name)) = catalog.resolve(alias)
            {
                *group = Some(resolved_group.to_string());
                *name = Some(resolved_name.to_string());
            }
        }
        dependencies.sort();
        dependencies.dedup();
    }

    // K-P3: the package census is the OWN tier's whole ground.
    let mut census: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
    for file in &files {
        if let Some(package) = facts_of[file].package.as_deref() {
            census.entry(package).or_default().push(file);
        }
    }

    let mut file_rows: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut external: BTreeSet<String> = BTreeSet::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut import_edges: BTreeSet<(String, String, String, String)> = BTreeSet::new();
    for file in &files {
        let facts = &facts_of[file];
        for import in &facts.imports {
            if import.wildcard {
                // PARKED (K-P3): star expansion needs the dir↔package law
                // Kotlin does not carry. No edge, no external fact, no
                // finding — the park is stated in the record.
                continue;
            }
            let (package, basename) = match import.path.rsplit_once('.') {
                Some((package, basename)) => (package, basename),
                // A one-segment import names the DEFAULT package — no
                // Kotlin source can import it; it is not first party.
                None => {
                    insert_external(&mut external, &mut module_external, file, &import.path);
                    continue;
                }
            };
            let candidates: Vec<&String> = match census.get(package) {
                Some(files) => files.iter().copied(),
                None => {
                    insert_external(&mut external, &mut module_external, file, &import.path);
                    continue;
                }
            }
            .collect();
            // Resolution key (K-P3): (package, basename). A unique hit is
            // ONE file edge; otherwise file-set edges over the package
            // census — every file declaring that package carries the edge
            // (honest weight, no pick).
            let basename_hits: Vec<&String> = candidates
                .iter()
                .filter(|candidate| basename_of(candidate) == basename)
                .copied()
                .collect();
            let targets: Vec<&String> = if basename_hits.len() == 1 {
                basename_hits
            } else {
                candidates.clone()
            };
            for target in targets {
                if target == file {
                    // Same-file rows state nothing (the R2-1 lineage).
                    continue;
                }
                file_rows
                    .entry((file.to_string(), (*target).to_string()))
                    .or_default()
                    .insert(basename.to_string());
                import_edges.insert((
                    unit_of(file, &units).to_string(),
                    file.to_string(),
                    unit_of(target, &units).to_string(),
                    (*target).to_string(),
                ));
            }
        }
    }
    for dependencies in unit_dependencies.values() {
        for dependency in dependencies {
            if let Some(name) = dependency.external_name() {
                external.insert(name);
            }
        }
    }
    // Manifest externals ride `module_external` keyed by the OWNING UNIT —
    // the unit-rooted key shape the engine's external guards monitor (go
    // parity, compare/external.rs `check_external_free`): a gradle dependency is a
    // unit-level fact, attributed where the module tier lives.
    for (unit, dependencies) in &unit_dependencies {
        for name in gradle::external_names(dependencies) {
            module_external.entry(unit.clone()).or_default().insert(name);
        }
    }

    // The tripwire law (the ts/python precedent): scanned files with parsed
    // imports whose model states no edge, no external fact and no
    // report-only outcome mean extraction saw nothing.
    if let Some(tripwire) = tripwire_error(
        files.len(),
        import_statements,
        !file_rows.is_empty(),
        !external.is_empty(),
        !report_only.is_empty(),
    ) {
        return Err(tripwire);
    }

    // Vacuity carrier (the ts/python C5 mirror verbatim): the carrier rides
    // a MISSING manifest, never an empty deps list — a root whose module list
    // declares no build script anywhere must not scan to a silent green; a
    // script stating an empty dependency set states a fact.
    if !manifest_found {
        report_only.push(
            "vacuous constraint: scan.unit_manifests: no package manifest states dependencies"
                .to_string(),
        );
    }

    // K-P4: the test tier co-occurs — the `*Test.kt` convention AND
    // junit-family evidence on the owning unit. `src/test/` is
    // corroboration only; the directory never carries the tier alone.
    let junit_units: BTreeSet<&str> = unit_dependencies
        .iter()
        .filter(|(_, dependencies)| dependencies.iter().any(Dependency::junit_evidence))
        .map(|(unit, _)| unit.as_str())
        .collect();
    let test_gated_modules: BTreeSet<String> = files
        .iter()
        .filter(|file| test_tier_convention(file))
        .filter(|file| junit_units.contains(unit_of(file, &units)))
        .cloned()
        .collect();

    // K-P3: split packages are LEGAL Kotlin — one package declared by
    // several modules rides the report-only census line.
    let mut package_units: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for file in &files {
        if let Some(package) = facts_of[file].package.as_deref() {
            package_units
                .entry(package)
                .or_default()
                .insert(unit_of(file, &units));
        }
    }
    for (package, owners) in &package_units {
        if owners.len() > 1 {
            report_only.push(format!(
                "split package: {package} — declared by {} modules: {}",
                owners.len(),
                owners
                    .iter()
                    .map(|owner| owner.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    // K-P5 (amended): the scan-time boundary advisory. Zone = the spec's
    // declared module grouping; the driver knows its own module map, the
    // spec grouping and the exclusions at scan time — exactly the context
    // the verify layer lacks. Report-only, exit 0, never a finding.
    if let Some(spec) = loaded.as_ref() {
        advisory_lines(&spec.modules, &files, &facts_of, &units, &mut report_only);
    }

    // Edges (rust shape, K-P2): hard unit edges from the settings-declared
    // project dependencies; module edges from the import crossings the
    // census resolved; file edges carry the file-grained facts.
    let mut hard_edges: BTreeSet<(String, String)> = BTreeSet::new();
    for (unit, dependencies) in &unit_dependencies {
        for dependency in dependencies {
            if let Dependency::Project { path } = dependency {
                hard_edges.insert((unit.clone(), path.clone()));
            }
        }
    }
    let edges: Vec<crate::archspec::model::Edge> = hard_edges
        .iter()
        .filter(|(from, to)| from != to)
        .map(|(from, to)| crate::archspec::model::Edge {
            from: from.clone(),
            to: to.clone(),
        })
        .collect();

    let mut module_counts: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for (from_unit, _from, to_unit, to) in &import_edges {
        if from_unit == to_unit {
            continue;
        }
        module_counts
            .entry((from_unit.clone(), to_unit.clone()))
            .or_default()
            .insert(basename_of(to).to_string());
    }
    // K-P2 (module-tier dotted facts are MANDATORY): a gradle-path project
    // dependency is itself a module-tier fact, statically visible without a
    // single import — a multi-module tree wired by `project(":x")` must
    // reach `depgraph` with the tier present. A dependency naming a target
    // outside the settings module list states no edge (the unresolved-include
    // line already owns that gap).
    let unit_names: BTreeSet<&str> = units.iter().map(|unit| unit.name.as_str()).collect();
    for (from_unit, dependencies) in &unit_dependencies {
        for dependency in dependencies {
            if let Dependency::Project { path } = dependency
                && path != from_unit
                && unit_names.contains(path.as_str())
            {
                module_counts
                    .entry((from_unit.clone(), path.clone()))
                    .or_default();
            }
        }
    }
    let module_edges: Vec<ModuleEdge> = module_counts
        .iter()
        .map(|((from, to), symbols)| ModuleEdge {
            unit: from.clone(),
            from: from.clone(),
            to: to.clone(),
            symbols: symbols.iter().cloned().collect(),
            type_only: false,
        })
        .collect();
    let file_edges: Vec<FileEdge> = file_rows
        .iter()
        .map(|((from, to), _)| FileEdge {
            unit: unit_of(from, &units).to_string(),
            from: from.clone(),
            to: to.clone(),
            kind: FileFactKind::Use,
        })
        .collect();

    // The kotlin driver states NO root module declarations (capability row
    // `root-module-declarations = not-emitted`, go parity): its module tier
    // rides `module_edges` (gradle-path rows), so the declarations map stays
    // empty and engagement reads the unit and module-edge tier only.
    let declarations: BTreeMap<String, Vec<ModuleDeclaration>> = BTreeMap::new();

    let mut unit_manifests: BTreeMap<String, ManifestInfo> = BTreeMap::new();
    for unit in &units {
        let dependencies = unit_dependencies
            .get(&unit.name)
            .map(|dependencies| gradle::external_names(dependencies).into_iter().collect())
            .unwrap_or_default();
        unit_manifests.insert(
            unit.name.clone(),
            ManifestInfo {
                publish: None,
                dependencies,
                features: Vec::new(),
            },
        );
    }

    report_only.sort();
    report_only.dedup();

    Ok(Model {
        units: model_units,
        edges,
        external: external.into_iter().collect(),
        module_edges,
        module_external: module_external
            .into_iter()
            .map(|(file, names)| (file, names.into_iter().collect()))
            .collect(),
        root_module_declarations: declarations,
        unit_manifests,
        test_gated_modules,
        file_edges,
        report_only_lines: report_only,
        ..Model::new("kotlin")
    })
}

/// The tripwire law (the ts/python precedent): files with parsed imports
/// whose model states no edge and no external fact means extraction saw
/// nothing — a parser that silently states nothing cannot pass green.
fn tripwire_error(
    files: usize,
    import_statements: usize,
    has_edges: bool,
    has_external: bool,
    has_report_only: bool,
) -> Option<String> {
    if files > 0 && import_statements > 0 && !has_edges && !has_external && !has_report_only {
        return Some(format!(
            "kotlin driver tripwire: {files} scanned files carry {import_statements} import statements, yet the model states no edge or external fact"
        ));
    }
    None
}

/// The K-P4 convention half: the `*Test.kt` file-name convention. The
/// convention alone carries no tier — it is ANDed with the junit-family
/// evidence (the C2 law ported). `*Tests.kt` / `*Spec.kt` are NAMED-PARKED.
fn test_tier_convention(file: &str) -> bool {
    file.rsplit('/').next().unwrap_or(file).ends_with("Test.kt")
}

fn basename_of(file: &str) -> &str {
    let name = file.rsplit('/').next().unwrap_or(file);
    name.strip_suffix(".kt").unwrap_or(name)
}

/// The unit owning a file: the longest module-directory match.
fn unit_of<'u>(file: &str, units: &'u [UnitSpec]) -> &'u str {
    let mut best: Option<&'u UnitSpec> = None;
    for unit in units {
        if unit.name == "." {
            continue;
        }
        if under_dir(file, &unit.dir)
            && best.is_none_or(|current| unit.dir.len() > current.dir.len())
        {
            best = Some(unit);
        }
    }
    best.map(|unit| unit.name.as_str()).unwrap_or(".")
}

fn under_dir(file: &str, dir: &str) -> bool {
    if dir == "." || dir.is_empty() {
        return !file.contains('/');
    }
    file.starts_with(&format!("{dir}/"))
}

fn units_for(units: &[UnitSpec]) -> Vec<String> {
    units
        .iter()
        .filter(|unit| unit.name != ".")
        .map(|unit| unit.dir.clone())
        .collect()
}

fn insert_external(
    external: &mut BTreeSet<String>,
    module_external: &mut BTreeMap<String, BTreeSet<String>>,
    file: &str,
    path: &str,
) {
    // The top-segment law (the python tier parity): `android.content.Intent`
    // is the EXTERNAL fact `android` — attribution detail never a gate.
    let top = path.split('.').next().unwrap_or(path);
    external.insert(top.to_string());
    module_external
        .entry(file.to_string())
        .or_default()
        .insert(top.to_string());
}

/// The driver's OWN exclusion set (K-P7): `<moduleDir>/build` for every
/// module map entry, replaced by a statically-visible literal relocation.
/// The shared mechanical list is NOT touched here, and a directory named
/// `build` that is not a module map entry's build dir stays indexed.
fn own_exclusion_set(plan: &settings::ModulePlan, root: &Path) -> Vec<PathBuf> {
    let mut exclusions = Vec::new();
    let mut modules: Vec<&UnitSpec> = plan.units.iter().collect();
    let root_unit = UnitSpec {
        name: ".".to_string(),
        dir: ".".to_string(),
    };
    modules.push(&root_unit);
    for module in modules {
        // A relocation stated in the module's OWN build script wins over a
        // settings-file (subprojects-wide) relocation; either replaces the
        // literal `build` name (K-P7 — the stated path is excluded, the
        // unsaid one is not).
        let relocation = module_build_dir_override(root, &module.dir)
            .or_else(|| plan.settings_build_dir.clone().filter(|r| !r.is_empty()));
        let dir = match relocation.as_deref() {
            Some(relocation) => join_rel(&module.dir, relocation),
            None => join_rel(&module.dir, "build"),
        };
        if root.join(&dir).is_dir() {
            exclusions.push(PathBuf::from(dir));
        }
    }
    exclusions
}

/// The literal `layout.buildDirectory.set(file("…"))` stated by a module's
/// own build script (best effort — the same file is read for its dependency
/// facts later; an unreadable script relocates nothing).
fn module_build_dir_override(root: &Path, dir: &str) -> Option<String> {
    if dir == "." || dir.is_empty() {
        return None;
    }
    for name in ["build.gradle.kts", "build.gradle"] {
        if let Ok(facts) = gradle::scan_build_file(&root.join(dir).join(name))
            && let Some(relocation) = facts.build_dir_override
            && !relocation.is_empty()
        {
            return Some(relocation);
        }
    }
    None
}

fn join_rel(dir: &str, tail: &str) -> String {
    if dir == "." || dir.is_empty() {
        tail.to_string()
    } else {
        format!("{dir}/{tail}")
    }
}

/// The K-P5 scan-time advisory: one line per (zone, package) whose member
/// files sit in more than one spec zone. Zones are the spec's declared
/// module groupings; membership reads the shipped pattern matcher. A
/// package is SPLIT when its members' zone claims diverge — members the
/// same zones claim (however nested the zones are) state one grouping,
/// not a split.
fn advisory_lines(
    modules: &[spec::Module],
    files: &[String],
    facts_of: &BTreeMap<String, syntax::FileFacts>,
    units: &[UnitSpec],
    report_only: &mut Vec<String>,
) {
    if modules.is_empty() {
        return;
    }
    let mut package_zones: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    let mut package_claims: BTreeMap<&str, BTreeSet<BTreeSet<&str>>> = BTreeMap::new();
    for file in files {
        let Some(package) = facts_of[file].package.as_deref() else {
            continue;
        };
        let unit = unit_of(file, units);
        let mut claims: BTreeSet<&str> = BTreeSet::new();
        for zone in modules {
            if zone_claims(zone, file, unit) {
                *package_zones
                    .entry(package)
                    .or_default()
                    .entry(zone.name.as_str())
                    .or_default() += 1;
                claims.insert(zone.name.as_str());
            }
        }
        package_claims.entry(package).or_default().insert(claims);
    }
    for (package, counts) in package_zones {
        // Divergent member claims are the split; a single claim set — even
        // the empty one no zone covers — is not the advisory's to state.
        if package_claims.get(package).is_none_or(|sets| sets.len() < 2) {
            continue;
        }
        let total: usize = counts.values().sum();
        // The zone holding the most members owns the package; the line
        // counts the members it does not hold.
        let (home, held) = counts
            .iter()
            .max_by_key(|(name, held)| (*held, *name))
            .map(|(name, held)| (*name, *held))
            .unwrap_or(("", 0));
        report_only.push(format!(
            "boundary advisory: zone {home} splits the package {package} — {} member files outside",
            total - held
        ));
    }
}

/// Zone membership: the spec's match set over the unit name, the file path
/// and the module identity — the shipped pattern semantics.
fn zone_claims(zone: &spec::Module, file: &str, unit: &str) -> bool {
    zone.matches.units.iter().any(|pattern| {
        matches_pattern(unit, pattern) || matches_pattern(unit, &format!(":{pattern}"))
    }) || zone
        .matches
        .paths
        .iter()
        .any(|pattern| matches_pattern(file, pattern))
        || zone.matches.modules.iter().any(|pattern| {
            matches_pattern(file, pattern) || matches_pattern(unit, pattern)
        })
}

/// The scanned `.kt` files (the source predicate is `.kt` ONLY, K-P1:
/// `.kts` scripts are fact-legs, never vertices), repo-relative, git
/// spelling, sorted — plus the `.java` shape census (K-P6, count only).
fn collect_files(
    root: &Path,
    declared: Option<&Vec<PathBuf>>,
    own: &[PathBuf],
) -> Result<(Vec<String>, usize), String> {
    let mut files = Vec::new();
    let mut java = 0usize;
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !excluded_walk(entry.path(), root, declared, own))
    {
        let entry = entry.map_err(|err| format!("cannot scan tree: {err}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if language::is_kotlin_source(&name) {
            if let Some(rel) = entry.path().strip_prefix(root).ok().map(spell) {
                files.push(rel);
            }
        } else if name.ends_with(".java") {
            java += 1;
        }
    }
    files.sort();
    Ok((files, java))
}

fn excluded_walk(
    path: &Path,
    root: &Path,
    declared: Option<&Vec<PathBuf>>,
    own: &[PathBuf],
) -> bool {
    if path
        .components()
        .any(|component| matches!(component, Component::Normal(name)
            if language::is_excluded_dir(&name.to_string_lossy())))
    {
        return true;
    }
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    if rel == Path::new("") {
        return false;
    }
    declared.is_some_and(|exclusions| {
        exclusions
            .iter()
            .any(|entry| rel.starts_with(entry))
    }) || own.iter().any(|entry| rel.starts_with(entry))
}

fn spell(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{basename_of, test_tier_convention, tripwire_error, unit_of, MODEL_HONESTY};
    use crate::archspec::scan::kotlin::settings::UnitSpec;

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
    fn the_test_convention_is_the_test_kt_suffix() {
        assert!(test_tier_convention("app/src/test/java/AppTest.kt"));
        assert!(!test_tier_convention("app/src/test/java/AppTests.kt"));
        assert!(!test_tier_convention("app/src/test/java/Spec.kt"));
        assert!(!test_tier_convention("app/src/main/java/App.kt"));
    }

    #[test]
    fn the_blind_spot_sentence_is_the_welded_bytes() {
        assert_eq!(
            MODEL_HONESTY,
            "Same-package references need no import in Kotlin. The kotlin driver models edges only from import statements. A vertex can depend on same-package code with no edge in the model."
        );
    }

    #[test]
    fn resolution_strips_the_source_suffix_from_the_basename() {
        assert_eq!(basename_of("app/src/main/Thing.kt"), "Thing");
    }

    #[test]
    fn the_longest_module_directory_owns_a_file() {
        let units = vec![
            UnitSpec {
                name: "app".to_string(),
                dir: "app".to_string(),
            },
            UnitSpec {
                name: "app::ui".to_string(),
                dir: "app/ui".to_string(),
            },
        ];
        assert_eq!(unit_of("app/ui/src/Main.kt", &units), "app::ui");
        assert_eq!(unit_of("app/src/Main.kt", &units), "app");
        assert_eq!(unit_of("elsewhere/Top.kt", &units), ".");
    }
}
