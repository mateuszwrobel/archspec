mod common;

use common::{stderr, stdout};
use std::collections::BTreeSet;

/// Extract a metric value from a text report line like `components:          2`.
/// The first matching line wins; metrics render before categories, so names
/// shared between sections resolve to the metrics row.
fn metric_value(text: &str, name: &str) -> String {
    text.lines()
        .find(|line| line.starts_with(&format!("{name}:")))
        .and_then(|line| line.split(':').nth(1))
        .map(str::trim)
        .unwrap_or_else(|| panic!("metric {name:?} missing from report:\n{text}"))
        .to_string()
}

/// Extract a count from the `Violations by category` section, e.g. `edges:     1`.
fn category_value(text: &str, name: &str) -> String {
    text.lines()
        .skip_while(|line| !line.starts_with("Violations by category"))
        .skip(2)
        .find(|line| line.starts_with(&format!("{name}:")))
        .and_then(|line| line.split(':').nth(1))
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

/// Metric values dictated by the scan model. Metric definition: internal counts
/// every production unit→unit edge between in-tree units (including cross-crate
/// Rust edges) plus unit-internal module edges; external counts unit→external
/// targets only. Module-level external targets are not folded into external.
fn scan_model_edge_metrics(fixture: &common::Fixture) -> (usize, usize) {
    let scan = fixture.run(&["scan"]);
    assert_eq!(scan.status.code(), Some(0), "stderr: {}", stderr(&scan));
    let model: serde_json::Value = serde_json::from_str(&stdout(&scan)).expect("scan JSON");
    let units: BTreeSet<&str> = model["units"]
        .as_array()
        .expect("model units array")
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    let mut unit_edges = 0usize;
    for edge in model["edges"].as_array().expect("model edges array") {
        let from = edge["from"].as_str().expect("edge from");
        let to = edge["to"].as_str().expect("edge to");
        assert!(
            units.contains(from) && units.contains(to),
            "hard edge {from} -> {to} is not unit→unit between in-tree units: units {units:?}"
        );
        unit_edges += 1;
    }
    let module_edges = model["module_edges"]
        .as_array()
        .expect("model module_edges array")
        .len();
    let external = model["external"]
        .as_array()
        .expect("model external array")
        .len();
    (unit_edges + module_edges, external)
}

/// Two crates where billing depends on auth; the spec declares both components
/// and allows the edge. Sources match the spec exactly.
fn matching_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n",
    );
    fixture
}

/// Three crates where billing depends on portal; the spec explicitly forbids
/// the billing -> portal edge. The ONLY divergence is the forbidden edge.
fn forbidden_edge_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\", \"crates/portal\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nportal = { path = \"../portal\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "crates/portal/Cargo.toml",
        "[package]\nname = \"portal\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/portal/src/lib.rs", "pub fn portal() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\nforbidden = [\"portal\"]\n\n[[module]]\nname = \"portal\"\nmatches = { units = [\"portal\"] }\n",
    );
    fixture
}

/// Several divergences at once: a missing declared component (ghost), an
/// unexpected crate (extra), and a forbidden edge (billing -> portal).
fn several_violations_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\", \"crates/portal\", \"crates/extra\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\nportal = { path = \"../portal\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "crates/portal/Cargo.toml",
        "[package]\nname = \"portal\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/portal/src/lib.rs", "pub fn portal() {}\n");
    fixture.write(
        "crates/extra/Cargo.toml",
        "[package]\nname = \"extra\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/extra/src/lib.rs", "pub fn extra() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\nforbidden = [\"portal\"]\n\n[[module]]\nname = \"portal\"\nmatches = { units = [\"portal\"] }\n\n[[module]]\nname = \"ghost\"\nmatches = { units = [\"ghost\"] }\n",
    );
    fixture
}

/// Three crates: `shared` owns core_a and core_b, and `app` depends on core_a.
/// Both hard edges are unit→unit edges in the scanned tree, so the metric
/// definition changed to count them as production internal edges.
fn metrics_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/core_a\", \"crates/core_b\", \"crates/app\"]\n",
    );
    fixture.write(
        "crates/core_a/Cargo.toml",
        "[package]\nname = \"core_a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore_b = { path = \"../core_b\" }\n",
    );
    fixture.write("crates/core_a/src/lib.rs", "pub fn a() {}\n");
    fixture.write(
        "crates/core_b/Cargo.toml",
        "[package]\nname = \"core_b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/core_b/src/lib.rs", "pub fn b() {}\n");
    fixture.write(
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore_a = { path = \"../core_a\" }\n",
    );
    fixture.write("crates/app/src/lib.rs", "pub fn app() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"shared\"\nmatches = { units = [\"core_a\", \"core_b\"] }\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"shared\"]\n",
    );
    fixture
}

/// The code contains only `auth`; the spec additionally declares a `ghost`
/// component that no unit matches.
fn missing_component_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"ghost\"\nmatches = { units = [\"ghost\"] }\n",
    );
    fixture
}

/// A Go tree whose `auth/store` package nests under the declared `auth`
/// component but matches no declared component: an unassigned sub-unit.
fn unassigned_unit_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\n");
    fixture.write("auth/auth.go", "package auth\n");
    fixture.write("auth/store/store.go", "package store\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\"] }\n",
    );
    fixture
}

/// A single crate (`app`) with four top-level modules whose cross-dependencies
/// form module-tier edges. The spec declares four module boundaries via
/// `matches.modules`, exactly matching the source.
fn module_boundaries_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "mod auth;\nmod billing;\nmod config;\nmod portal;\n",
    );
    fixture.write("src/billing.rs", "use crate::auth;\nuse crate::config;\npub fn bill() {}\n");
    fixture.write("src/auth.rs", "use crate::portal;\npub fn auth() {}\n");
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture.write("src/portal.rs", "pub fn portal() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\", \"config\"]\n\n[[module]]\nname = \"auth\"\nmatches = { modules = [\"app::auth\"] }\n\n[module.allowed]\ndepend_on = [\"portal\"]\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"app::config\"] }\n\n[[module]]\nname = \"portal\"\nmatches = { modules = [\"app::portal\"] }\n",
    );
    fixture
}

#[test]
fn report_scenario_twenty_two_module_boundaries_count_as_components() {
    let fixture = module_boundaries_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "module-boundary project must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert_eq!(
        metric_value(&out, "components"),
        "4",
        "components must count the declared module boundaries, not the single crate:\n{out}"
    );
    assert_eq!(metric_value(&out, "units"), "1");
    assert_eq!(
        metric_value(&out, "edges (internal)"),
        "3",
        "all module-tier edges are unit-internal (single crate):\n{out}"
    );
    assert_eq!(
        metric_value(&out, "edges (external)"),
        "0",
        "no true external crates:\n{out}"
    );
    assert!(out.contains("Result: clean"), "summary:\n{out}");
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

#[test]
fn report_unit_tier_workspace_metrics_follow_scan_tiers() {
    let fixture = metrics_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    // This previously asserted the old boundary-keyed split. The scan model
    // proves the workspace has two production unit→unit edges and no
    // unit→external targets, so both edges are internal under the new contract.
    let (model_internal, model_external) = scan_model_edge_metrics(&fixture);
    assert_eq!(
        (model_internal, model_external),
        (2, 0),
        "fixture model truth"
    );
    assert_eq!(metric_value(&out, "components"), "2");
    assert_eq!(metric_value(&out, "units"), "3");
    assert_eq!(
        metric_value(&out, "edges (internal)"),
        model_internal.to_string()
    );
    assert_eq!(
        metric_value(&out, "edges (external)"),
        model_external.to_string()
    );
    assert!(out.contains("Result: clean"), "summary:\n{out}");
}

/// Two crates in a mutual dependency cycle; the spec allows both edges and
/// declares a `no_cycles` constraint at error severity. The ONLY divergence is
/// the cycle.
fn cycle_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[module.allowed]\ndepend_on = [\"billing\"]\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"auth\", \"billing\"]\nseverity = \"error\"\n",
    );
    fixture
}

#[test]
fn report_scenario_one_clean_project_exits_zero_with_metrics() {
    let fixture = matching_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "clean project must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("Architecture report"), "header:\n{out}");
    assert!(
        out.contains("Spec: architecture.spec.toml (language: rust)"),
        "spec line:\n{out}"
    );
    assert!(out.contains("Metrics"), "metrics section:\n{out}");
    // Metric definition changed: `billing -> auth` is a production unit→unit
    // edge between in-tree crates, so it is internal even though the crates
    // belong to different components. There are no unit→external targets.
    let (model_internal, model_external) = scan_model_edge_metrics(&fixture);
    assert_eq!(
        (model_internal, model_external),
        (1, 0),
        "fixture model truth"
    );
    assert_eq!(metric_value(&out, "components"), "2");
    assert_eq!(metric_value(&out, "units"), "2");
    assert_eq!(
        metric_value(&out, "edges (internal)"),
        model_internal.to_string()
    );
    assert_eq!(
        metric_value(&out, "edges (external)"),
        model_external.to_string()
    );
    assert_eq!(metric_value(&out, "cycles detected"), "0");
    assert!(out.contains("no violations found"), "diff:\n{out}");
    assert!(out.contains("Result: clean"), "summary:\n{out}");
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

#[test]
fn report_scenario_two_markdown_carries_same_metrics_and_diff() {
    let fixture = matching_fixture();
    let text = fixture.run(&["report"]);
    let md = fixture.run(&["report", "--format", "markdown"]);

    assert_eq!(text.status.code(), Some(0), "text run must exit 0");
    assert_eq!(
        md.status.code(),
        Some(0),
        "markdown run must exit 0 (stderr: {})",
        stderr(&md)
    );
    let text_out = stdout(&text);
    let md_out = stdout(&md);
    assert!(
        md_out.contains("# Architecture report"),
        "md header:\n{md_out}"
    );
    assert!(
        md_out.contains("Spec: `architecture.spec.toml` (language: rust)"),
        "md spec line:\n{md_out}"
    );
    assert!(md_out.contains("## Metrics"), "md metrics:\n{md_out}");
    for metric in [
        "components",
        "units",
        "edges (internal)",
        "edges (external)",
        "cycles detected",
    ] {
        let value = metric_value(&text_out, metric);
        assert!(
            md_out.contains(&format!("| {metric} | {value} |")),
            "md must carry {metric}={value}:\n{md_out}"
        );
    }
    assert!(md_out.contains("no violations found"), "md diff:\n{md_out}");
    assert!(
        md_out.contains("**Result:** clean"),
        "md summary:\n{md_out}"
    );
    assert!(stderr(&md).is_empty(), "no stderr on success");
}

#[test]
fn report_scenario_three_forbidden_edge_is_listed_and_counted() {
    let fixture = forbidden_edge_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "violations demand a non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("forbidden edge present: billing -> portal"),
        "diff line:\n{out}"
    );
    assert!(out.contains("Violations by category"), "categories:\n{out}");
    assert_eq!(category_value(&out, "edges"), "1");
    assert!(
        out.contains("Result: 1 violation (1 edge, 0 cycles)"),
        "summary:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on violation run");
}

#[test]
fn report_scenario_four_output_file_contains_report_and_stdout_empty() {
    let fixture = forbidden_edge_fixture();
    let output = fixture.run(&["report", "--output", "report.md"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "violations demand a non-zero exit (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "stdout must stay empty with --output"
    );
    let file = fixture.read("report.md");
    assert!(file.contains("Architecture report"), "file header:\n{file}");
    assert!(
        file.contains("forbidden edge present: billing -> portal"),
        "file diff:\n{file}"
    );
}

#[test]
fn report_scenario_five_several_violations_exit_nonzero() {
    let fixture = several_violations_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "violations demand a non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("missing component: ghost"), "missing:\n{out}");
    assert!(
        out.contains("unexpected component: extra"),
        "unexpected:\n{out}"
    );
    assert!(
        out.contains("forbidden edge present: billing -> portal"),
        "forbidden edge:\n{out}"
    );
    assert_eq!(category_value(&out, "components"), "2");
    assert_eq!(category_value(&out, "edges"), "1");
    assert!(
        out.contains("Result: 3 violations (1 edge, 0 cycles)"),
        "summary:\n{out}"
    );
}

#[test]
fn report_scenario_six_unchanged_project_is_byte_identical() {
    let fixture = several_violations_fixture();
    let first = fixture.run(&["report"]);
    let second = fixture.run(&["report"]);

    assert_eq!(
        first.status.code(),
        Some(1),
        "violations demand a non-zero exit"
    );
    assert_eq!(
        second.status.code(),
        Some(1),
        "violations demand a non-zero exit"
    );
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "report must be byte-identical across runs"
    );
}

#[test]
fn report_scenario_seven_markdown_output_is_byte_identical() {
    let fixture = several_violations_fixture();
    let first = fixture.run(&["report", "--format", "markdown", "--output", "r.md"]);
    assert_eq!(
        first.status.code(),
        Some(1),
        "violations demand a non-zero exit"
    );
    let first_content = fixture.read("r.md");

    let second = fixture.run(&["report", "--format", "markdown", "--output", "r.md"]);
    assert_eq!(
        second.status.code(),
        Some(1),
        "violations demand a non-zero exit"
    );
    let second_content = fixture.read("r.md");

    assert_eq!(
        first_content, second_content,
        "markdown report file must be byte-identical across runs"
    );
}

#[test]
fn report_scenario_eight_metrics_count_unit_tier_edges_across_components() {
    let fixture = metrics_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    // Both `app -> core_a` and `core_a -> core_b` are in-tree unit→unit edges.
    // Component membership no longer moves those edges into external.
    let (model_internal, model_external) = scan_model_edge_metrics(&fixture);
    assert_eq!(
        (model_internal, model_external),
        (2, 0),
        "fixture model truth"
    );
    assert_eq!(metric_value(&out, "components"), "2");
    assert_eq!(metric_value(&out, "units"), "3");
    assert_eq!(
        metric_value(&out, "edges (internal)"),
        model_internal.to_string()
    );
    assert_eq!(
        metric_value(&out, "edges (external)"),
        model_external.to_string()
    );
    assert_eq!(metric_value(&out, "cycles detected"), "0");
    assert!(out.contains("Result: clean"), "summary:\n{out}");
}

#[test]
fn report_scenario_nine_declared_component_with_no_units_is_listed() {
    let fixture = missing_component_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "missing component demands a non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("missing component: ghost"), "diff:\n{out}");
    assert_eq!(category_value(&out, "components"), "1");
    assert!(
        out.contains("Result: 1 violation (0 edges, 0 cycles)"),
        "summary:\n{out}"
    );
}

#[test]
fn report_scenario_ten_unit_not_covered_by_any_component_is_listed() {
    let fixture = unassigned_unit_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "unassigned unit demands a non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("unassigned unit: example.com/demo/auth/store"),
        "diff:\n{out}"
    );
    assert_eq!(category_value(&out, "components"), "1");
    assert!(
        out.contains("Result: 1 violation (0 edges, 0 cycles)"),
        "summary:\n{out}"
    );
}

#[test]
fn report_scenario_eleven_cycle_is_listed_and_counted() {
    let fixture = cycle_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "cycle demands a non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("cycle detected in: auth -> billing -> auth"),
        "diff:\n{out}"
    );
    assert_eq!(metric_value(&out, "cycles detected"), "1");
    assert_eq!(category_value(&out, "cycles"), "1");
    assert!(
        out.contains("Result: 1 violation (0 edges, 1 cycle)"),
        "summary:\n{out}"
    );
}

#[test]
fn report_rejects_unknown_flag() {
    let fixture = matching_fixture();
    let output = fixture.run(&["report", "--wat"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("unknown flag: --wat"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn report_rejects_two_positional_paths() {
    let fixture = matching_fixture();
    let output = fixture.run(&["report", "crates/auth", "crates/billing"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("expected at most one path argument"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn report_rejects_missing_path() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["report", "/no/such/dir"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path does not exist: /no/such/dir"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn report_rejects_regular_file_path() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\n");
    let output = fixture.run(&["report", "Cargo.toml"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path is not a directory: Cargo.toml"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn report_rejects_directory_without_spec() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = []\n");
    let output = fixture.run(&["report"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("spec file not found:") && err.contains("architecture.spec.toml"),
        "must name the expected spec file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn report_rejects_invalid_spec() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = []\n");
    fixture.write("architecture.spec.toml", "[project\nlanguage = \"rust\"\n");
    let output = fixture.run(&["report"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("architecture.spec.toml")
            && err.contains("malformed TOML"),
        "must name the spec file and the reason:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn report_rejects_unsupported_format() {
    let fixture = matching_fixture();
    let output = fixture.run(&["report", "--format", "html"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("unsupported format: html (supported: text, markdown, json)"),
        "stderr: {}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn report_rejects_directory_without_sources_in_declared_language() {
    let fixture = common::Fixture::new();
    fixture.write("architecture.spec.toml", "[project]\nlanguage = \"rust\"\n");
    let output = fixture.run(&["report"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("no rust sources found under:"),
        "must say no sources, the language, and where:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn report_missing_driver_suggests_doctor() {
    let fixture = unassigned_unit_fixture();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_archspec"))
        .arg("report")
        .current_dir(&fixture.root)
        .env("ARCHSPEC_DISABLE_DRIVERS", "go")
        .output()
        .expect("run archspec");

    assert_ne!(output.status.code(), Some(0));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("go driver unavailable (toolchain not found)")
            && err.contains("run 'archspec doctor' to diagnose"),
        "must name the language and suggest doctor:\n{err}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).is_empty(),
        "no report on operational error"
    );
}

#[test]
fn report_parse_failure_names_file_and_does_not_create_output() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "fn broken( {\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n",
    );
    let output = fixture.run(&["report", "--output", "out.md"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("failed to parse source:") && err.contains("lib.rs"),
        "must name the failing file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no partial report");
    assert!(
        !fixture.path("out.md").exists(),
        "output file must not be created on failure"
    );
}

#[test]
fn report_markdown_renders_violation_diff_and_category_table() {
    let fixture = forbidden_edge_fixture();
    let text = fixture.run(&["report"]);
    let md = fixture.run(&["report", "--format", "markdown"]);

    assert_eq!(
        text.status.code(),
        Some(1),
        "violations demand a non-zero exit"
    );
    assert_eq!(
        md.status.code(),
        Some(1),
        "violations demand a non-zero exit"
    );
    let text_out = stdout(&text);
    let md_out = stdout(&md);

    // Category table present with the same counts.
    let edge_count = category_value(&text_out, "edges");
    assert!(
        md_out.contains("| Category | Count |")
            && md_out.contains(&format!("| edges | {edge_count} |")),
        "md category table must carry the edges count:\n{md_out}"
    );

    // Diff item rendered in markdown (backtick body).
    assert!(
        md_out.contains("- forbidden edge present: `billing -> portal`"),
        "md must render the diff item:\n{md_out}"
    );

    // Summary matches text.
    assert!(
        md_out.contains("**Result:**") && !md_out.contains("Result: clean"),
        "md summary must report violations:\n{md_out}"
    );
    assert!(
        !text_out.contains("no violations found"),
        "text must NOT be clean for the forbidden-edge fixture:\n{text_out}"
    );
}

#[test]
fn report_missing_edge_is_listed_and_counted() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    // Spec declares a required edge billing -> auth that the code lacks.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\nallowed = { depend_on = [\"auth\"] }\n",
    );
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "missing edge demands a non-zero exit"
    );
    let out = stdout(&output);
    assert!(
        out.contains("missing edge: billing -> auth"),
        "report must print the canonical `missing edge:` label, exactly as verify does:\n{out}"
    );
    assert!(
        !out.contains("allowed edge absent"),
        "the retired report-side label must never render:\n{out}"
    );
    let edge_count = category_value(&out, "edges");
    assert!(edge_count != "0", "missing edge must be counted:\n{out}");
}

// The missing-edge finding has one canonical name (`missing edge`, as verify
// renders it and the diagnostics catalog documents it). The report renderer
// once printed it as `allowed edge absent`; no source file may keep printing
// or mentioning that retired string.
#[test]
fn no_source_file_prints_the_retired_allowed_edge_absent_label() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let src = std::path::Path::new(manifest).join("src");
    let mut offenders: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(&src)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let body = std::fs::read_to_string(entry.path()).expect("read source file");
        if body.contains("allowed edge absent") {
            offenders.push(
                entry
                    .path()
                    .strip_prefix(manifest)
                    .unwrap_or(entry.path())
                    .display()
                    .to_string(),
            );
        }
    }
    assert!(
        offenders.is_empty(),
        "`allowed edge absent` must not exist in src, found in: {offenders:?}"
    );
}

/// A single `app` crate with a `commands` subtree and a `config` module whose
/// submodule references the crate-root type, producing crate-root module edges
/// (`app::commands::index -> app`) alongside an intra-module edge
/// (`app::commands::index -> app::config`). Three module edges in total, two of
/// which touch the crate root.
fn crate_root_edges_report_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub struct App;\nmod commands;\nmod config;\n");
    fixture.write("src/commands/mod.rs", "pub mod index;\n");
    fixture.write(
        "src/commands/index.rs",
        "use crate::App;\nuse crate::config;\npub fn index() {}\n",
    );
    fixture.write("src/config.rs", "use crate::App;\npub fn config() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"app::commands\"] }\n\n[module.allowed]\ndepend_on = [\"app\", \"config\"]\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"app::config\"] }\n\n[module.allowed]\ndepend_on = [\"app\"]\n",
    );
    fixture
}

/// report #23: all three module edges must be counted — none silently dropped
/// (#50). Every module edge lives inside the single `app` crate, so each is
/// unit-internal: `edges_internal` = 3 and `edges_external` = 0 (no external
/// crates).
#[test]
fn report_crate_root_edges_all_counted_no_silent_drop() {
    let fixture = crate_root_edges_report_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    let internal: usize = metric_value(&out, "edges (internal)").parse().expect("internal count");
    let external: usize = metric_value(&out, "edges (external)").parse().expect("external count");
    assert_eq!(
        internal + external,
        3,
        "all three module edges (two touching the crate root) must be counted:\n{out}"
    );
    assert_eq!(
        internal, 3,
        "every module edge is internal (all endpoints share the app crate):\n{out}"
    );
    assert_eq!(external, 0, "no true external crates:\n{out}");
    assert_eq!(metric_value(&out, "components"), "3");
    assert!(out.contains("Result: clean"), "summary:\n{out}");
}

/// report #24: a single crate whose modules `a` and `b` are each declared as
/// their OWN boundary. The `a -> b` module edge must be counted INTERNAL — both
/// endpoints belong to the same crate (`app`) — not external. Internal/external
/// is keyed on the owning unit, not boundary-declaration granularity.
#[test]
fn report_internal_edges_keyed_on_unit_not_boundary() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub fn a() {}\n");
    fixture.write("src/b.rs", "use crate::a;\npub fn b() { a::a(); }\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { modules = [\"app::a\"] }\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"app::b\"] }\n\n[module.allowed]\ndepend_on = [\"a\"]\n",
    );
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    let internal: usize = metric_value(&out, "edges (internal)").parse().expect("internal count");
    let external: usize = metric_value(&out, "edges (external)").parse().expect("external count");
    assert!(
        internal >= 1,
        "a -> b is unit-internal and must be counted internal:\n{out}"
    );
    assert_eq!(
        external, 0,
        "no true external crates; the module edge must NOT be external:\n{out}"
    );
    assert!(out.contains("Result: clean"), "summary:\n{out}");
}

/// report #25: a single crate with one internal module edge (`app::b -> app::a`)
/// AND three true external crates (serde, tokio, clap). `edges_internal` counts
/// the unit-internal edge; `edges_external` reflects the external crates — the
/// two are distinguishable and not conflated.
#[test]
fn report_internal_and_external_distinguishable() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\ntokio = \"1\"\nclap = \"4\"\n",
    );
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub fn a() {}\n");
    fixture.write("src/b.rs", "use crate::a;\npub fn b() { a::a(); }\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { modules = [\"app::a\"] }\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"app::b\"] }\n\n[module.allowed]\ndepend_on = [\"a\"]\n",
    );
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert_eq!(
        metric_value(&out, "edges (internal)"),
        "1",
        "the unit-internal module edge must be counted internal:\n{out}"
    );
    assert_eq!(
        metric_value(&out, "edges (external)"),
        "3",
        "edges_external reflects the three true external crates, not internal edges:\n{out}"
    );
}

#[test]
fn report_markdown_embeds_import_map_diagram() {
    let fixture = matching_fixture();
    let output = fixture.run(&["report", "--format", "markdown"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "markdown run must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("## Diagram"), "diagram section:\n{out}");
    assert!(
        out.contains("```mermaid"),
        "mermaid code fence must be embedded:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

#[test]
fn report_markdown_marks_forbidden_imports() {
    let fixture = forbidden_edge_fixture();
    let output = fixture.run(&["report", "--format", "markdown"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "markdown violations demand a non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("## Forbidden imports"),
        "forbidden imports section:\n{out}"
    );
    assert!(
        out.contains("- `billing -> portal`"),
        "forbidden edge bullet:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on violation run");
}

#[test]
fn report_markdown_marks_cycles() {
    let fixture = cycle_fixture();
    let output = fixture.run(&["report", "--format", "markdown"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "markdown violations demand a non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("## Cycles"), "cycles section:\n{out}");
    assert!(
        out.contains("- `auth -> billing -> auth`"),
        "cycle bullet:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on violation run");
}

#[test]
fn report_text_has_no_enrichment_sections() {
    let fixture = matching_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "text run must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        !out.contains("## Diagram"),
        "plain text must not contain a Diagram section:\n{out}"
    );
    assert!(
        !out.contains("## Forbidden imports"),
        "plain text must not contain a Forbidden imports section:\n{out}"
    );
    assert!(
        !out.contains("## Cycles"),
        "plain text must not contain a Cycles section:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

#[test]
fn report_markdown_omits_empty_violation_sections() {
    let fixture = matching_fixture();
    let output = fixture.run(&["report", "--format", "markdown"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "markdown run must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("## Diagram"), "diagram section:\n{out}");
    assert!(
        !out.contains("## Forbidden imports"),
        "clean project must not list forbidden imports:\n{out}"
    );
    assert!(
        !out.contains("## Cycles"),
        "clean project must not list cycles:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

/// Single crate whose spec carries a `depend_on` target (`storage`) naming no
/// declared module: the ONLY finding is a dead-reference warning. No cycle, so
/// any cycles category is a mislabel.
fn dead_reference_report_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"storage\"]\n",
    );
    fixture
}

/// Mutual auth <-> billing dependency forms a cycle monitored by a warning-severity
/// `no_cycles` constraint; `auth` additionally `depend_on`s an undeclared `ghost`.
/// The only two findings are one warning-severity cycle and one dead reference,
/// so the report must keep them in distinct categories.
fn warning_cycle_plus_dead_reference_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[module.allowed]\ndepend_on = [\"billing\", \"ghost\"]\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"auth\", \"billing\"]\nseverity = \"warning\"\n",
    );
    fixture
}

// A dead-reference warning must render under its own `dead reference` category,
// never relabeled as a cycle. With no cycle present, no cycles category may
// appear and the cycles metric must stay 0.
#[test]
fn report_dead_reference_renders_under_own_category_not_cycle() {
    let fixture = dead_reference_report_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report is informational, exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "own category:\n{out}");
    assert!(out.contains("\"storage\""), "names the target:\n{out}");
    assert!(
        !out.contains("cycle detected in:"),
        "a dead reference must not be relabeled as a cycle:\n{out}"
    );
    assert_eq!(
        metric_value(&out, "cycles detected"),
        "0",
        "a dead reference is not a cycle and must not count as one:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

// Warning-severity findings keep their categories apart: a warning cycle renders
// under the cycles category while a dead reference renders under its own, and the
// cycles metric counts only the cycle.
#[test]
fn report_warning_findings_keep_categories_apart() {
    let fixture = warning_cycle_plus_dead_reference_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report is informational, exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("cycle detected in: auth -> billing -> auth"),
        "warning cycle renders under cycles category:\n{out}"
    );
    assert!(out.contains("dead reference:"), "dead reference category:\n{out}");
    assert!(out.contains("\"ghost\""), "names the dead target:\n{out}");
    assert!(
        !out.contains("cycle detected in: module 'auth'"),
        "the dead reference must not be relabeled as a cycle:\n{out}"
    );
    assert_eq!(
        metric_value(&out, "cycles detected"),
        "1",
        "only the warning cycle counts as a cycle, not the dead reference:\n{out}"
    );
    assert_eq!(
        category_value(&out, "cycles"),
        "1",
        "cycles category counts the cycle alone:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

#[test]
fn report_violating_pair_exits_nonzero_text() {
    let fixture = forbidden_edge_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("Result: 1 violation"),
        "report must still carry the result line:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "stderr: {}", stderr(&output));
}

#[test]
fn report_violating_pair_exits_nonzero_markdown() {
    let fixture = forbidden_edge_fixture();
    let output = fixture.run(&["report", "--format", "markdown"]);

    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("**Result:**"),
        "markdown report must still carry the result line:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "stderr: {}", stderr(&output));
}

#[test]
fn report_check_fresh_artefact_for_violating_code_exits_nonzero() {
    let fixture = forbidden_edge_fixture();
    let write = fixture.run(&["report", "--output", "report.md"]);
    assert_eq!(write.status.code(), Some(1), "stderr: {}", stderr(&write));

    let check = fixture.run(&["report", "--check", "--output", "report.md"]);
    assert_eq!(check.status.code(), Some(1), "stderr: {}", stderr(&check));
    assert!(
        stdout(&check).is_empty(),
        "--check must not print the report body"
    );
    assert!(stderr(&check).is_empty(), "stderr: {}", stderr(&check));
}

/// A Go module with two production package edges (`app -> core`, `core -> util`)
/// and one external usage, each package declared as its own component.
fn go_edge_metrics_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "go.mod",
        "module example.com/demo\ngo 1.21\n\nrequire (\n\texample.com/third v0.0.0\n)\n",
    );
    fixture.write(
        "app/app.go",
        "package app\n\nimport (\n\t\"example.com/demo/core\"\n\t\"example.com/third\"\n)\n\nfunc App() {\n\tcore.Core()\n\tthird.Third()\n}\n",
    );
    fixture.write(
        "core/core.go",
        "package core\n\nimport \"example.com/demo/util\"\n\nfunc Core() {\n\tutil.Util()\n}\n",
    );
    fixture.write("util/util.go", "package util\n\nfunc Util() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"example.com/demo/app\"] }\n\n[module.allowed]\ndepend_on = [\"core\"]\n\n[[module]]\nname = \"core\"\nmatches = { units = [\"example.com/demo/core\"] }\n\n[module.allowed]\ndepend_on = [\"util\"]\n\n[[module]]\nname = \"util\"\nmatches = { units = [\"example.com/demo/util\"] }\n",
    );
    fixture
}

#[test]
fn report_go_edge_metrics_match_scan_model() {
    let fixture = go_edge_metrics_fixture();
    let scan = fixture.run(&["scan"]);
    assert_eq!(scan.status.code(), Some(0), "stderr: {}", stderr(&scan));
    let model: serde_json::Value = serde_json::from_str(&stdout(&scan)).expect("scan JSON");
    let unit_edges = model["edges"]
        .as_array()
        .expect("model edges array")
        .len();
    let module_edges = model["module_edges"]
        .as_array()
        .expect("model module_edges array")
        .len();
    let external = model["external"]
        .as_array()
        .expect("model external array")
        .len();
    let scan_internal = unit_edges + module_edges;
    let total_scanned_edges = scan_internal + external;

    let output = fixture.run(&["report"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "clean report must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert_eq!(
        metric_value(&out, "edges (internal)"),
        scan_internal.to_string(),
        "Go internal edges must match the scan model's unit-edge tier:\n{out}"
    );
    assert_eq!(
        metric_value(&out, "edges (external)"),
        external.to_string(),
        "Go external edges must match the scan model's external tier:\n{out}"
    );
    let internal: usize = metric_value(&out, "edges (internal)")
        .parse()
        .expect("internal count");
    let external_count: usize = metric_value(&out, "edges (external)")
        .parse()
        .expect("external count");
    assert_eq!(
        internal + external_count,
        total_scanned_edges,
        "the two metrics must together cover every scanned edge:\n{out}"
    );
}

// --- `--format json` -------------------------------------------------------

/// The diff-section lines of a text report as (category, message) pairs.
fn text_diff_findings(text: &str) -> Vec<(String, String)> {
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

/// The findings array of a parsed json report as (category, message) pairs,
/// asserting every entry carries a valid severity.
fn json_findings(value: &serde_json::Value) -> Vec<(String, String)> {
    value["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .map(|finding| {
            let severity = finding["severity"].as_str().expect("finding severity");
            assert!(
                matches!(severity, "error" | "warning"),
                "unknown finding severity: {severity}"
            );
            (
                finding["category"].as_str().expect("finding category").to_string(),
                finding["message"].as_str().expect("finding message").to_string(),
            )
        })
        .collect()
}

#[test]
fn report_json_violations_parse_match_text_and_exit_one() {
    let fixture = several_violations_fixture();
    let text = fixture.run(&["report"]);
    assert_eq!(text.status.code(), Some(1), "text run must exit 1 on violations");

    let output = fixture.run(&["report", "--format", "json"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "json run must exit 1 on violations (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "no stderr on violation run");
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("json report parses");
    assert_eq!(value["language"].as_str(), Some("rust"));
    let findings = json_findings(&value);
    assert!(!findings.is_empty(), "violating tree yields findings");
    assert_eq!(
        findings,
        text_diff_findings(&stdout(&text)),
        "json findings must equal the text-rendered set"
    );
}

#[test]
fn report_json_clean_fixture_exits_zero_with_empty_findings() {
    let fixture = matching_fixture();
    let output = fixture.run(&["report", "--format", "json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "clean run exits 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("json report parses");
    assert!(
        json_findings(&value).is_empty(),
        "clean run reports empty findings"
    );
    // Metrics mirror the scan model tiers.
    let (internal, external) = scan_model_edge_metrics(&fixture);
    let metrics = &value["metrics"];
    assert_eq!(metrics["units"].as_u64(), Some(2));
    assert_eq!(metrics["components"].as_u64(), Some(2));
    assert_eq!(
        metrics["edges_internal"].as_u64(),
        Some(internal as u64),
        "internal edges mirror the scan tiers"
    );
    assert_eq!(
        metrics["edges_external"].as_u64(),
        Some(external as u64),
        "external edges mirror the scan tiers"
    );
}

#[test]
fn report_json_warning_findings_carry_warning_severity_and_exit_zero() {
    let fixture = dead_reference_report_fixture();
    let output = fixture.run(&["report", "--format", "json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "warnings-only stays informational, exit 0 (stderr: {})",
        stderr(&output)
    );
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("json report parses");
    let findings = value["findings"].as_array().expect("findings array");
    assert!(!findings.is_empty(), "the dead reference yields a finding");
    for finding in findings {
        assert_eq!(
            finding["severity"].as_str(),
            Some("warning"),
            "warning-level entries carry warning severity:\n{finding}"
        );
    }
    assert!(findings
        .iter()
        .any(|f| f["category"].as_str() == Some("dead reference")));
}

#[test]
fn report_json_output_file_is_valid_json_and_stdout_empty() {
    let fixture = forbidden_edge_fixture();
    let output = fixture.run(&["report", "--format", "json", "--output", "report.json"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "violations still exit 1 with --output"
    );
    assert!(stdout(&output).is_empty(), "stdout stays empty with --output");
    let file = fixture.read("report.json");
    let value: serde_json::Value = serde_json::from_str(&file).expect("file is valid json");
    let findings = json_findings(&value);
    assert_eq!(
        findings,
        vec![(
            "forbidden edge present".to_string(),
            "billing -> portal".to_string()
        )]
    );
}

// --- destination precedence: explicit --format vs configured destination ---

/// Violating tree plus a configured report destination. The `[output] report`
/// path holds the default text artefact — what a plain `report` writes and
/// what `report --check` compares against.
fn config_destination_fixture() -> common::Fixture {
    let fixture = forbidden_edge_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    fixture
}

/// The foot-gun: an explicit `--format` that differs from the destination's
/// text format must go to stdout and leave the configured artefact untouched,
/// not serialize JSON into the `.md` path with an empty stdout.
#[test]
fn report_json_format_with_config_destination_goes_to_stdout_and_spares_the_file() {
    let fixture = config_destination_fixture();
    let seed = fixture.run(&["report"]);
    assert_eq!(seed.status.code(), Some(1), "stderr: {}", stderr(&seed));
    let seeded = fixture.read("docs/archspec/report.md");
    assert!(seeded.contains("Architecture report"), "seeded text artefact:\n{seeded}");

    let output = fixture.run(&["report", "--format", "json"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "violations still exit 1 (stderr: {})",
        stderr(&output)
    );
    let value: serde_json::Value = serde_json::from_str(&stdout(&output))
        .expect("explicit --format json must land on stdout, not in the file");
    assert_eq!(
        json_findings(&value),
        vec![(
            "forbidden edge present".to_string(),
            "billing -> portal".to_string()
        )]
    );
    assert_eq!(
        fixture.read("docs/archspec/report.md"),
        seeded,
        "the configured text artefact must stay byte-identical"
    );
}

#[test]
fn report_markdown_format_with_config_destination_goes_to_stdout_and_creates_no_file() {
    let fixture = config_destination_fixture();
    let output = fixture.run(&["report", "--format", "markdown"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "violations still exit 1 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("# Architecture report"), "markdown on stdout:\n{out}");
    assert!(
        !fixture.path("docs/archspec/report.md").exists(),
        "a different explicit format must not be written into the configured destination"
    );
}

#[test]
fn report_explicit_text_format_with_config_destination_still_writes_the_file() {
    let fixture = config_destination_fixture();
    let output = fixture.run(&["report", "--format", "text"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).is_empty(),
        "same-format run keeps stdout empty"
    );
    let file = fixture.read("docs/archspec/report.md");
    assert!(file.contains("Architecture report"), "file:\n{file}");
}

#[test]
fn report_no_format_with_config_destination_writes_the_file_as_today() {
    let fixture = config_destination_fixture();
    let output = fixture.run(&["report"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).is_empty(),
        "no --format: stdout empty, file written"
    );
    let file = fixture.read("docs/archspec/report.md");
    assert!(file.contains("Architecture report"), "file:\n{file}");
}

#[test]
fn report_output_flag_wins_over_config_destination_even_with_json() {
    let fixture = config_destination_fixture();
    let output = fixture.run(&["report", "--format", "json", "--output", "report.json"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).is_empty(),
        "stdout stays empty when --output is given"
    );
    let file = fixture.read("report.json");
    let value: serde_json::Value = serde_json::from_str(&file).expect("--output file is json");
    assert!(!json_findings(&value).is_empty(), "violations yield findings");
    assert!(
        !fixture.path("docs/archspec/report.md").exists(),
        "the config destination must stay untouched when --output wins"
    );
}

/// The regression behind the foot-gun chain: a stray `--format json` run must
/// not poison the committed text artefact, so `report --check` still finds it
/// fresh afterwards.
#[test]
fn report_check_stays_fresh_after_json_stdout_run_with_config_destination() {
    let fixture = matching_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    let seed = fixture.run(&["report"]);
    assert_eq!(seed.status.code(), Some(0), "stderr: {}", stderr(&seed));

    let json = fixture.run(&["report", "--format", "json"]);
    assert_eq!(json.status.code(), Some(0), "stderr: {}", stderr(&json));
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&json)).expect("json on stdout");
    assert!(json_findings(&value).is_empty(), "clean run: empty findings");

    let check = fixture.run(&["report", "--check"]);
    assert_eq!(
        check.status.code(),
        Some(0),
        "--check must still find the text artefact fresh (stderr: {})",
        stderr(&check)
    );
}
