//! Misuse diagnostics US 02: the agent guide's probe-before-report
//! discipline. The `Reporting a gap` section of `docs/archspec/agents.md`
//! is welded literal: an agent that would file a claim it never ran finds
//! the three verbatim quotes it must attach, the known-boundaries rows that
//! document the honest silences, and the which-command-proves-which-claim
//! table — whose commands are run green here so the guide can never route
//! an agent to a dead command. Payload hygiene: the section cites no
//! GitHub issue numbers; provenance lives in the private worklog.

mod common;

use common::Fixture;
use std::fs;
use std::path::PathBuf;

fn agents_guide() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/archspec/agents.md");
    fs::read_to_string(&path).expect("read agents guide")
}

fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The section bytes, from its heading to the next `##` heading.
fn reporting_section(guide: &str) -> &str {
    let start = guide
        .find("## Reporting a gap")
        .expect("the guide carries a Reporting a gap section");
    let rest = &guide[start..];
    match rest[3..].find("\n## ") {
        Some(end) => &rest[..end + 1],
        None => rest,
    }
}

#[test]
fn guide_states_probe_before_report_bytes() {
    let guide = agents_guide();
    let section = flatten(reporting_section(&guide));
    for literal in [
        "run the claim against the RELEASE binary you cite",
        "`archspec --version`",
        "verbatim command",
        "verbatim output",
        "known-boundaries",
        "before calling silence a defect",
    ] {
        assert!(
            section.contains(literal),
            "the Reporting a gap section must state {literal:?} verbatim, got:\n{section}"
        );
    }
}

#[test]
fn known_boundaries_rows_point_at_owner_surfaces() {
    let guide = agents_guide();
    let section = flatten(reporting_section(&guide));
    // The three honest-silence families, each pointing at the surface that
    // states the boundary (this is a pointer table, not a restatement).
    for row in [
        "semicolon-bodied",
        "not-emitted",
        "by decision",
        "capability matrix",
        "help roles",
        "ADR-019",
    ] {
        assert!(
            section.contains(row),
            "the known-boundaries table must cover {row:?}, got:\n{section}"
        );
    }
}

#[test]
fn guide_maps_each_claim_kind_to_a_proving_command() {
    let guide = agents_guide();
    let section = reporting_section(&guide);
    for command in [
        "archspec scan",
        "archspec verify --strict",
        "archspec inspect tree",
        "archspec capability matrix",
    ] {
        assert!(
            section.contains(command),
            "the which-command table must name `{command}`:\n{section}"
        );
    }
}

#[test]
fn every_proving_command_runs_green() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("src/lib.rs", "pub mod api;\npub mod core;\n");
    fixture.write("src/api.rs", "use crate::core;\npub fn surface() { core::inner() }\n");
    fixture.write("src/core.rs", "pub fn inner() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
    );
    for args in [
        vec!["scan"],
        vec!["verify", "--strict"],
        vec!["inspect", "tree"],
        vec!["capability", "matrix"],
    ] {
        let output = fixture.run(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "`archspec {}` must exit 0 for the guide's table to be alive",
            args.join(" ")
        );
    }
}

#[test]
fn reporting_section_names_no_issue_numbers() {
    // Payload hygiene: generic pattern language; the private worklog ledger
    // carries the issue provenance.
    let guide = agents_guide();
    let section = reporting_section(&guide);
    for forbidden in ["issue #", "Issue #", "ISSUE #", "github.com"] {
        assert!(
            !section.contains(forbidden),
            "payload prose must describe patterns generically, found {forbidden:?}"
        );
    }
}
