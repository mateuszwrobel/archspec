use crate::archspec::model::{Edge, ManifestInfo, Model, ModuleEdge, Unit};

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use walkdir::WalkDir;

type ProjectSources = (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>);
type ScannedFiles = BTreeMap<String, ProjectSources>;

/// C# driver (csharp): units = projects, hard edges = project references,
/// soft tier = namespaces (converted to `::` module paths so the shared
/// `::`-hardcoded verify engine resolves them), external = NuGet packages,
/// `module_external` = referenced packages IN USE per module (the same package
/// vocabulary the rust/go drivers put in the tier), manifests = per-project
/// csproj facts. A referenced package counts as used through a `using N;`
/// when its id namespace-matches `N` via the longest-prefix rule in
/// [`used_packages_for_using`]; namespaces are not external-tier entries.
/// A `using` of a namespace owned by ANOTHER unit records a cross-unit module
/// edge (usage fact in code, alongside the csproj-derived unit edge — build
/// fact) when the owner resolves unambiguously by longest prefix and is
/// reference-reachable (directly or transitively) — anything else cannot
/// compile and is no source fact: unresolvable targets keep flowing to the
/// external attribution rule, ambiguous or unreachable ones emit nothing.
/// A project whose resolved package set names a test runner (see
/// [`is_test_project`]) is C#'s test tier: whole projects
/// are the test unit here, so like go's `*_test.go` files they are excluded at
/// the scan from every production tier — no serialized marker, by the go
/// precedent of excluding at the scan when the fact cannot be module-path data.
pub fn extract(root: &Path) -> Result<Model, String> {
    // Absolute base so Include references (which may be absolute or use `..`
    // back to the tree) resolve identically regardless of how the root was
    // given (relative, `.`, or absolute).
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let projects = collect_projects(&root);
    if projects.is_empty() {
        return extract_single_unit(&root);
    }

    // Test-tier classification precedes every production fact; the predicate
    // is [`is_test_project`]. Like the go driver a test project is excluded at
    // the scan with no serialized fact — no unit, no edge, no soft tier, no
    // external attribution, no manifest entry — so a tree with test projects
    // scans to the same model as the same tree without them, and
    // `verify`/`update` consume the shape unchanged (nothing left to gate).
    // Dropped projects keep their declared namespaces as DEAD prefixes: a
    // production `using App.Tests.Specs;` names the test tier, not a module of
    // the referencing unit, and prefix-owner resolution must not resurrect the
    // dropped project as a phantom edge, module, or seeded boundary.
    let mut production: Vec<(String, std::path::PathBuf, ManifestInfo)> = Vec::new();
    let mut dropped_namespaces: BTreeSet<String> = BTreeSet::new();
    for (rel_path, full_path) in &projects {
        let project_dir = full_path.parent().unwrap_or(&root);
        let manifest = manifest_info(project_dir, &root, full_path);
        if is_test_project(&manifest) {
            dropped_namespaces.extend(scan_project_sources(project_dir)?.0);
            continue;
        }
        production.push((rel_path.clone(), full_path.clone(), manifest));
    }

    let mut units = Vec::new();
    let mut unit_manifests: BTreeMap<String, ManifestInfo> = BTreeMap::new();
    for (rel_path, _full_path, manifest) in &production {
        let name = project_name(rel_path);
        let path = match Path::new(rel_path).parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().into_owned(),
            _ => ".".to_string(),
        };
        units.push(Unit {
            name: name.clone(),
            kind: "project".to_string(),
            path,
            root: String::new(),
            crate_ids: BTreeSet::new(),
        });
        unit_manifests.insert(name, manifest.clone());
    }
    units.sort_by(|left, right| left.name.cmp(&right.name));

    // Map normalized full csproj path -> unit name. An Include resolves to the
    // full path of the referenced csproj; matching on the path (not the name)
    // keeps distinct projects with identical stems from collapsing. A reference
    // resolving to a test project names no production unit and emits no edge.
    let mut unit_by_csproj: BTreeMap<std::path::PathBuf, String> = BTreeMap::new();
    for (rel_path, _full_path, _manifest) in &production {
        unit_by_csproj.insert(normalize(&root.join(rel_path)), project_name(rel_path));
    }
    let from_csproj = |rel_path: &str| normalize(&root.join(rel_path));

    let mut edge_set: BTreeSet<(String, String)> = BTreeSet::new();
    for (rel_path, full_path, _manifest) in &production {
        let from = project_name(rel_path);
        let from_key = from_csproj(rel_path);
        let project_dir = full_path.parent().unwrap_or(&root);
        for reference in project_references(project_dir, full_path)? {
            // csproj Include values conventionally use backslash separators;
            // normalize to `/` so path joining works on any platform.
            let normalized_ref = reference.replace('\\', "/");
            let reference_path = Path::new(&normalized_ref);
            let resolved = normalize(&project_dir.join(reference_path));
            if let Some(name) = unit_by_csproj.get(&resolved) {
                if from_key != resolved {
                    edge_set.insert((from.clone(), name.clone()));
                }
            }
        }
    }
    let edges: Vec<Edge> = edge_set
        .into_iter()
        .map(|(from, to)| Edge { from, to })
        .collect();
    // Transitive reference reachability (see [`reference_reachability`]): the
    // condition under which a cross-unit using can compile at all — code in a
    // project sees the namespaces of every project reachable through project
    // references, not just the ones referenced directly.
    let reference_reach = reference_reachability(&edges);

    // ---- Soft tier, external deps, and per-unit manifests --------------------
    // Namespace -> owning unit. Every namespace declared anywhere in the tree.
    let mut namespace_unit: BTreeMap<String, String> = BTreeMap::new();
    // Namespaces declared by MORE THAN ONE unit: prefix ownership is
    // ambiguous, so a cross-unit `using` of one resolves to no honest owner.
    let mut ambiguous_namespaces: BTreeSet<String> = BTreeSet::new();
    // unit name -> sorted declared namespaces (converted to `::` module paths).
    let mut soft_structure: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // unit name -> (namespace declared in that unit -> referenced packages the
    // namespace's usings make visible). Values are package ids — the same
    // vocabulary forbid rules are written in on every driver.
    let mut module_external: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    // unit name -> module edges (from, to) -> used symbols (empty for C# usings).
    let mut module_edge_map: BTreeMap<String, BTreeMap<(String, String), BTreeSet<String>>> =
        BTreeMap::new();

    // Scan each project's sources once: namespaces + per-namespace usings.
    // `unit_manifests` already carries the resolved csproj facts (test-tier
    // projects were dropped with the classification pass above), so the loop
    // never reparses a manifest.
    let mut scanned: ScannedFiles = BTreeMap::new();
    for unit in &units {
        let project_dir = if unit.path == "." {
            root.to_path_buf()
        } else {
            root.join(&unit.path)
        };
        let (namespaces, usings) = scan_project_sources(&project_dir)?;
        scanned.insert(unit.name.clone(), (namespaces.clone(), usings.clone()));
        let mut sorted: Vec<String> = namespaces.iter().map(|ns| ns_to_module(ns)).collect();
        // Sentinel-presence is the truth condition: the "" key exists exactly
        // when some namespace-less file contributed at least one using (the
        // entry is created with its first target), so the root module joins the
        // soft structure iff it carries a fact — a using-free namespace-less
        // file adds no module, like any other empty one.
        let root_key = root_module_key(&unit.name);
        if usings.contains_key("") && !sorted.contains(&root_key) {
            sorted.push(root_key.clone());
        }
        sorted.sort();
        if !sorted.is_empty() {
            soft_structure.insert(unit.name.clone(), sorted);
        }
        // Record namespace ownership for cross-project resolution. A namespace
        // second-declared by a different unit joins the ambiguity set.
        for ns in &namespaces {
            if let Some(previous) = namespace_unit.get(ns) {
                if previous != &unit.name {
                    ambiguous_namespaces.insert(ns.clone());
                }
            }
            namespace_unit.insert(ns.clone(), unit.name.clone());
        }
        module_external.insert(unit.name.clone(), BTreeMap::new());
        module_edge_map.insert(unit.name.clone(), BTreeMap::new());
    }

    // Cross-project namespace ownership is only fully known after every unit is
    // scanned, so resolve usings in a second pass.
    for unit in &units {
        let (_namespaces, usings) = scanned
            .get(&unit.name)
            .cloned()
            .unwrap_or_default();
        let referenced: Vec<String> = unit_manifests
            .get(&unit.name)
            .map(|manifest| manifest.dependencies.clone())
            .unwrap_or_default();
        // The unit-root sentinel ("" = namespace-less file) translates to the
        // project's root module key before it can reach any model tier.
        let root_key = root_module_key(&unit.name);
        let from_module = |from_ns: &str| {
            if from_ns.is_empty() {
                root_key.clone()
            } else {
                ns_to_module(from_ns)
            }
        };
        for (from_ns, targets) in &usings {
            for target in targets {
                // A target under a dropped test project's namespace is a
                // reference into the test tier — not a production fact: no
                // owner, no module edge, no external attribution.
                if falls_under_namespace(target, &dropped_namespaces) {
                    continue;
                }
                // Resolve the deepest declared namespace prefix of the target.
                match resolve_owner_prefix(target, &namespace_unit) {
                    Some((_, owner)) if owner == unit.name => {
                        // Within-project: a soft module edge.
                        let from = from_module(from_ns);
                        let to = ns_to_module(target);
                        if from != to {
                            module_edge_map
                                .entry(unit.name.clone())
                                .or_default()
                                .entry((from, to))
                                .or_default();
                        }
                    }
                    Some((prefix, owner)) => {
                        // Cross-unit usage: code truth alongside the csproj
                        // build truth, recorded as a module edge of the USING
                        // unit when the using can actually compile. It cannot
                        // when the matched prefix is declared by several units
                        // (the edge could name any of them — the honest output
                        // is none) or when the owner is reachable through
                        // neither direct nor transitive references (a using
                        // that would not build is no source fact, the same
                        // stance as an uncompilable package using).
                        if !ambiguous_namespaces.contains(prefix)
                            && reference_reach
                                .get(unit.name.as_str())
                                .is_some_and(|reachable| reachable.contains(&owner))
                        {
                            let from = from_module(from_ns);
                            let to = ns_to_module(target);
                            if from != to {
                                module_edge_map
                                    .entry(unit.name.clone())
                                    .or_default()
                                    .entry((from, to))
                                    .or_default();
                            }
                        }
                    }
                    None => {
                        // External namespace: attribute the referenced packages
                        // it makes visible (documented longest-prefix
                        // approximation). A using that maps to no referenced
                        // package is not a dependency fact and drops — no
                        // placeholder, no crash; the namespace itself lives in
                        // the soft tier or nowhere.
                        for package in used_packages_for_using(target, &referenced) {
                            let from = from_module(from_ns);
                            module_external
                                .entry(unit.name.clone())
                                .or_default()
                                .entry(from)
                                .or_default()
                                .insert(package);
                        }
                    }
                }
            }
        }
    }

    // Aggregate module edges into the model shape.
    let mut module_edges: Vec<ModuleEdge> = Vec::new();
    for (unit_name, edges) in &module_edge_map {
        for ((from, to), symbols) in edges {
            let mut symbols: Vec<String> = symbols.iter().cloned().collect();
            symbols.sort();
            module_edges.push(ModuleEdge {
                unit: unit_name.clone(),
                from: from.clone(),
                to: to.clone(),
                symbols,
            });
        }
    }
    module_edges.sort_by(|left, right| {
        (&left.unit, &left.from, &left.to).cmp(&(&right.unit, &right.from, &right.to))
    });

    // External deps: the distinct NuGet packages referenced across the tree.
    let mut external: BTreeSet<String> = BTreeSet::new();
    for manifest in unit_manifests.values() {
        external.extend(manifest.dependencies.iter().cloned());
    }
    let external_sorted: Vec<String> = external.into_iter().collect();

    // module_external in model shape: dotted module path -> sorted package ids.
    // Keys are global while the map is per-unit, and distinct units can own the
    // same key (unit `A.B`'s namespace `A.B` and unit `A.B.C`'s namespace-less
    // composition root both resolve to `A::B`), so the flatten UNIONS the
    // package sets per shared key — a plain insert would be last-write-wins
    // and silently drop one unit's fact.
    let mut merged: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for per_unit in module_external.values() {
        for (module, crates) in per_unit {
            merged
                .entry(module.clone())
                .or_default()
                .extend(crates.iter().cloned());
        }
    }
    let module_external_model: BTreeMap<String, Vec<String>> = merged
        .into_iter()
        .map(|(module, crates)| {
            let mut sorted: Vec<String> = crates.into_iter().collect();
            sorted.sort();
            (module, sorted)
        })
        .collect();

    let manifest = unit_manifests
        .values()
        .next()
        .cloned()
        .filter(|_| unit_manifests.len() == 1);

    Ok(Model {
        schema_version: 1,
        language: "csharp".to_string(),
        units,
        edges,
        usage: Default::default(),
        soft_structure,
        external: external_sorted,
        module_edges,
        manifest,
        root_public_exports: Default::default(),
        root_glob_exports: Default::default(),
        root_empty_glob_exports: Default::default(),
        module_external: module_external_model,
        root_module_declarations: Default::default(),
        unit_manifests,
        unresolved_module_files: Default::default(),
        test_gated_modules: Default::default(),
        facade_roots: Default::default(),
    })
}
fn collect_projects(root: &Path) -> Vec<(String, std::path::PathBuf)> {
    let mut projects = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("csproj") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        // Same exclusion predicate as the `.cs` source walk: excluded
        // directories never contribute units, only sources.
        if is_excluded_cs(rel) {
            continue;
        }
        projects.push((rel.to_string_lossy().into_owned(), path.to_path_buf()));
    }
    projects.sort();
    projects
}

/// The C# test-tier predicate: a project is a test project when its resolved
/// package set (csproj `PackageReference` items plus the nearest central
/// props — the same vocabulary `manifest_info` reports) names a test runner
/// or test SDK: the project-tier mirror of rust's `#[cfg(test)]` modules and
/// go's `*_test.go` files. Runner-package evidence, evaluated per resolved
/// dependency id:
/// - the xunit family (`xunit`, `Xunit.Runner.VisualStudio`, …) or the nunit
///   family (`NUnit`, `NUnit3TestAdapter`, …) — a family PREFIX match, so
///   satellites (`xunit.analyzers`, `NUnit.Analyzers`) count as evidence by
///   design: they appear only in test projects;
/// - exactly `Microsoft.NET.Test.Sdk` — the SDK shape modern MSTest is
///   declared in. MSTest is recognized ONLY through this id; a pre-SDK
///   MSTest project naming `MSTest.TestFramework`/`MSTest.TestAdapter`
///   without the SDK stays production — a documented residual pinned by the
///   framework-list scenario.
///
/// NuGet ids are case-insensitive, so the match lowercases. An assertion
/// library without a runner (`FluentAssertions`) is NOT evidence — production
/// tooling uses it too. Project names play no part: `.Tests` is lore, a
/// runner-free `LegacyTests` stays production and a `Helpers` project
/// referencing xunit is test tier.
pub(crate) fn is_test_project(manifest: &ManifestInfo) -> bool {
    manifest
        .dependencies
        .iter()
        .map(String::as_str)
        .any(is_test_framework_package)
}

/// The id-level runner evidence behind [`is_test_project`]: the xunit and
/// nunit families by case-insensitive prefix, and the Microsoft test host
/// SDK by exact id. The full evidence rules — satellites, the MSTest SDK
/// shape, assertion libraries, and the irrelevance of project names — are
/// stated on [`is_test_project`].
fn is_test_framework_package(dependency: &str) -> bool {
    let id = dependency.to_ascii_lowercase();
    id.starts_with("xunit") || id.starts_with("nunit") || id == "microsoft.net.test.sdk"
}

/// Unit name for a csproj: the file stem (e.g. `Orders.csproj` -> `Orders`).
fn project_name(rel_path: &str) -> String {
    Path::new(rel_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string()
}

/// Parse a csproj for `<ProjectReference Include="...">` entries, returning the
/// include paths (relative to the referencing project's directory).
fn project_references(_project_dir: &Path, full_path: &Path) -> Result<Vec<String>, String> {
    let raw = std::fs::read_to_string(full_path)
        .map_err(|err| format!("failed to read {}: {err}", full_path.display()))?;
    let mut reader = quick_xml::Reader::from_str(&raw);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_project_reference = false;
    let mut include = None;
    let mut references = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                if e.name().as_ref() == "ProjectReference" {
                    in_project_reference = true;
                    include = None;
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == "Include" {
                            include = Some(attr.value.into_owned());
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                if e.name().as_ref() == "ProjectReference" {
                    let mut inc = None;
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == "Include" {
                            inc = Some(attr.value.into_owned());
                        }
                    }
                    if let Some(inc) = inc {
                        references.push(inc);
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                if e.name().as_ref() == "ProjectReference" && in_project_reference {
                    if let Some(include) = include.take() {
                        references.push(include);
                    }
                    in_project_reference = false;
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => {
                return Err(format!(
                    "failed to parse project reference: {}",
                    full_path.display()
                ))
            }
            _ => {}
        }
        buf.clear();
    }
    references.sort();
    references.dedup();
    Ok(references)
}

/// Per-unit manifest facts from the project's csproj. `publish` maps
/// `IsPackable=false` to `Some(false)` (a package explicitly not packable);
/// a project that is packable or omits the flag resolves as publishable.
/// `dependencies` are the NuGet package names the project references: the
/// csproj's own `PackageReference` include items plus the centrally declared
/// `PackageReference` items of the nearest props file (central package
/// management — under CPM the versions live in props `PackageVersion` items,
/// so a csproj reference may carry no version; both shapes resolve to the
/// same name tier). `features` has no C# analog and stays empty.
pub(crate) fn manifest_info(project_dir: &Path, root: &Path, full_path: &Path) -> ManifestInfo {
    let raw = std::fs::read_to_string(full_path).unwrap_or_default();
    let mut publish = None;
    let mut dependencies = BTreeSet::new();
    let mut reader = quick_xml::Reader::from_str(&raw);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_is_packable = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                if e.name().as_ref() == "PackageReference" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == "Include" {
                            let value = attr.value.to_string();
                            if !value.is_empty() {
                                dependencies.insert(value);
                            }
                        }
                    }
                } else if e.name().as_ref() == "IsPackable" {
                    in_is_packable = true;
                }
            }
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                if e.name().as_ref() == "PackageReference" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == "Include" {
                            let value = attr.value.to_string();
                            if !value.is_empty() {
                                dependencies.insert(value);
                            }
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                if in_is_packable {
                    let text = e.as_ref().trim().to_lowercase();
                    publish = Some(text == "true");
                    in_is_packable = false;
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                if e.name().as_ref() == "PackageReference" {
                } else if e.name().as_ref() == "IsPackable" {
                    in_is_packable = false;
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    dependencies.extend(central_reference_names(project_dir, root));

    ManifestInfo {
        publish,
        dependencies: dependencies.into_iter().collect(),
        features: Vec::new(),
    }
}

/// The centrally declared `PackageReference` names applying to a project:
/// walking upward from the project directory to the tree root, the nearest
/// directory holding a central props file (`Directory.Packages.props` or
/// `Directory.Build.props`) wins — its `PackageReference` items are the
/// project's central entries and it shadows any farther props file
/// (project-level overrides root, mirroring MSBuild discovery). `PackageVersion`
/// entries are version bookkeeping only — versions do not imply usage — and
/// the model has no version fields, so neither reaches the tier.
fn central_reference_names(project_dir: &Path, root: &Path) -> BTreeSet<String> {
    let mut dir = project_dir.to_path_buf();
    loop {
        let mut found: Option<BTreeSet<String>> = None;
        for file_name in ["Directory.Packages.props", "Directory.Build.props"] {
            let candidate = dir.join(file_name);
            if candidate.is_file() {
                found
                    .get_or_insert_with(BTreeSet::new)
                    .extend(props_package_references(&candidate));
            }
        }
        if let Some(names) = found {
            return names;
        }
        if dir == root {
            return BTreeSet::new();
        }
        match dir.parent() {
            Some(parent) if parent.starts_with(root) => dir = parent.to_path_buf(),
            _ => return BTreeSet::new(),
        }
    }
}

/// The `PackageReference` include names declared in one props file, parsed
/// with the same lightweight XML-event walk as the csproj reader. Only
/// `Include` counts: an `Update` attribute edits an item declared elsewhere
/// and never evidences a reference of this project. A `PackageReference`
/// bearing an MSBuild `Condition` — on the item or its `ItemGroup` — is ignored
/// entirely, as tier evidence and as an external dependency alike: conditions
/// (`Condition="'$(IsTestProject)'=='true'"` is the common test-wiring shape)
/// are unevaluable without MSBuild, and counting them would silently mark
/// every project below the props file test tier and erase the production
/// model. Honesty over false precision: the conditional reference is not
/// modeled at all.
fn props_package_references(path: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return names;
    };
    let mut reader = quick_xml::Reader::from_str(&raw);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut group_conditional = false;
    loop {
        let include = match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e))
                if e.name().as_ref() == "PackageReference" =>
            {
                if group_conditional {
                    None
                } else {
                    reference_include(e)
                }
            }
            Ok(quick_xml::events::Event::Empty(ref e))
                if e.name().as_ref() == "PackageReference" =>
            {
                if group_conditional {
                    None
                } else {
                    reference_include(e)
                }
            }
            Ok(quick_xml::events::Event::Start(ref e)) if e.name().as_ref() == "ItemGroup" => {
                group_conditional = has_condition(e);
                None
            }
            Ok(quick_xml::events::Event::End(ref e)) if e.name().as_ref() == "ItemGroup" => {
                group_conditional = false;
                None
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };
        if let Some(include) = include.filter(|value| !value.is_empty()) {
            names.insert(include);
        }
        buf.clear();
    }
    names
}

fn reference_include(event: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    if has_condition(event) {
        return None;
    }
    event
        .attributes()
        .flatten()
        .find(|attr| attr.key.as_ref() == "Include")
        .map(|attr| attr.value.into_owned())
}

/// True when an element carries an MSBuild `Condition` attribute — its value
/// is unknowable to a static scan, so conditioned items are never modeled.
fn has_condition(event: &quick_xml::events::BytesStart<'_>) -> bool {
    event
        .attributes()
        .flatten()
        .any(|attr| attr.key.as_ref() == "Condition")
}

/// Scan every `.cs` file under a project directory for declared namespaces and
/// `using` targets, both attributed to the namespace they appear in.
///
/// Returns `(namespaces, usings)` where `usings` maps a namespace (dotted, as
/// written) to the set of `using` target namespaces in files of that namespace.
/// A file-scoped `namespace Foo.Bar;` owns all usings in the file; usings in a
/// file with no namespace at all are attributed to the unit root (the empty
/// string is used as a sentinel and translated later).
fn scan_project_sources(project_dir: &Path) -> Result<ProjectSources, String> {
    let mut namespaces: BTreeSet<String> = BTreeSet::new();
    let mut usings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for entry in WalkDir::new(project_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("cs") {
            continue;
        }
        let rel = entry.path().strip_prefix(project_dir).unwrap_or(entry.path());
        let excluded = is_excluded_cs(rel);
        if excluded {
            continue;
        }
        let source = std::fs::read_to_string(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        scan_cs_source(&source, &mut namespaces, &mut usings);
    }
    Ok((namespaces, usings))
}

/// True when any rel-path component is `bin`, `obj`, `node_modules`, or starts
/// with `.`. Matches the inspect exclusion rule so `scan` and `inspect` see the
/// same source tree.
pub(crate) fn is_excluded_cs(rel: &std::path::Path) -> bool {
    rel.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name.starts_with('.') || name == "bin" || name == "obj" || name == "node_modules"
    })
}

/// Tokenize a `.cs` source, skipping comments and strings, and collect:
/// - `namespace` declarations (file-scoped `namespace A.B;` and block
///   `namespace A.B {`) into `namespaces`
/// - `using A.B.C;` (and `using static`, `using Alias = ...`, `global using`)
///   targets into `usings`, keyed by the namespace the using belongs to.
///   A file with no namespace attribute its usings to "" (unit root).
pub(crate) fn scan_cs_source(
    source: &str,
    namespaces: &mut BTreeSet<String>,
    usings: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let bytes = source.as_bytes();
    let mut i = 0usize;
    // Current namespace context: stack for block namespaces; when the file
    // declares a file-scoped namespace, it applies to the whole file.
    let mut ns_stack: Vec<String> = Vec::new();
    // Sentinel: at least one using appeared before any namespace was declared.
    let mut pending_root_usings: Vec<String> = Vec::new();
    // True when the file declared any namespace; a file that never declares one
    // (top-level-statements composition root) attributes its usings to the unit
    // root sentinel instead of dropping them.
    let mut saw_namespace = false;

    while i < bytes.len() {
        let c = bytes[i];
        // Skip comments and string/char literals.
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
        if c == b'"' {
            i = skip_string(bytes, i);
            continue;
        }
        if c == b'@' && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
            i = skip_verbatim_string(bytes, i);
            continue;
        }
        if c == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
            i = skip_interpolated_string(bytes, i);
            continue;
        }
        if c == b'\'' {
            i = skip_char(bytes, i);
            continue;
        }

        if c.is_ascii_alphabetic() {
            if matches_identifier(bytes, i, b"namespace") {
                i += "namespace".len();
                i = skip_ws(bytes, i);
                let (ns, after_ns) = read_dotted(bytes, i);
                i = after_ns;
                i = skip_ws(bytes, i);
                if i < bytes.len() && bytes[i] == b';' {
                    // File-scoped namespace: applies to the whole file.
                    ns_stack.clear();
                    ns_stack.push(ns.clone());
                } else if i < bytes.len() && bytes[i] == b'{' {
                    ns_stack.push(ns.clone());
                }
                if !ns.is_empty() {
                    namespaces.insert(ns);
                    saw_namespace = true;
                }
                continue;
            }
            if matches_identifier(bytes, i, b"using") {
                i += "using".len();
                i = skip_ws(bytes, i);
                // `using var x = ...;` / `using (expr) { }` are resource
                // management statements, not namespace imports.
                if i < bytes.len() && bytes[i] == b'(' {
                    i = skip_to_semicolon(bytes, i);
                    continue;
                }
                if matches_identifier(bytes, i, b"var") {
                    // `using var name = ...;` — skip to the semicolon.
                    i = skip_to_semicolon(bytes, i);
                    continue;
                }
                // Optional `global` prefix (global using).
                if matches_identifier(bytes, i, b"global") {
                    i += "global".len();
                    i = skip_ws(bytes, i);
                }
                // Optional `static` (using static) — target is a type/namespace.
                if matches_identifier(bytes, i, b"static") {
                    i += "static".len();
                    i = skip_ws(bytes, i);
                }
                // Optional alias `using Alias = A.B.C;` — read past `... =`.
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.') {
                    i += 1;
                }
                // If we stopped right before ` = `, the preceding dotted token
                // is an alias, not a namespace.
                i = skip_ws(bytes, i);
                if i < bytes.len() && bytes[i] == b'=' {
                    i += 1;
                    i = skip_ws(bytes, i);
                } else {
                    i = start;
                }
                let (target, _after_target) = read_dotted(bytes, i);
                if !target.is_empty() {
                    if let Some(ns) = ns_stack.last() {
                        usings
                            .entry(ns.clone())
                            .or_default()
                            .insert(target);
                    } else {
                        pending_root_usings.push(target);
                    }
                }
                // Advance past the rest of the statement.
                i = skip_to_semicolon(bytes, i);
                continue;
            }
        }
        i += 1;
    }

    // A file-scoped namespace may be declared before or after usings; usings
    // before the declaration still belong to it. If only one namespace was
    // declared in the whole file, fold pending usings into it. A file that
    // declares no namespace at all (composition root) attributes its usings to
    // the unit-root sentinel (""), translated to the project's root module by
    // the callers.
    if !pending_root_usings.is_empty() {
        let entry = if ns_stack.len() == 1 {
            usings.entry(ns_stack[0].clone()).or_default()
        } else if !saw_namespace {
            usings.entry(String::new()).or_default()
        } else {
            return;
        };
        for target in &pending_root_usings {
            entry.insert(target.clone());
        }
    }
}

/// The dotted name starting at `i` (e.g. `System.Text`). Returns ("", i) when
/// there is no dotted name. The returned index is the position just past the
/// name.
fn read_dotted(bytes: &[u8], i: usize) -> (String, usize) {
    let mut end = i;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'.')
    {
        end += 1;
    }
    if end == i {
        return (String::new(), i);
    }
    // A dotted name may be `@ident` (verbatim identifier); strip the `@`.
    let name = String::from_utf8_lossy(&bytes[i..end]).replace('@', "");
    (name, end)
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

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i].is_ascii_whitespace()) {
        i += 1;
    }
    i
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

fn skip_string(bytes: &[u8], mut i: usize) -> usize {
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

fn skip_verbatim_string(bytes: &[u8], mut i: usize) -> usize {
    i += 2;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

fn skip_interpolated_string(bytes: &[u8], mut i: usize) -> usize {
    // `$"..."` — handle escapes and nested braces roughly; `{}` interpolation
    // can contain strings/comments, but a conservative scan to the closing
    // quote is sufficient for keyword detection.
    i += 2;
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

fn skip_char(bytes: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'\'' {
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

/// Advance to the character after the next `;` (or end of input).
fn skip_to_semicolon(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        if bytes[i] == b';' {
            return i + 1;
        }
        // Nested strings/comments inside the statement (e.g. `using var x = $"..."`).
        if bytes[i] == b'"' {
            i = skip_string(bytes, i);
            continue;
        }
        i += 1;
    }
    bytes.len()
}

/// The dotted prefixes of `target`, longest first: the whole target, then each
/// truncation at a dot, down to the first segment (for `A.B.C`: `A.B.C`,
/// `A.B`, `A`). This is the iteration behind the longest-prefix ownership
/// rule, shared by [`resolve_owner`], [`falls_under_namespace`], and the
/// inspect file graph so the scan and the inspect map can never drift on
/// which declaration owns a `using` target.
pub(crate) fn namespace_prefixes(target: &str) -> impl Iterator<Item = &str> + '_ {
    let mut remaining = target.matches('.').count() + 1;
    let mut end = target.len();
    std::iter::from_fn(move || {
        if remaining == 0 {
            return None;
        }
        remaining -= 1;
        let prefix = &target[..end];
        end = target[..end].rfind('.').unwrap_or(0);
        Some(prefix)
    })
}

/// Resolve the owning unit of a namespace target: the deepest declared
/// namespace that is a prefix (dotted) of the target. Returns None when the
/// target is external.
fn resolve_owner(target: &str, namespace_unit: &BTreeMap<String, String>) -> Option<String> {
    resolve_owner_prefix(target, namespace_unit).map(|(_, owner)| owner.to_string())
}

/// [`resolve_owner`] with the matched declaration prefix, which the cross-unit
/// ambiguity check needs: the deepest declared prefix decides BOTH the owner
/// and whether that owner is honest (a prefix declared by two units is not).
fn resolve_owner_prefix<'a>(
    target: &'a str,
    namespace_unit: &'a BTreeMap<String, String>,
) -> Option<(&'a str, &'a str)> {
    namespace_prefixes(target).find_map(|candidate| {
        namespace_unit
            .get(candidate)
            .map(|owner| (candidate, owner.as_str()))
    })
}

/// Transitive closure of the project-reference graph per source unit: what
/// each project can SEE at compile time. MSBuild flows references down the
/// graph, so a project compiles against the namespaces of every project
/// reachable through references, not just the ones it references directly.
/// Units without outgoing references are absent from the map (they reach
/// nothing but themselves).
fn reference_reachability(edges: &[Edge]) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in edges {
        adjacency.entry(&edge.from).or_default().push(&edge.to);
    }
    let mut reachable: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (start, first_hops) in &adjacency {
        let mut seen = BTreeSet::new();
        let mut queue: Vec<&str> = first_hops.clone();
        while let Some(current) = queue.pop() {
            if seen.insert(current) {
                if let Some(next) = adjacency.get(current) {
                    queue.extend(next.iter().copied());
                }
            }
        }
        reachable.insert(*start, seen);
    }
    reachable
}

/// True when `target` equals or falls under any namespace in `namespaces`
/// (segment-wise dotted prefixes — the same rule [`resolve_owner`] uses).
fn falls_under_namespace(target: &str, namespaces: &BTreeSet<String>) -> bool {
    namespace_prefixes(target).any(|candidate| namespaces.contains(candidate))
}

/// The documented package→namespace approximation. Without a toolchain the
/// packages' public types are unverifiable, so namespace visibility is matched
/// against the referenced package ids themselves: a referenced package counts
/// as used through `using N;` when its id is a dotted prefix of `N`
/// (case-insensitively — ids like `xunit` align with namespaces like `Xunit`)
/// or the id and `N` share a dotted prefix of at least two segments
/// (`Microsoft.IdentityModel.JsonWebTokens` ↔ `Microsoft.IdentityModel.Tokens`),
/// and only among the candidates sharing the LONGEST such prefix (so a package
/// sharing a single vendor segment with the namespace cannot fire). Failure
/// modes of the approximation: a misaligned namespace deeper than the shared
/// two-segment prefix of two packages attributes both (tie), and an aligned
/// id-only coincidence (one-segment package id equal to the namespace root)
/// counts as used. A using matching no referenced package (typo, missing
/// reference — code that would not compile) attributes nothing.
fn used_packages_for_using(namespace: &str, referenced: &[String]) -> Vec<String> {
    let mut matched: Vec<String> = Vec::new();
    let mut longest = 0usize;
    for package in referenced {
        let shared = shared_prefix_segments(namespace, package);
        let candidate = shared >= 2 || is_dotted_prefix(package, namespace);
        if !candidate {
            continue;
        }
        match shared.cmp(&longest) {
            std::cmp::Ordering::Greater => {
                longest = shared;
                matched.clear();
                matched.push(package.clone());
            }
            std::cmp::Ordering::Equal => matched.push(package.clone()),
            std::cmp::Ordering::Less => {}
        }
    }
    matched
}

/// Count of leading dotted segments two dotted names share, compared
/// case-insensitively (package ids are case-insensitive; namespaces are not).
fn shared_prefix_segments(left: &str, right: &str) -> usize {
    left.split('.')
        .zip(right.split('.'))
        .take_while(|(l, r)| l.eq_ignore_ascii_case(r))
        .count()
}

/// True when every segment of `prefix` equals (case-insensitively) the leading
/// segment of the same position in `whole`.
fn is_dotted_prefix(prefix: &str, whole: &str) -> bool {
    let mut whole_parts = whole.split('.');
    prefix
        .split('.')
        .all(|part| whole_parts.next().is_some_and(|other| other.eq_ignore_ascii_case(part)))
}

/// Convert a dotted C# namespace to a `::`-separated module path.
fn ns_to_module(ns: &str) -> String {
    ns.replace('.', "::")
}

/// The module key owning files that declare no namespace (composition roots):
/// the project's root module — the first two segments of the unit's namespace
/// root (the csproj `RootNamespace` convention; the unit name is its default),
/// converted like any namespace. Mirrors the rust driver placing crate-root
/// files on the crate's own module.
fn root_module_key(unit_name: &str) -> String {
    ns_to_module(
        &unit_name
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join("."),
    )
}

/// Canonicalize a path lexically: resolve `.` and `..` components without
/// touching the filesystem (references may use `..` relative to a project dir).
fn normalize(path: &Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// No csproj found but csharp detected via .cs sources: one unit for the root.
fn extract_single_unit(root: &Path) -> Result<Model, String> {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let name = canonical
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "root".to_string());
    let unit = Unit {
        name: name.clone(),
        kind: "project".to_string(),
        path: ".".to_string(),
        root: String::new(),
        crate_ids: BTreeSet::new(),
    };

    let (namespaces, usings) = scan_project_sources(&canonical)?;
    let mut soft_structure: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut sorted: Vec<String> = namespaces.iter().map(|ns| ns_to_module(ns)).collect();
    let root_key = root_module_key(&name);
    if usings.contains_key("") && !sorted.contains(&root_key) {
        sorted.push(root_key.clone());
    }
    sorted.sort();
    if !sorted.is_empty() {
        soft_structure.insert(name.clone(), sorted);
    }

    let mut namespace_unit: BTreeMap<String, String> = BTreeMap::new();
    for ns in &namespaces {
        namespace_unit.insert(ns.clone(), name.clone());
    }

    let mut module_edge_map: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut module_external: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    // No csproj means no declared references: nothing can be attributed as a
    // used package, so the external tier stays empty (mirrors "a namespace that
    // maps to no referenced package is not a dependency fact").
    let referenced: Vec<String> = Vec::new();
    let from_module = |from_ns: &str| {
        if from_ns.is_empty() {
            root_key.clone()
        } else {
            ns_to_module(from_ns)
        }
    };
    for (from_ns, targets) in &usings {
        for target in targets {
            match resolve_owner(target, &namespace_unit) {
                Some(_owner) => {
                    let from = from_module(from_ns);
                    let to = ns_to_module(target);
                    if from != to {
                        module_edge_map.entry((from, to)).or_default();
                    }
                }
                None => {
                    for package in used_packages_for_using(target, &referenced) {
                        let from = from_module(from_ns);
                        module_external.entry(from).or_default().insert(package);
                    }
                }
            }
        }
    }
    let mut module_edges: Vec<ModuleEdge> = module_edge_map
        .into_iter()
        .map(|((from, to), symbols)| {
            let mut symbols: Vec<String> = symbols.into_iter().collect();
            symbols.sort();
            ModuleEdge {
                unit: name.clone(),
                from,
                to,
                symbols,
            }
        })
        .collect();
    module_edges.sort_by(|left, right| {
        (&left.unit, &left.from, &left.to).cmp(&(&right.unit, &right.from, &right.to))
    });

    let mut module_external_model: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (module, crates) in module_external {
        let mut sorted: Vec<String> = crates.into_iter().collect();
        sorted.sort();
        module_external_model.insert(module, sorted);
    }

    Ok(Model {
        schema_version: 1,
        language: "csharp".to_string(),
        units: vec![unit],
        edges: Vec::new(),
        usage: Default::default(),
        soft_structure,
        external: Vec::new(),
        module_edges,
        manifest: None,
        root_public_exports: Default::default(),
        root_glob_exports: Default::default(),
        root_empty_glob_exports: Default::default(),
        module_external: module_external_model,
        root_module_declarations: Default::default(),
        unit_manifests: Default::default(),
        unresolved_module_files: Default::default(),
        test_gated_modules: Default::default(),
        facade_roots: Default::default(),
    })
}