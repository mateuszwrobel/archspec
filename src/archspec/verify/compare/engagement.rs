//! Constraint engagement: vacuity detection (a check that guarded an
//! empty domain), unmatched-claim warnings, and dead-reference
//! resolution with the nearest-name diagnostics.

use std::collections::{BTreeSet};
use super::claims::{Claim, ModuleRef, entry_specificity};
use super::cycles::file_claim_index;
use super::external::{manifest_targets, present_elements};
use super::mapping::file_grained_rows;
use super::pattern::{common_ancestor, matches_pattern, module_listed, module_matches, module_path_matches, rel_after, strip_unit};
use super::claims::flatten_claims;
use crate::archspec::{capability, language};
use crate::archspec::spec;
use crate::archspec::model::{Model, ModuleEdge};
use crate::archspec::spec::{Constraint, Module, Stereotype};

/// Engagement diagnostics over the claim set at any depth (D9a/D9b), run
/// model-side so languages without module-tier facts are covered too:
/// (a) a claim whose `matches` resolve to no model element is an
///     `unmatched claim` — except where the missing-component error already
///     names it, so a claim is never reported twice;
/// (b) a claim whose every claimed model path an equal-or-more-specific
///     other claim also covers is a `shadowed claim` — its rules can never
///     fire differently than the shadow's, exactly the silence this whole
///     change exists to close.
pub(super) fn check_claim_engagement(
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
    // The ts driver states file vertices in its declarations even when a
    // file has no internal edge — an external-only or export-only file is
    // still a module the spec can honestly claim. The family languages
    // carry those vertices in soft_structure, which the loop above already
    // reads; for ts the declarations are that carrier.
    if model.language == "ts" {
        for declarations in model.root_module_declarations.values() {
            paths.extend(declarations.iter().map(|decl| decl.name.as_str()));
        }
    }
    // The python driver states its file vertices the same way (the ts D10
    // precedent): a file with no internal edge is still a claimable module.
    // The kotlin driver does NOT: it states its modules as units and
    // gradle-path module rows (K-P3), so engagement reads the unit and
    // module-edge tier only — the capability row reads not-emitted.
    if model.language == "python" {
        for declarations in model.root_module_declarations.values() {
            paths.extend(declarations.iter().map(|decl| decl.name.as_str()));
        }
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
pub(super) fn check_vacuous_constraints(
    model: &Model,
    modules: &[Module],
    constraints: &[Constraint],
    module_edges: &BTreeSet<(String, String)>,
    submodule_edges: &[ModuleEdge],
    claims: &[Claim<'_>],
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
            // non-empty (a cycle check over no edges trivially passes). With
            // the file tier in scope (D7) the carrier keys on FACTS, not the
            // capability row's label: granular facts that are EMPTY print
            // the empty-adjacency detail; absent facts (row not-emitted)
            // print the capability-naming detail — posture-derived scope
            // included, never silence.
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
                let posture = language::from_name(&model.language).map(capability::posture);
                let scope = constraint
                    .scope
                    .as_deref()
                    .or(posture.map(|(scope, _)| scope))
                    .unwrap_or(capability::SCOPE_CLAIMS);
                let mut details: Vec<String> = Vec::new();
                if !engaged {
                    details.push(
                        "constraint's modules have no dependency edges among them \
                         (cycle check trivially passes)"
                            .to_string(),
                    );
                }
                if scope == capability::SCOPE_BOTH {
                    let granular = language::from_name(&model.language).is_some_and(|driver| {
                        capability::emission(driver, capability::FACT_FILE_GRAINED_DEPENDENCY_FACTS)
                            == Some(capability::GRANULAR)
                    });
                    if granular {
                        let file_engaged =
                            file_grained_rows(model).into_iter().any(|(from, to)| {
                                from != to
                                    && [from, to].iter().all(|file| {
                                        constraint.modules.is_empty()
                                            || file_claim_index(model, file, claims)
                                                .is_some_and(|index| group.contains(claims[index].name))
                                    })
                            });
                        if !file_engaged {
                            details.push(
                                "no file-grained dependency edges among the constraint's files \
                                 (file-tier cycle check trivially passes)"
                                    .to_string(),
                            );
                        }
                    } else {
                        details.push(format!(
                            "driver emits no {} fact (capability {}): the file-tier cycle \
                             check cannot engage — see `archspec help capabilities`",
                            capability::FACT_FILE_GRAINED_DEPENDENCY_FACTS,
                            capability::NOT_EMITTED
                        ));
                    }
                }
                if details.is_empty() {
                    None
                } else {
                    Some(details.join("; "))
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
                } else if language::from_name(&model.language) == Some(language::Language::Go)
                    || language::from_name(&model.language)
                        == Some(language::Language::Python)
                {
                    // D9 (workplan archspec_module_public_api_diagrams): go has
                    // no crate, so the rust sentence was wrong vocabulary on a
                    // go tree. The neutral arm names capability, tier, and the
                    // remedy pointer — no driver-specific token. The python arm
                    // joins it for the same reason: a distribution root states
                    // no public-API fact (the python driver record parks
                    // `__all__`).
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
                    Some(capability::DECLARATIONS_NO_GATING) => Some(format!(
                        "driver emits declarations without gating facts (capability {}): \
                         the constraint cannot engage",
                        capability::DECLARATIONS_NO_GATING
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
pub(super) fn soft_visibility_verifiable(model: &Model) -> bool {
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
/// Each such reference becomes a `dead reference` entry, split by class:
///
/// - exact declared module / stereotype name -> engaged, no finding;
/// - name resolves to something present in the source model (a module path, a
///   unit) but declared nowhere -> "exists in source but undeclared", warning;
/// - otherwise, when the driver can see soft module structure at all ->
///   "does not exist", plus a did-you-mean candidate (the closest declared
///   name within a small edit distance) where one exists — ERROR severity
///   since the ts-driver plan D10/R3-11 promotion (the returned first vector;
///   the shipped shape was warning);
/// - otherwise -> "not verifiable from source" — the capability table says
///   this run carries no module-tier fact, so non-existence cannot be claimed,
///   warning.
///
/// Returns (error bodies without the category label, fully rendered warning
/// entries).
///
/// `constraint.modules` references are NOT reported here: the schema validator
/// already hard-errors on undeclared module names there. `contract.forbid`
/// references are collected recursively from submodules too — submodule
/// contracts are enforced, so their targets are checked like top-level ones.
/// Entries are rendered as `<category>: <detail>` and deduplicated/sorted for
/// a byte-stable report.
pub(super) fn check_reference_engagement(
    model: &Model,
    modules: &[Module],
    stereotypes: &[Stereotype],
    soft_verifiable: bool,
) -> (Vec<String>, Vec<String>) {
    let declared: BTreeSet<String> = spec::claim_names(modules).into_iter().collect();
    let declared: BTreeSet<&str> = declared.iter().map(String::as_str).collect();
    let declared_stereotypes: BTreeSet<&str> = stereotypes
        .iter()
        .map(|stereotype| stereotype.name.as_str())
        .collect();
    let mut errors: BTreeSet<String> = BTreeSet::new();
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
            .map(|entry| ("allowed.depend_on", entry.target.as_str()))
            .chain(
                module
                    .allowed
                    .forbidden
                    .iter()
                    .map(|entry| ("allowed.forbidden", entry.target.as_str())),
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
                // The `does not exist` class — error severity since the
                // ts-driver plan D10/R3-11 promotion. Body only: the
                // renderer and the report attach the category label.
                errors.insert(format!(
                    "module '{}' {field} target \"{target}\" does not exist — \
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
    (errors.into_iter().collect(), found.into_iter().collect())
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
pub(super) fn did_you_mean(target: &str, candidates: &BTreeSet<&str>) -> String {
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
