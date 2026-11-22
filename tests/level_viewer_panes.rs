//! Workplan level_views scenario 04: recursive panes — bucket-relative
//! drill-down with named refusals in the `--format viewer` document.
//!
//! A level-viewer document drills into each bucket that carries structural
//! internal rows. A pane re-projects its rows RELATIVE TO THE BUCKET ROOT
//! (the FOLD-ROOT LAW — global directory depth is the forbidden shape) and
//! draws the FINEST relative fold that fits the ceilings, through the fit
//! machinery of sibling 02 and the ONE fold site of sibling 01. A pane over
//! ceiling at its finest relative granularity speaks the named in-pane
//! refusal row `pane over ceiling at <granularity> — narrow: archspec scan
//! <bucket-path> --output <artefact>, then archspec diagram --source scan
//! <artefact> [--level <fit>]` (TERMINATION LAW — never an empty pane,
//! never a raw dump) and restates the completeness census row (M6 residue).
//! Node ids are PANE-SCOPED — the bucket path plus the relative label — so
//! identical relative labels in distinct panes stay distinct nodes
//! (MINOR-16). The document states one whole-document total row exactly
//! once per emit: top-cross + Σ pane-internal = raw (MINOR-17). A pane
//! nests under its bucket node as SCOPE, never ownership. The stderr
//! ledger machinery (siblings 01 and 03) stays byte-identical; these cells
//! read the DOCUMENT.

mod common;

use common::{Fixture, stderr, stdout};

/// Run the built binary in the fixture dir and return (code, out, err).
fn run(fixture: &Fixture, args: &[&str]) -> (i32, String, String) {
    let output = fixture.run(args);
    (
        output.status.code().unwrap_or(-1),
        stdout(&output),
        stderr(&output),
    )
}

/// Count non-overlapping occurrences.
fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// The document's spelling of a row: every model string enters the DOM
/// through the shared XML escape, so a row with placeholder tokens ships
/// with its angle brackets escaped (the escape site is `viewer::html_escape`).
fn esc(row: &str) -> String {
    row.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// A short window around the first pane marker, for failure output.
fn excerpt(doc: &str) -> String {
    let at = doc.find("<g class=\"pane").unwrap_or(0);
    let tail: String = doc[at..].chars().take(600).collect();
    format!("…{tail}…")
}

/// The `--format viewer --level dir:1` document for an artefact: one
/// bucket per top-level directory, one drill-down pane per bucket with
/// internal rows.
fn viewer_doc(fixture: &Fixture) -> String {
    let (code, out, err) = run(
        fixture,
        &[
            "diagram",
            "--source",
            "scan",
            "model.json",
            "--level",
            "dir:1",
            "--format",
            "viewer",
        ],
    );
    assert_eq!(code, 0, "the level viewer render must exit 0 — stderr:\n{err}");
    out
}

/// The scan artefact behind every cell: the named units, the module-edge
/// rows, and the optional unresolvable-import census.
fn artefact(units: &[(&str, &str)], edges: &[String], gaps: &[(&str, &str)]) -> Fixture {
    let fixture = Fixture::new();
    let unit_rows: Vec<String> = units
        .iter()
        .map(|(name, path)| format!("{{\"name\":\"{name}\",\"kind\":\"lib\",\"path\":\"{path}\"}}"))
        .collect();
    let edge_rows: Vec<String> = edges
        .iter()
        .map(|row| {
            let (from, to) = row.split_once('|').expect("row spelled from|to");
            format!("{{\"unit\":\"model\",\"from\":\"{from}\",\"to\":\"{to}\",\"symbols\":[]}}")
        })
        .collect();
    let gap_rows: Vec<String> = gaps
        .iter()
        .map(|(file, specifier)| {
            format!("{{\"file\":\"{file}\",\"specifier\":\"{specifier}\",\"line\":\"1\"}}")
        })
        .collect();
    let gaps = if gap_rows.is_empty() {
        String::new()
    } else {
        format!(",\"unresolvable_imports\":[{}]", gap_rows.join(","))
    };
    fixture.write(
        "model.json",
        &format!(
            "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{}],\"edges\":[],\"module_edges\":[{}]{}}}",
            unit_rows.join(","),
            edge_rows.join(","),
            gaps
        ),
    );
    fixture
}

fn rows(pairs: &[(&str, &str)]) -> Vec<String> {
    pairs
        .iter()
        .map(|(from, to)| format!("{from}|{to}"))
        .collect()
}

/// Done-when 1 — bucket-relative fold. The bucket `a` carries 600 internal
/// rows whose file fold puts 600 distinct pairs on the pane — OVER the edge
/// ceiling at global label depth; folded RELATIVE to the bucket root `a/w`
/// the `dir:1` fold aggregates them onto the eight `g…`/`h…` directory
/// buckets and FITS — the witness shape (a deep bucket fits at relative
/// `dir:1` where its file fold is over ceiling). The pane draws that
/// fitting relative fold with RELATIVE
/// labels; the global label shape never appears.
#[test]
fn pane_folds_bucket_relative_never_at_global_depth() {
    let edges: Vec<String> = (0..600)
        .map(|i| format!("a/w/g{}/x{i}.rs|a/w/h{}/y{i}.rs", i % 8, (i + 1) % 8))
        .collect();
    let fixture = artefact(&[("alpha", "a")], &edges, &[]);
    let doc = viewer_doc(&fixture);
    assert!(
        doc.contains("<g class=\"pane\" data-bucket=\"a\" data-level=\"dir:1\">"),
        "the pane for bucket `a` draws the fitting BUCKET-RELATIVE dir:1 fold:\n{}",
        excerpt(&doc)
    );
    assert!(
        doc.contains("data-name=\"a/g0\""),
        "the pane node id is pane-scoped — bucket path plus relative label:\n{}",
        excerpt(&doc)
    );
    assert!(
        doc.contains(">g0</text>"),
        "the pane row carries the RELATIVE label `g0`, not a global path:\n{}",
        excerpt(&doc)
    );
    assert!(
        !doc.contains("data-name=\"a/w/g0\"") && !doc.contains(">a/w/g0</text>"),
        "a pane never folds at GLOBAL directory depth — the global label \
         `a/w/g0` must not appear as a pane node:\n{}",
        excerpt(&doc)
    );
}

/// Done-when 2 — refusal leaf. The bucket `b` carries 600 internal rows
/// whose FINEST relative granularity — `dir:1` over the root `b` — already
/// draws 600 distinct pairs: over ceiling at the first fold of the walk.
/// The pane speaks the named in-pane refusal row with the real bucket path
/// substituted, restates the completeness census row (the fixture declares
/// two unresolvable specifiers — the 2-specifier witness shape, M6), and
/// carries ZERO raw rows: no pane node, no pane arrow, and no drawn or
/// empty pane region beside the refusal.
#[test]
fn refusal_leaf_names_the_row_and_restates_the_census() {
    let edges: Vec<String> = (0..600)
        .map(|i| format!("b/g{i}/x{i}.rs|b/h{i}/y{i}.rs"))
        .collect();
    let fixture = artefact(
        &[("beta", "b")],
        &edges,
        &[
            ("b/g1/x1.rs", "ghost-one"),
            ("b/g2/x2.rs", "ghost-two"),
        ],
    );
    let doc = viewer_doc(&fixture);
    let row = esc(
        "pane over ceiling at dir:1 — narrow: archspec scan b --output <artefact>, then archspec diagram --source scan <artefact> [--level <fit>]",
    );
    assert_eq!(
        count(&doc, &row),
        1,
        "the refusal pane speaks the named row exactly once:\n{}",
        excerpt(&doc)
    );
    assert!(
        doc.contains("<g class=\"pane refusal\" data-bucket=\"b\">"),
        "the refusal pane nests under its bucket node as a refusal region:\n{}",
        excerpt(&doc)
    );
    assert!(
        doc.contains("completeness restated: unresolvable imports census: 2 specifiers across 2 files"),
        "a refusal pane restates the completeness census row (M6 residue):\n{}",
        excerpt(&doc)
    );
    assert!(
        !doc.contains("data-name=\"b/g1/x1.rs\"")
            && !count(&doc, "<line class=\"edge direct\" data-layer=\"direct\" data-from=\"b/g") > 0,
        "a refusal pane carries ZERO raw rows — no node, no arrow:\n{}",
        excerpt(&doc)
    );
    assert_eq!(
        count(&doc, "data-bucket=\"b\""),
        1,
        "no empty pane and no drawn pane beside the refusal — one region per bucket:\n{}",
        excerpt(&doc)
    );
}

/// Done-when 3 — id namespace. Two buckets carry the same relative labels
/// (`a/mod.rs` under each bucket's own root). The panes draw identical
/// relative labels, and the node ids stay pane-scoped: every id is
/// `bucket + label`, so the two `a/mod.rs` labels never share an id.
#[test]
fn pane_scoped_ids_keep_identical_relative_labels_distinct() {
    let fixture = artefact(
        &[("charlie", "c"), ("delta", "d")],
        &rows(&[
            ("c/x/a/mod.rs", "c/x/b/util.rs"),
            ("d/x/a/mod.rs", "d/x/b/util.rs"),
        ]),
        &[],
    );
    let doc = viewer_doc(&fixture);
    assert!(
        doc.contains("data-name=\"c/a/mod.rs\""),
        "pane `c` carries its own `a/mod.rs` node:\n{}",
        excerpt(&doc)
    );
    assert!(
        doc.contains("data-name=\"d/a/mod.rs\""),
        "pane `d` carries its own `a/mod.rs` node:\n{}",
        excerpt(&doc)
    );
    assert_eq!(
        count(&doc, "data-name=\"c/a/mod.rs\""),
        1,
        "the pane-scoped id appears exactly once — no collision inside pane `c`"
    );
    assert_eq!(
        count(&doc, "data-name=\"d/a/mod.rs\""),
        1,
        "the pane-scoped id appears exactly once — no collision inside pane `d`"
    );
    assert_eq!(
        count(&doc, ">a/mod.rs</text>"),
        2,
        "both panes show the SAME relative label — the ids, not the labels, \
         keep the nodes distinct:\n{}",
        excerpt(&doc)
    );
}

/// Done-when 4 — total row. The two-pane fixture with one cross-bucket row
/// carries the whole-document total EXACTLY once, and its arithmetic holds:
/// top-cross + Σ pane-internal = raw = the rows the artefact declares.
#[test]
fn total_row_prints_once_and_its_arithmetic_holds() {
    let pairs = [
        ("c/x/a/mod.rs", "c/x/b/util.rs"),
        ("c/x/b/util.rs", "c/x/a/mod.rs"),
        ("d/x/a/mod.rs", "d/x/b/util.rs"),
        ("c/x/a/mod.rs", "d/x/a/mod.rs"),
    ];
    let fixture = artefact(&[("charlie", "c"), ("delta", "d")], &rows(&pairs), &[]);
    let doc = viewer_doc(&fixture);
    assert_eq!(
        count(&doc, "document total: "),
        1,
        "the whole-document total row prints EXACTLY once per emit:\n{}",
        excerpt(&doc)
    );
    let row = doc
        .lines()
        .find(|line| line.contains("document total: "))
        .expect("the total row");
    let numbers: Vec<usize> = row
        .split_whitespace()
        .filter_map(|token| token.parse::<usize>().ok())
        .collect();
    assert_eq!(numbers.len(), 3, "the total row carries three counts: {row}");
    assert_eq!(
        numbers[0] + numbers[1],
        numbers[2],
        "top-cross + Σ pane-internal = raw: {row}"
    );
    assert_eq!(numbers[2], pairs.len(), "raw rows name the artefact's rows: {row}");
    assert!(
        row.contains("top-cross +") && row.contains("pane-internal ="),
        "the total row states the census in the shipped words: {row}"
    );
}

/// Done-when 5 — nesting is scope. The pane region nests INSIDE its bucket
/// node's `<g>` (no `</g>` between the bucket node's open tag and the pane
/// group), and the document states no ownership claim anywhere: the pane
/// names its bucket as SCOPE only.
#[test]
fn pane_nests_under_the_bucket_node_as_scope_not_ownership() {
    let fixture = artefact(
        &[("charlie", "c"), ("delta", "d")],
        &rows(&[
            ("c/x/a/mod.rs", "c/x/b/util.rs"),
            ("d/x/a/mod.rs", "d/x/b/util.rs"),
        ]),
        &[],
    );
    let doc = viewer_doc(&fixture);
    for bucket in ["c", "d"] {
        let open = format!("<g class=\"node module\" data-name=\"{bucket}\">");
        let at = doc
            .find(&open)
            .unwrap_or_else(|| panic!("the bucket node {bucket} is absent:\n{}", excerpt(&doc)));
        let pane_at = doc[at..]
            .find("<g class=\"pane")
            .expect("a pane follows the bucket node");
        let between = &doc[at + open.len()..at + pane_at];
        assert!(
            !between.contains("</g>"),
            "the pane nests INSIDE the bucket node's <g> — the bucket node \
             cannot close before its pane opens: {between}"
        );
        assert!(
            doc[at + pane_at..].contains("</g>"),
            "the bucket node closes after its pane — the nesting is closed"
        );
        assert!(
            doc.contains(&format!("data-bucket=\"{bucket}\"")),
            "the pane names the bucket it is SCOPE for"
        );
    }
    let lower = doc.to_lowercase();
    for forbidden in ["owns", "owned", "belongs", "parent"] {
        assert!(
            !lower.contains(forbidden),
            "the document states scope, never ownership — `{forbidden}` appears nowhere"
        );
    }
}
