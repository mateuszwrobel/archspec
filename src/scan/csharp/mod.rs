pub mod syntax;

use crate::model::{
    DeclaredFacts, Edge, EventBoundShape, EventRecordMember, EventShapes, ManifestInfo, Model,
    ModuleEdge, Role, Unit, matching_exclusion,
};

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use walkdir::WalkDir;

/// Per-unit source facts: declared namespaces, per-namespace `using` targets,
/// the type-position reference map ([`syntax::TypeFacts`], US 08), the
/// entrypoint-gate evidence (whether the unit has a Program/Startup file —
/// see [`is_entrypoint_source_file`]) and whether the registration family
/// ([`DI_REGISTRATION_FAMILY`]) is called ANYWHERE in the unit's production
/// files (S3: composition is the gate AND this fact) and whether an
/// `[assembly: TypeForwardedTo]` attribute appears (the facade predicate's
/// re-export evidence, S4).
type ProjectSources = (
    BTreeSet<String>,
    BTreeMap<String, BTreeSet<String>>,
    Option<syntax::TypeFacts>,
    syntax::ShapeFacts,
    bool,
    bool,
    bool,
);
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
/// A project the test-tier predicate recognizes (see [`is_test_project`] and
/// [`is_test_sdk`]) is C#'s test tier: whole projects
/// are the test unit here, so like go's `*_test.go` files they are excluded at
/// the scan from every production tier — no serialized marker, by the go
/// precedent of excluding at the scan when the fact cannot be module-path data.
/// Every dropped project is stated on the report surface with one
/// `test-gated unit:` line (GitHub issue #39: a silent disappearance is an
/// audit hazard, like a silent promotion).
/// A project the author declared out of the architecture with an
/// `excluded_sources` entry joins the same never-silent discipline at
/// project discovery (GitHub issue #49): one `excluded unit: <path> —
/// excluded_sources` census line per dropped project, its namespaces kept
/// as DEAD prefixes exactly like the test-gated drop, and no unit, no unit
/// edge and no module edge from it. The consult is the existing component-
/// wise form of `model::matching_exclusion` (entry `external` matches
/// `external/Ext.csproj`, never `externalapp/Ext.csproj`); glob-form
/// entries match no csproj path and keep the zero-match liveness advisory
/// unchanged. File-level pruning INSIDE a kept project (GitHub issue #60)
/// lives in [`scan_project_sources`]: the same entries are consulted on
/// each `.cs` file's root-relative path behind the unchanged mechanical
/// gate, and a pruned file contributes no namespace, no using, no type
/// fact, no shape fact and no composition evidence — one sorted
/// `excluded source: <path> — excluded_sources` census line per pruned
/// file, kept projects only (a project already dropped at discovery
/// never reaches the file gate, so no path states itself twice). The
/// entry liveness report (`spec.rs`, "any file counts") stays
/// driver-independent: an entry that prunes files under a kept project
/// is ACTIVE and prints no `matched nothing` advisory.
pub fn extract(root: &Path, facts: &DeclaredFacts) -> Result<Model, String> {
    // Absolute base so Include references (which may be absolute or use `..`
    // back to the tree) resolve identically regardless of how the root was
    // given (relative, `.`, or absolute).
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let projects = collect_projects(&root);
    if projects.is_empty() {
        return extract_single_unit(&root, facts);
    }

    // Test-tier classification precedes every production fact; the predicate
    // is [`is_test_project`] (package evidence) and [`is_test_sdk`] (the csproj
    // `Sdk` attribute). Like the go driver a test project is excluded at
    // the scan with no serialized fact — no unit, no edge, no soft tier, no
    // external attribution, no manifest entry — so a tree with test projects
    // scans to the same model as the same tree without them, and
    // `verify`/`update` consume the shape unchanged (nothing left to gate).
    // The exclusion is never silent (GitHub issue #39): every dropped project
    // renders one report-only census line, in sorted project order.
    // Dropped projects keep their declared namespaces as DEAD prefixes: a
    // production `using App.Tests.Specs;` names the test tier, not a module of
    // the referencing unit, and prefix-owner resolution must not resurrect the
    // dropped project as a phantom edge, module, or seeded boundary.
    let mut production: Vec<(String, std::path::PathBuf, ManifestInfo)> = Vec::new();
    let mut dropped_namespaces: BTreeSet<String> = BTreeSet::new();
    let mut test_gated_lines: Vec<String> = Vec::new();
    // Suppressed test markers ride out of every `manifest_info` call —
    // dropped projects included — so a marker the conditions hide states
    // itself once whatever gate its project lands on (issue #45, rung 1).
    let mut marker_facts: Vec<(std::path::PathBuf, usize, String)> = Vec::new();
    // Declared-exclusion census lines (GitHub issue #49), the never-silent
    // law's second class at the project tier, sorted before rendering.
    let mut excluded_lines: Vec<String> = Vec::new();
    for (rel_path, full_path) in &projects {
        let project_dir = full_path.parent().unwrap_or(&root);
        let manifest = manifest_info(project_dir, &root, full_path, &mut marker_facts);
        // The declared-exclusion consult precedes the test gate: an entry the
        // author declared is the stronger statement, and one dropped project
        // prints exactly one census line. The consult is the existing
        // component-wise prefix/dir form over the project-relative csproj
        // path (issue #49) — no second matcher, no glob semantics here (glob
        // entries match no csproj path and keep the liveness advisory); the
        // mechanical `is_excluded_cs` gate in `collect_projects` stays the
        // first door and this consult only adds drops. The manifest above
        // still rode out of `manifest_info` (the marker law), but the project
        // enters no tier from here.
        if matching_exclusion(&facts.exclusions, Path::new(rel_path)).is_some() {
            let unit_path = match Path::new(rel_path).parent() {
                Some(parent) if !parent.as_os_str().is_empty() => {
                    parent.to_string_lossy().into_owned()
                }
                _ => ".".to_string(),
            };
            excluded_lines.push(format!("excluded unit: {unit_path} — excluded_sources"));
            // DEAD prefixes, the test-gated law: a production `using` that
            // names an excluded project's namespace must not resurrect it as
            // a phantom edge, module or seeded boundary. The declared facts
            // ride the walk so an entry-internal file of a dropped project
            // contributes nothing here either; no census sink — the project
            // has already stated its one line above (issue #60, no
            // double-report law).
            dropped_namespaces.extend(scan_project_sources(project_dir, facts, &root, None)?.0);
            continue;
        }
        let evidence = test_gate_evidence(&manifest, project_sdk(full_path).as_deref());
        if let Some(marker) = evidence {
            let unit_path = match Path::new(rel_path).parent() {
                Some(parent) if !parent.as_os_str().is_empty() => {
                    parent.to_string_lossy().into_owned()
                }
                _ => ".".to_string(),
            };
            test_gated_lines.push(format!("test-gated unit: {unit_path} — {marker}"));
            dropped_namespaces.extend(scan_project_sources(project_dir, facts, &root, None)?.0);
            continue;
        }
        production.push((rel_path.clone(), full_path.clone(), manifest));
    }

    let mut units = Vec::new();
    let mut unit_manifests: BTreeMap<String, ManifestInfo> = BTreeMap::new();
    // Unit name -> its csproj path: the identity properties
    // (`RootNamespace`/`AssemblyName`) resolve the unit's root namespace in
    // the source-scan pass below (roles reality US 01).
    let mut unit_csproj: BTreeMap<String, std::path::PathBuf> = BTreeMap::new();
    for (rel_path, full_path, manifest) in &production {
        let name = project_name(rel_path);
        unit_csproj.insert(name.clone(), full_path.clone());
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
            if let Some(name) = unit_by_csproj.get(&resolved)
                && from_key != resolved {
                    edge_set.insert((from.clone(), name.clone()));
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

    // unit name -> module path -> public type names (US 01): the union of the
    // production units' public-API facts, attributed like the soft tier
    // (namespace `A.B` -> `A::B`; global declarations -> the unit root key).
    // A namespace declared by several units UNIONS its types (the
    // module_external flatten law: no last-write-wins).
    let mut module_public_types: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    // Scan each project's sources once: namespaces + per-namespace usings
    // (+ type-position facts under `syntax`). `unit_manifests` already carries
    // the resolved csproj facts (test-tier projects were dropped with the
    // classification pass above), so the loop never reparses a manifest.
    let mut scanned: ScannedFiles = BTreeMap::new();
    // Unit name -> its resolved root namespace (roles reality US 01): the
    // address every roles claim and every namespace-less ("sentinel") file
    // fact uses — see [`resolve_root_namespace`].
    let mut unit_root_ns: BTreeMap<String, String> = BTreeMap::new();
    // Tree-wide declared-type map (namespace -> type names it declares): the
    // precision anchor for bare-name resolution (US 08), merged across
    // production units so a `using App.Core;` + `new Engine()` resolves even
    // when `Engine` lives in the referenced project. Dropped test projects
    // contribute nothing (their references are no production fact anyway).
    let mut declared_types: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    // Event payload-shape accumulators (workplan archspec_async_depends,
    // D2/M5, scenario 04): the publisher record census attributed like the
    // public-API facts above (namespace -> `::` module; root declarations
    // -> the unit-root key), the enum census likewise, and the consumer
    // bound shapes keyed by UNIT — a bind is code in a unit, so its owner-
    // ship needs no namespace resolution. Merge is union: the same record
    // declared as a partial in two files states the union of its members.
    let mut event_records: BTreeMap<String, BTreeMap<String, Vec<EventRecordMember>>> =
        BTreeMap::new();
    let mut event_bases: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    let mut event_enums: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut event_bounds: BTreeMap<String, BTreeMap<String, EventBoundShape>> = BTreeMap::new();
    // File-level declared-exclusion census (GitHub issue #60), the never-silent
    // law's third class: one line per `.cs` file a declared entry prunes from
    // a KEPT project's source walk. A set so nested-csproj overlap (two walks
    // reaching one file) still states each path exactly once; iteration order
    // is sorted.
    let mut excluded_file_lines: BTreeSet<String> = BTreeSet::new();
    for unit in &units {
        let project_dir = if unit.path == "." {
            root.to_path_buf()
        } else {
            root.join(&unit.path)
        };
        let (namespaces, usings, type_facts, shape_facts, entrypoint_file, registration_family, forwarded) =
            scan_project_sources(&project_dir, facts, &root, Some(&mut excluded_file_lines))?;
        // The unit's root address resolves from its csproj identity properties
        // and its declared namespaces BEFORE any fact is attributed (US 01).
        let csproj_root = unit_csproj
            .get(&unit.name)
            .and_then(|path| csproj_root_namespace(path));
        let root_ns = resolve_root_namespace(&unit.name, csproj_root.as_deref(), &namespaces);
        unit_root_ns.insert(unit.name.clone(), root_ns.clone());
        let root_key = ns_to_module(&root_ns);
        if let Some(facts) = &type_facts {
            for (ns, names) in &facts.declared_types {
                declared_types
                    .entry(ns.clone())
                    .or_default()
                    .extend(names.iter().cloned());
            }
            // Public-API facts (US 01): production-tier declarations only —
            // test projects were dropped before this loop and their
            // declarations join nothing. Attribution mirrors the soft tier.
            for (ns, names) in &facts.public_types {
                let module = if ns.is_empty() {
                    root_key.clone()
                } else {
                    ns_to_module(ns)
                };
                module_public_types
                    .entry(module)
                    .or_default()
                    .extend(names.iter().cloned());
            }
        }
        // Shape-fact attribution (scenario 04): records and enums mirror
        // the public-API attribution above; bounds key on the unit. The
        // merge is a UNION — a partial record's members join across files,
        // and a bound field observed in any file of the unit is bound by
        // the unit.
        for (ns, records) in &shape_facts.records {
            let module = if ns.is_empty() {
                root_key.clone()
            } else {
                ns_to_module(ns)
            };
            let by_name = event_records.entry(module).or_default();
            for (name, members) in records {
                let entry = by_name.entry(name.clone()).or_default();
                for (member, field_type) in members {
                    let member = EventRecordMember {
                        name: member.clone(),
                        field_type: field_type.clone(),
                    };
                    if !entry.contains(&member) {
                        entry.push(member);
                    }
                }
                entry.sort();
                entry.dedup();
            }
        }
        for (ns, bases) in &shape_facts.bases {
            let module = if ns.is_empty() {
                root_key.clone()
            } else {
                ns_to_module(ns)
            };
            let by_name = event_bases.entry(module).or_default();
            for (name, list) in bases {
                let entry = by_name.entry(name.clone()).or_default();
                entry.extend(list.iter().cloned());
                entry.sort();
                entry.dedup();
            }
        }
        for (ns, names) in &shape_facts.enums {
            let module = if ns.is_empty() {
                root_key.clone()
            } else {
                ns_to_module(ns)
            };
            event_enums.entry(module).or_default().extend(names.iter().cloned());
        }
        if !shape_facts.bounds.is_empty() {
            let unit_bounds = event_bounds.entry(unit.name.clone()).or_default();
            for (name, (fields, projected)) in &shape_facts.bounds {
                let entry = unit_bounds.entry(name.clone()).or_default();
                entry.fields.extend(fields.iter().cloned());
                entry.projected.extend(projected.iter().cloned());
            }
        }
        scanned.insert(
            unit.name.clone(),
            (
                namespaces.clone(),
                usings.clone(),
                type_facts,
                shape_facts,
                entrypoint_file,
                registration_family,
                forwarded,
            ),
        );
        let mut sorted: Vec<String> = namespaces.iter().map(|ns| ns_to_module(ns)).collect();
        // Sentinel-presence is the truth condition: the "" key exists exactly
        // when some namespace-less file contributed at least one using (the
        // entry is created with its first target), so the root module joins the
        // soft structure iff it carries a fact — a using-free namespace-less
        // file adds no module, like any other empty one.
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
            if let Some(previous) = namespace_unit.get(ns)
                && previous != &unit.name {
                    ambiguous_namespaces.insert(ns.clone());
                }
            namespace_unit.insert(ns.clone(), unit.name.clone());
        }
        module_external.insert(unit.name.clone(), BTreeMap::new());
        module_edge_map.insert(unit.name.clone(), BTreeMap::new());
    }

    // Role derivations (roles US 02) address the unit's root module key, and
    // distinct units CAN share one key (unit `A.B`'s namespace `A.B` and unit
    // `A.B.C`'s namespace-less composition root both resolve to `A::B`), so
    // Role claims are collected per unit at the unit's resolved root and
    // merged at the end: same role states once, different roles on one key
    // state nothing (ambiguity guard) — never a cross-unit override.
    let mut role_claims: Vec<(String, Role)> = Vec::new();

    // Cross-project namespace ownership is only fully known after every unit is
    // scanned, so resolve usings in a second pass.
    for unit in &units {
        let (_namespaces, usings, type_facts, _shape_facts, entrypoint_file, registration_family, forwarded) =
            scanned
            .get(&unit.name)
            .cloned()
            .unwrap_or_else(|| {
                (
                    BTreeSet::new(),
                    BTreeMap::new(),
                    None,
                    syntax::ShapeFacts::default(),
                    false,
                    false,
                    false,
                )
            });
        let referenced: Vec<String> = unit_manifests
            .get(&unit.name)
            .map(|manifest| manifest.dependencies.clone())
            .unwrap_or_default();
        // The unit-root sentinel ("" = namespace-less file) translates to the
        // project's root module key before it can reach any model tier. The
        // scan pass resolved every production unit; the fallback is the
        // ladder's own last voice (the unit's full name), unreachable here.
        let root_ns = unit_root_ns
            .get(&unit.name)
            .cloned()
            .unwrap_or_else(|| unit.name.clone());
        let root_key = ns_to_module(&root_ns);
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
                    Some((prefix, owner)) if owner == unit.name => {
                        // Within-project: a soft module edge.
                        let from = from_module(from_ns);
                        let (to, symbols) = edge_target(prefix, target);
                        if from != to {
                            module_edge_map
                                .entry(unit.name.clone())
                                .or_default()
                                .entry((from, to))
                                .or_default()
                                .extend(symbols);
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
                            let (to, symbols) = edge_target(prefix, target);
                            if from != to {
                                module_edge_map
                                    .entry(unit.name.clone())
                                    .or_default()
                                    .entry((from, to))
                                    .or_default()
                                    .extend(symbols);
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
        // ---- Type-position edges (US 08) -----------------------------------
        // The SAME declared-namespace ownership machinery resolves every type
        // reference, but the ownership-MISS branch records NOTHING: the
        // external tier stays using-driven (decision: type-position
        // references never add module_external entries, so the
        // tier is purely a using-driven fact). Qualified references and
        // surviving bare candidates merge into one set per context, so a
        // fully-qualified and a bare reference to the same type dedup into
        // the SAME (from, to) edge — including edges the using pass already
        // recorded, since module_edge_map is shared.
        if let Some(facts) = &type_facts {
            let mut references: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for (from_ns, qualified) in &facts.qualified {
                references
                    .entry(from_ns.clone())
                    .or_default()
                    .extend(qualified.iter().cloned());
            }
            for (from_ns, candidates) in &facts.candidates {
                let mut by_ident: BTreeMap<&str, BTreeSet<&String>> = BTreeMap::new();
                for (ident, reference) in candidates {
                    by_ident.entry(ident.as_str()).or_default().insert(reference);
                }
                for (ident, cands) in by_ident {
                    // A candidate is REAL when it reduces to `P.ident` with P
                    // a declared namespace and ident a TYPE P declares (the
                    // case-sensitive anchor that kills local-variable noise).
                    // Several REAL resolutions of one identifier could not
                    // compile, so honestly none is recorded (ambiguity guard).
                    let mut survivors = cands
                        .into_iter()
                        .filter(|reference| {
                            candidate_resolves_as_declared_type(
                                reference,
                                ident,
                                &namespace_unit,
                                &declared_types,
                            )
                        });
                    let Some(reference) = survivors.next().cloned() else {
                        continue;
                    };
                    if survivors.next().is_some() {
                        continue;
                    }
                    references.entry(from_ns.clone()).or_default().insert(reference);
                }
            }
            for (from_ns, references) in references {
                let from = from_module(&from_ns);
                for reference in &references {
                    if falls_under_namespace(reference, &dropped_namespaces) {
                        continue;
                    }
                    match resolve_owner_prefix(reference, &namespace_unit) {
                        Some((prefix, owner)) if owner == unit.name => {
                            let (to, symbols) = edge_target(prefix, reference);
                            if from != to {
                                module_edge_map
                                    .entry(unit.name.clone())
                                    .or_default()
                                    .entry((from.clone(), to))
                                    .or_default()
                                    .extend(symbols);
                            }
                        }
                        // The same honesty guards as the using path: an
                        // ambiguously declared prefix or an unreachable
                        // owner could not compile — no source fact.
                        Some((prefix, owner))
                            if !ambiguous_namespaces.contains(prefix)
                                && reference_reach
                                    .get(unit.name.as_str())
                                    .is_some_and(|reachable| reachable.contains(&owner)) =>
                        {
                            let (to, symbols) = edge_target(prefix, reference);
                            if from != to {
                                module_edge_map
                                    .entry(unit.name.clone())
                                    .or_default()
                                    .entry((from.clone(), to))
                                    .or_default()
                                    .extend(symbols);
                            }
                        }
                        // A guard below refused (ambiguously declared prefix
                        // or an owner not reaching the symbol): no source
                        // fact — the inner condition was the arm's body.
                        Some(_) => {}
                        // Ownership miss (BCL/NuGet type used without a
                        // using): no edge and NO external attribution — by
                        // decision the external tier stays the using path's.
                        None => {}
                    }
                }
            }
        }
        // Roles derive from this unit's own facts (roles US 02, S3): the
        // facade predicate reads the root module's using/type facts; the
        // composition predicate is an ENTRYPOINT GATE — a Program/Startup
        // file or the csproj's own run declaration (host Sdk, OutputType=Exe)
        // — AND a registration-family fact anywhere in the unit's production
        // files. Registration calls are a fact source ONLY — they join no
        // edge map and engage no rule yet.
        let gate_open = entrypoint_file
            || unit_csproj
                .get(&unit.name)
                .is_some_and(|path| csproj_entrypoint_gate(path));
        derive_unit_roles(
            &root_ns,
            &usings,
            &type_facts,
            registration_family && gate_open,
            forwarded,
            &mut role_claims,
        );
    }

    // Aggregate module edges into the model shape.
    let mut module_edges: Vec<ModuleEdge> = Vec::new();
    for (unit_name, edges) in &module_edge_map {
        for ((from, to), symbols) in edges {
            let mut symbols: Vec<String> = symbols.iter().cloned().collect();
            symbols.sort();
            module_edges.push(ModuleEdge {
                type_only: false,
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

    // Same role from several units states once; conflicting claims on one key
    // state nothing (ambiguity guard — the merge is per-claim, not per-predicate).
    let roles = roles_map(role_claims);

    // Sorted vectors: the fact tier is deterministic like every other map.
    let module_public_types: BTreeMap<String, Vec<String>> = module_public_types
        .into_iter()
        .map(|(module, types)| (module, types.into_iter().collect()))
        .collect();

    Ok(Model {
        units,
        edges,
        soft_structure,
        external: external_sorted,
        module_edges,
        manifest,
        module_public_types,
        module_external: module_external_model,
        unit_manifests,
        roles,
        // The `test-gated unit` census first (project order, unchanged
        // bytes), then the declared-exclusion census sorted by line (GitHub
        // issue #49 — deterministic, one line per dropped project), then the
        // file-level declared-exclusion census, sorted by path (GitHub issue
        // #60 — one line per excluded source file), then one sorted line per
        // suppressed test marker under an unevaluated condition (GitHub
        // issue #45, rung 1) — report-only bytes, serde-skipped, never the
        // machine body.
        report_only_lines: {
            excluded_lines.sort();
            test_gated_lines.extend(excluded_lines);
            test_gated_lines.extend(excluded_file_lines);
            test_gated_lines.extend(test_marker_lines(&marker_facts, &root));
            test_gated_lines
        },
        // The drift census reads these rows from `verify`'s own scan; no
        // serialized byte carries them (serde-skip in `model.rs`), so every
        // non-csharp and every non-declaring model byte stays identical.
        event_shapes: EventShapes {
            records: event_records,
            bases: event_bases,
            enums: event_enums,
            bounds: event_bounds,
        },
        ..Model::new("csharp")
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

/// The C# test-tier predicate (package evidence): a project is a test project
/// when its resolved package set (csproj `PackageReference` items plus the
/// nearest central props — the same vocabulary `manifest_info` reports) names
/// a test runner or test host: the project-tier mirror of rust's `#[cfg(test)]`
/// modules and go's `*_test.go` files. Runner-package evidence, evaluated per
/// resolved dependency id:
/// - the xunit family (`xunit`, `Xunit.Runner.VisualStudio`, …) or the nunit
///   family (`NUnit`, `NUnit3TestAdapter`, …) — a family PREFIX match, so
///   satellites (`xunit.analyzers`, `NUnit.Analyzers`) count as evidence by
///   design: they appear only in test projects;
/// - exactly `Microsoft.NET.Test.Sdk` — the classic MSTest SDK shape;
/// - the Microsoft.Testing.Platform family (`Microsoft.Testing.Platform`,
///   `Microsoft.Testing.Platform.MSBuild`, …) and the MSTest family
///   (`MSTest.TestFramework`, `MSTest.TestAdapter`, …) — prefix matches, the
///   satellites-by-design law applied to the MSTest 3 / testing-platform
///   shape (GitHub issue #39: every `MSTest.*` id appears only in test
///   projects, so the pre-SDK MSTest residual is retired).
///
/// The project-shape half of the evidence — a csproj whose root `Sdk` attribute
/// names the `MSTest.Sdk` family — is [`is_test_sdk`] and reaches the tier
/// through [`test_gate_evidence`], the predicate every consumer uses.
///
/// NuGet ids are case-insensitive, so the match lowercases. An assertion
/// library without a runner (`FluentAssertions`) is NOT evidence — production
/// tooling uses it too. Project names play no part: `.Tests` is lore, a
/// runner-free `LegacyTests` stays production and a `Helpers` project
/// referencing xunit is test tier.
pub fn is_test_project(manifest: &ManifestInfo) -> bool {
    manifest
        .dependencies
        .iter()
        .map(String::as_str)
        .any(is_test_framework_package)
}

/// The csproj test-SDK predicate: the root `Project` element's `Sdk` attribute
/// names the MSTest SDK (GitHub issue #39) — the `MSTest.Sdk` family by
/// case-insensitive prefix, so the versioned spelling `MSTest.Sdk/3.6.4`
/// counts the same as bare `MSTest.Sdk`. An MSTest 3 / Microsoft.Testing.
/// Platform project declares its SDK this way and carries no runner package
/// at all, so the attribute alone must mark the tier.
pub fn is_test_sdk(sdk: &str) -> bool {
    sdk.to_ascii_lowercase().starts_with("mstest.sdk")
}

/// The combined test-gate evidence (GitHub issue #39): the first test marker
/// the project carries, `Some(marker)` verbatim — a matched package id in
/// sorted order, else the `Sdk` attribute value. `None` is production. Every
/// consumer of the C# test tier adjudicates through this one predicate: the
/// scan driver (drop + census line) and the inspect renderer (namespace
/// ownership), so the two can never disagree.
pub fn test_gate_evidence(manifest: &ManifestInfo, sdk: Option<&str>) -> Option<String> {
    manifest
        .dependencies
        .iter()
        .find(|dependency| is_test_framework_package(dependency.as_str()))
        .cloned()
        .or_else(|| sdk.filter(|value| is_test_sdk(value)).map(str::to_string))
}

/// The root `Project` element's `Sdk` attribute of one csproj, if any. Only
/// the root element counts: an `Sdk` attribute on an `Import` element is an
/// import mechanism, not the project's SDK identity. Consumed by the scan
/// driver and the inspect renderer through [`test_gate_evidence`].
pub fn project_sdk(full_path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(full_path).ok()?;
    let mut reader = quick_xml::Reader::from_str(&raw);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) | Ok(quick_xml::events::Event::Empty(ref e)) => {
                if e.name().as_ref() == "Project" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == "Sdk" {
                            let value = attr.value.to_string();
                            if !value.is_empty() {
                                return Some(value);
                            }
                        }
                    }
                    return None;
                }
            }
            Ok(quick_xml::events::Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

/// The id-level runner evidence behind [`is_test_project`]: the xunit, nunit,
/// Microsoft.Testing.Platform and MSTest families by case-insensitive prefix,
/// and the Microsoft test host SDK by exact id. The full evidence rules —
/// satellites, the SDK shapes, assertion libraries, and the irrelevance of
/// project names — are stated on [`is_test_project`].
fn is_test_framework_package(dependency: &str) -> bool {
    let id = dependency.to_ascii_lowercase();
    id.starts_with("xunit")
        || id.starts_with("nunit")
        || id == "microsoft.net.test.sdk"
        || id.starts_with("microsoft.testing.platform")
        || id.starts_with("mstest.")
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
/// A `PackageReference` covered by an unevaluated MSBuild `Condition` (on the
/// item, its `ItemGroup`, or inside a `Choose` subtree) never gates: when its
/// `Include` names a test package it is EXCLUDED from `dependencies` — so the
/// test-tier predicates stay clean — and recorded in `markers` for the
/// report-only diagnostic (GitHub issue #45, rung 1); a conditioned non-test
/// reference counts, its presence being real. `markers` carries
/// `(file, line, include)` triples for every project walked through this
/// function, dropped ones included; the caller renders them.
pub fn manifest_info(
    project_dir: &Path,
    root: &Path,
    full_path: &Path,
    markers: &mut Vec<(std::path::PathBuf, usize, String)>,
) -> ManifestInfo {
    let raw = std::fs::read_to_string(full_path).unwrap_or_default();
    let mut publish = None;
    let mut dependencies = BTreeSet::new();
    let mut reader = quick_xml::Reader::from_str(&raw);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_is_packable = false;
    let mut scopes = CondStack(Vec::new());
    let mut previous = 0usize;
    loop {
        let event = reader.read_event_into(&mut buf);
        let position = reader.buffer_position() as usize;
        let since = &raw[previous..position];
        previous = position;
        match event {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                if e.name().as_ref() == "ItemGroup" {
                    scopes.push(has_condition(e));
                } else if e.name().as_ref() == "Choose" {
                    scopes.push(true);
                } else if e.name().as_ref() == "PackageReference" {
                    note_csproj_reference(
                        e,
                        &scopes,
                        full_path,
                        event_line(&raw, since, position, e.name().as_ref()),
                        markers,
                        &mut dependencies,
                    );
                } else if e.name().as_ref() == "IsPackable" {
                    in_is_packable = true;
                }
            }
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                if e.name().as_ref() == "PackageReference" {
                    note_csproj_reference(
                        e,
                        &scopes,
                        full_path,
                        event_line(&raw, since, position, e.name().as_ref()),
                        markers,
                        &mut dependencies,
                    );
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
                if e.name().as_ref() == "ItemGroup" || e.name().as_ref() == "Choose" {
                    scopes.pop();
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

    dependencies.extend(central_reference_names(project_dir, root, markers));

    ManifestInfo {
        publish,
        dependencies: dependencies.into_iter().collect(),
        features: Vec::new(),
    }
}

/// One csproj `PackageReference`: unconditional items count as before; a
/// conditioned item whose `Include` names a test package is excluded from
/// `dependencies` and recorded as a marker (rung-1 posture — a marker the
/// scan cannot evaluate gates nothing and says so); a conditioned non-test
/// item counts, its presence being real (GitHub issue #45, rung 1). `line`
/// is the source line of the item's opening tag, computed by the walk with
/// [`event_line`].
fn note_csproj_reference(
    event: &quick_xml::events::BytesStart<'_>,
    scopes: &CondStack,
    full_path: &Path,
    line: usize,
    markers: &mut Vec<(std::path::PathBuf, usize, String)>,
    dependencies: &mut BTreeSet<String>,
) {
    let Some(include) = include_attribute(event).filter(|value| !value.is_empty()) else {
        return;
    };
    if has_condition(event) || scopes.conditioned() {
        if is_test_framework_package(&include) {
            markers.push((full_path.to_path_buf(), line, include));
        } else {
            dependencies.insert(include);
        }
    } else {
        dependencies.insert(include);
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
fn central_reference_names(
    project_dir: &Path,
    root: &Path,
    markers: &mut Vec<(std::path::PathBuf, usize, String)>,
) -> BTreeSet<String> {
    let mut dir = project_dir.to_path_buf();
    loop {
        let mut found: Option<BTreeSet<String>> = None;
        for file_name in ["Directory.Packages.props", "Directory.Build.props"] {
            let candidate = dir.join(file_name);
            if candidate.is_file() {
                found
                    .get_or_insert_with(BTreeSet::new)
                    .extend(props_package_references(&candidate, markers));
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
/// bearing an MSBuild `Condition` — on the item, its `ItemGroup`, or inside
/// a `Choose`/`When`/`Otherwise` subtree — is ignored as tier evidence and
/// as an external dependency alike: conditions (`Condition="'$(IsTestProject)'=='true'"`
/// is the common test-wiring shape) are unevaluable without MSBuild, and
/// counting them would silently mark every project below the props file test
/// tier and erase the production model. Honesty over false precision: the
/// conditional reference is not modeled at all. The suppression is never
/// silent for a TEST marker (GitHub issue #45, rung 1): when the suppressed
/// item's `Include` names a test package, the item is recorded in `markers`
/// so the run states one report-only diagnostic line for it.
fn props_package_references(
    path: &Path,
    markers: &mut Vec<(std::path::PathBuf, usize, String)>,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return names;
    };
    let mut reader = quick_xml::Reader::from_str(&raw);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut scopes = CondStack(Vec::new());
    let mut previous = 0usize;
    loop {
        let event = reader.read_event_into(&mut buf);
        let position = reader.buffer_position() as usize;
        let since = &raw[previous..position];
        previous = position;
        match event {
            Ok(quick_xml::events::Event::Start(ref e))
                if e.name().as_ref() == "ItemGroup" || e.name().as_ref() == "Choose" =>
            {
                // A `Choose` subtree is unevaluated as a whole — the push
                // covers its `When` and `Otherwise` arms without tracking
                // those elements.
                scopes.push(e.name().as_ref() == "Choose" || has_condition(e));
            }
            Ok(quick_xml::events::Event::End(ref e))
                if e.name().as_ref() == "ItemGroup" || e.name().as_ref() == "Choose" =>
            {
                scopes.pop();
            }
            Ok(quick_xml::events::Event::Start(ref e))
            | Ok(quick_xml::events::Event::Empty(ref e))
                if e.name().as_ref() == "PackageReference" =>
            {
                if scopes.conditioned() {
                    // Suppressed by an enclosing scope: excluded from the
                    // names whatever the item's own attributes say; a test
                    // package states itself as a marker.
                    if let Some(include) = include_attribute(e)
                        .filter(|value| !value.is_empty() && is_test_framework_package(value))
                    {
                        markers.push((
                            path.to_path_buf(),
                            event_line(&raw, since, position, e.name().as_ref()),
                            include,
                        ));
                    }
                } else {
                    match reference_include(e) {
                        Some(include) => {
                            if !include.is_empty() {
                                names.insert(include);
                            }
                        }
                        None => {
                            // `None` means the item carries its own
                            // `Condition` or no `Include` at all; a
                            // conditioned test package names itself as a
                            // marker, a missing `Include` stays silent.
                            if let Some(include) = include_attribute(e)
                                .filter(|value| is_test_framework_package(value))
                            {
                                markers.push((
                                    path.to_path_buf(),
                                    event_line(&raw, since, position, e.name().as_ref()),
                                    include,
                                ));
                            }
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
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

/// The `Include` attribute value of an element regardless of any `Condition`
/// — the condition walk consults it to judge whether a suppressed
/// `PackageReference` names a test package (GitHub issue #45, rung 1).
fn include_attribute(event: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    event
        .attributes()
        .flatten()
        .find(|attr| attr.key.as_ref() == "Include")
        .map(|attr| attr.value.into_owned())
}

/// MSBuild condition scopes for the csproj and props XML walks (GitHub
/// issue #45, rung 1). One entry per OPEN scope element: an `ItemGroup`
/// pushes whether it carries a `Condition`; a `Choose` pushes `true` always,
/// because every `Choose` subtree — `When` and `Otherwise` arms alike — is
/// unevaluated, so the arms need no tracking of their own. A stack rather
/// than a flag is what keeps a nested unconditioned `ItemGroup` from
/// clobbering the enclosing scope's condition: the matching `End` restores
/// the parent entry. Presence only — values are never parsed. The deferred
/// condition evaluator (rung 2) attaches its semantics here.
struct CondStack(Vec<bool>);

impl CondStack {
    fn push(&mut self, conditioned: bool) {
        self.0.push(conditioned);
    }
    fn pop(&mut self) {
        self.0.pop();
    }
    /// True when the element at the current position stands under an
    /// unevaluated condition carried by any enclosing scope.
    fn conditioned(&self) -> bool {
        self.0.iter().any(|scope| *scope)
    }
}

/// The 1-based source line holding the opening `<` of the token of the event
/// just read. `position` is `Reader::buffer_position` — quick-xml 0.42 places
/// it just after the last emitted event's token — and `since_previous` is the
/// text consumed since the previous event. The line is the newlines up to
/// `position` plus one, MINUS the newlines inside the token after its
/// opening `<`, so blank lines between tags and CRLF endings never shift the
/// count and the stated line is the one that holds the element. The cut
/// scans back to the last `<name` of the slice, so an intervening XML
/// comment carrying a `<` cannot misplace it.
fn event_line(raw: &str, since_previous: &str, position: usize, name: &str) -> usize {
    let marker = format!("<{name}");
    let token_tail = match since_previous
        .rfind(marker.as_str())
        .map(|index| index + marker.len())
        .or_else(|| since_previous.rfind('<').map(|index| index + 1))
    {
        Some(start) => &since_previous[start..],
        None => "",
    };
    1 + raw[..position].matches('\n').count() - token_tail.matches('\n').count()
}

/// Render the report-only lines for suppressed test markers (GitHub issue
/// #45, rung 1): a test-runner `PackageReference` the scan does not count
/// because an unevaluated MSBuild `Condition` covers it is not gating
/// evidence, and every distinct `(file, line, include)` states itself once
/// as `test marker under unevaluated condition: <file>:<line>`, sorted by
/// that key. `file` is the marker path relative to the scan root (the root
/// itself renders the bare file name), the same path shape the
/// `test_gated_lines` rendering uses. Report-only: the lines join neither
/// the machine body nor any verdict.
fn test_marker_lines(markers: &[(std::path::PathBuf, usize, String)], root: &Path) -> Vec<String> {
    let mut seen: BTreeSet<(String, usize, String)> = BTreeSet::new();
    for (path, line, include) in markers {
        let file = match path.strip_prefix(root) {
            Ok(relative) if !relative.as_os_str().is_empty() => {
                relative.to_string_lossy().into_owned()
            }
            _ => path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default(),
        };
        seen.insert((file, *line, include.clone()));
    }
    seen.into_iter()
        .map(|(file, line, _include)| {
            format!("test marker under unevaluated condition: {file}:{line}")
        })
        .collect()
}

/// Scan every `.cs` file under a project directory for declared namespaces,
/// `using` targets (both attributed to the namespace they appear in) and the
/// type-position reference map ([`syntax::TypeFacts`]). Per file the grammar
/// walk ([`syntax::scan_cs_source`]) fills the namespace/using collections and
/// the type facts with the attribution semantics the graph assembly expects,
/// and the shape pass ([`syntax::scan_cs_shapes`]) derives the event payload
/// shapes (record members, enum census, bound fields — workplan
/// archspec_async_depends, D2/M5; the join of bindings with member-access
/// chains happens per FILE, so a variable name in one file can never borrow
/// another file's bindings).
///
/// Returns `(namespaces, usings, type_facts, shape_facts, entrypoint_file, registration_family, forwarded)`
/// where `usings` maps a namespace (dotted, as written) to the set of `using`
/// target namespaces in files of that namespace. A file-scoped `namespace Foo.Bar;`
/// owns all usings in the file; usings in a file with no namespace at all are
/// attributed to the unit root (the empty string is used as a sentinel and
/// translated later). The two composition facts (S3): `entrypoint_file` marks
/// that some file follows the Program/Startup convention
/// ([`is_entrypoint_source_file`]), and `registration_family` that a call of
/// the [`DI_REGISTRATION_FAMILY`] appears in ANY production file of the unit —
/// composition requires both, this pair plus the csproj's own gate evidence.
/// `forwarded` marks an assembly-level `TypeForwardedTo` attribute anywhere
/// in the unit — the facade predicate's re-export evidence (S4).
///
/// `facts` and `root` carry the declared-exclusion gate (GitHub issue #60):
/// every `.cs` file whose ROOT-relative path sits at or under a declared
/// entry is skipped before any fact is read — the file contributes nothing
/// to any tier. `census` is the report-only sink for the
/// `excluded source: <path> — excluded_sources` census lines: Some only
/// when this walk belongs to a KEPT project (the project-level drop from
/// issue #49 already stated its own line — the no-double-report law).
fn scan_project_sources(
    project_dir: &Path,
    facts: &DeclaredFacts,
    root: &Path,
    mut census: Option<&mut BTreeSet<String>>,
) -> Result<ProjectSources, String> {
    let mut namespaces: BTreeSet<String> = BTreeSet::new();
    let mut usings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut type_facts = Some(syntax::TypeFacts::default());
    let mut shape_facts = syntax::ShapeFacts::default();
    let mut entrypoint_file = false;
    let mut registration_family = false;
    let mut forwarded = false;

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
        // Declared-exclusion gate (GitHub issue #60): the author's entries
        // adjudicate on the file's ROOT-relative path through the same
        // component-wise `model::matching_exclusion` the project arm uses —
        // entry `generated` matches `App/generated/Gen.cs`, never
        // `App/generated_assets/Gen.cs`. The mechanical gate above stays the
        // first door: a `bin`/`obj`/`node_modules`/dot file was never a unit
        // source, so it never states an exclusion line. A pruned file
        // contributes NO fact: no namespace, no using, no type fact, no
        // shape fact, no entrypoint/registration/forwarded evidence. The
        // census sink is Some only for kept projects, so a project already
        // dropped at discovery (issue #49) never double-reports.
        let root_rel = entry.path().strip_prefix(root).unwrap_or(rel);
        if matching_exclusion(&facts.exclusions, root_rel).is_some() {
            if let Some(lines) = census.as_deref_mut() {
                lines.insert(format!(
                    "excluded source: {} — excluded_sources",
                    root_rel.display()
                ));
            }
            continue;
        }
        let source = std::fs::read_to_string(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if is_entrypoint_source_file(path) {
            entrypoint_file = true;
        }
        registration_family |= has_di_registration_calls(&source);
        forwarded |= has_type_forwarded_to(&source);
        {
            let mut per_file = syntax::TypeFacts::default();
            syntax::scan_cs_source(&source, &mut namespaces, &mut usings, &mut per_file)
                .map_err(|err| {
                    format!("failed to parse {}: {err}", path.display())
                })?;
            if let Some(collected) = type_facts.as_mut() {
                collected.merge(per_file);
            }
        }
        // The shape pass (workplan archspec_async_depends, D2/M5): its
        // join of bindings with chains runs INSIDE the per-file call, so
        // merging below only unions already-joined results — a variable
        // name in one file never borrows another file's bindings.
        let per_file_shapes =
            syntax::scan_cs_shapes(&source).map_err(|err| {
                format!("failed to parse {}: {err}", path.display())
            })?;
        shape_facts.merge(per_file_shapes);
    }
    Ok((
        namespaces,
        usings,
        type_facts,
        shape_facts,
        entrypoint_file,
        registration_family,
        forwarded,
    ))
}

/// True when any rel-path component is `bin`, `obj`, `node_modules`, or starts
/// with `.`. Matches the inspect exclusion rule so `scan` and `inspect` see the
/// same source tree.
pub fn is_excluded_cs(rel: &std::path::Path) -> bool {
    rel.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name.starts_with('.') || name == "bin" || name == "obj" || name == "node_modules"
    })
}

/// The dotted prefixes of `target`, longest first: the whole target, then each
/// truncation at a dot, down to the first segment (for `A.B.C`: `A.B.C`,
/// `A.B`, `A`). This is the iteration behind the longest-prefix ownership
/// rule, shared by [`resolve_owner_prefix`], [`falls_under_namespace`], and the
/// inspect file graph so the scan and the inspect map can never drift on
/// which declaration owns a `using` target.
pub fn namespace_prefixes(target: &str) -> impl Iterator<Item = &str> + '_ {
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
/// namespace that is a prefix (dotted) of the target, returned together with
/// the matched declaration prefix. Returns None when the target is external.
/// The prefix decides both the owner AND whether that owner is honest (a
/// prefix declared by two units is not), and the addressing
/// rule (see [`edge_target`]).
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

/// True when a bare-name CANDIDATE reference (US 08) is real: its deepest
/// declared prefix `P` leaves exactly the single segment `ident` as the tail,
/// and `ident` is a TYPE declared under `P` (case-sensitive — a local named
/// `engine` never matches the type `Engine`). This is the anchored filter
/// that keeps the bare path silent on locals, instance chains and BCL names
/// while a qualified `using App.Core;` + `new Engine()` resolves precisely.
fn candidate_resolves_as_declared_type(
    reference: &str,
    ident: &str,
    namespace_unit: &BTreeMap<String, String>,
    declared_types: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    match resolve_owner_prefix(reference, namespace_unit) {
        Some((prefix, _owner)) => {
            let tail = reference[prefix.len()..].trim_start_matches('.');
            tail == ident && declared_types.get(prefix).is_some_and(|types| types.contains(ident))
        }
        None => false,
    }
}

/// The module-edge endpoint and crossing symbols for a `using` target that
/// resolved to a declared namespace prefix: the edge addresses the deepest
/// DECLARED prefix — the rust driver's
/// deepest-known-module rule, one addressing vocabulary across languages —
/// and carries the remaining target segments (usually the single type name a
/// type-targeted `using App.Core.Engine;` points at) as the symbols crossing
/// the boundary. A target that is itself a declared namespace (a pure
/// namespace using) keeps that endpoint with no symbols.
fn edge_target(prefix: &str, target: &str) -> (String, Vec<String>) {
    let symbols = target
        .strip_prefix(prefix)
        .map(|tail| tail.trim_start_matches('.'))
        .filter(|tail| !tail.is_empty())
        .map(|tail| tail.split('.').map(str::to_string).collect())
        .unwrap_or_default();
    (ns_to_module(prefix), symbols)
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
            if seen.insert(current)
                && let Some(next) = adjacency.get(current) {
                    queue.extend(next.iter().copied());
                }
        }
        reachable.insert(*start, seen);
    }
    reachable
}

/// True when `target` equals or falls under any namespace in `namespaces`
/// (segment-wise dotted prefixes — the same rule [`resolve_owner_prefix`] uses).
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

/// The pilot driver's event-participation derivation (workplan
/// archspec_async_depends, D3): does this unit's CODE participate in the
/// event type named by the fully-qualified dotted `event` identity — as
/// producer or consumer? True when either serialized model fact carries it:
/// (a) the unit OWNS the type — its `soft_structure` modules contain the
/// event's namespace module and the `module_public_types` census names the
/// type there; (b) the unit has a `module_edge` whose `to` module is the
/// event's namespace module — the cross-project (or same-project) `using`
/// that references it. Reads only the serialized model, never the tree; the
/// `verify::compare::async_depends` arms call it for the from-unit (arm (b))
/// of each `[[async_depends]]` entry, and the to-unit arm (arm (c)) calls it
/// as its exact-FQN leg of [`derives_subscribe_participation`].
pub fn derives_event_participation(model: &Model, unit: &str, event: &str) -> bool {
    let (namespace, name) = match event.rsplit_once('.') {
        Some((namespace, name)) => (namespace, name),
        None => ("", event),
    };
    let module = ns_to_module(namespace);
    let owns = model
        .soft_structure
        .get(unit)
        .is_some_and(|modules| modules.contains(&module))
        && model
            .module_public_types
            .get(&module)
            .is_some_and(|types| types.iter().any(|declared| declared == name));
    owns || {
        model
            .module_edges
            .iter()
            .any(|edge| edge.unit == unit && edge.to == module)
    }
}

/// The to-unit (arm (c)) subscribe side (workplan archspec_async_depends,
/// scenario 06, consumer verdict on 0.6.86): participates when the exact-FQN
/// predicate above holds OR the unit carries the event by SIMPLE NAME — the
/// type-name tail after the last `.`/`:`, the same resolution rule the drift
/// census uses at `verify::compare::async_depends::event_resolves` and
/// `check_shape_drift` (one rule, two spelling sites: the census's model-wide
/// lookup and the unit-scoped leg of [`unit_holds_type_by_simple_name`],
/// shared with the drift pairing since scenario 10). The duplicated-contract idiom gives each
/// unit its own namespaced copy of the event, so the declared FQN's namespace
/// names the PUBLISHER's module and cannot address the consumer's copy; the
/// simple-name facts that do address it are exactly the two the model already
/// derives:
/// - the unit's own declared-type census names that simple name in one of the
///   unit's modules (a copy under the unit's own namespace), or
/// - the unit's derived bound shapes carry an entry for that simple name
///   (`event_shapes.bounds`, keyed unit -> declaration simple name) — a typed
///   handler variable, an `IIntegrationEventHandler<T>` implementation's
///   `Handle(T, …)` parameter, binds fields and registers without any literal
///   `Subscribe` call.
///
/// The exact-FQN leg stays byte-for-byte the shipped shared-contracts and
/// direct-reference path; a to-unit with neither fact is still refused by
/// name (arm (c) never degrades into unconditional silence). The from-unit
/// arm (b) keeps the exact-FQN law alone.
///
/// Scenario 10 extracted the simple-name leg into [`unit_holds_type_by_simple_name`]
/// so the drift projection pairing of
/// `verify::compare::async_depends::check_shape_drift` calls the SAME
/// implementation — the pairing rule is shared, never mirrored.
pub fn derives_subscribe_participation(model: &Model, unit: &str, event: &str) -> bool {
    derives_event_participation(model, unit, event)
        || unit_holds_type_by_simple_name(model, unit, event_type_simple_name(event))
}

/// The census's tail rule (scenario 10, the extraction scenario 06 deferred):
/// the type's SIMPLE NAME inside an event identity — rust spells identities
/// with `::`, csharp with `.`; either separator's last segment is the type's
/// name. ONE implementation: arm (c) above and the drift projection pairing
/// call this — a second hand-written copy of the tail rule is the defect
/// class this function closes.
pub fn event_type_simple_name(event: &str) -> &str {
    event.rsplit(['.', ':']).next().unwrap_or(event)
}

/// The simple-name leg of scenario 06's rule as ONE implementation (arm (c)
/// is its identity caller; scenario 10 routes the drift projection pairing
/// through the same unit scoping): True when the unit's derived facts hold
/// the type named by the SIMPLE name `name` — either its derived bound shapes
/// (`event_shapes.bounds`, keyed unit -> declaration simple name) or the
/// declared-type census of one of the unit's modules (its own namespaced
/// copy, the duplicated-contract idiom).
pub fn unit_holds_type_by_simple_name(model: &Model, unit: &str, name: &str) -> bool {
    model
        .event_shapes
        .bounds
        .get(unit)
        .is_some_and(|bounds| bounds.contains_key(name))
        || unit_module_census_holds(model, unit, &model.module_public_types, |types| {
            types.iter().any(|declared| declared == name)
        })
}

/// The unit-scoped half of the simple-name pairing: True when some module
/// `unit` owns carries a census entry satisfying `holds`. The scoping rule
/// scenario 06 spelled inline and scenario 10 extracted — ONE implementation
/// serving every module-keyed census (the identity arms' declared-type
/// census, the drift pairing's record-shape census), so no caller re-mirrors
/// the loop (a second hand-written copy is a scenario-10 done-when failure).
pub fn unit_module_census_holds<T>(
    model: &Model,
    unit: &str,
    census: &BTreeMap<String, T>,
    holds: impl Fn(&T) -> bool,
) -> bool {
    model.soft_structure.get(unit).is_some_and(|modules| {
        modules
            .iter()
            .any(|module| census.get(module).is_some_and(&holds))
    })
}

/// Resolve a unit's root namespace (roles reality US 01) — the module every
/// namespace-less ("sentinel") file fact attributes to and both role
/// predicates address. The ladder: (a) the csproj `RootNamespace` property,
/// else `AssemblyName` (MSBuild's own default chain); (b) the longest common
/// dotted prefix of the namespaces the unit's files declare, TRUNCATED to the
/// unit-name segment count — a project whose files all sit under
/// `A.B.C.Extras` roots at its own name `A.B.C`, not at a sub-module and not
/// at the shared parent `A::B` every sibling would collide on; (c) the full
/// unit name. ≤2-segment unit names without properties keep the first-two-
/// segments era's bytes exactly — the ladder's new voices only show on
/// corporate-depth names and property-bearing projects.
fn resolve_root_namespace(
    unit_name: &str,
    csproj_root: Option<&str>,
    namespaces: &BTreeSet<String>,
) -> String {
    if let Some(value) = csproj_root.map(str::trim).filter(|value| !value.is_empty()) {
        return value.to_string();
    }
    // The unit name's segment COUNT caps the truncation (empty segments a
    // dot-prefixed directory name yields are not name segments).
    let stem_segments = unit_name.split('.').filter(|segment| !segment.is_empty()).count();
    match common_namespace_prefix(namespaces) {
        Some(prefix) => prefix
            .split('.')
            .take(stem_segments.max(1))
            .collect::<Vec<_>>()
            .join("."),
        // No namespaces (top-level-statements-only projects) — or namespaces
        // sharing nothing, which behaves like none: the name fallback.
        None => unit_name.to_string(),
    }
}

/// The longest common dotted prefix of a set of namespaces, or `None` for an
/// empty set or namespaces sharing no first segment.
fn common_namespace_prefix(namespaces: &BTreeSet<String>) -> Option<String> {
    let mut namespaces = namespaces.iter().map(String::as_str);
    let first = namespaces.next()?;
    let mut prefix: Vec<&str> = first.split('.').collect();
    for namespace in namespaces {
        let shared = namespace
            .split('.')
            .zip(prefix.iter().copied())
            .take_while(|(left, right)| left == right)
            .count();
        prefix.truncate(shared);
        if prefix.is_empty() {
            break;
        }
    }
    (!prefix.is_empty()).then(|| prefix.join("."))
}

/// The csproj identity properties' text (US 01): the first literal
/// `<RootNamespace>` element, else the first `<AssemblyName>` — MSBuild's
/// default chain. Conditioned duplicates stay out of the ladder's way (first
/// literal wins, a documented residual); a read or parse failure states
/// `None` — the stem ladder covers what the csproj does not tell.
fn csproj_root_namespace(full_path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(full_path).ok()?;
    let mut reader = quick_xml::Reader::from_str(&raw);
    reader.config_mut().trim_text(true);
    let mut inside: Option<String> = None;
    let mut root_namespace: Option<String> = None;
    let mut assembly_name: Option<String> = None;
    loop {
        let event = reader.read_event();
        match event {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let qname = e.name();
                let name: &str = qname.as_ref();
                if name == "RootNamespace" || name == "AssemblyName" {
                    inside = Some(name.to_string());
                }
            }
            Ok(quick_xml::events::Event::Text(ref t)) => {
                if let Some(name) = inside.take() {
                    let raw: &str = t.as_ref();
                    let value = quick_xml::escape::unescape(raw)
                        .unwrap_or(std::borrow::Cow::Borrowed(raw))
                        .trim()
                        .to_string();
                    if value.is_empty() {
                        continue;
                    }
                    if name == "RootNamespace" {
                        root_namespace.get_or_insert(value);
                    } else {
                        assembly_name.get_or_insert(value);
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                let qname = e.name();
                let name: &str = qname.as_ref();
                if name == "RootNamespace" || name == "AssemblyName" {
                    inside = None;
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    root_namespace.or(assembly_name)
}

/// The csproj's entrypoint-gate evidence (S3): the `Sdk` attribute names a
/// host model (`Microsoft.NET.Sdk.Web` / `.Worker`) or a property group
/// states `OutputType=Exe` — .NET's own declaration that the unit RUNS,
/// whichever file carries its wiring. A read or parse failure states
/// `false`; absence of gate evidence is honest, the filename convention
/// still opens the gate separately.
fn csproj_entrypoint_gate(full_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(full_path) else {
        return false;
    };
    let mut reader = quick_xml::Reader::from_str(&raw);
    reader.config_mut().trim_text(true);
    let mut inside_output_type = false;
    let mut gate = false;
    loop {
        let event = reader.read_event();
        match event {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let qname = e.name();
                match qname.as_ref() {
                    "Project" => {
                        for attribute in e.attributes().flatten() {
                            if attribute.key.as_ref() != "Sdk" {
                                continue;
                            }
                            let value: &str = attribute.value.as_ref();
                            let host_model =
                                value.ends_with(".Web") || value.ends_with(".Worker");
                            if host_model {
                                gate = true;
                            }
                        }
                    }
                    "OutputType" => inside_output_type = true,
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Text(ref t)) => {
                if inside_output_type {
                    let raw: &str = t.as_ref();
                    if raw.trim().eq_ignore_ascii_case("exe") {
                        gate = true;
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                let qname = e.name();
                if qname.as_ref() == "OutputType" {
                    inside_output_type = false;
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    gate
}

/// The DI-registration call family behind the composition predicate (roles
/// US 02): the `IServiceCollection` registration methods expressing cross-layer
/// interface→implementation and hosted-service wiring — the shapes a
/// layered C# service tree states in its composition root
/// (`api/Shop.Api/Program.cs`). Matched as
/// case-sensitive C# method names in the call shape `.Name<…>` / `.Name(…)`;
/// framework `Add…` calls outside the family (AddControllers, AddLogging) and
/// hand-rolled extensions (AddRustSyncClient) are NOT evidence — a name-based
/// guess there would state a role the code does not prove. The EF and caching
/// members joined the family with S3: a composition root that wires through
/// them wires the application exactly as one calling the lifetime members.
const DI_REGISTRATION_FAMILY: [&str; 6] =
    ["AddScoped", "AddTransient", "AddSingleton", "AddHostedService", "AddDbContext", "AddMemoryCache"];

/// The entrypoint file convention: a `.cs` file named `Program.cs` or
/// `Startup.cs` (case-insensitive stem — the two shapes .NET uses for
/// composition wiring). The name IS the entrypoint marker; no manifest
/// OutputType fact participates.
fn is_entrypoint_source_file(path: &Path) -> bool {
    path.file_stem().is_some_and(|stem| {
        let stem = stem.to_string_lossy();
        stem.eq_ignore_ascii_case("Program") || stem.eq_ignore_ascii_case("Startup")
    })
}

/// True when the source of an entrypoint file contains a call of the
/// [`DI_REGISTRATION_FAMILY`]: a family name with `.` before it (skipping
/// whitespace — chains may wrap lines) and `<` or `(` after it. Matched on the
/// source with comments and string/char literals removed
/// ([`without_comments_or_literals`]), so a registration only named in a
/// comment or literal is no fact. An identifier merely ENDING in a family name
/// (`obj.AddScopedWithRetry(`) fails the trailing-shape test and a chain
/// segment without the leading `.` (`AddScoped(` as a local function call) is
/// not instance wiring — both state nothing.
fn has_di_registration_calls(source: &str) -> bool {
    let code = without_comments_or_literals(source);
    DI_REGISTRATION_FAMILY.iter().any(|name| {
        code.match_indices(name).any(|(index, _)| {
            let before = code[..index].trim_end();
            let after = code[index + name.len()..].trim_start();
            before.ends_with('.') && (after.starts_with('<') || after.starts_with('('))
        })
    })
}

/// True when the source carries an assembly-level `TypeForwardedTo`
/// attribute (S4): `TypeForwardedTo` in a call/attribute shape — preceded by
/// `:` (the `[assembly: …]` spelling, qualified or not) or by `.` (fully
/// qualified) and followed by `(`. Comments and literals are stripped first
/// (same guard as the registration family).
fn has_type_forwarded_to(source: &str) -> bool {
    let code = without_comments_or_literals(source);
    code.match_indices("TypeForwardedTo").any(|(index, _)| {
        let before = code[..index].trim_end();
        let after = code[index + "TypeForwardedTo".len()..].trim_start();
        (before.ends_with('.') || before.ends_with(':')) && after.starts_with('(')
    })
}

/// The source with line and block comments removed and string, verbatim and
/// char literals replaced by a single space (token separation kept), so call
/// recognition reads code only. Raw string literals are not tracked
/// separately: their opening quotes still delimit stripped content.
fn without_comments_or_literals(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut index = 0;
    let len = chars.len();
    while index < len {
        match chars[index] {
            '/' if chars.get(index + 1) == Some(&'/') => {
                while index < len && chars[index] != '\n' {
                    index += 1;
                }
            }
            '/' if chars.get(index + 1) == Some(&'*') => {
                index += 2;
                while index + 1 < len && !(chars[index] == '*' && chars[index + 1] == '/') {
                    index += 1;
                }
                index += 2;
                out.push(' ');
            }
            '"' => {
                index += 1;
                while index < len && chars[index] != '"' {
                    if chars[index] == '\\' {
                        index += 1;
                    }
                    index += 1;
                }
                index += 1;
                out.push(' ');
            }
            '\'' => {
                index += 1;
                while index < len && chars[index] != '\'' {
                    if chars[index] == '\\' {
                        index += 1;
                    }
                    index += 1;
                }
                index += 1;
                out.push(' ');
            }
            '@' if chars.get(index + 1) == Some(&'"') => {
                index += 2;
                while index < len {
                    if chars[index] == '"' {
                        if chars.get(index + 1) == Some(&'"') {
                            index += 2;
                        } else {
                            index += 1;
                            break;
                        }
                    } else {
                        index += 1;
                    }
                }
                out.push(' ');
            }
            other => {
                out.push(other);
                index += 1;
            }
        }
    }
    out
}

/// Derive one unit's role entries from its own facts (roles US 02), adding
/// model keys to the per-predicate sets. Both predicates address the unit's
/// root module key with disjoint evidence:
/// - `facade`: a root module that has root-tier facts, declares no types,
///   and whose facts re-export. C# states that shape in three evidence
///   shapes (S4): a using-only root file (the sentinel or under the root
///   namespace — including `global using` files), an `[assembly:
///   TypeForwardedTo]` forwarding file, or both. A root file that declares
///   a type — any wrapper/`static class Program` spelling — is NOT a
///   facade: BY DESIGN (rust parity, `root_defines_items`), stated not
///   accidental. A root module with no facts at all is not in the model and
///   gains nothing: absence states "no role", never "denied". The entrypoint
///   gate opens no facade exception: an entrypoint root that wires nothing
///   stays the conduit reading the laundering check sanctions (ADR-017).
/// - `composition`: the unit's entrypoint gate is open — a Program/Startup
///   file, a host `Sdk` (`.Web`/`.Worker`) or `OutputType=Exe` — AND the
///   [`DI_REGISTRATION_FAMILY`] is called somewhere in its production files
///   (S3: gate AND family, neither alone states the role). Registration
///   facts are a derivation input only — they join no edge map and engage no
///   rule yet.
///
/// The unit's composition predicate (gate AND family, S3 — computed by the
/// caller) claims composition; otherwise a using-only root states facade.
/// The unit claims at most one role, at its own resolved root: composition
/// beating facade is this unit's exclusivity, not a cross-unit override.
fn derive_unit_roles(
    root_ns: &str,
    usings: &BTreeMap<String, BTreeSet<String>>,
    type_facts: &Option<syntax::TypeFacts>,
    composition: bool,
    forwarded: bool,
    claims: &mut Vec<(String, Role)>,
) {
    let root_key = ns_to_module(root_ns);
    if composition {
        claims.push((root_key, Role::Composition));
        return;
    }
    let using_fact =
        usings.contains_key("") || usings.get(root_ns).is_some_and(|t| !t.is_empty());
    let type_fact = type_facts.as_ref().is_some_and(|facts| {
        facts
            .declared_types
            .get("")
            .is_some_and(|names| !names.is_empty())
            || facts
                .declared_types
                .get(root_ns)
                .is_some_and(|names| !names.is_empty())
    });
    if (using_fact || forwarded) && !type_fact {
        claims.push((root_key, Role::Facade));
    }
}

/// Merge the per-unit root claims into the model's roles map: the same role
/// claimed under one key states that role once, and a key two units claim
/// with DIFFERENT roles states nothing — the ambiguity guard. Composition
/// beating facade is an exclusivity WITHIN a unit (derive_unit_roles), never
/// a cross-unit override: one unit's claim must not swallow, silence or
/// outvote another's.
fn roles_map(claims: Vec<(String, Role)>) -> BTreeMap<String, Role> {
    let mut roles: BTreeMap<String, Role> = BTreeMap::new();
    let mut doubted: BTreeSet<String> = BTreeSet::new();
    for (key, role) in claims {
        if doubted.contains(&key) {
            continue;
        }
        match roles.get(&key) {
            Some(existing) if *existing == role => {}
            Some(_) => {
                roles.remove(&key);
                doubted.insert(key);
            }
            None => {
                roles.insert(key, role);
            }
        }
    }
    roles
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
/// The same path as the csproj shape: identical source facts (grammar), the
/// same prefix resolution, and the same [`edge_target`] addressing — only the
/// graph is trivial (one unit, no references, empty external tier), so no
/// attribution branches are specialized here. The declared-exclusion file
/// gate (GitHub issue #60) rides the same walk, and its census lines render
/// like the csproj shape's: the root unit is one kept project.
fn extract_single_unit(root: &Path, facts: &DeclaredFacts) -> Result<Model, String> {
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

    let mut excluded_file_lines: BTreeSet<String> = BTreeSet::new();
    let (namespaces, usings, type_facts, _shape_facts, entrypoint_file, registration_family, forwarded) =
        scan_project_sources(&canonical, facts, &canonical, Some(&mut excluded_file_lines))?;
    let mut soft_structure: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut sorted: Vec<String> = namespaces.iter().map(|ns| ns_to_module(ns)).collect();
    let unit_csproj = std::fs::read_dir(root).ok().and_then(|entries| {
        entries.flatten().find(|entry| {
            entry.path().extension().and_then(|ext| ext.to_str()) == Some("csproj")
        })
    });
    let csproj_root = unit_csproj
        .as_ref()
        .and_then(|entry| csproj_root_namespace(&entry.path()));
    let csproj_gate = unit_csproj
        .as_ref()
        .is_some_and(|entry| csproj_entrypoint_gate(&entry.path()));
    let root_key = resolve_root_namespace(&name, csproj_root.as_deref(), &namespaces);
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
            match resolve_owner_prefix(target, &namespace_unit) {
                Some((prefix, _owner)) => {
                    let from = from_module(from_ns);
                    let (to, symbols) = edge_target(prefix, target);
                    if from != to {
                        module_edge_map
                            .entry((from, to))
                            .or_default()
                            .extend(symbols);
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
    // Type-position references (US 08) join the same
    // single-unit edge map: qualified references plus bare candidates whose
    // ONE real resolution is a type declared under a declared namespace the
    // file can see. An ownership miss records nothing — the external tier of
    // a csproj-less tree is empty by construction either way.
    if let Some(facts) = &type_facts {
        let mut references: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (from_ns, qualified) in &facts.qualified {
            references
                .entry(from_ns.clone())
                .or_default()
                .extend(qualified.iter().cloned());
        }
        for (from_ns, candidates) in &facts.candidates {
            let mut by_ident: BTreeMap<&str, BTreeSet<&String>> = BTreeMap::new();
            for (ident, reference) in candidates {
                by_ident.entry(ident.as_str()).or_default().insert(reference);
            }
            for (ident, cands) in by_ident {
                let mut survivors = cands
                    .into_iter()
                    .filter(|reference| {
                        candidate_resolves_as_declared_type(
                            reference,
                            ident,
                            &namespace_unit,
                            &facts.declared_types,
                        )
                    });
                let Some(reference) = survivors.next().cloned() else {
                    continue;
                };
                if survivors.next().is_some() {
                    continue;
                }
                references.entry(from_ns.clone()).or_default().insert(reference);
            }
        }
        for (from_ns, references) in references {
            let from = from_module(&from_ns);
            for reference in &references {
                if let Some((prefix, _owner)) = resolve_owner_prefix(reference, &namespace_unit) {
                    let (to, symbols) = edge_target(prefix, reference);
                    if from != to {
                        module_edge_map
                            .entry((from.clone(), to))
                            .or_default()
                            .extend(symbols);
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
                type_only: false,
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

    // The csproj-less shape derives roles from the same predicates as the
    // csproj path (roles US 02) — one unit claiming at its resolved root.
    let mut role_claims: Vec<(String, Role)> = Vec::new();
    let gate_open = entrypoint_file || csproj_gate;
    derive_unit_roles(
        &root_key,
        &usings,
        &type_facts,
        registration_family && gate_open,
        forwarded,
        &mut role_claims,
    );
    let roles = roles_map(role_claims);

    // Public-API facts for the csproj-less shape, attributed exactly like the
    // csproj path (US 01): namespaces to their module paths, global
    // declarations to the unit root key.
    let mut public_types_sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(facts) = &type_facts {
        for (ns, names) in &facts.public_types {
            let module = if ns.is_empty() {
                root_key.clone()
            } else {
                ns_to_module(ns)
            };
            public_types_sets
                .entry(module)
                .or_default()
                .extend(names.iter().cloned());
        }
    }
    let module_public_types: BTreeMap<String, Vec<String>> = public_types_sets
        .into_iter()
        .map(|(module, types)| (module, types.into_iter().collect()))
        .collect();

    Ok(Model {
        units: vec![unit],
        soft_structure,
        module_edges,
        module_public_types,
        module_external: module_external_model,
        roles,
        // File-level declared-exclusion census, sorted by path (issue #60):
        // report-only bytes, serde-skipped, never the machine body.
        report_only_lines: excluded_file_lines.into_iter().collect(),
        ..Model::new("csharp")
    })
}

/// The driver-port seam entry (workplan events_value_join scenario 10):
/// a unit struct implementing `crate::driver::Driver` by calling this
/// module's own [`extract`] — body logic moves zero lines.
pub(crate) struct CsharpDriver;

impl crate::driver::Driver for CsharpDriver {
    fn extract(&self, root: &Path, declared: &DeclaredFacts) -> Result<Model, String> {
        extract(root, declared)
    }
}
