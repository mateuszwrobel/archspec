mod common;

use common::{stderr, stdout};

/// A two-crate cargo workspace (`billing` depends on `auth`) whose spec matches
/// the sources exactly. Used by report/diagram config-destination tests.
fn workspace_spec_fixture() -> common::Fixture {
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

/// A single-crate rust tree (`app` package with `src/lib.rs`). Used by
/// inspect/scan config-destination tests, which need no spec.
fn single_crate_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    fixture
}

#[test]
fn config_report_default_writes_configured_path() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "stdout must stay empty when the config default writes a file"
    );
    let file = fixture.read("docs/archspec/report.md");
    assert!(file.contains("Architecture report"), "file header:\n{file}");
}

#[test]
fn config_inspect_default_writes_configured_path() {
    let fixture = single_crate_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\ninspect = \"docs/archspec/inspect.mmd\"\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "inspect must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "stdout must stay empty when the config default writes a file"
    );
    let file = fixture.read("docs/archspec/inspect.mmd");
    assert!(file.contains("graph"), "file must be mermaid:\n{file}");
}

#[test]
fn config_scan_default_writes_configured_path() {
    let fixture = single_crate_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nscan = \"docs/archspec/scan.json\"\n",
    );
    let output = fixture.run(&["scan"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "scan must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "stdout must stay empty when the config default writes a file"
    );
    let file = fixture.read("docs/archspec/scan.json");
    assert!(file.contains("\"language\""), "file must be a JSON model:\n{file}");
}

#[test]
fn config_diagram_default_writes_configured_path() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\ndiagram = \"docs/archspec/diagram.mmd\"\n",
    );
    let output = fixture.run(&["diagram"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "diagram must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "stdout must stay empty when the config default writes a file"
    );
    let file = fixture.read("docs/archspec/diagram.mmd");
    assert!(file.contains("graph"), "file must be mermaid:\n{file}");
}

#[test]
fn config_output_flag_overrides_default() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    let output = fixture.run(&["report", "--output", "custom.md"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "stdout must stay empty with --output"
    );
    let file = fixture.read("custom.md");
    assert!(file.contains("Architecture report"), "file header:\n{file}");
    assert!(
        !fixture.path("docs/archspec/report.md").exists(),
        "configured default must not be written when --output overrides"
    );
}

#[test]
fn config_missing_file_falls_back_to_stdout() {
    let fixture = workspace_spec_fixture();
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("Architecture report"),
        "no config file: report must fall back to stdout:\n{}",
        stdout(&output)
    );
}

#[test]
fn config_missing_key_falls_back_to_stdout() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nscan = \"docs/archspec/scan.json\"\n",
    );
    let output = fixture.run(&["report"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "report must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("Architecture report"),
        "no report key: report must fall back to stdout:\n{}",
        stdout(&output)
    );
    assert!(
        !fixture.path("docs/archspec/report.md").exists(),
        "no report file may be written without a report key"
    );
}

#[test]
fn config_malformed_toml_is_operational_error() {
    let fixture = workspace_spec_fixture();
    fixture.write("archspec.toml", "[output\nreport = \"x\"\n");
    let output = fixture.run(&["report"]);

    assert_ne!(
        output.status.code(),
        Some(0),
        "malformed config must fail the run"
    );
    let err = stderr(&output);
    assert!(
        err.contains("archspec.toml"),
        "stderr must name the config file:\n{err}"
    );
    assert!(
        stdout(&output).is_empty(),
        "no report on operational error"
    );
    assert!(
        !fixture.path("docs/archspec/report.md").exists(),
        "nothing may be written on malformed config"
    );
}