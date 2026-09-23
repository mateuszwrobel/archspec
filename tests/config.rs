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

// acceptance row 1 (US 04, D01): a destination write of a human body says so.
/// A single-crate rust tree whose sources carry a module tier (`app::auth` is
/// imported by `app::billing`), so the module-tier views run on it; the spec
/// matches the sources, so `report` and `diagram` do too.
fn module_tree_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
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
    assert_eq!(
        stdout(&output),
        "wrote docs/archspec/report.md\n",
        "a destination write of the human report echoes exactly one wrote line"
    );
    let file = fixture.read("docs/archspec/report.md");
    assert!(file.contains("Architecture report"), "file header:\n{file}");
}

// acceptance row 2 (US 04, D01): same for the mermaid (human) import map.
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
    assert_eq!(
        stdout(&output),
        "wrote docs/archspec/inspect.mmd\n",
        "a destination write of a human diagram echoes exactly one wrote line"
    );
    let file = fixture.read("docs/archspec/inspect.mmd");
    assert!(file.contains("graph"), "file must be mermaid:\n{file}");
}

// acceptance row 3, in its blind4 D01 form: a machine body written to its
// destination echoes the universal `wrote <path>` line — the echo lives on
// stdout, the body's bytes never move.
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
    assert_eq!(
        stdout(&output),
        "wrote docs/archspec/scan.json\n",
        "a machine destination write echoes exactly one wrote line (blind4 D01)"
    );
    let file = fixture.read("docs/archspec/scan.json");
    let model: serde_json::Value =
        serde_json::from_str(&file).expect("destination must parse as the model JSON");
    assert_eq!(model["language"], "rust", "model:\n{file}");
}

// acceptance row 4 (US 04, D01): same for the mermaid (human) model diagram.
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
    assert_eq!(
        stdout(&output),
        "wrote docs/archspec/diagram.mmd\n",
        "a destination write of a human diagram echoes exactly one wrote line"
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
    assert_eq!(
        stdout(&output),
        "wrote custom.md\n",
        "an explicit --output write echoes the universal wrote line (blind4 D01)"
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
    assert!(stdout(&output).is_empty(), "no report on operational error");
    assert!(
        !fixture.path("docs/archspec/report.md").exists(),
        "nothing may be written on malformed config"
    );
}

// ---------------------------------------------------------------------------
// US 04 (workplan_archspec_fanout_and_output): stdout states where the body
// went. A destination write of a human body echoes `wrote <path>` (D01); a
// `--format` run that bypasses a configured destination opens with
// `note: destination <path> not written` (D02); machine bodies — json, plantuml
// — carry no status line on any path (D08); with no destination configured the
// bytes are today's bytes. Exit codes are untouched throughout.
// ---------------------------------------------------------------------------

/// acceptance row 9: the wrote line is the only thing stdout moves — the bytes
/// at the destination equal those a named `--output` write produces for the
/// same tree.
#[test]
fn config_wrote_line_accompanies_unchanged_body_bytes() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    let destination = fixture.run(&["report"]);
    let flag = fixture.run(&["report", "--output", "copy.md"]);

    assert_eq!(
        stdout(&destination),
        "wrote docs/archspec/report.md\n",
        "the wrote line is stdout's only content (stderr: {})",
        stderr(&destination)
    );
    assert_eq!(
        stdout(&flag),
        "wrote copy.md\n",
        "--output writes are echoed the same universal way (blind4 D01)"
    );
    assert_eq!(
        fixture.read("docs/archspec/report.md"),
        fixture.read("copy.md"),
        "only stdout moves: the destination body equals the flag-write body"
    );
}

/// acceptance row 10 (D02): an explicit `--format` whose human body goes to
/// stdout says the configured destination was not written, and the destination
/// keeps the bytes it had.
#[test]
fn config_format_bypass_states_destination_not_written() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    fixture.write("docs/archspec/report.md", "COMMITTED REPORT\n");

    let output = fixture.run(&["report", "--format", "markdown"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "the bypass must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    let mut lines = out.lines();
    assert_eq!(
        lines.next(),
        Some("note: destination docs/archspec/report.md not written"),
        "the statement is the first line:\n{out}"
    );
    assert!(
        lines
            .next()
            .is_some_and(|line| line.contains("Architecture report")),
        "the body follows the statement:\n{out}"
    );
    assert_eq!(
        out.matches("destination ").count(),
        1,
        "exactly one statement line:\n{out}"
    );
    assert_eq!(
        fixture.read("docs/archspec/report.md"),
        "COMMITTED REPORT\n",
        "the bypass never touches the destination"
    );
}

/// The echo and the statement are mutually exclusive: naming the format the
/// destination is provisioned for writes it (and echoes `wrote`), so a run never
/// prints both lines.
#[test]
fn config_explicit_provisioned_format_writes_and_echoes_only() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    fixture.write("docs/archspec/report.md", "COMMITTED REPORT\n");

    let output = fixture.run(&["report", "--format", "text"]);

    assert_eq!(output.status.code(), Some(0), "exit 0 expected");
    let out = stdout(&output);
    assert_eq!(out, "wrote docs/archspec/report.md\n", "stdout:\n{out}");
    assert!(
        fixture
            .read("docs/archspec/report.md")
            .contains("Architecture report"),
        "the provisioned format must land in the destination"
    );
}

/// acceptance row 11 (D02/D08): a machine body on stdout is pure — parseable
/// json with no status line anywhere inside — while the configured destination
/// keeps its bytes.
#[test]
fn config_machine_body_to_stdout_is_byte_clean_and_destination_untouched() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    fixture.write("docs/archspec/report.md", "COMMITTED REPORT\n");

    let output = fixture.run(&["report", "--format", "json"]);

    assert_eq!(output.status.code(), Some(0), "exit 0 expected");
    let out = stdout(&output);
    let value: serde_json::Value = serde_json::from_str(&out).expect("stdout is pure json");
    assert!(value["metrics"].is_object(), "metrics missing:\n{out}");
    assert_eq!(
        fixture.read("docs/archspec/report.md"),
        "COMMITTED REPORT\n",
        "a machine format must not write the text destination"
    );
}

/// D01's universal form: a machine body written to its destination echoes the
/// one `wrote <path>` line on stdout — the plantuml bytes themselves stay
/// byte-clean inside the file.
#[test]
fn config_machine_body_to_destination_echoes_wrote_line() {
    let fixture = workspace_spec_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\ndiagram = \"docs/archspec/diagram.mmd\"\n",
    );
    let output = fixture.run(&["diagram", "--format", "plantuml"]);

    assert_eq!(output.status.code(), Some(0), "exit 0 expected");
    assert_eq!(
        stdout(&output),
        "wrote docs/archspec/diagram.mmd\n",
        "a machine destination write echoes exactly one wrote line (blind4 D01)"
    );
    assert!(
        fixture
            .read("docs/archspec/diagram.mmd")
            .starts_with("@startuml"),
        "the plantuml body must land in the destination"
    );
}

/// acceptance row 12: a plantuml body sent to stdout carries no note anywhere,
/// whether or not a destination is configured.
#[test]
fn config_machine_body_to_stdout_carries_no_status_line() {
    let fixture = module_tree_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\ndiagram = \"docs/archspec/diagram.mmd\"\n",
    );

    for args in [
        vec!["depgraph", "modules", "--format", "plantuml"],
        vec!["inspect", "--format", "plantuml"],
    ] {
        let output = fixture.run(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?} must exit 0 (stderr: {})",
            stderr(&output)
        );
        let out = stdout(&output);
        assert!(
            out.starts_with("@startuml"),
            "{args:?} stdout must open with the body:\n{out}"
        );
        assert!(
            !out.contains("note:") && !out.contains("wrote "),
            "{args:?} machine body carries no status line:\n{out}"
        );
    }
}

/// acceptance row 7 in its US 04 form: nothing configured, nothing echoed —
/// every render command keeps today's bytes on every path (body on stdout, no
/// wrote line, no destination statement) when no `[output]` destination or
/// `--output` flag routes it.
#[test]
fn no_destination_configured_keeps_stdout_byte_clean() {
    for config in [None, Some("[output]\nscan = \"docs/archspec/scan.json\"\n")] {
        let fixture = module_tree_fixture();
        if let Some(config) = config {
            // `scan` is not run below and no other key is configured, so none of
            // these runs has a destination to announce.
            fixture.write("archspec.toml", config);
        }
        for args in [
            vec!["report"],
            vec!["report", "--format", "markdown"],
            vec!["diagram"],
            vec!["inspect"],
            vec!["depgraph", "modules"],
        ] {
            let output = fixture.run(&args);
            assert_eq!(
                output.status.code(),
                Some(0),
                "{args:?} must exit 0 (stderr: {})",
                stderr(&output)
            );
            let out = stdout(&output);
            assert!(!out.is_empty(), "{args:?} keeps its body on stdout:\n{out}");
            assert!(
                !out.contains("wrote ") && !out.contains("not written"),
                "{args:?} must echo nothing without a destination:\n{out}"
            );
            assert!(
                !fixture.path("docs/archspec").exists(),
                "{args:?} must write nothing without a destination"
            );
        }
    }
}

/// `depgraph` has no `[output]` key: its only destination is `--output`, so an
/// `[output]` table never makes it echo a wrote line.
#[test]
fn depgraph_has_no_configured_destination_to_echo() {
    let fixture = module_tree_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\n",
    );
    let output = fixture.run(&["depgraph", "modules"]);

    assert_eq!(output.status.code(), Some(0), "exit 0 expected");
    let out = stdout(&output);
    assert!(out.contains("graph"), "graph on stdout:\n{out}");
    assert!(
        !out.contains("wrote "),
        "depgraph has no configured destination to echo:\n{out}"
    );
}
