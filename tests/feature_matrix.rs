mod common;
#[allow(dead_code)]
mod shared;

use shared::driver::{Driver, Language};
use shared::report::{BehaviorReport, MatrixReport};
use std::path::Path;
use std::sync::OnceLock;

/// One matrix run over all drivers, shared by the report gate and the
/// diagnostic-coverage guard so every scenario executes exactly once.
static MATRIX_RUN: OnceLock<(MatrixReport, Vec<String>)> = OnceLock::new();

fn matrix_run() -> &'static (MatrixReport, Vec<String>) {
    MATRIX_RUN.get_or_init(|| {
        let drivers = Driver::all();
        let scenarios = shared::scenarios::all();
        let mut report = shared::report::empty_report();
        let mut failures: Vec<String> = Vec::new();

        for driver in &drivers {
            let lang = driver.language.as_str();
            for scenario in &scenarios {
                let feature_index = report
                    .features
                    .iter()
                    .position(|feature| feature.name == scenario.feature.as_str())
                    .expect("every scenario feature must be present in the report");
                let implemented = report.features[feature_index]
                    .language_capability
                    .get(lang)
                    .map(|capability| capability == "implemented")
                    .unwrap_or(false);

                let applies = scenario
                    .applies
                    .as_ref()
                    .map(|applies| shared::scenarios::applies_here(driver, applies))
                    .unwrap_or(true);

                let mut skip_reasons = std::collections::BTreeMap::new();
                let status = if !implemented || !applies {
                    if !implemented {
                        skip_reasons.insert(
                            lang.to_string(),
                            format!("{} not implemented", scenario.feature.as_str()),
                        );
                    } else if let Some(applies) = &scenario.applies {
                        let reason = match applies {
                            shared::scenario::Applies::Capability(fact)
                            | shared::scenario::Applies::Inert(fact) => {
                                format!("{fact}: {}", skip_row(lang, fact))
                            }
                            shared::scenario::Applies::Custom(_) => {
                                "project shape not applicable".to_string()
                            }
                        };
                        skip_reasons.insert(lang.to_string(), reason);
                    }
                    "skipped".to_string()
                } else {
                    // Heartbeat: name the cell before it runs — a stalled
                    // scenario shows up as the last MILE line, not silence.
                    eprintln!("MILE cell {lang}::{}", scenario.name);
                    let fx = common::Fixture::new();
                    // A panicking helper (for example `Driver::scan` on a
                    // tree the driver cannot scan) fails its own cell by
                    // name instead of aborting the whole matrix — silence
                    // and collateral kills are both illegal here.
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (scenario.run)(driver, &fx)
                    }));
                    match outcome {
                        Ok(result) => match result {
                            Ok(()) => "pass".to_string(),
                            Err(message) => {
                                failures.push(format!(
                                    "{} [{}] {}: {}",
                                    lang,
                                    scenario.feature.as_str(),
                                    scenario.name,
                                    message
                                ));
                                "fail".to_string()
                            }
                        },
                        Err(payload) => {
                            let message = payload
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                                .unwrap_or_else(|| "panicked without a message".to_string());
                            failures.push(format!(
                                "{} [{}] {}: panicked: {}",
                                lang,
                                scenario.feature.as_str(),
                                scenario.name,
                                message
                            ));
                            "fail".to_string()
                        }
                    }
                };

                let behavior = BehaviorReport {
                    name: scenario.name.to_string(),
                    description: scenario.description.to_string(),
                    status: std::iter::once((lang.to_string(), status)).collect(),
                    skip_reasons,
                };
                merge_behavior(&mut report.features[feature_index], behavior);
            }
        }
        (report, failures)
    })
}

/// Quote a capability table row through the table's own machine projection.
fn skip_row(language: &str, fact: &str) -> String {
    shared::capability::table()
        .emission(language, fact)
        .unwrap_or("<fact not in table>")
        .to_string()
}

/// The feature-matrix entry point: runs every shared scenario against every
/// language driver, asserts the gate (no "fail" on an implemented feature),
/// regenerates the report files into the target directory, and asserts that
/// the committed report files match the regenerated ones byte for byte.
#[test]
fn feature_matrix() {
    let (report, failures) = matrix_run();

    // Gate assertion: a failing scenario on a driver where the feature is
    // implemented fails the whole suite. "pass" and "skipped" are fine.
    // This runs before the artefact freshness check so the failing cells —
    // the primary signal — are named even when reports are also stale.
    assert!(
        failures.is_empty(),
        "feature matrix gate failed:\n{}",
        failures.join("\n")
    );

    write_reports(report);

    let mut passed = 0usize;
    let mut skipped = 0usize;
    for feature in &report.features {
        for behavior in &feature.behaviors {
            for status in behavior.status.values() {
                match status.as_str() {
                    "pass" => passed += 1,
                    "skipped" => skipped += 1,
                    _ => {}
                }
            }
        }
    }
    println!(
        "feature matrix: {} scenarios × {} drivers ({} passed, {} skipped)",
        shared::scenarios::all().len(),
        Language::ALL.len(),
        passed,
        skipped
    );
}

/// Merge a single-driver behavior into the feature's behavior list: reuse the
/// existing entry when the scenario name already ran on another driver.
fn merge_behavior(feature: &mut shared::report::FeatureReport, behavior: BehaviorReport) {
    match feature
        .behaviors
        .iter_mut()
        .find(|existing| existing.name == behavior.name)
    {
        Some(existing) => {
            for (lang, status) in &behavior.status {
                existing.status.insert(lang.clone(), status.clone());
            }
            for (lang, reason) in &behavior.skip_reasons {
                existing.skip_reasons.insert(lang.clone(), reason.clone());
            }
        }
        None => feature.behaviors.push(behavior),
    }
}

/// Regenerate the report files into a writable location and keep the
/// committed payload artifacts honest. A containerized gate leg mounts the
/// checkout read-only, so the test writes the rendered reports under the
/// target directory (`CARGO_TARGET_DIR` when set, else `<manifest>/target`)
/// and then compares each rendered report against the committed artifact in
/// the source tree — reading a read-only source is legal, writing into it is
/// not. A stale committed artifact fails here and names the regenerated copy
/// to carry over it.
fn write_reports(report: &shared::report::MatrixReport) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = match std::env::var("CARGO_TARGET_DIR") {
        Ok(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
        _ => root.join("target"),
    };
    let out = target.join("feature-matrix");
    std::fs::create_dir_all(&out).expect("create the feature-matrix output directory");
    for (name, rendered) in [
        ("feature-matrix.json", shared::report::report_to_json(report)),
        ("feature-matrix.md", shared::report::report_to_markdown(report)),
    ] {
        let regenerated = out.join(name);
        std::fs::write(&regenerated, &rendered)
            .unwrap_or_else(|e| panic!("write {}: {e}", regenerated.display()));
        let committed = root.join(name);
        let stored = std::fs::read_to_string(&committed)
            .unwrap_or_else(|e| panic!("read committed {}: {e}", committed.display()));
        assert_eq!(
            stored, rendered,
            "committed {name} is stale: copy {} over {} and commit it",
            regenerated.display(),
            committed.display(),
        );
    }
}
/// Every diagnostic category `archspec verify` can render. The labels mirror
/// the finding strings emitted by `src/archspec/verify/compare/`
/// (`render_report` and its push sites) — warning-bucket categories render
/// with a `warning: ` prefix there; the label here is the unprefixed form.
/// The scenario list pins which behaviors must produce each label.
const DIAGNOSTIC_CATEGORIES: &[(&str, &[&str])] = &[
    (
        "missing component:",
        &["undeclared_component_is_reported_missing"],
    ),
    (
        "unexpected component:",
        &["undeclared_unit_reports_unexpected_component"],
    ),
    (
        "unassigned unit:",
        &["dotted_subunit_of_declared_component_is_unassigned"],
    ),
    (
        "ambiguous module match:",
        &["two_boundaries_claim_module_at_equal_specificity"],
    ),
    (
        "forbidden edge:",
        &["undeclared_dependency_exceeds_allowed_ceiling"],
    ),
    (
        "missing edge:",
        &["stale_allowed_dependency_reports_missing_edge"],
    ),
    (
        "disallowed cross-component dependency:",
        &["undeclared_dependency_exceeds_allowed_ceiling"],
    ),
    (
        "facade dependency:",
        &["import_of_facade_root_reexport_is_violation"],
    ),
    (
        "contract leak:",
        &[
            "submodule_contract_leak_reported_under_submodule_path",
            "top_level_contract_forbid_stereotype_surfaced_by_unit",
        ],
    ),
    (
        "cycle:",
        &[
            "cyclic_tree_fails_no_cycles_constraint",
            "warning_severity_cycle_routes_to_warning_bucket",
        ],
    ),
    ("public api leak:", &["rust_root_public_api_leak_pin"]),
    (
        "unverifiable glob export:",
        &["rust_unverifiable_glob_export_pin"],
    ),
    ("empty glob export:", &["rust_empty_glob_export_pin"]),
    (
        "forbidden external crate:",
        &["forbidden_external_dependency_is_reported"],
    ),
    (
        "manifest integrity:",
        &["forbidden_dependency_in_manifest_is_reported"],
    ),
    (
        "feature boundary:",
        &["feature_boundary_gated_pattern_without_declaration_fires_where_emitted"],
    ),
    (
        "forbidden submodule dependency:",
        &["forbidden_submodule_dependency_across_sibling_children"],
    ),
    (
        "unresolved module file:",
        &["rust_unresolved_module_file_pin"],
    ),
    (
        "unowned module edge endpoint:",
        &["unclaimed_module_edge_endpoint_is_reported_unowned"],
    ),
    (
        "laundered forbidden edge:",
        &["forbidden_edge_routed_through_conduit_hop_is_laundered"],
    ),
    (
        "dead reference:",
        &["dead_reference_to_source_module_classifies_per_language"],
    ),
    (
        "vacuous constraint:",
        &[
            "vacuous_cycle_constraint_surfaces_edgeless_group",
            "feature_boundary_missing_declarations_fact_surfaces_vacuous",
        ],
    ),
];

/// Completeness guard: every diagnostic `verify/compare` can emit must actually be
/// exercised on every non-rust driver — either the producing scenario passes
/// there, or the driver is legitimately absent: its capability probe reports
/// the scenario's feature not-implemented, or the scenario's applicability
/// predicate excludes the driver (a deliberate, contract-planned divergence).
/// A category with no scenario name, a stale scenario name, a fail, or a skip
/// that is neither probe-driven nor predicate-driven fails this test.
#[test]
fn every_diagnostic_category_is_proven_on_non_rust_drivers() {
    let (report, _) = matrix_run();
    let scenarios = shared::scenarios::all();
    let drivers = Driver::all();
    let mut gaps: Vec<String> = Vec::new();

    for (label, producers) in DIAGNOSTIC_CATEGORIES {
        assert!(!producers.is_empty(), "category {label} lists no scenario");
        for driver in drivers.iter().filter(|d| d.language != Language::Rust) {
            let lang = driver.language.as_str();
            let mut covered = false;
            for name in *producers {
                let scenario = scenarios
                    .iter()
                    .find(|scenario| scenario.name == *name)
                    .unwrap_or_else(|| panic!("category {label} names unknown scenario {name}"));
                let feature = report
                    .features
                    .iter()
                    .find(|feature| feature.name == scenario.feature.as_str())
                    .unwrap_or_else(|| panic!("unknown feature for scenario {name}"));
                let status = feature
                    .behaviors
                    .iter()
                    .find(|behavior| behavior.name == *name)
                    .and_then(|behavior| behavior.status.get(lang))
                    .map(String::as_str);
                match status {
                    Some("pass") => {
                        covered = true;
                        break;
                    }
                    Some("skipped") => {
                        let capability_absent = feature
                            .language_capability
                            .get(lang)
                            .map(|capability| capability == "not-implemented")
                            .unwrap_or(false);
                        let predicate_absent = scenario
                            .applies
                            .as_ref()
                            .map(|applies| !shared::scenarios::applies_here(driver, applies))
                            .unwrap_or(false);
                        if capability_absent || predicate_absent {
                            covered = true;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if !covered {
                gaps.push(format!("{label} [{lang}]"));
            }
        }
    }

    assert!(
        gaps.is_empty(),
        "diagnostic categories not proven on non-rust drivers:\n{}",
        gaps.join("\n")
    );
}
