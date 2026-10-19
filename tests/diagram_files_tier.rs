//! Phase C cells of the ts-driver design record — the diagram files tier
//! (D8, M13): `--tiers files` renders the territory's file vertices and
//! first-party file edges in SPEC source mode only, marks cycle arrows
//! through the adjudication seam's cycle-membership query (never a
//! diagram-side SCC re-derivation — the marks agree verdict-for-verdict
//! with `verify`), assets render incoming-only and never cycle-marked,
//! the scan surface refuses the tier (extended territory refusal), the
//! size law refuses an unscoped render, and every default render byte is
//! unchanged.

mod common;

use common::Fixture;
use std::collections::BTreeMap;

fn ts_tree(spec: &str, files: &[(&str, &str)]) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write("architecture.spec.toml", spec);
    for (path, content) in files {
        tree.write(path, content);
    }
    tree
}

const CYCLE_SPEC: &str = "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n";

/// The registered refusal bytes (claim-registry weld: files-tier-scan-refusal).
const FILES_SCAN_REFUSAL: &str = "--tiers files requires the spec source — the files tier marks through the adjudication seam, which needs the spec's facts a scan artefact does not carry";

fn cycle_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "src/views/a.ts",
            "import { b } from \"../data/b\";\nimport \"../data/theme.css\";\nexport const a = b;\n",
        ),
        (
            "src/data/b.ts",
            "import { a } from \"../views/a\";\nexport const b = a;\n",
        ),
        ("src/data/theme.css", "body { margin: 0; }\n"),
    ]
}

/// The mermaid node ids a render declares, keyed by visible label.
fn mermaid_ids(out: &str) -> BTreeMap<String, String> {
    let mut ids = BTreeMap::new();
    for line in out.lines() {
        let line = line.trim();
        let Some((id, rest)) = line.split_once('[') else {
            continue;
        };
        let Some(label) = rest.strip_prefix("\"").and_then(|r| r.strip_suffix("\"]")) else {
            continue;
        };
        ids.insert(label.to_string(), id.to_string());
    }
    ids
}

/// Every mermaid arrow of a render as (from label, to label, marked).
fn mermaid_arrows(out: &str) -> Vec<(String, String, bool)> {
    let ids = mermaid_ids(out);
    let by_id: BTreeMap<&str, &str> = ids.iter().map(|(l, i)| (i.as_str(), l.as_str())).collect();
    let label = |id: &str| by_id.get(id).copied().unwrap_or(id).to_string();
    let mut arrows = Vec::new();
    for line in out.lines() {
        let line = line.trim();
        if let Some((from, to)) = line.split_once(" --> ") {
            arrows.push((label(from), label(to), false));
        } else if let Some((from, to)) = line.split_once(" -.->|violates spec| ") {
            arrows.push((label(from), label(to), true));
        }
    }
    arrows
}

/// Scenario "file-grained rendering is opt-in and marked through the seam":
/// the files tier adds a cluster of territory files with first-party file
/// arrows on the spec source under `--module`.
#[test]
fn files_tier_renders_territory_files_and_file_edges() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());
    let render = tree.run(&["diagram", ".", "--module", "ui", "--tiers", "files"]);
    assert_eq!(render.status.code(), Some(0), "{}", common::stderr(&render));
    let out = common::stdout(&render);
    assert!(out.contains("subgraph files"), "a files cluster appears:\n{out}");
    for path in ["src/views/a.ts", "src/data/b.ts", "src/data/theme.css"] {
        assert!(
            out.contains(&format!("[\"{path}\"]")),
            "file vertex by its repo-relative path with extension: {path}\n{out}"
        );
    }
}

/// M13's proving cell: the dashed marks agree verdict-for-verdict with
/// `verify`'s cycle findings — the seam is the only verdict source. The
/// render exits 0 (a diagram never adjudicates); `verify` reds exit 1.
#[test]
fn files_tier_marks_agree_verdict_for_verdict_with_verify() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());

    let verify = tree.run(&["verify", "."]);
    assert_eq!(verify.status.code(), Some(1), "verify reds the cycle");
    let line = common::stdout(&verify)
        .lines()
        .chain(common::stderr(&verify).lines())
        .find(|line| line.trim_start().starts_with("cycle: "))
        .expect("verify reports the file cycle")
        .trim()
        .to_string();
    let path = line
        .trim_start_matches("cycle: ")
        .split_once(": ")
        .map(|(_, rest)| rest)
        .unwrap_or(line.trim_start_matches("cycle: "));
    let nodes: Vec<&str> = path.split(" -> ").collect();
    let mut expected: Vec<(String, String)> = Vec::new();
    for pair in nodes.windows(2) {
        expected.push((pair[0].to_string(), pair[1].to_string()));
    }

    let render = tree.run(&["diagram", ".", "--module", "ui", "--tiers", "files"]);
    assert_eq!(
        render.status.code(),
        Some(0),
        "the diagram renders the verdict, it does not adjudicate: {}",
        common::stderr(&render)
    );
    let out = common::stdout(&render);
    let arrows = mermaid_arrows(&out);
    for (from, to) in &expected {
        assert!(
            arrows.iter().any(|(f, t, marked)| f == from && t == to && *marked),
            "the pair {from} -> {to} verify reddens renders dashed `violates spec`:\n{out}"
        );
    }
    let marked_count = arrows.iter().filter(|(_, _, marked)| *marked).count();
    assert_eq!(
        marked_count,
        expected.len(),
        "exactly the verdicts verify reddens are marked — no diagram-side re-derivation:\n{out}"
    );

    // The green twin: a tree without a cycle renders solid arrows and a
    // green verify — verdict agreement in both directions.
    let clean = ts_tree(
        CYCLE_SPEC,
        &[(
            "src/views/a.ts",
            "import { b } from \"../data/b\";\nexport const a = b;\n",
        ), ("src/data/b.ts", "export const b = 1;\n")],
    );
    let verify_clean = clean.run(&["verify", "."]);
    assert_eq!(verify_clean.status.code(), Some(0), "the tree is clean");
    let render_clean = clean.run(&["diagram", ".", "--module", "ui", "--tiers", "files"]);
    assert_eq!(render_clean.status.code(), Some(0));
    let out = common::stdout(&render_clean);
    assert!(
        mermaid_arrows(&out)
            .iter()
            .all(|(_, _, marked)| !marked),
        "a green verdict renders no marks:\n{out}"
    );
}

/// R2-6/R3-3: the asset vertex joins the cluster with its incoming arrow
/// only; a graph fact — an asset imports nothing — keeps it out of every
/// cycle's member set, so its arrows never mark.
#[test]
fn asset_vertex_renders_incoming_only_and_never_cycle_marked() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());
    let render = tree.run(&["diagram", ".", "--module", "ui", "--tiers", "files"]);
    assert_eq!(render.status.code(), Some(0), "{}", common::stderr(&render));
    let out = common::stdout(&render);
    let arrows = mermaid_arrows(&out);
    let incoming: Vec<_> = arrows
        .iter()
        .filter(|(_, to, _)| to == "src/data/theme.css")
        .collect();
    assert!(
        !incoming.is_empty() && incoming.iter().all(|(_, _, marked)| !marked),
        "the asset carries incoming arrows only and no mark:\n{out}"
    );
    assert!(
        arrows.iter().all(|(from, _, _)| from != "src/data/theme.css"),
        "the asset imports nothing — no outgoing arrow:\n{out}"
    );
}

/// R3-8: the extended territory refusal — `--tiers files` with
/// `--source scan` is a usage error (exit 64) naming the rule.
#[test]
fn files_tier_with_scan_source_is_refused() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());
    let scan = tree.run(&["scan", "--output", "model.json"]);
    assert_eq!(scan.status.code(), Some(0), "{}", common::stderr(&scan));
    let refused =
        tree.run(&["diagram", "--source", "scan", "model.json", "--tiers", "files"]);
    assert_eq!(refused.status.code(), Some(64));
    let err = common::stderr(&refused);
    // The registered canonical bytes, stated verbatim (claim-registry weld:
    // files-tier-scan-refusal).
    assert!(err.contains(FILES_SCAN_REFUSAL), "names the rule broken: {err}");
    assert!(common::stdout(&refused).is_empty(), "no partial diagram");
}

/// M13's size law: `--tiers files` without a `--module` or `--modules-dir`
/// territory is a usage error (exit 64) naming the rule.
#[test]
fn files_tier_without_territory_scope_is_refused() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());
    let refused = tree.run(&["diagram", ".", "--tiers", "files"]);
    assert_eq!(refused.status.code(), Some(64));
    let err = common::stderr(&refused);
    assert!(
        err.contains(
            "--tiers files requires --module or --modules-dir — the files tier renders a claim's territory, never the whole tree"
        ),
        "names the territory flags the rule requires: {err}"
    );
    assert!(common::stdout(&refused).is_empty(), "no partial diagram");

    // A scan-only value named on the spec source keeps the shipped refusal.
    let shipped = tree.run(&["diagram", ".", "--tiers", "units"]);
    assert_eq!(shipped.status.code(), Some(64));
    assert!(
        common::stderr(&shipped).contains("--tiers requires --source scan"),
        "the shipped refusal is unchanged: {}",
        common::stderr(&shipped)
    );
}

/// The unknown-value refusal names the extended vocabulary.
#[test]
fn unknown_tier_value_names_the_extended_vocabulary() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());
    let scan = tree.run(&["scan", "--output", "model.json"]);
    assert_eq!(scan.status.code(), Some(0));
    let refused =
        tree.run(&["diagram", "--source", "scan", "model.json", "--tiers", "bogus"]);
    assert_eq!(refused.status.code(), Some(64));
    let err = common::stderr(&refused);
    assert!(
        err.contains("units") && err.contains("modules") && err.contains("files"),
        "the supported-tier list names files beside units and modules: {err}"
    );
}

/// D7's vacuity law on the render surface after the rust facts landed: a
/// language whose file facts ARE emitted renders what the facts show — the
/// note stays reserved for not-emitted drivers. With a claim whose patterns
/// address the files (a path glob), the cluster carries the real rows; the
/// vacuity note never prints for rust again.
#[test]
fn files_tier_on_a_rust_spec_renders_the_serialized_rows() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write("src/lib.rs", "pub mod core;\npub mod ui;\n");
    tree.write("src/core.rs", "pub fn core() {}\n");
    tree.write("src/ui.rs", "use crate::core;\npub fn ui() { core::core(); }\n");
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
    );

    let render = tree.run(&["diagram", ".", "--module", "app", "--tiers", "files"]);
    assert_eq!(render.status.code(), Some(0), "{}", common::stderr(&render));
    let out = common::stdout(&render);
    assert!(out.contains("subgraph files"), "the rows render a cluster:\n{out}");
    for path in ["src/lib.rs", "src/core.rs", "src/ui.rs"] {
        assert!(
            out.contains(&format!("[\"{path}\"]")),
            "the serialized file rows name {path}:\n{out}"
        );
    }
    let arrows = mermaid_arrows(&out);
    assert!(
        arrows
            .iter()
            .any(|(from, to, marked)| from == "src/ui.rs" && to == "src/core.rs" && !*marked),
        "the use row renders as a solid file arrow:\n{out}"
    );
    let err = common::stderr(&render);
    assert!(
        !err.contains("note:"),
        "rust facts are emitted — no vacuity note: {err}"
    );
}

/// The stated absence on the render surface: rust rows are unit-internal,
/// so a cross-unit dependency contributes no file vertex, and a tree whose
/// unit has a single file serializes no rows — the render stays the module
/// view's bytes with no cluster and no note (granular row, no facts to
/// draw). A not-emitted driver (go) still gets the vacuity note.
#[test]
fn files_tier_states_absence_by_render_not_fabrication() {
    // Rust: two single-file members, one dependency — unit granular only.
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\n",
    );
    for name in ["a", "b"] {
        let deps = if name == "a" {
            "[dependencies]\nb = { path = \"../b\" }\n"
        } else {
            ""
        };
        tree.write(
            &format!("crates/{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n{deps}"
            ),
        );
        tree.write(&format!("crates/{name}/src/lib.rs"), "pub fn f() {}\n");
    }
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
    );

    let plain = tree.run(&["diagram", ".", "--module", "a"]);
    assert_eq!(plain.status.code(), Some(0), "{}", common::stderr(&plain));
    let flagged = tree.run(&["diagram", ".", "--module", "a", "--tiers", "files"]);
    assert_eq!(flagged.status.code(), Some(0), "{}", common::stderr(&flagged));
    let out = common::stdout(&flagged);
    assert!(
        !out.contains("subgraph files"),
        "a unit-granular dependency fabricates no file vertices:\n{out}"
    );
    assert_eq!(
        out,
        common::stdout(&plain),
        "the render body stays the module view's bytes"
    );
    let err = common::stderr(&flagged);
    assert!(
        !err.contains("note:"),
        "the rust row reads granular — an empty render states no vacuity note: {err}"
    );

    // Go: the row still reads not-emitted — the note states the vacuity.
    let go_tree = Fixture::new();
    go_tree.write("go.mod", "module demo\n\ngo 1.22\n");
    go_tree.write("main.go", "package main\n\nfunc main() {}\n");
    go_tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
    );
    let go_plain = go_tree.run(&["diagram", ".", "--module", "demo"]);
    assert_eq!(go_plain.status.code(), Some(0), "{}", common::stderr(&go_plain));
    let go_flagged = go_tree.run(&["diagram", ".", "--module", "demo", "--tiers", "files"]);
    assert_eq!(
        go_flagged.status.code(),
        Some(0),
        "{}",
        common::stderr(&go_flagged)
    );
    assert!(
        !common::stdout(&go_flagged).contains("subgraph files"),
        "go file facts are not emitted — no file vertices are fabricated"
    );
    let err = common::stderr(&go_flagged);
    assert!(
        err.contains("note:") && err.contains("the files tier renders no file vertices"),
        "the not-emitted vacuity is stated, not silence: {err}"
    );
}

/// The byte-identity law on the module view: without the `files` value the
/// render carries no cluster — the shipped bytes never moved.
#[test]
fn module_render_without_files_value_is_byte_unchanged() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());
    let plain = tree.run(&["diagram", ".", "--module", "ui"]);
    assert_eq!(plain.status.code(), Some(0), "{}", common::stderr(&plain));
    let out = common::stdout(&plain);
    assert!(
        !out.contains("subgraph files") && !out.contains("src/views/a.ts"),
        "no files tier without the flag — the default bytes are the shipped ones:\n{out}"
    );
}

/// The `--modules-dir` batch renders the cluster into every claim-named
/// file and `--check` gates those bytes.
#[test]
fn modules_dir_batch_renders_clusters_and_check_gates_them() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());
    let batch = tree.run(&[
        "diagram",
        ".",
        "--modules-dir",
        "diagrams",
        "--tiers",
        "files",
    ]);
    assert_eq!(batch.status.code(), Some(0), "{}", common::stderr(&batch));
    let written = common::stdout(&batch);
    assert!(
        written.contains("wrote diagrams/ui.mmd"),
        "the batch echoes the wrote line: {written}"
    );
    let body = tree.read("diagrams/ui.mmd");
    assert!(
        body.contains("subgraph files") && body.contains("[\"src/views/a.ts\"]"),
        "the batch file carries the cluster:\n{body}"
    );

    let fresh = tree.run(&[
        "diagram",
        ".",
        "--modules-dir",
        "diagrams",
        "--tiers",
        "files",
        "--check",
    ]);
    assert_eq!(
        fresh.status.code(),
        Some(0),
        "the flagged render is up to date: {}",
        common::stderr(&fresh)
    );

    // The same batch WITHOUT the flag is stale against the flagged bytes.
    let stale = tree.run(&["diagram", ".", "--modules-dir", "diagrams", "--check"]);
    assert_ne!(
        stale.status.code(),
        Some(0),
        "the cluster bytes are not the default bytes — check flags the drift"
    );
}

/// A cycle only a `warning`-severity constraint reports stays solid in the
/// render — the mark vocabulary follows the red verdicts, not the warnings.
#[test]
fn warning_severity_cycle_stays_solid_in_the_files_render() {
    let tree = ts_tree(
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\nseverity = \"warning\"\n",
        &cycle_files(),
    );
    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "a warning-severity cycle does not redden: {}",
        common::stdout(&verify)
    );
    let render = tree.run(&["diagram", ".", "--module", "ui", "--tiers", "files"]);
    assert_eq!(render.status.code(), Some(0), "{}", common::stderr(&render));
    let out = common::stdout(&render);
    assert!(
        mermaid_arrows(&out)
            .iter()
            .all(|(_, _, marked)| !marked),
        "warning verdicts render no `violates spec` marks:\n{out}"
    );
}

/// The plantuml surface renders the cluster with the shipped violation
/// grammar (quoted components, red labelled arrows).
#[test]
fn files_tier_renders_plantuml_with_the_violation_grammar() {
    let tree = ts_tree(CYCLE_SPEC, &cycle_files());
    let render = tree.run(&[
        "diagram",
        ".",
        "--module",
        "ui",
        "--tiers",
        "files",
        "--format",
        "plantuml",
    ]);
    assert_eq!(render.status.code(), Some(0), "{}", common::stderr(&render));
    let out = common::stdout(&render);
    assert!(out.contains("package files {"), "the cluster appears:\n{out}");
    assert!(
        out.contains("component \"src/views/a.ts\""),
        "file vertices are quoted components:\n{out}"
    );
    assert!(
        out.contains("\"src/views/a.ts\" -[#red]-> \"src/data/b.ts\" : violates spec"),
        "the marked arrow uses the shipped violation grammar:\n{out}"
    );
}
