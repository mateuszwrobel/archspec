// Report rendering (design §9, report/output.md): the extracted model vs the
// declared spec, as a human-readable report. `verify` gates; `report` explains
// and propagates rule violations through the process exit status.
// Text and markdown carry identical content (same metrics, same diff, same
// ordering); only the markup differs. Everything is sorted/canonical so output
// is byte-stable across runs (report/output.md determinism).

use crate::archspec::model::Model;
use crate::archspec::spec::Module;
use crate::archspec::verify::compare::{self, Diff, Mapping};
use serde::Serialize;
use std::borrow::Cow;

/// Model-derived metrics plus per-category violation counts. Metrics are read
/// straight off the extracted model and the mapping — no scoring, no extra
/// computation (report/output.md). Serialized verbatim by `--format json`
/// (informational; the canonical model JSON stays `scan`'s output).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Metrics {
    /// Extracted components: distinct assigned declared components plus added
    /// (unexpected) components.
    pub components: usize,
    /// Extracted units (hard boundaries).
    pub units: usize,
    /// Production edges internal to the project (unit→unit plus unit-internal
    /// module edges).
    pub edges_internal: usize,
    /// External dependency edges (unit→external targets).
    pub edges_external: usize,
    /// Distinct dependency cycles found (error- and warning-severity).
    pub cycles: usize,
    /// Violations in the components category (missing, unexpected, unassigned).
    pub violations_components: usize,
    /// Violations in the edges category (forbidden, missing, disallowed,
    /// facade).
    pub violations_edges: usize,
    /// Violations in the contracts category (contract leaks).
    pub violations_contracts: usize,
    /// Violations in the cycles category.
    pub violations_cycles: usize,
}

impl Metrics {
    /// Total violations across all categories.
    pub fn total(&self) -> usize {
        self.violations_components
            + self.violations_edges
            + self.violations_contracts
            + self.violations_cycles
    }
}

/// Compute metrics from the extracted model and the diff. The mapping is
/// augmented with the soft (module) tier so declared module boundaries count as
/// components. Edge metrics mirror the model's scan tiers verbatim: internal is
/// the production unit→unit tier plus unit-internal module edges; external is
/// the unit→external tier. Module-level external uses remain in their own tier
/// and are not folded into the external edge count.
pub fn compute_metrics(model: &Model, mapping: &Mapping, modules: &[Module], diff: &Diff) -> Metrics {
    let augmented = compare::augment_with_modules(mapping, model, modules);
    let components =
        compare::resolved_components(&augmented).len() + augmented.unexpected_components.len();
    let (internal, external) = compare::count_model_edges(model);
    let cycles = diff.cycles.len()
        + diff
            .warnings
            .iter()
            .filter(|warning| warning.starts_with("cycle: "))
            .count();
    Metrics {
        components,
        units: model.units.len(),
        edges_internal: internal,
        edges_external: external,
        cycles,
        violations_components: diff.missing_components.len()
            + diff.unexpected_components.len()
            + diff.unassigned_units.len()
            + diff.ambiguous_module_matches.len(),
        violations_edges: diff.forbidden_edges.len()
            + diff.missing_edges.len()
            + diff.disallowed_cross_component.len()
            + diff.facade_dependencies.len(),
        violations_contracts: diff.contract_leaks.len(),
        violations_cycles: cycles,
    }
}

/// One report finding: the stable category id (the same label the text and
/// markdown renderers print, and the `help diagnostics` catalog names), the
/// severity (`error` for error-level divergences, `warning` for warning-level
/// entries and vacuous guards), and the rendered message body. This triple is
/// the machine-readable contract of `report --format json` (report/output.md);
/// the prose inside `message` may evolve, the category ids may not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub category: String,
    pub severity: &'static str,
    pub message: String,
}

/// The full diff as findings in canonical report order (ADR-010): components
/// first, then edges, then contracts, then cycles, then warnings, then vacuous
/// guards. Warning entries carry their own pre-rendered category label; warning
/// cycles render exactly like error-level ones. `report` is informational and
/// has no `--strict` gate. All three renderers consume this single source.
pub fn findings(diff: &Diff) -> Vec<Finding> {
    let mut items: Vec<Finding> = Vec::new();
    {
        let mut push = |category: &'static str, message: String| {
            items.push(Finding {
                category: category.to_string(),
                severity: "error",
                message,
            })
        };
        for name in &diff.missing_components {
            push("missing component", name.clone());
        }
        for name in &diff.unexpected_components {
            push("unexpected component", name.clone());
        }
        for name in &diff.unassigned_units {
            push("unassigned unit", name.clone());
        }
        for entry in &diff.ambiguous_module_matches {
            push("ambiguous module match", entry.clone());
        }
        for edge in &diff.forbidden_edges {
            push("forbidden edge present", edge.clone());
        }
        for edge in &diff.missing_edges {
            push("missing edge", edge.clone());
        }
        for edge in &diff.disallowed_cross_component {
            push("disallowed cross-component dependency", edge.clone());
        }
        for edge in &diff.facade_dependencies {
            push("facade dependency", edge.clone());
        }
        for leak in &diff.contract_leaks {
            push("contract leak", leak.clone());
        }
        for cycle in &diff.cycles {
            push("cycle detected in", cycle.clone());
        }
    }
    // Warning entries are pre-rendered as `<category>: <detail>`. A warning
    // cycle renders exactly like an error-level cycle (report is informational
    // and has no `--strict` gate); every other category keeps its own label.
    for warning in &diff.warnings {
        let (category, message) = match warning.split_once(": ") {
            Some(("cycle", body)) => ("cycle detected in".to_string(), body.to_string()),
            Some((category, body)) => (category.to_string(), body.to_string()),
            None => ("warning".to_string(), warning.clone()),
        };
        items.push(Finding {
            category,
            severity: "warning",
            message,
        });
    }
    // Vacuous-guard entries are pre-rendered `[constraint #N] <type>: <detail>`;
    // report is informational and lists them verbatim, like any finding. They
    // are always warning-severity, so they never gate the run.
    for vacuous in &diff.vacuous_constraints {
        items.push(Finding {
            category: "vacuous constraint".to_string(),
            severity: "warning",
            message: vacuous.clone(),
        });
    }
    items
}

/// The findings as report-style (prefix, body) lines — the shared iteration
/// behind the text and markdown diff sections.
fn diff_items(diff: &Diff) -> Vec<(Cow<'static, str>, String)> {
    findings(diff)
        .into_iter()
        .map(|finding| (Cow::Owned(finding.category), finding.message))
        .collect()
}

/// Plain-text report (report/output.md example format). Every report contains
/// header, metrics, diff, and summary; the category table only appears when
/// there are violations.
pub fn render_text(language: &str, metrics: &Metrics, diff: &Diff) -> String {
    let mut out = String::new();
    out.push_str("Architecture report\n");
    out.push_str(&"=".repeat("Architecture report".len()));
    out.push_str("\n\n");
    out.push_str(&format!(
        "Spec: architecture.spec.toml (language: {language})\n\n"
    ));

    out.push_str("Metrics\n");
    out.push_str(&"-".repeat("Metrics".len()));
    out.push('\n');
    for (label, value) in metric_rows(metrics) {
        out.push_str(&format!("{:<20} {value}\n", format!("{label}:")));
    }
    out.push('\n');

    if metrics.total() > 0 {
        out.push_str("Violations by category\n");
        out.push_str(&"-".repeat("Violations by category".len()));
        out.push('\n');
        for (label, value) in category_rows(metrics) {
            if value > 0 {
                out.push_str(&format!("{:<10} {value}\n", format!("{label}:")));
            }
        }
        out.push('\n');
    }

    out.push_str("Diff: extracted vs declared\n");
    out.push_str(&"-".repeat("Diff: extracted vs declared".len()));
    out.push('\n');
    let items = diff_items(diff);
    if items.is_empty() {
        out.push_str("no violations found\n");
    } else {
        for (prefix, body) in items {
            out.push_str(&format!("{prefix}: {body}\n"));
        }
    }
    out.push('\n');
    out.push_str(&summary(metrics, "Result:"));
    out
}

/// Markdown report: same content as the text report, rendered as `#` header,
/// metric/category tables, a list for the diff, and a bold result line. When a
/// file-level import map is available, `diagram` embeds it as a mermaid fence;
/// forbidden edges/disallowed cross-component deps and cycles get dedicated
/// sections between the category table and the diff.
pub fn render_markdown(
    language: &str,
    metrics: &Metrics,
    diff: &Diff,
    diagram: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str("# Architecture report\n\n");
    out.push_str(&format!(
        "Spec: `architecture.spec.toml` (language: {language})\n\n"
    ));

    out.push_str("## Metrics\n\n");
    out.push_str("| Metric | Value |\n");
    out.push_str("|---|---|\n");
    for (name, value) in metric_rows(metrics) {
        out.push_str(&format!("| {name} | {value} |\n"));
    }
    out.push('\n');

    if metrics.total() > 0 {
        out.push_str("## Violations by category\n\n");
        out.push_str("| Category | Count |\n");
        out.push_str("|---|---|\n");
        for (name, value) in category_rows(metrics) {
            if value > 0 {
                out.push_str(&format!("| {name} | {value} |\n"));
            }
        }
        out.push('\n');
    }

    if let Some(diagram) = diagram {
        out.push_str("## Diagram\n\n");
        out.push_str("```mermaid\n");
        out.push_str(diagram);
        out.push_str("\n```\n\n");
    }

    let forbidden: Vec<&str> = diff
        .forbidden_edges
        .iter()
        .map(String::as_str)
        .chain(diff.disallowed_cross_component.iter().map(String::as_str))
        .chain(diff.facade_dependencies.iter().map(String::as_str))
        .collect();
    if !forbidden.is_empty() {
        out.push_str("## Forbidden imports\n\n");
        for item in &forbidden {
            out.push_str(&format!("- `{item}`\n"));
        }
        out.push('\n');
    }

    let cycles: Vec<&str> = diff
        .cycles
        .iter()
        .map(String::as_str)
        .chain(
            diff.warnings
                .iter()
                .filter_map(|warning| warning.strip_prefix("cycle: ")),
        )
        .collect();
    if !cycles.is_empty() {
        out.push_str("## Cycles\n\n");
        for cycle in &cycles {
            out.push_str(&format!("- `{cycle}`\n"));
        }
        out.push('\n');
    }

    out.push_str("## Diff: extracted vs declared\n\n");
    let items = diff_items(diff);
    if items.is_empty() {
        out.push_str("no violations found\n\n");
    } else {
        for (prefix, body) in items {
            out.push_str(&format!("- {prefix}: `{body}`\n"));
        }
        out.push('\n');
    }
    out.push_str(&summary(metrics, "**Result:**"));
    out
}

/// Machine-readable report: the same summary + diff the text renderers consume,
/// serialized. `findings` is the contract (category ids match the
/// `help diagnostics` catalog); `metrics` is informational and mirrors the scan
/// model tiers. The canonical model JSON stays `scan`'s output — the report
/// JSON embeds no second model encoding (workplan_report_json_format decisions).
pub fn render_json(language: &str, metrics: &Metrics, diff: &Diff) -> String {
    #[derive(Serialize)]
    struct ReportJson<'a> {
        language: &'a str,
        metrics: &'a Metrics,
        findings: Vec<Finding>,
    }
    let report = ReportJson {
        language,
        metrics,
        findings: findings(diff),
    };
    format!("{}\n", serde_json::to_string_pretty(&report).expect("report serializes"))
}

/// Metric rows in canonical order. The label doubles as the text-render
/// left column and the markdown table name.
fn metric_rows(metrics: &Metrics) -> Vec<(&'static str, usize)> {
    vec![
        ("components", metrics.components),
        ("units", metrics.units),
        ("edges (internal)", metrics.edges_internal),
        ("edges (external)", metrics.edges_external),
        ("cycles detected", metrics.cycles),
    ]
}

/// Category rows in canonical order (components, edges, contracts, cycles).
fn category_rows(metrics: &Metrics) -> Vec<(&'static str, usize)> {
    vec![
        ("components", metrics.violations_components),
        ("edges", metrics.violations_edges),
        ("contracts", metrics.violations_contracts),
        ("cycles", metrics.violations_cycles),
    ]
}

/// Summary line: `Result: clean` when nothing differs, otherwise the total
/// violation count plus the edge/cycle breakdown (report/output.md).
fn summary(metrics: &Metrics, result_marker: &str) -> String {
    if metrics.total() == 0 {
        return format!("{result_marker} clean\n");
    }
    format!(
        "{result_marker} {} ({}, {})\n",
        plural(metrics.total(), "violation"),
        plural(metrics.violations_edges, "edge"),
        plural(metrics.violations_cycles, "cycle"),
    )
}

fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archspec::commands::help::DIAGNOSTICS_HELP;

    /// A diff with exactly one entry in every finding field, warnings rendered
    /// the way the producers pre-render them. The renderers iterate this shape,
    /// so the labels it yields are the full emitted-category contract.
    fn populated_diff() -> Diff {
        Diff {
            missing_components: vec!["Ghost".into()],
            unexpected_components: vec!["Portal".into()],
            unassigned_units: vec!["app::orphan".into()],
            ambiguous_module_matches: vec![
                "app::start claimed by both entries (equal specificity)".into(),
            ],
            forbidden_edges: vec!["a -> b".into()],
            missing_edges: vec!["billing -> auth".into()],
            disallowed_cross_component: vec!["p -> q".into()],
            facade_dependencies: vec!["app::engine -> app".into()],
            contract_leaks: vec!["app exposes entity (forbidden)".into()],
            cycles: vec!["a -> b -> a".into()],
            public_api_leaks: vec!["app exposes X (not allowlisted)".into()],
            unverifiable_glob_exports: vec!["app exposes core::types::*".into()],
            empty_glob_exports: vec![
                "app exposes empty_mod::* (resolves to no public items)".into(),
            ],
            forbidden_external_crates: vec!["app -> clap".into()],
            not_external_free: vec!["core imports Newtonsoft.Json".into()],
            manifest_integrity: vec!["crates/app/Cargo.toml has forbidden dependency clap".into()],
            feature_boundaries: vec!["app depends on gated module x (feature: f)".into()],
            forbidden_submodule_dependencies: vec!["o::common -> o::control_loop".into()],
            warnings: vec![
                "cycle: a -> b -> a".into(),
                "dead reference: module 'app' allowed.depend_on target \"ghost\" does not exist"
                    .into(),
                "laundered forbidden edge: a -> b via hidden".into(),
                "unresolved module file: app::ghost".into(),
                "unowned module edge endpoint: app::hidden".into(),
            ],
            vacuous_constraints: vec![
                "[constraint #1] forbid_external_crates: 'from' pattern matches nothing".into(),
            ],
        }
    }

    /// Categories the verify renderer prints (`--strict` rendering, so warning
    /// lines carry their own label instead of the `warning: ` prefix).
    fn verify_emitted_labels() -> Vec<String> {
        compare::render_report(&populated_diff(), true)
            .lines()
            .filter_map(|line| line.strip_prefix("  "))
            .filter_map(|line| line.split_once(": ").map(|(label, _)| label.to_string()))
            .collect()
    }

    /// Categories the report renderer prints, straight from the (prefix, body)
    /// list its renderers iterate.
    fn report_emitted_labels() -> Vec<String> {
        let mut labels: Vec<String> = diff_items(&populated_diff())
            .into_iter()
            .map(|(prefix, _)| prefix.into_owned())
            .collect();
        labels.sort();
        labels.dedup();
        labels
    }

    #[test]
    fn every_emitted_category_is_in_the_diagnostics_catalog() {
        let mut labels = verify_emitted_labels();
        labels.extend(report_emitted_labels());
        labels.sort();
        labels.dedup();
        assert!(labels.len() > 15, "expected the full emitted-category set");
        for label in &labels {
            assert!(
                DIAGNOSTICS_HELP.contains(label.as_str()),
                "emitted category \"{label}\" is missing from `help diagnostics`:\n{DIAGNOSTICS_HELP}"
            );
        }
    }

    #[test]
    fn every_verify_emitted_category_is_in_the_output_docs() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let docs = std::fs::read_to_string(format!(
            "{manifest}/docs/archspec/commands/verify/output.md"
        ))
        .expect("verify output docs");
        for label in verify_emitted_labels() {
            assert!(
                docs.contains(label.as_str()),
                "verify-emitted category \"{label}\" is missing from verify/output.md"
            );
        }
    }

    /// The diff-section lines of a rendered text report as (category, message).
    fn text_findings(text: &str) -> Vec<(String, String)> {
        text.lines()
            .skip_while(|line| !line.starts_with("Diff: extracted vs declared"))
            .skip(2)
            .take_while(|line| !line.trim().is_empty())
            .filter_map(|line| {
                line.split_once(": ")
                    .map(|(category, body)| (category.to_string(), body.to_string()))
            })
            .collect()
    }

    #[test]
    fn json_findings_equal_the_text_rendered_findings() {
        let diff = populated_diff();
        let metrics = Metrics::default();
        let rendered = render_json("rust", &metrics, &diff);
        let value: serde_json::Value = serde_json::from_str(&rendered).expect("json parses");
        let json_findings: Vec<(String, String)> = value["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .map(|finding| {
                (
                    finding["category"].as_str().expect("category").to_string(),
                    finding["message"].as_str().expect("message").to_string(),
                )
            })
            .collect();
        assert_eq!(json_findings, text_findings(&render_text("rust", &metrics, &diff)));
        assert_eq!(value["language"].as_str(), Some("rust"));
    }

    #[test]
    fn finding_severities_split_error_and_warning_levels() {
        let items = findings(&populated_diff());
        for finding in &items {
            assert!(
                matches!(finding.severity, "error" | "warning"),
                "unknown severity {:?}",
                finding.severity
            );
        }
        let has = |category: &str, severity: &str| {
            items
                .iter()
                .any(|f| f.category == category && f.severity == severity)
        };
        assert!(has("missing component", "error"));
        assert!(has("contract leak", "error"));
        assert!(has("cycle detected in", "error"), "error-level cycle");
        assert!(
            has("cycle detected in", "warning"),
            "warning cycle keeps the cycle category"
        );
        assert!(has("dead reference", "warning"));
        assert!(has("vacuous constraint", "warning"));
    }
}
