//! issue 9 pointer guard — the wrong-edge routing lives on the depgraph help
//! surfaces.
//!
//! A reader who asks `depgraph` "which edges are wrong?" hits a spec-free
//! projection (the command doctrine: nothing rendered but current state). The
//! capability itself lives on the diagram side — `archspec diagram --source
//! scan <scan.json> --tiers modules` marks spec-violating links through the
//! shared Adjudicator — so the depgraph surfaces carry ONE pointer sentence,
//! not a second marking surface (decision-check:
//! `rust-arch-test-kit/worklog/2026-11-05-archspec-issue9-depgraph-marks-decision-check.md`,
//! ruling D6 and BLOCKER-1: no new projection surface, discoverability only).
//!
//! This guard is the adversarial pin: drop the pointer (or its verb form)
//! from either depgraph help surface and this test goes red; the
//! `depgraph-violations-pointer` registry entry welds the sentence bytes and
//! the fan-out scan welds the doc echo.

mod common;

use common::{stderr, stdout, Fixture};

/// The registered pointer sentence (claim `depgraph-violations-pointer`) —
/// bytes welded, position free.
const VIOLATIONS_POINTER_SENTENCE: &str = "To ask which edges are wrong, render the same model with `archspec diagram --source scan <scan.json> --tiers modules` — under a governing spec that view marks spec-violating edges.";

#[test]
fn depgraph_help_surfaces_point_wrong_edge_questions_at_diagram_scan_mode() {
    let fixture = Fixture::new();
    for args in [vec!["depgraph", "--help"], vec!["help", "depgraph"]] {
        let output = fixture.run(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "`archspec {}` must exit 0: {}",
            args.join(" "),
            stderr(&output)
        );
        let out = stdout(&output);
        // The pointer verb form: the command a wrong-edge question is routed
        // to. A future edit that drops the pointer — or rewords it away from
        // the scan source — fails here first.
        assert!(
            out.contains("diagram --source scan"),
            "`archspec {}` must name the scan-source render for wrong-edge \
             questions, got:\n{out}",
            args.join(" ")
        );
        // The full registered sentence, byte-identical.
        assert!(
            out.contains(VIOLATIONS_POINTER_SENTENCE),
            "`archspec {}` must state the violations pointer verbatim:\n{out}",
            args.join(" ")
        );
    }
}
