//! Edge-class comparison: the boundary graph over both tiers and the
//! `allowed.depend_on`/`forbidden` adjudication, facade and laundering
//! checks, and the submodule-edge construction the submodule arm reads.

use std::collections::{BTreeMap, BTreeSet};
use super::adjudicate::{Verdict, pair_verdict};
use super::claims::{Claim, claim_unit_named};
use super::cycles::{submodule_identity, ts_submodule_identity};
use super::diff::Diff;
use super::mapping::{Mapping, is_test_module};
use super::claims::claim_by_name;
use super::claims::claim_owner;
use super::claims::subtree_names;

use crate::archspec::model::{Model, ModuleEdge, Role};
use crate::archspec::spec::{Constraint, Module};

/// Claim-named dependency edges for `forbid_submodule_dependency`. Trees with
/// a native module tier already carry real module edges. A single go.mod tree
/// whose claims partition its packages has the derived tier: unit edges become
/// owner edges named by the claims, so `common_ancestor` yields the declared
/// parent and the check reads the same shape as on a rust tree. Edges inside
/// one group drop out, matching the native derivation.
pub(super) fn submodule_dependency_edges(
    model: &Model,
    mapping: &Mapping,
    claims: &[Claim<'_>],
) -> Vec<ModuleEdge> {
    if matches!(model.language.as_str(), "ts" | "python" | "kotlin") {
        return ts_submodule_dependency_edges(model, claims);
    }
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
            type_only: false,
            unit: from.clone(),
            from,
            to,
            symbols: Vec::new(),
        })
        .collect()
}

/// The ts projection (C3): the file-identified module edges re-attributed to
/// their claim owners, so `common_ancestor` yields the declared parent and
/// `forbid_submodule_dependency` adjudicates ts trees in the shipped
/// claim-named vocabulary — the `parent::child` wall the pilot recorded is
/// down. Endpoint attribution follows the same order as `place_module_edge`:
/// a `matches.modules` pattern addressing the file, else the claim naming the
/// edge's package unit. An endpoint no claim addresses keeps its file
/// identity; edges inside one owner drop out, matching the shipped
/// derivations; a test-gated endpoint excludes the edge — the R1 tier law,
/// read through the same `is_test_gated` channel every module-edge check uses.
fn ts_submodule_dependency_edges(model: &Model, claims: &[Claim<'_>]) -> Vec<ModuleEdge> {
    let owner = |edge: &ModuleEdge, endpoint: &str| -> Option<usize> {
        claim_owner(endpoint, claims).or_else(|| claim_unit_named(&edge.unit, claims))
    };
    let mut projected: BTreeSet<(String, String)> = BTreeSet::new();
    for edge in &model.module_edges {
        if is_test_module(model, &edge.from) || is_test_module(model, &edge.to) {
            continue;
        }
        let from = owner(edge, &edge.from);
        let to = owner(edge, &edge.to);
        if let (Some(from), Some(to)) = (from, to)
            && from == to
        {
            continue;
        }
        // An endpoint no claim addresses keeps its FILE identity, restated in
        // the `::` address space the rule adjudicates in: directories become
        // segments and the `.ts` suffix is noise (the identity law the shared
        // scenario dialect `driver.module_path` writes).
        let from = from
            .map(|index| claims[index].name.to_string())
            .unwrap_or_else(|| submodule_identity(&model.language, &edge.from));
        let to = to
            .map(|index| claims[index].name.to_string())
            .unwrap_or_else(|| submodule_identity(&model.language, &edge.to));
        if from != to {
            projected.insert((from, to));
        }
    }
    projected
        .into_iter()
        .map(|(from, to)| ModuleEdge {
            type_only: false,
            unit: from.clone(),
            from,
            to,
            symbols: Vec::new(),
        })
        .collect()
}

/// The ts dialect of `forbid_submodule_dependency` addressing: `parent` and
/// full-path `from`/`forbid` entries naming files or directories restate in
/// the `::` space above (bare names carry no separator and pass through).
/// Keeps the shipped rust/c# reading of the constraint byte-identical.
pub(super) fn ts_submodule_constraints(constraints: &[Constraint]) -> Vec<Constraint> {
    constraints
        .iter()
        .map(|constraint| {
            if constraint.kind != "forbid_submodule_dependency" {
                return constraint.clone();
            }
            Constraint {
                parent: ts_submodule_identity(&constraint.parent),
                from: constraint.from.iter().map(|e| ts_submodule_identity(e)).collect(),
                forbid: constraint.forbid.iter().map(|e| ts_submodule_identity(e)).collect(),
                ..constraint.clone()
            }
        })
        .collect()
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
///
/// The per-entry `type_only` law (D3): the pair's crossing kind engages only
/// where the governing entry declares `exempt` — an exempt entry admits a
/// type-only crossing and reddens the value crossing; the `count` default
/// adjudicates every kind exactly as shipped.
pub(super) fn check_allowed_edges(
    pairs: &BTreeSet<(String, String)>,
    value_pairs: &BTreeSet<(String, String)>,
    model: &Model,
    modules: &[Module],
    claims: &[Claim<'_>],
    diff: &mut Diff,
) {
    for (from_module, to_module) in pairs {
        let value_crossing = value_pairs.contains(&(from_module.clone(), to_module.clone()));
        match pair_verdict(from_module, to_module, value_crossing, model, modules, claims) {
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

/// Missing edge (#11): a declared `allowed.depend_on` target is a declared
/// claim, but no extracted edge connects the declaring claim's territory —
/// its whole subtree, so an edge owned by a policy-inheriting descendant
/// satisfies the ancestor's obligation exactly once — to that claim. A
/// policy-free nested claim adds no obligation of its own (D2): its
/// inherited rule fires at the declaring ancestor, so nested and flat
/// equivalents list the same pair with the same text.
pub(super) fn check_missing_edges(pairs: &BTreeSet<(String, String)>, claims: &[Claim<'_>], diff: &mut Diff) {
    for (index, claim) in claims.iter().enumerate() {
        if !claim.allowed.declared {
            continue;
        }
        let territory = subtree_names(claims, index);
        for entry in &claim.allowed.depend_on {
            let target = entry.target.as_str();
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
pub(super) fn check_facade_roots(model: &Model, diff: &mut Diff) {
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
pub(super) fn check_forbidden_laundering(
    pairs: &BTreeSet<(String, String)>,
    fallback_pairs: &BTreeSet<(String, String)>,
    modules: &[Module],
    warnings: &mut Vec<String>,
) {
    let connects = |a: &str, b: &str| a == b || reachable(pairs, a, b);
    for module in modules {
        for entry in &module.allowed.forbidden {
            let target = entry.target.as_str();
            let target_declared = modules.iter().any(|candidate| candidate.name == target);
            if !target_declared || pairs.contains(&(module.name.clone(), target.to_string())) {
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