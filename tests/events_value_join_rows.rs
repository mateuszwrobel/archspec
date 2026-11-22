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
//!
//! Workplan `visibility_phase2` scenario 02 extends this file with cells
//! G..J: the third matcher verb (field value) fed through the SAME welded
//! join — the join reads value bytes only, so field sites earn the same
//! rows the two shipped verbs earn.

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

// ---------------------------------------------------------------------------
// cells G..J — the third matcher verb: field value (workplan
// visibility_phase2 scenario 02). The join is UNTOUCHED: these cells feed it
// field-value sites and assert the SAME rows the two shipped verbs earn —
// the join reads value bytes only, never the anchor (join.rs is welded
// here, so its immutability is proven by this file compiling against the
// frozen bytes plus the empty byte-diff leg of the lane record).
// ---------------------------------------------------------------------------

/// The field-value pair on the rust driver: emit side `field = "topic"`
/// (publish), subscribe side `field = "handles"` (subscribe) — the sides
/// carry DISTINCT nouns exactly as the shipped call-verb pairs do: one
/// noun on both sides would put every site under both roles (the
/// ambiguous-leg arm is the honest answer for that declaration, not a
/// join). SYNTHETIC nouns (F10): `topic` and `handles` are generic words
/// no corpus declaration owns.
const FIELD_BUS_PAIR: &str = "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"field-bus\"\n\n\
    [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\nfield = \"topic\"\n\n\
    [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\nfield = \"handles\"\n";

/// cell G — the collector faces: a struct-expression field, a macro-token
/// object, and an array that IS a field value all contribute field-value
/// sites; a non-field position (a plain call argument, a tuple-position
/// string, a struct field of a non-matching noun) contributes none. Three
/// matching publish sites in `api` plus one subscribe site in `worker`
/// over ONE value join to ONE row — the volume-collapse shape the
/// openmercato ACL class shows, at fixture scale.
#[test]
fn field_value_sites_from_every_object_like_position_join_to_one_row() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"api\", \"worker\"]\nresolver = \"2\"\n",
    );
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "{FIELD_BUS_PAIR}\n[[module]]\nname = \"api\"\nmatches = {{ units = [\"api\"] }}\n\n[[module]]\nname = \"worker\"\nmatches = {{ units = [\"worker\"] }}\n"
        ),
    );
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "macro_rules! frame {\n    ($($t:tt)*) => {};\n}\n\n\
         struct Envelope {\n    topic: &'static str,\n    label: &'static str,\n    body: u8,\n}\n\n\
         pub fn announce() {\n    let message = Envelope { topic: \"orders.created\", label: \"none\", body: 1 };\n    \
         frame!(meta: { topic: \"orders.created\" });\n    frame!(topic: [\"orders.created\"]);\n    \
         frame!(frame: (\"orders.created\", \"tuple.field\"));\n    send(\"orders.created\");\n}\n\n\
         fn send(_event: &str) {}\n",
    );
    fixture.write(
        "worker/Cargo.toml",
        "[package]\nname = \"worker\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "worker/src/lib.rs",
        "struct Handler {\n    handles: &'static str,\n}\n\n\
         pub fn handler() -> Handler {\n    Handler { handles: \"orders.created\" }\n}\n",
    );
    let output = scan(&fixture);
    let json = stdout(&output);
    let err = stderr(&output);
    assert_eq!(
        err
            .lines()
            .filter(|line| line.starts_with("literal sites:") && line.contains("sites=4"))
            .count(),
        2,
        "one census line per side over the merged (pattern, language, \
         anchor) key: the three topic-keyed emit sites plus the one \
         handles-keyed subscribe site (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "async-row derived"),
        1,
        "four sites over one value collapse to ONE derived row — the \
         (from, to, event) law is the join's, untouched (stderr: {err})"
    );
    assert!(
        !err.contains("ambiguous-leg") && !err.contains("one-sided-leg"),
        "oriented joined sites name nothing extra (stderr: {err})"
    );
    assert!(
        !json.contains("tuple.field"),
        "a tuple position states no site and no candidate (json: {json})"
    );
    assert_eq!(
        json.matches("\"pattern\":\"field-bus\"").count(),
        4,
        "exactly four literal_sites rows carry the declared pattern — the \
         non-matching nouns are candidates, never sites (json: {json})"
    );
    assert!(
        json.contains("\"anchor\":\"field-value\""),
        "the rows carry the third anchor word (json: {json})"
    );
}

/// cell H — the ts collector faces: source keys, string keys and array
/// values all earn field-value sites; a call argument, an object inside a
/// non-field array, and a shorthand pair state none. Emit-only pair: every
/// matched site names `one-sided-leg` (zero silence), no join fires.
#[test]
fn ts_field_value_sites_cover_source_string_and_array_keys() {
    let fixture = Fixture::new();
    fixture.write(
        "package.json",
        "{\n  \"name\": \"demo-app\",\n  \"version\": \"1.0.0\"\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[pattern_pair]]\nid = \"field-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"ts\"\nrole = \"publish\"\nfield = \"channel\"\n",
    );
    fixture.write(
        "src/publish.ts",
        "const sourceKey = {\n  channel: \"bus.one\",\n  label: \"bus.decoy\",\n};\n\
         const stringKey = {\n  \"channel\": \"bus.two\",\n};\n\
         const grouped = {\n  channel: [\"bus.three\", \"bus.four\"],\n  list: [[\"bus.nested\"]],\n  alias,\n};\n\
         function run() {\n  send(\"bus.callarg\");\n  register([\"bus.tuple\"]);\n}\n\
         function send(_e: string) {}\nfunction register(_e: string[]) {}\n",
    );
    let output = scan(&fixture);
    let err = stderr(&output);
    assert_eq!(
        count_lines(&err, "literal sites: pattern \"field-bus\" language \"ts\" anchor field-value sites=4"),
        1,
        "source key, string key and the two array elements of a field value \
         are sites; decoys are not (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "one-sided-leg"),
        4,
        "every matched publish site names itself — zero silence, no \
         subscribe side exists (stderr: {err})"
    );
    for decoy in ["bus.callarg", "bus.tuple", "bus.nested", "bus.decoy"] {
        assert!(
            !err.contains(decoy),
            "the non-field position {decoy:?} states no row (stderr: {err})"
        );
    }
}

/// cell I — one raw value under two field conventions: the join keys per
/// convention (the value-namespace arm, read through verb 3 for the first
/// time) and names both sites. Cell F's shape with field nouns.
#[test]
fn two_field_conventions_on_one_raw_value_do_not_join_and_name_both() {
    let spec = "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"conv-a\"\n\n\
        [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\nfield = \"channel\"\n\n\
        [[pattern_pair]]\nid = \"conv-b\"\n\n\
        [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\nfield = \"label\"\n";
    let source = "struct A {\n    channel: &'static str,\n}\n\nstruct B {\n    label: &'static str,\n}\n\n\
        pub fn shapes() {\n    let a = A { channel: \"shared.value\" };\n    let b = B { label: \"shared.value\" };\n}\n";
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
        "both sites land in their own named rows (stderr: {err})"
    );
}

/// cell J — renaming the field noun moves zero verdicts: the declaration
/// and the source keys rename together, and the joined rows are the same
/// bytes (the F3 no-name-inference law read through verb 3 — a site is a
/// KEYED POSITION the declaration names, never a name the engine knows).
#[test]
fn renaming_the_declared_field_noun_moves_zero_verdicts() {
    fn tree(noun: &str) -> Fixture {
        let fixture = Fixture::new();
        fixture.write(
            "Cargo.toml",
            "[workspace]\nmembers = [\"api\", \"worker\"]\nresolver = \"2\"\n",
        );
        fixture.write(
            "architecture.spec.toml",
            &format!(
                "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"field-bus\"\n\n\
                 [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\nfield = \"{noun}\"\n\n\
                 [pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\nfield = \"handles\"\n\n\
                 [[module]]\nname = \"api\"\nmatches = {{ units = [\"api\"] }}\n\n[[module]]\nname = \"worker\"\nmatches = {{ units = [\"worker\"] }}\n"
            ),
        );
        for (name, key) in [("api", noun), ("worker", "handles")] {
            fixture.write(
                &format!("{name}/Cargo.toml"),
                &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
            );
            fixture.write(
                &format!("{name}/src/lib.rs"),
                &format!(
                    "struct Frame {{\n    {key}: &'static str,\n}}\n\npub fn frame() -> Frame {{\n    Frame {{ {key}: \"orders.created\" }}\n}}\n"
                ),
            );
        }
        fixture
    }
    let before = stdout(&scan(&tree("topic")));
    let after = stdout(&scan(&tree("xyzzy")));
    let edges = |json: &str| {
        let start = json.find("\"async_edges\":").expect("async_edges present");
        let rest = &json[start..];
        rest[..rest.find(']').expect("edges list ends")].to_string()
    };
    assert_eq!(
        edges(&before),
        edges(&after),
        "renaming the noun in the declaration AND the source moves no verdict"
    );
}
