//! The no_cycles arm: boundary-tier and file-tier cycle detection over
//! the declared groups, SCC computation, and cycle path rendering.

use std::collections::{BTreeMap, BTreeSet};
use super::claims::{Claim, claim_unit_fallback};
use super::mapping::file_grained_rows;
use super::claims::claim_owner;
use crate::archspec::{capability, language};
use crate::archspec::model::{Model, Unit};
use crate::archspec::spec::{Constraint, Module};

/// `no_cycles` check (#9, severity #12-#14) with the D5 scope selector: the
/// The file-tier cycles one constraint REPORTS (D5/D6): the file-grained
/// adjacency, kind-collapsed (a type-only edge is an edge in the cycle
/// graph; rust `mod` declaration rows are the one exclusion — composition is
/// not a dependency, see `file_grained_rows`), with same-file edges dropped
/// at projection entry (R2-1), yields
/// one deterministic representative path per SCC whose members share the
/// constraint's territory — suppressed when its claim projection is a cycle
/// already reported at the claim tier (R3-4). Returns `(path, members)` per
/// reported cycle; the members are the deduplicated path nodes, the atoms of
/// the D6 attribution. Shared by `check_cycles` (the finding lines) and the
/// seam's cycle-membership query (the diagram's marks) — one resolver, never
/// two (M13).
pub(super) fn file_tier_report_cycles(
    constraint: &Constraint,
    model: &Model,
    group: &BTreeSet<String>,
    claim_cycle_edges: &BTreeSet<(String, String)>,
    claims: &[Claim<'_>],
) -> Vec<(Vec<String>, Vec<String>)> {
    let adjacency: BTreeSet<(String, String)> = file_grained_rows(model)
        .into_iter()
        .filter(|(from, to)| from != to)
        .filter(|(from, to)| {
            // Rust `mod` declaration rows are composition, never a
            // dependency: the cycle graph excludes them (the diagram keeps
            // rendering them — they ARE the file tree's structure). A pair
            // backed by at least one non-`mod` row stays; a ring whose
            // closing edge is a bare declaration is the parent-owns-child
            // fact the module tier never edges (the tier-comparability law
            // of the D6 projection dedup, ADR-023).
            language::from_name(&model.language) != Some(language::Language::Rust)
                || model.file_edges.iter().any(|edge| {
                    edge.kind != crate::archspec::model::FileFactKind::Mod
                        && edge.from.as_str() == *from
                        && edge.to.as_str() == *to
                })
        })
        .filter(|(from, to)| {
            // Test-gated files never participate, mirroring the module-tier
            // exclusion (`is_test_gated`) and the go `*_test.go` / ts test-
            // file law: the `mod tests` declaration plus the tests' import
            // of its parent is a declaration pair, not a production ring.
            [from, to].iter().all(|file| {
                !model.is_test_gated(&file_module_identity(model, file))
            })        })
        .map(|(from, to)| (from.to_string(), to.to_string()))
        .collect();
    let files: BTreeSet<String> = adjacency
        .iter()
        .flat_map(|(from, to)| [from.clone(), to.clone()])
        .filter(|file| {
            constraint.modules.is_empty()
                || file_claim_index(model, file, claims)
                    .is_some_and(|index| group.contains(claims[index].name))
        })
        .collect();
    let file_induced: BTreeSet<(String, String)> = adjacency
        .iter()
        .filter(|(from, to)| files.contains(from) && files.contains(to))
        .cloned()
        .collect();
    let mut reports = Vec::new();
    for scc in strongly_connected_components(&files, &file_induced) {
        let Some(path) = find_cycle(&scc, &file_induced) else {
            continue;
        };
        if projection_reports_claim_cycle(model, &scc, &file_induced, claims, claim_cycle_edges) {
            continue;
        }
        // Attribution is over the CYCLE's members (path), not the whole
        // SCC: one cycle one finding, one ancestor of ITS members.
        let cycle_members: Vec<String> = {
            let mut seen = BTreeSet::new();
            path.iter()
                .filter(|node| seen.insert((*node).clone()))
                .cloned()
                .collect()
        };
        reports.push((path, cycle_members));
    }
    reports
}

/// effective scope is the constraint's declared `scope`, derived from the
/// posture table when absent (D7). The claim tier always adjudicates — for
/// each such constraint, the extracted module-pair edges induced on the
/// constraint's modules (all declared modules if the list is empty) yield
/// every strongly connected component of size greater than one, one
/// representative canonically smallest cycle path per SCC; those bytes are
/// today's behavior at every scope (the scope truth table). At scope `both`
/// the file tier adjudicates too: the model's file-grained adjacency (the
/// `file-grained-dependency-facts` rows, kind-collapsed — the cycle graph
/// counts every edge regardless of kind, D3, except rust `mod` declaration
/// rows, which are composition and never participate (see
/// `file_grained_rows`) —, restricted to the constraint's
/// territory, contributes one finding per file SCC, attributed to the nearest
/// common claim ancestor of ALL its members over the claims union a synthetic
/// tree root (D6): `cycle detected in: <claim>: <file> -> <file>` when the
/// members share a claiming ancestor, `cycle detected in: <file> -> <file>`
/// when they share none. A files-tier SCC is suppressed iff its members'
/// nearest claiming ancestors form a cycle already reported at the claim
/// tier — one violation one finding, coarsest tier first (D6, R3-4). A cycle
/// found by a constraint whose `severity` is `"warning"` goes to the warning
/// bucket (rendered as `<category>: <detail>`); anything else is an error.
/// Returns `(error_cycles, warning_cycles)`.
pub(super) fn check_cycles(
    module_edges: &BTreeSet<(String, String)>,
    model: &Model,
    constraints: &[Constraint],
    modules: &[Module],
    claims: &[Claim<'_>],
) -> (Vec<String>, Vec<String>) {
    let all_modules: BTreeSet<String> = modules.iter().map(|module| module.name.clone()).collect();
    let posture = language::from_name(&model.language).map(capability::posture);
    let file_facts_granular = language::from_name(&model.language).is_some_and(|language| {
        capability::emission(language, capability::FACT_FILE_GRAINED_DEPENDENCY_FACTS)
            == Some(capability::GRANULAR)
    });
    let mut error_cycles: BTreeSet<String> = BTreeSet::new();
    let mut warning_cycles: BTreeSet<String> = BTreeSet::new();

    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "no_cycles")
    {
        let scope = constraint
            .scope
            .as_deref()
            .or(posture.map(|(scope, _)| scope))
            .unwrap_or(capability::SCOPE_CLAIMS);
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
        // The edges of the cycles this constraint actually REPORTS at the
        // claim tier — the only story the files tier may be deduped against
        // (R3-4/R3-7: an adjacency the claim report never printed cannot
        // stand in for a file cycle, even when it exists).
        let mut claim_cycle_edges: BTreeSet<(String, String)> = BTreeSet::new();

        for scc in strongly_connected_components(&group, &induced) {
            if let Some(path) = find_cycle(&scc, &induced) {
                for pair in path.windows(2) {
                    claim_cycle_edges.insert((pair[0].clone(), pair[1].clone()));
                }
                if severity_is_warning {
                    warning_cycles.insert(path.join(" -> "));
                } else {
                    error_cycles.insert(path.join(" -> "));
                }
            }
        }

        if scope != capability::SCOPE_BOTH {
            continue;
        }
        // The file tier (D5/D6): the shared extractor `file_tier_report_cycles`
        // — the same cycle set the diagram's files tier marks from (M13).
        if !file_facts_granular {
            continue;
        }
        for (path, members) in
            file_tier_report_cycles(constraint, model, &group, &claim_cycle_edges, claims)
        {
            let finding = file_cycle_finding(model, &members, &path, claims);
            if severity_is_warning {
                warning_cycles.insert(finding);
            } else {
                error_cycles.insert(finding);
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

/// The module-path spelling of a file identity (territory spelling, ts-driver
/// design record): a rust `.rs` file projects onto the module the scanner
/// spells for its backing module — `<unit>::<dir>::<stem>`, a `mod.rs`
/// folding into its directory and a unit root file folding into the unit
/// path (the lexical law `internal/cycle-count-probe.py` measures with; a
/// file attributed by `#[path]` projects lexically like any other). A dotted
/// path or a non-rust file identity is already module-spelled: identity.
/// This is the projection the file tier adjudicates claims against, so a
/// dotted-module claim covers the files backing its module — never a
/// silently-vacuous file tier (the vacuity carrier states facts, not
/// spelling accidents).
fn file_module_identity(model: &Model, path: &str) -> String {
    if language::from_name(&model.language) != Some(language::Language::Rust)
        || path.contains("::")
        || !path.ends_with(".rs")
    {
        return path.to_string();
    }
    // The unit whose `src/` root the file sits under (longest prefix wins).
    let mut best: Option<(&Unit, usize)> = None;
    for unit in &model.units {
        let src = if unit.path == "." {
            "src/".to_string()
        } else {
            format!("{}/src/", unit.path.trim_end_matches('/'))
        };
        if path.starts_with(&src) && best.is_none_or(|(_, len)| src.len() > len) {
            best = Some((unit, src.len()));
        }
    }
    let Some((unit, prefix)) = best else {
        // No unit source root matched: project relative to a plain `src/`
        // under the first unit — the probe's fallback law. A path under no
        // source root at all stays identity and claims through nothing else.
        let (unit, rel) = match (model.units.first(), path.strip_prefix("src/")) {
            (Some(unit), Some(rel)) => (unit, rel),
            _ => return path.to_string(),
        };
        return project_rust_stem(unit, rel);
    };
    project_rust_stem(unit, &path[prefix..])
}

/// Fold a `src/`-relative rust file into its module path: `mod.rs` folds
/// into its directory, a unit root file (`lib.rs`, `main.rs`, `bin/x.rs`)
/// folds into the unit path itself.
fn project_rust_stem(unit: &Unit, rel: &str) -> String {
    if rel == unit.root {
        return unit.name.clone();
    }
    let stem = rel.strip_suffix(".rs").unwrap_or(rel);
    let mut segs: Vec<&str> = stem.split('/').collect();
    if segs.last().copied() == Some("mod") {
        segs.pop();
    }
    if segs.is_empty() {
        return unit.name.clone();
    }
    let mut dotted = String::from(unit.name.as_str());
    for seg in segs {
        dotted.push_str("::");
        dotted.push_str(seg);
    }
    dotted
}

/// The nearest common ancestor of the cycle members over the claims union a
/// synthetic tree root (D6): the deepest claim on every member's
/// self-to-root parent chain. `None` is the synthetic root — members under
/// disjoint top-level claims, or a member no claim covers — where the emitted
/// line carries no claiming segment and names every member instead.
pub(super) fn common_claim_ancestor<'a>(
    model: &Model,
    members: &[String],
    claims: &[Claim<'a>],
) -> Option<&'a str> {
    let mut chains: Vec<Vec<usize>> = Vec::with_capacity(members.len());
    for member in members {
        let index = file_claim_index(model, member, claims)?;
        let mut chain = Vec::new();
        let mut current = Some(index);
        while let Some(step) = current {
            chain.push(step);
            current = claims[step].parent;
        }
        chains.push(chain);
    }
    chains
        .iter()
        .min_by_key(|chain| chain.len())?
        .iter()
        .find(|index| {
            chains
                .iter()
                .all(|chain| chain.iter().any(|candidate| *candidate == **index))
        })
        .map(|index| claims[*index].name)
}

/// The `::` address of a ts file endpoint: directory separators become
/// segments and the `.ts` suffix is noise. This is the identity the shared
/// scenario dialect (`driver.module_path` on a ts driver) addresses files by,
/// so `common_ancestor`, `rel_after` and the bare-name concession all read
/// ts trees in the same space as rust module paths.
pub(super) fn ts_submodule_identity(path: &str) -> String {
    submodule_identity("ts", path)
}

/// The `::` address of a file-identity endpoint on the ts, python or kotlin
/// driver: directory separators become segments and the source suffix
/// (`.ts`, `.py`, `.kt`) is noise. Rust and go never route here — their
/// module tier is stated in native module paths already.
pub(super) fn submodule_identity(language: &str, path: &str) -> String {
    let stem = match language {
        "python" => path.strip_suffix(".py").unwrap_or(path),
        "kotlin" => path.strip_suffix(".kt").unwrap_or(path),
        _ => path.strip_suffix(".ts").unwrap_or(path),
    };
    stem.replace('/', "::")
}

/// The claim index claiming one file identity — the D6 nearest claiming
/// ancestor over the claims, the same ownership the edge placement applies
/// to module-edge endpoints (`matches.modules` patterns first, the unit-tier
/// fallback beside it). File identities are projected to their module paths
/// first (territory spelling): claims address modules, never raw paths.
/// `None` states the synthetic tree root: no claim covers the file, and the
/// root is where the attribution lands (R2-5).
pub(super) fn file_claim_index(model: &Model, file: &str, claims: &[Claim<'_>]) -> Option<usize> {
    let identity = file_module_identity(model, file);
    claim_owner(&identity, claims).or_else(|| claim_unit_fallback(&identity, claims))
}

/// The files-tier finding line (D6, pinned by the design record's emitted
/// strings table): with a common claiming ancestor the line carries the
/// claiming segment `cycle detected in: <claim>: <file> -> <file> -> <file>`;
/// attributed to the synthetic root it carries none and names every member.
fn file_cycle_finding(
    model: &Model,
    members: &[String],
    path: &[String],
    claims: &[Claim<'_>],
) -> String {
    match common_claim_ancestor(model, members, claims) {
        Some(claim) => format!("{claim}: {}", path.join(" -> ")),
        None => path.join(" -> "),
    }
}

/// Cross-tier dedup by claim PROJECTION (D6, R3-4/R3-7): the file SCC is
/// suppressed iff every adjacency WITNESSED INSIDE the SCC (both endpoints
/// members) projects onto a claim edge that a cycle REPORTED by this very
/// constraint printed, and the projections form a cycle themselves. A
/// projection onto edges the claim report never showed — an unreported
/// adjacency, a containment hop, an off-territory endpoint — is not the
/// claim tier's story: the file finding prints beside it.
fn projection_reports_claim_cycle(
    model: &Model,
    members: &[String],
    file_edges: &BTreeSet<(String, String)>,
    claims: &[Claim<'_>],
    claim_cycle_edges: &BTreeSet<(String, String)>,
) -> bool {
    let mut ancestors: BTreeSet<String> = BTreeSet::new();
    let mut projected: BTreeSet<(String, String)> = BTreeSet::new();
    for (from, to) in file_edges {
        if !members.contains(from) || !members.contains(to) {
            continue;
        }
        let (Some(from_index), Some(to_index)) = (
            file_claim_index(model, from, claims),
            file_claim_index(model, to, claims),
        ) else {
            continue;
        };
        let (from_claim, to_claim) = (claims[from_index].name, claims[to_index].name);
        if from_claim == to_claim {
            continue;
        }
        if !claim_cycle_edges.contains(&(from_claim.to_string(), to_claim.to_string())) {
            // The claim tier never printed this edge — its report cannot
            // stand in for this file cycle (D6), the finding stands alone.
            return false;
        }
        ancestors.insert(from_claim.to_string());
        ancestors.insert(to_claim.to_string());
        projected.insert((from_claim.to_string(), to_claim.to_string()));
    }
    if ancestors.len() < 2 {
        return false;
    }
    strongly_connected_components(&ancestors, &projected)
        .into_iter()
        .any(|scc| scc.len() > 1)
}

/// Tarjan's strongly connected components over the induced graph, each SCC
/// sorted for deterministic iteration.
pub(super) fn strongly_connected_components(
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
                    // The closing node was pushed at entry and is only popped
                    // here, so the stack is non-empty at every pop of this loop
                    // (self-audit F5).
                    let popped = self
                        .stack
                        .pop()
                        .expect("SCC root is still on the Tarjan stack");
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
pub(super) fn find_cycle(nodes: &[String], edges: &BTreeSet<(String, String)>) -> Option<Vec<String>> {
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