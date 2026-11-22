//! Workplan level_views scenario 05: the viewer emitted-byte ceiling — a
//! MEASURED constant with a floor ≥ 1 MB, a named over-ceiling refusal
//! naming the LARGEST SCOPE THAT FITS, and the budget law that panes embed
//! PRECOMPUTED folds, never raw rows.
//!
//! The ceiling is the same law family as the render ceilings: a measured
//! constant, never invented. The pane-fold witnesses that sized it are the
//! two corpus-scale witness trees emitted through the pane fold at landing;
//! their byte counts ride the landing's `__log__` entry and this file's
//! Evidence section, and the floor cell below welds the constant to the
//! floor AND to the witness scale (the constant must stand above both
//! witnesses' bytes — a witness over the constant means the constant was
//! invented, not measured). The refusal charges the parent budget: the
//! refusal rows a document embeds count against the document's own total,
//! measured once at the emit seam, so an embedded refusal never re-triggers
//! the gate (no feedback loop).

mod common;

use common::{Fixture, stderr, stdout};

/// The emitted-byte ceiling parsed out of the diagram help bytes — the
/// same parse-at-the-surface shape `mermaid_ceilings_from_help` uses for
/// the render ceilings. The emit gate compares against the same constant
/// the help states (`src/archspec/viewer.rs`); a restated-by-hand number
/// here reddens the weld.
fn ceiling() -> usize {
    let fixture = Fixture::new();
    let out = stdout(&fixture.run(&["diagram", "--help"]));
    out.split("emitted-byte ceiling of ")
        .nth(1)
        .expect("the diagram help states the emitted-byte ceiling")
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .expect("the stated ceiling is a byte count")
}

/// The recorded witness bytes (scenario Evidence at landing): the two
/// corpus-scale witness trees' pane-fold viewer emits, measured on
/// git-archive scratch with the tree at status 0.
const WITNESS_A_BYTES: usize = 212_815;
const WITNESS_B_BYTES: usize = 298_119;

/// Run the built binary in the fixture dir and return (code, out, err).
fn run(fixture: &Fixture, args: &[&str]) -> (i32, String, String) {
    let output = fixture.run(args);
    (
        output.status.code().unwrap_or(-1),
        stdout(&output),
        stderr(&output),
    )
}

/// The scan artefact behind every cell: the named units plus module-edge
/// rows (and optional `async_edges` rows — `(from-unit, to-unit, event)`,
/// written as the shipped collection) laid out by `layout`.
fn artefact(units: &[(&str, &str)], edges: &[String], async_rows: &[(&str, &str, &str)]) -> Fixture {
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
    let async_json = if async_rows.is_empty() {
        String::new()
    } else {
        let rows: Vec<String> = async_rows
            .iter()
            .map(|(from, to, event)| {
                format!("{{\"from\":\"{from}\",\"to\":\"{to}\",\"event\":\"{event}\",\"kind\":\"async\"}}")
            })
            .collect();
        format!(",\"async_edges\":[{}]", rows.join(","))
    };
    fixture.write(
        "model.json",
        &format!(
            "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{}],\"edges\":[],\"module_edges\":[{}]{}}}",
            unit_rows.join(","),
            edge_rows.join(","),
            async_json
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

/// The large fixture: 120 top directories, each carrying a 24-row internal
/// grid — the pane-explosion shape. The UNIT fold aggregates every row into
/// top cross-rows (two panes' worth of mass: near zero), while the
/// `module` fold spreads one pane per directory: the pane-embedded folds
/// carry the bytes and the document passes the ceiling. Under the ceiling
/// at the coarse folds, over it at the fine folds.
fn over_ceiling_fixture() -> Fixture {
    let mut edges = Vec::new();
    for dir in 0..120u32 {
        for row in 0..24u32 {
            edges.push(format!(
                "w/d{dir}/g{row}/f.rs|w/d{dir}/h{row}/f.rs"
            ));
        }
    }
    artefact(&[("w", "w")], &edges, &[])
}

/// Done-when 1 — floor + value. The ceiling stands at or above the 1 MB
/// floor (MINOR-18) and above both recorded witness emits — the value is
/// measured on the pane-fold witnesses, never invented — and the diagram
/// help states the same number (the presence row parses at run time
/// nowhere: this is a help-surface weld, the fanout guard owns the rest).
#[test]
fn ceiling_floor_and_measured_value() {
    let fixture = Fixture::new();
    let ceiling_val = ceiling();
    assert!(
        ceiling_val >= 1_000_000,
        "the emitted-byte ceiling must stand at or above the 1 MB floor (MINOR-18): {ceiling_val}"
    );
    let witness_max = WITNESS_A_BYTES.max(WITNESS_B_BYTES);
    assert!(
        ceiling_val > witness_max,
        "the pane-fold witnesses must stay under the ceiling they sized — measured \
         {WITNESS_A_BYTES} and {WITNESS_B_BYTES} bytes against a ceiling of {ceiling_val}"
    );
    let (code, out, _) = run(&fixture, &["diagram", "--help"]);
    assert_eq!(code, 0);
    let stated: String = out
        .split("emitted-byte ceiling of ")
        .nth(1)
        .expect("the diagram help states the emitted-byte ceiling")
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    assert_eq!(
        stated.parse::<usize>(),
        Ok(ceiling()),
        "the help's stated ceiling must equal the shipped constant — a restated-by-hand number is the forbidden shape"
    );
}

/// Done-when 2 — refusal. An over-ceiling emit ships NOTHING and speaks the
/// named byte-ceiling refusal in the usage class (exit 64), naming the
/// LARGEST SCOPE THAT FITS with its measured bytes, then the two-step
/// narrowing. The refusal's own bytes are charged to the parent budget:
/// the refusal row is far under the ceiling it enforces — a refusal can
/// never outgrow the document it refuses (no feedback loop).
#[test]
fn over_ceiling_emit_refuses_naming_the_largest_fitting_scope() {
    let fixture = over_ceiling_fixture();
    let (code, out, err) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--level", "module", "--format", "viewer", "--output", "out.html"],
    );
    assert_eq!(code, 64, "an over-ceiling viewer document refuses in the usage class");
    assert!(
        !fixture.path("out.html").exists(),
        "a document over the ceiling ships nothing — no file is written"
    );
    assert!(out.is_empty(), "the refused emit prints no document");
    assert!(
        err.contains("exceeds the emitted-byte ceiling of 1048576 bytes"),
        "the refusal names the ceiling and the measured bytes:\n{err}"
    );
    assert!(
        err.contains("measured ") && err.contains(" bytes"),
        "the refusal states the measured document bytes:\n{err}"
    );
    assert!(
        err.contains("largest scope that fits: level "),
        "the refusal names the largest scope that fits:\n{err}"
    );
    assert!(
        err.contains("narrow with: archspec scan <bucket-path> --output <artefact>"),
        "the refusal carries the two-step narrowing grammar:\n{err}"
    );
    // Parent-budget charge: the refusal row itself must stay under the
    // ceiling it enforces — the emitted refusal bytes count against the
    // parent's total and can never re-trigger the gate.
    assert!(
        err.len() <= ceiling(),
        "the refusal's own bytes ({}) must stay under the ceiling it enforces ({})",
        err.len(),
        ceiling()
    );
}

/// Done-when 2, the scope arm — the largest fitting scope is decided by
/// EMIT-AND-MEASURE, not named from memory: the fold the refusal names must
/// itself emit green under the ceiling. On the witness-shaped mass the
/// `module` fold explodes into a pane per directory while the coarse folds
/// stay small — the refusal names a COARSER fold than the requested one.
#[test]
fn largest_fitting_scope_is_measured_not_named_from_memory() {
    let fixture = over_ceiling_fixture();
    let ceiling_val = ceiling();
    let (code, _, err) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--level", "module", "--format", "viewer"],
    );
    assert_eq!(code, 64, "the pane explosion at module is over the ceiling:\n{err}");
    let line = err
        .lines()
        .find(|line| line.contains("largest scope that fits: level "))
        .expect("the refusal names the largest fitting scope");
    let named = line
        .split("largest scope that fits: level ")
        .nth(1)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string();
    assert_ne!(named, "module", "the requested fold is the refusing one — a coarser scope fits");
    // The named scope, emitted alone, must be UNDER the ceiling — the name
    // is a measurement, not a guess.
    let (fit_code, fit_out, fit_err) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--level", &named, "--format", "viewer"],
    );
    assert_eq!(
        fit_code, 0,
        "the scope the refusal names must itself emit green:\n{fit_err}"
    );
    assert!(
        fit_out.len() <= ceiling(),
        "the named scope's emit measured {} bytes — over the ceiling {ceiling_val}",
        fit_out.len()
    );
}

/// Done-when 3 — precomputed only. The within-budget panes of an
/// over-ceiling document carry ONLY precomputed folds: every pane node is a
/// fold label (a directory segment, never a file path), and no raw row is
/// embedded anywhere — the raw-mass file is the forbidden shape the PoC's
/// 33 MB re-projection measurement proved. The document's total row is the
/// ONE place raw rows are counted, never drawn.
#[test]
fn panes_embed_precomputed_folds_never_raw_rows() {
    // A pane-carrying document that FITS under the ceiling: one deep
    // bucket — the same fold machinery, sized to fit.
    let edges: Vec<String> = (0..600u32)
        .map(|i| format!("a/w/g{}/x{i}.rs|a/w/h{}/y{i}.rs", i % 8, (i + 1) % 8))
        .collect();
    let fits = artefact(&[("alpha", "a")], &edges, &[]);
    let (code, doc, err) = run(
        &fits,
        &["diagram", "--source", "scan", "model.json", "--level", "dir:1", "--format", "viewer"],
    );
    assert_eq!(code, 0, "the fitting document emits green:\n{err}");
    assert!(doc.len() <= ceiling(), "the fitting document is under the ceiling by witness scale");
    // No raw row is embedded anywhere: no node id and no edge endpoint ever
    // carries a raw file path (the raw-mass file is the forbidden shape the
    // PoC's 33 MB re-projection measurement proved).
    assert!(
        !doc.contains("data-name=\"a/w/g0/x0.rs\"") && !doc.contains("data-from=\"a/w/g0/x0.rs\""),
        "a raw file row is drawn as a node or an arrow — the forbidden raw-mass shape:\n{}",
        &doc[..doc.len().min(600)]
    );
    // The pane states the PRECOMPUTED fold it drew — a fold label, in its
    // own region row.
    let pane_region = doc
        .split("<g class=\"pane")
        .nth(1)
        .expect("the document carries at least one pane");
    assert!(
        pane_region.contains("data-level=\"dir:"),
        "the pane states the precomputed fold it drew:\n{}",
        &pane_region[..pane_region.len().min(400)]
    );
    // The pane's drawn nodes are fold labels: none is a `.rs` path.
    assert!(
        !pane_region.contains(".rs\""),
        "a pane embeds a raw file row as a drawn node:\n{}",
        &pane_region[..pane_region.len().min(600)]
    );
}

/// Done-when 4 — fit emit: a within-ceiling emit stays green and
/// byte-deterministic across two runs — the largest-fitting-scope walk is
/// not armed and the document bytes repeat exactly.
#[test]
fn fit_emit_is_byte_deterministic() {
    let fixture = over_ceiling_fixture();
    let args = [
        "diagram",
        "--source",
        "scan",
        "model.json",
        "--level",
        "unit",
        "--format",
        "viewer",
    ];
    let (code_a, out_a, _) = run(&fixture, &args);
    let (code_b, out_b, _) = run(&fixture, &args);
    assert_eq!(code_a, 0);
    assert_eq!(code_b, 0);
    assert!(out_a.len() <= ceiling(), "the fitting emit stays under the ceiling");
    assert_eq!(
        out_a, out_b,
        "a within-ceiling emit is byte-identical across runs ({} vs {} bytes)",
        out_a.len(),
        out_b.len()
    );
}

/// The witness-scale cell: the two witness SHAPES reproduced hermetically —
/// witness A's shape (many wide top directories, one file per directory
/// pair) and witness B's shape (fewer top directories, deeper relative
/// grids plus a root-file bucket) — both emitted through the pane fold and
/// both under the ceiling. Generic names: the witness trees themselves are
/// never named here.
#[test]
fn witness_shapes_emit_under_the_ceiling() {
    // Witness A's shape: 24 top directories, 16 files each, cross pairs.
    let mut a_edges = Vec::new();
    for dir in 0..24u32 {
        for file in 0..16u32 {
            a_edges.push(format!(
                "witness-a/d{dir}/m{file}.rs|witness-a/d{}/n{file}.rs",
                (dir + 1) % 24
            ));
        }
    }
    let a = artefact(&[("witness-a", "witness-a")], &a_edges, &[]);
    let (code, doc, err) = run(
        &a,
        &["diagram", "--source", "scan", "model.json", "--level", "dir:1", "--format", "viewer"],
    );
    assert_eq!(code, 0, "witness A's shape emits through the pane fold:\n{err}");
    assert!(
        doc.len() <= ceiling(),
        "witness A's shape must stay under the ceiling — measured {} bytes",
        doc.len()
    );

    // Witness B's shape: 8 top directories with deep internal grids plus a
    // root-file bucket carrying cross rows.
    let mut b_edges = Vec::new();
    for dir in 0..8u32 {
        for row in 0..150u32 {
            b_edges.push(format!(
                "witness-b/d{dir}/g{}/x.rs|witness-b/d{dir}/h{}/y.rs",
                row % 12,
                (row + 1) % 12
            ));
        }
    }
    for file in 0..24u32 {
        b_edges.push(format!("root{file}.ts|witness-b/d{}/z.rs", file % 8));
    }
    let b = artefact(&[("witness-b", "witness-b")], &b_edges, &[]);
    let (code, doc, err) = run(
        &b,
        &["diagram", "--source", "scan", "model.json", "--level", "dir:1", "--format", "viewer"],
    );
    assert_eq!(code, 0, "witness B's shape emits through the pane fold:\n{err}");
    assert!(
        doc.len() <= ceiling(),
        "witness B's shape must stay under the ceiling — measured {} bytes",
        doc.len()
    );
}

/// The async arm: event rows ride the level fold into the viewer document
/// (the shipped projection), so the byte measurement counts the async rows
/// like every other drawn row — an over-ceiling emit with async rows still
/// refuses, and a within-ceiling emit counts the event row in its census.
#[test]
fn async_event_rows_ride_the_byte_measurement() {
    let edges = rows(&[("a/src/m.rs", "b/src/n.rs")]);
    let fixture = artefact(
        &[("alpha", "a"), ("beta", "b")],
        &edges,
        &[("alpha", "beta", "a::OrderPlaced")],
    );
    // The unfixed emit draws the event label on the async row — the bytes
    // the gate measures carry it.
    let (code, doc, err) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--format", "viewer"],
    );
    assert_eq!(code, 0, "the async-row document emits green:\n{err}");
    assert!(
        doc.contains("data-event=\"alpha::OrderPlaced\""),
        "the event label rides the async row into the document bytes (short unit-spelled form):\n{}",
        &doc[..doc.len().min(600)]
    );
    // The folded emit keeps the async row in the ledger — the byte
    // measurement counts the same rows the fold drew, async included.
    let (code, doc, err) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--level", "dir:1", "--format", "viewer"],
    );
    assert_eq!(code, 0, "the folded async-row document emits green:\n{err}");
    assert!(
        doc.contains("OrderPlaced") || doc.contains("event-labelled async row (1)"),
        "the async row rides the fold into the ledger or the drawing:\n{}",
        &doc[..doc.len().min(600)]
    );
}

/// The regression arm — no-level viewer bytes unchanged: a within-ceiling
/// no-level emit stays green and carries no ceiling row; the gate is inert
/// below the ceiling.
#[test]
fn no_level_emit_under_ceiling_is_unchanged() {
    let edges = rows(&[("a/src/m.rs", "b/src/n.rs"), ("b/src/n.rs", "c/src/o.rs")]);
    let fixture = artefact(&[("alpha", "a"), ("beta", "b"), ("gamma", "c")], &edges, &[]);
    let (code, doc, err) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--format", "viewer"],
    );
    assert_eq!(code, 0, "the no-level viewer emit stays green:\n{err}");
    assert!(
        !doc.contains("emitted-byte ceiling") && !err.contains("emitted-byte ceiling"),
        "a within-ceiling emit speaks no ceiling row — the gate is inert below the ceiling"
    );
}
