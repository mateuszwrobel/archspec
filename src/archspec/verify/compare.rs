// Core structural compare machinery shared by `verify`, and later `report` and
// `update`. Pipeline (design §4): map extracted units onto declared components,
// compare the extracted vs declared component sets and edge sets, render the
// verdict. Diff categories are grouped in canonical report order (output.md).

use crate::archspec::capability;
use crate::archspec::language;
use crate::archspec::model::{ManifestInfo, Model, ModuleEdge, Role, Unit};
use crate::archspec::spec::{self, Allowed, Constraint, Contract, MatchSet, Module, Stereotype};
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

    /// Every error-tier field in canonical report order (ADR-010), each paired
    /// with the stable category label its entries render under (the same label
    /// `help diagnostics` catalogs and `report/output.md` declares). This is
    /// the single source of both the exit gate (`has_errors` below) and the
    /// report's error findings: a field counted by the verdict is a field the
    /// renderers print, so a non-zero exit can never coexist with a clean
    /// claim — the silent-red weld. Warning-tier fields (`warnings`,
    /// `vacuous_constraints`) never gate without `--strict` and are not
    /// listed here; the renderers consume them separately.
    pub fn error_fields(&self) -> Vec<(&'static str, &[String])> {
        vec![
            ("missing component", &self.missing_components),
            ("unexpected component", &self.unexpected_components),
            ("unassigned unit", &self.unassigned_units),
            ("ambiguous module match", &self.ambiguous_module_matches),
            ("forbidden edge present", &self.forbidden_edges),
            ("missing edge", &self.missing_edges),
            ("disallowed cross-component dependency", &self.disallowed_cross_component),
            ("facade dependency", &self.facade_dependencies),
            ("contract leak", &self.contract_leaks),
            ("cycle detected in", &self.cycles),
            ("public api leak", &self.public_api_leaks),
            ("unverifiable glob export", &self.unverifiable_glob_exports),
            ("empty glob export", &self.empty_glob_exports),
            ("forbidden external crate", &self.forbidden_external_crates),
            ("not external free", &self.not_external_free),
            ("manifest integrity", &self.manifest_integrity),
            ("feature boundary", &self.feature_boundaries),
            ("forbidden submodule dependency", &self.forbidden_submodule_dependencies),
        ]
    }

    /// True if any error-level item is present (severity defaults to error;
    /// every category except `no_cycles` is error-level by declaration). An
    /// error-level item is by definition a field of `error_fields` — there is
    /// no second list of gated fields.
    fn has_errors(&self) -> bool {
        self.error_fields()
            .iter()
            .any(|(_, entries)| !entries.is_empty())
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

/// A declared claim at any depth: a top-level `[[module]]` or a nested
/// `[[module.submodules]]` table, flattened depth-first with a parent link.
/// The jurisdiction resolver reads claims, never module tiers directly: the
/// most specific claim owns an edge's adjudication whether it sits flat or
/// nested (D1), a claim without a declared `allowed` table inherits the
/// nearest ancestor that has one (D2), ancestor prohibitions bind downward
/// cumulatively (D5), and ancestor-descendant pairs are composition (D3).
pub struct Claim<'a> {
    pub name: &'a str,
    pub matches: &'a MatchSet,
    pub allowed: &'a Allowed,
    pub contract: &'a Contract,
    pub parent: Option<usize>,
    pub depth: usize,
}

/// Minimal (name, allowed, contract) view of a flattened claim, so the
/// reference-engagement loop reads claims at any depth in one pass.
struct ModuleRef<'a> {
    name: &'a str,
    allowed: &'a Allowed,
    contract: &'a Contract,
}

/// Flatten the claim tree depth-first. Parent indices refer to this vector.
pub fn flatten_claims<'a>(modules: &'a [Module]) -> Vec<Claim<'a>> {
    fn walk<'m>(mods: &'m [Module], parent: Option<usize>, depth: usize, out: &mut Vec<Claim<'m>>) {
        for module in mods {
            let index = out.len();
            out.push(Claim {
                name: &module.name,
                matches: &module.matches,
                allowed: &module.allowed,
                contract: &module.contract,
                parent,
                depth,
            });
            walk(&module.submodules, Some(index), depth + 1, out);
        }
    }
    let mut claims = Vec::new();
    walk(modules, None, 0, &mut claims);
    claims
}

/// Every declared claim at every depth — the count the pass confirmation
/// prints, so a nested spec counts what it engages (D10).
pub fn count_claims(modules: &[Module]) -> usize {
    spec::claim_names(modules).len()
}

/// The claim owning `module_path` at any depth: highest `matches.modules`
/// specificity wins; an equal-specificity tie breaks deterministically by
/// depth (a nested claim addresses its territory more precisely than the
/// wider pattern that reaches it) and then by name, so declaration order
/// never changes a verdict (D8). Equal-specificity ties are also reported by
/// `check_ambiguous_module_matches`, which spans depths.
fn claim_owner<'a>(path: &str, claims: &[Claim<'a>]) -> Option<usize> {
    let mut best: Option<(usize, usize, &str, usize)> = None;
    for (index, claim) in claims.iter().enumerate() {
        let Some(score) = claim
            .matches
            .modules
            .iter()
            .filter_map(|pattern| entry_specificity(pattern, path))
            .max()
        else {
            continue;
        };
        match best {
            Some((b_score, b_depth, b_name, _))
                if score < b_score
                    || (score == b_score && claim.depth == b_depth && claim.name >= b_name)
                    || (score == b_score && claim.depth < b_depth) => {}
            _ => best = Some((score, claim.depth, claim.name, index)),
        }
    }
    best.map(|(_, _, _, index)| index)
}

/// The unit-fallback owner of a module path no `matches.modules` pattern
/// claims: the FIRST top-level claim whose `matches.units` globs name the
/// path's unit — first-declared-wins, unchanged (submodules claim the module
/// tier only, D7).
fn claim_unit_fallback(path: &str, claims: &[Claim<'_>]) -> Option<usize> {
    let unit = path.split("::").next().unwrap_or(path);
    claims.iter().enumerate().find_map(|(index, claim)| {
        (claim.depth == 0
            && claim
                .matches
                .units
                .iter()
                .any(|pattern| matches_pattern(unit, pattern)))
        .then_some(index)
    })
}

/// The dependency policy governing a claim: its own declared `allowed` table,
/// else the nearest ancestor's (D2); the root without any declared table
/// keeps today's implicit deny-all ceiling.
fn policy_allowed<'a>(claims: &[Claim<'a>], index: usize) -> &'a Allowed {
    let mut current = Some(index);
    while let Some(at) = current {
        if claims[at].allowed.declared {
            return claims[at].allowed;
        }
        current = claims[at].parent;
    }
    claims[index].allowed
}

/// Cumulative prohibitions for a claim: its own `allowed.forbidden` plus every
/// ancestor's — a descendant may narrow allowances but can never cancel an
/// ancestor prohibition (D5, the one-way ratchet).
fn cumulative_forbidden<'a>(claims: &[Claim<'a>], index: usize) -> BTreeSet<&'a str> {
    let mut forbidden = BTreeSet::new();
    let mut current = Some(index);
    while let Some(at) = current {
        forbidden.extend(claims[at].allowed.forbidden.iter().map(String::as_str));
        current = claims[at].parent;
    }
    forbidden
}

/// True when one claim is the other's direct-or-transitive ancestor — the
/// containment relation whose edges are composition, never dependencies (D3).
fn related_by_containment(claims: &[Claim<'_>], a: usize, b: usize) -> bool {
    let mut current = Some(b);
    while let Some(at) = current {
        if at == a {
            return true;
        }
        current = claims[at].parent;
    }
    let mut current = Some(a);
    while let Some(at) = current {
        if at == b {
            return true;
        }
        current = claims[at].parent;
    }
    false
}

/// The claim subtree: the claim itself plus every nested descendant.
fn subtree_names<'a>(claims: &[Claim<'a>], index: usize) -> BTreeSet<&'a str> {
    let mut names = BTreeSet::new();
    for (at, claim) in claims.iter().enumerate() {
        let mut current = Some(at);
        while let Some(step) = current {
            if step == index {
                names.insert(claim.name);
                break;
            }
            current = claims[step].parent;
        }
    }
    names
}

fn claim_by_name<'a>(claims: &[Claim<'a>], name: &str) -> Option<usize> {
    claims.iter().position(|claim| claim.name == name)
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
fn check_ambiguous_module_matches(model: &Model, claims: &[Claim<'_>]) -> Vec<String> {
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
        let claims: Vec<(usize, &str, &str)> = claims
            .iter()
            .filter_map(|claim| {
                claim
                    .matches
                    .modules
                    .iter()
                    .filter_map(|pattern| {
                        entry_specificity(pattern, path).map(|score| (score, pattern.as_str()))
                    })
                    .max_by_key(|(score, _)| *score)
                    .map(|(score, entry)| (score, claim.name, entry))
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

/// Claim-named dependency edges for `forbid_submodule_dependency`. Trees with
/// a native module tier already carry real module edges. A single go.mod tree
/// whose claims partition its packages has the derived tier: unit edges become
/// owner edges named by the claims, so `common_ancestor` yields the declared
/// parent and the check reads the same shape as on a rust tree. Edges inside
/// one group drop out, matching the native derivation.
fn submodule_dependency_edges(model: &Model, mapping: &Mapping) -> Vec<ModuleEdge> {
    if model.language != "go" {
        return model.module_edges.clone();
    }
    let owner = |endpoint: &str| -> String {
        mapping
            .unit_module
            .get(endpoint)
            .cloned()
            .unwrap_or_else(|| endpoint.to_string())
    };
    let mut projected: BTreeSet<(String, String)> = BTreeSet::new();
    for edge in &model.module_edges {
        let from = owner(&edge.from);
        let to = owner(&edge.to);
        if from != to {
            projected.insert((from, to));
        }
    }
    projected
        .into_iter()
        .map(|(from, to)| ModuleEdge {
            unit: from.clone(),
            from,
            to,
            symbols: Vec::new(),
        })
        .collect()
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

    let claims = flatten_claims(modules);
    // A top-level claim that declares only module paths and matched nothing is
    // engagement surface, not a missing component: it never named a unit-tier
    // component to begin with. It reports as the `unmatched claim` warning
    // below, at either depth, so nested and flat equivalents agree.
    let unit_backed: BTreeSet<&str> = modules
        .iter()
        .filter(|module| !module.matches.units.is_empty() || module.matches.modules.is_empty())
        .map(|module| module.name.as_str())
        .collect();
    let mut diff = Diff {
        missing_components: declared
            .difference(&extracted)
            .filter(|name| unit_backed.contains(*name))
            .map(|name| name.to_string())
            .collect(),
        unexpected_components: mapping.unexpected_components.clone(),
        unassigned_units: mapping.unassigned_units.clone(),
        ambiguous_module_matches: check_ambiguous_module_matches(model, &claims),
        ..Default::default()
    };

    for paths in model.unresolved_module_files.values() {
        for path in paths {
            diff.warnings
                .push(format!("unresolved module file: {path}"));
        }
    }
    let composition_sources = composition_sources(model);
    let graph = resolve_module_edges(model, &mapping, modules, &claims, &composition_sources);
    for path in &graph.unowned_endpoints {
        diff.warnings
            .push(format!("unowned module edge endpoint: {path}"));
    }
    check_facade_roots(model, &mut diff);
    check_allowed_edges(&graph.pairs, model, modules, &claims, &mut diff);
    check_missing_edges(&graph.pairs, &claims, &mut diff);
    let missing_components: BTreeSet<String> = diff.missing_components.iter().cloned().collect();
    check_claim_engagement(model, &claims, &missing_components, &mut diff.warnings);
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

    // The expose chain law (workplan
    // rust-arch-test-kit/worklog/workplan_archspec_module_public_api_diagrams.md):
    // leaks join the shipped `public api leak` category, staleness/ambiguity/
    // dead-reference lines join `warnings`, and the per-claim fact-source
    // vacuity rides the `vacuous constraint` channel below (D7). Verdict-
    // neutral when no claim declares a set (D2) — every vector stays empty.
    let expose = adjudicate_expose(&mapping, model, modules, stereotypes);
    diff.public_api_leaks.extend(expose.leaks);
    diff.public_api_leaks.sort();
    diff.warnings.extend(expose.warnings);
    let mut expose_vacuous = expose.vacuous;

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

    let submodule_edges = submodule_dependency_edges(model, &mapping);
    let (submodules, mut warning_submodules) = check_forbid_submodule(&submodule_edges, constraints);
    diff.forbidden_submodule_dependencies = submodules;
    diff.warnings.append(&mut warning_submodules);

    diff.warnings.append(&mut check_reference_engagement(
        model,
        modules,
        stereotypes,
        soft_verifiable,
    ));

    diff.vacuous_constraints =
        check_vacuous_constraints(model, modules, constraints, &graph.pairs, &submodule_edges);
    diff.vacuous_constraints.append(&mut expose_vacuous);
    diff.vacuous_constraints.sort();

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
    claims: &[Claim<'_>],
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
        match place_module_edge(edge, model, claims, modules, composition_sources) {
            EdgePlacement::Pair {
                from,
                to,
                fallback,
                source_is_composition,
            } => {
                if from != to {
                    let pair = (from.to_string(), to.to_string());
                    if fallback && !source_is_composition {
                        fallback_pairs.insert(pair.clone());
                    }
                    pairs.insert(pair);
                }
            }
            EdgePlacement::Composition => {}
            EdgePlacement::Unowned { paths } => {
                unowned_endpoints.extend(paths);
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
    claims: &[Claim<'_>],
    diff: &mut Diff,
) {
    for (from_module, to_module) in pairs {
        match pair_verdict(from_module, to_module, model, modules, claims) {
            Some(Verdict::Forbidden) => diff
                .forbidden_edges
                .push(format!("{from_module} -> {to_module}")),
            Some(Verdict::Disallowed) => diff
                .disallowed_cross_component
                .push(format!("{from_module} -> {to_module}")),
            None => {}
        }
    }
}

/// The verdict class a boundary pair earns, or `None` when no rule reds it.
/// This is the one adjudication `verify`'s edge findings and the diagram's
/// mark decision read — the seam the scan-diagram module-tier plan pins
/// (ADR-018 identity ownership + ADR-020 jurisdiction): the diagram marks
/// exactly what `verify` would report, never a re-derivation. A pair whose
/// source names no claim is adjudicated by nothing and stays `None`. An
/// explicitly forbidden pair is never also disallowed (finding #5 wins).
pub enum Verdict {
    /// Banned by the source's cumulative `allowed.forbidden` (finding #5).
    Forbidden,
    /// Outside the policy ceiling — the class verify renders as
    /// `disallowed cross-component dependency` (finding #6).
    Disallowed,
}

fn pair_verdict(
    from_module: &str,
    to_module: &str,
    model: &Model,
    modules: &[Module],
    claims: &[Claim<'_>],
) -> Option<Verdict> {
    let index = claim_by_name(claims, from_module)?;
    // The ratchet (D5): every ancestor prohibition binds this claim's
    // edges; the policy ceiling (D2) is the claim's own declared table or
    // the nearest ancestor's.
    if cumulative_forbidden(claims, index).contains(to_module) {
        return Some(Verdict::Forbidden);
    }
    if !policy_allowed(claims, index)
        .depend_on
        .iter()
        .any(|target| target == to_module)
        && !unit_tier_identity_allows(from_module, to_module, model, modules)
    {
        return Some(Verdict::Disallowed);
    }
    None
}


/// The adjudication seam consumed by `verify`'s findings and the diagram's
/// scan-source marks and node names (workplan
/// workplan_archspec_scan_diagram_module_tier, Assumptions + Internal
/// Architecture): `(model edge, governing spec) → verdict class + owning
/// claim` over one resolution path — `map_units` → `augment_with_modules` →
/// `effective_module_tier` → flattened claims — exactly the pipeline `compare`
/// adjudicates with, so a diagram never states a verdict `verify` does not
/// reach (ADR-018 + ADR-020). Specless renders pass empty modules: with no
/// spec there is nothing to adjudicate and every edge lands unowned.
pub struct Adjudicator<'a> {
    claims: Vec<Claim<'a>>,
    mapping: Mapping,
    modules: &'a [Module],
    /// Pairs carrying a verdict (forbidden or disallowed) over the boundary
    /// graph — the mark set the diagram reads.
    violating: BTreeSet<(String, String)>,
    composition_sources: BTreeSet<String>,
    /// The derived model (`effective_module_tier`), exactly the one verify's
    /// `resolve_module_edges` places edges against.
    derived: Cow<'a, Model>,
    /// True when adjudication addresses module paths with `/` separators
    /// (the go native-tier projection of `effective_module_tier`); artefact
    /// paths arrive in the drivers' `::` spelling and are projected.
    slashed_paths: bool,
    /// The expose chain verdicts for the governing spec — the widened seam
    /// query (D11): the per-module diagram's public-API marks read this set,
    /// never a diagram-side re-derivation. A spec declaring no expose anywhere
    /// yields the neutral verdicts, so the coarse render carries no API marks.
    expose: ExposeVerdicts,
}

impl<'a> Adjudicator<'a> {
    /// Build the seam for one (model, governing spec) pair — the same
    /// preprocessing `compare` runs, so every verdict matches `verify`'s.
    pub fn new(model: &'a Model, modules: &'a [Module], stereotypes: &'a [Stereotype]) -> Self {
        let base = map_units(&model.units, modules);
        let mapping = augment_with_modules(&base, model, modules);
        let derived = effective_module_tier(model, &mapping);
        let claims = flatten_claims(modules);
        let composition_sources = composition_sources(&derived);
        let graph =
            resolve_module_edges(&derived, &mapping, modules, &claims, &composition_sources);
        let violating = graph
            .pairs
            .iter()
            .filter(|(from, to)| pair_verdict(from, to, &derived, modules, &claims).is_some())
            .cloned()
            .collect();
        let slashed_paths = derived.language == "go" && derived.has_module_tier();
        let expose = adjudicate_expose(&mapping, &derived, modules, stereotypes);
        Self {
            claims,
            mapping,
            modules,
            violating,
            composition_sources,
            derived,
            slashed_paths,
            expose,
        }
    }

    /// Project an artefact module path into the adjudication path space.
    fn adjudication_path<'b>(&self, path: &'b str) -> Cow<'b, str> {
        if self.slashed_paths {
            Cow::Owned(path.replace("::", "/"))
        } else {
            Cow::Borrowed(path)
        }
    }

    /// True when the unit-tier edge resolves to a boundary pair `verify`
    /// reds — the same pair its `forbidden edge` / `disallowed cross-component
    /// dependency` findings name (the pair law, both flag states). Edges that
    /// resolve inside one boundary, or whose source resolves nowhere, carry no
    /// verdict and stay solid.
    pub fn unit_edge_violates(&self, from: &str, to: &str) -> bool {
        let (Some(from), Some(to)) = (
            self.mapping.unit_module.get(from),
            self.mapping.unit_module.get(to),
        ) else {
            return false;
        };
        from != to && self.violating.contains(&(from.clone(), to.clone()))
    }

    /// The boundary claim naming a module path's adjudication — the owning
    /// claim whose `allowed` table judges its edges (the node-identity rule),
    /// or `None` when no claim owns the endpoint.
    pub fn claim_naming(&self, path: &str) -> Option<&'a str> {
        let path = self.adjudication_path(path);
        claim_owner(&path, &self.claims)
            .or_else(|| claim_unit_fallback(&path, &self.claims))
            .map(|index| self.claims[index].name)
    }

    /// True when the governing spec's expose law leaks any public symbol of
    /// the module at this path — the seam answer the per-module diagram's
    /// public-API mark reads (D11): dashed exactly when `verify` reds a
    /// crossing of this source. False everywhere the spec declares no expose
    /// (the coarse render carries no API marks; `declares_expose` states that
    /// the view is not an API-declaring authority).
    pub fn api_leak_mark(&self, from_path: &str) -> bool {
        if self.expose.leaked_by_claim.is_empty() {
            return false;
        }
        let path = self.adjudication_path(from_path);
        // The adjudication rows (D2): the source module's public tier minus
        // unit-root keys, which the unit tier owns. The mark fires when the
        // claim governing this path leaks a symbol this row holds — the same
        // symbol `verify` names in its `public api leak` finding. The verdict
        // is the source module's surface law, so the mark rides the source
        // row, not the edge's target-side symbol names.
        let is_unit_root = self
            .derived
            .units
            .iter()
            .any(|unit| unit.name == from_path);
        let row = if is_unit_root {
            None
        } else {
            self.derived.module_public_types.get(from_path)
        };
        self.expose.leaked_by_claim.iter().any(|(claim, leaked)| {
            !leaked.is_empty()
                && self
                    .claims
                    .iter()
                    .any(|c| c.name == claim.as_str() && claim_covers_path(c, &path, &self.derived))
                && row.is_some_and(|names| names.iter().any(|name| leaked.contains(name)))
        })
    }

    /// The module paths of one named claim's territory — own unit roots
    /// excluded (D10), every path the model shows that the claim covers by
    /// `matches` or by name nesting. The per-module render's node set (D11);
    /// an unknown claim name yields nothing (the command validates first).
    pub fn territory_paths(&self, claim: &str) -> BTreeSet<String> {
        let Some(owner) = self.claims.iter().find(|c| c.name == claim) else {
            return BTreeSet::new();
        };
        let mut paths: BTreeSet<String> = BTreeSet::new();
        for edge in &self.derived.module_edges {
            for path in [&edge.from, &edge.to] {
                let projected = self.adjudication_path(path);
                if claim_covers_path(owner, &projected, &self.derived) {
                    paths.insert(path.clone());
                }
            }
        }
        for path in self.derived.module_public_types.keys() {
            let projected = self.adjudication_path(path);
            if claim_covers_path(owner, &projected, &self.derived) {
                paths.insert(path.clone());
            }
        }
        for (unit, structure) in &self.derived.soft_structure {
            for path in structure {
                if path == unit {
                    continue; // the unit root row is the allowlist's territory (D10)
                }
                let projected = self.adjudication_path(path);
                if claim_covers_path(owner, &projected, &self.derived) {
                    paths.insert(path.clone());
                }
            }
        }
        paths
    }

    /// The claims beneath one named claim (name-prefixed submodules), for the
    /// per-module render's nested nodes.
    pub fn subclaims_of(&self, claim: &str) -> Vec<&'a str> {
        let prefix = format!("{claim}::");
        self.claims
            .iter()
            .filter(|c| c.name != claim && c.name.starts_with(&prefix))
            .map(|c| c.name)
            .collect()
    }

    /// Where an artefact module edge lands, adjudicated through the placement
    /// law `verify`'s boundary graph is built from. Claim-named endpoints
    /// arrive by their owner's name — the name `verify`'s findings use — and
    /// unowned ones path-named; `violates` reads the same pair set `verify`
    /// reports. Placement never re-derives: test-gated, containment and
    /// same-claim pairs are composition exactly as `verify` sees them.
    pub fn module_edge_placement(&self, edge: &'a ModuleEdge) -> ModuleRender<'a> {
        let projected_from = self.adjudication_path(&edge.from);
        let projected_to = self.adjudication_path(&edge.to);
        let projected = ModuleEdge {
            unit: self.adjudication_path(&edge.unit).into_owned(),
            from: projected_from.into_owned(),
            to: projected_to.into_owned(),
            symbols: edge.symbols.clone(),
        };
        match place_module_edge(
            &projected,
            &self.derived,
            &self.claims,
            self.modules,
            &self.composition_sources,
        ) {
            EdgePlacement::Pair { from, to, .. } => ModuleRender::Arrow {
                from,
                from_path: &edge.from,
                to,
                to_path: &edge.to,
                violates: self.violating.contains(&(from.to_string(), to.to_string())),
                api_mark: self.api_leak_mark(&edge.from),
            },
            EdgePlacement::Composition => ModuleRender::Nesting {
                from: self.claim_naming(&edge.from).unwrap_or(edge.from.as_str()),
                from_path: &edge.from,
                to: self.claim_naming(&edge.to).unwrap_or(edge.to.as_str()),
                to_path: &edge.to,
            },
            EdgePlacement::Unowned { .. } => ModuleRender::Arrow {
                from: self.claim_naming(&edge.from).unwrap_or(edge.from.as_str()),
                from_path: &edge.from,
                to: self.claim_naming(&edge.to).unwrap_or(edge.to.as_str()),
                to_path: &edge.to,
                violates: false,
                api_mark: self.api_leak_mark(&edge.from),
            },
        }
    }
}

/// The diagram-facing shape of the placement law: where the arrow goes,
/// under which node identities, carrying each endpoint's raw module path for
/// the dual label, and whether the pair carries a verdict (workplan
/// workplan_archspec_scan_diagram_module_tier, node-identity, one-arrow and
/// one-mark-vocabulary decisions).
pub enum ModuleRender<'a> {
    /// An arrow between the two named nodes — solid when `violates` is false,
    /// dashed `violates spec` when true.
    Arrow {
        from: &'a str,
        from_path: &'a str,
        to: &'a str,
        to_path: &'a str,
        violates: bool,
        /// Public-API mark from the seam (see `api_leak_mark`): the source
        /// module holds a public symbol the expose law leaked. Dashed
        /// rendering with `violates`.
        api_mark: bool,
    },
    /// Composition or attribution artefact: the diagram draws no arrow; the
    /// endpoints are still nodes of the module tier.
    Nesting {
        from: &'a str,
        from_path: &'a str,
        to: &'a str,
        to_path: &'a str,
    },
}
/// Where one extracted module edge lands on the boundary graph — the single
/// placement law `resolve_module_edges` (verify's findings) and the diagram's
/// module-tier render both consult, so a drawn arrow and an adjudicated pair
/// can never disagree (same edge, same verdict). The arms mirror the
/// original `resolve_module_edges` loop: containment pairs, same-claim pairs
/// and misattributed sanctioned hops are composition, not dependencies;
/// unowned endpoints adjudicate nothing and surface in `unowned_endpoints`.
pub enum EdgePlacement<'a> {
    /// A cross-boundary pair on the boundary graph, named by claim.
    Pair {
        from: &'a str,
        to: &'a str,
        /// At least one endpoint was attributed through the unit fallback.
        fallback: bool,
        /// The `from` path carries the composition role.
        source_is_composition: bool,
    },
    /// One claim owns both endpoints, or the pair is containment-related, or
    /// it is the composition-attribution artefact, or a test-gated endpoint:
    /// never an edge, never a verdict.
    Composition,
    /// An endpoint no claim owns: path-named, adjudicated nothing.
    Unowned { paths: Vec<String> },
}

fn place_module_edge<'a>(
    edge: &ModuleEdge,
    model: &Model,
    claims: &[Claim<'a>],
    modules: &[Module],
    composition_sources: &BTreeSet<String>,
) -> EdgePlacement<'a> {
    if is_test_module(model, &edge.from) || is_test_module(model, &edge.to) {
        return EdgePlacement::Composition;
    }
    let from_owner = claim_owner(&edge.from, claims)
        .map(|index| (index, false))
        .or_else(|| claim_unit_fallback(&edge.from, claims).map(|index| (index, true)));
    let to_owner = claim_owner(&edge.to, claims)
        .map(|index| (index, false))
        .or_else(|| claim_unit_fallback(&edge.to, claims).map(|index| (index, true)));
    let mut unowned: Vec<String> = Vec::new();
    if from_owner.is_none() {
        unowned.push(edge.from.clone());
    }
    if to_owner.is_none() {
        unowned.push(edge.to.clone());
    }
    let (Some(from_owner), Some(to_owner)) = (from_owner, to_owner) else {
        return EdgePlacement::Unowned { paths: unowned };
    };
    // Containment (D3): both endpoints resolve inside one claim subtree along
    // an ancestor-descendant pair — a parent reaching its own parts, either
    // direction, is composition. The edge neither adjudicates nor launders:
    // it is not an edge at all on the boundary graph.
    if related_by_containment(claims, from_owner.0, to_owner.0) {
        return EdgePlacement::Composition;
    }
    if claims[from_owner.0].name == claims[to_owner.0].name {
        return EdgePlacement::Composition;
    }
    let source_is_composition = composition_sources.contains(&edge.from);
    if source_is_composition
        && from_owner.1
        && !modules.iter().any(|module| {
            module.name == claims[from_owner.0].name
                && module
                    .matches
                    .units
                    .iter()
                    .any(|pattern| matches_pattern(&edge.unit, pattern))
        })
    {
        // Misattributed sanctioned hop: the composition path was projected
        // onto a boundary whose `units` do not cover the unit the wiring edge
        // actually lives in (rollup splits multi-segment unit roots at the
        // first `::`, so a solution root `Company::App` lands on the
        // neighbour claiming `Company`). That pair is an artefact of
        // attribution, not an observed dependency — it neither adjudicates
        // nor launders (roles US 05b).
        return EdgePlacement::Composition;
    }
    EdgePlacement::Pair {
        from: claims[from_owner.0].name,
        to: claims[to_owner.0].name,
        fallback: from_owner.1 || to_owner.1,
        source_is_composition,
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
/// claim, but no extracted edge connects the declaring claim's territory —
/// its whole subtree, so an edge owned by a policy-inheriting descendant
/// satisfies the ancestor's obligation exactly once — to that claim. A
/// policy-free nested claim adds no obligation of its own (D2): its
/// inherited rule fires at the declaring ancestor, so nested and flat
/// equivalents list the same pair with the same text.
fn check_missing_edges(pairs: &BTreeSet<(String, String)>, claims: &[Claim<'_>], diff: &mut Diff) {
    for (index, claim) in claims.iter().enumerate() {
        if !claim.allowed.declared {
            continue;
        }
        let territory = subtree_names(claims, index);
        for target in &claim.allowed.depend_on {
            let target_declared = claim_by_name(claims, target).is_some();
            let exercised = pairs
                .iter()
                .any(|(from, to)| to == target && territory.contains(from.as_str()));
            if target_declared && !exercised {
                diff.missing_edges
                    .push(format!("{} -> {target}", claim.name));
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
/// the same `module_path_matches` rule as module ownership) and the units the
/// enclosing claim has assigned, filtered to those the submodule's boundary
/// path matches (submodules claim the module tier only, so the unit surface
/// is always the ancestor's claimed territory seen through the boundary) —
/// matching stereotype patterns against those strings by the
/// full-path/bare-name rule.
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
        let inherited: BTreeSet<&str> = module.contract.forbid.iter().map(String::as_str).collect();
        let empty_units = BTreeSet::new();
        let ancestor_units: &BTreeSet<String> =
            mapping.assignments.get(&module.name).unwrap_or(&empty_units);
        collect_submodule_contract_leaks(
            model,
            &module.submodules,
            &inherited,
            ancestor_units,
            stereotypes,
            &mut leaks,
        );
    }
    leaks.into_iter().collect()
}

/// Recurse the contract-leak check into submodules (one level per nesting,
/// boundary = the submodule's declared path patterns). Ancestors'
/// `contract.forbid` lists bind every descendant's surface cumulatively —
/// the prohibition half of the one-way ratchet (D5); the leak is attributed
/// to the claim whose files expose the forbidden stereotype.
fn collect_submodule_contract_leaks<'a>(
    model: &Model,
    submodules: &'a [Module],
    inherited: &BTreeSet<&'a str>,
    ancestor_units: &BTreeSet<String>,
    stereotypes: &[Stereotype],
    leaks: &mut BTreeSet<String>,
) {
    for submodule in submodules {
        let boundary: Vec<String> = if submodule.matches.modules.is_empty() {
            vec![submodule.name.clone()]
        } else {
            submodule.matches.modules.clone()
        };
        let mut forbids: BTreeSet<&str> = inherited.clone();
        forbids.extend(submodule.contract.forbid.iter().map(String::as_str));
        for forbidden in &forbids {
            let exposed = stereotypes.iter().any(|stereotype| {
                &stereotype.name == forbidden
                    && submodule_surface_matches(stereotype, &boundary, ancestor_units, model)
            });
            if exposed {
                leaks.insert(format!(
                    "{} exposes {forbidden} (forbidden)",
                    submodule.name
                ));
            }
        }
        collect_submodule_contract_leaks(
            model,
            &submodule.submodules,
            &forbids,
            ancestor_units,
            stereotypes,
            leaks,
        );
    }
}

/// The expose chain law's verdict set (workplan
/// rust-arch-test-kit/worklog/workplan_archspec_module_public_api_diagrams.md):
/// leaks are `<owner> exposes <name> (not exposed)` payloads for the shipped
/// `public api leak` category; staleness, ambiguity, and dead-reference lines
/// are warning payloads; `vacuous` entries are pre-rendered carriers for the
/// `vacuous constraint` channel; `leaked_by_claim` is the widened seam query
/// the per-module diagram reads (D11) — the diagram never re-derives.
pub struct ExposeVerdicts {
    pub leaks: Vec<String>,
    pub warnings: Vec<String>,
    pub vacuous: Vec<String>,
    pub leaked_by_claim: BTreeMap<String, BTreeSet<String>>,
}

/// True when `claim`'s territory contains module path `path`: a declared
/// `matches.modules` pattern addresses it, or a claimed unit owns it — the
/// unit itself (its root module, excluded from the fact read by D10 before
/// this call) or any `unit::…` path beneath it.
fn claim_covers_path(claim: &Claim<'_>, path: &str, model: &Model) -> bool {
    claim
        .matches
        .modules
        .iter()
        .any(|pattern| module_path_matches(pattern, path))
        || model.units.iter().any(|unit| {
            claim
                .matches
                .units
                .iter()
                .any(|pattern| matches_pattern(&unit.name, pattern))
                && (path == unit.name || path.starts_with(&format!("{}::", unit.name)))
        })
}

/// The public type names a claim's territory exports — the module public-API
/// fact rows it covers, unit-root rows already excluded (D10).
fn territory_symbols(claim: &Claim<'_>, rows: &[(String, Vec<String>)], model: &Model) -> Vec<String> {
    rows.iter()
        .filter(|(path, _)| claim_covers_path(claim, path, model))
        .flat_map(|(_, names)| names.iter().cloned())
        .collect()
}

/// True when a stereotype classifies any of the claim's own surface: the
/// claimed units through the shipped unit-level rule, or the module paths
/// beneath the claim's boundary through the shipped submodule-surface rule
/// (the same machinery `contract.forbid` reads — assumption A5).
fn stereotype_engages_claim(
    claim: &Claim<'_>,
    claims: &[Claim<'_>],
    stereotype: &Stereotype,
    mapping: &Mapping,
    model: &Model,
) -> bool {
    let own_units: BTreeSet<String> = if let Some(units) = mapping.assignments.get(claim.name) {
        units.clone()
    } else if let Some(parent) = claim.parent {
        mapping
            .assignments
            .get(claims[parent].name)
            .cloned()
            .unwrap_or_default()
    } else {
        BTreeSet::new()
    };
    if own_units
        .iter()
        .any(|unit| stereotype_matches_unit(unit, stereotype))
    {
        return true;
    }
    let boundary: Vec<String> = if claim.matches.modules.is_empty() {
        vec![claim.name.to_string()]
    } else {
        claim.matches.modules.clone()
    };
    submodule_surface_matches(stereotype, &boundary, &own_units, model)
}

/// The expose chain law over one (model, claims) pair. A claim that declares
/// no `contract.expose` set makes no claim (verdict-neutral, D2); a declaring
/// ancestor binds every descendant export (the set at a boundary is the
/// intersection over the declaring chain); a symbol fires once, attributed to
/// the deepest declaring claim whose territory holds it (D3). The fact read is
/// the module public-API rows minus the unit-root keys (D10: those rows are
/// the allowlist's sole territory); a driver that carries no such rows leaves
/// the symbol arm vacuous per claim (D7) while the stereotype arm still
/// adjudicates through the shipped surface machinery.
pub fn adjudicate_expose(
    mapping: &Mapping,
    model: &Model,
    modules: &[Module],
    stereotypes: &[Stereotype],
) -> ExposeVerdicts {
    let claims = flatten_claims(modules);
    let declaring: Vec<&Claim<'_>> = claims
        .iter()
        .filter(|claim| claim.contract.expose.is_some())
        .collect();
    if declaring.is_empty() {
        return ExposeVerdicts {
            leaks: Vec::new(),
            warnings: Vec::new(),
            vacuous: Vec::new(),
            leaked_by_claim: BTreeMap::new(),
        };
    }
    let unit_names: BTreeSet<&str> = model.units.iter().map(|unit| unit.name.as_str()).collect();
    let rows: Vec<(String, Vec<String>)> = model
        .module_public_types
        .iter()
        .filter(|(key, _)| !unit_names.contains(key.as_str()))
        .map(|(key, names)| (key.clone(), names.clone()))
        .collect();
    let facts_present = !rows.is_empty();
    let declared_stereotypes: BTreeSet<&str> =
        stereotypes.iter().map(|s| s.name.as_str()).collect();

    let mut leaks: BTreeSet<String> = BTreeSet::new();
    let mut warnings: BTreeSet<String> = BTreeSet::new();
    let mut vacuous: BTreeSet<String> = BTreeSet::new();
    let mut leaked_by_claim: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    // Chain adjudication, one pass per symbol row (D2/D3).
    for (path, names) in &rows {
        let chain: Vec<&Claim<'_>> = declaring
            .iter()
            .copied()
            .filter(|claim| claim_covers_path(claim, path, model))
            .collect();
        if chain.is_empty() {
            continue;
        }
        let owner = chain
            .iter()
            .copied()
            .max_by_key(|claim| (claim.depth, std::cmp::Reverse(claim.name)))
            .expect("chain non-empty");
        for name in names {
            let uncovered = chain.iter().any(|claim| !expose_covers(claim, name, stereotypes));
            if uncovered {
                leaks.insert(format!("{} exposes {name} (not exposed)", owner.name));
                leaked_by_claim
                    .entry(owner.name.to_string())
                    .or_default()
                    .insert(name.clone());
            }
        }
    }

    // Per-entry classification over each declaring claim (D4/D5): an entry
    // naming a declared stereotype adjudicates the stereotype arm; a glob is
    // a symbol pattern over facts; a bare name is a symbol reference whose
    // non-existence anywhere is a shipped dead reference (forbid parity —
    // globs name no thing, so they take only the staleness ladder).
    let all_symbols: Vec<&String> = rows.iter().flat_map(|(_, names)| names.iter()).collect();
    for claim in &declaring {
        let entries = claim.contract.expose.as_ref().expect("filter declares");
        let territory = territory_symbols(claim, &rows, model);
        let mut symbol_arm_entries = false;
        for entry in entries {
            match declared_stereotypes
                .get(entry.as_str())
                .and_then(|name| stereotypes.iter().find(|s| &s.name == name))
            {
                Some(stereotype) => {
                    let engages =
                        stereotype_engages_claim(claim, &claims, stereotype, mapping, model);
                    if !engages {
                        warnings.insert(format!(
                            "stale contract export: {} declares {entry} (never exported across its territory)",
                            claim.name
                        ));
                    }
                    let ambiguous = territory.iter().any(|symbol| {
                        stereotype
                            .match_set
                            .names
                            .iter()
                            .chain(stereotype.match_set.paths.iter())
                            .chain(stereotype.match_set.units.iter())
                            .any(|pattern| matches_pattern(symbol, pattern))
                    });
                    if ambiguous {
                        warnings.insert(format!(
                            "ambiguous expose entry: {} entry {entry} names stereotype {} and matches symbols",
                            claim.name, stereotype.name
                        ));
                    }
                }
                None => {
                    symbol_arm_entries = true;
                    if !facts_present {
                        continue; // the arm states its own vacuity below (D7)
                    }
                    let engaged = territory
                        .iter()
                        .any(|symbol| matches_pattern(symbol, entry));
                    if engaged {
                        continue;
                    }
                    if entry.contains('*')
                        || all_symbols.iter().any(|symbol| matches_pattern(symbol, entry))
                    {
                        warnings.insert(format!(
                            "stale contract export: {} declares {entry} (never exported across its territory)",
                            claim.name
                        ));
                    } else {
                        warnings.insert(format!(
                            "dead reference: module '{}' contract.expose target \"{entry}\" names no \
                             stereotype — declare a stereotype named \"{entry}\" or fix the reference{}",
                            claim.name,
                            did_you_mean(entry, &declared_stereotypes)
                        ));
                    }
                }
            }
        }
        if !facts_present && symbol_arm_entries {
            vacuous.insert(format!(
                "[contract.expose of {}]: no module-tier public-API facts to check for the \
                 public-API surface (see archspec help capabilities)",
                claim.name
            ));
        }
    }

    ExposeVerdicts {
        leaks: leaks.into_iter().collect(),
        warnings: warnings.into_iter().collect(),
        vacuous: vacuous.into_iter().collect(),
        leaked_by_claim,
    }
}

/// True when the claim's declared set covers symbol `name`: a symbol-level
/// entry matching through the shipped glob (D6), or a stereotype entry whose
/// class the symbol falls under (the entry's arm decided classification, not
/// polarity — both arms permit).
fn expose_covers(claim: &Claim<'_>, name: &str, stereotypes: &[Stereotype]) -> bool {
    let entries = match claim.contract.expose.as_ref() {
        Some(entries) => entries,
        None => return false,
    };
    entries.iter().any(|entry| {
        if let Some(stereotype) = stereotypes.iter().find(|s| s.name == *entry) {
            return stereotype
                .match_set
                .names
                .iter()
                .chain(stereotype.match_set.paths.iter())
                .chain(stereotype.match_set.units.iter())
                .any(|pattern| matches_pattern(name, pattern));
        }
        matches_pattern(name, entry)
    })
}

/// Engagement diagnostics over the claim set at any depth (D9a/D9b), run
/// model-side so languages without module-tier facts are covered too:
/// (a) a claim whose `matches` resolve to no model element is an
///     `unmatched claim` — except where the missing-component error already
///     names it, so a claim is never reported twice;
/// (b) a claim whose every claimed model path an equal-or-more-specific
///     other claim also covers is a `shadowed claim` — its rules can never
///     fire differently than the shadow's, exactly the silence this whole
///     change exists to close.
fn check_claim_engagement(
    model: &Model,
    claims: &[Claim<'_>],
    missing_components: &BTreeSet<String>,
    warnings: &mut Vec<String>,
) {
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    for module_paths in model.soft_structure.values() {
        paths.extend(module_paths.iter().map(String::as_str));
    }
    for edge in &model.module_edges {
        paths.insert(edge.from.as_str());
        paths.insert(edge.to.as_str());
    }
    let claims_a_path = |pattern: &str, path: &str| module_path_matches(pattern, path);
    for claim in claims {
        let matched = paths.iter().any(|path| {
            claim
                .matches
                .modules
                .iter()
                .any(|pattern| claims_a_path(pattern, path))
        }) || model.units.iter().any(|unit| {
            claim
                .matches
                .units
                .iter()
                .any(|pattern| matches_pattern(&unit.name, pattern))
                || (model.language == "go"
                    && claim
                        .matches
                        .modules
                        .iter()
                        .any(|pattern| module_path_matches(pattern, &unit.name)))
        });
        if !matched && !missing_components.contains(claim.name) {
            warnings.push(format!("unmatched claim: {}", claim.name));
        }
    }
    for first in claims {
        let claimed: Vec<&str> = paths
            .iter()
            .copied()
            .filter(|path| {
                first
                    .matches
                    .modules
                    .iter()
                    .any(|pattern| claims_a_path(pattern, path))
            })
            .collect();
        if claimed.is_empty() {
            continue;
        }
        for other in claims {
            if other.name == first.name {
                continue;
            }
            let covers = claimed.iter().all(|path| {
                let mine = first
                    .matches
                    .modules
                    .iter()
                    .filter_map(|pattern| entry_specificity(pattern, path))
                    .max();
                let theirs = other
                    .matches
                    .modules
                    .iter()
                    .filter_map(|pattern| entry_specificity(pattern, path))
                    .max();
                match (mine, theirs) {
                    (Some(mine), Some(theirs)) => {
                        theirs > mine
                            || (theirs == mine
                                && (other.depth > first.depth
                                    || (other.depth == first.depth
                                        && other.name < first.name)))
                    }
                    (Some(_), None) => false,
                    (None, _) => true,
                }
            });
            if covers {
                warnings.push(format!(
                    "shadowed claim: {} is fully claimed by {}",
                    first.name, other.name
                ));
            }
        }
    }
}

/// True if a stereotype's match-set hits the submodule's surface: units the
/// enclosing claim has assigned whose name the submodule's boundary path
/// matches (submodules claim the module tier only, so the unit surface is the
/// ancestor's claimed territory seen through the boundary), or a module path
/// inside the boundary matched by the full-path/bare-name rule.
fn submodule_surface_matches(
    stereotype: &Stereotype,
    boundary: &[String],
    ancestor_units: &BTreeSet<String>,
    model: &Model,
) -> bool {
    let units_claimed = ancestor_units.iter().any(|unit| {
        boundary
            .iter()
            .any(|pattern| module_path_matches(pattern, unit))
            && stereotype_matches_unit(unit, stereotype)
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
        // Module-tier public API facts (the csharp shape of this constraint,
        // US 02): `allowed` patterns glob-match MODULE paths; every public
        // type of a module outside the list is a leak attributed to that
        // module. Unlike a boundary's `matches.modules`, an allowlist entry
        // claims no ancestor territory — gating a namespace does not expose
        // namespaces beneath it, mirroring the rust root shape where only
        // listed exports are allowed. The map is empty wherever the
        // root-export family above is the fact source, so drivers keep
        // exactly one of the two loops hot.
        for (module, types) in &model.module_public_types {
            let allowlisted = constraint
                .allowed
                .iter()
                .any(|pattern| matches_pattern(module, pattern));
            if !allowlisted {
                for type_name in types {
                    push_finding(
                        format!("{module} exposes {type_name} (not allowlisted)"),
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
fn check_forbid_submodule(edges: &[ModuleEdge], constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "forbid_submodule_dependency")
    {
        for edge in edges {
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
    submodule_edges: &[ModuleEdge],
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
            // Engaged iff there is a public API fact to check: any
            // crate-root export family member (rust) or any module carrying
            // public types (csharp). On a module-fact tree whose `allowed`
            // patterns address no such module, the allowlist gates nothing —
            // stated like the `from`-pattern arm instead of passing silently
            // (US 03: engagement honesty). No facts at all announce the
            // emptiness with the driver's own vocabulary: csharp states
            // "public API facts", rust keeps its crate-root sentence
            // verbatim (byte-pinned by the rust vacuity guards).
            "public_api_allowlist" => {
                let root_surface = !(model.root_public_exports.is_empty()
                    && model.root_glob_exports.is_empty()
                    && model.root_empty_glob_exports.is_empty());
                if root_surface {
                    None
                } else if !model.module_public_types.is_empty() {
                    let engaged = constraint.allowed.iter().any(|pattern| {
                        model
                            .module_public_types
                            .keys()
                            .any(|module| matches_pattern(module, pattern))
                    });
                    if engaged {
                        None
                    } else {
                        let patterns: Vec<String> = constraint
                            .allowed
                            .iter()
                            .map(|pattern| format!("\"{pattern}\""))
                            .collect();
                        // Misuse deflection (misuse-diagnostics US 01): an
                        // entry written as a bare module NAME where a module
                        // PATH was meant gets one concrete corrected path
                        // named on this line — the single context statement
                        // of an allowlist gating nothing (leak lines stay
                        // byte-unchanged). Exact structural equality only;
                        // no candidate means the sentence byte-unchanged.
                        // Rust trees cannot reach this arm: the module-tier
                        // fact map is empty wherever root exports are the
                        // fact source, so the rust sentence below is never
                        // near this machinery.
                        let nudge = allowlist_path_nudge(&constraint.allowed, model, modules);
                        Some(format!(
                            "'allowed' pattern {} matches no module with public types in the model{nudge}",
                            patterns.join(", ")
                        ))
                    }
                } else if language::from_name(&model.language) == Some(language::Language::Csharp) {
                    Some("no public API facts to check".to_string())
                } else if language::from_name(&model.language) == Some(language::Language::Go) {
                    // D9 (workplan archspec_module_public_api_diagrams): go has
                    // no crate, so the rust sentence was wrong vocabulary on a
                    // go tree. The neutral arm names capability, tier, and the
                    // remedy pointer — no driver-specific token.
                    Some(
                        "no module-tier public-API facts to check for the public-API surface \
                         (see archspec help capabilities)"
                            .to_string(),
                    )
                } else {
                    Some("no crate-root public exports to check".to_string())
                }
            }
            // Engaged iff any module edge has a common ancestor matching `parent`
            // and a source submodule matching a `from` pattern.
            "forbid_submodule_dependency" => {
                let engaged = submodule_edges.iter().any(|edge| {
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
    let declared: BTreeSet<String> = spec::claim_names(modules).into_iter().collect();
    let declared: BTreeSet<&str> = declared.iter().map(String::as_str).collect();
    let declared_stereotypes: BTreeSet<&str> = stereotypes
        .iter()
        .map(|stereotype| stereotype.name.as_str())
        .collect();
    let mut found: BTreeSet<String> = BTreeSet::new();
    for module in flatten_claims(modules)
        .iter()
        .map(|claim| ModuleRef {
            name: claim.name,
            allowed: claim.allowed,
            contract: claim.contract,
        })
    {
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
                     but undeclared — references resolve to declared claim names at any depth",
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
    }
    found.into_iter().collect()
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

/// The did-you-mean suffix for a vacuous `public_api_allowlist` on a
/// module-path-tier tree (misuse-diagnostics US 01). An `allowed` entry
/// without `::` may be a bare module NAME where a module PATH was meant.
/// A candidate is a module path carrying public-type facts whose last
/// `::`-separated segment equals the entry case-sensitively, or a
/// fact-carrying path addressed by a module DECLARED under exactly that
/// name (the reported misread: `allowed = ["contracts"]` beside a module
/// `contracts` owning `Sample::Contracts`). Exact equality only — no edit
/// distance, no case folding, no invented spellings. Candidates are
/// deduplicated and sorted; the suffix names up to three, then counts the
/// rest. No candidate: empty string, the vacuity sentence byte-unchanged.
/// Rust trees never reach this helper — the fact map it reads is empty on
/// rust by driver construction.
fn allowlist_path_nudge(allowed: &[String], model: &Model, modules: &[Module]) -> String {
    let mut candidates: BTreeSet<&String> = BTreeSet::new();
    for entry in allowed.iter().filter(|entry| !entry.contains("::")) {
        for path in model.module_public_types.keys() {
            let tail = path.rsplit("::").next().unwrap_or(path);
            let addressed_by_name = modules.iter().any(|module| {
                module.name == *entry
                    && module
                        .matches
                        .modules
                        .iter()
                        .any(|pattern| module_path_matches(pattern, path))
            });
            if tail == entry.as_str() || addressed_by_name {
                candidates.insert(path);
            }
        }
    }
    let total = candidates.len();
    if total == 0 {
        return String::new();
    }
    let named: Vec<String> = candidates
        .iter()
        .take(3)
        .map(|path| format!("\"{path}\""))
        .collect();
    let list = if total > 3 {
        format!(
            "{}...and {} more module path{}",
            named.join(", "),
            total - 3,
            if total - 3 == 1 { "" } else { "s" }
        )
    } else if named.len() == 1 {
        named[0].clone()
    } else if named.len() == 2 {
        format!("{} or {}", named[0], named[1])
    } else {
        format!("{}, {} or {}", named[0], named[1], named[2])
    };
    format!("; allowed entries are module paths — did you mean {list}?")
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
        count_claims(modules),
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
