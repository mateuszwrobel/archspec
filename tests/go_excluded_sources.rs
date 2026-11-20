//! GitHub issue #60 items 2-3 — the GO arm: a declared `excluded_sources`
//! entry adjudicates at the `.go` SOURCE FILE walk. A pruned file contributes
//! no fact (no unit, no edge, no module edge, no external attribution, no
//! literal candidate) and states itself with exactly one
//! `excluded source: <path> — excluded_sources` census line, sorted and never
//! silent (the issue #39 law). The mechanical gate — dot-entries, `vendor`,
//! `testdata` and the `*_test.go` test tier — stays the FIRST door and says
//! nothing: mechanically excluded files were never unit sources. Go derives
//! units FROM files, so a package whose every file is pruned yields NO unit
//! at all (the mirror of the rust manifest-derived rule, and unlike csharp
//! there is no go project arm — the file line is the only census line).
//! Liveness is unchanged and driver-independent (`spec.rs`): a pruned file
//! still counts as a MATCH, so an entry that prunes files never reads dead.
//! The candidate feed walks the same package map again — a pruned file states
//! itself once, with and without a declaring `[[pattern_pair]]`. Non-declaring
//! trees stay byte-identical.

mod common;

use common::Fixture;
use serde_json::Value;

/// The census forms pinned by the claim registry (`csharp-excluded-sources`,
/// owner surface: `help diagnostics`). The dedicated guard for those
/// placeholder bytes is this file: the exact-bytes cells below render them.
#[allow(dead_code)]
const CENSUS_FORMS: [&str; 1] = ["excluded source: <path> — excluded_sources"];

fn go_spec(entries: &str) -> String {
    format!(
        "[project]\nlanguage = \"go\"\n{entries}\n[[module]]\nname = \"pkg\"\nmatches = {{ units = [\"example.dev/demo/pkg\"] }}\n"
    )
}

/// The single-module fixture: `pkg` and `bus` kept, `external` and `allprune`
/// each a one-file package named by an entry (`allprune`'s ONLY file is
/// pruned — the fully-pruned-package shape).
fn go_tree(extra_entries: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write("go.mod", "module example.dev/demo\n\ngo 1.22\n");
    tree.write(
        "pkg/a.go",
        "package pkg\n\nimport \"example.dev/demo/bus\"\n\nvar _ = bus.Bus\n",
    );
    tree.write("bus/bus.go", "package bus\n\ntype Bus struct{}\n");
    tree.write(
        "external/ext.go",
        "package external\n\nimport \"example.dev/demo/bus\"\n\nvar _ = bus.Bus\n",
    );
    tree.write(
        "allprune/dead.go",
        "package allprune\n\nconst Name = \"dead\"\n",
    );
    tree.write(
        "architecture.spec.toml",
        &go_spec(extra_entries),
    );
    tree
}

fn entries_block(entries: &[&str]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    format!(
        "\nexcluded_sources = [\n  {},\n]\n",
        entries.join(",\n  ")
    )
}

fn file_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("excluded source: "))
        .map(str::to_string)
        .collect()
}

fn liveness_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("excluded source matched nothing:"))
        .map(str::to_string)
        .collect()
}

fn scan_model(tree: &Fixture) -> Value {
    let out = tree.run(&["scan", "--output", "model.json"]);
    assert!(
        out.status.success(),
        "scan must succeed:\n{}",
        common::stderr(&out)
    );
    serde_json::from_str(&tree.read("model.json")).expect("model.json parses")
}

fn unit_names(model: &Value) -> Vec<String> {
    model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name").to_string())
        .collect()
}

fn edge_pairs(model: &Value) -> Vec<(String, String)> {
    model["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().expect("from").to_string(),
                edge["to"].as_str().expect("to").to_string(),
            )
        })
        .collect()
}

/// Cells A+B: a directory entry prunes whole packages. A package keeping one
/// file keeps its unit and its edges; a package whose ONLY file is pruned
/// yields NO unit at all (units are derived from files). Each pruned file
/// states itself once, sorted; no fact of either file survives.
#[test]
fn pruned_packages_state_each_file_once_and_a_fully_pruned_package_yields_no_unit() {
    let tree = go_tree(&entries_block(&[
        "{ path = \"external\", reason = \"vendored external code\" }",
        "{ path = \"allprune\", reason = \"generated package\" }",
    ]));
    let out = tree.run(&["scan"]);
    assert_eq!(
        file_lines(&out),
        vec![
            "excluded source: allprune/dead.go — excluded_sources",
            "excluded source: external/ext.go — excluded_sources",
        ],
        "each pruned file states itself once, sorted:\n{}",
        common::stderr(&out)
    );
    let model = scan_model(&tree);
    let units = unit_names(&model);
    assert!(
        units.contains(&"example.dev/demo/pkg".to_string()),
        "the kept package keeps its unit:\n{units:?}"
    );
    assert!(
        !units.iter().any(|name| name.contains("allprune")),
        "a fully pruned package yields NO unit (go units come from files):\n{units:?}"
    );
    assert!(
        !units.iter().any(|name| name.contains("external")),
        "the pruned-only-file package yields no unit either:\n{units:?}"
    );
    let edges = edge_pairs(&model);
    assert!(
        edges
            .iter()
            .any(|(from, to)| from == "example.dev/demo/pkg" && to == "example.dev/demo/bus"),
        "the kept import edge survives:\n{edges:?}"
    );
    assert!(
        !edges.iter().any(|(from, _)| from.contains("external")),
        "the pruned file's import fact is gone:\n{edges:?}"
    );
    let body = serde_json::to_string(&model).unwrap_or_default();
    assert!(
        !body.contains("allprune") && !body.contains("ext.go"),
        "no model row references the pruned files:\n{body}"
    );
}

/// Component-wise matching: the entry `external` matches `external/ext.go`
/// and NEVER `externalapp/ext.go` — the neighbouring package stays a unit
/// with its facts and states nothing.
#[test]
fn matching_is_component_wise_neighbour_packages_stay_whole() {
    let tree = go_tree(&entries_block(&[
        "{ path = \"external\", reason = \"vendored external code\" }"
    ]));
    tree.write(
        "externalapp/app.go",
        "package externalapp\n\nimport \"example.dev/demo/bus\"\n\nvar _ = bus.Bus\n",
    );
    let out = tree.run(&["scan"]);
    assert_eq!(
        file_lines(&out),
        vec!["excluded source: external/ext.go — excluded_sources"],
        "only the component-wise match is pruned:\n{}",
        common::stderr(&out)
    );
    let model = scan_model(&tree);
    let units = unit_names(&model);
    assert!(
        units.contains(&"example.dev/demo/externalapp".to_string()),
        "the neighbour package stays a unit:\n{units:?}"
    );
    assert!(
        edge_pairs(&model)
            .iter()
            .any(|(from, _)| from == "example.dev/demo/externalapp"),
        "the neighbour package keeps its edge facts"
    );
}

/// Cell C (go half of the dedupe law): the pattern candidate feed walks the
/// same package map again — the pruned file still states itself EXACTLY once
/// per run, with and without a declaring `[[pattern_pair]]` for go, and its
/// literal contributes no candidate.
#[test]
fn candidate_feed_walk_states_one_line_per_file_not_two() {
    let build = |pattern: &str| {
        let tree = Fixture::new();
        tree.write("go.mod", "module example.dev/demo\n\ngo 1.22\n");
        tree.write(
            "pkg/a.go",
            "package pkg\n\nfunc Publish() {}\n\nfunc Go() { Publish(\"order-created\") }\n",
        );
        tree.write(
            "external/ext.go",
            "package external\n\nfunc Publish() {}\n\nfunc Go() { Publish(\"external-event\") }\n",
        );
        tree.write(
            "architecture.spec.toml",
            &format!(
                "[project]\nlanguage = \"go\"\n\nexcluded_sources = [ {{ path = \"external\", reason = \"vendored external code\" }} ]\n[[module]]\nname = \"pkg\"\nmatches = {{ units = [\"example.dev/demo/pkg\"] }}\n{pattern}"
            ),
        );
        tree
    };
    let pattern = "\n[[pattern_pair]]\nid = \"demo-bus\"\n\n[pattern_pair.emit]\nlanguage = \"go\"\nrole = \"publish\"\ncall = \"Publish\"\narg = 0\n\n[pattern_pair.subscribe]\nlanguage = \"go\"\nproperty = \"On\"\n";
    for (label, spec_pattern) in [("no pattern", ""), ("pattern feed", pattern)] {
        let tree = build(spec_pattern);
        for args in [vec!["scan"], vec!["verify", "."]] {
            let out = tree.run(&args);
            assert_eq!(
                file_lines(&out),
                vec!["excluded source: external/ext.go — excluded_sources"],
                "exactly one census line per run ({label}, {}):\n{}",
                args[0],
                common::stderr(&out)
            );
        }
    }
    let tree = build(pattern);
    let model = scan_model(&tree);
    assert!(
        !serde_json::to_string(&model).unwrap().contains("external-event"),
        "the pruned file's literal is not a candidate:\n{model}"
    );
}

/// Cell D: liveness unchanged — an entry pruning files is ALIVE and prints
/// no advisory; a dead entry states itself once while the alive one stays
/// silent. `*_test.go` and `vendor` sit behind the mechanical FIRST door: a
/// test file under an excluded directory states no file line (the mechanical
/// skip fires first and the file was never a unit source).
#[test]
fn liveness_survives_the_prune_and_the_test_tier_states_nothing() {
    let tree = go_tree(&entries_block(&[
        "{ path = \"external\", reason = \"vendored external code\" }",
        "{ path = \"tools/absent\", reason = \"nothing there\" }",
    ]));
    tree.write("external/ext_test.go", "package external\n\nfunc TestX() {}\n");
    for args in [vec!["scan"], vec!["verify", "."]] {
        let out = tree.run(&args);
        assert_eq!(
            file_lines(&out),
            vec!["excluded source: external/ext.go — excluded_sources"],
            "the pruned production file states itself; the test file never \
             reaches the declared door ({}):\n{}",
            args[0],
            common::stderr(&out)
        );
        assert_eq!(
            liveness_lines(&out),
            vec!["excluded source matched nothing: tools/absent — nothing there"],
            "the pruning entry is alive (pruned files count as matches):\n{}",
            common::stderr(&out)
        );
    }
}

/// Cell G (go half): the mechanical `vendor` door is FIRST at the driver —
/// a file under `vendor` never reaches the declared gate and states no
/// census line. Liveness keeps the VENDORED class narrowed (round-6 P-VENDOR
/// R11): the driver-independent walk still COUNTS files under `vendor`, so
/// the entry reads ALIVE, unlike a `target`-class entry on the rust side.
#[test]
fn mechanical_vendor_door_states_no_line_and_the_vendored_class_stays_narrowed() {
    let tree = go_tree(&entries_block(&[
        "{ path = \"vendor/lib\", reason = \"vendored dependencies\" }"
    ]));
    tree.write("vendor/lib/x.go", "package lib\n\nconst Name = \"x\"\n");
    for args in [vec!["scan"], vec!["verify", "."]] {
        let out = tree.run(&args);
        assert!(
            file_lines(&out).is_empty(),
            "a file under the mechanical `vendor` door states NO declared-exclusion line ({}):\n{}",
            args[0],
            common::stderr(&out)
        );
        assert!(
            liveness_lines(&out).is_empty(),
            "the VENDORED class stays narrowed for liveness (R11): the file \
             under `vendor` keeps the entry alive ({}):\n{}",
            args[0],
            common::stderr(&out)
        );
    }
}

/// Cell F: the census is report-only — `model.json` carries no census field,
/// a non-declaring tree prints no census line, and a declaring entry that
/// matches nothing moves no machine byte (the liveness line is spec-side,
/// not driver-side).
#[test]
fn census_is_report_only_and_declaring_nothing_moves_no_machine_byte() {
    let declaring = go_tree(&entries_block(&[
        "{ path = \"external\", reason = \"vendored external code\" }"
    ]));
    let out = declaring.run(&["scan", "--output", "model.json"]);
    assert_eq!(file_lines(&out).len(), 1, "the pruned file states itself");
    let raw = declaring.read("model.json");
    assert!(
        !raw.contains("report_only_lines") && !raw.contains("excluded source"),
        "the serialized model carries no census field (serde-skip law):\n{raw}"
    );
    // Byte identity: a non-declaring tree and a tree declaring an entry that
    // matches no file produce the SAME model.json (the machine body is the
    // same bytes; only the spec-side liveness line differs).
    let plain = go_tree("");
    let matched_elsewhere = go_tree(&entries_block(&[
        "{ path = \"nomatch\", reason = \"matches nothing\" }"
    ]));
    let plain_out = plain.run(&["scan", "--output", "model.json"]);
    assert!(
        file_lines(&plain_out).is_empty() && liveness_lines(&plain_out).is_empty(),
        "a tree declaring nothing prints nothing:\n{}",
        common::stderr(&plain_out)
    );
    let plain_model = plain.read("model.json");
    let dead_model_out = matched_elsewhere.run(&["scan", "--output", "model.json"]);
    assert!(
        file_lines(&dead_model_out).is_empty(),
        "an entry pruning no file states no file line:\n{}",
        common::stderr(&dead_model_out)
    );
    assert_eq!(
        plain_model,
        matched_elsewhere.read("model.json"),
        "a declaring entry that prunes nothing moves zero machine bytes"
    );
}
