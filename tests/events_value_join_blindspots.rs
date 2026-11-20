//! Scenario 06 — the `dispatch-blinded-literals` projection row (workplan
//! `events_value_join`, ADR-069 clause 4.8 + decision-check F-W4 option a).
//! The done-when cells:
//!
//! - (cell A) a dual-marker fixture (root marker selects the driver; a
//!   nested directory carries markers for two languages) with an event
//!   literal in the LOSING language's file → exactly ONE
//!   `dispatch-blinded-literals` row naming zone, langs, claimed driver and
//!   file count, ZERO joined rows, exit 0, verdict not red — the F-W4
//!   proving test; the losing file is never parsed, so the census input is
//!   the dispatch row plus the declared pattern census, never a scan of
//!   the blinded files;
//! - (cell B) the zero arms: a tree with no dispatch row prints zero
//!   projection rows, and a tree whose declaration carries no side for the
//!   zone's losing language prints zero rows (silent-by-zero-input);
//! - (cell C) the shipped `AMBIGUOUS-MARKERS` lines stay byte-identical to
//!   the base binary on the same fixture (the projection reads those rows,
//!   it never moves one byte of them);
//! - (weld) the projection source itself through `include!` — the parser
//!   grammar, the zero-input arms and the zone/file census, the single
//!   spelling of the law (the join weld of scenario 04, same shape).
//!
//! Names are synthetic (F10); the language facts are generic.

mod common;

use common::{Fixture, stderr, stdout};

#[allow(dead_code)]
mod projection {
    include!("../src/archspec/events/blindspots.rs");
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The declaring spec for the blinded-zone fixture: a rust publish side and
/// a ts subscribe side, plus the `[project] excluded_sources` entry the
/// shipped language-agreement law asks for by name when a dual-marker zone
/// carries the losing language's sources (`language mismatch: … declare
/// the tree with excluded_sources entries`). The entry moves NO dispatch
/// byte: `marker_report` groups markers behind the MECHANICAL exclusion
/// set only, so the `AMBIGUOUS-MARKERS` row stands unchanged.
fn blinded_zone_spec() -> &'static str {
    "[project]\nlanguage = \"rust\"\nexcluded_sources = [ { path = \"pkg\", reason = \"the zone ships its ts side separately\" } ]\n\n[[pattern_pair]]\nid = \"w06-bus\"\n\n\
     [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"emit\"\narg = 0\n\n\
     [pattern_pair.subscribe]\nlanguage = \"ts\"\nrole = \"subscribe\"\ncall = \"listen\"\narg = 0\n"
}

/// A rust-rooted tree whose nested `pkg` directory is a dual-marker zone:
/// `Cargo.toml` (rust, the run's own driver — not a loser) and
/// `package.json` (ts — the language that loses dispatch). `pkg/events.ts`
/// carries the event literal in the LOSING language's file; the rust
/// publish site lives in the modeled crate.
fn blinded_zone_tree(zone_package_json: bool, spec: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\"]\nresolver = \"2\"\n",
    );
    fixture.write("architecture.spec.toml", spec);
    fixture.write(
        "app/Cargo.toml",
        "[package]\nname = \"w06-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "app/src/lib.rs",
        "pub fn announce() {\n    emit(\"orders.created\");\n}\n\nfn emit(_event: &str) {}\n",
    );
    fixture.write(
        "pkg/Cargo.toml",
        "[package]\nname = \"w06-zone\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    if zone_package_json {
        fixture.write("pkg/package.json", "{\"name\": \"w06-zone\", \"version\": \"0.1.0\"}\n");
    }
    fixture.write(
        "pkg/events.ts",
        "export function init() {\n    listen(\"orders.created\");\n}\n\nfunction listen(_event: string) {}\n",
    );
    fixture
}

/// The scenario's declaring fixture: the dual-marker zone at full strength.
fn blinded_zone_fixture() -> Fixture {
    blinded_zone_tree(true, blinded_zone_spec())
}

/// The projection row the cell-A fixture must print — one byte family,
/// welded to the claim canonical with the placeholders resolved.
const CELL_A_ROW: &str = "dispatch-blinded-literals zone=pkg langs=\"ts\" claimed=rust files=0";

// ---------------------------------------------------------------------------
// cell A — dual-marker zone with the losing literal: ONE row, zero joins
// ---------------------------------------------------------------------------

/// The F-W4 proving test: the dispatch law hands the zone to the run's
/// driver, the declared ts side therefore collects nothing inside the zone
/// — and the census SAYS so with exactly one `dispatch-blinded-literals`
/// row. The join stays empty (the losing publish… subscribe pair can never
/// meet: the blinded file was never parsed), the run exits 0, and no
/// verdict moves. The dispatch row itself prints unchanged.
#[test]
fn dual_marker_zone_blinding_a_declared_language_prints_one_row_and_no_joins() {
    let fixture = blinded_zone_fixture();
    let out = fixture.run(&["scan", ".", "--output", "scan.json"]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    let err = stderr(&out);

    // The shipped dispatch row stands unchanged (this scenario reads it).
    assert_eq!(
        err.matches("AMBIGUOUS-MARKERS").count(),
        1,
        "the dispatch face still answers exactly one row:\n{err}"
    );
    assert!(
        err.contains(
            "AMBIGUOUS-MARKERS: pkg — markers: Cargo.toml (rust), package.json (ts), modeled as: rust"
        ),
        "the shipped dispatch bytes moved:\n{err}"
    );

    // The projection: exactly ONE row, naming zone, langs, driver, count.
    assert_eq!(
        err.matches("dispatch-blinded-literals").count(),
        1,
        "exactly one dispatch-blinded row per zone carrying a dispatch row:\n{err}"
    );
    assert!(err.contains(CELL_A_ROW), "the row bytes:\n{err}");

    // Zero silence holds: the publish site is NAMED, never joined silently,
    // and zero async rows are derived (the blinded half never joined).
    assert!(
        err.contains("one-sided-leg pattern=\"w06-bus\""),
        "the publish site must land in its named zero-silence row:\n{err}"
    );
    assert!(
        !err.contains("async-row derived"),
        "a blinded counterpart never joins — no derived rows here:\n{err}"
    );

    // The zero-input face: the census states the ts side's ZERO sites, so
    // the projection's absence-by-zero-input stays distinguishable from
    // silence — here the row is present and the zero is its input.
    assert!(
        err.contains("literal sites: pattern \"w06-bus\" language \"ts\" anchor call-name+arg-position sites=0"),
        "the blinded side's census zero must be stated:\n{err}"
    );

    // Report-only channel: the machine body carries no projection bytes.
    assert!(
        !fixture.read("scan.json").contains("dispatch-blinded"),
        "the row never rides the machine body"
    );
}

// ---------------------------------------------------------------------------
// cell B — the zero arms
// ---------------------------------------------------------------------------

/// A tree with no dispatch row prints zero projection rows: the zone
/// carries one language's marker only, the dispatch face stays silent, and
/// the projection — a pure reading of the dispatch surface — is silent
/// with it.
#[test]
fn a_tree_without_a_dispatch_row_prints_zero_projection_rows() {
    let fixture = blinded_zone_tree(false, blinded_zone_spec());
    let out = fixture.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    let err = stderr(&out);
    assert!(
        !err.contains("AMBIGUOUS-MARKERS"),
        "no dual-marker zone — no dispatch row:\n{err}"
    );
    assert!(
        !err.contains("dispatch-blinded-literals"),
        "no dispatch row — zero projection rows:\n{err}"
    );
}

/// The declaration gate's zero arm: the dispatch row stands, but the
/// declaration carries no side for the zone's losing language — the
/// projection has no census input to group and prints nothing
/// (silent-by-zero-input, the row's inputs stated by the literal-site
/// census alone… which a pattern-free run does not print at all: nothing
/// is declared, 4.2 arm 1).
#[test]
fn a_declaring_tree_without_a_side_for_the_losing_language_prints_zero_rows() {
    let spec = "[project]\nlanguage = \"rust\"\nexcluded_sources = [ { path = \"pkg\", reason = \"the zone ships its ts side separately\" } ]\n";
    let fixture = blinded_zone_tree(true, spec);
    let out = fixture.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    let err = stderr(&out);
    assert_eq!(
        err.matches("AMBIGUOUS-MARKERS").count(),
        1,
        "the dispatch face is declaration-independent — its row still prints:\n{err}"
    );
    assert!(
        !err.contains("dispatch-blinded-literals"),
        "no declared side for the losing language — zero projection rows:\n{err}"
    );
    assert!(
        !err.contains("literal sites:") && !err.contains("one-sided-leg"),
        "a pattern-free run prints no event-census bytes at all (4.2 arm 1):\n{err}"
    );
}

/// The third zero arm, at the seam: a declared side EXISTS for the losing
/// language but the declaration's language never loses this tree's
/// dispatch — a zone whose losing languages are all undeclared prints
/// nothing. Here the ts side is declared and the zone loses ts, so the
/// sibling control is a declaration whose declared set is exactly the
/// winning languages: declare rust on BOTH sides and the losing ts names
/// no declared side — zero rows even though the dispatch row stands.
#[test]
fn a_zone_whose_losing_language_is_undeclared_prints_zero_rows() {
    let spec = "[project]\nlanguage = \"rust\"\nexcluded_sources = [ { path = \"pkg\", reason = \"the zone ships its ts side separately\" } ]\n\n\
                [[pattern_pair]]\nid = \"w06-rust-only\"\n\n\
                [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"emit\"\narg = 0\n\n\
                [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"listen\"\narg = 0\n";
    let fixture = blinded_zone_tree(true, spec);
    let out = fixture.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    let err = stderr(&out);
    assert_eq!(
        err.matches("AMBIGUOUS-MARKERS").count(),
        1,
        "the dispatch row is unchanged by the declaration shape:\n{err}"
    );
    assert!(
        !err.contains("dispatch-blinded-literals"),
        "the losing ts language carries no declared side — zero projection rows:\n{err}"
    );
}

// ---------------------------------------------------------------------------
// cell C — the shipped dispatch face stays byte-identical
// ---------------------------------------------------------------------------

/// The byte-identity witness: on the shipped spec-free dual-marker fixture
/// the `AMBIGUOUS-MARKERS` lines diff empty against the base binary — the
/// projection reads those rows and moves none of them. A base binary
/// resolves as `ARCHSPEC` -> `archspec` on PATH -> named SKIP (the
/// global-install resolution rule).
#[test]
fn the_shipped_dispatch_lines_diff_empty_against_the_base_binary() {
    let fixture = Fixture::new();
    fixture.write("package.json", "{\"name\": \"w06-parent\", \"version\": \"0.1.0\"}\n");
    fixture.write("src/app.ts", "export const app = 1;\n");
    fixture.write(
        "pkg/Cargo.toml",
        "[package]\nname = \"w06-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("pkg/package.json", "{\"name\": \"w06-probe\", \"version\": \"0.1.0\"}\n");
    fixture.write("pkg/src/lib.rs", "pub fn local_helper() -> u32 { 1 }\n");
    fixture.write("pkg/src/index.js", "export function localHelper() { return 1; }\n");

    let fresh = fixture.run(&["scan", "."]);
    assert!(fresh.status.success(), "rooted scan failed: {}", stderr(&fresh));
    let Some(base) = base_binary() else {
        eprintln!("dispatch-parity: SKIP — no base binary (ARCHSPEC unset, none on PATH)");
        return;
    };
    let base_out = std::process::Command::new(&base)
        .args(["scan", "."])
        .current_dir(&fixture.root)
        .output()
        .expect("run base binary");
    let rows = |out: &std::process::Output| -> Vec<String> {
        stderr(out)
            .lines()
            .filter(|line| line.starts_with("AMBIGUOUS-MARKERS"))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(
        rows(&fresh),
        rows(&base_out),
        "the AMBIGUOUS-MARKERS lines moved — fresh:\n{}\nbase:\n{}",
        stderr(&fresh),
        stderr(&base_out)
    );
    assert!(!rows(&fresh).is_empty(), "the fixture must carry a dispatch row");
    let _ = stdout(&fresh);
}

fn base_binary() -> Option<String> {
    if let Some(path) = std::env::var("ARCHSPEC").ok().filter(|p| !p.is_empty()) {
        return Some(path);
    }
    let probe = std::process::Command::new("archspec")
        .arg("--version")
        .output()
        .ok()?;
    probe.status.success().then(|| "archspec".to_string())
}

// ---------------------------------------------------------------------------
// weld — the ONE projection implementation through include!
// ---------------------------------------------------------------------------

/// The parser grammar read off the shipped row's bytes, and the
/// projection's input laws: a losing language the declaration names earns
/// ONE row; an undeclared loser stays silent-by-zero-input; the file
/// census counts DISTINCT blinded-language files INSIDE the zone and
/// nothing else (claimed-language files never count, sibling directories
/// never match). Welded to the single site — never a second spelling.
#[test]
fn the_projection_parser_and_input_census_weld_to_the_single_site() {
    use projection::{dispatch_blinded_rows, parse_dispatch_line, SiteRef};

    let line = "AMBIGUOUS-MARKERS: pkg — markers: Cargo.toml (rust), package.json (ts), modeled as: ts";
    let row = parse_dispatch_line(line).expect("the shipped line parses");
    assert_eq!(row.zone, "pkg");
    assert_eq!(row.langs, vec!["rust", "ts"]);
    assert_eq!(row.claimed, "ts");
    assert_eq!(
        parse_dispatch_line("nested foreign marker: pkg/go.mod (go) — not modelled"),
        None,
        "a foreign-marker line is not a dispatch row"
    );

    let declared: std::collections::BTreeSet<String> = ["rust", "ts"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let sites = [
        SiteRef { path: "pkg/src/lib.rs", language: "rust" },
        SiteRef { path: "pkg/src/other.rs", language: "rust" },
        SiteRef { path: "pkg/src/lib.rs", language: "rust" }, // second site, same file
        SiteRef { path: "pkg/src/index.js", language: "ts" }, // claimed language — never counts
        SiteRef { path: "pkgs/src/lib.rs", language: "rust" }, // sibling dir — never inside
    ];
    let rows = dispatch_blinded_rows(
        &format!("{line}\nnested foreign marker: x/package.json (ts) — not modelled\n"),
        &declared,
        &sites,
    );
    assert_eq!(
        rows,
        vec!["dispatch-blinded-literals zone=pkg langs=\"rust\" claimed=ts files=2".to_string()],
        "one row per dispatch-carrying zone; DISTINCT files of the blinded language inside the zone"
    );

    // Zero arms at the pure seam: no dispatch row, and a declaration that
    // names no losing side, both print nothing.
    let ts_only: std::collections::BTreeSet<String> =
        ["ts"].into_iter().map(str::to_string).collect();
    assert!(dispatch_blinded_rows(line, &ts_only, &sites).is_empty());
    assert!(dispatch_blinded_rows("", &declared, &sites).is_empty());
    // A nested zone path matches component-wise, never by string prefix.
    let nested = "AMBIGUOUS-MARKERS: crates/node — markers: Cargo.toml (rust), package.json (ts), modeled as: ts";
    let rows = dispatch_blinded_rows(nested, &declared, &[SiteRef {
        path: "crates/nodejs/src/lib.rs",
        language: "rust",
    }]);
    assert_eq!(
        rows,
        vec!["dispatch-blinded-literals zone=crates/node langs=\"rust\" claimed=ts files=0".to_string()],
        "the sibling `crates/nodejs` is not inside zone `crates/node`"
    );
}
