// Core structural compare machinery shared by `verify`, and later `report` and
// `update`. Pipeline (design §4): map extracted units onto declared components,
// compare the extracted vs declared component sets and edge sets, render the
// verdict. Diff categories are grouped in canonical report order (output.md).

mod adjudicate;
mod boundaries;
mod claims;
mod contracts;
mod cycles;
mod diff;
mod edges;
mod engagement;
mod external;
mod mapping;
mod pattern;
mod surfaces;

pub use mapping::{Mapping, augment_with_modules, effective_module_tier, map_units, resolved_components};
pub use claims::{flatten_claims};
pub use adjudicate::{Adjudicator, ModuleRender};
pub use contracts::adjudicate_expose;
pub use mapping::file_grained_rows;
pub use pattern::matches_pattern;
pub use diff::{Diff, pass_confirmation, render_report};
pub use mapping::{count_model_edges};

use boundaries::check_feature_boundary;
use boundaries::check_forbid_submodule;
use claims::check_ambiguous_module_matches;
use contracts::check_contract_leaks;
use cycles::check_cycles;
use edges::check_allowed_edges;
use edges::check_facade_roots;
use edges::check_forbidden_laundering;
use edges::check_missing_edges;
use adjudicate::{composition_sources, resolve_module_edges};
use edges::submodule_dependency_edges;
use edges::ts_submodule_constraints;
use engagement::check_claim_engagement;
use engagement::check_reference_engagement;
use engagement::check_vacuous_constraints;
use engagement::soft_visibility_verifiable;
use external::check_external_free;
use external::check_forbid_external;
use external::check_manifest_integrity;
use surfaces::check_public_api;

use crate::archspec::model::{Model, Role};
use crate::archspec::spec::{Constraint, Module, Stereotype};
use std::collections::{BTreeSet};

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
        dead_references: Vec::new(),
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
    check_allowed_edges(&graph.pairs, &graph.value_pairs, model, modules, &claims, &mut diff);
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
    let (cycles, mut warnings) = check_cycles(&graph.pairs, model, constraints, modules, &claims);
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

    let submodule_edges = submodule_dependency_edges(model, &mapping, &claims);
    let restated_submodule_constraints: Vec<Constraint>;
    let submodule_constraints: &[Constraint] = if model.language == "ts" {
        restated_submodule_constraints = ts_submodule_constraints(constraints);
        &restated_submodule_constraints
    } else {
        constraints
    };
    let (submodules, mut warning_submodules) =
        check_forbid_submodule(&submodule_edges, submodule_constraints);
    diff.forbidden_submodule_dependencies = submodules;
    diff.warnings.append(&mut warning_submodules);

    let (mut dead_errors, mut dead_warnings) = check_reference_engagement(
        model,
        modules,
        stereotypes,
        soft_verifiable,
    );
    diff.dead_references.append(&mut dead_errors);
    diff.warnings.append(&mut dead_warnings);

    diff.vacuous_constraints =
        check_vacuous_constraints(model, modules, submodule_constraints, &graph.pairs, &submodule_edges, &claims);
    diff.vacuous_constraints.append(&mut expose_vacuous);
    // R8 vacuity carriers for the two flipped ts rows without constraint
    // rows: the rule is engaged by DECLARED surface (a forbidden ban, a
    // facade role in the roles map) yet the edges it adjudicates are EMPTY.
    // The carrier keys on the FACTS, never on the capability row's label —
    // a tree that resolves no edges must not print a green that lies about
    // a declared ban or a declared root. ts-gated: the flipped rows are ts
    // rows; rust, csharp and go keep their shipped output bytes (the
    // byte-identity law, D1).
    if matches!(model.language.as_str(), "ts" | "python" | "kotlin") {
        if model.module_edges.is_empty()
            && modules.iter().any(|module| !module.allowed.forbidden.is_empty())
        {
            diff.vacuous_constraints.push(
                "verify.forbidden_laundering: no module edges resolved — the laundering check trivially passes"
                    .to_string(),
            );
        }
        let facade_roots: BTreeSet<&String> = model
            .roles
            .iter()
            .filter(|(_, role)| matches!(role, Role::Facade))
            .map(|(root, _)| root)
            .collect();
        if !facade_roots.is_empty()
            && model.module_edges.iter().all(|edge| {
                !facade_roots.contains(&edge.to) || edge.from == edge.to
            })
        {
            diff.vacuous_constraints.push(
                "verify.root_facade: no edge enters a facade root — the facade rule trivially passes"
                    .to_string(),
            );
        }
    }
    diff.vacuous_constraints.sort();

    diff.warnings.sort();
    diff.forbidden_edges.sort();
    diff.missing_edges.sort();
    diff.disallowed_cross_component.sort();
    diff
}

#[cfg(test)]
use crate::archspec::{capability, language};

#[cfg(test)]
mod tests {
    use super::*;

    /// The weld the `file_grained_rows` doc promises: every language whose
    /// `file-grained-dependency-facts` row reads granular must project a
    /// non-empty adjacency from a model carrying rows in the shape either
    /// granular driver serializes (file-keyed `module_edges` — the ts shape;
    /// `file_edges` — the rust shape). A granular row without a projection
    /// arm here projects nothing from any model and reddens this test —
    /// the row can never silently certify a substrate the adjudication
    /// cannot read (B5 law).
    #[test]
    fn every_granular_file_facts_row_has_a_projection_arm() {
        for lang in [
            language::Language::Rust,
            language::Language::Csharp,
            language::Language::Go,
            language::Language::Ts,
            language::Language::Python,
        ] {
            if capability::emission(lang, capability::FACT_FILE_GRAINED_DEPENDENCY_FACTS)
                != Some(capability::GRANULAR)
            {
                continue;
            }
            let name = lang.as_str();
            let module_keyed = Model::from_json(&format!(
                r#"{{"schema_version":1,"language":"{name}","units":[],"edges":[],
                    "module_edges":[{{"unit":"u","from":"src/a.x","to":"src/b.x"}}]}}"#
            ))
            .expect("module_edges-shaped model loads");
            let file_keyed = Model::from_json(&format!(
                r#"{{"schema_version":1,"language":"{name}","units":[],"edges":[],
                    "file_edges":[{{"unit":"u","from":"src/a.x","to":"src/b.x","kind":"mod"}}]}}"#
            ))
            .expect("file_edges-shaped model loads");
            let projected = !file_grained_rows(&module_keyed).is_empty()
                || !file_grained_rows(&file_keyed).is_empty();
            assert!(
                projected,
                "the granular {name} row for file-grained-dependency-facts has no \
                 projection arm in file_grained_rows — the row certifies facts the \
                 adjudication cannot read"
            );
        }
    }
}
