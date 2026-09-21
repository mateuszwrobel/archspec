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
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n");
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

/// A single crate whose modules import each other, for depgraph/inspect views.
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

fn write_then_corrupt(fixture: &Fixture, cmd: &[&str], dest: &str) {
    let mut args: Vec<&str> = cmd.to_vec();
    args.extend(["--output", dest]);
    let make = fixture.run(&args);
    assert_eq!(
        make.status.code(),
        Some(0),
        "setup run must produce the artefact: {}",
        stderr(&make)
    );
    fixture.write(dest, "STALE ON PURPOSE\n");
}

// --------------------------- diagram --check ---------------------------

#[test]
fn diagram_check_passes_when_artefact_matches() {
    let fixture = spec_fixture(BASIC_SPEC);
    let make = fixture.run(&["diagram", "--output", "out.mmd"]);
    assert_eq!(make.status.code(), Some(0), "{}", stderr(&make));
    let before = fixture.read("out.mmd");

    let check = fixture.run(&["diagram", "--check", "--output", "out.mmd"]);
    assert_eq!(check.status.code(), Some(0), "{}", stderr(&check));
    assert!(stdout(&check).is_empty(), "stdout empty on fresh check");
    assert_eq!(fixture.read("out.mmd"), before, "check must not touch a fresh file");
}

#[test]
fn diagram_check_fails_when_artefact_differs_and_writes_nothing() {
    let fixture = spec_fixture(BASIC_SPEC);
    write_then_corrupt(&fixture, &["diagram"], "out.mmd");

    let check = fixture.run(&["diagram", "--check", "--output", "out.mmd"]);
    assert_ne!(check.status.code(), Some(0), "stale must fail");
    let err = stderr(&check);
    assert!(err.contains("differs from generated output"), "{err}");
    assert!(err.contains("out.mmd"), "{err}");
    assert!(err.contains("archspec diagram"), "{err}");
    assert_eq!(fixture.read("out.mmd"), "STALE ON PURPOSE\n", "check must not overwrite");
}

#[test]
fn diagram_check_fails_when_artefact_missing_and_creates_nothing() {
    let fixture = spec_fixture(BASIC_SPEC);
    let check = fixture.run(&["diagram", "--check", "--output", "missing.mmd"]);
    assert_ne!(check.status.code(), Some(0), "missing must fail");
    assert!(stderr(&check).contains("is missing"), "{}", stderr(&check));
    assert!(!fixture.path("missing.mmd").exists(), "check must not create the file");
}

#[test]
fn diagram_check_requires_a_destination() {
    let fixture = spec_fixture(BASIC_SPEC);
    let check = fixture.run(&["diagram", "--check"]);
    assert_ne!(check.status.code(), Some(0), "no destination must fail");
    assert!(
        stderr(&check).contains("requires an output destination"),
        "{}",
        stderr(&check)
    );
}

// --------------------------- scan --check ---------------------------

#[test]
fn scan_check_passes_when_artefact_matches() {
    let fixture = rust_pair_fixture();
    let make = fixture.run(&["scan", "--output", "model.json"]);
    assert_eq!(make.status.code(), Some(0), "{}", stderr(&make));

    let check = fixture.run(&["scan", "--check", "--output", "model.json"]);
    assert_eq!(check.status.code(), Some(0), "{}", stderr(&check));
    assert!(stdout(&check).is_empty(), "stdout empty on fresh check");
}

#[test]
fn scan_check_fails_when_artefact_differs() {
    let fixture = rust_pair_fixture();
    write_then_corrupt(&fixture, &["scan"], "model.json");
    let check = fixture.run(&["scan", "--check", "--output", "model.json"]);
    assert_ne!(check.status.code(), Some(0));
    assert!(
        stderr(&check).contains("differs from generated output"),
        "{}",
        stderr(&check)
    );
}

// --------------------------- report --check ---------------------------

#[test]
fn report_check_passes_when_artefact_matches() {
    let fixture = rust_pair_fixture();
    let make = fixture.run(&["report", "--output", "report.md"]);
    assert_eq!(make.status.code(), Some(0), "{}", stderr(&make));

    let check = fixture.run(&["report", "--check", "--output", "report.md"]);
    assert_eq!(check.status.code(), Some(0), "{}", stderr(&check));
    assert!(stdout(&check).is_empty(), "stdout empty on fresh check");
}

#[test]
fn report_check_fails_when_artefact_differs() {
    let fixture = rust_pair_fixture();
    write_then_corrupt(&fixture, &["report"], "report.md");
    let check = fixture.run(&["report", "--check", "--output", "report.md"]);
    assert_ne!(check.status.code(), Some(0));
    assert!(stderr(&check).contains("differs from generated output"), "{}", stderr(&check));
    assert_eq!(fixture.read("report.md"), "STALE ON PURPOSE\n", "check must not overwrite");
}

#[test]
fn report_check_requires_a_destination() {
    let fixture = rust_pair_fixture();
    let check = fixture.run(&["report", "--check"]);
    assert_ne!(check.status.code(), Some(0));
    assert!(
        stderr(&check).contains("requires an output destination"),
        "{}",
        stderr(&check)
    );
}

// --------------------------- inspect --check ---------------------------

#[test]
fn inspect_check_passes_when_artefact_matches() {
    let fixture = edge_fixture();
    let make = fixture.run(&["inspect", "--output", "inspect.mmd"]);
    assert_eq!(make.status.code(), Some(0), "{}", stderr(&make));

    let check = fixture.run(&["inspect", "--check", "--output", "inspect.mmd"]);
    assert_eq!(check.status.code(), Some(0), "{}", stderr(&check));
}

#[test]
fn inspect_check_fails_when_artefact_differs() {
    let fixture = edge_fixture();
    write_then_corrupt(&fixture, &["inspect"], "inspect.mmd");
    let check = fixture.run(&["inspect", "--check", "--output", "inspect.mmd"]);
    assert_ne!(check.status.code(), Some(0));
    assert!(stderr(&check).contains("differs from generated output"), "{}", stderr(&check));
}

#[test]
fn inspect_check_requires_a_destination() {
    let fixture = edge_fixture();
    let check = fixture.run(&["inspect", "--check"]);
    assert_ne!(check.status.code(), Some(0));
    assert!(
        stderr(&check).contains("requires an output destination"),
        "{}",
        stderr(&check)
    );
}

// --------------------------- depgraph --check ---------------------------

#[test]
fn depgraph_check_passes_when_artefact_matches() {
    let fixture = edge_fixture();
    let make = fixture.run(&["depgraph", "modules", "--output", "graph.mmd"]);
    assert_eq!(make.status.code(), Some(0), "{}", stderr(&make));

    let check = fixture.run(&["depgraph", "modules", "--check", "--output", "graph.mmd"]);
    assert_eq!(check.status.code(), Some(0), "{}", stderr(&check));
    assert!(stdout(&check).is_empty(), "stdout empty on fresh check");
}

#[test]
fn depgraph_check_fails_when_artefact_differs() {
    let fixture = edge_fixture();
    write_then_corrupt(&fixture, &["depgraph", "modules"], "graph.mmd");
    let check = fixture.run(&["depgraph", "modules", "--check", "--output", "graph.mmd"]);
    assert_ne!(check.status.code(), Some(0));
    assert!(stderr(&check).contains("differs from generated output"), "{}", stderr(&check));
}

#[test]
fn depgraph_check_requires_a_destination() {
    let fixture = edge_fixture();
    let check = fixture.run(&["depgraph", "modules", "--check"]);
    assert_ne!(check.status.code(), Some(0));
    assert!(
        stderr(&check).contains("requires an output destination"),
        "{}",
        stderr(&check)
    );
}

// --------------------------- help text ---------------------------

#[test]
fn every_output_command_documents_check_flag() {
    let fixture = Fixture::new();
    for cmd in ["scan", "diagram", "report", "inspect", "depgraph"] {
        let help = fixture.run(&[cmd, "--help"]);
        assert_eq!(help.status.code(), Some(0), "{} help must exit 0", cmd);
        let out = stdout(&help);
        assert!(out.contains("--check"), "{cmd} --help must document --check:\n{out}");
    }
}
