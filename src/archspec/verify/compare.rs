// Core structural compare machinery shared by `verify`, and later `report` and
// `update`. Pipeline (design §4): map extracted units onto declared components,
// compare the extracted vs declared component sets and edge sets, render the
// verdict. Diff categories are grouped in canonical report order (output.md).

use crate::archspec::capability;
use crate::archspec::language;
use crate::archspec::model::{ManifestInfo, Model, ModuleEdge, Role, Unit};
use crate::archspec::spec::{Constraint, Module, Stereotype};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

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

/// Every difference across all checks, in canonical report order (ADR-010,
/// output.md). Each field is a sorted list of the rendered item bodies.
#[derive(Debug, Clone, Default)]
pub struct Diff {
    /// Declared in the spec, absent from the source model.
    pub missing_components: Vec<String>,
    /// Present in the source model, not declared in the spec.
    pub unexpected_components: Vec<String>,
    /// Extracted unit matched by no declared component.
    pub unassigned_units: Vec<String>,
    /// Soft module path claimed by two declared boundaries with equal
    /// specificity (prefix grouping overlap); ownership is reported, not
    /// guessed. Each entry names the path and both claiming entries.
    pub ambiguous_module_matches: Vec<String>,
    /// Edge present in the source but banned by `allowed.forbidden`.
    pub forbidden_edges: Vec<String>,
    /// Edge declared by `allowed.depend_on`, absent from the source.
    pub missing_edges: Vec<String>,
    /// Edge crossing components without being in `allowed.depend_on`.
    pub disallowed_cross_component: Vec<String>,
    /// Structural publication-only root facade violation: an internal module
    /// depends on its own crate-root unit whose root file defines nothing (only
    /// `mod` declarations + re-exports). Such an edge launders any ban through
    /// the umbrella, so it fails regardless of `depend_on` (the fix is to
    /// canonicalize the import, not to declare the edge).
    pub facade_dependencies: Vec<String>,
    /// Forbidden stereotype exposed by a component's own units.
    pub contract_leaks: Vec<String>,
    /// `no_cycles` constraint violated over the declared groups.
    pub cycles: Vec<String>,
    /// `public_api_allowlist` violation: a public export not in `allowed`.
    pub public_api_leaks: Vec<String>,
    /// `public_api_allowlist` violation: a glob re-export at a unit root whose
    /// exported set cannot be resolved from source (external crate, unknown
    /// path, cfg-blocked declaration or chain link, poisoned chain), so it is
    /// reported as unverifiable rather than silently skipped. Globs that do
    /// resolve contribute their enumerated names to `public_api_leaks`.
    pub unverifiable_glob_exports: Vec<String>,
    /// `public_api_allowlist` violation: a glob re-export at a unit root that
    /// resolves to a same-crate module exporting zero public items. An empty
    /// derived set would silently drop a checked surface, so it fails closed.
    pub empty_glob_exports: Vec<String>,
    /// `forbid_external_crates` violation: a listed module imports a banned crate.
    pub forbidden_external_crates: Vec<String>,
    /// `external_free` violation: a matched module/unit is attributed external
    /// packages — the zero-dependency purity guard fired.
    pub not_external_free: Vec<String>,
    /// `manifest_integrity` violation: manifest diverges from declared requirements.
    pub manifest_integrity: Vec<String>,
    /// `feature_boundary` violation: unallowed dep on a gated module, a gated
    /// module missing its cfg, or an unknown gated-module pattern.
    pub feature_boundaries: Vec<String>,
    /// `forbid_submodule_dependency` violation: a listed submodule depends on a
    /// forbidden sibling within a parent.
    pub forbidden_submodule_dependencies: Vec<String>,
    /// Warning-level items (design §9): divergences whose constraint declared
    /// `severity = "warning"`. Currently only `no_cycles` carries a severity.
    /// Each entry is already fully rendered as `<category>: <detail>` (e.g.
    /// `cycle: A -> B -> A`); the report prefixes it with `warning: ` unless
    /// `--strict` promotes it to an error line. Listed after all error items.
    pub warnings: Vec<String>,
    /// Vacuous-guard findings (plan_model_driven_workflow.md, Feature 2): a
    /// constraint whose effective domain is empty — no module/edge/manifest/
    /// export to check — reported so a silent pass is never mistaken for a real
    /// check. Always warning-severity regardless of the constraint's declared
    /// severity. Each entry is pre-rendered as `[constraint #N] <type>: <detail>`
    /// (no `vacuous constraint:` prefix); the report adds that prefix, plus a
    /// `warning: ` prefix unless `--strict` promotes it. Listed last.
    pub vacuous_constraints: Vec<String>,
}

impl Diff {
    /// True if no error-level divergence and no warning-level divergence.
    pub fn is_empty(&self) -> bool {
        !self.has_errors() && self.warnings.is_empty() && self.vacuous_constraints.is_empty()
    }

    /// Run verdict: an error-level divergence always fails; a warning-level
    /// divergence (including a vacuous constraint) fails only under `--strict`
    /// (which promotes it to an error).
    pub fn fails(&self, strict: bool) -> bool {
        self.has_errors()
            || (strict && (!self.warnings.is_empty() || !self.vacuous_constraints.is_empty()))
    }

    /// True if any error-level item is present (severity defaults to error;
    /// every category except `no_cycles` is error-level by declaration).
    fn has_errors(&self) -> bool {
        !self.missing_components.is_empty()
            || !self.unexpected_components.is_empty()
            || !self.unassigned_units.is_empty()
            || !self.ambiguous_module_matches.is_empty()
            || !self.forbidden_edges.is_empty()
            || !self.missing_edges.is_empty()
            || !self.disallowed_cross_component.is_empty()
            || !self.contract_leaks.is_empty()
            || !self.cycles.is_empty()
            || !self.public_api_leaks.is_empty()
            || !self.unverifiable_glob_exports.is_empty()
            || !self.empty_glob_exports.is_empty()
            || !self.forbidden_external_crates.is_empty()
            || !self.not_external_free.is_empty()
            || !self.manifest_integrity.is_empty()
            || !self.feature_boundaries.is_empty()
            || !self.forbidden_submodule_dependencies.is_empty()
            || !self.facade_dependencies.is_empty()
    }
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
        if let Some((parent, _)) = name.rsplit_once(separator) {
            if !parent.is_empty() {
                return Some(parent);
            }
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

/// Match a `matches.modules` pattern against a dotted module path. Accepts a
/// pattern addressing the full path, its bare last segment, or the path with
/// its unit prefix stripped (so `auth::config` matches `app::auth::config`).
/// Also treats the pattern as addressing a subtree: it matches the named module
/// and every module path beneath it (any ancestor), so `commands` covers
/// `commands::start`, `commands::init`, &c. Ancestor checks apply to both the
/// full path and its unit-stripped form.
fn module_path_matches(pattern: &str, module_path: &str) -> bool {
    if module_matches(pattern, module_path) {
        return true;
    }
    let stripped = strip_unit(module_path);
    if stripped != module_path && module_matches(pattern, stripped) {
        return true;
    }
    let mut current = module_path;
    while let Some((rest, _)) = current.rsplit_once("::") {
        current = rest;
        if module_matches(pattern, current) {
            return true;
        }
        let ancestor_stripped = strip_unit(current);
        if ancestor_stripped != current && module_matches(pattern, ancestor_stripped) {
            return true;
        }
    }
    false
}

/// The boundary-level dependency graph extracted from the model.
struct BoundaryGraph {
    /// Deduplicated cross-boundary pairs from both tiers.
    pairs: BTreeSet<(String, String)>,
    /// Pairs whose ownership resolved through the unit fallback: hops across
    /// territory no boundary declares (unit wiring, unclaimed modules).
    fallback_pairs: BTreeSet<(String, String)>,
    /// Soft-edge endpoints claimed by no boundary at all.
    unowned_endpoints: BTreeSet<String>,
}

/// The declared boundary that owns the given dotted module path (see the
/// specificity rules on `entry_specificity`), plus whether the winning claim
/// came from the unit-fallback step (the module was claimed by no
/// `matches.modules` entry and only rolls up through its unit). Fallback-owned
/// hops route through territory no boundary deliberately declares — the unit
/// wiring module or an unclaimed module — unlike a module boundary reached by
/// an explicit pattern.
fn module_owner_with_fallback<'a>(
    module_path: &str,
    modules: &'a [Module],
) -> Option<(&'a str, bool)> {
    let mut best: Option<(usize, &'a str)> = None;
    for module in modules {
        if let Some(score) = module
            .matches
            .modules
            .iter()
            .filter_map(|pattern| entry_specificity(pattern, module_path))
            .max()
        {
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, module.name.as_str()));
            }
        }
    }
    if let Some((_, owner)) = best {
        return Some((owner, false));
    }
    let unit = module_path.split("::").next().unwrap_or(module_path);
    modules.iter().find_map(|module| {
        module
            .matches
            .units
            .iter()
            .any(|pattern| matches_pattern(unit, pattern))
            .then_some((module.name.as_str(), true))
    })
}

/// Composition-role PATHS of the roles map. The laundering exemption is
/// stated at HOP level on these paths: a hop is sanctioned wiring exactly
/// when its source path is a composition key — the composition unit wiring
/// out. Which boundary (if any) claims the path's unit is irrelevant:
/// boundary-level whitelisting was the laundering hole (a banned route
/// through ANY module of a boundary that happened to own a composition path
/// escaped — a planted bin main plus a catch-all claiming the bin's unit
/// laundered the whole boundary), and the stage-1/2/3 attribution ladder it
/// needed misattributed multi-segment paths to the neighbouring unit.
fn composition_sources(model: &Model) -> BTreeSet<String> {
    model
        .roles
        .iter()
        .filter(|(_, role)| matches!(role, Role::Composition))
        .map(|(path, _)| path.clone())
        .collect()
}
/// Specificity of one `matches.modules` entry against a dotted module path:
/// how much of the path the entry pins, in characters. `None` when the entry
/// does not match at all (same accept set as `module_path_matches`). An entry
/// naming the module exactly pins the whole path and always wins; otherwise the
/// longest matching prefix wins, so `commands::start*` claims `start` over
/// `commands::**`'s broader subtree claim only when it pins more.
fn entry_specificity(pattern: &str, module_path: &str) -> Option<usize> {
    if pattern == module_path {
        return Some(usize::MAX);
    }
    let mut best: Option<usize> = None;
    let mut consider = |score: Option<usize>| {
        if let Some(score) = score {
            best = Some(best.map_or(score, |b| b.max(score)));
        }
    };
    consider(pinned_prefix(pattern, module_path));
    let stripped = strip_unit(module_path);
    if stripped != module_path {
        consider(
            pinned_prefix(pattern, stripped)
                .map(|p| module_path.chars().count() - stripped.chars().count() + p),
        );
    }
    let mut current = module_path;
    while let Some((rest, _)) = current.rsplit_once("::") {
        current = rest;
        consider(pinned_prefix(pattern, current).map(|p| p.min(current.chars().count())));
        let ancestor_stripped = strip_unit(current);
        if ancestor_stripped != current {
            consider(
                pinned_prefix(pattern, ancestor_stripped)
                    .map(|p| current.chars().count() - ancestor_stripped.chars().count() + p),
            );
        }
    }
    best
}

/// How many leading characters of `target` an entry pins: the whole target for
/// an exact or full-glob match (including a bare-name match against the last
/// segment), the literal prefix before a trailing `*` for a prefix pattern
/// (which may also pin through the bare last segment).
fn pinned_prefix(pattern: &str, target: &str) -> Option<usize> {
    let target_len = target.chars().count();
    if pattern.ends_with('*') && !pattern[..pattern.len() - 1].contains('*') {
        let literal = &pattern[..pattern.len() - 1];
        if target.starts_with(literal) {
            return Some(literal.chars().count());
        }
        if target
            .rsplit("::")
            .next()
            .is_some_and(|bare| bare.starts_with(literal))
        {
            return Some(target_len);
        }
        return None;
    }
    if pattern == target || matches_pattern(target, pattern) {
        return Some(target_len);
    }
    if target
        .rsplit("::")
        .next()
        .is_some_and(|bare| matches_pattern(bare, pattern))
    {
        return Some(target_len);
    }
    None
}

/// Soft module paths claimed by two or more declared boundaries with equal
/// specificity. Prefix entries make overlaps real (`commands::**` and
/// `**::start` can claim the same module), so the conflict is reported against
/// the model rather than resolved by luck; the message names the module path
/// and both claiming entries.
fn check_ambiguous_module_matches(model: &Model, modules: &[Module]) -> Vec<String> {
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    for module_paths in model.soft_structure.values() {
        paths.extend(module_paths.iter().map(String::as_str));
    }
    for edge in &model.module_edges {
        paths.insert(edge.from.as_str());
        paths.insert(edge.to.as_str());
    }
    let mut found = BTreeSet::new();
    for path in paths {
        let claims: Vec<(usize, &str, &str)> = modules
            .iter()
            .filter_map(|module| {
                module
                    .matches
                    .modules
                    .iter()
                    .filter_map(|pattern| {
                        entry_specificity(pattern, path).map(|score| (score, pattern.as_str()))
                    })
                    .max_by_key(|(score, _)| *score)
                    .map(|(score, entry)| (score, module.name.as_str(), entry))
            })
            .collect();
        let Some(top) = claims.iter().map(|(score, _, _)| *score).max() else {
            continue;
        };
        let tied: Vec<&(usize, &str, &str)> = claims
            .iter()
            .filter(|(score, _, _)| *score == top)
            .collect();
        for (i, (_, name_a, entry_a)) in tied.iter().enumerate() {
            for (_, name_b, entry_b) in tied.iter().skip(i + 1) {
                if name_a != name_b {
                    let (name_a, entry_a, name_b, entry_b) = if name_a <= name_b {
                        (*name_a, *entry_a, *name_b, *entry_b)
                    } else {
                        (*name_b, *entry_b, *name_a, *entry_a)
                    };
                    found.insert(format!(
                        "{path} claimed by both {name_a} entry '{entry_a}' and {name_b} entry '{entry_b}' (equal specificity)"
                    ));
                }
            }
        }
    }
    found.into_iter().collect()
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
fn is_test_module(model: &Model, module_path: &str) -> bool {
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
fn owning_unit(model: &Model, module_path: &str) -> Option<String> {
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

    // A go unit IS an addressable module-tier path (its import path), so a
    // `matches.modules` pattern claims it directly — in a single-module tree
    // (no scan tier to look paths up in) and in a go.work tree alike, where
    // the unit names carry the same import paths the tier projects.
    let go_unit_paths = model.language == "go";
    let go_unit_claims = |module: &Module, unit: &str| {
        go_unit_paths
            && module
                .matches
                .modules
                .iter()
                .any(|pattern| module_path_matches(pattern, unit))
    };

    for module in modules {
        let matched = model.soft_structure.values().flatten().any(|path| {
            module
                .matches
                .modules
                .iter()
                .any(|pattern| module_path_matches(pattern, path))
        }) || model
            .units
            .iter()
            .any(|unit| go_unit_claims(module, &unit.name));
        if matched {
            module_matched.insert(module.name.clone());
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
        let owner = modules.iter().find(|module| {
            module
                .matches
                .modules
                .iter()
                .any(|pattern| paths.iter().any(|path| module_path_matches(pattern, path)))
                || go_unit_claims(module, unit)
        });
        match owner {
            Some(owner) => {
                assignments
                    .entry(owner.name.clone())
                    .or_default()
                    .insert(unit.clone());
                unit_module.insert(unit.clone(), owner.name.clone());
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
            unit,
            from,
            to,
            symbols: Vec::new(),
        })
        .collect();
    Cow::Owned(derived)
}

/// Structural compare (ADR-010): extracted vs declared component sets, plus
/// edge, contract, and cycle checks. Order-insensitive; every category is
/// accumulated, never fail-fast.
pub fn compare(
    mapping: &Mapping,
    model: &Model,
    modules: &[Module],
    stereotypes: &[Stereotype],
    constraints: &[Constraint],
) -> Diff {
    let soft_verifiable = soft_visibility_verifiable(model);
    let mapping = augment_with_modules(mapping, model, modules);
    let derived = effective_module_tier(model, &mapping);
    let model = derived.as_ref();
    let declared: BTreeSet<&str> = modules.iter().map(|module| module.name.as_str()).collect();
    let resolved = resolved_components(&mapping);
    let extracted: BTreeSet<&str> = resolved.iter().map(String::as_str).collect();

    let mut diff = Diff {
        missing_components: declared
            .difference(&extracted)
            .map(|name| name.to_string())
            .collect(),
        unexpected_components: mapping.unexpected_components.clone(),
        unassigned_units: mapping.unassigned_units.clone(),
        ambiguous_module_matches: check_ambiguous_module_matches(model, modules),
        ..Default::default()
    };

    for paths in model.unresolved_module_files.values() {
        for path in paths {
            diff.warnings
                .push(format!("unresolved module file: {path}"));
        }
    }
    let composition_sources = composition_sources(model);
    let graph = resolve_module_edges(model, &mapping, modules, &composition_sources);
    for path in &graph.unowned_endpoints {
        diff.warnings
            .push(format!("unowned module edge endpoint: {path}"));
    }
    check_facade_roots(model, &mut diff);
    check_allowed_edges(&graph.pairs, model, modules, &mut diff);
    check_missing_edges(&graph.pairs, modules, &mut diff);
    check_forbidden_laundering(
        &graph.pairs,
        &graph.fallback_pairs,
        modules,
        &mut diff.warnings,
    );
    diff.contract_leaks = check_contract_leaks(&mapping, model, modules, stereotypes);
    let (cycles, mut warnings) = check_cycles(&graph.pairs, constraints, modules);
    diff.cycles = cycles;
    diff.warnings.append(&mut warnings);

    let (leaks, glob_leaks, empty_globs, mut warning_leaks) =
        check_public_api(&mapping, model, modules, constraints);
    diff.public_api_leaks = leaks;
    diff.unverifiable_glob_exports = glob_leaks;
    diff.empty_glob_exports = empty_globs;
    diff.warnings.append(&mut warning_leaks);

    let (external, mut warning_external) = check_forbid_external(model, constraints);
    diff.forbidden_external_crates = external;
    diff.warnings.append(&mut warning_external);

    let (impure, mut warning_impure) = check_external_free(model, constraints);
    diff.not_external_free = impure;
    diff.warnings.append(&mut warning_impure);

    let (manifest, mut warning_manifest) = check_manifest_integrity(model, constraints);
    diff.manifest_integrity = manifest;
    diff.warnings.append(&mut warning_manifest);

    let (boundaries, mut warning_boundaries) = check_feature_boundary(model, constraints);
    diff.feature_boundaries = boundaries;
    diff.warnings.append(&mut warning_boundaries);

    let (submodules, mut warning_submodules) = check_forbid_submodule(model, constraints);
    diff.forbidden_submodule_dependencies = submodules;
    diff.warnings.append(&mut warning_submodules);

    diff.warnings.append(&mut check_reference_engagement(
        model,
        modules,
        stereotypes,
        soft_verifiable,
    ));

    diff.vacuous_constraints = check_vacuous_constraints(model, modules, constraints, &graph.pairs);

    diff.warnings.sort();
    diff.forbidden_edges.sort();
    diff.missing_edges.sort();
    diff.disallowed_cross_component.sort();
    diff
}

/// Map extracted edges onto declared-boundary pairs, deduplicated, over the
/// union of both tiers. Hard unit `edges` resolve through the unit->boundary
/// mapping; soft `module_edges` resolve each endpoint to its owning boundary
/// via `matches.modules`. Intra-boundary edges do not cross a boundary.
///
/// Soft-tier endpoints owned by no boundary cannot be placed on the boundary
/// graph, so the pair is not reportable as a boundary edge; they are returned
/// as `unowned_endpoints` and reported as warnings instead of vanishing,
/// because dropping them silently is how a dependency hides behind a module
/// the spec never claims. Pairs whose ownership came through the unit
/// fallback are also returned in `fallback_pairs`: they cross territory no
/// boundary declares through `matches.modules` (unit wiring, unclaimed
/// modules), which is what lets a banned pair be routed around. One
/// exclusion applies: hops SOURCED by a composition-role path are the
/// composition unit wiring out — sanctioned whatever boundary the attribution
/// landed them on — so they lose the conduit character while staying in
/// `pairs` (a banned route still reaches its target through them).
fn resolve_module_edges(
    model: &Model,
    mapping: &Mapping,
    modules: &[Module],
    composition_sources: &BTreeSet<String>,
) -> BoundaryGraph {
    let mut pairs = BTreeSet::new();
    let mut fallback_pairs = BTreeSet::new();
    let mut unowned_endpoints = BTreeSet::new();
    for edge in &model.edges {
        let (Some(from_module), Some(to_module)) = (
            mapping.unit_module.get(&edge.from),
            mapping.unit_module.get(&edge.to),
        ) else {
            continue;
        };
        if from_module != to_module {
            pairs.insert((from_module.clone(), to_module.clone()));
        }
    }
    for edge in &model.module_edges {
        if is_test_module(model, &edge.from) || is_test_module(model, &edge.to) {
            continue;
        }
        let from_module = module_owner_with_fallback(&edge.from, modules);
        let to_module = module_owner_with_fallback(&edge.to, modules);
        match (from_module, to_module) {
            (Some((from_module, from_fallback)), Some((to_module, to_fallback))) => {
                if composition_sources.contains(&edge.from)
                    && from_fallback
                    && !modules.iter().any(|module| {
                        module.name == from_module
                            && module
                                .matches
                                .units
                                .iter()
                                .any(|pattern| matches_pattern(&edge.unit, pattern))
                    })
                {
                    // Misattributed sanctioned hop: the composition path was
                    // projected onto a boundary whose `units` do not cover the
                    // unit the wiring edge actually lives in (rollup splits
                    // multi-segment unit roots at the first `::`, so a
                    // solution root `Company::App` lands on the neighbour
                    // claiming `Company`). That pair is an artefact of
                    // attribution, not an observed dependency — it neither
                    // adjudicates nor launders (roles US 05b).
                    continue;
                }
                if from_module != to_module {
                    let pair = (from_module.to_string(), to_module.to_string());
                    if (from_fallback || to_fallback)
                        && !composition_sources.contains(&edge.from)
                    {
                        fallback_pairs.insert(pair.clone());
                    }
                    pairs.insert(pair);
                }
            }
            (None, None) => {
                unowned_endpoints.insert(edge.from.clone());
                unowned_endpoints.insert(edge.to.clone());
            }
            (None, Some(_)) => {
                unowned_endpoints.insert(edge.from.clone());
            }
            (Some(_), None) => {
                unowned_endpoints.insert(edge.to.clone());
            }
        }
    }
    BoundaryGraph {
        pairs,
        fallback_pairs,
        unowned_endpoints,
    }
}

/// Forbidden edge (#5): the target module is explicitly in the source module's
/// `allowed.forbidden`. Disallowed cross-component dependency (#6): the target
/// is not in `allowed.depend_on` either. An explicitly forbidden edge is
/// reported once as forbidden, never also as disallowed.
///
/// The allow/deny lookup follows ADR-018: allowances are owned by the boundary
/// that matched the edge's source UNIT identity, so a dependency attributed
/// under a namespace/module identity that its source's unit boundary already
/// grants is not re-adjudicated against the allowance-less module boundary it
/// resolved to. An attributed pair the owning unit boundary also denies is
/// reported with its attributed text; pairs whose source has no unit
/// attribution are adjudicated by boundary-name equality alone, unchanged.
fn check_allowed_edges(
    pairs: &BTreeSet<(String, String)>,
    model: &Model,
    modules: &[Module],
    diff: &mut Diff,
) {
    for (from_module, to_module) in pairs {
        let Some(from_spec) = modules.iter().find(|module| &module.name == from_module) else {
            continue;
        };
        if from_spec
            .allowed
            .forbidden
            .iter()
            .any(|target| target == to_module)
        {
            diff.forbidden_edges
                .push(format!("{from_module} -> {to_module}"));
        } else if !from_spec
            .allowed
            .depend_on
            .iter()
            .any(|target| target == to_module)
            && !unit_tier_identity_allows(from_module, to_module, model, modules)
        {
            diff.disallowed_cross_component
                .push(format!("{from_module} -> {to_module}"));
        }
    }
}

/// The boundary claiming `unit` through a `matches.units` pattern — the
/// unit-tier attribution of `map_units`, deliberately ignoring the module-tier
/// fallbacks of `augment_with_modules`: only a boundary that claims the unit
/// by its unit identity owns allowances for edges attributed under it
/// (ADR-018).
fn unit_owner_boundary<'a>(unit: &str, modules: &'a [Module]) -> Option<&'a str> {
    modules
        .iter()
        .find(|module| {
            module
                .matches
                .units
                .iter()
                .any(|pattern| matches_pattern(unit, pattern))
        })
        .map(|module| module.name.as_str())
}

/// True when the pair's unit-tier identity is granted: each endpoint resolves
/// to its owning unit's boundary (falling back to the endpoint's own name
/// where no boundary claims that identity by units), and the SOURCE owner's
/// allow-list names the target without forbidding it. `false` when the source
/// has no unit attribution at all (module-only specs keep today's adjudication)
/// or when both endpoints collapse into one boundary: a dependency inside a
/// single unit stays module-attributed and is judged by the module boundaries
/// that name it (ADR-018).
fn unit_tier_identity_allows(
    from: &str,
    to: &str,
    model: &Model,
    modules: &[Module],
) -> bool {
    let Some(from_unit) = owning_unit(model, from) else {
        return false;
    };
    let Some(owner) = unit_owner_boundary(&from_unit, modules) else {
        return false;
    };
    let target = owning_unit(model, to)
        .and_then(|unit| unit_owner_boundary(&unit, modules))
        .unwrap_or(to);
    if target == owner {
        return false;
    }
    modules.iter().find(|module| module.name == owner).is_some_and(|spec| {
        spec.allowed.depend_on.iter().any(|t| t == target)
            && !spec.allowed.forbidden.iter().any(|t| t == target)
    })
}

/// Missing edge (#11): a declared `allowed.depend_on` target is a declared
/// module, but no extracted edge connects the source module to it.
fn check_missing_edges(pairs: &BTreeSet<(String, String)>, modules: &[Module], diff: &mut Diff) {
    for module in modules {
        for target in &module.allowed.depend_on {
            let target_declared = modules.iter().any(|candidate| &candidate.name == target);
            if target_declared && !pairs.contains(&(module.name.clone(), target.clone())) {
                diff.missing_edges
                    .push(format!("{} -> {target}", module.name));
            }
        }
    }
}

/// Publication-only root facade (structural, no constraint declares it): a
/// Every module path the model's roles map marks `facade` (derived per driver:
/// the rust crate root defining nothing but `mod` declarations and re-exports,
/// the c# using-facts-only root namespace) is publication surface. An edge
/// INTO that root from anything other than a composition-role-carrying module
/// is reported regardless of `depend_on`: the umbrella is not a dependency
/// target, and an internal→root edge launders any ban routed through it (the
/// laundering check reports the ban itself; this check kills the conduit).
/// Listing the root in `depend_on` legalizes the pair for the boundary checks
/// only — the fix here is canonicalizing the import (rust) or consuming the
/// facade's product namespaces instead of its root (c#). Root→internal edges
/// (declarations, re-exports) are the facade's publication direction and stay
/// legal; cross-unit consumption of the umbrella through the root namespace is
/// exactly the edge this rule reports, whatever unit records it.
fn check_facade_roots(model: &Model, diff: &mut Diff) {
    for (root, role) in &model.roles {
        if !matches!(role, Role::Facade) {
            continue;
        }
        for edge in &model.module_edges {
            // Only edges INTO the facade root consume it; the root's own
            // publication edges (root -> internals) are the facade direction.
            if &edge.to != root || &edge.from == root {
                continue;
            }
            // Test-gated scaffolding on either end is excluded from the graph,
            // same as every other module-edge check.
            if is_test_module(model, &edge.from) || is_test_module(model, &edge.to) {
                continue;
            }
            // The exemption is the composition ROLE, not a `main` name: a
            // composition root wiring the umbrella (rust bin `main` linking
            // its lib, a c# composition root referencing the umbrella) is
            // sanctioned wiring, not consumption. Any other source — internal
            // modules, umbrella consumers, or a module that merely happens to
            // be named `main` without the role — is reported.
            if matches!(model.roles.get(&edge.from), Some(Role::Composition)) {
                continue;
            }
            diff.facade_dependencies
                .push(format!("{} -> {}", edge.from, edge.to));
        }
    }
    diff.facade_dependencies.sort();
}

/// Laundered forbidden edge: an explicitly forbidden pair `from -> target`
/// is not directly extracted, yet the banned boundary itself exits into
/// territory no `matches.modules` claims — a fallback hop whose other end
/// reaches the ban. Routing a ban through one's own undeclared territory
/// violates the same declaration; reported as a warning, promoted by
/// `--strict`. The conduit is the banned module's OWN exit hop: hops deeper
/// in the route are other boundaries' territory and answer to those
/// boundaries' own bans. An exit hop SOURCED by a composition-role path is
/// sanctioned wiring (excluded from `fallback_pairs` upstream), so a ban
/// bridged by explicit boundaries and composition wiring never launders; a
/// coexisting purely declared route does not mask a conduit route.
fn check_forbidden_laundering(
    pairs: &BTreeSet<(String, String)>,
    fallback_pairs: &BTreeSet<(String, String)>,
    modules: &[Module],
    warnings: &mut Vec<String>,
) {
    let connects = |a: &str, b: &str| a == b || reachable(pairs, a, b);
    for module in modules {
        for target in &module.allowed.forbidden {
            let target_declared = modules.iter().any(|candidate| &candidate.name == target);
            if !target_declared || pairs.contains(&(module.name.clone(), target.clone())) {
                continue;
            }
            let rides_conduit = fallback_pairs.iter().any(|(hop_from, hop_to)| {
                hop_from == &module.name && connects(hop_to, target)
            });
            if !rides_conduit {
                continue;
            }
            if let Some(path) = reachable_path(pairs, &module.name, target) {
                let intermediates = path[1..path.len() - 1].join(" -> ");
                warnings.push(format!(
                    "laundered forbidden edge: {} -> {target} via {intermediates}",
                    module.name
                ));
            }
        }
    }
}

fn reachable(pairs: &BTreeSet<(String, String)>, from: &str, target: &str) -> bool {
    reachable_path(pairs, from, target).is_some()
}

/// Reachability over the pair graph with predecessor reconstruction (any
/// shortest path). `from` is never revisited.
fn reachable_path(
    pairs: &BTreeSet<(String, String)>,
    from: &str,
    target: &str,
) -> Option<Vec<String>> {
    let mut visited: BTreeSet<&str> = BTreeSet::new();
    visited.insert(from);
    let mut came_from: BTreeMap<&str, &str> = BTreeMap::new();
    let mut frontier: Vec<&str> = Vec::new();
    for (a, b) in pairs {
        if a.as_str() == from && visited.insert(b.as_str()) {
            came_from.insert(b.as_str(), from);
            frontier.push(b.as_str());
        }
    }
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for node in frontier {
            if node == target {
                let mut path = vec![node.to_string()];
                let mut cursor = node;
                while let Some(prev) = came_from.get(cursor).copied() {
                    path.push(prev.to_string());
                    cursor = prev;
                }
                path.reverse();
                return Some(path);
            }
            for (a, b) in pairs {
                if a.as_str() == node && visited.insert(b.as_str()) {
                    came_from.insert(b.as_str(), node);
                    next.push(b.as_str());
                }
            }
        }
        frontier = next;
    }
    None
}

/// Contract leak (#8, #90-#92): a module's `contract.forbid` names a
/// stereotype, and one of the module's own units matches that stereotype's
/// match-set. The exposed stereotype bleeds out of the declared contract.
/// Phase-1 matches stereotype patterns (names/paths/units) against unit names;
/// symbol-level checks arrive with phase 2. Submodules are enforced by
/// recursion: a contract on a submodule applies to the surface crossing that
/// submodule's boundary path — module paths under the boundary (attributed by
/// the same `module_path_matches` rule as module ownership) and units claimed
/// by the submodule's `matches.units` globs — matching stereotype patterns
/// against those path strings by the full-path/bare-name rule.
fn check_contract_leaks(
    mapping: &Mapping,
    model: &Model,
    modules: &[Module],
    stereotypes: &[Stereotype],
) -> Vec<String> {
    let mut leaks = BTreeSet::new();
    for module in modules {
        if let Some(unit_names) = mapping.assignments.get(&module.name) {
            for forbidden in &module.contract.forbid {
                let exposed = stereotypes
                    .iter()
                    .filter(|stereotype| &stereotype.name == forbidden)
                    .any(|stereotype| {
                        unit_names
                            .iter()
                            .any(|unit| stereotype_matches_unit(unit, stereotype))
                    });
                if exposed {
                    leaks.insert(format!("{} exposes {forbidden} (forbidden)", module.name));
                }
            }
        }
        collect_submodule_contract_leaks(model, &module.submodules, stereotypes, &mut leaks);
    }
    leaks.into_iter().collect()
}

/// Recurse the contract-leak check into submodules (one level per nesting,
/// boundary = the submodule's declared path patterns).
fn collect_submodule_contract_leaks(
    model: &Model,
    submodules: &[Module],
    stereotypes: &[Stereotype],
    leaks: &mut BTreeSet<String>,
) {
    for submodule in submodules {
        let boundary: Vec<String> = if submodule.matches.modules.is_empty() {
            vec![submodule.name.clone()]
        } else {
            submodule.matches.modules.clone()
        };
        for forbidden in &submodule.contract.forbid {
            let exposed = stereotypes.iter().any(|stereotype| {
                &stereotype.name == forbidden
                    && submodule_surface_matches(stereotype, submodule, model, &boundary)
            });
            if exposed {
                leaks.insert(format!(
                    "{} exposes {forbidden} (forbidden)",
                    submodule.name
                ));
            }
        }
        collect_submodule_contract_leaks(model, &submodule.submodules, stereotypes, leaks);
    }
}

/// True if a stereotype's match-set hits the submodule's surface: a unit
/// claimed by its `matches.units` globs (top-level semantics), or a module
/// path inside the boundary matched by the full-path/bare-name rule.
fn submodule_surface_matches(
    stereotype: &Stereotype,
    submodule: &Module,
    model: &Model,
    boundary: &[String],
) -> bool {
    let units_claimed = submodule.matches.units.iter().any(|pattern| {
        model.units.iter().any(|unit| {
            matches_pattern(&unit.name, pattern) && stereotype_matches_unit(&unit.name, stereotype)
        })
    });
    units_claimed
        || model.soft_structure.values().flatten().any(|path| {
            boundary
                .iter()
                .any(|pattern| module_path_matches(pattern, path))
                && stereotype
                    .match_set
                    .names
                    .iter()
                    .chain(stereotype.match_set.paths.iter())
                    .chain(stereotype.match_set.units.iter())
                    .any(|pattern| module_matches(pattern, path))
        })
}

fn stereotype_matches_unit(value: &str, stereotype: &Stereotype) -> bool {
    stereotype
        .match_set
        .names
        .iter()
        .chain(stereotype.match_set.paths.iter())
        .chain(stereotype.match_set.units.iter())
        .any(|pattern| matches_pattern(value, pattern))
}

/// `no_cycles` check (#9, severity #12-#14): for each such constraint, take the
/// extracted module-pair edges induced on the constraint's modules (all
/// declared modules if the list is empty) and find every strongly connected
/// component of size greater than one. Each such SCC is a cycle among the
/// declared groups; one representative, canonically smallest cycle path is
/// rendered per SCC. A cycle found by a constraint whose `severity` is
/// `"warning"` goes to the warning bucket (rendered as `<category>: <detail>`);
/// anything else is an error. Returns `(error_cycles, warning_cycles)`.
fn check_cycles(
    module_edges: &BTreeSet<(String, String)>,
    constraints: &[Constraint],
    modules: &[Module],
) -> (Vec<String>, Vec<String>) {
    let all_modules: BTreeSet<String> = modules.iter().map(|module| module.name.clone()).collect();
    let mut error_cycles: BTreeSet<String> = BTreeSet::new();
    let mut warning_cycles: BTreeSet<String> = BTreeSet::new();

    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "no_cycles")
    {
        let severity_is_warning = constraint.severity == "warning";
        let group: BTreeSet<String> = if constraint.modules.is_empty() {
            all_modules.clone()
        } else {
            constraint
                .modules
                .iter()
                .filter(|name| all_modules.contains(*name))
                .cloned()
                .collect()
        };
        let induced: BTreeSet<(String, String)> = module_edges
            .iter()
            .filter(|(from, to)| group.contains(from) && group.contains(to))
            .cloned()
            .collect();

        for scc in strongly_connected_components(&group, &induced) {
            if let Some(path) = find_cycle(&scc, &induced) {
                if severity_is_warning {
                    warning_cycles.insert(path.join(" -> "));
                } else {
                    error_cycles.insert(path.join(" -> "));
                }
            }
        }
    }

    // A cycle covered by both a warning and an error constraint must fail: an
    // error-level finding wins over the warning. Only then sort for the report.
    let mut warnings: Vec<String> = warning_cycles
        .difference(&error_cycles)
        .map(|path| format!("cycle: {path}"))
        .collect();
    warnings.sort();
    let mut errors: Vec<String> = error_cycles.into_iter().collect();
    errors.sort();
    (errors, warnings)
}

/// Tarjan's strongly connected components over the induced graph, each SCC
/// sorted for deterministic iteration.
fn strongly_connected_components(
    group: &BTreeSet<String>,
    edges: &BTreeSet<(String, String)>,
) -> Vec<Vec<String>> {
    struct Tarjan<'a> {
        edges: &'a BTreeSet<(String, String)>,
        group: &'a BTreeSet<String>,
        index: usize,
        indices: BTreeMap<String, usize>,
        lowlink: BTreeMap<String, usize>,
        on_stack: BTreeSet<String>,
        stack: Vec<String>,
        components: Vec<Vec<String>>,
    }

    impl<'a> Tarjan<'a> {
        fn visit(&mut self, node: &str) {
            self.indices.insert(node.to_string(), self.index);
            self.lowlink.insert(node.to_string(), self.index);
            self.index += 1;
            self.stack.push(node.to_string());
            self.on_stack.insert(node.to_string());

            for (from, to) in self.edges {
                if from != node || !self.group.contains(to) {
                    continue;
                }
                if !self.indices.contains_key(to) {
                    self.visit(to);
                    let candidate = self.lowlink[to];
                    let current = self.lowlink[node];
                    self.lowlink
                        .insert(node.to_string(), current.min(candidate));
                } else if self.on_stack.contains(to) {
                    let candidate = self.indices[to];
                    let current = self.lowlink[node];
                    self.lowlink
                        .insert(node.to_string(), current.min(candidate));
                }
            }

            if self.lowlink[node] == self.indices[node] {
                let mut component = Vec::new();
                loop {
                    let popped = self.stack.pop().unwrap();
                    self.on_stack.remove(&popped);
                    if popped == node {
                        component.push(popped);
                        break;
                    }
                    component.push(popped);
                }
                component.sort();
                self.components.push(component);
            }
        }
    }

    let mut tarjan = Tarjan {
        edges,
        group,
        index: 0,
        indices: BTreeMap::new(),
        lowlink: BTreeMap::new(),
        on_stack: BTreeSet::new(),
        stack: Vec::new(),
        components: Vec::new(),
    };

    for node in group {
        if !tarjan.indices.contains_key(node) {
            tarjan.visit(node);
        }
    }
    tarjan.components
}

/// Deterministic representative cycle for a strongly connected component: start
/// at the lexicographically smallest node and greedily follow the smallest
/// outgoing target until a node on the path repeats; the repeated segment is
/// the cycle. A singleton without a self-loop yields no cycle.
fn find_cycle(nodes: &[String], edges: &BTreeSet<(String, String)>) -> Option<Vec<String>> {
    let start = nodes.iter().next()?;
    let mut path = vec![start.clone()];
    let mut position: BTreeMap<String, usize> = BTreeMap::new();
    position.insert(start.clone(), 0);
    let mut current = start.clone();

    loop {
        let next = edges
            .iter()
            .filter(|(from, to)| *from == current && nodes.contains(to))
            .map(|(_, to)| to)
            .min()
            .cloned();
        let next = next?;
        if let Some(&index) = position.get(&next) {
            let mut cycle = path[index..].to_vec();
            cycle.push(next.clone());
            return Some(cycle);
        }
        position.insert(next.clone(), path.len());
        path.push(next.clone());
        current = next;
    }
}

/// Route a finding into the error bucket (detail only; the report adds the
/// `<category>: ` prefix) or, for a `warning`-severity constraint, into the
/// pre-rendered warning bucket as `<category>: <detail>`.
fn push_finding(
    detail: String,
    severity: &str,
    category: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if severity == "warning" {
        warnings.push(format!("{category}: {detail}"));
    } else {
        errors.push(detail);
    }
}

/// Match a module-reference pattern against a dotted module path: true if the
/// pattern glob-matches the full path or its bare (last-segment) name. Lets a
/// pattern like `common` match `app::orchestration::common`.
fn module_matches(pattern: &str, module_path: &str) -> bool {
    if matches_pattern(module_path, pattern) {
        return true;
    }
    module_path
        .rsplit("::")
        .next()
        .is_some_and(|bare| matches_pattern(bare, pattern))
}

/// True if the module path, or any of its ancestor module paths, matches one of
/// the patterns (so a `from` pattern also covers the module's submodules).
fn module_listed(module_path: &str, patterns: &[String]) -> bool {
    let mut current = module_path;
    loop {
        if patterns
            .iter()
            .any(|pattern| module_matches(pattern, current))
        {
            return true;
        }
        match current.rsplit_once("::") {
            Some((rest, _)) => current = rest,
            None => return false,
        }
    }
}

/// Strip the unit (first segment) from a dotted module path for reporting, so
/// `app::billing::sub` renders as `billing::sub`.
fn strip_unit(module_path: &str) -> &str {
    match module_path.split_once("::") {
        Some((_, rest)) if !rest.is_empty() => rest,
        _ => module_path,
    }
}

/// The longest common dotted-prefix of two module paths (their shared ancestor).
fn common_ancestor(left: &str, right: &str) -> Option<String> {
    let mut common: Vec<&str> = Vec::new();
    for (x, y) in left.split("::").zip(right.split("::")) {
        if x == y {
            common.push(x);
        } else {
            break;
        }
    }
    if common.is_empty() {
        None
    } else {
        Some(common.join("::"))
    }
}

/// The portion of `path` after the ancestor prefix (`common` -> empty), used to
/// isolate a submodule name within a parent.
fn rel_after(path: &str, ancestor: &str) -> String {
    match path.strip_prefix(&format!("{ancestor}::")) {
        Some(rest) => rest.to_string(),
        None => String::new(),
    }
}

/// `public_api_allowlist` (#16-#18, amended by the glob-enumeration plan):
/// every public item exported from a crate root must be in the constraint's
/// `allowed` list. Reserved root plumbing (non-public items) never counts as an
/// export (see `root_public_exports`). Resolved root globs land in that same
/// enumerated set; globs the scan could not resolve are reported as
/// unverifiable, and globs resolving to an empty surface fail closed — neither
/// is silently skipped.
fn check_public_api(
    mapping: &Mapping,
    model: &Model,
    modules: &[Module],
    constraints: &[Constraint],
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut glob_errors = Vec::new();
    let mut empty_errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "public_api_allowlist")
    {
        // Crate-root exports belong to the unit itself; a boundary owning the
        // unit by `matches.units` is named instead (22a/22b/22c).
        let owner = |unit: &str| -> String {
            if modules
                .iter()
                .any(|m| m.matches.units.iter().any(|p| matches_pattern(unit, p)))
            {
                mapping
                    .unit_module
                    .get(unit)
                    .map(String::as_str)
                    .unwrap_or(unit)
                    .to_string()
            } else {
                unit.to_string()
            }
        };
        for (unit, exports) in &model.root_public_exports {
            let owner = owner(unit);
            for export in exports {
                let allowlisted = constraint
                    .allowed
                    .iter()
                    .any(|pattern| matches_pattern(export, pattern));
                if !allowlisted {
                    push_finding(
                        format!("{owner} exposes {export} (not allowlisted)"),
                        &constraint.severity,
                        "public api leak",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }
        for (unit, globs) in &model.root_glob_exports {
            let owner = owner(unit);
            for glob in globs {
                push_finding(
                    format!("{owner} exposes {glob}"),
                    &constraint.severity,
                    "unverifiable glob export",
                    &mut glob_errors,
                    &mut warnings,
                );
            }
        }
        for (unit, globs) in &model.root_empty_glob_exports {
            let owner = owner(unit);
            for glob in globs {
                push_finding(
                    format!("{owner} exposes {glob} (resolves to no public items)"),
                    &constraint.severity,
                    "empty glob export",
                    &mut empty_errors,
                    &mut warnings,
                );
            }
        }
    }
    errors.sort();
    errors.dedup();
    glob_errors.sort();
    glob_errors.dedup();
    empty_errors.sort();
    empty_errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, glob_errors, empty_errors, warnings)
}

/// `forbid_external_crates` (#19-#21): a module matching a `from` pattern (or
/// one of its submodules) must not import a crate matching a `forbid` pattern.
/// The owning module of each external import comes from the model's
/// `module_external` map.
fn check_forbid_external(model: &Model, constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "forbid_external_crates")
    {
        for (module_path, crates) in &model.module_external {
            if !module_listed(module_path, &constraint.from) {
                continue;
            }
            for crate_name in crates {
                if constraint
                    .forbid
                    .iter()
                    .any(|pattern| matches_pattern(crate_name, pattern))
                {
                    push_finding(
                        format!("{} -> {crate_name}", strip_unit(module_path)),
                        &constraint.severity,
                        "forbidden external crate",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }
    }
    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, warnings)
}

/// Elements present in the model that a constraint `from` pattern can address:
/// unit names (the unit tier every driver emits) plus module paths wherever a
/// structure tier exists (soft paths, edge endpoints, external attributions).
/// `external_free` engagement reads presence — not externals — because an
/// attributed zero is precisely the state it monitors.
fn present_elements(model: &Model) -> BTreeSet<String> {
    let mut elements: BTreeSet<String> = model.units.iter().map(|unit| unit.name.clone()).collect();
    elements.extend(model.soft_structure.values().flatten().cloned());
    elements.extend(model.module_external.keys().cloned());
    for edge in &model.module_edges {
        elements.insert(edge.from.clone());
        elements.insert(edge.to.clone());
    }
    elements
}

/// `external_free`: a matched module or unit must be attributed zero external
/// packages. Engagement is presence-only (see `present_elements`): a matched
/// element with no externals PASSES — a real check, not a vacuous pass — and
/// any attributed package is a violation naming the module and the packages.
/// A module-tier pattern monitors the subtree through the same
/// `module_path_matches` resolution the vacuity detector uses; a pattern
/// naming a unit monitors every external attributed anywhere under that unit
/// (unit-rooted `module_external` keys, the whole unit on a tier-less go tree).
fn check_external_free(model: &Model, constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "external_free")
    {
        for (module_path, packages) in &model.module_external {
            if packages.is_empty() {
                continue;
            }
            if !constraint
                .from
                .iter()
                .any(|pattern| module_path_matches(pattern, module_path))
            {
                continue;
            }
            let names: BTreeSet<&str> = packages.iter().map(String::as_str).collect();
            push_finding(
                format!(
                    "{} imports {}",
                    strip_unit(module_path),
                    names.into_iter().collect::<Vec<_>>().join(", ")
                ),
                &constraint.severity,
                "not external free",
                &mut errors,
                &mut warnings,
            );
        }
    }
    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, warnings)
}

/// `manifest_integrity` (#22-#25, #53-#55): each package manifest is checked
/// individually — a violation names the manifest that fails it, and a required
/// feature must be present in every checked manifest (a sibling having it does
/// not mask a member that lacks it). Targets are the per-unit manifests when a
/// workspace scan surfaced them; otherwise the root manifest is a single
/// `Cargo.toml` target (preserves behavior for non-rust drivers).
fn check_manifest_integrity(
    model: &Model,
    constraints: &[Constraint],
) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "manifest_integrity")
    {
        for (label, manifest) in manifest_targets(model) {
            if let Some(required) = constraint.require_publish {
                let resolved = manifest.publish.unwrap_or(false);
                if required && !resolved {
                    push_finding(
                        format!("{label} publish=false (required true)"),
                        &constraint.severity,
                        "manifest integrity",
                        &mut errors,
                        &mut warnings,
                    );
                } else if !required && resolved {
                    push_finding(
                        format!("{label} publish=true (required false)"),
                        &constraint.severity,
                        "manifest integrity",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
            for forbidden in &constraint.forbidden_dependencies {
                if manifest
                    .dependencies
                    .iter()
                    .any(|dep| matches_pattern(dep, forbidden))
                {
                    push_finding(
                        format!("{label} has forbidden dependency {forbidden}"),
                        &constraint.severity,
                        "manifest integrity",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
            for required in &constraint.required_features {
                if !manifest
                    .features
                    .iter()
                    .any(|feature| matches_pattern(feature, required))
                {
                    push_finding(
                        format!("{label} missing required feature {required}"),
                        &constraint.severity,
                        "manifest integrity",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }
    }
    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, warnings)
}

/// The `(label, ManifestInfo)` entries a `manifest_integrity` constraint checks,
/// deduplicated by manifest path (a package with multiple compilation targets
/// shares one manifest). When per-unit manifests are absent, falls back to the
/// root manifest as a single manifest entry. The label uses the language's
/// manifest file name (Cargo.toml for rust, csproj for csharp, go.mod for go).
fn manifest_targets(model: &Model) -> Vec<(String, ManifestInfo)> {
    let manifest_file = |path: &str| {
        if path == "." {
            manifest_file_name(&model.language).to_string()
        } else {
            format!("{}/{}", path, manifest_file_name(&model.language))
        }
    };
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut targets: Vec<(String, ManifestInfo)> = Vec::new();
    if model.unit_manifests.is_empty() {
        if let Some(manifest) = &model.manifest {
            targets.push((manifest_file("."), manifest.clone()));
        }
        return targets;
    }
    let mut units: Vec<&Unit> = model.units.iter().collect();
    units.sort_by(|left, right| left.name.cmp(&right.name));
    for unit in units {
        let Some(manifest) = model.unit_manifests.get(&unit.name) else {
            continue;
        };
        let label = manifest_file(&unit.path);
        if seen.insert(label.clone()) {
            targets.push((label, manifest.clone()));
        }
    }
    targets
}

/// The manifest file name for a language (labels in `manifest_integrity`
/// findings). Defaults to `Cargo.toml` for unknown languages.
fn manifest_file_name(language: &str) -> &'static str {
    match language {
        "csharp" => "csproj",
        "go" => "go.mod",
        _ => "Cargo.toml",
    }
}

/// `feature_boundary` (#26-#29, #47): gated modules must be declared under
/// `cfg(feature = "...")`, the declared feature must match the module's actual
/// gate, only allowed modules may depend on them (a gated module covers its
/// submodules as a boundary subtree), and each `gated_modules` pattern must
/// match a declared module. On a driver whose capability row says
/// `root-module-declarations` is not-emitted every check of this constraint is
/// inert — the constraint is skipped here (no per-pattern #29 findings are
/// invented from an empty declaration map) and surfaces as a
/// capability-reason vacuous constraint in `check_vacuous_constraints`
/// instead.
fn check_feature_boundary(model: &Model, constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let driver_emits_declarations = !matches!(
        language::from_name(&model.language).and_then(|driver_language| {
            capability::emission(driver_language, capability::FACT_ROOT_MODULE_DECLARATIONS)
        }),
        Some(capability::NOT_EMITTED)
    );
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "feature_boundary")
    {
        if !driver_emits_declarations {
            continue;
        }
        // Declarations across all units: (unit, name, gated, feature, file).
        let declarations: Vec<(String, String, bool, Option<String>, String)> = model
            .root_module_declarations
            .iter()
            .flat_map(|(unit, decls)| {
                decls.iter().map(move |decl| {
                    (
                        unit.clone(),
                        decl.name.clone(),
                        decl.gated,
                        decl.feature.clone(),
                        decl.file.clone(),
                    )
                })
            })
            .collect();

        // #29: a gated_modules pattern matching no declared module.
        for pattern in &constraint.gated_modules {
            let known = declarations.iter().any(|(unit, name, _, _, _)| {
                module_matches(pattern, &format!("{unit}::{name}"))
                    || matches_pattern(name, pattern)
            });
            if !known {
                push_finding(
                    format!("gated module pattern '{pattern}' matches no declared module"),
                    &constraint.severity,
                    "feature boundary",
                    &mut errors,
                    &mut warnings,
                );
            }
        }

        // The full paths of the matched gated modules, plus #28 gating checks
        // and #47 feature cross-checks.
        let mut gated_paths: BTreeSet<String> = BTreeSet::new();
        for (unit, name, gated, feature, file) in &declarations {
            let full = format!("{unit}::{name}");
            let matched = constraint
                .gated_modules
                .iter()
                .any(|pattern| module_matches(pattern, &full) || matches_pattern(name, pattern));
            if !matched {
                continue;
            }
            gated_paths.insert(full.clone());
            if !gated {
                push_finding(
                    format!(
                        "{file} module '{name}' not gated under cfg(feature = \"{}\")",
                        constraint.feature
                    ),
                    &constraint.severity,
                    "feature boundary",
                    &mut errors,
                    &mut warnings,
                );
            } else if let Some(actual) = feature {
                // #47: the declared gate must be the module's actual gate.
                if actual != &constraint.feature {
                    push_finding(
                        format!(
                            "{file} module '{name}' gated under feature \"{actual}\", not \"{}\"",
                            constraint.feature
                        ),
                        &constraint.severity,
                        "feature boundary",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }

        // allowed_from: explicit list, or the default `root` + gated modules.
        let allowed_from: Vec<String> = if constraint.allowed_from.is_empty() {
            let mut default = vec!["root".to_string()];
            default.extend(gated_paths.iter().cloned());
            default
        } else {
            constraint.allowed_from.clone()
        };

        // #27: only allowed modules (or submodules inside a gated subtree) may
        // depend on a gated module or any of its submodules.
        for edge in &model.module_edges {
            if !in_gated_boundary(&edge.to, &gated_paths) {
                continue;
            }
            let unit = edge.unit.clone();
            let is_root = edge.from == unit;
            let allowed = (is_root && allowed_from.iter().any(|p| p == "root"))
                || in_gated_boundary(&edge.from, &gated_paths)
                || allowed_from_subtree(&edge.from, &allowed_from);
            if !allowed {
                push_finding(
                    format!(
                        "{} depends on gated module {} (feature: {})",
                        strip_unit(&edge.from),
                        strip_unit(&edge.to),
                        constraint.feature
                    ),
                    &constraint.severity,
                    "feature boundary",
                    &mut errors,
                    &mut warnings,
                );
            }
        }
    }
    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, warnings)
}

/// True if the module path, or any of its ancestors, is an exact gated path: a
/// module is part of a gated boundary (subtree) when it is a gated module or
/// nested under one. So `tauri::commands` sits inside a boundary for gated
/// `tauri`, both as a dependency target and as a dependent.
fn in_gated_boundary(module_path: &str, gated_paths: &BTreeSet<String>) -> bool {
    let mut current = module_path;
    loop {
        if gated_paths.contains(current) {
            return true;
        }
        match current.rsplit_once("::") {
            Some((rest, _)) => current = rest,
            None => return false,
        }
    }
}

/// True if the module path, or any of its ancestors, matches an `allowed_from`
/// pattern (a gated boundary or an explicit allowed source), mirroring the gated
/// boundary's subtree semantics so a submodule of a gated module may depend on
/// the gated module. `root` is handled by the caller, never here.
fn allowed_from_subtree(module_path: &str, allowed_from: &[String]) -> bool {
    let mut current = module_path;
    loop {
        if allowed_from
            .iter()
            .any(|pattern| pattern != "root" && module_matches(pattern, current))
        {
            return true;
        }
        match current.rsplit_once("::") {
            Some((rest, _)) => current = rest,
            None => return false,
        }
    }
}

/// `forbid_submodule_dependency` (#30-#32): within a `parent` module, a listed
/// `from` submodule must not depend on a forbidden sibling submodule.
fn check_forbid_submodule(model: &Model, constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "forbid_submodule_dependency")
    {
        for edge in &model.module_edges {
            let Some(ancestor) = common_ancestor(&edge.from, &edge.to) else {
                continue;
            };
            if !module_path_matches(&constraint.parent, &ancestor) {
                continue;
            }
            let from_sub = rel_after(&edge.from, &ancestor);
            let to_sub = rel_after(&edge.to, &ancestor);
            if from_sub.is_empty() || to_sub.is_empty() {
                continue;
            }
            let from_listed = constraint
                .from
                .iter()
                .any(|pattern| module_path_matches(pattern, &edge.from));
            if !from_listed {
                continue;
            }
            let forbid_matches = constraint
                .forbid
                .iter()
                .any(|pattern| module_path_matches(pattern, &edge.to));
            if forbid_matches {
                let parent = strip_unit(&ancestor);
                push_finding(
                    format!("{parent}::{from_sub} -> {parent}::{to_sub}"),
                    &constraint.severity,
                    "forbidden submodule dependency",
                    &mut errors,
                    &mut warnings,
                );
            }
        }
    }
    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, warnings)
}

/// Vacuous-guard diagnostics (plan_model_driven_workflow.md, Feature 2): for
/// every constraint whose effective domain is empty, emit a warning-severity
/// entry so a check that verifies nothing is surfaced instead of silently
/// passing. A constraint is vacuous only when its real check found nothing,
/// which by construction means its domain is empty (a finding implies a
/// non-empty domain); `feature_boundary` is vacuous by capability whenever
/// the driver's table row says `root-module-declarations` is not-emitted.
/// Each returned entry is pre-rendered
/// `[constraint #N] <type>: <detail>` with `#N` the 1-indexed position in
/// `constraints`; entries are sorted for a byte-stable report.
fn check_vacuous_constraints(
    model: &Model,
    modules: &[Module],
    constraints: &[Constraint],
    module_edges: &BTreeSet<(String, String)>,
) -> Vec<String> {
    let all_modules: BTreeSet<String> = modules.iter().map(|module| module.name.clone()).collect();
    let mut out: Vec<String> = Vec::new();
    for (index, constraint) in constraints.iter().enumerate() {
        let tag = format!("[constraint #{}] {}", index + 1, constraint.kind);
        let detail = match constraint.kind.as_str() {
            // Engaged iff any `from` pattern matches a module with external deps.
            "forbid_external_crates" => {
                let engaged = model
                    .module_external
                    .keys()
                    .any(|module| module_listed(module, &constraint.from));
                if engaged {
                    None
                } else {
                    let unmatched: Vec<String> = constraint
                        .from
                        .iter()
                        .filter(|pattern| {
                            !model
                                .module_external
                                .keys()
                                .any(|module| module_listed(module, &[(*pattern).clone()]))
                        })
                        .map(|pattern| format!("\"{pattern}\""))
                        .collect();
                    Some(format!(
                        "'from' pattern {} matches no module with external dependencies",
                        unmatched.join(", ")
                    ))
                }
            }
            // Engaged iff a `from` pattern matches at least one element
            // present in the model (a module path where a structure tier
            // exists, else a unit/package name) — presence, not externals:
            // a matched pure element is engaged and passing, while a pattern
            // matching nothing guards nothing and is reported here. The
            // wording stays distinct from the pure pass, which prints no
            // diagnostic at all.
            "external_free" => {
                let elements = present_elements(model);
                let engaged = constraint
                    .from
                    .iter()
                    .any(|pattern| elements.iter().any(|e| module_path_matches(pattern, e)));
                if engaged {
                    None
                } else {
                    let patterns: Vec<String> = constraint
                        .from
                        .iter()
                        .map(|pattern| format!("\"{pattern}\""))
                        .collect();
                    Some(format!(
                        "'from' pattern {} matches no module or unit present in the model",
                        patterns.join(", ")
                    ))
                }
            }
            // Engaged iff the induced pair set over the constraint's group is
            // non-empty (a cycle check over no edges trivially passes).
            "no_cycles" => {
                let group: BTreeSet<String> = if constraint.modules.is_empty() {
                    all_modules.clone()
                } else {
                    constraint
                        .modules
                        .iter()
                        .filter(|name| all_modules.contains(*name))
                        .cloned()
                        .collect()
                };
                let engaged = module_edges
                    .iter()
                    .any(|(from, to)| group.contains(from) && group.contains(to));
                if engaged {
                    None
                } else {
                    Some(
                        "constraint's modules have no dependency edges among them \
                         (cycle check trivially passes)"
                            .to_string(),
                    )
                }
            }
            // Engaged iff there is at least one manifest target to check.
            "manifest_integrity" => {
                if manifest_targets(model).is_empty() {
                    Some("no manifests to check".to_string())
                } else {
                    None
                }
            }
            // Engaged iff any crate-root public export (named or glob-derived),
            // unverifiable glob, or empty glob exists.
            "public_api_allowlist" => {
                if model.root_public_exports.is_empty()
                    && model.root_glob_exports.is_empty()
                    && model.root_empty_glob_exports.is_empty()
                {
                    Some("no crate-root public exports to check".to_string())
                } else {
                    None
                }
            }
            // Engaged iff any module edge has a common ancestor matching `parent`
            // and a source submodule matching a `from` pattern.
            "forbid_submodule_dependency" => {
                let engaged = model.module_edges.iter().any(|edge| {
                    let Some(ancestor) = common_ancestor(&edge.from, &edge.to) else {
                        return false;
                    };
                    if !module_path_matches(&constraint.parent, &ancestor) {
                        return false;
                    }
                    let from_sub = rel_after(&edge.from, &ancestor);
                    if from_sub.is_empty() {
                        return false;
                    }
                    constraint
                        .from
                        .iter()
                        .any(|pattern| module_path_matches(pattern, &edge.from))
                });
                if engaged {
                    None
                } else {
                    Some("no intra-parent submodule edges to check".to_string())
                }
            }
            // Engaged iff the driver emits the root-module-declarations fact
            // the whole constraint reads: a not-emitted row can never match a
            // declaration, so the constraint is vacuous by capability, stated
            // once instead of invented per gated_modules pattern (#29 spam).
            "feature_boundary" => {
                match language::from_name(&model.language).and_then(|driver_language| {
                    capability::emission(driver_language, capability::FACT_ROOT_MODULE_DECLARATIONS)
                }) {
                    Some(capability::NOT_EMITTED) => Some(format!(
                        "driver emits no {} fact (capability {}): the constraint cannot engage",
                        capability::FACT_ROOT_MODULE_DECLARATIONS,
                        capability::NOT_EMITTED
                    )),
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(detail) = detail {
            out.push(format!("{tag}: {detail}"));
        }
    }
    out.sort();
    out
}

/// True if this run can see soft module structure in source, per the
/// capability table: a granular row is always visible;
/// a not-emitted row is never visible. An unknown driver keeps the plain
/// exists/does-not-exist classification.
fn soft_visibility_verifiable(model: &Model) -> bool {
    !matches!(
        language::from_name(&model.language).and_then(|driver_language| capability::emission(
            driver_language,
            capability::FACT_MODULE_TIER
        )),
        Some(capability::NOT_EMITTED)
    )
}

/// Reference engagement diagnostics:
/// a reference in `allowed.depend_on`, `allowed.forbidden`, or
/// `contract.forbid` that can never engage a declared boundary or stereotype
/// is inert — the rule silently verifies nothing. The edge and missing-edge
/// checks resolve targets by exact equality against declared top-level module
/// names, and contract leaks filter stereotypes by exact name, so any other
/// reference (a typo, a removed module, a nested/submodule path) never fires.
/// Each such reference becomes a warning-severity `dead reference` entry:
///
/// - exact declared module / stereotype name -> engaged, no finding;
/// - name resolves to something present in the source model (a module path, a
///   unit) but declared nowhere -> "exists in source but undeclared";
/// - otherwise, when the driver can see soft module structure at all ->
///   "does not exist", plus a did-you-mean candidate (the closest declared
///   name within a small edit distance) where one exists;
/// - otherwise -> "not verifiable from source" — the capability table says
///   this run carries no module-tier fact, so non-existence cannot be claimed.
///
/// `constraint.modules` references are NOT reported here: the schema validator
/// already hard-errors on undeclared module names there. `contract.forbid`
/// references are collected recursively from submodules too — submodule
/// contracts are enforced, so their targets are checked like top-level ones.
/// Entries are rendered as `<category>: <detail>` and deduplicated/sorted for
/// a byte-stable report.
fn check_reference_engagement(
    model: &Model,
    modules: &[Module],
    stereotypes: &[Stereotype],
    soft_verifiable: bool,
) -> Vec<String> {
    let declared: BTreeSet<&str> = modules.iter().map(|module| module.name.as_str()).collect();
    let declared_stereotypes: BTreeSet<&str> = stereotypes
        .iter()
        .map(|stereotype| stereotype.name.as_str())
        .collect();
    let mut found: BTreeSet<String> = BTreeSet::new();
    for module in modules {
        for (field, target) in module
            .allowed
            .depend_on
            .iter()
            .map(|target| ("allowed.depend_on", target.as_str()))
            .chain(
                module
                    .allowed
                    .forbidden
                    .iter()
                    .map(|target| ("allowed.forbidden", target.as_str())),
            )
        {
            if declared.contains(target) {
                continue;
            }
            if source_reference_exists(target, model) {
                found.insert(format!(
                    "dead reference: module '{}' {field} target \"{target}\" exists in source \
                     but undeclared — references resolve to declared top-level module names only",
                    module.name
                ));
            } else if soft_verifiable {
                found.insert(format!(
                    "dead reference: module '{}' {field} target \"{target}\" does not exist — \
                     create it or fix the reference{}",
                    module.name,
                    did_you_mean(target, &declared)
                ));
            } else {
                found.insert(format!(
                    "dead reference: module '{}' {field} target \"{target}\" not verifiable \
                     from source — driver emits no {} fact for this tree{}",
                    module.name,
                    capability::FACT_MODULE_TIER,
                    did_you_mean(target, &declared)
                ));
            }
        }
        for forbidden in &module.contract.forbid {
            if declared_stereotypes.contains(forbidden.as_str()) {
                continue;
            }
            found.insert(format!(
                "dead reference: module '{}' contract.forbid target \"{forbidden}\" names no \
                 stereotype — declare a stereotype named \"{forbidden}\" or fix the reference{}",
                module.name,
                did_you_mean(forbidden, &declared_stereotypes)
            ));
        }
        collect_submodule_dead_references(&module.submodules, &declared_stereotypes, &mut found);
    }
    found.into_iter().collect()
}

/// Submodule `contract.forbid` targets get the same dead-reference treatment
/// as top-level ones (#93): submodule contracts are enforced, so a target
/// naming no declared stereotype can never engage and is reported under the
/// submodule's full boundary path.
fn collect_submodule_dead_references(
    submodules: &[Module],
    declared_stereotypes: &BTreeSet<&str>,
    found: &mut BTreeSet<String>,
) {
    for submodule in submodules {
        for forbidden in &submodule.contract.forbid {
            if declared_stereotypes.contains(forbidden.as_str()) {
                continue;
            }
            found.insert(format!(
                "dead reference: module '{}' contract.forbid target \"{forbidden}\" names no \
                 stereotype — declare a stereotype named \"{forbidden}\" or fix the reference{}",
                submodule.name,
                did_you_mean(forbidden, declared_stereotypes)
            ));
        }
        collect_submodule_dead_references(&submodule.submodules, declared_stereotypes, found);
    }
}

/// True if a reference target addresses something present in the source model:
/// a declared unit name, or a module path in either tier (soft structure or
/// module-edge endpoints) matched by the same full-path/bare-name rules the
/// module matcher uses.
fn source_reference_exists(target: &str, model: &Model) -> bool {
    if model.units.iter().any(|unit| unit.name == target) {
        return true;
    }
    let path_hits = model
        .soft_structure
        .values()
        .flatten()
        .any(|path| reference_matches_path(target, path));
    if path_hits {
        return true;
    }
    model.module_edges.iter().any(|edge| {
        reference_matches_path(target, &edge.from) || reference_matches_path(target, &edge.to)
    })
}

fn reference_matches_path(target: &str, module_path: &str) -> bool {
    module_matches(target, module_path) || module_matches(target, strip_unit(module_path))
}

/// The did-you-mean suffix for an absent reference: the closest declared name
/// by Levenshtein distance, proposed only when the distance is small relative
/// to the reference length (at most one third, floor, minimum 1). Ties break
/// on the name, so the suggestion is deterministic.
fn did_you_mean(target: &str, candidates: &BTreeSet<&str>) -> String {
    let threshold = (target.chars().count() / 3).max(1);
    let best = candidates
        .iter()
        .filter(|name| **name != target)
        .map(|name| (levenshtein(target, name), *name))
        .filter(|(distance, _)| *distance <= threshold)
        .min();
    match best {
        Some((_, name)) => format!(" (did you mean \"{name}\"?)"),
        None => String::new(),
    }
}

fn levenshtein(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0usize; right.len() + 1];
    for (i, l) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, r) in right.iter().enumerate() {
            let cost = usize::from(l != r);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

/// Glob match over a unit/component name or path. Supports `*` (any run of
/// characters, including path separators) and `**` (same as `*` here). Mirrors
/// the spec.md glob conventions (`*Entity`, `**/domain/**`).
pub fn matches_pattern(value: &str, pattern: &str) -> bool {
    glob_match(value.as_bytes(), pattern.as_bytes())
}

fn glob_match(value: &[u8], pattern: &[u8]) -> bool {
    let mut v = 0;
    let mut p = 0;
    let mut star: Option<usize> = None;
    let mut star_v = 0;

    while v < value.len() {
        if p < pattern.len() && (pattern[p] == b'*') {
            star = Some(p);
            star_v = v;
            p += 1;
        } else if p < pattern.len() && pattern[p] == value[v] {
            p += 1;
            v += 1;
        } else if let Some(star) = star {
            star_v += 1;
            v = star_v;
            p = star + 1;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

/// Pass verdict: short deterministic confirmation on stdout.
pub fn pass_confirmation(modules: &[Module], constraints_checked: usize) -> String {
    format!(
        "ok: architecture.spec.toml matches source model ({} modules, {} constraints checked)\n",
        modules.len(),
        constraints_checked
    )
}

/// Full diff report: header plus every divergence, canonically ordered and
/// byte-stable (output.md). Error items keep the plain `<category>: <detail>`
/// lines; warning items (design §9) are listed after all errors, each prefixed
/// with `warning: ` unless `--strict` promotes them to error lines. `strict`
/// only changes how warnings are labelled and whether the run fails — it does
/// not reorder or drop any item.
pub fn render_report(diff: &Diff, strict: bool) -> String {
    let vacuous_only =
        !diff.has_errors() && diff.warnings.is_empty() && !diff.vacuous_constraints.is_empty();
    let mut out = String::new();
    if vacuous_only {
        // A run whose only findings are vacuous constraints never claims a
        // match; the header names each vacuous guard instead.
        let tokens: BTreeSet<&str> = diff
            .vacuous_constraints
            .iter()
            .map(|entry| entry.split(':').next().unwrap_or(entry).trim())
            .collect();
        let mut tokens: Vec<&str> = tokens.into_iter().collect();
        tokens.sort();
        out.push_str(&format!(
            "architecture.spec.toml has vacuous constraints: {}\n",
            tokens.join(", ")
        ));
    } else {
        out.push_str("architecture.spec.toml does not match source model\n");
    }
    for name in &diff.missing_components {
        out.push_str(&format!("  missing component: {name}\n"));
    }
    for name in &diff.unexpected_components {
        out.push_str(&format!("  unexpected component: {name}\n"));
    }
    for name in &diff.unassigned_units {
        out.push_str(&format!("  unassigned unit: {name}\n"));
    }
    for entry in &diff.ambiguous_module_matches {
        out.push_str(&format!("  ambiguous module match: {entry}\n"));
    }
    for edge in &diff.forbidden_edges {
        out.push_str(&format!("  forbidden edge: {edge}\n"));
    }
    for edge in &diff.missing_edges {
        out.push_str(&format!("  missing edge: {edge}\n"));
    }
    for edge in &diff.disallowed_cross_component {
        out.push_str(&format!(
            "  disallowed cross-component dependency: {edge}\n"
        ));
    }
    for edge in &diff.facade_dependencies {
        out.push_str(&format!("  facade dependency: {edge}\n"));
    }
    for leak in &diff.contract_leaks {
        out.push_str(&format!("  contract leak: {leak}\n"));
    }
    for cycle in &diff.cycles {
        out.push_str(&format!("  cycle: {cycle}\n"));
    }
    for leak in &diff.public_api_leaks {
        out.push_str(&format!("  public api leak: {leak}\n"));
    }
    for glob in &diff.unverifiable_glob_exports {
        out.push_str(&format!("  unverifiable glob export: {glob}\n"));
    }
    for glob in &diff.empty_glob_exports {
        out.push_str(&format!("  empty glob export: {glob}\n"));
    }
    for crate_use in &diff.forbidden_external_crates {
        out.push_str(&format!("  forbidden external crate: {crate_use}\n"));
    }
    for impurity in &diff.not_external_free {
        out.push_str(&format!("  not external free: {impurity}\n"));
    }
    for manifest in &diff.manifest_integrity {
        out.push_str(&format!("  manifest integrity: {manifest}\n"));
    }
    for boundary in &diff.feature_boundaries {
        out.push_str(&format!("  feature boundary: {boundary}\n"));
    }
    for submodule in &diff.forbidden_submodule_dependencies {
        out.push_str(&format!("  forbidden submodule dependency: {submodule}\n"));
    }
    for warning in &diff.warnings {
        if strict {
            out.push_str(&format!("  {warning}\n"));
        } else {
            out.push_str(&format!("  warning: {warning}\n"));
        }
    }
    for vacuous in &diff.vacuous_constraints {
        if strict {
            out.push_str(&format!("  vacuous constraint: {vacuous}\n"));
        } else {
            out.push_str(&format!("  warning: vacuous constraint: {vacuous}\n"));
        }
    }
    out
}
