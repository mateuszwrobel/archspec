//! Scenario 09 — driver contributions: go, python and the kotlin property
//! arm feed the literal-site census and the value join (workplan
//! `events_value_join`, the driver-gap cure; ADR-069 4.8's flip, decisions
//! A1/A6 extended). The done-when cells:
//!
//! - (cell A) a go fixture with declared sides prints `literal sites:`
//!   count lines — including a stated `sites=0` for a side nothing calls —
//!   and ZERO `driver-gap:` lines;
//! - (cell B) the python fixture same, and a python LIST-element literal
//!   under a declared property side prints `sites=0` (the unmatched-shape
//!   reading: stated by the count, never by a gap row);
//! - (cell C) the kotlin false-zero cure: a top-level `const val` is one
//!   property candidate, a `private const val` is none; renaming the
//!   binding AND the declaration noun moves zero verdicts (the F3 law
//!   restated on the new arm);
//! - (cell D) cross-language value join with a GO side (welded from the ONE
//!   implementation — the `include!` shape of `events_value_join_rows.rs`
//!   cell E): one shared value joins to exactly one row; a value spelled
//!   differently on one side joins to zero rows and names each site once;
//! - (cell E) zero silence over the new sites: a go fixture whose
//!   subscriber never fires prints no async row and ONE named row, verdict
//!   not red;
//! - (cell F) the capability weld: `capability matrix` reads granular for
//!   go and python, csharp keeps the type-identity covering source, and no
//!   shipped driver's `literal-site-census` row reads `not-emitted` — the
//!   sweep that keeps the `driver-gap:` row a future-language face.
//!
//! Names are synthetic; no corpus, vendor or account string appears in
//! this file (F10).

mod common;

use common::{Fixture, stderr, stdout};

#[allow(dead_code)]
mod value_join {
    include!("../src/archspec/events/join.rs");
}

fn scan(fixture: &Fixture) -> std::process::Output {
    fixture.run(&["scan", "--output", "-"])
}

fn count_lines(haystack: &str, prefix: &str) -> usize {
    haystack
        .lines()
        .filter(|line| line.starts_with(prefix))
        .count()
}

// ---------------------------------------------------------------------------
// cell A — go contributes sites, zero driver-gap rows
// ---------------------------------------------------------------------------

/// A go tree with one emit call whose first argument is the event string,
/// one call whose second argument carries a plain literal (the spelled
/// position law), and a declared subscribe side nothing calls: the census
/// states `sites=1` and a stated `sites=0`, the model carries the emit
/// row, and no `driver-gap:` line exists anywhere.
#[test]
fn go_sides_print_sites_counts_including_a_stated_zero() {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module demo/shop\n\ngo 1.22\n");
    fixture.write(
        "main.go",
        "package main\n\nfunc main() {\n\tpublish(\"orders.created\")\n\tarchive(store, \"orders.created\")\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[pattern_pair]]\nid = \"demo-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"go\"\nrole = \"publish\"\ncall = \"publish\"\narg = 0\n\n\
         [[pattern_pair]]\nid = \"demo-listen\"\n\n\
         [pattern_pair.subscribe]\nlanguage = \"go\"\nrole = \"subscribe\"\ncall = \"nothingListens\"\narg = 0\n",
    );
    let output = scan(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&output)
    );
    let err = stderr(&output);
    let census: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .collect();
    assert_eq!(
        census,
        vec![
            "literal sites: pattern \"demo-bus\" language \"go\" anchor call-name+arg-position sites=1",
            "literal sites: pattern \"demo-listen\" language \"go\" anchor call-name+arg-position sites=0",
        ],
        "one count line per declared side in declaration order, the unmatched \
         side stating its ZERO (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "driver-gap:"),
        0,
        "go is funded — a driver-gap row is unreachable here (stderr: {err})"
    );
    let model = stdout(&output);
    assert!(
        model.contains(
            "\"path\":\"main.go\",\"language\":\"go\",\"value\":\"orders.created\",\"anchor\":\"call-name+arg-position\",\"pattern\":\"demo-bus\""
        ),
        "the go emit call rides the model with the F8 seam shape:\n{model}"
    );
}

/// The go property arm: package-level `const`/`var` string values are
/// `exported-property-value` candidates (the corpus had zero, so the
/// fixture carries them); a function-local declaration is not package
/// surface and an `iota` value states no literal.
#[test]
fn go_package_level_string_bindings_are_property_candidates() {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module demo/shop\n\ngo 1.22\n");
    fixture.write(
        "conf/conf.go",
        "package conf\n\nconst Filename = \"auth.json\"\n\nvar Mode = \"strict\"\n\nconst Serial = 42\n\nfunc Local() {\n\tconst inner = \"not-package-surface\"\n\tuse(inner)\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[pattern_pair]]\nid = \"demo-file\"\n\n\
         [pattern_pair.subscribe]\nlanguage = \"go\"\nrole = \"subscribe\"\nproperty = \"Filename\"\n\n\
         [[pattern_pair]]\nid = \"demo-serial\"\n\n\
         [pattern_pair.subscribe]\nlanguage = \"go\"\nrole = \"subscribe\"\nproperty = \"Serial\"\n",
    );
    let output = scan(&fixture);
    let err = stderr(&output);
    let census: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .collect();
    assert_eq!(
        census,
        vec![
            "literal sites: pattern \"demo-file\" language \"go\" anchor exported-property-value sites=1",
            "literal sites: pattern \"demo-serial\" language \"go\" anchor exported-property-value sites=0",
        ],
        "the const string joins the census; the int states a stated ZERO \
         (stderr: {err})"
    );
    assert!(
        stdout(&output).contains(
            "\"path\":\"conf/conf.go\",\"language\":\"go\",\"value\":\"auth.json\",\"anchor\":\"exported-property-value\",\"pattern\":\"demo-file\""
        ),
        "the package-level const rides the model:\n{}",
        stdout(&output)
    );
}

// ---------------------------------------------------------------------------
// cell B — python contributes sites; a LIST element is a stated zero
// ---------------------------------------------------------------------------

/// A python tree with a module-level call and a module-level string
/// binding, plus the corpus-shape list whose element carries a value no
/// funded verb names: the sides print their counts, the list-element side
/// states `sites=0`, and the scan is green with zero gap rows.
#[test]
fn python_sides_print_sites_counts_and_the_list_element_states_zero() {
    let fixture = Fixture::new();
    fixture.write("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    fixture.write(
        "pkg/bus.py",
        "MODE = \"strict\"\n\nTOPICS = [\"command-dispatch\", \"orders.created\"]\n\n\
         def run(registry):\n    publish(\"orders.created\")\n    registry.dispatch(TOPICS[0])\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"python\"\n\n[[pattern_pair]]\nid = \"demo-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"python\"\nrole = \"publish\"\ncall = \"publish\"\narg = 0\n\n\
         [[pattern_pair]]\nid = \"demo-topics\"\n\n\
         [pattern_pair.subscribe]\nlanguage = \"python\"\nrole = \"subscribe\"\nproperty = \"TOPICS\"\n\n\
         [[pattern_pair]]\nid = \"demo-mode\"\n\n\
         [pattern_pair.subscribe]\nlanguage = \"python\"\nrole = \"subscribe\"\nproperty = \"MODE\"\n",
    );
    let output = scan(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&output)
    );
    let err = stderr(&output);
    let census: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .collect();
    assert_eq!(
        census,
        vec![
            "literal sites: pattern \"demo-bus\" language \"python\" anchor call-name+arg-position sites=1",
            "literal sites: pattern \"demo-topics\" language \"python\" anchor exported-property-value sites=0",
            "literal sites: pattern \"demo-mode\" language \"python\" anchor exported-property-value sites=1",
        ],
        "the call site counts, the LIST element states its zero (the \
         unmatched-shape reading — never a gap row), the module-level \
         string binding counts (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "driver-gap:"),
        0,
        "python is funded — a driver-gap row is unreachable here (stderr: {err})"
    );
    let model = stdout(&output);
    assert!(
        model.contains(
            "\"path\":\"pkg/bus.py\",\"language\":\"python\",\"value\":\"strict\",\"anchor\":\"exported-property-value\",\"pattern\":\"demo-mode\""
        ),
        "the module-level binding rides the model:\n{model}"
    );
    assert!(
        !model.contains("\"value\":\"command-dispatch\""),
        "a list element never enters the site collection — the zero is the \
         statement:\n{model}"
    );
}

// ---------------------------------------------------------------------------
// cell C — the kotlin property arm cures the false zero
// ---------------------------------------------------------------------------

/// The kotlin tree declares one top-level `const val EVENT` and one
/// `private const val SECRET`: the declared property side on EVENT prints
/// `sites=1` (the false-zero cure), the side on SECRET prints `sites=0`
/// (visibility is grammar-decided, the ts-`export` posture). Renaming the
/// binding AND the declaration noun (identical semantics, identical
/// positions) moves ZERO verdicts — the F3 law restated on the new arm.
#[test]
fn kotlin_top_level_property_cures_the_false_zero() {
    let tree = kotlin_property_tree("EVENT");
    let output = scan(&tree);
    assert_eq!(
        output.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&output)
    );
    let err = stderr(&output);
    let census: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .collect();
    assert_eq!(
        census,
        vec![
            "literal sites: pattern \"demo-event\" language \"kotlin\" anchor exported-property-value sites=1",
            "literal sites: pattern \"demo-secret\" language \"kotlin\" anchor exported-property-value sites=0",
        ],
        "the top-level const is ONE candidate; the private const is none \
         (stderr: {err})"
    );
    assert!(
        stdout(&output).contains(
            "\"path\":\"App.kt\",\"language\":\"kotlin\",\"value\":\"orders.created\",\"anchor\":\"exported-property-value\",\"pattern\":\"demo-event\""
        ),
        "the top-level const val rides the model:\n{}",
        stdout(&output)
    );
}

#[test]
fn renaming_the_kotlin_property_moves_zero_verdicts() {
    let declared = kotlin_property_tree("EVENT");
    let renamed = kotlin_property_tree("renamedBinding");
    let first = scan(&declared);
    let second = scan(&renamed);
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "renaming the binding AND its declaration moved model bytes — a \
         name-inference arm would be the only way (F3 on the property arm)"
    );
    let census: Vec<String> = stderr(&first)
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .map(str::to_string)
        .collect();
    let census_renamed: Vec<String> = stderr(&second)
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .map(str::to_string)
        .collect();
    assert_eq!(census, census_renamed, "the census moved with the rename — F3 breach");
}

/// One kotlin root (no settings file: the implicit root unit) whose top
/// level declares `const val <name> = "orders.created"` plus a
/// `private const val SECRET`. The property side names the binding by
/// EQUALITY, so the rename cell spells the same tree under two nouns.
fn kotlin_property_tree(name: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "App.kt",
        &format!(
            "package demo\n\nconst val {name} = \"orders.created\"\nprivate const val SECRET = \"hidden\"\n\nfun main() {{\n    publish({name})\n}}\n"
        ),
    );
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"kotlin\"\n\n[[pattern_pair]]\nid = \"demo-event\"\n\n\
             [pattern_pair.subscribe]\nlanguage = \"kotlin\"\nrole = \"subscribe\"\nproperty = \"{name}\"\n\n\
             [[pattern_pair]]\nid = \"demo-secret\"\n\n\
             [pattern_pair.subscribe]\nlanguage = \"kotlin\"\nrole = \"subscribe\"\nproperty = \"SECRET\"\n"
        ),
    );
    fixture
}

// ---------------------------------------------------------------------------
// cell D — cross-language value join with a go side (welded implementation)
// ---------------------------------------------------------------------------

/// The single-scan substrate answers one driver per root (the cell-E law
/// of `events_value_join_rows.rs`), so the cross-language face is welded
/// from `src/archspec/events/join.rs` itself — the ONE implementation. One
/// go publish site and one ts exported-property site over ONE value join
/// to exactly ONE row whose endpoints are two units of two languages.
#[test]
fn go_publish_site_and_ts_property_site_join_to_one_row() {
    let sites = [
        value_join::JoinSite {
            unit: "scripts/docs-tool",
            path: "scripts/docs-tool/translator.go",
            language: "go",
            value: "auth.json",
            pattern: "auth-file",
            anchor: "call-name+arg-position",
            noun: "Join",
        },
        value_join::JoinSite {
            unit: "ext/codex",
            path: "ext/codex/src/auth-cache-key.ts",
            language: "ts",
            value: "auth.json",
            pattern: "auth-file",
            anchor: "exported-property-value",
            noun: "authFileName",
        },
    ];
    let sides = [
        value_join::JoinSide {
            pattern: "auth-file",
            language: "go",
            noun: "Join",
            role: Some("publish"),
        },
        value_join::JoinSide {
            pattern: "auth-file",
            language: "ts",
            noun: "authFileName",
            role: Some("subscribe"),
        },
    ];
    let outcome = value_join::value_join(
        &sites,
        &sides,
        &std::collections::BTreeSet::new(),
        &std::collections::BTreeSet::new(),
    );
    assert_eq!(outcome.named, Vec::<String>::new(), "both halves are present");
    assert_eq!(
        outcome.rows,
        vec![value_join::JoinedRow {
            from: "scripts/docs-tool".to_string(),
            to: "ext/codex".to_string(),
            event: "auth.json".to_string(),
            pattern: "auth-file".to_string(),
        }],
        "ONE row over the shared value — the go side participated by value"
    );
}

/// The same pair with the value spelled differently on one side: zero
/// rows and exactly one `one-sided-leg` row per site (the zero-silence
/// bar — a missed join names both halves, never silence).
#[test]
fn mismatched_values_join_zero_rows_and_name_each_site_once() {
    let sites = [
        value_join::JoinSite {
            unit: "scripts/docs-tool",
            path: "scripts/docs-tool/translator.go",
            language: "go",
            value: "auth.json",
            pattern: "auth-file",
            anchor: "call-name+arg-position",
            noun: "Join",
        },
        value_join::JoinSite {
            unit: "ext/codex",
            path: "ext/codex/src/auth-cache-key.ts",
            language: "ts",
            value: "auth.json.v2",
            pattern: "auth-file",
            anchor: "exported-property-value",
            noun: "authFileName",
        },
    ];
    let sides = [
        value_join::JoinSide {
            pattern: "auth-file",
            language: "go",
            noun: "Join",
            role: Some("publish"),
        },
        value_join::JoinSide {
            pattern: "auth-file",
            language: "ts",
            noun: "authFileName",
            role: Some("subscribe"),
        },
    ];
    let outcome = value_join::value_join(
        &sites,
        &sides,
        &std::collections::BTreeSet::new(),
        &std::collections::BTreeSet::new(),
    );
    assert!(outcome.rows.is_empty(), "no shared value — no row");
    assert_eq!(outcome.named.len(), 2, "both sites name themselves once");
    assert!(
        outcome
            .named
            .iter()
            .any(|line| line.starts_with("one-sided-leg pattern=\"auth-file\" value=\"auth.json\" role publish")),
        "the go half states its missing counterpart: {:?}",
        outcome.named
    );
    assert!(
        outcome.named.iter().any(|line| {
            line.starts_with("one-sided-leg pattern=\"auth-file\" value=\"auth.json.v2\" role subscribe")
        }),
        "the ts half states its missing counterpart: {:?}",
        outcome.named
    );
}

// ---------------------------------------------------------------------------
// cell E — zero silence over the new sites (single scan, go sides)
// ---------------------------------------------------------------------------

/// A go tree whose publish site fires and whose subscriber never does:
/// zero async rows, exactly ONE `one-sided-leg` named row, exit 0 — the
/// A1 bar over a site that only exists because scenario 09 funded the go
/// feed.
#[test]
fn go_dangling_subscriber_joins_zero_and_names_one_row() {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module demo/shop\n\ngo 1.22\n");
    fixture.write(
        "main.go",
        "package main\n\nfunc main() {\n\tpublish(\"orders.created\")\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[pattern_pair]]\nid = \"demo-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"go\"\nrole = \"publish\"\ncall = \"publish\"\narg = 0\n\n\
         [pattern_pair.subscribe]\nlanguage = \"go\"\nrole = \"subscribe\"\ncall = \"neverCalled\"\narg = 0\n",
    );
    let output = scan(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a dangling subscriber is stated, never red (stderr: {})",
        stderr(&output)
    );
    let json = stdout(&output);
    assert!(
        !json.contains("\"async_edges\":[{"),
        "joined=0 — the dangling half joins nothing:\n{json}"
    );
    let err = stderr(&output);
    assert_eq!(
        count_lines(&err, "one-sided-leg "),
        1,
        "exactly ONE named row carries the unmatched value (stderr: {err})"
    );
    assert_eq!(
        count_lines(&err, "unfunded-shape "),
        0,
        "go is funded — the counterpart is MISSING, not unfunded (stderr: {err})"
    );
}

// ---------------------------------------------------------------------------
// cell F — the capability weld
// ---------------------------------------------------------------------------

/// `capability matrix` is the machine surface of the flip: the go and
/// python `literal-site-census` rows read `granular`, csharp keeps its
/// type-identity covering source, and the sweep proves NO shipped driver
/// states `not-emitted` for this fact — the `driver-gap:` row keeps its
/// meaning as the face of a FUTURE driver, never a live one.
#[test]
fn capability_matrix_reads_funded_for_go_and_python() {
    let fixture = Fixture::new();
    let output = fixture.run(&["capability", "matrix"]);
    assert_eq!(output.status.code(), Some(0), "matrix must print");
    let matrix = stdout(&output);
    let rows: Vec<&str> = matrix
        .lines()
        .filter(|line| line.ends_with(" literal-site-census") || line.contains(" literal-site-census "))
        .collect();
    assert!(
        rows.contains(&"capability go literal-site-census granular"),
        "the go row must read granular:\n{rows:#?}"
    );
    assert!(
        rows.contains(&"capability python literal-site-census granular"),
        "the python row must read granular:\n{rows:#?}"
    );
    assert!(
        rows.contains(&"capability csharp literal-site-census via type-identity facts (no literal census)"),
        "csharp keeps the type-identity covering source:\n{rows:#?}"
    );
    assert!(
        !rows.iter().any(|row| row.ends_with("not-emitted")),
        "no shipped driver's literal-site-census row may read not-emitted \
         after the driver-gap cure:\n{rows:#?}"
    );
    for language in ["rust", "go", "ts", "python", "kotlin"] {
        let granular = fixture.run(&["capability", "granular", language, "literal-site-census"]);
        assert_eq!(
            granular.status.code(),
            Some(0),
            "{language} must answer granular on the funded fact"
        );
    }
}
