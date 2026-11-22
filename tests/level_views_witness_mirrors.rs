//! Workplan level_views scenario 07: the fixture mirrors of the witness
//! battery's five level corpus legs (`internal/witness-battery.sh`). The
//! battery law keeps a tree-drift failure and a code failure separable:
//! every battery expectation is mirrored HERE at fixture scale, so
//! "cell green + leg red" reads as corpus drift and "cell red" reads as
//! the engine moving. The corpora themselves are never fixture inputs and
//! are never named in this file (the payload rows stay generic — the
//! per-corpus receipt numbers live with the battery, outside the payload).
//!
//! Mirrored laws:
//! - the FIT receipt shape: the ledger row parses with `drawn + self = raw`
//!   exact, the emitted graph is node lines + edge lines under the parsed
//!   ceilings, and the self share falls out of the ledger (`fit_leg_1`);
//! - the dir fold buckets by the first directory segments of the endpoint
//!   spelling, tree-root files landing in ONE named bucket (`fit_leg_2`);
//! - the bucket-relative re-projection: a scan rooted at a bucket folds
//!   dir:1 RELATIVE to that root — no bucket label carries a path
//!   separator (`fit_leg_3`);
//! - the accounting exactness is level-invariant and artefact-invariant:
//!   the raw term equals the artefact's module-edge rows at EVERY level
//!   (`fit_leg_4`);
//! - the over-ceiling refusal names the two-step narrowing grammar verbatim
//!   and the two steps execute (`fit_leg_5` — the execution law itself is
//!   also welded by `level_fit_refusal.rs` against a scanned subtree).

mod common;

use common::{assert_declared_before_reference, Fixture};
use std::path::Path;

/// Ledger row parse: `accounting exact: <d> drawn + <s> self = <r> raw rows at level <lv>`.
fn ledger(stderr: &str) -> Option<(usize, usize, usize, String)> {
    let row = stderr.lines().find(|line| line.starts_with("accounting exact: "))?;
    let middle = row
        .trim_start_matches("accounting exact: ")
        .split(" raw ")
        .next()?;
    let level = row
        .split(" at level ")
        .nth(1)
        .unwrap_or_default()
        .to_string();
    let (drawn, rest) = middle.split_once(" drawn + ")?;
    let (self_term, raw) = rest.split_once(" self = ")?;
    Some((
        drawn.trim().parse().ok()?,
        self_term.trim().parse().ok()?,
        raw.trim().parse().ok()?,
        level,
    ))
}

/// The rendered-graph arithmetic the legs recompute from the emitted bytes:
/// one header line, then node lines and edge lines. Returns
/// `(nodes, edges, rows_excluding_header)`.
fn render_shape(body: &str) -> (usize, usize, usize) {
    let lines: Vec<&str> = body.lines().filter(|line| !line.trim().is_empty()).collect();
    assert!(
        lines.first().is_some_and(|line| line.trim() == "graph TD"),
        "the render must open with the graph header, got: {:?}",
        lines.first()
    );
    let edges = lines
        .iter()
        .filter(|line| line.contains("-->") || line.contains("-.->"))
        .count();
    let nodes = lines.len() - 1 - edges;
    (nodes, edges, lines.len() - 1)
}

/// Mermaid ceilings parsed from the binary's own help rationale row —
/// never restated from memory (the battery parse law, mirrored).
fn ceilings(binary: &Path) -> (usize, usize) {
    let out = std::process::Command::new(binary)
        .args(["diagram", "--help"])
        .output()
        .expect("diagram --help");
    let text = String::from_utf8_lossy(&out.stdout).into_owned()
        + &String::from_utf8_lossy(&out.stderr);
    let digits: Vec<usize> = text
        .split_whitespace()
        .skip_while(|token| *token != "ceilings")
        .filter_map(|value| value.replace(',', "").parse().ok())
        .take(2)
        .collect();
    assert_eq!(digits.len(), 2, "the help stopped stating the ceiling pair");
    (digits[0], digits[1])
}

fn scan_artefact(model_json: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write("model.json", model_json);
    fixture
}

/// Mirror of the unit-level FIT leg: the ledger row is exact, the self tally
/// rides the advisory face, and the emitted graph re-computes to
/// nodes + edges under the ceilings with the self share falling out of the
/// ledger (4 raw rows, 2 drawn + 2 self).
#[test]
fn fit_leg_1_unit_level_ledger_and_render_rows_are_exact_under_the_ceilings() {
    let fixture = scan_artefact(
        r#"{"schema_version":1,"language":"ts","units":[{"name":"alpha","kind":"package","path":"src/alpha"},{"name":"beta","kind":"package","path":"src/beta"}],"edges":[],"module_edges":[{"unit":"alpha","from":"src/alpha/x.ts","to":"src/beta/y.ts","symbols":["a"]},{"unit":"alpha","from":"src/alpha/x.ts","to":"src/beta/y.ts","symbols":[]},{"unit":"alpha","from":"src/alpha/x.ts","to":"src/alpha/z.ts","symbols":[]},{"unit":"beta","from":"src/beta/y.ts","to":"src/beta/w.ts","symbols":[]}]}"#,
    );
    let out = fixture.run(&[
        "diagram",
        "--source",
        "scan",
        "model.json",
        "--level",
        "unit",
        "--output",
        "out.mmd",
    ]);
    assert_eq!(out.status.code(), Some(0), "the unit FIT render must land");
    let err = common::stderr(&out);
    let (drawn, self_term, raw, level) = ledger(&err).expect("the ledger row is missing");
    assert_eq!((drawn, self_term, raw), (2, 2, 4), "the ledger moved");
    assert_eq!(level, "unit");
    assert!(
        err.contains("self rows: alpha \u{d7}1") && err.contains("beta \u{d7}1"),
        "the self tally must name both self buckets: {err}"
    );
    assert!(
        err.contains("note: descriptive only — verify gates the module tier"),
        "the descriptive note must ride every level render"
    );
    let body = fixture.read("out.mmd");
    assert_declared_before_reference("unit level render", &body);
    let (nodes, edges, rows) = render_shape(&body);
    assert_eq!((nodes, edges, rows), (2, 1, 3), "nodes + edges must account for every graph line");
    assert!(body.contains("|2 rows|"), "the aggregated edge carries its raw-row count");
    let (edge_ceiling, char_ceiling) = ceilings(&std::path::PathBuf::from(env!("CARGO_BIN_EXE_archspec")));
    assert!(
        edges <= edge_ceiling && body.len() <= char_ceiling,
        "a FIT receipt renders under both ceilings"
    );
    // The self share falls out of the ledger alone (the receipt's own arithmetic).
    assert_eq!(self_term * 100 / raw, 50, "self share percent from the ledger");
}

/// Mirror of the dir:1 FIT leg: buckets are the first directory segments,
/// tree-root files fold into ONE named root bucket, and an intra-bucket
/// raw row rides the self term.
#[test]
fn fit_leg_2_dir1_buckets_by_the_first_segment_with_the_root_class_named() {
    let fixture = scan_artefact(
        r#"{"schema_version":1,"language":"ts","units":[{"name":"root","kind":"package","path":"."}],"edges":[],"module_edges":[{"unit":"root","from":"a/x1.ts","to":"b/y1.ts","symbols":[]},{"unit":"root","from":"b/y1.ts","to":"c/z1.ts","symbols":[]},{"unit":"root","from":"a/deep/x2.ts","to":"a/x1.ts","symbols":[]},{"unit":"root","from":"r.ts","to":"b/y1.ts","symbols":[]}]}"#,
    );
    let out = fixture.run(&[
        "diagram",
        "--source",
        "scan",
        "model.json",
        "--level",
        "dir:1",
        "--output",
        "out.mmd",
    ]);
    assert_eq!(out.status.code(), Some(0), "the dir:1 FIT render must land");
    let err = common::stderr(&out);
    let (drawn, self_term, raw, level) = ledger(&err).expect("the ledger row is missing");
    assert_eq!((drawn, self_term, raw), (3, 1, 4), "the dir:1 fold moved");
    assert_eq!(level, "dir:1");
    assert!(err.contains("a \u{d7}1"), "the self bucket must be named by segment: {err}");
    let body = fixture.read("out.mmd");
    for bucket in ["a", "b", "c", "root"] {
        assert!(
            body.contains(&format!(r#"["{bucket}"]"#)) || body.contains(&format!("{bucket} -->")) || body.lines().any(|line| line.trim() == bucket),
            "bucket {bucket} must appear as its own node"
        );
    }
    let (nodes, edges, rows) = render_shape(&body);
    assert_eq!(nodes + edges, rows, "every graph line is a node or an edge line");
}

/// Mirror of the bucket-relative re-projection leg: a scan rooted at the
/// bucket (an artefact whose endpoints are bucket-relative spellings) folds
/// dir:1 RELATIVE to the bucket — no emitted bucket label carries a path
/// separator, and the rendered row count re-computes from the bytes.
#[test]
fn fit_leg_3_bucket_relative_reldir1_labels_carry_no_path_separator() {
    let fixture = scan_artefact(
        r#"{"schema_version":1,"language":"ts","units":[{"name":"bucket","kind":"package","path":"."}],"edges":[],"module_edges":[{"unit":"bucket","from":"tools/x.ts","to":"core/y.ts","symbols":[]},{"unit":"bucket","from":"tools/x.ts","to":"core/y.ts","symbols":[]},{"unit":"bucket","from":"core/y.ts","to":"utils/z.ts","symbols":[]},{"unit":"bucket","from":"rooted.ts","to":"tools/x.ts","symbols":[]}]}"#,
    );
    let out = fixture.run(&[
        "diagram",
        "--source",
        "scan",
        "model.json",
        "--level",
        "dir:1",
        "--output",
        "out.mmd",
    ]);
    assert_eq!(out.status.code(), Some(0), "the reldir1 render must land");
    let err = common::stderr(&out);
    let (drawn, self_term, raw, level) = ledger(&err).expect("the ledger row is missing");
    assert_eq!((drawn, self_term, raw), (4, 0, 4), "the bucket-relative fold moved");
    assert_eq!(level, "dir:1");
    let body = fixture.read("out.mmd");
    // Bucket labels are the emitted node labels (`id ["label"]` rows and
    // bare id rows) — none may carry a path separator.
    for chunk in body.split(r#"["#) {
        let label = chunk.split("\"]").next().unwrap_or_default();
        if !label.is_empty() && !label.contains('\n') {
            assert!(
                !label.contains('/'),
                "bucket label {label:?} carries a path separator — the fold is tree-relative, not bucket-relative"
            );
        }
    }
    assert!(
        !body.contains(r#"["a/"#) && !body.contains("a/\""),
        "no bucket label may spell a nested path"
    );
    let (nodes, edges, rows) = render_shape(&body);
    assert!(nodes >= 3 && edges >= 3 && rows == nodes + edges, "the row count re-computes from the emitted lines");
}

/// Mirror of the accounting-exactness leg: the raw term equals the artefact's
/// module-edge row count at EVERY level the legs receipt (the level- and
/// layer-invariant raw total), and drawn + self equals it at each one.
#[test]
fn fit_leg_4_accounting_exact_row_sums_to_the_artefact_rows_at_every_level() {
    let fixture = scan_artefact(
        r#"{"schema_version":1,"language":"ts","units":[{"name":"alpha","kind":"package","path":"src/alpha"},{"name":"beta","kind":"package","path":"src/beta"}],"edges":[],"module_edges":[{"unit":"alpha","from":"src/alpha/x.ts","to":"src/beta/y.ts","symbols":[]},{"unit":"alpha","from":"src/alpha/deep/x2.ts","to":"src/alpha/x.ts","symbols":[]},{"unit":"beta","from":"src/beta/y.ts","to":"src/beta/w.ts","symbols":[]},{"unit":"beta","from":"src/beta/y.ts","to":"src/alpha/x.ts","symbols":[]},{"unit":"alpha","from":"r.ts","to":"src/beta/y.ts","symbols":[]}]}"#,
    );
    for level in ["unit", "dir:1", "dir:2", "module"] {
        let out = fixture.run(&[
            "diagram", "--source", "scan", "model.json", "--level", level, "--output", "-",
        ]);
        assert_eq!(out.status.code(), Some(0), "level {level} must render (fixture scale)");
        let err = common::stderr(&out);
        let (drawn, self_term, raw, row_level) =
            ledger(&err).unwrap_or_else(|| panic!("the ledger row is missing at level {level}"));
        assert_eq!(raw, 5, "the raw term must equal the artefact's module-edge rows at level {level}");
        assert_eq!(drawn + self_term, raw, "drawn + self must equal raw at level {level}");
        assert_eq!(row_level, level);
    }
}

/// Mirror of the narrowing-grammar leg: an over-ceiling fold emits NOTHING,
/// refuses at exit 64, and the refusal names the fits plus the two-step
/// narrowing grammar verbatim; then the two steps run on a real scanned
/// bucket and the ledger lands exact.
#[test]
fn fit_leg_5_refusal_names_the_two_step_grammar_and_the_steps_execute() {
    let fixture = Fixture::new();
    // The plant: 501 DISTINCT cross pairs — over the parsed edge ceiling at
    // level module, fitting at every coarser fold. One unit per source row:
    // rows that fold into one pair would FIT (the pair count is the ceiling
    // subject, not the raw-row count).
    let mut rows = String::new();
    let mut units = String::new();
    for index in 0..501 {
        if index > 0 {
            rows.push(',');
            units.push(',');
        }
        rows.push_str(&format!(
            r#"{{"unit":"m{index}","from":"src/m{index}/x.ts","to":"sink/y.ts","symbols":[]}}"#
        ));
        units.push_str(&format!(
            r#"{{"name":"m{index}","kind":"package","path":"src/m{index}"}}"#
        ));
    }
    units.push_str(r#",{"name":"sink","kind":"package","path":"sink"}"#);
    fixture.write(
        "plant.json",
        &format!(
            r#"{{"schema_version":1,"language":"ts","units":[{units}],"edges":[],"module_edges":[{rows}]}}"#
        ),
    );
    let out = fixture.run(&[
        "diagram", "--source", "scan", "plant.json", "--level", "module", "--output", "-",
    ]);
    assert_eq!(out.status.code(), Some(64), "an over-ceiling fold refuses at 64");
    let err = common::stderr(&out);
    assert!(out.stdout.is_empty(), "an over-ceiling refusal emits NOTHING on stdout");
    assert!(err.contains("renders nothing; fits:"), "the fits clause is missing: {err}");
    assert!(
        err.contains("narrow with: archspec scan <bucket-path> --output <artefact>"),
        "step one of the grammar is missing"
    );
    assert!(
        err.contains("then archspec diagram --source scan <artefact> [--level <fit>]"),
        "step two of the grammar is missing"
    );

    // The two steps run on a real scanned tree (the battery's exec face):
    // scan the subtree, then re-project the artefact at a fitting level.
    // The cargo workspace shape is the one welded by level_fit_refusal.rs —
    // a clean tree (every import resolves inside the workspace) exits 0.
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"alpha\", \"beta\"]\n");
    fixture.write(
        "alpha/Cargo.toml",
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("alpha/src/lib.rs", "pub fn g() {}\n");
    fixture.write(
        "beta/Cargo.toml",
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nalpha = { path = \"../alpha\" }\n",
    );
    fixture.write("beta/src/lib.rs", "mod util;\nuse alpha::g;\npub fn h() { util::k(); g(); }\n");
    fixture.write("beta/src/util.rs", "pub fn k() {}\n");
    let scan = fixture.run(&["scan", ".", "--output", "bucket-scan.json"]);
    assert_eq!(scan.status.code(), Some(0), "the bucket scan must land: {}", common::stderr(&scan));
    let render = fixture.run(&[
        "diagram", "--source", "scan", "bucket-scan.json", "--level", "dir:1",
        "--output", "bucket.mmd",
    ]);
    assert_eq!(render.status.code(), Some(0), "the narrowed render must land: {}", common::stderr(&render));
    let err = common::stderr(&render);
    let (drawn, self_term, raw, _) = ledger(&err).expect("the narrowed render must carry the ledger row");
    assert_eq!(drawn + self_term, raw, "the narrowed ledger stays exact");
    // The intra-crate file edge is the module-edge row; the cross-crate
    // import rides the declared unit pairs (the unit-delta advisory face).
    assert!(raw >= 1, "the scanned tree must carry at least one module-edge row: {err}");
}
