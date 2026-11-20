//! The ambiguous-publish attribution census face (workplan events_value_join
//! scenario 05, ADR-069 4.4a(b)): a hand-declared `[[async_depends]]` row
//! that passes the verify arms under declaration trust while naming the
//! DECLARING unit instead of the DISPATCHING unit — the M-4 assembly-scan
//! class — prints exactly ONE named census row on the REPORT-ONLY channel,
//! never a verdict (the roadmap §2 commitment: a silent zero output is
//! forbidden where the mismatch fires). Cells:
//!
//! - (cell A) a declaring fixture whose declared row names the declaring
//!   unit while the dispatch-side derived fact names another: exactly ONE
//!   named row on the verify stderr, exit 0 under `--strict` too, the
//!   census class counted on the json and report surfaces, zero findings;
//! - (cell B) declaring side == dispatching side: ZERO attribution bytes,
//!   the row merges through ONE `PLUGIN-OVERLAP` row as scenario 04 shipped;
//! - (cell C) a non-declaring fixture: ZERO attribution bytes, `scan.json`
//!   carries no attribution byte (`report_only_lines` is serde-skipped —
//!   the base-binary byte identity of the done-when rides the corpus
//!   battery leg recorded in the scenario Evidence);
//! - (cell W) the comparison law welded to the ONE implementation
//!   (`events/attribution.rs` through `include!`, never a second spelling).
//!
//! Endpoints carry the tree-root-canonical unit ids (4.4a(i), scenario 01);
//! the pattern sides and derived rows are scenario 04's machinery — this
//! face adds no join logic. The nouns are synthetic (F10).

mod common;

use common::{Fixture, stderr, stdout};

#[allow(dead_code)]
mod attribution_weld {
    include!("../src/archspec/events/attribution.rs");
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The demo-bus pattern pair on the rust driver with the hand-declared edge
/// appended: `from` names the unit the scenario cell puts on the declared
/// row, so the same tree answers both 4.4a(b) arms — mismatching (`declweld`
/// declares, `pubweld` dispatches) and attributed (`pubweld` both sides).
/// The event identity is the TYPE FQN shape of the M-4 witness: the verify
/// arm resolves its tail (`ThingDone`) in the model-wide declared-type
/// census, while the publish/subscribe arms stay capability-inert for the
/// rust driver — the declaration-trust that makes the mismatching row
/// green-but-wrong in the first place.
fn demo_bus_spec(declared_from: &str) -> String {
    format!(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"pubweld\"\nmatches = {{ units = [\"pubweld\"] }}\n\n[[module]]\nname = \"subweld\"\nmatches = {{ units = [\"subweld\"] }}\n\n[[module]]\nname = \"declweld\"\nmatches = {{ units = [\"declweld\"] }}\n\n[[pattern_pair]]\nid = \"demo-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"notify\"\narg = 0\n\n\
         [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"listen\"\narg = 0\n\n\
         [[async_depends]]\nfrom = \"{declared_from}\"\nto = \"subweld\"\nevent = \"pubweld::ThingDone\"\n"
    )
}

/// A three-crate rust workspace: `pubweld` carries the publish site (the
/// DISPATCHING side the derived fact names), `subweld` the subscribe site,
/// `declweld` the unit that merely HOLDS the declaration — the assembly
/// unit of the M-4 shape, carrying no site of either pattern.
fn declaring_workspace(declared_from: &str) -> Fixture {
    let fx = Fixture::new();
    fx.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"pubweld\", \"subweld\", \"declweld\"]\nresolver = \"2\"\n",
    );
    fx.write("architecture.spec.toml", demo_bus_spec(declared_from).as_str());
    fx.write(
        "pubweld/Cargo.toml",
        "[package]\nname = \"pubweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write(
        "pubweld/src/lib.rs",
        "pub struct ThingDone;\n\npub fn go() {\n    notify(\"pubweld::ThingDone\");\n}\n\nfn notify(_event: &str) {}\n",
    );
    fx.write(
        "subweld/Cargo.toml",
        "[package]\nname = \"subweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write(
        "subweld/src/lib.rs",
        "pub fn init() {\n    listen(\"pubweld::ThingDone\");\n}\n\nfn listen(_event: &str) {}\n",
    );
    fx.write(
        "declweld/Cargo.toml",
        "[package]\nname = \"declweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write("declweld/src/lib.rs", "pub fn wiring() {}\n");
    fx
}

/// The class name every surface counts the census under.
const CLASS: &str = "ambiguous-publish-attribution";

fn census_lines(output: &std::process::Output) -> Vec<String> {
    stderr(output)
        .lines()
        .filter(|line| line.contains(CLASS))
        .map(str::to_string)
        .collect()
}

// ---------------------------------------------------------------------------
// Cell A — mismatch: exactly ONE named row, verdict unchanged
// ---------------------------------------------------------------------------

/// The M-4 class made VISIBLE: the declared row names `declweld` (the unit
/// that DECLARES the edge) while the dispatch-side derived fact names
/// `pubweld` (the unit that DISPATCHES the value). `verify` passes both
/// arms unchanged — declaration trust stands, the row is never re-keyed —
/// and prints the ONE named census row on the report channel; `--strict`
/// keeps the run green (a report-only line can never go red); the json
/// findings stay empty while the environment census counts the class; the
/// report surface counts the same class in its table (the census is one
/// computation, two renderings).
#[test]
fn mismatching_declared_row_prints_one_census_row_and_stays_green() {
    let tree = declaring_workspace("declweld");
    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "the face never reddens a verdict: the declared row passes the verify arms: {}",
        stdout(&verify)
    );
    let lines = census_lines(&verify);
    assert_eq!(
        lines,
        vec![format!(
            "{CLASS}: row=(declweld, subweld, pubweld::ThingDone) dispatching pubweld — \
             the dispatch-side facts for this value and target name pubweld, not the \
             declared from; the declared row stands unchanged"
        )],
        "exactly ONE named census row, the dispatching unit named (4.4a(b))"
    );
    let strict = tree.run(&["verify", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(0),
        "the census rides the report-only channel: `--strict` never promotes it: {}",
        stdout(&strict)
    );

    // The machine surface: zero findings of the census class, the class
    // counted once in the environment census — the count over the lines
    // this run already emitted, never a second computation.
    let json = tree.run(&["verify", "--json"]);
    assert_eq!(json.status.code(), Some(0));
    let body: serde_json::Value =
        serde_json::from_str(&stdout(&json)).expect("verify --json must parse");
    assert!(
        body["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .all(|finding| finding["category"].as_str() != Some(CLASS)),
        "the census never rides the findings array: {body}"
    );
    assert_eq!(
        body["environment"]["classes"][CLASS], 1,
        "the environment census counts exactly one {CLASS} line: {body}"
    );

    // The report surface counts the same class (one adjudication, every
    // renderer): the table row is the same computation the stderr line fed.
    let report = tree.run(&["report", "--format", "text"]);
    assert_eq!(report.status.code(), Some(0));
    assert!(
        stdout(&report).contains(CLASS),
        "the report surface counts the census class: {}",
        stdout(&report)
    );
}

// ---------------------------------------------------------------------------
// Cell B — match: zero rows, the merge machinery unchanged
// ---------------------------------------------------------------------------

/// The zero-row arm: a declared row the dispatch-side facts attribute
/// identically is the scenario-04 merge case — ONE `PLUGIN-OVERLAP` row,
/// the derived row claimed by the declared identity, and ZERO attribution
/// bytes: the face never fires where the declaring side IS the dispatching
/// side (4.4a(b), never a false positive).
#[test]
fn attributed_declared_row_prints_zero_census_rows() {
    let tree = declaring_workspace("pubweld");
    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "the attributed row verifies green: {}",
        stdout(&verify)
    );
    assert!(
        census_lines(&verify).is_empty(),
        "declaring side == dispatching side prints no attribution bytes"
    );
    let errors = stderr(&verify);
    assert!(
        errors.contains("PLUGIN-OVERLAP row=(pubweld, subweld, pubweld::ThingDone) producers=2"),
        "the merge machinery is unchanged — the identity-equal pair still merges: {errors}"
    );
    assert!(
        !errors.contains("async-row derived"),
        "the derived row stays claimed by the declared identity: {errors}"
    );
}

// ---------------------------------------------------------------------------
// Cell C — non-declaring: zero rows, machine bytes untouched
// ---------------------------------------------------------------------------

/// A tree that declares nothing never reaches the comparison: the derived
/// set is empty, the census prints nothing on every surface, and the model
/// body carries no attribution byte — `report_only_lines` is serde-skipped,
/// so `scan.json` cannot grow a byte from this face (the report-only
/// channel law; the base-binary byte identity of the done-when rides the
/// corpus battery leg, the scenario Evidence records its verdict).
#[test]
fn non_declaring_tree_prints_zero_rows_and_serializes_no_attribution_bytes() {
    let fx = Fixture::new();
    fx.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"pubweld\", \"subweld\"]\nresolver = \"2\"\n",
    );
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"pubweld\"\nmatches = { units = [\"pubweld\"] }\n\n[[module]]\nname = \"subweld\"\nmatches = { units = [\"subweld\"] }\n",
    );
    fx.write(
        "pubweld/Cargo.toml",
        "[package]\nname = \"pubweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write(
        "pubweld/src/lib.rs",
        "pub struct ThingDone;\n\npub fn go() {\n    notify(\"pubweld::ThingDone\");\n}\n\nfn notify(_event: &str) {}\n",
    );
    fx.write(
        "subweld/Cargo.toml",
        "[package]\nname = \"subweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write(
        "subweld/src/lib.rs",
        "pub fn init() {\n    listen(\"pubweld::ThingDone\");\n}\n\nfn listen(_event: &str) {}\n",
    );

    let scan = fx.run(&["scan", "--output", "-"]);
    assert_eq!(scan.status.code(), Some(0));
    assert!(
        !stderr(&scan).contains(CLASS),
        "a non-declaring run prints no attribution bytes: {}",
        stderr(&scan)
    );
    let body = stdout(&scan);
    assert!(
        !body.contains(CLASS) && !body.contains("attribution"),
        "the model body carries no attribution byte — the census channel is serde-skipped: {body}"
    );
    let verify = fx.run(&["verify", "."]);
    assert_eq!(verify.status.code(), Some(0));
    assert!(
        census_lines(&verify).is_empty(),
        "a non-declaring verify prints no attribution bytes"
    );
}

// ---------------------------------------------------------------------------
// Cell W — the comparison law welded to the ONE implementation
// ---------------------------------------------------------------------------

/// The `(String, String, String)` storage → `FactRow` adapter with the
/// borrow lifetime stated (a closure cannot name it).
fn fact<'a>(row: &'a (String, String, String)) -> attribution_weld::FactRow<'a> {
    attribution_weld::FactRow {
        from: &row.0,
        to: &row.1,
        event: &row.2,
    }
}

/// The welded file answers both 4.4a(b) arms on pre-canonicalized inputs —
/// the mismatch names the sorted set of dispatching units, the attributed
/// identity and the pair no dispatch fact shares print nothing. Macro text
/// to the scanner: these cells run THE file the binary compiles.
#[test]
fn welded_comparison_names_dispatching_units_and_holds_both_zero_arms() {
    let rows = [
        (
            "pubweld".to_string(),
            "subweld".to_string(),
            "thing.done".to_string(),
        ),
        (
            "pubweld".to_string(),
            "otherweld".to_string(),
            "thing.done".to_string(),
        ),
    ];
    let dispatch = rows.iter().map(fact).collect::<Vec<_>>();
    let declared = vec![attribution_weld::FactRow {
        from: "declweld",
        to: "subweld",
        event: "thing.done",
    }];
    let lines = attribution_weld::attribution_rows(&declared, &dispatch);
    assert_eq!(
        lines,
        vec![format!(
            "{CLASS}: row=(declweld, subweld, thing.done) dispatching pubweld — \
             the dispatch-side facts for this value and target name pubweld, not the \
             declared from; the declared row stands unchanged"
        )],
        "the welded file prints the ONE named row for the mismatching identity"
    );

    let attributed = vec![attribution_weld::FactRow {
        from: "pubweld",
        to: "subweld",
        event: "thing.done",
    }];
    assert!(
        attribution_weld::attribution_rows(&attributed, &dispatch).is_empty(),
        "the attributed identity prints zero rows"
    );
    let unshared = vec![attribution_weld::FactRow {
        from: "declweld",
        to: "nowhere",
        event: "other.event",
    }];
    assert!(
        attribution_weld::attribution_rows(&unshared, &dispatch).is_empty(),
        "a pair no dispatch fact shares prints zero rows"
    );
}
