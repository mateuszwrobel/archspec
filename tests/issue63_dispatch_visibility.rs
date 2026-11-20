//! GitHub issue #63 — the depth-dependent root-marker dispatch, cured to
//! VISIBILITY parity.
//!
//! One directory carrying BOTH `Cargo.toml` and `package.json` answered two
//! different questions depending on where the scan was rooted: a STANDALONE
//! scan of the directory refused (`ambiguous root markers`, exit 1, no model
//! — the fail-closed arm, unchanged byte-for-byte), while a scan rooted one
//! level higher silently let one driver claim it (the other language's facts
//! died behind one `nested foreign marker … not modelled` row). Per ADR-069
//! clause 4.8 the dispatch law binds at EVERY depth: ambiguous root markers
//! either refuse with the named refusal or NAME THE WINNER with a census row.
//! This lane ships the first step — VISIBILITY: one deterministic
//! `AMBIGUOUS-MARKERS: <relpath> — markers: <table order>, modeled as:
//! <winner>` report-only row per ambiguous nested directory, riding the same
//! stderr report channel as the foreign-marker line (the machine body stays
//! free of report bytes: scan.json byte-identical). The winner-selection law
//! is UNCHANGED — the scan's driver still claims the tree, the losing
//! language stays unmodelled; refusal/authority semantics are a separate
//! future decision. No zone declaration resolves a nested ambiguity yet
//! (the nested-spec discovery face has not shipped), so every ambiguous
//! nested directory rows.
//!
//! Cells (per the fix plan):
//! 1. nested dual-marker dir → exactly one AMBIGUOUS-MARKERS row, the
//!    winning language's model unchanged, the losing language still
//!    unmodelled (visibility-only fix);
//! 2. standalone scan of the same dual dir → the refusal row unchanged
//!    byte-for-byte, no AMBIGUOUS-MARKERS row (the refusal IS the named
//!    answer at that depth);
//! 3. unambiguous trees → ZERO new rows and a byte-identical scan.json
//!    (the non-declaring law; foreign-marker lines ride unchanged);
//! 4. deterministic ordering under repeated runs (cmp), with the sorted
//!    line set pinned across both report classes.

mod common;

use common::{Fixture, stderr, stdout};

/// The issue's repro tree: `parent/` is a ts root; `pkg/` carries markers
/// for rust AND ts plus one source each.
fn polyglot_tree() -> Fixture {
    let fx = Fixture::new();
    fx.write("package.json", "{\"name\": \"parent-probe\", \"version\": \"0.1.0\"}\n");
    fx.write(
        "pkg/Cargo.toml",
        "[package]\nname = \"polyglot-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write("pkg/package.json", "{\"name\": \"polyglot-probe\", \"version\": \"0.1.0\"}\n");
    fx.write("pkg/src/lib.rs", "pub fn local_helper() -> u32 { 1 }\n");
    fx.write("pkg/src/index.js", "export function localHelper() { return 1; }\n");
    fx
}

// ---------------------------------------------------------------------------
// cell 1 — nested dual-marker dir: one row, winner named, model unchanged
// ---------------------------------------------------------------------------

#[test]
fn nested_dual_marker_dir_names_exactly_one_ambiguous_row() {
    let fx = polyglot_tree();
    let out = fx.run(&["scan", "."]);
    assert!(out.status.success(), "rooted scan failed: {}", stderr(&out));
    let err = stderr(&out);
    let row = "AMBIGUOUS-MARKERS: pkg — markers: Cargo.toml (rust), package.json (ts), modeled as: ts";
    assert_eq!(
        err.matches("AMBIGUOUS-MARKERS").count(),
        1,
        "exactly one AMBIGUOUS-MARKERS row per ambiguous directory:\n{err}"
    );
    assert!(err.contains(row), "stderr: {err}");
    // The existing never-silent arm rides unchanged beside the new row.
    assert!(
        err.contains("nested foreign marker: pkg/Cargo.toml (rust) — not modelled"),
        "stderr: {err}"
    );
    // The machine body carries no report bytes (scan.json byte-identical
    // law): the row never enters the model JSON.
    let model = stdout(&out);
    assert!(!model.contains("AMBIGUOUS-MARKERS"), "model body: {model}");
}

#[test]
fn the_winning_language_model_is_unchanged_and_the_losing_language_stays_unmodelled() {
    // VISIBILITY, not authority: the ts driver still wins (the model
    // spells ts and carries the JS facts), and the rust side of the zone
    // stays unmodelled EXACTLY as before the row — the row changes no
    // model byte (the issue's measured B-model, pinned at the fact level).
    let fx = polyglot_tree();
    let out = fx.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    let model = stdout(&out);
    assert!(model.contains("\"language\":\"ts\""), "model: {model}");
    assert!(model.contains("parent-probe"), "model: {model}");
    assert!(model.contains("pkg/src/index.js"), "model: {model}");
    // The losing language contributes nothing: no rust fact anywhere.
    assert!(!model.contains("pkg/src/lib.rs"), "model: {model}");
    assert!(!model.contains("local_helper"), "model: {model}");
}

// ---------------------------------------------------------------------------
// cell 2 — standalone scan of the dual dir: the refusal arm byte-identical
// ---------------------------------------------------------------------------

#[test]
fn standalone_scan_of_the_dual_dir_refuses_byte_identically() {
    // The same directory scanned at itself answers the SAME named answer
    // the pre-fix binary gave: exit 1, the refusal, no model, and NO
    // AMBIGUOUS-MARKERS row — the refusal is the depth-0 arm, unchanged.
    let fx = polyglot_tree();
    let out = fx.run_in("pkg", &["scan", "."]);
    assert_eq!(out.status.code(), Some(1), "standalone scan must refuse");
    assert_eq!(
        stderr(&out),
        "error: ambiguous root markers: Cargo.toml (rust), package.json (ts) — pin [project] language or scan a single-language root\n",
        "the standalone refusal arm stays byte-identical"
    );
    assert!(stdout(&out).is_empty(), "a refusal writes no machine body");
}

// ---------------------------------------------------------------------------
// cell 3 — unambiguous trees: zero new rows, scan.json byte-identical
// ---------------------------------------------------------------------------

#[test]
fn unambiguous_trees_carry_zero_ambiguous_rows() {
    // One foreign marker per directory (the go/opencode-plugin shape), and
    // one language spelled by a marker FAMILY (the python pair) — neither
    // is an ambiguity, so neither rows. The existing foreign-marker bytes
    // ride unchanged beside the silence.
    let go = Fixture::new();
    go.write("go.mod", "module example.com/tool\n\ngo 1.21\n");
    go.write("cmd/tool/main.go", "package main\n\nfunc main() {}\n");
    go.write("opencode-plugin/package.json", "{\"name\": \"plugin\"}\n");
    go.write("opencode-plugin/index.ts", "export const p = 1;\n");
    let out = go.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    let err = stderr(&out);
    assert!(!err.contains("AMBIGUOUS-MARKERS"), "stderr: {err}");
    assert!(
        err.contains("nested foreign marker: opencode-plugin/package.json (ts) — not modelled"),
        "stderr: {err}"
    );

    let py = Fixture::new();
    py.write("go.mod", "module example.com/tool\n\ngo 1.21\n");
    py.write("main.go", "package main\n\nfunc main() {}\n");
    py.write("py/pyproject.toml", "[project]\nname = \"py\"\n");
    py.write("py/requirements.txt", "six\n");
    let out = py.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    assert!(!stderr(&out).contains("AMBIGUOUS-MARKERS"), "stderr: {}", stderr(&out));
}

#[test]
fn unambiguous_scan_json_is_byte_identical_across_runs() {
    // The non-declaring law at the byte level: the model destination is
    // deterministic (a second run --check answers `up to date`) and no
    // report byte enters it.
    let fx = Fixture::new();
    fx.write("go.mod", "module example.com/tool\n\ngo 1.21\n");
    fx.write("cmd/tool/main.go", "package main\n\nfunc main() {}\n");
    fx.write("opencode-plugin/package.json", "{\"name\": \"plugin\"}\n");
    fx.write("opencode-plugin/index.ts", "export const p = 1;\n");
    let wrote = fx.run(&["scan", ".", "--output", "scan.json"]);
    assert!(wrote.status.success(), "scan failed: {}", stderr(&wrote));
    assert_eq!(stdout(&wrote), "wrote scan.json\n");
    let checked = fx.run(&["scan", ".", "--output", "scan.json", "--check"]);
    assert_eq!(
        checked.status.code(),
        Some(0),
        "--check failed: {}{}",
        stdout(&checked),
        stderr(&checked)
    );
    assert_eq!(stdout(&checked), "ok: scan.json up to date\n");
    assert!(!fx.read("scan.json").contains("AMBIGUOUS-MARKERS"), "the report row never rides the machine body");
}

// ---------------------------------------------------------------------------
// cell 4 — deterministic ordering under repeated runs
// ---------------------------------------------------------------------------

#[test]
fn ambiguous_rows_are_deterministic_across_repeated_runs() {
    // Three runs of the same tree compare byte-identical on BOTH channels
    // (cmp), and the sorted line set is pinned exactly across both report
    // classes: AMBIGUOUS-MARKERS rows before the foreign-marker lines
    // (ASCII sort of the one emitted line set — the emission law), marker
    // lists in ROOT_MARKERS table order.
    let fx = Fixture::new();
    fx.write("package.json", "{\"name\": \"parent-probe\"}\n");
    fx.write("src/app.ts", "export const a = 1;\n");
    fx.write("a/Cargo.toml", "[package]\nname = \"a\"\n");
    fx.write("a/go.mod", "module example.com/a\n");
    fx.write("b/Cargo.toml", "[package]\nname = \"b\"\n");
    fx.write("b/package.json", "{}\n");
    fx.write("c/go.mod", "module example.com/c\n");
    let first = fx.run(&["scan", "."]);
    assert!(first.status.success(), "scan failed: {}", stderr(&first));
    for _ in 0..2 {
        let again = fx.run(&["scan", "."]);
        assert_eq!(
            common::stderr(&again),
            common::stderr(&first),
            "the report channel must be byte-identical across runs"
        );
        assert_eq!(stdout(&again), stdout(&first), "the machine body must be byte-identical across runs");
    }
    let report = stderr(&first);
    let mut census: Vec<&str> = report
        .lines()
        .filter(|line| line.starts_with("AMBIGUOUS-MARKERS") || line.starts_with("nested foreign marker"))
        .collect();
    let sorted: Vec<&str> = {
        let mut v = census.clone();
        v.sort();
        v
    };
    assert_eq!(census, sorted, "the census line set must arrive sorted");
    census.sort();
    assert_eq!(
        census,
        [
            "AMBIGUOUS-MARKERS: a — markers: Cargo.toml (rust), go.mod (go), modeled as: ts",
            "AMBIGUOUS-MARKERS: b — markers: Cargo.toml (rust), package.json (ts), modeled as: ts",
            "nested foreign marker: a/Cargo.toml (rust) — not modelled",
            "nested foreign marker: a/go.mod (go) — not modelled",
            "nested foreign marker: b/Cargo.toml (rust) — not modelled",
            "nested foreign marker: c/go.mod (go) — not modelled",
        ],
        "the full census line set, sorted, pinned byte-for-byte"
    );
}
