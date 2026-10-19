//! External-purity arms: forbid_external_crates, external_free, and
//! manifest_integrity over the model's manifest view.

use std::collections::{BTreeSet};
use super::diff::push_finding;
use super::pattern::{matches_pattern, module_listed, module_path_matches, pattern_names_ts_unit, strip_unit};

use crate::archspec::model::{ManifestInfo, Model, Unit};
use crate::archspec::spec::Constraint;

/// `forbid_external_crates` (#19-#21): a module matching a `from` pattern (or
/// one of its submodules) must not import a crate matching a `forbid` pattern.
/// The owning module of each external import comes from the model's
/// `module_external` map.
pub(super) fn check_forbid_external(model: &Model, constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "forbid_external_crates")
    {
        for (module_path, crates) in &model.module_external {
            if !module_listed(module_path, &constraint.from)
                && !constraint
                    .from
                    .iter()
                    .any(|pattern| pattern_names_ts_unit(model, pattern))
            {
                continue;
            }
            for crate_name in crates {
                if constraint
                    .forbid
                    .iter()
                    .any(|pattern| matches_pattern(crate_name, pattern))
                {
                    push_finding(
                        format!("{} -> {crate_name}", strip_unit(module_path)),
                        &constraint.severity,
                        "forbidden external crate",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }
    }
    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, warnings)
}

/// Elements present in the model that a constraint `from` pattern can address:
/// unit names (the unit tier every driver emits) plus module paths wherever a
/// structure tier exists (soft paths, edge endpoints, external attributions).
/// `external_free` engagement reads presence — not externals — because an
/// attributed zero is precisely the state it monitors.
pub(super) fn present_elements(model: &Model) -> BTreeSet<String> {
    let mut elements: BTreeSet<String> = model.units.iter().map(|unit| unit.name.clone()).collect();
    elements.extend(model.soft_structure.values().flatten().cloned());
    elements.extend(model.module_external.keys().cloned());
    for edge in &model.module_edges {
        elements.insert(edge.from.clone());
        elements.insert(edge.to.clone());
    }
    // Ts file vertices without an edge or external fact still exist (see
    // check_claim_engagement): the declarations state them. The python
    // driver states its file vertices the same way (the ts D10 shape).
    if model.language == "ts" {
        for declarations in model.root_module_declarations.values() {
            elements.extend(declarations.iter().map(|decl| decl.name.clone()));
        }
    }
    if model.language == "python" {
        for declarations in model.root_module_declarations.values() {
            elements.extend(declarations.iter().map(|decl| decl.name.clone()));
        }
    }
    // The kotlin driver states its file vertices the same way (the ts D10
    // shape).
    if model.language == "kotlin" {
        for declarations in model.root_module_declarations.values() {
            elements.extend(declarations.iter().map(|decl| decl.name.clone()));
        }
    }
    elements
}

/// `external_free`: a matched module or unit must be attributed zero external
/// packages. Engagement is presence-only (see `present_elements`): a matched
/// element with no externals PASSES — a real check, not a vacuous pass — and
/// any attributed package is a violation naming the module and the packages.
/// A module-tier pattern monitors the subtree through the same
/// `module_path_matches` resolution the vacuity detector uses; a pattern
/// naming a unit monitors every external attributed anywhere under that unit
/// (unit-rooted `module_external` keys, the whole unit on a tier-less go tree).
pub(super) fn check_external_free(model: &Model, constraints: &[Constraint]) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "external_free")
    {
        for (module_path, packages) in &model.module_external {
            if packages.is_empty() {
                continue;
            }
            if !constraint
                .from
                .iter()
                .any(|pattern| {
                    module_path_matches(pattern, module_path)
                        || pattern_names_ts_unit(model, pattern)
                })
            {
                continue;
            }
            let names: BTreeSet<&str> = packages.iter().map(String::as_str).collect();
            push_finding(
                format!(
                    "{} imports {}",
                    strip_unit(module_path),
                    names.into_iter().collect::<Vec<_>>().join(", ")
                ),
                &constraint.severity,
                "not external free",
                &mut errors,
                &mut warnings,
            );
        }
    }
    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, warnings)
}

/// `manifest_integrity` (#22-#25, #53-#55): each package manifest is checked
/// individually — a violation names the manifest that fails it, and a required
/// feature must be present in every checked manifest (a sibling having it does
/// not mask a member that lacks it). Targets are the per-unit manifests when a
/// workspace scan surfaced them; otherwise the root manifest is a single
/// `Cargo.toml` target (preserves behavior for non-rust drivers).
pub(super) fn check_manifest_integrity(
    model: &Model,
    constraints: &[Constraint],
) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "manifest_integrity")
    {
        for (label, manifest) in manifest_targets(model) {
            if let Some(required) = constraint.require_publish {
                let resolved = manifest.publish.unwrap_or(false);
                if required && !resolved {
                    push_finding(
                        format!("{label} publish=false (required true)"),
                        &constraint.severity,
                        "manifest integrity",
                        &mut errors,
                        &mut warnings,
                    );
                } else if !required && resolved {
                    push_finding(
                        format!("{label} publish=true (required false)"),
                        &constraint.severity,
                        "manifest integrity",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
            for forbidden in &constraint.forbidden_dependencies {
                if manifest
                    .dependencies
                    .iter()
                    .any(|dep| matches_pattern(dep, forbidden))
                {
                    push_finding(
                        format!("{label} has forbidden dependency {forbidden}"),
                        &constraint.severity,
                        "manifest integrity",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
            for required in &constraint.required_features {
                if !manifest
                    .features
                    .iter()
                    .any(|feature| matches_pattern(feature, required))
                {
                    push_finding(
                        format!("{label} missing required feature {required}"),
                        &constraint.severity,
                        "manifest integrity",
                        &mut errors,
                        &mut warnings,
                    );
                }
            }
        }
    }
    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    (errors, warnings)
}

/// The `(label, ManifestInfo)` entries a `manifest_integrity` constraint checks,
/// deduplicated by manifest path (a package with multiple compilation targets
/// shares one manifest). When per-unit manifests are absent, falls back to the
/// root manifest as a single manifest entry. The label uses the language's
/// manifest file name (Cargo.toml for rust, csproj for csharp, go.mod for go).
pub(super) fn manifest_targets(model: &Model) -> Vec<(String, ManifestInfo)> {
    let manifest_file = |path: &str| {
        if path == "." {
            manifest_file_name(&model.language).to_string()
        } else {
            format!("{}/{}", path, manifest_file_name(&model.language))
        }
    };
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut targets: Vec<(String, ManifestInfo)> = Vec::new();
    if model.unit_manifests.is_empty() {
        if let Some(manifest) = &model.manifest {
            targets.push((manifest_file("."), manifest.clone()));
        }
        return targets;
    }
    let mut units: Vec<&Unit> = model.units.iter().collect();
    units.sort_by(|left, right| left.name.cmp(&right.name));
    for unit in units {
        let Some(manifest) = model.unit_manifests.get(&unit.name) else {
            continue;
        };
        let label = manifest_file(&unit.path);
        if seen.insert(label.clone()) {
            targets.push((label, manifest.clone()));
        }
    }
    targets
}

/// The manifest file name for a language (labels in `manifest_integrity`
/// findings). Defaults to `Cargo.toml` for unknown languages.
fn manifest_file_name(language: &str) -> &'static str {
    match language {
        "csharp" => "csproj",
        "go" => "go.mod",
        _ => "Cargo.toml",
    }
}