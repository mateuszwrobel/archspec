//! Unit-to-component mapping and the effective module-tier view:
//! assign extracted units onto declared components, fold module-path
//! matches into the mapping, and derive the module tier the checks read.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use super::pattern::{matches_pattern, module_path_matches};
use super::claims::flatten_claims;
use crate::archspec::language;

use crate::archspec::model::{Model, ModuleEdge, Unit};
use crate::archspec::spec::{MatchSet, Module};

/// Result of mapping extracted units onto declared components.
#[derive(Debug, Clone)]
pub struct Mapping {
    /// Declared component name -> unit names assigned to it.
    pub assignments: BTreeMap<String, BTreeSet<String>>,
    /// Extracted unit name -> declared component it was assigned to (assigned
    /// units only; unmatched units are classified below).
    pub unit_module: BTreeMap<String, String>,
    /// Extracted units matched by no declared component, treated as
    /// component-level additions (`unexpected component: X`).
    pub unexpected_components: Vec<String>,
    /// Extracted units matched by no declared component but nested under a
    /// declared component's namespace (`unassigned unit: X`).
    pub unassigned_units: Vec<String>,
    /// Declared boundaries matched by at least one extracted module path
    /// (soft tier). A boundary present only via `matches.modules` is not
    /// reported as a missing component.
    pub module_matched: BTreeSet<String>,
}

/// Assign each extracted unit to the first declared component whose
/// `matches.units` glob matches (declaration order wins). Units matching no
/// component are never dropped (spec.md) and split into two categories:
///
/// - **unexpected component** (added component): the unit is a component-level
///   hard unit whose name does not nest under any declared component. In
///   phase-1 all extracted units are hard units (crate/project/package), so a
///   flat unmatched unit is the code having grown a component the spec does not
///   declare.
/// - **unassigned unit**: the unit's parent namespace (last `/` or `.` segment
///   stripped, e.g. `example.com/demo/auth/store` -> `example.com/demo/auth`)
///   matches a declared component. The unit is a sub-unit of that component
///   that no declared `matches.units` glob covers. Phase-1 scanners emit flat
///   hard units, so this fires when a hard unit's *name* implies nesting under
///   a declared namespace (Go package trees, dotted C# project stems).
pub fn map_units(units: &[Unit], modules: &[Module]) -> Mapping {
    let mut assignments: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut unit_module: BTreeMap<String, String> = BTreeMap::new();
    let mut unexpected_components: Vec<String> = Vec::new();
    let mut unassigned_units: Vec<String> = Vec::new();

    let mut units_sorted: Vec<&Unit> = units.iter().collect();
    units_sorted.sort_by(|left, right| left.name.cmp(&right.name));

    for unit in units_sorted {
        let matched = modules.iter().find(|module| {
            module
                .matches
                .units
                .iter()
                .any(|pattern| matches_pattern(&unit.name, pattern))
        });
        match matched {
            Some(module) => {
                assignments
                    .entry(module.name.clone())
                    .or_default()
                    .insert(unit.name.clone());
                unit_module.insert(unit.name.clone(), module.name.clone());
            }
            None => {
                if nested_under_declared_module(&unit.name, modules) {
                    unassigned_units.push(unit.name.clone());
                } else {
                    unexpected_components.push(unit.name.clone());
                }
            }
        }
    }

    Mapping {
        assignments,
        unit_module,
        unexpected_components,
        unassigned_units,
        module_matched: BTreeSet::new(),
    }
}

/// True if any ancestor namespace of the unit name matches a declared module.
/// Walks every prefix obtained by stripping trailing `/` or `.` segments, so a
/// sub-unit nested arbitrarily deep under a declared component is classified as
/// unassigned, not as an added component.
fn nested_under_declared_module(name: &str, modules: &[Module]) -> bool {
    let mut current = name;
    while let Some(parent) = strip_last_segment(current) {
        if namespace_matches_module(parent, modules) {
            return true;
        }
        current = parent;
    }
    false
}

fn strip_last_segment(name: &str) -> Option<&str> {
    for separator in ['/', '.'] {
        if let Some((parent, _)) = name.rsplit_once(separator)
            && !parent.is_empty() {
                return Some(parent);
            }
    }
    None
}

/// True if a parent namespace matches a declared component by the same
/// `matches.units` glob rules used for units.
fn namespace_matches_module(namespace: &str, modules: &[Module]) -> bool {
    modules.iter().any(|module| {
        module
            .matches
            .units
            .iter()
            .any(|pattern| matches_pattern(namespace, pattern))
    })
}

/// The resolved component set: distinct assigned declared boundaries plus any
/// boundary matched by a module path but owning no unit. Mirrors the extracted
/// set `compare` uses to detect missing components.
pub fn resolved_components(mapping: &Mapping) -> BTreeSet<String> {
    mapping
        .assignments
        .keys()
        .cloned()
        .chain(mapping.module_matched.iter().cloned())
        .collect()
}

/// A test-gated module (declared `#[cfg(test)]` or an equivalent test-only
/// `cfg(all(..., test, ...))`) and its descendants are scaffolding: it is not a
/// real boundary (update folds it into its top-level boundary) and a test module
/// depending on production modules is normal, not an architecture violation.
/// Resolution is by the model's proven test-gated set (`Model::is_test_gated`),
/// NOT by the path *name* — a module merely named `tests` without a test cfg is
/// production and must participate in the dependency graph and cycles.
pub(super) fn is_test_module(model: &Model, module_path: &str) -> bool {
    model.is_test_gated(module_path)
}

/// Edge counts taken directly from the model's scan tiers, with a dependency
/// stated in both tiers counted exactly once: internal is the number of
/// DISTINCT internal dependency pairs at unit tier — the production unit→unit
/// edges, set-unioned with every module edge projected onto its endpoint
/// units (self-pairs dropped; a module-tier pair with no unit-tier twin adds
/// exactly one pair). External is the unit→external tier. Module-level
/// external uses remain in their own tier and are not folded into the external
/// edge count. Returns `(internal, external)`.
/// A test-gated module (declared `#[cfg(test)]` or an equivalent test-only
/// `cfg(all(..., test, ...))`) and its descendants are scaffolding: it is not a
/// real boundary (update folds it into its top-level boundary) and a test module
/// depending on production modules is normal, not an architecture violation.
/// Resolution is by the model's proven test-gated set (`Model::is_test_gated`),
/// NOT by the path *name* — a module merely named `tests` without a test cfg is
/// production and must participate in the dependency graph and cycles.
pub fn count_model_edges(model: &Model) -> (usize, usize) {
    let mut pairs: BTreeSet<(String, String)> = model
        .edges
        .iter()
        .filter(|edge| edge.from != edge.to)
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect();
    for edge in &model.module_edges {
        if is_test_module(model, &edge.from) || is_test_module(model, &edge.to) {
            continue;
        }
        // `unit` is contractually the owner of `from`, so it is the fallback
        // there; an unowned `to` keeps the module path as its identity, so a
        // dependency no unit claims still counts once instead of vanishing or
        // doubling a stated pair.
        let from_unit = owning_unit(model, &edge.from).unwrap_or_else(|| edge.unit.clone());
        let to_unit = owning_unit(model, &edge.to).unwrap_or_else(|| edge.to.clone());
        if from_unit != to_unit {
            pairs.insert((from_unit, to_unit));
        }
    }
    (pairs.len(), model.external.len())
}

/// The unit whose soft tier contains `module_path`: the unit owning the longest
/// soft path equal to the path or a `::`-prefix of it, else — when the model
/// carries no soft tier covering the path (a single-module go tree) — the
/// longest unit name that prefixes the path under any tier separator (`::`,
/// `.` or `/`: the separators the drivers address units with, membership-keyed
/// like the `depgraph` projections). `None` when no unit claims the path; the
/// caller substitutes its contract fallback or the path's own identity.
pub(super) fn owning_unit(model: &Model, module_path: &str) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for (unit, paths) in &model.soft_structure {
        for path in paths {
            let covers = module_path == path || module_path.starts_with(&format!("{path}::"));
            if covers && best.as_ref().is_none_or(|(len, _)| path.len() > *len) {
                best = Some((path.len(), unit.clone()));
            }
        }
    }
    if let Some((_, unit)) = best {
        return Some(unit);
    }
    let dotted = module_path.replace("::", ".");
    let slashed = module_path.replace("::", "/");
    for unit in &model.units {
        let name = &unit.name;
        let hit = module_path.starts_with(&format!("{name}::"))
            || dotted == *name
            || dotted.starts_with(&format!("{name}."))
            || slashed == *name
            || slashed.starts_with(&format!("{name}/"));
        if hit && best.as_ref().is_none_or(|(len, _)| name.len() > *len) {
            best = Some((name.len(), unit.name.clone()));
        }
    }
    best.map(|(_, unit)| unit)
}

/// Augment the unit-tier `Mapping` with the soft (module) tier. A boundary is
/// "present" if it matched a unit OR a module path; a unit is accounted for if
/// its name matched a boundary's `matches.units` OR one of its module paths
/// matched a boundary's `matches.modules`. Units accounted for only via the
/// module tier are re-classified out of `unexpected`/`unassigned` and assigned
/// to the first matching boundary (declaration order), so their edges still
/// resolve. Boundaries matched by at least one module path anywhere are
/// recorded in `module_matched` so they are not reported as missing.
pub fn augment_with_modules(mapping: &Mapping, model: &Model, modules: &[Module]) -> Mapping {
    let mut assignments = mapping.assignments.clone();
    let mut unit_module = mapping.unit_module.clone();
    let mut unexpected_components = Vec::new();
    let mut unassigned_units = Vec::new();
    let mut module_matched: BTreeSet<String> = BTreeSet::new();

    // Attribution reads claims at every depth (D1): on go a unit IS an
    // addressable module-tier path (its import path), so a `matches.modules`
    // pattern claims it directly — in a single-module tree (no scan tier to
    // look paths up in) and in a go.work tree alike, whether the pattern sits
    // on a top-level module or a nested submodule.
    let go_unit_paths = model.language == "go";
    let go_unit_claims = |matches: &MatchSet, unit: &str| {
        go_unit_paths
            && matches
                .modules
                .iter()
                .any(|pattern| module_path_matches(pattern, unit))
    };
    let claims = flatten_claims(modules);

    for claim in &claims {
        let matched = model.soft_structure.values().flatten().any(|path| {
            claim
                .matches
                .modules
                .iter()
                .any(|pattern| module_path_matches(pattern, path))
        }) || model
            .units
            .iter()
            .any(|unit| go_unit_claims(claim.matches, &unit.name));
        if matched {
            module_matched.insert(claim.name.to_string());
        }
    }

    for unit in mapping
        .unexpected_components
        .iter()
        .chain(mapping.unassigned_units.iter())
    {
        let paths: Vec<&String> = model
            .soft_structure
            .get(unit)
            .into_iter()
            .flatten()
            .collect();
        let owner = claims.iter().find(|claim| {
            claim
                .matches
                .modules
                .iter()
                .any(|pattern| paths.iter().any(|path| module_path_matches(pattern, path)))
                || go_unit_claims(claim.matches, unit)
        });
        match owner {
            Some(owner) => {
                assignments
                    .entry(owner.name.to_string())
                    .or_default()
                    .insert(unit.clone());
                unit_module.insert(unit.clone(), owner.name.to_string());
            }
            None => {
                if mapping.unassigned_units.contains(unit) {
                    unassigned_units.push(unit.clone());
                } else {
                    unexpected_components.push(unit.clone());
                }
            }
        }
    }
    unexpected_components.sort();
    unassigned_units.sort();

    Mapping {
        assignments,
        unit_module,
        unexpected_components,
        unassigned_units,
        module_matched,
    }
}

/// The file-grained dependency rows as the drivers serialize them — the
/// FACTS the capability row certifies (B5 law: adjudication and vacuity key
/// on facts, never on the row's label). The ts driver keys its `module_edges`
/// rows by file identity (ts-driver design D10); the rust driver serializes
/// its file-grained rows in `file_edges` and keeps `module_edges`
/// module-granular dotted paths. A driver whose row reads not-emitted has no
/// arm here, and callers gate on the capability row first; the weld test in
/// the suite reddens if a granular row ever lacks a projection arm — a
/// granular language can never silently project nothing.
pub fn file_grained_rows(model: &Model) -> Vec<(&str, &str)> {
    match language::from_name(&model.language) {
        Some(language::Language::Ts) => model
            .module_edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect(),
        // The python driver keys its `module_edges` rows by file identity
        // exactly like ts (python driver record P2), so the same arm shape
        // projects; a namespace-dir TARGET row is module-grain and
        // participates as the directory path, the same way the ts arm
        // projects whatever the driver serialized.
        Some(language::Language::Python) => model
            .module_edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect(),
        Some(language::Language::Rust) => model
            .file_edges
            .iter()
            // Every row kind renders: `mod` rows ARE the file tree's
            // structure on the diagram surface. The cycle adjudication
            // excludes them separately — composition is not a dependency
            // (see the mod-row law in `file_tier_report_cycles`).
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect(),
        // The kotlin driver keys `file_edges` by file identity and keeps
        // `module_edges` gradle-unit-granular (kotlin driver record
        // K-P2/K-P3), the rust serialization shape without `mod` rows —
        // every kotlin row is an import-use fact.
        Some(language::Language::Kotlin) => model
            .file_edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect(),
        _ => Vec::new(),
    }
}

/// The effective module tier a go tree presents to the boundary checks.
/// A go.work tree already carries the native tier; the scan dotted its import
/// paths for rendering, and compare addresses them as plain import paths, so
/// the paths are projected back with `/` separators (go paths cannot contain
/// `:`; the projection is exact). A single go.mod tree carries no tier at
/// all: when the spec declares modules that claim its packages, the
/// declarations become the tier (the declared-grouping model) — membership is
/// the resolved unit→boundary assignment (`matches.units` globs plus units
/// reclaimed through `matches.modules` paths) and the module edges are the
/// real unit edges crossing groups, the same shape the native tier derives
/// from workspace members. The derivation is visible by construction: scan
/// output stays tier-free for a single go.mod tree, a native tier is never
/// overridden, and the derivation lives and dies with the compare that
/// derived it. Rust and csharp models never reach this projection.
pub fn effective_module_tier<'a>(model: &'a Model, mapping: &Mapping) -> Cow<'a, Model> {
    if model.language != "go" {
        return Cow::Borrowed(model);
    }
    let mut derived = model.clone();
    if model.has_module_tier() {
        derived.soft_structure = model
            .soft_structure
            .iter()
            .map(|(module, paths)| {
                (
                    module.replace("::", "/"),
                    paths.iter().map(|path| path.replace("::", "/")).collect(),
                )
            })
            .collect();
        derived.module_edges = model
            .module_edges
            .iter()
            .map(|edge| ModuleEdge {
                type_only: false,
                unit: edge.unit.replace("::", "/"),
                from: edge.from.replace("::", "/"),
                to: edge.to.replace("::", "/"),
                symbols: edge.symbols.clone(),
            })
            .collect();
        return Cow::Owned(derived);
    }
    if mapping.assignments.is_empty() {
        return Cow::Borrowed(model);
    }
    derived.soft_structure = mapping
        .assignments
        .iter()
        .filter(|(_, units)| !units.is_empty())
        .map(|(module, units)| {
            let mut paths: Vec<String> = units.iter().cloned().collect();
            paths.sort();
            (module.clone(), paths)
        })
        .collect();
    let mut projected: BTreeSet<(String, String, String)> = BTreeSet::new();
    for edge in &model.edges {
        let (Some(from_owner), Some(to_owner)) = (
            mapping.unit_module.get(&edge.from),
            mapping.unit_module.get(&edge.to),
        ) else {
            continue;
        };
        if from_owner != to_owner {
            projected.insert((from_owner.clone(), edge.from.clone(), edge.to.clone()));
        }
    }
    derived.module_edges = projected
        .into_iter()
        .map(|(unit, from, to)| ModuleEdge {
            type_only: false,
            unit,
            from,
            to,
            symbols: Vec::new(),
        })
        .collect();
    Cow::Owned(derived)
}