//! `explain` verb surface tests (GitHub issue #26): the exit-code contract
//! (usage 64, operational 70 — findings never move the code) and the parity
//! claim against the verify path in its rust-native shape. The capability
//! matrix row `[verbs.explain]` names the missing-spec test below.

mod common;

use common::{stderr, stdout, Fixture};

/// A two-package workspace whose packages import each other, so the model
/// carries a unit-level pair in both directions. The spec declares both
/// packages as modules and one `no_cycles` row over them at error severity.
fn cycle_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n");
    fixture.write(
        "a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nb = { path = \"../b\" }\n",
    );
    fixture.write("a/src/lib.rs", "pub fn a() {}\n");
    fixture.write(
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\na = { path = \"../a\" }\n",
    );
    fixture.write("b/src/lib.rs", "pub fn b() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n\
         [[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\n\n\
         [[constraint]]\ntype = \"no_cycles\"\nmodules = [\"a\", \"b\"]\nseverity = \"error\"\n",
    );
    fixture
}

/// A single-package crate with a clean spec: `app` declares the one unit.
fn clean_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn a() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
    fixture
}

/// Probe `verb:explain:missing-spec`: explain loads the spec before it
/// traces, call for call as verify does — a missing spec is an operational
/// precondition (70), never a verdict (1) and never a usage mistake (64).
#[test]
fn explain_missing_spec_answers_operational_class() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn a() {}\n");
    let output = fixture.run(&["explain", "a", "b"]);
    assert_eq!(
        output.status.code(),
        Some(70),
        "probe verb:explain:missing-spec"
    );
    assert_eq!(
        stderr(&output),
        "error: no architecture.spec.toml found in: .\n",
        "the missing-spec bytes must match the verify path exactly — explain reuses it"
    );
    assert!(stdout(&output).is_empty(), "stdout carries nothing");
}

/// The usage class: fewer than two positional endpoints is a usage mistake (64).
#[test]
fn explain_without_pair_arguments_answers_usage() {
    let fixture = clean_fixture();
    let output = fixture.run(&["explain"]);
    assert_eq!(output.status.code(), Some(64), "probe usage:explain:noargs");
    assert!(
        stderr(&output).contains("usage: archspec explain <source> <target>"),
        "the usage line must name the pair arguments:\n{}",
        stderr(&output)
    );
}

/// The usage class: an unknown flag is refused before any load runs (64).
#[test]
fn explain_unknown_flag_answers_usage() {
    let fixture = clean_fixture();
    let output = fixture.run(&["explain", "a", "b", "--verdict"]);
    assert_eq!(
        output.status.code(),
        Some(64),
        "probe usage:explain:unknown-flag"
    );
}

/// Bound B1 parity in the rust-native shape: verify's finding for the pair is
/// the same body explain renders, and the exit codes split by role — verify
/// adjudicates (1), explain renders (0).
#[test]
fn explain_renders_the_cycle_verify_adjudicates_without_moving_the_exit_code() {
    let fixture = cycle_fixture();
    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "verify must adjudicate the cycle red:\n{}",
        stdout(&verify)
    );
    let verify_out = stdout(&verify);
    let cycle_body = verify_out
        .lines()
        .find_map(|line| {
            line.find("cycle: ")
                .map(|at| line[at + "cycle: ".len()..].trim().to_string())
        })
        .unwrap_or_else(|| panic!("verify must list the cycle:\n{verify_out}"));

    let output = fixture.run(&["explain", "a", "b"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a finding never moves explain's exit code:\n{}",
        stdout(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains(&format!("cycle detected in: {cycle_body}")),
        "the findings section must render the body verify printed:\n{out}"
    );
    assert!(
        out.contains("module-graph row a -> b: present"),
        "the trace must state the module-graph row:\n{out}"
    );
    assert!(
        out.contains("pair adjudicated: yes"),
        "both endpoints are inside the declared territory:\n{out}"
    );
    assert!(
        out.contains("note: explain renders the same load, extract and compare calls `archspec verify` runs"),
        "the renderer note must close the trace:\n{out}"
    );
}

/// An endpoint spelled neither as a declared module nor as a model file is
/// stated as unresolved — explain guesses nothing and still exits 0.
#[test]
fn explain_states_an_unresolved_endpoint() {
    let fixture = clean_fixture();
    let output = fixture.run(&["explain", "app", "ghost"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("target: unresolved: not a declared module name"),
        "the unresolved spelling must be stated:\n{out}"
    );
    assert!(
        out.contains("module-graph row app -> ghost: absent"),
        "an unresolved endpoint cannot hold a module-graph row:\n{out}"
    );
}

/// The constraint census filter: a glob that matches no row says so in the
/// constraints section — the census stays explicit, never silently empty.
#[test]
fn explain_constraint_filter_states_an_empty_census() {
    let fixture = cycle_fixture();
    let output = fixture.run(&["explain", "a", "b", "--constraint", "feature_*"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("none — the spec declares no constraint row under this filter"),
        "an empty filtered census must be stated:\n{out}"
    );
    assert!(
        !out.contains("[0] no_cycles"),
        "the filter must hide the unmatched row:\n{out}"
    );
}
