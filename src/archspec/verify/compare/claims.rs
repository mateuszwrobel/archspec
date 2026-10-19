//! Declared-claim tree and jurisdiction resolution: flatten `[[module]]`
//! and `[[module.submodules]]` depth-first, resolve which claim owns a
//! module path (specificity, unit fallback, inheritance, composition).

use std::collections::{BTreeSet};
use super::pattern::{matches_pattern, strip_unit};
use crate::archspec::model::Model;
use crate::archspec::spec;
use crate::archspec::spec::{Allowed, Contract, MatchSet, Module};

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
pub(super) struct ModuleRef<'a> {
    pub(super) name: &'a str,
    pub(super) allowed: &'a Allowed,
    pub(super) contract: &'a Contract,
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
pub(super) fn claim_owner<'a>(path: &str, claims: &[Claim<'a>]) -> Option<usize> {
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
pub(super) fn claim_unit_fallback(path: &str, claims: &[Claim<'_>]) -> Option<usize> {
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

/// The unit-fallback owner by the EDGE's declared unit name (C1, the ts
/// shape of the unit tier): a ts module edge is file-identified
/// (`src/hidden.ts`), so the fallback's path parsing names a directory, not
/// a unit — but the edge carries the owning package as `unit`. When the
/// path parsing attributes nothing, THIS names that unit against the claims'
/// `matches.units` globs — the same first-declared-wins rule, against the
/// honest unit. For path-identified languages the path parsing attributes
/// every edge this could attribute (the path's unit is the edge unit), so
/// the arm only ever fires on file-identified edges.
pub(super) fn claim_unit_named(unit: &str, claims: &[Claim<'_>]) -> Option<usize> {
    if unit.is_empty() {
        return None;
    }
    claims.iter().enumerate().find_map(|(index, claim)| {
        // The file-territory gate (D10 law): a claim stating no file
        // selectors owns the package component but never an edge endpoint
        // carried by a path — the unowned-endpoint category stays honest on
        // file backends. A claim pairing `units` with file selectors does
        // own them, which is the conduit shape the laundering check reads.
        (!claim.matches.modules.is_empty()
            && claim.depth == 0
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
pub(super) fn policy_allowed<'a>(claims: &[Claim<'a>], index: usize) -> &'a Allowed {
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
pub(super) fn cumulative_forbidden<'a>(claims: &[Claim<'a>], index: usize) -> BTreeSet<&'a str> {
    let mut forbidden = BTreeSet::new();
    let mut current = Some(index);
    while let Some(at) = current {
        forbidden.extend(
            claims[at]
                .allowed
                .forbidden
                .iter()
                .map(|entry| entry.target.as_str()),
        );
        current = claims[at].parent;
    }
    forbidden
}

/// True when one claim is the other's direct-or-transitive ancestor — the
/// containment relation whose edges are composition, never dependencies (D3).
pub(super) fn related_by_containment(claims: &[Claim<'_>], a: usize, b: usize) -> bool {
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
pub(super) fn subtree_names<'a>(claims: &[Claim<'a>], index: usize) -> BTreeSet<&'a str> {
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

pub(super) fn claim_by_name<'a>(claims: &[Claim<'a>], name: &str) -> Option<usize> {
    claims.iter().position(|claim| claim.name == name)
}

/// Specificity of one `matches.modules` entry against a dotted module path:
/// how much of the path the entry pins, in characters. `None` when the entry
/// does not match at all (same accept set as `module_path_matches`). An entry
/// naming the module exactly pins the whole path and always wins; otherwise the
/// longest matching prefix wins, so `commands::start*` claims `start` over
/// `commands::**`'s broader subtree claim only when it pins more.
pub(super) fn entry_specificity(pattern: &str, module_path: &str) -> Option<usize> {
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
pub(super) fn check_ambiguous_module_matches(model: &Model, claims: &[Claim<'_>]) -> Vec<String> {
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
