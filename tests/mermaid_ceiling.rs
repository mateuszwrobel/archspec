//! The mermaid ceiling law armed at the emit seam (improvement-sweep HIGH-1
//! cure, 2026-11-20). The scenario-05 rationale states the fact — mermaid
//! hosts pin ceilings of 500 edges and 50,000 characters, and a graph over
//! either ceiling renders nothing — and the sweep found the arm missing: a
//! 700-edge synthetic scan artefact made `archspec diagram --source scan`
//! print its full mermaid body at exit 0, so the tool's own output was the
//! silent failure the rationale warns about. These cells are the adversarial
//! set for the arm: an over-ceiling render refuses loud at exit 64 and
//! touches nothing, a render at either ceiling still renders, and the
//! ceiling-free formats keep their shipped bytes.
mod common;

use common::{stderr, stdout, Fixture};

/// A synthetic scan artefact with `edges + 1` units and exactly `edges`
/// structural edge rows (unit u000 → u001 → …), so the rendered mermaid
/// graph carries exactly `edges` `-->` rows. The model JSON is written
/// byte-cheap by hand; the format is the one `scan --output` emits.
fn artefact_fixture(edges: usize) -> Fixture {
    let fx = Fixture::new();
    let units: Vec<String> = (0..=edges)
        .map(|i| format!("  {{ \"name\": \"u{i:03}\", \"kind\": \"lib\", \"path\": \"u{i:03}\" }}"))
        .collect();
    let edges_json: Vec<String> = (0..edges)
        .map(|i| format!("  {{ \"from\": \"u{i:03}\", \"to\": \"u{:03}\" }}", i + 1))
        .collect();
    fx.write(
        "model.json",
        &format!(
            "{{\n  \"schema_version\": 1,\n  \"language\": \"rust\",\n  \"units\": [\n{}\n  ],\n  \"edges\": [\n{}\n  ],\n  \"external\": [],\n  \"async_edges\": []\n}}\n",
            units.join(",\n"),
            edges_json.join(",\n"),
        ),
    );
    fx
}

/// The sweep's exact exploit shape with a destination configured: an
/// over-ceiling render must refuse at exit 64 and leave the committed
/// destination byte-untouched — the refusal happens before any emit, so no
/// partial body and no truncation can reach a file.
#[test]
fn an_over_ceiling_render_refuses_and_touches_nothing() {
    let fx = artefact_fixture(501);
    fx.write("archspec.toml", "[output]\ndiagram = \"diagram.mmd\"\n");
    fx.write("diagram.mmd", "COMMITTED UNDER-CEILING GRAPH\n");
    let refused = fx.run(&["diagram", "--source", "scan", "model.json"]);
    assert_eq!(
        refused.status.code(),
        Some(64),
        "an over-ceiling render rides the layer arms' usage class:\n{}",
        stderr(&refused)
    );
    let err = stderr(&refused);
    assert!(
        err.contains(
            "this render exceeds the mermaid ceilings of 500 edges and 50,000 characters"
        ),
        "the refusal states the ceiling fact: {err}"
    );
    assert!(err.contains("measured 501 edges and "), "{err}");
    assert!(
        err.contains("a graph over either ceiling renders nothing"),
        "{err}"
    );
    assert!(
        err.contains("clear it with --format dot"),
        "the refusal names the clear-arms: {err}"
    );
    assert!(
        !stdout(&refused).contains("graph"),
        "an over-ceiling render emits no body on stdout:\n{}",
        stdout(&refused)
    );
    assert_eq!(fx.read("diagram.mmd"), "COMMITTED UNDER-CEILING GRAPH\n");
}

/// The sweep's stdout shape (no destination configured): the full-body print
/// at exit 0 is the failure class — after the cure the same command is
/// nonzero with the named row and an empty stdout.
#[test]
fn the_sweep_exploit_prints_no_body_and_exits_nonzero() {
    let fx = artefact_fixture(501);
    let refused = fx.run(&["diagram", "--source", "scan", "model.json"]);
    assert_ne!(refused.status.code(), Some(0), "the 700-edge print-at-exit-0 exploit must refuse");
    assert!(stdout(&refused).is_empty(), "nothing prints on refusal:\n{}", stdout(&refused));
    assert!(stderr(&refused).contains("this render exceeds the mermaid ceilings"), "refusal row: {}", stderr(&refused));
}

/// Boundary law: the ceilings are inclusive — a render of exactly 500 edge
/// rows is at the ceiling, not over it, and renders normally.
#[test]
fn a_render_at_the_edge_ceiling_still_renders() {
    let fx = artefact_fixture(500);
    let ran = fx.run(&["diagram", "--source", "scan", "model.json"]);
    assert_eq!(ran.status.code(), Some(0), "{}", stderr(&ran));
    let out = stdout(&ran);
    assert!(out.starts_with("graph TD"), "{out}");
    assert_eq!(
        out.lines().filter(|line| line.contains("-->")).count(),
        500,
        "the boundary render carries exactly the 500 allowed edge rows"
    );
    assert!(
        !stderr(&ran).contains("exceeds the mermaid ceilings"),
        "an at-ceiling render speaks no refusal: {}",
        stderr(&ran)
    );
}

/// The cure must not push users into a worse host: plantuml carries no
/// mermaid-host ceiling, so the same over-ceiling model still renders with
/// the format switch the refusal names.
#[test]
fn the_same_model_renders_with_the_named_clear_arm() {
    let fx = artefact_fixture(501);
    let ran = fx.run(&["diagram", "--source", "scan", "model.json", "--format", "plantuml"]);
    assert_eq!(ran.status.code(), Some(0), "{}", stderr(&ran));
    assert!(stdout(&ran).starts_with("@startuml"));
}
