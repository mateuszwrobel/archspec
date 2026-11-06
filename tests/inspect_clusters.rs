//! Deterministic clustering overlay — `archspec inspect tree|scanner
//! --clusters` (lane A, workplan archspec_inspect_clusters).
//!
//! The overlay is advisory display: it APPENDS a structure verdict line and
//! sorted assignment lines to the model render, and the `--json` machine
//! form carries exactly the additive `clusters` key. These tests pin the
//! contract through the CLI: (a) without the flag nothing changes — no
//! overlay token appears in any inspect mode; (b) with the flag the diagram
//! bytes, artefact bytes and `--check` verdicts are unchanged; (c) every
//! structure verdict class renders its exact line; (d) both caps name their
//! skip line and never fail a consumer; (e) the refusals are usage errors.
//!
//! The clustering ARITHMETIC (merge predicate, tie law, cap classes) is
//! pinned at the pure-module level in `src/archspec/inspect/clusters.rs`;
//! this suite pins the surface those unit tests feed. All fixtures are
//! synthetic (contract §6): generic words and letters, no real crate names.
//!
//! Canonical acceptance rows: `docs/archspec/commands/inspect/acceptance.md`
//! (clustering rows).

mod common;

use common::{Fixture, stderr, stdout};
use serde_json::Value;
use std::collections::BTreeSet;

/// A cargo tree whose modules form the given dependency graph: each entry is
/// (module, dependencies). Module ids are single letters so node ids sort
/// predictably (`<pkg>::a` …).
fn tree(name: &str, modules: &[(&str, &[&str])]) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    );
    let mut lib = String::new();
    for (module, deps) in modules {
        lib.push_str(&format!("pub mod {module};\n"));
        let mut body = String::new();
        for dep in *deps {
            body.push_str(&format!("use crate::{dep};\n"));
        }
        body.push_str("pub const X: u32 = 1;\n");
        fixture.write(&format!("src/{module}.rs"), &body);
    }
    fixture.write("src/lib.rs", &lib);
    fixture
}

fn tri_fixture() -> Fixture {
    tree("tri", &[("a", &["b"]), ("b", &["c"]), ("c", &["a"])])
}

/// Two triangles joined by one bridge edge — the `n clusters` class: the
/// bridge pair's integer predicate is negative, so two clusters survive.
fn bridge_fixture() -> Fixture {
    tree(
        "bridge",
        &[
            ("a", &["b"]),
            ("b", &["c"]),
            ("c", &["a", "d"]),
            ("d", &["e"]),
            ("e", &["f"]),
            ("f", &["d"]),
        ],
    )
}

/// Three disjoint pairs — every candidate pair carries the same predicate
/// value (the round-1 tie class), so merge order is fixed only by the sorted
/// scan and the (min-member, min-member) law.
fn tiedense_fixture() -> Fixture {
    tree(
        "tiedense",
        &[
            ("a", &["b"]),
            ("b", &[]),
            ("c", &["d"]),
            ("d", &[]),
            ("e", &["f"]),
            ("f", &[]),
        ],
    )
}

/// Declared modules with no edges between them — the empty class.
fn edgeless_fixture() -> Fixture {
    tree("edgeless", &[("a", &[]), ("b", &[]), ("c", &[])])
}

/// The overlay part of stdout: from the structure (or skip) line to the end.
/// Panics when no overlay line is present.
fn overlay_lines(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with("structure: ") || l.starts_with("clusters: skipped"))
        .expect("overlay verdict line present");
    lines[start..].iter().map(|l| l.to_string()).collect()
}

fn assignment_pairs(overlay: &[String]) -> Vec<(String, String)> {
    overlay[1..]
        .iter()
        .map(|line| {
            let (node, cluster) = line
                .split_once(" -> ")
                .expect("assignment line is `node -> cluster`");
            (node.to_string(), cluster.to_string())
        })
        .collect()
}

fn has_assignment_line(text: &str) -> bool {
    text.lines()
        .any(|l| l.contains(" -> ") && !l.contains("-->"))
}

// ---------------------------------------------------------------------------
// Scenario: default bytes unchanged by the flag
// ---------------------------------------------------------------------------

#[test]
fn no_overlay_token_appears_in_any_inspect_mode_without_the_flag() {
    let fixture = bridge_fixture();
    for args in [
        vec!["inspect", "."],
        vec!["inspect", "tree", "."],
        vec!["inspect", "scanner", "."],
    ] {
        let output = fixture.run(&args);
        assert_eq!(output.status.code(), Some(0), "args {args:?}");
        let out = stdout(&output);
        assert!(
            !out.contains("structure: "),
            "args {args:?} carry a structure line"
        );
        assert!(
            !out.contains("clusters:"),
            "args {args:?} carry a clusters line"
        );
        assert!(
            !has_assignment_line(&out),
            "args {args:?} carry assignment lines"
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario: overlay appends, never rewrites
// ---------------------------------------------------------------------------

#[test]
fn the_overlay_appends_after_unchanged_diagram_bytes() {
    let fixture = bridge_fixture();
    let off = stdout(&fixture.run(&["inspect", "tree", "."]));
    let on = stdout(&fixture.run(&["inspect", "tree", ".", "--clusters"]));
    assert!(
        on.starts_with(&off),
        "the diagram part is byte-identical to the run without the flag"
    );
    let overlay = overlay_lines(&on);
    assert_eq!(overlay[0], "structure: 2 clusters");
    assert_eq!(&on[off.len()..], overlay.join("\n") + "\n");
}

#[test]
fn output_destination_artefacts_are_identical_with_and_without_the_flag() {
    let fixture = bridge_fixture();
    let off = fixture.run(&["inspect", "tree", ".", "--output", "off.mmd"]);
    assert_eq!(off.status.code(), Some(0));
    let on = fixture.run(&["inspect", "tree", ".", "--clusters", "--output", "on.mmd"]);
    assert_eq!(on.status.code(), Some(0));
    assert_eq!(fixture.read("off.mmd"), fixture.read("on.mmd"));
    // The overlay goes to stdout; the artefact keeps the diagram alone.
    let overlay = overlay_lines(&stdout(&on));
    assert!(overlay[0].starts_with("structure: "));
    assert!(!fixture.read("on.mmd").contains("structure:"));
}

#[test]
fn check_verdicts_are_unchanged_by_the_flag() {
    let fixture = bridge_fixture();
    fixture.run(&["inspect", "tree", ".", "--output", "d.mmd"]);
    for extra in [&[], &["--clusters"][..]] {
        let mut args = vec!["inspect", "tree", ".", "--check", "--output", "d.mmd"];
        args.extend(extra);
        let output = fixture.run(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "fresh check stays green: {extra:?}"
        );
        assert!(stdout(&output).starts_with("ok: d.mmd up to date"));
    }
    fixture.write("d.mmd", "stale bytes\n");
    for extra in [&[], &["--clusters"][..]] {
        let mut args = vec!["inspect", "tree", ".", "--check", "--output", "d.mmd"];
        args.extend(extra);
        let output = fixture.run(&args);
        assert_ne!(
            output.status.code(),
            Some(0),
            "stale check stays red: {extra:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario: assignment lines carry the model's node ids
// ---------------------------------------------------------------------------

#[test]
fn assignments_name_exactly_the_model_node_ids() {
    let fixture = bridge_fixture();
    let on = stdout(&fixture.run(&["inspect", "tree", ".", "--clusters"]));
    let model_nodes: BTreeSet<String> = on
        .lines()
        .filter(|l| !l.contains("subgraph")) // subgraph titles are not node ids
        .filter_map(|l| {
            let open = l.find("[\"")?;
            let rest = &l[open + 2..];
            let close = rest.find("\"]")?;
            Some(rest[..close].to_string())
        })
        .collect();
    let overlay = overlay_lines(&on);
    let assignment_nodes: BTreeSet<String> = assignment_pairs(&overlay)
        .into_iter()
        .map(|(node, _)| node)
        .collect();
    assert_eq!(
        assignment_nodes, model_nodes,
        "assignments carry exactly the model's node ids"
    );
    // Every cluster column is the lexicographically smallest member.
    let pairs = assignment_pairs(&overlay);
    for (node, cluster) in &pairs {
        let members: Vec<&str> = pairs
            .iter()
            .filter(|(_, c)| c == cluster)
            .map(|(n, _)| n.as_str())
            .collect();
        assert!(
            members.contains(&cluster.as_str()),
            "cluster {cluster} contains itself"
        );
        assert!(
            members.iter().all(|m| *m >= cluster.as_str()),
            "cluster {cluster} is named by its smallest member"
        );
        assert!(!node.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Scenario: structure verdicts
// ---------------------------------------------------------------------------

#[test]
fn structure_verdict_n_clusters() {
    let fixture = bridge_fixture();
    let output = fixture.run(&["inspect", "tree", ".", "--clusters"]);
    assert_eq!(output.status.code(), Some(0));
    let overlay = overlay_lines(&stdout(&output));
    assert_eq!(overlay[0], "structure: 2 clusters");
    let clusters: BTreeSet<String> = assignment_pairs(&overlay)
        .into_iter()
        .map(|(_, cluster)| cluster)
        .collect();
    assert_eq!(
        clusters.len(),
        2,
        "the line's count equals the number of distinct columns"
    );
}

#[test]
fn structure_verdict_empty() {
    let fixture = edgeless_fixture();
    let output = fixture.run(&["inspect", "tree", ".", "--clusters"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the empty class never fails a consumer"
    );
    let out = stdout(&output);
    let overlay = overlay_lines(&out);
    assert_eq!(
        overlay,
        vec!["structure: empty".to_string()],
        "no assignment lines follow"
    );
}

#[test]
fn structure_verdict_single_region_names_every_node() {
    let fixture = tri_fixture();
    let output = fixture.run(&["inspect", "tree", ".", "--clusters"]);
    assert_eq!(output.status.code(), Some(0));
    let overlay = overlay_lines(&stdout(&output));
    assert_eq!(overlay[0], "structure: none (single region)");
    let pairs = assignment_pairs(&overlay);
    assert_eq!(
        pairs.len(),
        3,
        "the block names every node in the one cluster"
    );
    for (_, cluster) in &pairs {
        assert_eq!(cluster, "tri::a");
    }
}

// ---------------------------------------------------------------------------
// Scenario: deterministic across runs (golden weld)
// ---------------------------------------------------------------------------

#[test]
fn tie_dense_overlay_is_deterministic_and_matches_the_golden() {
    let fixture = tiedense_fixture();
    let first = stdout(&fixture.run(&["inspect", "tree", ".", "--clusters"]));
    let second = stdout(&fixture.run(&["inspect", "tree", ".", "--clusters"]));
    assert_eq!(first, second, "fresh processes render identical bytes");
    let json: Value = serde_json::from_str(&stdout(&fixture.run(&[
        "inspect",
        "tree",
        ".",
        "--clusters",
        "--json",
    ])))
    .expect("machine form parses");
    // The golden is the canonical join line block: sorted `node<TAB>cluster`
    // lines with a trailing newline (workplan §Decisions — the PoC's
    // determinism mechanism transferred 1:1).
    let lines: Vec<String> = json["clusters"]["assignments"]
        .as_array()
        .expect("assignments array")
        .iter()
        .map(|a| {
            format!(
                "{}\t{}",
                a["node"].as_str().expect("node id"),
                a["cluster"].as_str().expect("cluster id")
            )
        })
        .collect();
    let golden = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/goldens/inspect-clusters/tie-dense.txt"
    ))
    .expect("tie-dense golden present");
    assert_eq!(golden, lines.join("\n") + "\n");
}

// ---------------------------------------------------------------------------
// Scenario: caps refuse with named skip lines
// ---------------------------------------------------------------------------

#[test]
fn node_cap_refuses_with_a_named_skip_line() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"wide\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    // 10 001 modules: each declares itself and reaches m0, so every module is
    // a graph node (module nodes are edge endpoints plus soft structure).
    let mut lib = String::new();
    for i in 0..=10_000 {
        lib.push_str(&format!("pub mod m{i:05};\n"));
        if i == 0 {
            fixture.write("src/m00000.rs", "pub const X: u32 = 0;\n");
        } else {
            fixture.write(
                &format!("src/m{i:05}.rs"),
                "use crate::m00000;\npub const X: u32 = 1;\n",
            );
        }
    }
    fixture.write("src/lib.rs", &lib);
    let output = fixture.run(&["inspect", "tree", ".", "--clusters"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the overlay never fails a consumer"
    );
    let overlay = overlay_lines(&stdout(&output));
    assert_eq!(
        overlay,
        vec!["clusters: skipped (nodes=10001 > cap 10000)".to_string()]
    );
}

#[test]
fn edge_cap_refuses_with_a_named_skip_line() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"dense\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    // A 511-module clique: 511×510 directed module edges fold to 260 610
    // symmetrized undirected edges — beyond the cap with a within-cap node
    // count (511 ≤ 10 000).
    let n = 511usize;
    let mut lib = String::new();
    for i in 0..n {
        lib.push_str(&format!("pub mod m{i:03};\n"));
        let mut body = String::new();
        for j in 0..n {
            if i != j {
                body.push_str(&format!("use crate::m{j:03};\n"));
            }
        }
        body.push_str("pub const X: u32 = 1;\n");
        fixture.write(&format!("src/m{i:03}.rs"), &body);
    }
    fixture.write("src/lib.rs", &lib);
    let output = fixture.run(&["inspect", "tree", ".", "--clusters"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the overlay never fails a consumer"
    );
    let overlay = overlay_lines(&stdout(&output));
    assert_eq!(
        overlay,
        vec!["clusters: skipped (edges=260610 > cap 130000)".to_string()]
    );
}

// ---------------------------------------------------------------------------
// Scenario: machine form carries the additive key
// ---------------------------------------------------------------------------

#[test]
fn machine_form_carries_exactly_the_clusters_key() {
    let fixture = bridge_fixture();
    let output = fixture.run(&["inspect", "tree", ".", "--clusters", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    let value: Value = serde_json::from_str(out.trim()).expect("one JSON object on stdout");
    let keys: Vec<&str> = value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, vec!["clusters"], "exactly the additive key");
    let clusters = &value["clusters"];
    assert_eq!(clusters["schema"], "archspec-clusters-v0");
    assert_eq!(clusters["structure"], "2 clusters");
    assert_eq!(clusters["nodes"], 6);
    assert_eq!(
        clusters["edges"], 7,
        "the symmetrized, self-loop-free edge count"
    );
    let assignments = clusters["assignments"]
        .as_array()
        .expect("assignments array");
    let nodes: Vec<&str> = assignments
        .iter()
        .map(|a| a["node"].as_str().expect("node id"))
        .collect();
    let mut sorted = nodes.clone();
    sorted.sort_unstable();
    assert_eq!(nodes, sorted, "assignments are sorted by node id");
    assert_eq!(nodes.len(), 6);
}

#[test]
fn the_skip_classes_carry_zero_assignments_in_the_machine_form() {
    let fixture = edgeless_fixture();
    let output = fixture.run(&["inspect", "tree", ".", "--clusters", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_str(stdout(&output).trim()).expect("parses");
    assert_eq!(value["clusters"]["structure"], "empty");
    assert!(
        value["clusters"]["assignments"]
            .as_array()
            .expect("array")
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// Scenario: flag refusals are usage errors
// ---------------------------------------------------------------------------

#[test]
fn clusters_on_the_file_level_mode_is_a_refused_usage_error() {
    let fixture = tri_fixture();
    let output = fixture.run(&["inspect", ".", "--clusters"]);
    assert_eq!(output.status.code(), Some(64));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output)
            .contains("--clusters is only valid on the model modes: archspec inspect tree|scanner")
    );
}

#[test]
fn json_without_clusters_is_a_refused_usage_error() {
    let fixture = tri_fixture();
    let output = fixture.run(&["inspect", "tree", ".", "--json"]);
    assert_eq!(output.status.code(), Some(64));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output)
            .contains("--json renders the clustering overlay only: pass it alongside --clusters")
    );
}
