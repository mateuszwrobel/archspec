//! Exit-taxonomy retrofit (spec-toolkit family conformance, P2). Each changed
//! exit path is named by exactly one test here, and each test names the
//! conformance probe that measures it (specs/conformance/run-conformance.sh
//! row). The doctrine contract these paths implement:
//!
//! - usage class, exit 64 (universal): no command, unknown command, unknown
//!   flag, `help` typo — an argument mistake, never a verdict;
//! - an undeclared verb is refused as an unknown command — exit 64;
//! - operational-precondition class, exit 70: the spec file a verb needs is
//!   missing — never answered as a verdict;
//! - verdict codes unchanged: violations stay exit 1, a clean run stays 0;
//! - error messages on every changed path stay byte-identical — only the
//!   exit code moved.

mod common;

use common::{stderr, stdout, Fixture};

/// A Rust tree with a spec that forbids the real edge: `verify` completes the
/// verdict and reports a violation (the pinned class this retrofit must not
/// renumber).
fn violation_fixture() -> Fixture {
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
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = []\n",
    );
    fixture
}

/// Probe `usage:noargs`: bare `archspec` prints the command list on stdout and
/// exits 64 — a usage mistake, not a request.
#[test]
fn noargs_is_a_usage_error_exits_64_and_prints_the_command_list() {
    let fixture = Fixture::new();
    let output = fixture.run(&[]);
    assert_eq!(output.status.code(), Some(64), "probe usage:noargs");
    assert!(
        stdout(&output).contains("usage: archspec <command> [args]"),
        "the command list still prints:\n{}",
        stdout(&output)
    );
    assert!(stderr(&output).is_empty(), "help output never goes to stderr");
}

/// Probe `usage:unknown-command`: the refusal bytes are unchanged and the
/// class is usage.
#[test]
fn unknown_command_exits_64_with_the_unmodified_refusal_bytes() {
    let fixture = Fixture::new();
    let output = fixture.run(&["zzz-not-a-command"]);
    assert_eq!(output.status.code(), Some(64), "probe usage:unknown-command");
    assert_eq!(
        stderr(&output),
        "error: unknown command: zzz-not-a-command\nrun 'archspec --help' for the command list\n",
        "the refusal bytes must not move — only the code did"
    );
}

/// Probe `usage:unknown-flag`: every command parses flags through the same
/// gate, so the usage class fires identically on the probed verb; the message
/// bytes stay exactly as they were.
#[test]
fn unknown_flag_exits_64_naming_the_flag_with_unchanged_bytes() {
    let fixture = Fixture::new();
    let output = fixture.run(&["verify", "--zzz-not-a-flag"]);
    assert_eq!(output.status.code(), Some(64), "probe usage:unknown-flag");
    assert_eq!(
        stderr(&output),
        "error: unknown flag: --zzz-not-a-flag\n",
        "the unknown-flag bytes must not move — only the code did"
    );
}

/// Probe `usage:help-typo`: an unknown topic names itself, lists the valid
/// topics, hints at the index, and exits the usage class with unchanged bytes.
#[test]
fn help_typo_exits_64_naming_the_topic_with_unchanged_bytes() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "zzz-not-a-topic"]);
    assert_eq!(output.status.code(), Some(64), "probe usage:help-typo");
    assert_eq!(
        stderr(&output),
        "error: unknown help topic: zzz-not-a-topic\nvalid topics: commands, glob, spec, constraints, languages, roles, workflow, capabilities, diagnostics\nrun 'archspec help'\n",
        "the help-typo bytes must not move — only the code did"
    );
}

/// Probe `undeclared:status`: a family verb this tool does not declare is
/// refused as an unknown command — honest refusal, exit 64, never silence.
#[test]
fn undeclared_verb_status_exits_64_as_an_unknown_command() {
    let fixture = Fixture::new();
    let output = fixture.run(&["status"]);
    assert_eq!(output.status.code(), Some(64), "probe undeclared:status");
    assert!(
        stderr(&output).contains("error: unknown command: status"),
        "the undeclared verb is refused as an unknown command:\n{}",
        stderr(&output)
    );
}

/// Probe `verb:verify:missing-spec`: a missing spec is an operational
/// precondition (70), never a verdict (1) and never a usage mistake (64).
#[test]
fn verify_without_a_spec_exits_70_with_unchanged_bytes() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("src/lib.rs", "pub fn a() {}\n");
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(70),
        "probe verb:verify:missing-spec"
    );
    assert_eq!(
        stderr(&output),
        "error: no architecture.spec.toml found in: .\n",
        "the missing-spec bytes must not move — only the code did"
    );
    assert!(stdout(&output).is_empty(), "stdout carries nothing");
}

/// Probe `verb:generate:missing-spec`: the declared-model render needs the
/// spec file too — the same operational class.
#[test]
fn diagram_without_a_spec_exits_70_with_unchanged_bytes() {
    let fixture = Fixture::new();
    let output = fixture.run(&["diagram"]);
    assert_eq!(
        output.status.code(),
        Some(70),
        "probe verb:generate:missing-spec"
    );
    assert_eq!(
        stderr(&output),
        "error: no spec found under: .\n",
        "the missing-spec bytes must not move — only the code did"
    );
}

/// Probe `verb:check:missing-spec`: freshness renders the declared model
/// before comparing, so a missing spec answers 70 before any drift verdict.
#[test]
fn diagram_check_without_a_spec_exits_70() {
    let fixture = Fixture::new();
    let output = fixture.run(&["diagram", "--check"]);
    assert_eq!(
        output.status.code(),
        Some(70),
        "probe verb:check:missing-spec"
    );
    assert_eq!(
        stderr(&output),
        "error: no spec found under: .\n",
        "the missing-spec bytes must not move — only the code did"
    );
}

/// The report loader words the same precondition differently (its own
/// contract) — and moves only the code there as well.
#[test]
fn report_without_a_spec_exits_70_with_unchanged_bytes() {
    let fixture = Fixture::new();
    let output = fixture.run(&["report"]);
    assert_eq!(output.status.code(), Some(70), "missing spec via report");
    assert_eq!(
        stderr(&output),
        "error: spec file not found: ./architecture.spec.toml\n",
        "the report missing-spec bytes must not move — only the code did"
    );
}

/// The pinned counter-piece of the retrofit: a rule violation is still a
/// verdict, exits 1, and echoes nothing to stderr — the operational class
/// moved out, the verdict class stayed exactly where it was.
#[test]
fn verify_violation_verdict_still_exits_1_and_is_not_renumbered() {
    let fixture = violation_fixture();
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a violation stays a verdict: exit 1"
    );
    assert!(
        stdout(&output).contains("disallowed cross-component dependency"),
        "the diff report prints on stdout:\n{}",
        stdout(&output)
    );
    assert!(
        stderr(&output).is_empty(),
        "a rule violation is never echoed to stderr"
    );
}
