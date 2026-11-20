//! Scenario 04 — the first `Model::async_edges` construction site: the
//! generic cross-unit value join (workplan `events_value_join`, ADR-069
//! clauses 4.4/4.4a, decision-check F1/F3/F4/F-W3). The done-when cells:
//!
//! - (cell A) a declared `[[async_depends]]` row plus an identity-equal
//!   derived row render ONE arrow with exactly ONE `PLUGIN-OVERLAP` row —
//!   the F4 merge proving test (a derived row never suppresses a declared
//!   arrow: the render set is the union of both sources);
//! - (cell B) derived-only rows render one arrow with the derived marker,
//!   exit 0 under `verify --strict` — never red (F4's never-red arm: the
//!   verify arms stay gated on the declared table);
//! - (cell C) one dangling subscriber id plus one unoriented const:
//!   joined=0, exactly TWO named rows (`one-sided-leg` + `ambiguous-leg`),
//!   the verdict not red — the F1 zero-silence proving test;
//! - (cell D) a role-less pattern pair joins nothing and prints one
//!   `ambiguous-leg` row per matched site (the F3 proving test, 4.4a(v));
//! - (cell E) a rust publish site and a JS subscribe site over ONE value
//!   join to ONE row — the F-W3 proving test, welded from the ONE join
//!   implementation (`events/join.rs` through `include!`, never a second
//!   spelling of the law);
//! - (cell F) two declared conventions match one raw string: the join
//!   keys per convention, joins nothing, and names BOTH sites (4.4a(ii)).
//!
//! Endpoints carry the tree-root-canonical unit ids (4.4a(i), scenario 01);
//! the pattern sides come from scenario 03; the nouns are synthetic (F10).

mod common;

use common::{Fixture, stderr, stdout};

#[allow(dead_code)]
mod value_join {
    include!("../src/archspec/events/join.rs");
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The demo-bus pattern pair on the rust driver: the emit side is the
/// call-name+argument-position verb on `emit`, the subscribe side the same
/// verb on `listen`; both sides declare their enumerated role (the SOLE
/// orientation source — F3: the engine reads no semantics off the names).
const DEMO_BUS_PAIR: &str = "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"demo-bus\"\n\n\
    [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"emit\"\narg = 0\n\n\
    [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"listen\"\narg = 0\n";

/// A two-crate rust workspace: `api` publishes, `worker` subscribes. The
/// join meets the two units over ONE string value — the cross-unit shape
/// the corpus witnesses carry, spelled with synthetic nouns only (F10).
/// `worker` carries TWO modules over one internal reference so the model
/// holds the module tier the `depgraph` render needs.
fn demo_bus_tree(extra_spec: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"api\", \"worker\"]\nresolver = \"2\"\n",
    );
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "{DEMO_BUS_PAIR}\n[[module]]\nname = \"api\"\nmatches = {{ units = [\"api\"] }}\n\n[[module]]\nname = \"worker\"\nmatches = {{ units = [\"worker\"] }}\n{extra_spec}"
        ),
    );
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "pub fn announce() {\n    emit(\"orders.created\");\n}\n\nfn emit(_event: &str) {}\n",
    );
    fixture.write(
        "worker/Cargo.toml",
        "[package]\nname = \"worker\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "worker/src/lib.rs",
        "pub mod handler;\n\nfn listen(_event: &str) {}\n",
    );
    fixture.write(
        "worker/src/handler.rs",
        "use crate::listen;\n\npub fn init() {\n    listen(\"orders.created\");\n}\n",
    );
    fixture
}

/// One crate, caller-supplied sources and spec — the named-row cells need
/// source shapes (dangling ids, role-less consts, colliding conventions),
/// not the two-unit bus.
fn single_crate_tree(spec: &str, source: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("architecture.spec.toml", spec);
    fixture.write("src/lib.rs", source);
    fixture
}

/// One `scan --output -` that must succeed, returned for stdout/stderr
/// assertions.
fn scan(fixture: &Fixture) -> std::process::Output {
    let output = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&output)
    );
    output
}

fn count_lines(haystack: &str, prefix: &str) -> usize {
    haystack
        .lines()
        .filter(|line| line.starts_with(prefix))
        .count()
}

// ---------------------------------------------------------------------------
// cell A — declared + identity-equal derived: ONE arrow, ONE PLUGIN-OVERLAP
// ---------------------------------------------------------------------------

/// The F4 merge proving test (issue #48 follow-up, ADR-069 4.4): the
/// declared row and the derived row carry ONE identity after
/// canonicalization — the model states the derived row once, the merge
/// prints exactly ONE `PLUGIN-OVERLAP` row, and the units render draws
/// ONE arrow (the render set is the union: a derived row can never
/// suppress the declared arrow, it can only merge with it).
#[test]
fn declared_row_and_identity_equal_derived_row_merge_to_one_arrow() {
    let fixture = demo_bus_tree(
        "\n[[async_depends]]\nfrom = \"api\"\nto = \"worker\"\nevent = \"orders.created\"\n",
    );
    let output = scan(&fixture);
    let json = stdout(&output);
    let err = stderr(&output);
    assert!(
        json.contains("\"async_edges\":[{"),
        "a declaring scan states the derived async rows (the first \
         construction site of the frozen `async_edges` collection):\n{json}"
    );
    assert_eq!(
        json.matches("\"from\":\"api\"").count(),
        1,
        "the derived row rides the model once, in the canonical id spelling:\n{json}"
    );
    assert!(
        json.contains("\"event\":\"orders.created\"") && json.contains("\"kind\":\"async\""),
        "the derived row carries the joined VALUE and the async kind label:\n{json}"
    );
    assert_eq!(
        count_lines(&err, "PLUGIN-OVERLAP"),
        1,
        "identity-equal derived + declared merge with exactly ONE \
         PLUGIN-OVERLAP row (stderr: {err})"
    );
    let render = fixture.run(&["depgraph", "modules", "--aggregate", "units"]);
    assert_eq!(
        render.status.code(),
        Some(0),
        "the units render must pass (stderr: {})",
        stderr(&render)
    );
    assert_eq!(
        stdout(&render)
            .lines()
            .filter(|line| line.contains("-.->") && line.contains("api") && line.contains("worker"))
            .count(),
        1,
        "one merged identity renders ONE arrow — never two, never zero"
    );
}

// ---------------------------------------------------------------------------
// cell B — derived-only: one arrow, the derived marker, never red
// ---------------------------------------------------------------------------

/// The F4 never-red arm: with NO declared table the join still writes the
/// derived row, the units render draws one arrow, the census marks the
/// row derived-only by name, and `verify --strict` stays green — the
/// verify arms are gated on the declared table, so a derived-only model
/// can never go red on the machine facts alone.
#[test]
fn derived_only_row_renders_with_marker_and_never_goes_red() {
    let fixture = demo_bus_tree("");
    let output = scan(&fixture);
    let json = stdout(&output);
    let err = stderr(&output);
    assert!(
        json.contains("\"async_edges\":[{"),
        "the value join fills the model's async collection:\n{json}"
    );
    assert_eq!(
        count_lines(&err, "PLUGIN-OVERLAP"),
        0,
        "nothing merges when nothing is declared (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "async-row derived"),
        1,
        "the derived-only row states its derived marker once (stderr: {err})"
    );
    let render = fixture.run(&["depgraph", "modules", "--aggregate", "units"]);
    assert_eq!(
        stdout(&render)
            .lines()
            .filter(|line| line.contains("-.->") && line.contains("api") && line.contains("worker"))
            .count(),
        1,
        "the derived-only row renders one arrow (stdout: {})",
        stdout(&render)
    );
    let verify = fixture.run(&["verify", ".", "--strict"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "derived-only rows never go red (F4): {}",
        stdout(&verify)
    );
}

// ---------------------------------------------------------------------------
// cell C — dangling subscriber + unoriented const: joined=0, named rows=2
// ---------------------------------------------------------------------------

/// The F1 zero-silence proving test: a subscribe site whose value no
/// publish site carries is ONE `one-sided-leg` row; a const matched by a
/// side whose pair declares the const side with no role is ONE
/// `ambiguous-leg` row. Zero rows join, both shapes are stated on the
/// report surface, and the verdict stays not red.
#[test]
fn dangling_subscriber_and_unoriented_const_land_named_not_silent() {
    let spec = "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[pattern_pair]]\nid = \"bus\"\n\n\
        [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"emit\"\narg = 0\n\n\
        [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"listen\"\narg = 0\n\n\
        [[pattern_pair]]\nid = \"loose\"\n\n\
        [pattern_pair.emit]\nlanguage = \"rust\"\ncall = \"constant\"\narg = 0\n\n\
        [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"attach\"\narg = 0\n";
    let source = "pub fn shapes() {\n    listen(\"gone.event\");\n    constant(\"protocol.name\");\n}\n\n\
        fn listen(_event: &str) {}\nfn constant(_name: &str) {}\n";
    let fixture = single_crate_tree(spec, source);
    let output = scan(&fixture);
    let json = stdout(&output);
    let err = stderr(&output);
    assert!(
        !json.contains("\"async_edges\":[{"),
        "joined=0: nothing pairs here — the model states no async rows:\n{json}"
    );
    assert_eq!(
        count_lines(&err, "one-sided-leg"),
        1,
        "the dangling subscriber id is named, never silent (stderr: {err})"
    );
    assert!(
        err.contains("value=\"gone.event\""),
        "the named row carries the dangling value (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "ambiguous-leg"),
        1,
        "the unoriented const is named, never silent (stderr: {err})"
    );
    assert!(
        err.contains("value=\"protocol.name\""),
        "the named row carries the const value (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "unoriented "),
        0,
        "the pair whose other side states a role is not the pattern-level \
         unoriented class (stderr: {err})"
    );
    let verify = fixture.run(&["verify", ".", "--strict"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "named census rows are never verdicts (stdout: {}, stderr: {})",
        stdout(&verify),
        stderr(&verify)
    );
}

// ---------------------------------------------------------------------------
// cell D — role-less pattern pair: zero joined rows, one ambiguous-leg each
// ---------------------------------------------------------------------------

/// The F3 proving test (4.4a(v)): orientation comes ONLY from the declared
/// enumerated role. A pair with no role on either side joins NOTHING —
/// every matched site lands in exactly one `ambiguous-leg` row, plus the
/// pattern-level `unoriented` statement of why the pair joins nothing.
#[test]
fn role_less_pair_joins_nothing_and_names_every_site() {
    let spec = "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"loose\"\n\n\
        [pattern_pair.emit]\nlanguage = \"rust\"\ncall = \"constant\"\narg = 0\n\n\
        [pattern_pair.subscribe]\nlanguage = \"rust\"\ncall = \"attach\"\narg = 0\n";
    let source = "pub fn shapes() {\n    constant(\"v\");\n    constant(\"w\");\n    attach(\"v\");\n}\n\n\
        fn constant(_name: &str) {}\nfn attach(_name: &str) {}\n";
    let fixture = single_crate_tree(spec, source);
    let output = scan(&fixture);
    let json = stdout(&output);
    let err = stderr(&output);
    assert!(
        !json.contains("\"async_edges\":[{"),
        "a role-less pair joins nothing (F3):\n{json}"
    );
    assert_eq!(
        count_lines(&err, "ambiguous-leg"),
        3,
        "one ambiguous-leg row per matched site (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "unoriented "),
        1,
        "the role-less pair states its pattern-level unoriented row once \
         (stderr: {err})"
    );
}

// ---------------------------------------------------------------------------
// cell E — cross-language value join (welded from the ONE implementation)
// ---------------------------------------------------------------------------

/// The F-W3 proving test: a rust publish site and a JS subscribe site over
/// ONE string value join to ONE row. The single-scan substrate answers one
/// driver per root (4.8 zone territory is scenario 05+), so this cell
/// welds `src/archspec/events/join.rs` through `include!` — macro text to
/// the scanner, the welded bytes are the single site, never a second
/// implementation. The join reads `language` as a FIELD for the sides it
/// orients through — no arm keys on it.
#[test]
fn same_value_two_languages_join_to_one_row() {
    let sites = [
        value_join::JoinSite {
            unit: "apps/desktop",
            path: "apps/desktop/src-tauri/src/lib.rs",
            language: "rust",
            value: "gateway.wakeup",
            pattern: "wakeup-bus",
            anchor: "call-name+arg-position",
            noun: "notify",
        },
        value_join::JoinSite {
            unit: "apps/desktop/web",
            path: "apps/desktop/web/listen.js",
            language: "ts",
            value: "gateway.wakeup",
            pattern: "wakeup-bus",
            anchor: "exported-property-value",
            noun: "onWakeup",
        },
    ];
    let sides = [
        value_join::JoinSide {
            pattern: "wakeup-bus",
            language: "rust",
            noun: "notify",
            role: Some("publish"),
        },
        value_join::JoinSide {
            pattern: "wakeup-bus",
            language: "ts",
            noun: "onWakeup",
            role: Some("subscribe"),
        },
    ];
    let outcome = value_join::value_join(
        &sites,
        &sides,
        &std::collections::BTreeSet::new(),
        &std::collections::BTreeSet::new(),
    );
    assert_eq!(
        outcome.named,
        Vec::<String>::new(),
        "both sides are oriented"
    );
    assert_eq!(
        outcome.rows,
        vec![value_join::JoinedRow {
            from: "apps/desktop".to_string(),
            to: "apps/desktop/web".to_string(),
            event: "gateway.wakeup".to_string(),
            pattern: "wakeup-bus".to_string(),
        }],
        "ONE row over the shared value — language keyed nothing"
    );
}

// ---------------------------------------------------------------------------
// cell F — one raw string, two conventions: no join, both sites named
// ---------------------------------------------------------------------------

/// The 4.4a(ii) collision arm: the key space is the raw value bytes; when
/// TWO declared conventions match the SAME raw string the join keys per
/// convention — it never guesses that a bus key and a file key name one
/// channel. The split prints ONE `value-namespace` row and each site
/// still lands in its own named row (zero silence).
#[test]
fn two_conventions_on_one_raw_value_do_not_join_and_name_both() {
    let spec = "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"bus-a\"\n\n\
        [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"emitA\"\narg = 0\n\n\
        [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"listenA\"\narg = 0\n\n\
        [[pattern_pair]]\nid = \"conv-b\"\n\n\
        [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"emitB\"\narg = 0\n\n\
        [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"listenB\"\narg = 0\n";
    let source = "pub fn shapes() {\n    emitA(\"shared.value\");\n    listenB(\"shared.value\");\n}\n\n\
        fn emitA(_event: &str) {}\nfn listenB(_event: &str) {}\n";
    let fixture = single_crate_tree(spec, source);
    let output = scan(&fixture);
    let json = stdout(&output);
    let err = stderr(&output);
    assert!(
        !json.contains("\"async_edges\":[{"),
        "a convention collision joins nothing across the conventions:\n{json}"
    );
    assert_eq!(
        count_lines(&err, "value-namespace"),
        1,
        "the collision on the raw value names itself once (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "one-sided-leg"),
        2,
        "both sites land in their own named rows — zero silence holds \
         inside the split (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "ambiguous-leg"),
        0,
        "oriented sites never fall to the ambiguous arm (stderr: {err})"
    );
}
