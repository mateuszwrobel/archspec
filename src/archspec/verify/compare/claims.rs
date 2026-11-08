//! Declared-claim tree and jurisdiction resolution: flatten `[[module]]`
//! and `[[module.submodules]]` depth-first, resolve which claim owns a
//! module path (specificity, unit fallback, inheritance, composition).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
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

/// The claim subtree: the claim itself plus every nested descendant. The
/// reference walk kept for the adjacency parity test below; callers use
/// `subtree_names_with` (issue #17).
#[cfg(test)]
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

/// Parent→children adjacency of the claim tree (every index listed once
/// under its parent; roots appear in no list). Kept for the adjacency
/// parity test; production callers fold ancestor sets instead (issue #17).
#[cfg(test)]
pub(super) fn claim_children(claims: &[Claim<'_>]) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); claims.len()];
    for (index, claim) in claims.iter().enumerate() {
        if let Some(parent) = claim.parent {
            children[parent].push(index);
        }
    }
    children
}

/// `subtree_names` over a prebuilt adjacency: the same name set, visited in
/// time linear in the subtree instead of the whole claim list. Test-only
/// parity partner of the walk above (issue #17).
#[cfg(test)]
pub(super) fn subtree_names_with<'a>(
    children: &[Vec<usize>],
    claims: &[Claim<'a>],
    index: usize,
) -> BTreeSet<&'a str> {
    let mut names = BTreeSet::new();
    let mut stack = vec![index];
    while let Some(at) = stack.pop() {
        names.insert(claims[at].name);
        stack.extend(children[at].iter().copied());
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
    // Issue #17 fast path: with no `matches.modules` patterns declared the
    // per-path scan below can only produce empty candidate lists, so an
    // ambiguity is impossible and the paths need not be collected at all.
    if claims
        .iter()
        .all(|claim| claim.matches.modules.is_empty())
    {
        return Vec::new();
    }
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    for module_paths in model.soft_structure.values() {
        paths.extend(module_paths.iter().map(String::as_str));
    }
    for edge in &model.module_edges {
        paths.insert(edge.from.as_str());
        paths.insert(edge.to.as_str());
    }

    // Issue #17: one indexed pass replaces the claims-per-path scan. A
    // glob-less entry can only score a path by equaling the path, its
    // unit-stripped form, a `::` ancestor or a bare segment of either (the
    // `entry_specificity` arms), so those strings are the path's literal
    // candidates, looked up once each. Entries carrying a `*` keep the
    // per-path specificity scan; their count is what a spec declares, not
    // what a tree holds.
    let mut exact: HashMap<&str, Vec<(usize, usize)>> = HashMap::new();
    let mut globs: Vec<(usize, usize)> = Vec::new();
    for (claim, entry) in claims.iter().enumerate() {
        for (position, pattern) in entry.matches.modules.iter().enumerate() {
            if pattern.contains('*') {
                globs.push((claim, position));
            } else {
                exact
                    .entry(pattern.as_str())
                    .or_default()
                    .push((claim, position));
            }
        }
    }

    let mut found = BTreeSet::new();
    let mut best: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    for path in paths {
        best.clear();
        for (key, score) in literal_specificity_keys(path) {
            if let Some(hits) = exact.get(key) {
                for &(claim, position) in hits {
                    merge_candidate(&mut best, claim, position, score);
                }
            }
        }
        for &(claim, position) in &globs {
            let pattern = &claims[claim].matches.modules[position];
            if let Some(score) = entry_specificity(pattern, path) {
                merge_candidate(&mut best, claim, position, score);
            }
        }
        let entries: Vec<(usize, &str, &str)> = best
            .iter()
            .map(|(&claim, &(score, position))| {
                (
                    score,
                    claims[claim].name,
                    claims[claim].matches.modules[position].as_str(),
                )
            })
            .collect();
        let Some(top) = entries.iter().map(|(score, _, _)| *score).max() else {
            continue;
        };
        let tied: Vec<&(usize, &str, &str)> = entries
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

/// The literal candidate strings for one module path with their scores:
/// exactly the targets `entry_specificity` compares a glob-less pattern
/// against. The path itself scores `usize::MAX` (the equality early return);
/// every other arm pins its whole comparison target, so its score is that
/// arm's pinned length. The first occurrence of a key carries its highest
/// score; the merge keeps the maximum anyway.
fn literal_specificity_keys(path: &str) -> Vec<(&str, usize)> {
    fn bare(value: &str) -> &str {
        value.rsplit("::").next().unwrap_or(value)
    }
    let mut keys: Vec<(&str, usize)> = Vec::new();
    push_key(&mut keys, path, usize::MAX);
    let path_len = path.chars().count();
    if bare(path) != path {
        push_key(&mut keys, bare(path), path_len);
    }
    let stripped = strip_unit(path);
    if stripped != path {
        push_key(&mut keys, stripped, path_len);
        if bare(stripped) != stripped {
            push_key(&mut keys, bare(stripped), path_len);
        }
    }
    let mut current = path;
    while let Some((rest, _)) = current.rsplit_once("::") {
        current = rest;
        let current_len = current.chars().count();
        push_key(&mut keys, current, current_len);
        if bare(current) != current {
            push_key(&mut keys, bare(current), current_len);
        }
        let ancestor_stripped = strip_unit(current);
        if ancestor_stripped != current {
            push_key(&mut keys, ancestor_stripped, current_len);
            if bare(ancestor_stripped) != ancestor_stripped {
                push_key(&mut keys, bare(ancestor_stripped), current_len);
            }
        }
    }
    keys
}

/// Record a literal candidate key once: the first occurrence carries the
/// highest score, so a repeat never raises it.
fn push_key<'a>(keys: &mut Vec<(&'a str, usize)>, key: &'a str, score: usize) {
    if !keys.iter().any(|(seen, _)| *seen == key) {
        keys.push((key, score));
    }
}

/// Fold one candidate into the path's per-claim best: the highest score
/// wins, and an equal score keeps the earlier declared entry — the same
/// first-max law the per-claim `max_by_key` scan applied.
fn merge_candidate(
    best: &mut BTreeMap<usize, (usize, usize)>,
    claim: usize,
    position: usize,
    score: usize,
) {
    match best.get_mut(&claim) {
        Some((best_score, best_position)) => {
            if score > *best_score || (score == *best_score && position < *best_position) {
                *best_score = score;
                *best_position = position;
            }
        }
        None => {
            best.insert(claim, (score, position));
        }
    }
}

/// Indexed claim resolution for the per-edge adjudication loops (GitHub
/// issue #17). The scans above answer one path at a time by walking EVERY
/// claim; on a large file-tier model that is quadratic — the openclaw repro
/// (371k module edges, 54k claims, all claims `units`-only) burned many
/// minutes in the edge placement while `verify` on the same tree exited
/// early on its language precondition and never reached this code. The index
/// restates NO matching law: it memoizes the pure scans by their input
/// string (the answer depends only on the path or unit, never on the call
/// order) and adds one fast path — a pattern EQUAL to the whole path scores
/// `usize::MAX` in `entry_specificity`, the only way to reach that score,
/// so a hash lookup of that equality replaces the full scan whenever one
/// claim pins the path exactly; every other shape falls through to the
/// identical scan, memoized per distinct path.
pub(crate) struct ClaimIndex {
    /// `claim_owner`'s answer memoized by module path.
    owner_memo: RefCell<HashMap<String, Option<usize>>>,
    /// pattern string → claim indices declaring it verbatim; a hit scores
    /// `usize::MAX` and no other claim can score equal or higher.
    exact_map: HashMap<String, Vec<usize>>,
    /// first-declared-win memos for the unit-fallback arms (C1 law): the
    /// answer depends only on the unit string, never on the call order.
    /// Interior-mutable so the per-edge placement stays a `&self` call.
    fallback_memo: RefCell<HashMap<String, Option<usize>>>,
    named_memo: RefCell<HashMap<String, Option<usize>>>,
}

impl ClaimIndex {
    pub(crate) fn new(claims: &[Claim<'_>]) -> Self {
        let mut exact_map: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, claim) in claims.iter().enumerate() {
            for pattern in claim.matches.modules.iter() {
                let entry = exact_map.entry(pattern.clone()).or_default();
                if entry.last() != Some(&index) {
                    entry.push(index);
                }
            }
        }
        Self {
            owner_memo: RefCell::new(HashMap::new()),
            exact_map,
            fallback_memo: RefCell::new(HashMap::new()),
            named_memo: RefCell::new(HashMap::new()),
        }
    }

    /// `claim_owner`'s answer — the same tie-break as the scan above, via
    /// the exact-equality fast path or the memoized scan.
    pub(crate) fn claim_owner(&self, claims: &[Claim<'_>], path: &str) -> Option<usize> {
        if let Some(answer) = self.owner_memo.borrow().get(path) {
            return *answer;
        }
        let answer = match self.exact_map.get(path) {
            // Every hit scores `usize::MAX`; among equal scores the scan
            // keeps the claim with the greatest depth (a nested claim
            // addresses its territory more precisely, D8), then the
            // smallest name, independent of declaration order — restate.
            Some(indices) => indices.iter().copied().max_by(|a, b| {
                (claims[*a].depth, std::cmp::Reverse(claims[*a].name))
                    .cmp(&(claims[*b].depth, std::cmp::Reverse(claims[*b].name)))
            }),
            None => claim_owner(path, claims),
        };
        self.owner_memo.borrow_mut().insert(path.to_string(), answer);
        answer
    }

    /// `claim_unit_fallback`'s answer, memoized by the path's unit.
    pub(crate) fn claim_unit_fallback(&self, path: &str, claims: &[Claim<'_>]) -> Option<usize> {
        let unit = path.split("::").next().unwrap_or(path);
        if let Some(answer) = self.fallback_memo.borrow().get(unit) {
            return *answer;
        }
        let answer = claim_unit_fallback(path, claims);
        self.fallback_memo
            .borrow_mut()
            .insert(unit.to_string(), answer);
        answer
    }

    /// `claim_unit_named`'s answer, memoized by the edge unit.
    pub(crate) fn claim_unit_named(&self, unit: &str, claims: &[Claim<'_>]) -> Option<usize> {
        if unit.is_empty() {
            return None;
        }
        if let Some(answer) = self.named_memo.borrow().get(unit) {
            return *answer;
        }
        let answer = claim_unit_named(unit, claims);
        self.named_memo.borrow_mut().insert(unit.to_string(), answer);
        answer
    }
}

/// Ancestry closure of one claim: its own index plus every ancestor index
/// (the claim `subtree_names` reaches, expressed as indices). The
/// missing-edge inversion (issue #17) reads this instead of re-walking the
/// whole pair set per declared entry.
pub(super) fn ancestors_of(claims: &[Claim<'_>], index: usize) -> Vec<usize> {
    let mut chain = vec![index];
    let mut current = claims[index].parent;
    while let Some(at) = current {
        chain.push(at);
        current = claims[at].parent;
    }
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(name: &str, modules: &[&str], units: &[&str], submodules: Vec<Module>) -> Module {
        Module {
            name: name.to_string(),
            matches: MatchSet {
                modules: modules.iter().map(|p| p.to_string()).collect(),
                units: units.iter().map(|p| p.to_string()).collect(),
                ..Default::default()
            },
            contract: Contract::default(),
            allowed: Allowed::default(),
            submodules,
        }
    }

    /// Fixture law: every shape that separates the index arms — exact
    /// equality (the `usize::MAX` rank), a trailing-`*` prefix, a complex
    /// glob, an exact pattern also reached through `strip_unit`, an equal-
    /// MAX tie across depths and names, and units-only claims for the
    /// fallback arms.
    fn fixture() -> Vec<Module> {
        vec![
            module("exact", &["a::b"], &[], vec![]),
            // Same MAX score, deeper: the shallower claim must still win.
            module(
                "parent",
                &["a"],
                &[],
                vec![module("nested", &["a::b"], &[], vec![])],
            ),
            module("prefix", &["pkg::*"], &[], vec![]),
            module("complex", &["**::leaf"], &[], vec![]),
            module("deep", &["a::b::C"], &[], vec![]),
            // units-only claims (the openclaw seed shape): never a
            // `claim_owner` answer, always a fallback candidate.
            module("units-only", &[], &["@scope/p"], vec![]),
            module("units-and-files", &["src/hidden.ts"], &["@scope/q"], vec![]),
        ]
    }

    const PATHS: &[&str] = &[
        "a::b",
        "a",
        "a::b::C",
        "a::b::C::D",
        "pkg::sub::thing",
        "x::y::leaf",
        "unitx::a::b",
        "unowned::path",
        "src/hidden.ts",
        "@scope/p::m",
    ];

    #[test]
    fn indexed_claim_owner_matches_the_scan_on_every_shape() {
        let modules = fixture();
        let claims = flatten_claims(&modules);
        let index = ClaimIndex::new(&claims);
        for path in PATHS {
            let scan = claim_owner(path, &claims);
            // Twice: the second read must come from the memo unchanged.
            assert_eq!(index.claim_owner(&claims, path), scan, "path {path}");
            assert_eq!(index.claim_owner(&claims, path), scan, "path {path} (memo)");
        }
    }

    #[test]
    fn indexed_unit_fallbacks_match_the_scans() {
        let modules = fixture();
        let claims = flatten_claims(&modules);
        let index = ClaimIndex::new(&claims);
        for path in PATHS {
            let scan = claim_unit_fallback(path, &claims);
            assert_eq!(index.claim_unit_fallback(path, &claims), scan, "path {path}");
            assert_eq!(index.claim_unit_fallback(path, &claims), scan, "path {path} (memo)");
        }
        for unit in ["", "@scope/p", "@scope/q", "@scope/missing", "a"] {
            let scan = claim_unit_named(unit, &claims);
            assert_eq!(index.claim_unit_named(unit, &claims), scan, "unit {unit}");
            assert_eq!(index.claim_unit_named(unit, &claims), scan, "unit {unit} (memo)");
        }
    }

    /// The exact-equality fast path must order MAX ties exactly as the
    /// scan's tie-break does: smallest depth first, then smallest name,
    /// independent of declaration order.
    #[test]
    fn exact_ties_follow_the_depth_then_name_law() {
        let modules = vec![
            module("zeta", &["t::u"], &[], vec![]),
            module(
                "alpha-parent",
                &["t"],
                &[],
                vec![module("alpha", &["t::u"], &[], vec![])],
            ),
        ];
        let claims = flatten_claims(&modules);
        let index = ClaimIndex::new(&claims);
        // depth-1 "alpha" beats depth-0 "zeta" (the equal-MAX depth law),
        // and the scan agrees.
        assert_eq!(index.claim_owner(&claims, "t::u"), claim_owner("t::u", &claims));
        assert_eq!(claims[index.claim_owner(&claims, "t::u").unwrap()].name, "alpha");
    }

    /// Issue #17: the adjacency subtree walk returns exactly the name set
    /// the parent-chain scan produced, at every claim index.
    #[test]
    fn subtree_adjacency_matches_the_parent_walk() {
        let modules = fixture();
        let claims = flatten_claims(&modules);
        let children = claim_children(&claims);
        for index in 0..claims.len() {
            assert_eq!(
                subtree_names_with(&children, &claims, index),
                subtree_names(&claims, index),
                "index {index}"
            );
        }
    }

    /// Issue #17: for glob-less entries the indexed candidate keys carry
    /// exactly the scores `entry_specificity` computes — arm by arm, for
    /// every pattern×path pair the fixture separates.
    #[test]
    fn literal_specificity_keys_agree_with_entry_specificity() {
        for path in PATHS {
            for pattern in [
                "a::b", "a", "a::b::C", "a::b::C::D", "b", "C", "C::D", "pkg", "leaf",
                "unitx", "unitx::a", "unowned", "src/hidden.ts", "@scope/p", "p",
                "missing",
            ] {
                let direct = entry_specificity(pattern, path);
                let indexed = literal_specificity_keys(path)
                    .into_iter()
                    .find(|(key, _)| *key == pattern)
                    .map(|(_, score)| score);
                assert_eq!(direct, indexed, "pattern {pattern} vs path {path}");
            }
        }
    }
}
