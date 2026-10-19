//! Diff verdict data and the verify report renderer: the difference
//! carrier in canonical report order, its error-tier field table, and the
//! text rendering the verify and report commands print.

use std::collections::{BTreeSet};
use super::claims::count_claims;
use crate::archspec::spec::Module;

/// Every difference across all checks, in canonical report order (ADR-010,
/// output.md). Each field is a sorted list of the rendered item bodies.
#[derive(Debug, Clone, Default)]
pub struct Diff {
    /// Declared in the spec, absent from the source model.
    pub missing_components: Vec<String>,
    /// Present in the source model, not declared in the spec.
    pub unexpected_components: Vec<String>,
    /// Extracted unit matched by no declared component.
    pub unassigned_units: Vec<String>,
    /// Soft module path claimed by two declared boundaries with equal
    /// specificity (prefix grouping overlap); ownership is reported, not
    /// guessed. Each entry names the path and both claiming entries.
    pub ambiguous_module_matches: Vec<String>,
    /// Edge present in the source but banned by `allowed.forbidden`.
    pub forbidden_edges: Vec<String>,
    /// Edge declared by `allowed.depend_on`, absent from the source.
    pub missing_edges: Vec<String>,
    /// Edge crossing components without being in `allowed.depend_on`.
    pub disallowed_cross_component: Vec<String>,
    /// Structural publication-only root facade violation: an internal module
    /// depends on its own crate-root unit whose root file defines nothing (only
    /// `mod` declarations + re-exports). Such an edge launders any ban through
    /// the umbrella, so it fails regardless of `depend_on` (the fix is to
    /// canonicalize the import, not to declare the edge).
    pub facade_dependencies: Vec<String>,
    /// Forbidden stereotype exposed by a component's own units.
    pub contract_leaks: Vec<String>,
    /// `no_cycles` constraint violated over the declared groups.
    pub cycles: Vec<String>,
    /// `public_api_allowlist` violation: a public export not in `allowed`.
    pub public_api_leaks: Vec<String>,
    /// `public_api_allowlist` violation: a glob re-export at a unit root whose
    /// exported set cannot be resolved from source (external crate, unknown
    /// path, cfg-blocked declaration or chain link, poisoned chain), so it is
    /// reported as unverifiable rather than silently skipped. Globs that do
    /// resolve contribute their enumerated names to `public_api_leaks`.
    pub unverifiable_glob_exports: Vec<String>,
    /// `public_api_allowlist` violation: a glob re-export at a unit root that
    /// resolves to a same-crate module exporting zero public items. An empty
    /// derived set would silently drop a checked surface, so it fails closed.
    pub empty_glob_exports: Vec<String>,
    /// `forbid_external_crates` violation: a listed module imports a banned crate.
    pub forbidden_external_crates: Vec<String>,
    /// `external_free` violation: a matched module/unit is attributed external
    /// packages — the zero-dependency purity guard fired.
    pub not_external_free: Vec<String>,
    /// `manifest_integrity` violation: manifest diverges from declared requirements.
    pub manifest_integrity: Vec<String>,
    /// `feature_boundary` violation: unallowed dep on a gated module, a gated
    /// module missing its cfg, or an unknown gated-module pattern.
    pub feature_boundaries: Vec<String>,
    /// `forbid_submodule_dependency` violation: a listed submodule depends on a
    /// forbidden sibling within a parent.
    pub forbidden_submodule_dependencies: Vec<String>,
    /// A dead reference whose target names nothing in the model — the
    /// `does not exist` class (a removed claim, a forward-declared target
    /// never created). Error severity since the ts-driver plan D10/R3-11
    /// promotion: verify fails WITHOUT `--strict` and prints the line
    /// without the `warning: ` prefix (the shipped shape was
    /// warning-severity, rc-neutral). The other dead-reference
    /// classifications stay warning entries in `warnings`. Entries store
    /// the message body; the renderer attaches the `dead reference: `
    /// category label, exactly like the other error fields.
    pub dead_references: Vec<String>,
    /// Warning-level items (design §9): divergences whose constraint declared
    /// `severity = "warning"`. Currently only `no_cycles` carries a severity.
    /// Each entry is already fully rendered as `<category>: <detail>` (e.g.
    /// `cycle: A -> B -> A`); the report prefixes it with `warning: ` unless
    /// `--strict` promotes it to an error line. Listed after all error items.
    pub warnings: Vec<String>,
    /// Vacuous-guard findings (plan_model_driven_workflow.md, Feature 2): a
    /// constraint whose effective domain is empty — no module/edge/manifest/
    /// export to check — reported so a silent pass is never mistaken for a real
    /// check. Always warning-severity regardless of the constraint's declared
    /// severity. Each entry is pre-rendered as `[constraint #N] <type>: <detail>`
    /// (no `vacuous constraint:` prefix); the report adds that prefix, plus a
    /// `warning: ` prefix unless `--strict` promotes it. Listed last.
    pub vacuous_constraints: Vec<String>,
}

impl Diff {
    /// True if no error-level divergence and no warning-level divergence.
    pub fn is_empty(&self) -> bool {
        !self.has_errors() && self.warnings.is_empty() && self.vacuous_constraints.is_empty()
    }

    /// Run verdict: an error-level divergence always fails; a warning-level
    /// divergence (including a vacuous constraint) fails only under `--strict`
    /// (which promotes it to an error).
    pub fn fails(&self, strict: bool) -> bool {
        self.has_errors()
            || (strict && (!self.warnings.is_empty() || !self.vacuous_constraints.is_empty()))
    }

    /// Every error-tier field in canonical report order (ADR-010), each paired
    /// with the stable category label its entries render under (the same label
    /// `help diagnostics` catalogs and `report/output.md` declares). This is
    /// the single source of both the exit gate (`has_errors` below) and the
    /// report's error findings: a field counted by the verdict is a field the
    /// renderers print, so a non-zero exit can never coexist with a clean
    /// claim — the silent-red weld. Warning-tier fields (`warnings`,
    /// `vacuous_constraints`) never gate without `--strict` and are not
    /// listed here; the renderers consume them separately.
    pub fn error_fields(&self) -> Vec<(&'static str, &[String])> {
        vec![
            ("missing component", &self.missing_components),
            ("unexpected component", &self.unexpected_components),
            ("unassigned unit", &self.unassigned_units),
            ("ambiguous module match", &self.ambiguous_module_matches),
            ("forbidden edge present", &self.forbidden_edges),
            ("missing edge", &self.missing_edges),
            ("disallowed cross-component dependency", &self.disallowed_cross_component),
            ("facade dependency", &self.facade_dependencies),
            ("contract leak", &self.contract_leaks),
            ("cycle detected in", &self.cycles),
            ("public api leak", &self.public_api_leaks),
            ("unverifiable glob export", &self.unverifiable_glob_exports),
            ("empty glob export", &self.empty_glob_exports),
            ("forbidden external crate", &self.forbidden_external_crates),
            ("not external free", &self.not_external_free),
            ("manifest integrity", &self.manifest_integrity),
            ("feature boundary", &self.feature_boundaries),
            ("forbidden submodule dependency", &self.forbidden_submodule_dependencies),
            ("dead reference", &self.dead_references),
        ]
    }

    /// True if any error-level item is present (severity defaults to error;
    /// every category except `no_cycles` is error-level by declaration). An
    /// error-level item is by definition a field of `error_fields` — there is
    /// no second list of gated fields.
    fn has_errors(&self) -> bool {
        self.error_fields()
            .iter()
            .any(|(_, entries)| !entries.is_empty())
    }
}

/// Route a finding into the error bucket (detail only; the report adds the
/// `<category>: ` prefix) or, for a `warning`-severity constraint, into the
/// pre-rendered warning bucket as `<category>: <detail>`.
pub(super) fn push_finding(
    detail: String,
    severity: &str,
    category: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if severity == "warning" {
        warnings.push(format!("{category}: {detail}"));
    } else {
        errors.push(detail);
    }
}

/// Pass verdict: short deterministic confirmation on stdout.
pub fn pass_confirmation(modules: &[Module], constraints_checked: usize) -> String {
    format!(
        "ok: architecture.spec.toml matches source model ({} modules, {} constraints checked)\n",
        count_claims(modules),
        constraints_checked
    )
}

/// Full diff report: header plus every divergence, canonically ordered and
/// byte-stable (output.md). Error items keep the plain `<category>: <detail>`
/// lines; warning items (design §9) are listed after all errors, each prefixed
/// with `warning: ` unless `--strict` promotes them to error lines. `strict`
/// only changes how warnings are labelled and whether the run fails — it does
/// not reorder or drop any item.
pub fn render_report(diff: &Diff, strict: bool) -> String {
    let vacuous_only =
        !diff.has_errors() && diff.warnings.is_empty() && !diff.vacuous_constraints.is_empty();
    let mut out = String::new();
    if vacuous_only {
        // A run whose only findings are vacuous constraints never claims a
        // match; the header names each vacuous guard instead.
        let tokens: BTreeSet<&str> = diff
            .vacuous_constraints
            .iter()
            .map(|entry| entry.split(':').next().unwrap_or(entry).trim())
            .collect();
        let mut tokens: Vec<&str> = tokens.into_iter().collect();
        tokens.sort();
        out.push_str(&format!(
            "architecture.spec.toml has vacuous constraints: {}\n",
            tokens.join(", ")
        ));
    } else {
        out.push_str("architecture.spec.toml does not match source model\n");
    }
    for name in &diff.missing_components {
        out.push_str(&format!("  missing component: {name}\n"));
    }
    for name in &diff.unexpected_components {
        out.push_str(&format!("  unexpected component: {name}\n"));
    }
    for name in &diff.unassigned_units {
        out.push_str(&format!("  unassigned unit: {name}\n"));
    }
    for entry in &diff.ambiguous_module_matches {
        out.push_str(&format!("  ambiguous module match: {entry}\n"));
    }
    for edge in &diff.forbidden_edges {
        out.push_str(&format!("  forbidden edge: {edge}\n"));
    }
    for edge in &diff.missing_edges {
        out.push_str(&format!("  missing edge: {edge}\n"));
    }
    for edge in &diff.disallowed_cross_component {
        out.push_str(&format!(
            "  disallowed cross-component dependency: {edge}\n"
        ));
    }
    for edge in &diff.facade_dependencies {
        out.push_str(&format!("  facade dependency: {edge}\n"));
    }
    for leak in &diff.contract_leaks {
        out.push_str(&format!("  contract leak: {leak}\n"));
    }
    for cycle in &diff.cycles {
        out.push_str(&format!("  cycle: {cycle}\n"));
    }
    for leak in &diff.public_api_leaks {
        out.push_str(&format!("  public api leak: {leak}\n"));
    }
    for glob in &diff.unverifiable_glob_exports {
        out.push_str(&format!("  unverifiable glob export: {glob}\n"));
    }
    for glob in &diff.empty_glob_exports {
        out.push_str(&format!("  empty glob export: {glob}\n"));
    }
    for crate_use in &diff.forbidden_external_crates {
        out.push_str(&format!("  forbidden external crate: {crate_use}\n"));
    }
    for impurity in &diff.not_external_free {
        out.push_str(&format!("  not external free: {impurity}\n"));
    }
    for manifest in &diff.manifest_integrity {
        out.push_str(&format!("  manifest integrity: {manifest}\n"));
    }
    for boundary in &diff.feature_boundaries {
        out.push_str(&format!("  feature boundary: {boundary}\n"));
    }
    for submodule in &diff.forbidden_submodule_dependencies {
        out.push_str(&format!("  forbidden submodule dependency: {submodule}\n"));
    }
    for reference in &diff.dead_references {
        out.push_str(&format!("  dead reference: {reference}\n"));
    }
    for warning in &diff.warnings {
        if strict {
            out.push_str(&format!("  {warning}\n"));
        } else {
            out.push_str(&format!("  warning: {warning}\n"));
        }
    }
    for vacuous in &diff.vacuous_constraints {
        if strict {
            out.push_str(&format!("  vacuous constraint: {vacuous}\n"));
        } else {
            out.push_str(&format!("  warning: vacuous constraint: {vacuous}\n"));
        }
    }
    out
}