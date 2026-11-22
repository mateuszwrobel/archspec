//! Workplan level_views scenario 03: the declared-vs-projected unit delta
//! census on the `diagram --source scan --level unit` render surface.
//!
//! The projected unit graph (module-edge rows bucketed by CONTAINMENT onto
//! the model's units) and the model's declared unit edges are TWO graphs.
//! The `--level unit` render names their delta LOUD on STDERR: one
//! `unit delta model-only: <n> — <pairs>` row (pairs the model declares that
//! the fold draws nowhere) and one `unit delta projection-only: <n> — <pairs>`
//! row (pairs the fold draws that the model does not declare), `none` when a
//! class is empty. The rows are ADVISORY (decision-check MAJOR-5 fix,
//! MAJOR-13 posture): they never enter the render body, never enter the
//! model, and never change rc — a healthy rc=0 corpus stays rc=0 with the
//! delta named. The exit-1 gap law belongs to SCAN only and stays there
//! untouched (the pre-existing scan census cells stay green unchanged).
//! The projection composes with scenario 01's accounting ledger: the
//! `accounting exact: <d> drawn + <s> self = <r> raw` row stays exact at the
//! unit level — the census adds no ledger term. A pair is spelled
//! `from→to`; the graph-missing pairs never ship silently: the same scan at
//! `--tiers units` draws the declared graph, `--level unit` draws the
//! projected one, and the census names exactly the pairs that differ.

mod common;

use common::{Fixture, stderr, stdout};

/// The delta fixture: the declared unit edges and the projected unit pairs
/// differ by planted pairs in BOTH classes.
///
/// - `alpha→beta` and `beta→alpha` agree across the graphs (module rows
///   project onto the declared pairs — they appear in neither delta class);
/// - `beta→gamma` is declared but carries no module row — model-only;
/// - `workspace→beta` is projected from a root-unit file (`tools/x.rs`,
///   owned by no member, so the root unit claims it — the workspace-root
///   unit → member shape) but is not declared — projection-only;
/// - one intra-unit row folds onto the `beta` SELF bucket, so the ledger
///   composes: `3 drawn + 1 self = 4 raw` at level unit.
fn delta_artefact(edges: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        &format!(
            r#"{{"schema_version":1,"language":"rust","units":[{{"name":"workspace","kind":"lib","path":"."}},{{"name":"alpha","kind":"lib","path":"src/alpha"}},{{"name":"beta","kind":"lib","path":"src/beta"}},{{"name":"gamma","kind":"lib","path":"src/gamma"}}],"edges":{edges},"module_edges":[{{"unit":"alpha","from":"alpha::x","to":"beta::store","symbols":[]}},{{"unit":"beta","from":"beta::q","to":"alpha::y","symbols":[]}},{{"unit":"beta","from":"beta::m","to":"beta::n","symbols":[]}},{{"unit":"workspace","from":"tools/x.rs","to":"beta::store","symbols":[]}}]}}"#
        ),
    );
    fixture
}

/// The delta fixture with the declared graph planted from the projection's
/// own pairs: an empty delta — the census-off twin whose body bytes must
/// equal the delta-carrying twin's byte for byte.
fn delta_free_artefact() -> Fixture {
    delta_artefact(
        r#"[{"from":"alpha","to":"beta"},{"from":"beta","to":"alpha"},{"from":"workspace","to":"beta"}]"#,
    )
}

fn run(
    fixture: &Fixture,
    flag: &str,
    value: &str,
    format: &str,
) -> (String, String, Option<i32>) {
    let output = fixture.run(&[
        "diagram",
        "--source",
        "scan",
        "model.json",
        flag,
        value,
        "--format",
        format,
    ]);
    (
        stdout(&output),
        stderr(&output),
        output.status.code(),
    )
}

fn run_level(fixture: &Fixture, format: &str) -> (String, String) {
    let (body, err, code) = run(fixture, "--level", "unit", format);
    assert_eq!(
        code,
        Some(0),
        "the unit-level render must exit 0: {err}"
    );
    (body, err)
}

/// The ledger row's numbers: `accounting exact: <d> drawn + <s> self = <r> raw ...`.
fn ledger(stderr: &str) -> (usize, usize, usize) {
    let row = stderr
        .lines()
        .find(|line| line.starts_with("accounting exact: "))
        .unwrap_or_else(|| panic!("the ledger row is missing from:\n{stderr}"));
    let middle = row
        .trim_start_matches("accounting exact: ")
        .split(" raw ")
        .next()
        .expect("the raw clause");
    let mut numbers = middle
        .split([' ', '+', '='])
        .filter_map(|token| token.parse::<usize>().ok());
    let (drawn, self_rows, raw) = (
        numbers.next().expect("drawn"),
        numbers.next().expect("self"),
        numbers.next().expect("raw"),
    );
    assert_eq!(
        drawn + self_rows,
        raw,
        "the ledger must stay exact — drawn + self = raw: {row}"
    );
    (drawn, self_rows, raw)
}

/// Cell 1 — delta census names BOTH classes with exact counts, and the two
/// graphs never ship silently: the same scan at `--tiers units` draws the
/// declared graph (model-only pair visible, no census row), `--level unit`
/// draws the projected graph (projection-only pair visible) and the census
/// names exactly the pairs that differ. The ledger composes unchanged.
#[test]
fn delta_census_names_both_classes_and_the_two_graphs_never_ship_silently() {
    let fixture = delta_artefact(
        r#"[{"from":"alpha","to":"beta"},{"from":"beta","to":"alpha"},{"from":"beta","to":"gamma"}]"#,
    );
    let (body, err) = run_level(&fixture, "dot");
    assert!(
        err.contains("unit delta model-only: 1 — beta→gamma"),
        "the declared pair the fold draws nowhere is named model-only with its count:\n{err}"
    );
    assert!(
        err.contains("unit delta projection-only: 1 — workspace→beta"),
        "the projected pair the model does not declare is named projection-only with its count:\n{err}"
    );
    assert!(
        err.contains("self rows: beta ×1"),
        "the intra-unit row stays self-accounted by the scenario-01 tally:\n{err}"
    );
    assert!(
        err.contains("accounting exact: 3 drawn + 1 self = 4 raw rows at level unit"),
        "the census composes with the exact ledger — drawn + self = raw at unit level:\n{err}"
    );
    ledger(&err);
    assert!(
        body.contains("\"workspace\" -> \"beta\""),
        "the projected graph draws the projection-only pair at level unit:\n{body}"
    );
    assert!(
        !body.contains("unit delta"),
        "the census rows never enter the render body:\n{body}"
    );

    // The same scan at `--tiers units`: the shipped tier surface draws the
    // DECLARED graph — the model-only pair is visible there — and prints no
    // census row, because the census is owned by the level surface.
    let (tiers_body, tiers_err, code) = run(&fixture, "--tiers", "units", "dot");
    assert_eq!(code, Some(0), "the tiers render stays rc=0: {tiers_err}");
    assert!(
        tiers_body.contains("\"beta\" -> \"gamma\""),
        "the declared graph draws the model-only pair at the units tier:\n{tiers_body}"
    );
    assert!(
        !tiers_err.contains("unit delta"),
        "the census rides the level surface only — the tiers render prints no delta row:\n{tiers_err}"
    );
}

/// Cell 2 — rc unchanged and body census-invariant: the delta-carrying
/// fixture renders rc=0, and its body bytes equal byte for byte the body of
/// the census-off twin — the same projection whose declared graph is planted
/// from the projected pairs so the delta is empty (`none` on both rows).
/// The delta moves STDERR only.
#[test]
fn rc_stays_zero_and_the_body_is_census_invariant() {
    let with_delta = delta_artefact(
        r#"[{"from":"alpha","to":"beta"},{"from":"beta","to":"alpha"},{"from":"beta","to":"gamma"}]"#,
    );
    let without_delta = delta_free_artefact();
    for format in ["mermaid", "dot"] {
        let (body_a, err_a) = run_level(&with_delta, format);
        let (body_b, err_b) = run_level(&without_delta, format);
        assert!(
            err_a.contains("unit delta model-only: 1 — beta→gamma"),
            "the delta-carrying render names the model-only class at {format}:\n{err_a}"
        );
        assert!(
            err_b.contains("unit delta model-only: none"),
            "the delta-free twin states the class as none at {format}:\n{err_b}"
        );
        assert!(
            err_b.contains("unit delta projection-only: none"),
            "the delta-free twin states both classes as none at {format}:\n{err_b}"
        );
        assert_eq!(
            body_a, body_b,
            "the census-off and census-on bodies are byte-identical at {format} — the delta rides STDERR only"
        );
        for body in [&body_a, &body_b] {
            assert!(
                !body.contains("unit delta"),
                "the census rows never enter the {format} render body:\n{body}"
            );
        }
    }
}

/// Cell 3 — healthy-corpus posture: an rc=0 scan artefact carrying a
/// multi-package delta in both classes answers rc=0 with both delta rows
/// named and exact counts (the posture citation: rc=0 stays rc=0 with the
/// delta printed). No gap census is present, so no restatement row rides;
/// the ledger stays exact. The projection-only classes carry the shipped
/// bucket spellings: root-unit files claim the `workspace` bucket, a
/// tree-root file the one `root` class.
#[test]
fn healthy_artefact_stays_rc_zero_with_the_delta_named() {
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"workspace","kind":"lib","path":"."},{"name":"api","kind":"lib","path":"pkg/api"},{"name":"core","kind":"lib","path":"pkg/core"},{"name":"store","kind":"lib","path":"pkg/store"},{"name":"ui","kind":"lib","path":"app/ui"},{"name":"cli","kind":"lib","path":"app/cli"}],"edges":[{"from":"api","to":"core"},{"from":"core","to":"store"},{"from":"ui","to":"core"},{"from":"cli","to":"store"},{"from":"api","to":"store"},{"from":"cli","to":"core"}],"module_edges":[{"unit":"api","from":"api::a","to":"core::b","symbols":[]},{"unit":"core","from":"core::c","to":"store::d","symbols":[]},{"unit":"ui","from":"ui::e","to":"core::f","symbols":[]},{"unit":"cli","from":"cli::g","to":"store::h","symbols":[]},{"unit":"workspace","from":"scripts/gen.rs","to":"ui::e","symbols":[]},{"unit":"workspace","from":"assets/logo.rs","to":"cli::g","symbols":[]},{"unit":"workspace","from":"main.rs","to":"store::i","symbols":[]}]}"#,
    );
    let (body, err, code) = run(&fixture, "--level", "unit", "mermaid");
    assert_eq!(
        code,
        Some(0),
        "a healthy rc=0 artefact stays rc=0 at --level unit with the delta named: {err}"
    );
    assert!(
        err.contains("unit delta model-only: 2 — api→store, cli→core"),
        "the multi-package model-only class is named with its exact count:\n{err}"
    );
    assert!(
        err.contains("unit delta projection-only: 3 — root→store, workspace→cli, workspace→ui"),
        "the multi-package projection-only class is named with its exact count:\n{err}"
    );
    assert!(
        err.contains("accounting exact: 7 drawn + 0 self = 7 raw rows at level unit"),
        "the ledger stays exact beside a non-empty delta:\n{err}"
    );
    assert!(
        !err.contains("completeness restated:"),
        "a healthy artefact prints no restatement row:\n{err}"
    );
    assert!(!body.contains("unit delta"), "body stays census-free:\n{body}");
    ledger(&err);
}

/// Cell 4 — the SCAN law untouched (law-reading, no scan edit): a non-empty
/// gap census still forces `scan` exit 1 — the exit-1 law stays SCAN's own,
/// pinned unchanged by `tests/ts_unresolvable_imports_census.rs`. The render
/// surface of the same run's model answers rc=0 with the delta rows named:
/// the descriptive delta never moves an rc (MAJOR-13).
#[test]
fn scan_keeps_the_exit_one_gap_law_and_the_render_stays_rc_zero() {
    let tree = Fixture::new();
    tree.write("package.json", "{\n  \"name\": \"demo-ui\"\n}\n");
    tree.write(
        "tsconfig.json",
        "{\"compilerOptions\":{\"paths\":{\"@app/*\":[\"./src/app/*\"]}}}",
    );
    tree.write(
        "src/a.ts",
        "import { b } from \"./b\";\nimport { gone } from \"@app/miss\";\nexport const a = b;\n",
    );
    tree.write("src/b.ts", "import { gone } from \"@app/miss\";\nexport const b = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let scan_err = stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "the scan gap-census exit-1 law stays on SCAN unchanged: {scan_err}"
    );
    assert!(
        scan_err.contains("unresolvable imports census:"),
        "scan still names its census at the exit-1 gate: {scan_err}"
    );

    tree.write("model.json", &stdout(&scan));
    let (body, err, code) = run(&tree, "--level", "unit", "mermaid");
    assert_eq!(
        code,
        Some(0),
        "the render surface keeps rc=0 while scan owns the exit-1 law: {err}"
    );
    assert!(
        err.contains("unit delta model-only:") && err.contains("unit delta projection-only:"),
        "the render names the census rows without importing scan's exit effect:\n{err}"
    );
    assert!(!body.contains("unit delta"), "body stays census-free:\n{body}");
}
