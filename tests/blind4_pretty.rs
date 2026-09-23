//! `--pretty` for machine JSON (blind4 follow-up US_03).
//!
//! `scan` serializes the model as one compact line, unreadable and
//! undiffable in a review; `report --format json` is already indented but
//! rejects `--pretty`, so a script cannot pass one flag set across commands.
//! The contract: `scan --pretty` switches the JSON body to indented form —
//! stdout or destination alike — and `report` accepts `--pretty` on every
//! format without changing a byte. Default bytes of every command stay
//! exactly as they are today. Rule: D03 of
//! `rust-arch-test-kit/worklog/workplan_archspec_blind4_followup/sources.md`.

mod common;

use common::{stderr, stdout, Fixture};

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
    fixture
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
fn scan_default_body_is_one_compact_line_unchanged() {
    let fixture = rust_pair_fixture();
    let run = require_ok(&fixture, &["scan", "--output", "m.json"], "default scan");
    assert_eq!(
        stdout(&run),
        "wrote m.json\n",
        "the default invocation keeps its status line"
    );
    let body = fixture.read("m.json");
    assert!(body.ends_with("}\n"), "model json ends with one newline");
    assert_eq!(
        body.lines().count(),
        1,
        "default bytes stay a single compact line"
    );
}

#[test]
fn scan_pretty_indents_and_decodes_to_the_same_model() {
    let fixture = rust_pair_fixture();
    require_ok(&fixture, &["scan", "--output", "m.json"], "default scan");
    require_ok(
        &fixture,
        &["scan", "--pretty", "--output", "p.json"],
        "pretty scan",
    );
    let compact: serde_json::Value =
        serde_json::from_str(&fixture.read("m.json")).expect("default body parses");
    let pretty = fixture.read("p.json");
    let parsed: serde_json::Value = serde_json::from_str(&pretty).expect("pretty body parses");
    assert_eq!(
        compact, parsed,
        "the pretty flag must not change a single fact of the model"
    );
    assert!(
        pretty.lines().count() > 1 && pretty.contains("\n  "),
        "the pretty body must be indented over several lines:\n{pretty}"
    );
}

#[test]
fn scan_pretty_stdout_equals_the_pretty_file_bytes() {
    let fixture = rust_pair_fixture();
    require_ok(
        &fixture,
        &["scan", "--pretty", "--output", "p.json"],
        "pretty to file",
    );
    let run = require_ok(
        &fixture,
        &["scan", "--pretty", "--output", "-"],
        "pretty to stdout",
    );
    assert_eq!(
        stdout(&run),
        fixture.read("p.json"),
        "stdout and destination carry identical pretty bytes"
    );
}

#[test]
fn scan_pretty_to_destination_echoes_wrote_line() {
    let fixture = rust_pair_fixture();
    let run = require_ok(
        &fixture,
        &["scan", "--pretty", "--output", "p.json"],
        "pretty scan",
    );
    assert_eq!(
        stdout(&run),
        "wrote p.json\n",
        "the universal echo rides the pretty write unchanged"
    );
}

#[test]
fn scan_pretty_is_deterministic() {
    let fixture = rust_pair_fixture();
    require_ok(
        &fixture,
        &["scan", "--pretty", "--output", "p1.json"],
        "first",
    );
    require_ok(
        &fixture,
        &["scan", "--pretty", "--output", "p2.json"],
        "second",
    );
    assert_eq!(
        fixture.read("p1.json"),
        fixture.read("p2.json"),
        "repeated pretty renders are byte-identical"
    );
}

#[test]
fn report_json_accepts_pretty_byte_identically() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn run() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
    require_ok(
        &fixture,
        &["report", "--format", "json", "--output", "a.json"],
        "default json",
    );
    require_ok(
        &fixture,
        &["report", "--format", "json", "--pretty", "--output", "b.json"],
        "accepted pretty flag",
    );
    assert_eq!(
        fixture.read("a.json"),
        fixture.read("b.json"),
        "report json is already indented: --pretty accepts and changes nothing"
    );
}

#[test]
fn report_other_formats_accept_pretty_byte_identically() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn run() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
    let plain = require_ok(
        &fixture,
        &["report", "--format", "markdown", "--output", "r1.md"],
        "markdown default",
    );
    let pretty = require_ok(
        &fixture,
        &["report", "--format", "markdown", "--pretty", "--output", "r2.md"],
        "markdown with accepted flag",
    );
    assert_eq!(
        stdout(&plain),
        "wrote r1.md\n",
        "the plain run keeps its status line"
    );
    assert_eq!(
        stdout(&pretty),
        "wrote r2.md\n",
        "the accepted flag changes no status line"
    );
    assert_eq!(
        fixture.read("r1.md"),
        fixture.read("r2.md"),
        "--pretty must not touch a markdown body"
    );
}

#[test]
fn pretty_is_refused_outside_the_json_commands() {
    let fixture = Fixture::new();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n",
    );
    for cmd in [["diagram", "--pretty"], ["verify", "--pretty"]] {
        let run = fixture.run(&cmd);
        assert_ne!(
            run.status.code(),
            Some(0),
            "`{}` must not accept --pretty",
            cmd.join(" ")
        );
        assert!(
            stderr(&run).contains("unknown flag: --pretty"),
            "refusal names the flag: {}",
            stderr(&run)
        );
    }
}

#[test]
fn scan_help_documents_the_pretty_flag() {
    let fixture = Fixture::new();
    let run = require_ok(&fixture, &["scan", "--help"], "scan help");
    assert!(
        stdout(&run).contains("--pretty"),
        "scan help must teach the flag:\n{}",
        stdout(&run)
    );
}

#[test]
fn report_help_documents_the_pretty_flag() {
    let fixture = Fixture::new();
    let run = require_ok(&fixture, &["report", "--help"], "report help");
    assert!(
        stdout(&run).contains("--pretty"),
        "report help must teach the flag:\n{}",
        stdout(&run)
    );
}
