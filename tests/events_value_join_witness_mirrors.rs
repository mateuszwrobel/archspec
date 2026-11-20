//! Scenario 07 — in-tree mirrors of the witness-corpus battery (workplan
//! `events_value_join`, decision-check F-W1/F-W2/F-W3/F-W4, plan decision
//! D-Battery). Every expectation the witness battery (an internal tool,
//! not shipped in this payload) asserts on the external witness trees is
//! replayed HERE on tiny synthetic fixtures — the weld that keeps a
//! battery failure attributable: a cell green + a leg red means corpus
//! drift, a cell red means the engine moved (a battery expectation
//! without a mirror cell is a finding; the corpus trees are never fixture
//! inputs). The fixture-mirror table lives beside the battery script.
//!
//! Names are synthetic (F10); no witness-tree, vendor or account string
//! appears in this file — the shapes are spelled generically.

mod common;

use common::{Fixture, stderr, stdout};

#[allow(dead_code)]
mod value_join {
    include!("../src/archspec/events/join.rs");
}

fn parse(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_str(&stdout(output)).expect("scan --output - is valid JSON")
}

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

fn count_prefix(haystack: &str, prefix: &str) -> usize {
    haystack
        .lines()
        .filter(|line| line.starts_with(prefix))
        .count()
}

/// The battery's joined-or-named arithmetic, SAME law as the leg's census
/// block: every matched literal value is in a joined row or a named row
/// (one-sided / ambiguous / unfunded / shared-constant / value-namespace);
/// the unaccounted set must be empty — silence fails.
fn unaccounted(doc: &serde_json::Value, err: &str) -> Vec<String> {
    let mut values: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for site in doc["literal_sites"].as_array().into_iter().flatten() {
        values.insert(site["value"].as_str().unwrap_or_default().to_string());
    }
    for row in doc["async_edges"].as_array().into_iter().flatten() {
        values.remove(row["event"].as_str().unwrap_or_default());
    }
    for line in err.lines() {
        for marker in ["one-sided-leg ", "ambiguous-leg ", "shared-constant ", "unfunded-shape "] {
            if let Some(rest) = line.strip_prefix(marker)
                && let Some(value) = rest
                    .split(" value=\"")
                    .nth(1)
                    .and_then(|tail| tail.split('"').next())
            {
                values.remove(value);
            }
        }
        if let Some(rest) = line.strip_prefix("value-namespace value=\"") {
            values.remove(rest.split('"').next().unwrap_or_default());
        }
    }
    values.into_iter().collect()
}

// ---------------------------------------------------------------------------
// mirror 1 — the byte-identity leg: no pattern pair => the new faces are inert
// ---------------------------------------------------------------------------

/// F-W1/F6 mirror: the byte-identity leg proves the real corpus stays
/// byte-identical against the base binary; inside the tree the same law
/// reads as: a tree that declares modules and `[[async_depends]]` but NO
/// `[[pattern_pair]]` speaks ZERO new-face lines and carries NO
/// `literal_sites` member — the declaration gate, not a special case (the
/// witness tree this mirrors declares 18 async_depends rows and zero
/// pattern pairs, so the slice face is inert there by construction).
#[test]
fn declaring_without_pattern_pairs_leaves_the_new_faces_silent() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"api\", \"worker\"]\nresolver = \"2\"\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"core\"\nmatches = { units = [\"api\", \"worker\"] }\n\n[[async_depends]]\nfrom = \"api\"\nto = \"worker\"\nevent = \"w07m.declared.only\"\n",
    );
    tree.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "api/src/lib.rs",
        "pub fn announce() {\n    emit(\"w07m.unmatched.value\");\n}\n\nfn emit(_event: &str) {}\n",
    );
    tree.write(
        "worker/Cargo.toml",
        "[package]\nname = \"worker\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "worker/src/lib.rs",
        "pub fn init() {\n    listen(\"w07m.declared.only\");\n}\n\nfn listen(_event: &str) {}\n",
    );
    let output = scan(&tree);
    let doc = parse(&output);
    let err = stderr(&output);
    assert!(
        doc.get("literal_sites").is_none(),
        "a non-declaring scan carries no literal_sites member: {doc}"
    );
    for prefix in [
        "literal sites:",
        "async-row derived",
        "canonical producer:",
        "dispatch-blinded-literals",
        "one-sided-leg",
        "unfunded-shape",
        "ambiguous-publish-attribution",
    ] {
        assert_eq!(
            count_prefix(&err, prefix),
            0,
            "the face `{prefix}` speaks zero times on a tree declaring no pattern pair (F6)"
        );
    }
    // the pre-slice faces are untouched: the [[async_depends]] row is a
    // SPEC face (report/render), and the scan model builds async_edges at
    // the ONE declaration-gated site — with no pattern pair declared the
    // shipped declared-only scan bytes stay untouched (D8): no member.
    assert!(
        doc.get("async_edges").is_none(),
        "no pattern pair declared, so the scan model carries no async rows (D8 byte-identity): {doc}"
    );
}

// ---------------------------------------------------------------------------
// mirror 2 — the ts-corpus leg: joined-or-named + no behavioral routing
// ---------------------------------------------------------------------------

/// The declaring spec of the ts-corpus mirror: the CALL-class pair (the
/// emitter-site class: every call whose callee simple name matched, the
/// corpus ships dozens of such sites) and the PROPERTY-class pair (the
/// exported-handler class: the literal rides an exported property value).
const OM_BUS_PAIRS: &str = "\n[[pattern_pair]]\nid = \"w07m-om-bus\"\n\n\
    [pattern_pair.emit]\nlanguage = \"ts\"\nrole = \"publish\"\ncall = \"emitEvent\"\narg = 0\n\n\
    [pattern_pair.subscribe]\nlanguage = \"ts\"\nrole = \"subscribe\"\ncall = \"attachHandler\"\narg = 0\n\n\
    [[pattern_pair]]\nid = \"w07m-om-const\"\n\n\
    [pattern_pair.emit]\nlanguage = \"ts\"\nrole = \"publish\"\ncall = \"emitEvent\"\narg = 0\n\n\
    [pattern_pair.subscribe]\nlanguage = \"ts\"\nrole = \"subscribe\"\nproperty = \"event\"\n";

/// The ts monorepo of the ts-corpus mirror: a publisher emitting three
/// values at the declared call position, a call-class subscriber and a
/// property-class subscriber handler. With `None` spec body the pattern
/// pairs are absent (the non-declaring baseline of the same bytes).
fn om_workspace(extra_spec: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"w07m-monorepo\",\n  \"version\": \"0.1.0\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "architecture.spec.toml",
        &format!("[project]\nlanguage = \"ts\"\n{extra_spec}"),
    );
    for (name, source) in [
        (
            "w07m-pub",
            "export function announce() {\n  emitEvent(\"w07m.om.arrived\");\n  emitEvent(\"w07m.om.delayed\");\n  emitEvent(\"w07m.om.solo\");\n}\nfunction emitEvent(_e: string) {}\n",
        ),
        (
            "w07m-sub",
            "export function init() {\n  attachHandler(\"w07m.om.arrived\");\n}\nfunction attachHandler(_e: string) {}\n",
        ),
        (
            "w07m-const",
            "export const handler = {\n  metadata: {\n    event: \"w07m.om.delayed\",\n  },\n  async handle() {\n    return null;\n  },\n};\n",
        ),
    ] {
        tree.write(
            &format!("packages/{name}/package.json"),
            &format!("{{\"name\":\"{name}\",\"version\":\"0.0.0\"}}\n"),
        );
        tree.write(&format!("packages/{name}/src/index.ts"), source);
    }
    tree
}

/// F-W2/A1 mirror, arm one: declaring the pairs joins the matched values
/// that have both sides, NAMES the dangling one, and leaves NO matched
/// value unaccounted — the zero-silence bar the corpus leg computes over
/// the whole emitter-site class.
#[test]
fn matched_literals_are_joined_or_named_never_silent() {
    let tree = om_workspace(OM_BUS_PAIRS);
    let output = scan(&tree);
    let doc = parse(&output);
    let err = stderr(&output);
    let mut rows = std::collections::BTreeSet::new();
    for row in doc["async_edges"].as_array().expect("async edges") {
        rows.insert((
            row["from"].as_str().unwrap_or_default().to_string(),
            row["to"].as_str().unwrap_or_default().to_string(),
            row["event"].as_str().unwrap_or_default().to_string(),
        ));
    }
    assert_eq!(
        rows,
        std::collections::BTreeSet::from([
            (
                "packages/w07m-pub".to_string(),
                "packages/w07m-sub".to_string(),
                "w07m.om.arrived".to_string()
            ),
            (
                "packages/w07m-pub".to_string(),
                "packages/w07m-const".to_string(),
                "w07m.om.delayed".to_string()
            ),
        ]),
        "the call-class pair joins via attachHandler, the property-class pair via metadata.event: {err}"
    );
    assert!(
        err.contains("one-sided-leg pattern=\"w07m-om-bus\" value=\"w07m.om.solo\""),
        "the dangling emit is NAMED one-sided, never silent (A1): {err}"
    );
    assert!(
        err.contains("value-namespace value=\"w07m.om.arrived\""),
        "both conventions match the same raw value, the split is NAMED: {err}"
    );
    assert_eq!(
        unaccounted(&doc, &err),
        Vec::<String>::new(),
        "every matched literal is joined or named (joined-or-named law)"
    );
}

/// F-W2 mirror, arm two: an emitter literal is NEVER behavioral-routed.
/// The same bytes, declaring vs not declaring: every model member OUTSIDE
/// the three new faces (`literal_sites`, `async_edges`, and nothing else
/// the scan face carries) stays byte-identical.
#[test]
fn declaring_pattern_pairs_moves_no_behavioral_model_member() {
    let tree = om_workspace(OM_BUS_PAIRS);
    let declaring = parse(&scan(&tree));
    tree.write("architecture.spec.toml", "[project]\nlanguage = \"ts\"\n");
    let plain = parse(&scan(&tree));
    let mut moved: Vec<&String> = declaring
        .as_object()
        .expect("object")
        .keys()
        .filter(|key| {
            declaring[*key] != plain.get(*key).cloned().unwrap_or(serde_json::Value::Null)
        })
        .collect();
    moved.retain(|key| *key != "literal_sites" && *key != "async_edges");
    assert!(
        moved.is_empty(),
        "only the new faces moved: {moved:?} — an emitter literal routed a behavioral member (F-W2 breach)"
    );
}

// ---------------------------------------------------------------------------
// mirror 3 — the dual-driver leg: cross-language join, shared constant, blindness
// ---------------------------------------------------------------------------

/// M-5b mirror of the Tauri-shape battery pair: ONE declared pattern with
/// a rust publish side and a ts subscribe side; the value join (welded from
/// the ONE implementation) yields exactly ONE row across the two site
/// languages — the replay the corpus leg computes over two scans.
#[test]
fn one_pattern_two_languages_join_to_one_row() {
    let sites = [
        value_join::JoinSite {
            unit: "crates/w07m-node",
            path: "crates/w07m-node/src/w07_battery.rs",
            language: "rust",
            value: "w07m.tauri.join",
            pattern: "w07m-tauri",
            anchor: "call-name+arg-position",
            noun: "battery_tauri_emit",
        },
        value_join::JoinSite {
            unit: "crates/w07m-zone",
            path: "crates/w07m-zone/w07_battery.js",
            language: "ts",
            value: "w07m.tauri.join",
            pattern: "w07m-tauri",
            anchor: "call-name+arg-position",
            noun: "listen",
        },
    ];
    let sides = [
        value_join::JoinSide {
            pattern: "w07m-tauri",
            language: "rust",
            noun: "battery_tauri_emit",
            role: Some("publish"),
        },
        value_join::JoinSide {
            pattern: "w07m-tauri",
            language: "ts",
            noun: "listen",
            role: Some("subscribe"),
        },
    ];
    let outcome = value_join::value_join(
        &sites,
        &sides,
        &std::collections::BTreeSet::new(),
        &std::collections::BTreeSet::new(),
    );
    assert_eq!(outcome.named, Vec::<String>::new(), "both sides oriented");
    assert_eq!(
        outcome.rows,
        vec![value_join::JoinedRow {
            from: "crates/w07m-node".to_string(),
            to: "crates/w07m-zone".to_string(),
            event: "w07m.tauri.join".to_string(),
            pattern: "w07m-tauri".to_string(),
        }],
        "ONE row over the shared value — language keyed nothing (M-5b)"
    );
}

/// The halves-named arm of the corpus leg, mirrored on the live scan face:
/// a one-driver scan of the emit half NAMES the value (the counterpart
/// language collects no sites in this scan — one-sided-leg or
/// unfunded-shape, both spellings name), never silently drops it. The
/// subscribe half mirrors identically with the roles swapped.
#[test]
fn a_one_driver_scan_names_the_value_of_the_missing_half() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"w07m-node\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"w07m-tauri\"\n\n\
         [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"battery_tauri_emit\"\narg = 0\n\n\
         [pattern_pair.subscribe]\nlanguage = \"ts\"\nrole = \"subscribe\"\ncall = \"listen\"\narg = 0\n",
    );
    tree.write(
        "src/lib.rs",
        "pub fn w07_announce() {\n    battery_tauri_emit(\"w07m.tauri.join\");\n}\n\nfn battery_tauri_emit(_event: &str) {}\n",
    );
    let output = scan(&tree);
    let doc = parse(&output);
    let err = stderr(&output);
    assert!(
        err.contains("value=\"w07m.tauri.join\" role publish"),
        "the publish half is NAMED with its role (one-sided or unfunded): {err}"
    );
    assert_eq!(
        unaccounted(&doc, &err),
        Vec::<String>::new(),
        "the value is accounted even where the counterpart driver collected nothing"
    );
    assert!(
        doc["async_edges"].as_array().is_none_or(Vec::is_empty),
        "a half-scan joins nothing — the join lives across the two scans, not inside one"
    );
}

/// F-W3 mirror of the shared-constant witness: a `[[shared_constant]]`
/// declaration draws exactly ONE `canonical producer:` census row (with the
/// shared-constant marker) on the report face and ZERO async rows on every
/// scan face. The corpus witness is the protocol-version constant class
/// shared by two packages; the spelling here is generic.
#[test]
fn shared_constant_declaration_draws_one_report_row_and_zero_async_rows() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"w07m-monorepo\",\n  \"version\": \"0.1.0\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"protocol\"\nmatches = { units = [\"w07m-protocol\"], modules = [\"packages/w07m-protocol/src/index.ts\"] }\n\n[[module]]\nname = \"gateway\"\nmatches = { units = [\"w07m-gateway\"], modules = [\"packages/w07m-gateway/src/index.ts\"] }\n\n[module.allowed]\ndepend_on = [\"protocol\"]\n\n[[module]]\nname = \"root\"\nmatches = { units = [\"w07m-monorepo\"] }\n\n[[shared_constant]]\nclass = \"w07m-protocol-version\"\nproducer = \"packages/w07m-protocol\"\n",
    );
    tree.write(
        "packages/w07m-protocol/package.json",
        "{\n  \"name\": \"w07m-protocol\",\n  \"version\": \"0.0.0\",\n  \"main\": \"src/index.ts\"\n}\n",
    );
    tree.write(
        "packages/w07m-protocol/src/index.ts",
        "export const PROTOCOL_VERSION = 3;\n",
    );
    tree.write(
        "packages/w07m-gateway/package.json",
        "{\n  \"name\": \"w07m-gateway\",\n  \"version\": \"0.0.0\",\n  \"main\": \"src/index.ts\"\n}\n",
    );
    tree.write(
        "packages/w07m-gateway/src/index.ts",
        "import { PROTOCOL_VERSION } from \"w07m-protocol\";\nexport const version = PROTOCOL_VERSION;\n",
    );
    let scan_out = scan(&tree);
    let doc = parse(&scan_out);
    assert!(
        doc.get("async_edges").is_none() || doc["async_edges"].as_array().is_some_and(Vec::is_empty),
        "a shared-constant class feeds a constant, never an edge: {doc}"
    );
    assert!(
        doc.get("literal_sites").is_none(),
        "no pattern pair declares, so no literal sites: {doc}"
    );
    let report = tree.run(&["report", "--output", "-"]);
    assert_eq!(
        report.status.code(),
        Some(0),
        "a declaring report stays green: {}",
        stderr(&report)
    );
    let report_text = stdout(&report);
    let rows: Vec<&str> = report_text
        .lines()
        .filter(|line| line.starts_with("canonical producer:"))
        .collect();
    assert_eq!(
        rows,
        ["canonical producer: w07m-protocol-version -> packages/w07m-protocol — shared constant"],
        "ONE census row, constant-marked (F-W3)"
    );
}

/// F-W4 mirror of the dual-marker zone arm: a nested directory carrying
/// markers for the run's language AND a declared pattern language that
/// loses dispatch answers exactly ONE `dispatch-blinded-literals` row
/// naming the zone, the blinded set, the winner and the file count — the
/// census input the corpus leg greps, NAMED not silent.
#[test]
fn dual_marker_zone_answers_the_blinded_row_named() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"node\"]\nresolver = \"2\"\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\nexcluded_sources = [ { path = \"node/bindings\", reason = \"the zone ships its ts side separately\" } ]\n\n[[pattern_pair]]\nid = \"w07m-tauri\"\n\n\
         [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"battery_tauri_emit\"\narg = 0\n\n\
         [pattern_pair.subscribe]\nlanguage = \"ts\"\nrole = \"subscribe\"\ncall = \"listen\"\narg = 0\n",
    );
    tree.write(
        "node/Cargo.toml",
        "[package]\nname = \"node\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "node/src/lib.rs",
        "pub fn w07_announce() {\n    battery_tauri_emit(\"w07m.tauri.zone\");\n}\n\nfn battery_tauri_emit(_event: &str) {}\n",
    );
    // the zone: ONE directory carrying root markers for BOTH languages —
    // that is the dispatch row's trigger. The spec exclusion keeps the
    // ts sources out of the agreement census; marker_report reads markers
    // behind the mechanical set only, so the zone row stands.
    tree.write(
        "node/bindings/Cargo.toml",
        "[package]\nname = \"w07m-bindings\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "node/bindings/package.json",
        "{\"name\":\"w07m-bindings\",\"version\":\"0.0.0\"}\n",
    );
    tree.write(
        "node/bindings/index.js",
        "export function w07_init() {\n    listen(\"w07m.tauri.zone\");\n}\n\nfunction listen(_event) {}\n",
    );
    let output = scan(&tree);
    let err = stderr(&output);
    let rows: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("dispatch-blinded-literals"))
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "exactly ONE projection row for the one dual-marker zone: {err}"
    );
    assert!(
        rows[0].starts_with("dispatch-blinded-literals zone=node/bindings")
            && rows[0].contains("langs=\"ts\"")
            && rows[0].contains("claimed=rust"),
        "the row names zone, blinded set and winner: {}",
        rows[0]
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the projection is report-only, never a verdict"
    );
}

// ---------------------------------------------------------------------------
// scenario 09 mirrors — the funded go/python/kotlin legs (battery legs 4-8)
// ---------------------------------------------------------------------------

/// Mirror of battery legs 4 and 7 (the go feed leg and the go corpus-join
/// leg): the go call-anchor shape — a
/// package-qualified call whose argument at position 1 carries the value —
/// with the ts exported-property counterpart replayed by the welded join.
/// The scan side states its counts with ZERO `driver-gap:` rows; the
/// replay side yields exactly ONE row over the shared value.
#[test]
fn funded_go_sites_mirror_the_corpus_leg_counts_and_one_replayed_row() {
    let tree = Fixture::new();
    tree.write("go.mod", "module demo/docs-tool\n\ngo 1.22\n");
    tree.write(
        "translator.go",
        "package main\n\nimport \"path/filepath\"\n\nfunc cachePath(home string) string {\n\treturn filepath.Join(home, \"session-cache.json\")\n}\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[pattern_pair]]\nid = \"w09m-file\"\n\n\
         [pattern_pair.emit]\nlanguage = \"go\"\nrole = \"publish\"\ncall = \"Join\"\narg = 1\n\n\
         [pattern_pair.subscribe]\nlanguage = \"ts\"\nrole = \"subscribe\"\nproperty = \"cacheFileName\"\n",
    );
    let output = scan(&tree);
    let err = stderr(&output);
    let census: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .collect();
    assert_eq!(
        census,
        vec![
            "literal sites: pattern \"w09m-file\" language \"go\" anchor call-name+arg-position sites=1",
            "literal sites: pattern \"w09m-file\" language \"ts\" anchor exported-property-value sites=0",
        ],
        "the go side counts, the absent-language side states its zero (the named half):\n{err}"
    );
    assert_eq!(
        count_prefix(&err, "driver-gap:"),
        0,
        "go is funded — no gap row anywhere on this scan:\n{err}"
    );
    let doc = parse(&output);
    assert!(
        unaccounted(&doc, &err).is_empty(),
        "joined-or-named over the go sites failed"
    );
    // the leg's replay arithmetic, welded to the ONE implementation: the
    // two site sets sharing one value join to exactly one row.
    let sites = [
        value_join::JoinSite {
            unit: "go-rooted:.",
            path: "translator.go",
            language: "go",
            value: "session-cache.json",
            pattern: "w09m-file",
            anchor: "call-name+arg-position",
            noun: "Join",
        },
        value_join::JoinSite {
            unit: "ts-rooted:.",
            path: "src/cache-key.ts",
            language: "ts",
            value: "session-cache.json",
            pattern: "w09m-file",
            anchor: "exported-property-value",
            noun: "cacheFileName",
        },
    ];
    let sides = [
        value_join::JoinSide {
            pattern: "w09m-file",
            language: "go",
            noun: "Join",
            role: Some("publish"),
        },
        value_join::JoinSide {
            pattern: "w09m-file",
            language: "ts",
            noun: "cacheFileName",
            role: Some("subscribe"),
        },
    ];
    let outcome = value_join::value_join(
        &sites,
        &sides,
        &std::collections::BTreeSet::new(),
        &std::collections::BTreeSet::new(),
    );
    assert_eq!(outcome.rows.len(), 1, "the shared value replays to ONE row");
    assert!(outcome.named.is_empty(), "both halves present: {:?}", outcome.named);
}

/// Mirror of battery leg 5 (the python feed leg): the python shape — a module
/// that CALLS with a list element and binds a LIST of strings — states
/// `sites=0` on both declared sides (the unmatched-shape reading), zero
/// `driver-gap:` rows, and a module-level string binding WOULD count (the
/// candidate arm is present, the list is simply not a binding).
#[test]
fn funded_python_sites_mirror_the_corpus_leg_stated_zeros() {
    let tree = Fixture::new();
    tree.write("pyproject.toml", "[project]\nname = \"demo-skills\"\nversion = \"0.1.0\"\n");
    tree.write(
        "skill/run.py",
        "TOPICS = [\"dispatch.topic\"]\n\n\ndef run(registry):\n    registry.dispatch(TOPICS[0])\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"python\"\n\n[[pattern_pair]]\nid = \"w09m-py\"\n\n\
         [pattern_pair.emit]\nlanguage = \"python\"\nrole = \"publish\"\ncall = \"dispatch\"\narg = 0\n\n\
         [pattern_pair.subscribe]\nlanguage = \"python\"\nrole = \"subscribe\"\nproperty = \"TOPICS\"\n",
    );
    let output = scan(&tree);
    let err = stderr(&output);
    let census: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .collect();
    assert_eq!(
        census,
        vec![
            "literal sites: pattern \"w09m-py\" language \"python\" anchor call-name+arg-position sites=0",
            "literal sites: pattern \"w09m-py\" language \"python\" anchor exported-property-value sites=0",
        ],
        "the list element and the list binding state their zeros — counts, never gap rows:\n{err}"
    );
    assert_eq!(
        count_prefix(&err, "driver-gap:"),
        0,
        "python is funded — no gap row anywhere on this scan:\n{err}"
    );
}

/// Mirror of battery leg 6 (the kotlin property leg): the gradle-shaped kotlin tree
/// with a top-level `const val` cures the false zero — the property
/// anchor states `sites=1` (the corpus leg greps a real name at run time;
/// here the shape is spelled synthetically).
#[test]
fn funded_kotlin_property_mirror_states_sites_one_on_the_gradle_shape() {
    let tree = Fixture::new();
    tree.write("settings.gradle.kts", "include(\":app\")\n");
    tree.write("app/build.gradle.kts", "plugins { id(\"com.android.application\") }\n");
    tree.write(
        "app/src/main/kotlin/demo/LaunchMode.kt",
        "package demo\n\nconst val extraKey = \"demo.extra.value\"\n\nfun launch(intent: String) {}\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"kotlin\"\n\n[[pattern_pair]]\nid = \"w09m-kt\"\n\n\
         [pattern_pair.subscribe]\nlanguage = \"kotlin\"\nrole = \"subscribe\"\nproperty = \"extraKey\"\n",
    );
    let output = scan(&tree);
    let err = stderr(&output);
    let census: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .collect();
    assert_eq!(
        census,
        vec![
            "literal sites: pattern \"w09m-kt\" language \"kotlin\" anchor exported-property-value sites=1",
        ],
        "the top-level const is ONE candidate on the gradle shape — the false zero is cured:\n{err}"
    );
    assert_eq!(
        count_prefix(&err, "driver-gap:"),
        0,
        "kotlin was funded already — the property arm cures its false zero, not a gap row:\n{err}"
    );
}
