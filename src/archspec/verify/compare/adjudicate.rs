//! Pair-level verdict adjudication and placement: the Adjudicator the
//! diagram engines reuse, pair verdicts, type-only entry resolution, and
//! where a module edge lands in the boundary graph.

use std::borrow::Cow;
use std::collections::{BTreeSet};
use super::claims::{Claim, claim_unit_fallback, claim_unit_named, related_by_containment};
use super::contracts::{ExposeVerdicts, adjudicate_expose, claim_covers_path};
use super::cycles::{file_claim_index, file_tier_report_cycles, find_cycle, strongly_connected_components};
use super::mapping::{Mapping, augment_with_modules, is_test_module, map_units, owning_unit};
use super::pattern::matches_pattern;
use super::claims::claim_by_name;
use super::claims::claim_owner;
use super::claims::cumulative_forbidden;
use super::claims::flatten_claims;
use super::claims::policy_allowed;
use super::mapping::effective_module_tier;
use crate::archspec::{capability, language};
use crate::archspec::model::{Model, ModuleEdge, Role};

use crate::archspec::spec::{Constraint, Module, Stereotype};

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

pub(super) fn pair_verdict(
    from_module: &str,
    to_module: &str,
    value_crossing: bool,
    model: &Model,
    modules: &[Module],
    claims: &[Claim<'_>],
) -> Option<Verdict> {
    let index = claim_by_name(claims, from_module)?;
    // The ratchet (D5): every ancestor prohibition binds this claim's
    // edges; the policy ceiling (D2) is the claim's own declared table or
    // the nearest ancestor's.
    if cumulative_forbidden(claims, index).contains(to_module) {
        // The D3 per-entry exemption: an exempt prohibition admits the
        // pair's type-only crossing; a value crossing stays forbidden, and
        // any `count` entry on the chain keeps the prohibition binding.
        if entry_type_only(claims, index, to_module, EntryList::Forbidden) == Some("exempt")
            && !value_crossing
        {
            return None;
        }
        return Some(Verdict::Forbidden);
    }
    let exempted =
        entry_type_only(claims, index, to_module, EntryList::DependOn) == Some("exempt");
    if exempted && !value_crossing {
        // The exempt allowed entry admits the type-only crossing (D3).
        return None;
    }
    if !policy_allowed(claims, index)
        .depend_on
        .iter()
        .any(|entry| entry.target == to_module)
        && !unit_tier_identity_allows(from_module, to_module, value_crossing, model, modules)
    {
        return Some(Verdict::Disallowed);
    }
    if exempted {
        // The exempt entry still reddens the same import written as a
        // value import (D3) — the finding names the crossing.
        return Some(Verdict::Disallowed);
    }
    None
}

/// Which entry list of the governing table an exemption lookup reads.
enum EntryList {
    DependOn,
    Forbidden,
}

/// The entry-level `type_only` declaration governing the pair into `target`
/// (D3). `forbidden` entries bind cumulatively up the ancestor chain (the
/// D5 ratchet), so a single `count` entry naming the target anywhere on the
/// chain keeps the default binding; `depend_on` entries come from the one
/// governing table (the D2 ceiling). `Some("exempt")` answers only when an
/// entry names the target and no governing entry names it at the `count`
/// default; `None` when no entry names the target at all.
fn entry_type_only(
    claims: &[Claim<'_>],
    index: usize,
    target: &str,
    list: EntryList,
) -> Option<&'static str> {
    let mut declared = None;
    let mut current = match list {
        EntryList::Forbidden => Some(index),
        EntryList::DependOn => {
            let mut at = Some(index);
            while let Some(claim) = at {
                if claims[claim].allowed.declared {
                    break;
                }
                at = claims[claim].parent;
            }
            at
        }
    };
    while let Some(at) = current {
        let entries = match list {
            EntryList::Forbidden => &claims[at].allowed.forbidden,
            EntryList::DependOn => &claims[at].allowed.depend_on,
        };
        for entry in entries {
            if entry.target != target {
                continue;
            }
            if entry.type_only == "exempt" {
                if declared != Some("count") {
                    declared = Some("exempt");
                }
            } else {
                declared = Some("count");
            }
        }
        current = match list {
            EntryList::Forbidden => claims[at].parent,
            EntryList::DependOn => None,
        };
    }
    declared
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
    /// The full boundary pair set `verify`'s findings are adjudicated over
    /// (`resolve_module_edges` output) — the claim-tier adjacency the
    /// cycle-membership query dedups the files tier against (M13).
    module_pairs: BTreeSet<(String, String)>,
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
            .filter(|(from, to)| {
                let value_crossing =
                    graph.value_pairs.contains(&((*from).clone(), (*to).clone()));
                pair_verdict(from, to, value_crossing, &derived, modules, &claims).is_some()
            })
            .cloned()
            .collect();
        let module_pairs = graph.pairs;
        let slashed_paths = derived.language == "go" && derived.has_module_tier();
        let expose = adjudicate_expose(&mapping, &derived, modules, stereotypes);
        Self {
            claims,
            mapping,
            modules,
            violating,
            module_pairs,
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
            type_only: false,
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

    /// The claim a file's cycle findings attribute to (D6/D10 attribution:
    /// `file_claim_index`) — the same owner `verify` names in a `cycle:
    /// <claim>: <file> -> …` finding. `None` when no claim covers the file.
    pub fn file_claim_of(&self, file: &str) -> Option<&'a str> {
        file_claim_index(&self.derived, file, &self.claims).map(|index| self.claims[index].name)
    }

    /// The cycle-membership query (design record M13): the directed file
    /// pairs that appear in the file-tier cycle findings `verify` REDS for
    /// this spec — the error bucket only, since a warning-severity finding
    /// never reddens a run. The pairs come from `file_tier_report_cycles`,
    /// the very extractor `check_cycles` formats its findings from (same
    /// adjacency projection, same SCC pass, same projection-keyed dedup),
    /// so a marked arrow and a red verdict can never disagree — the diagram
    /// reads marks from here and never re-derives a cycle itself (ADR-021).
    /// Languages whose file facts are not emitted mark nothing (capability
    /// `file-grained-dependency-facts`, D7).
    pub fn file_cycle_marks(&self, constraints: &[Constraint]) -> BTreeSet<(String, String)> {
        let mut marks = BTreeSet::new();
        let Some(language) = language::from_name(&self.derived.language) else {
            return marks;
        };
        if capability::emission(language, capability::FACT_FILE_GRAINED_DEPENDENCY_FACTS)
            != Some(capability::GRANULAR)
        {
            return marks;
        }
        let posture = capability::posture(language);
        let all_modules: BTreeSet<String> = self.modules.iter().map(|module| module.name.clone()).collect();
        for constraint in constraints
            .iter()
            .filter(|constraint| constraint.kind == "no_cycles")
        {
            let scope = constraint
                .scope
                .as_deref()
                .or(Some(posture.0))
                .unwrap_or(capability::SCOPE_CLAIMS);
            if scope != capability::SCOPE_BOTH || constraint.severity == "warning" {
                continue;
            }
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
            let induced: BTreeSet<(String, String)> = self
                .module_pairs
                .iter()
                .filter(|(from, to)| group.contains(from) && group.contains(to))
                .cloned()
                .collect();
            let mut claim_cycle_edges: BTreeSet<(String, String)> = BTreeSet::new();
            for scc in strongly_connected_components(&group, &induced) {
                if let Some(path) = find_cycle(&scc, &induced) {
                    for pair in path.windows(2) {
                        claim_cycle_edges.insert((pair[0].clone(), pair[1].clone()));
                    }
                }
            }
            for (path, _members) in
                file_tier_report_cycles(constraint, &self.derived, &group, &claim_cycle_edges, &self.claims)
            {
                for pair in path.windows(2) {
                    marks.insert((pair[0].clone(), pair[1].clone()));
                }
            }
        }
        marks
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

/// The boundary-level dependency graph extracted from the model.
pub(super) struct BoundaryGraph {
    /// Deduplicated cross-boundary pairs from both tiers. The cycle tier
    /// consumes this set alone: it counts every edge regardless of kind
    /// (D3 — no exemption exists at the cycle tier).
    pub(super) pairs: BTreeSet<(String, String)>,
    /// Pairs carrying at least one VALUE crossing (ts `type_only = "exempt"`
    /// law, D3): unit-tier edges carry no kind fact and are value crossings;
    /// a ts module-edge row contributes here iff it is not type-only. A pair
    /// in `pairs` and absent here crosses type-only.
    pub(super) value_pairs: BTreeSet<(String, String)>,
    /// Pairs whose ownership resolved through the unit fallback: hops across
    /// territory no boundary declares (unit wiring, unclaimed modules).
    pub(super) fallback_pairs: BTreeSet<(String, String)>,
    /// Soft-edge endpoints claimed by no boundary at all.
    pub(super) unowned_endpoints: BTreeSet<String>,
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
pub(super) fn composition_sources(model: &Model) -> BTreeSet<String> {
    model
        .roles
        .iter()
        .filter(|(_, role)| matches!(role, Role::Composition))
        .map(|(path, _)| path.clone())
        .collect()
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
pub(super) fn resolve_module_edges(
    model: &Model,
    mapping: &Mapping,
    modules: &[Module],
    claims: &[Claim<'_>],
    composition_sources: &BTreeSet<String>,
) -> BoundaryGraph {
    let mut pairs = BTreeSet::new();
    let mut value_pairs = BTreeSet::new();
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
            let pair = (from_module.clone(), to_module.clone());
            // Unit-tier edges carry no kind fact — a value crossing (D3).
            value_pairs.insert(pair.clone());
            pairs.insert(pair);
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
                    if !edge.type_only {
                        value_pairs.insert(pair.clone());
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
        value_pairs,
        fallback_pairs,
        unowned_endpoints,
    }
}

pub(super) fn place_module_edge<'a>(
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
        .or_else(|| claim_unit_fallback(&edge.from, claims).map(|index| (index, true)))
        .or_else(|| claim_unit_named(&edge.unit, claims).map(|index| (index, true)));
    let to_owner = claim_owner(&edge.to, claims)
        .map(|index| (index, false))
        .or_else(|| claim_unit_fallback(&edge.to, claims).map(|index| (index, true)))
        .or_else(|| claim_unit_named(&edge.unit, claims).map(|index| (index, true)));
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
/// that name it (ADR-018). Under the D3 per-entry law an `exempt` entry
/// grants identity only to a type-only crossing — the value crossing is not
/// admitted through the exempt door.
fn unit_tier_identity_allows(
    from: &str,
    to: &str,
    value_crossing: bool,
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
        let mut count = false;
        let mut exempt = false;
        let named = spec.allowed.depend_on.iter().any(|entry| {
            if entry.target != *target {
                return false;
            }
            if entry.type_only == "exempt" {
                exempt = true;
            } else {
                count = true;
            }
            true
        });
        // The count default wins wherever it names the target (D3 default);
        // an exempt-only entry admits the type-only crossing alone.
        let door = !value_crossing || !exempt || count;
        named
            && !spec
                .allowed
                .forbidden
                .iter()
                .any(|entry| entry.target == *target)
            && door
    })
}