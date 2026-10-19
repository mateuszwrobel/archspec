//! Containment arms: feature_boundary over gated modules and
//! forbid_submodule_dependency over sibling submodules.

use std::collections::{BTreeSet};
use super::diff::push_finding;
use super::pattern::{common_ancestor, matches_pattern, module_matches, module_path_matches, rel_after, strip_unit};
use crate::archspec::{capability, language};
use crate::archspec::model::{Model, ModuleEdge};
use crate::archspec::spec::Constraint;

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
pub(super) fn check_feature_boundary(model: &Model, constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let driver_emits_declarations = !matches!(
        language::from_name(&model.language).and_then(|driver_language| {
            capability::emission(driver_language, capability::FACT_ROOT_MODULE_DECLARATIONS)
        }),
        Some(capability::NOT_EMITTED | capability::DECLARATIONS_NO_GATING)
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
pub(super) fn check_forbid_submodule(edges: &[ModuleEdge], constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
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