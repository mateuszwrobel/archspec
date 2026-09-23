//! `--check` green must speak (blind4 follow-up US_02).
//!
//! A green `--check` run currently prints nothing and exits 0, exactly like a
//! silent success elsewhere; an operator cannot tell "checked and fresh" from
//! a run that checked nothing. The contract: every command whose `--check`
//! compares a destination prints exactly `ok: <path> up to date` on stdout and
//! still exits 0, naming the compared path whether it came from `--output` or
//! from the `[output]` config table. Stale and missing checks keep their
//! current findings, exit codes and silence-before-verdict behavior: the
//! ok-line is additive to green only. Rule: D02 of
//! `rust-arch-test-kit/worklog/workplan_archspec_blind4_followup/sources.md`.

mod common;

use common::{stderr, stdout, Fixture};

fn spec_fixture(spec: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write("architecture.spec.toml", spec);
    fixture
}

const BASIC_SPEC: &str = r#"[project]
language = "rust"

[[module]]
name = "Billing"
matches = { units = ["Billing*"] }

[module.allowed]
depend_on = ["Shared"]

[[module]]
name = "Shared"
matches = { units = ["Shared*"] }
"#;

/// Two crates where billing depends on auth; the spec matches exactly.
fn rust_pair_fixture() -> Fixture {
    let fixture = Fixture::new();
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

/// A single crate whose modules import each other, for inspect views.
fn edge_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");
    fixture
}

/// One crate whose `billing` module imports `auth`: a model with a module
/// tree and one module-tier edge.
fn model_tree_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write(
        "src/auth/mod.rs",
        "pub mod tokens;\npub use tokens::Token;\n",
    );
    fixture.write("src/auth/tokens.rs", "pub struct Token;\n");
    fixture.write("src/billing/mod.rs", "use crate::auth::Token;\npub fn bill(_: Token) {}\n");
    fixture.write("architecture.spec.toml", model_tree_spec());
    fixture
}

fn model_tree_spec() -> &'static str {
    "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n"
}

fn require_ok(fixture: &Fixture, args: &[&str], context: &str) -> std::process::Output {
    let run = fixture.run(args);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{context}: {}",
        stderr(&run)
    );
    run
}

#[test]
fn scan_check_green_echoes_ok_line_naming_the_artefact() {
    let fixture = rust_pair_fixture();
    require_ok(&fixture, &["scan", "--output", "model.json"], "generate");
    let before = fixture.read("model.json");

    let check = require_ok(&fixture, &["scan", "--check", "--output", "model.json"], "check");
    assert_eq!(
        stdout(&check),
        "ok: model.json up to date\n",
        "green scan --check must print exactly the ok-line"
    );
    assert_eq!(
        fixture.read("model.json"),
        before,
        "a green check must not touch the artefact"
    );
}

#[test]
fn diagram_check_green_echoes_ok_line_naming_the_artefact() {
    let fixture = spec_fixture(BASIC_SPEC);
    require_ok(
        &fixture,
        &["diagram", "--output", "out.mmd"],
        "generate",
    );

    let check = require_ok(
        &fixture,
        &["diagram", "--check", "--output", "out.mmd"],
        "check",
    );
    assert_eq!(
        stdout(&check),
        "ok: out.mmd up to date\n",
        "green diagram --check must print exactly the ok-line"
    );
}

#[test]
fn report_check_green_echoes_ok_line_naming_the_artefact() {
    let fixture = rust_pair_fixture();
    require_ok(
        &fixture,
        &["report", "--output", "report.md"],
        "generate",
    );

    let check = require_ok(
        &fixture,
        &["report", "--check", "--output", "report.md"],
        "check",
    );
    assert_eq!(
        stdout(&check),
        "ok: report.md up to date\n",
        "green report --check must print exactly the ok-line"
    );
}

#[test]
fn inspect_check_green_echoes_ok_line_naming_the_artefact() {
    let fixture = edge_fixture();
    require_ok(
        &fixture,
        &["inspect", "--output", "inspect.mmd"],
        "generate",
    );

    let check = require_ok(
        &fixture,
        &["inspect", "--check", "--output", "inspect.mmd"],
        "check",
    );
    assert_eq!(
        stdout(&check),
        "ok: inspect.mmd up to date\n",
        "green inspect --check must print exactly the ok-line"
    );
}

#[test]
fn depgraph_check_green_echoes_ok_line_naming_the_artefact() {
    let fixture = model_tree_fixture();
    require_ok(
        &fixture,
        &["depgraph", "modules", "--format", "mermaid", "--output", "graph.mmd"],
        "generate",
    );

    let check = require_ok(
        &fixture,
        &[
            "depgraph",
            "modules",
            "--check",
            "--format",
            "mermaid",
            "--output",
            "graph.mmd",
        ],
        "check",
    );
    assert_eq!(
        stdout(&check),
        "ok: graph.mmd up to date\n",
        "green depgraph --check must print exactly the ok-line"
    );
}

#[test]
fn check_green_ok_line_names_configured_destination() {
    let fixture = model_tree_fixture();
    fixture.write("archspec.toml", "[output]\nreport = \"check-report.md\"\n");
    // Render through the configured destination (no flag): writes the file.
    require_ok(&fixture, &["report"], "generate via config destination");

    let check = require_ok(&fixture, &["report", "--check"], "check");
    assert_eq!(
        stdout(&check),
        "ok: check-report.md up to date\n",
        "the ok-line must name the configured destination path"
    );
}

#[test]
fn check_stale_prints_no_ok_line() {
    let fixture = rust_pair_fixture();
    require_ok(&fixture, &["scan", "--output", "model.json"], "generate");
    fixture.write("model.json", "STALE ON PURPOSE\n");

    let check = fixture.run(&["scan", "--check", "--output", "model.json"]);
    assert_ne!(check.status.code(), Some(0), "stale must fail");
    assert!(
        stderr(&check).contains("differs from generated output"),
        "the stale finding is unchanged: {}",
        stderr(&check)
    );
    assert!(
        stdout(&check).is_empty(),
        "a red check must print no ok-line, got: {}",
        stdout(&check)
    );
}

#[test]
fn check_missing_prints_no_ok_line() {
    let fixture = spec_fixture(BASIC_SPEC);
    let check = fixture.run(&["diagram", "--check", "--output", "missing.mmd"]);
    assert_ne!(check.status.code(), Some(0), "missing must fail");
    assert!(
        stderr(&check).contains("is missing"),
        "the missing finding is unchanged: {}",
        stderr(&check)
    );
    assert!(
        stdout(&check).is_empty(),
        "a red check must print no ok-line, got: {}",
        stdout(&check)
    );
    assert!(
        !fixture.path("missing.mmd").exists(),
        "a red check must not create the file"
    );
}

#[test]
fn check_ok_line_and_violation_exit_compose_fresh_report_with_violations() {
    // A model with a forbidden edge: report exits 1 on findings, and a fresh
    // artefact of it passes --check. The two verdicts are orthogonal: stdout
    // states the freshness, the exit code states the rules.
    let fixture = model_tree_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"app::auth\"] }\n[[module.depends_on]]\nto = \"app::billing\"\nstereotype = \"forbidden\"\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"app::billing\"] }\n",
    );
    let make = fixture.run(&["report", "--format", "markdown", "--output", "r.md"]);
    assert_eq!(
        make.status.code(),
        Some(1),
        "the violating report must exit 1 at generation: {}",
        stderr(&make)
    );

    let check = fixture.run(&[
        "report",
        "--check",
        "--format",
        "markdown",
        "--output",
        "r.md",
    ]);
    assert_eq!(
        check.status.code(),
        Some(1),
        "violations still drive the exit code through --check"
    );
    assert_eq!(
        stdout(&check),
        "ok: r.md up to date\n",
        "freshness verdict on stdout, violation verdict in the exit code"
    );
}
