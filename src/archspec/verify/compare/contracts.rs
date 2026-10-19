//! Contract leaks and the expose chain: stereotype contract enforcement
//! per component and the module-public-api expose adjudication that
//! feeds the public api leak category.

use std::collections::{BTreeMap, BTreeSet};
use super::claims::Claim;
use super::engagement::did_you_mean;
use super::mapping::Mapping;
use super::pattern::{matches_pattern, module_path_matches};
use super::surfaces::{stereotype_matches_unit, submodule_surface_matches};
use super::claims::flatten_claims;
use crate::archspec::model::Model;
use crate::archspec::spec::{Module, Stereotype};


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
pub(super) fn check_contract_leaks(
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
pub(super) fn claim_covers_path(claim: &Claim<'_>, path: &str, model: &Model) -> bool {
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