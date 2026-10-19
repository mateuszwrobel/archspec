//! The public_api_allowlist arm: exported public items of the allowed
//! roots, glob re-export resolution, and the file-grained projection
//! shared with the diagram engines.

use std::collections::{BTreeSet};
use super::cycles::submodule_identity;
use super::diff::push_finding;
use super::mapping::Mapping;
use super::pattern::{matches_pattern, module_matches, module_path_matches};

use crate::archspec::model::Model;
use crate::archspec::spec::{Constraint, Module, Stereotype};

/// True if a stereotype's match-set hits the submodule's surface: units the
/// enclosing claim has assigned whose name the submodule's boundary path
/// matches (submodules claim the module tier only, so the unit surface is the
/// ancestor's claimed territory seen through the boundary), or a module path
/// inside the boundary matched by the full-path/bare-name rule, or — ts only,
/// C3 — a source file inside the boundary: the ts module tree is file-backed,
/// its surface is the set of file identities the scan observed (module-edge
/// endpoints and export rows), and a boundary pattern addressing a file
/// classifies it through the same full-path/bare-name rule.
pub(super) fn submodule_surface_matches(
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
        || (matches!(model.language.as_str(), "ts" | "python" | "kotlin")
            && ts_surface_files(model).iter().any(|file| {
                // The ts territory convention mirrors the rust module tree:
                // a file `b.ts` and the directory `b/` of its nested parts
                // are one territory — restated in the `::` space, the shared
                // ancestor matcher sees the nesting the same way (C3). The
                // stereotype keeps the file-identity space: its pattern
                // matches the raw file path (the identity the model states)
                // or the restated `::` path (name and spelled-path forms).
                // The python driver shares the file-identity tier (the D10
                // shape), so the same restatement reads both trees.
                let restated = submodule_identity(&model.language, file);
                boundary.iter().any(|pattern| {
                    module_path_matches(&submodule_identity(&model.language, pattern), &restated)
                        && stereotype
                            .match_set
                            .names
                            .iter()
                            .chain(stereotype.match_set.paths.iter())
                            .chain(stereotype.match_set.units.iter())
                            .any(|stereotype_pattern| {
                                module_matches(stereotype_pattern, &restated)
                                    || matches_pattern(file, stereotype_pattern)
                            })
                })
            }))
}

/// The file identities a ts tree's scan observed: every module-edge endpoint
/// plus every export row's key — the honest file surface of a file-backed
/// module tree (C3). Test-gated files are no different here: the tier law
/// rides the edge and claim tiers, not a surface drop.
fn ts_surface_files(model: &Model) -> BTreeSet<String> {
    let mut files: BTreeSet<String> = BTreeSet::new();
    for edge in &model.module_edges {
        files.insert(edge.from.clone());
        files.insert(edge.to.clone());
    }
    files.extend(model.module_public_types.keys().cloned());
    files
}

pub(super) fn stereotype_matches_unit(value: &str, stereotype: &Stereotype) -> bool {
    stereotype
        .match_set
        .names
        .iter()
        .chain(stereotype.match_set.paths.iter())
        .chain(stereotype.match_set.units.iter())
        .any(|pattern| matches_pattern(value, pattern))
}

/// `public_api_allowlist` (#16-#18, amended by the glob-enumeration plan):
/// every public item exported from a crate root must be in the constraint's
/// `allowed` list. Reserved root plumbing (non-public items) never counts as an
/// export (see `root_public_exports`). Resolved root globs land in that same
/// enumerated set; globs the scan could not resolve are reported as
/// unverifiable, and globs resolving to an empty surface fail closed — neither
/// is silently skipped.
pub(super) fn check_public_api(
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