//! ts detection cells of the ts-driver design record (scenario "a ts tree
//! gets a driver" — the detection half).
//!
//! The `ts` arm is the FALLBACK driver: detection runs behind the mechanical
//! exclusion set (`node_modules`, `dist`, `target`, `.git`, vendor-style
//! directories), then selects in the fixed chain rust > csharp > go > ts.
//! These cells pin the chain and the exclusion set at the CLI surface; the
//! ts EXTRACTION cells live in `ts_scan.rs` with the extraction arm.

mod common;

use common::Fixture;

fn model_language(out: &std::process::Output) -> String {
    let json: serde_json::Value =
        serde_json::from_str(&common::stdout(out)).expect("stdout must be JSON");
    json["language"].as_str().expect("language field").to_string()
}

#[test]
fn two_root_markers_without_a_pin_are_ambiguous() {
    // DET-ROOT-AMBIGUOUS (the re-anchored both-orders cell): root markers
    // decide, a collision names EVERY marker file, and write order on disk
    // changes nothing.
    let a = Fixture::new();
    a.write("package.json", "{}\n");
    a.write("Cargo.toml", "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    a.write("src/lib.rs", "pub fn f() {}\n");
    let out = a.run(&["scan", "."]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", common::stderr(&out));
    assert!(
        common::stderr(&out).contains(
            "ambiguous root markers: Cargo.toml (rust), package.json (ts) — pin [project] language or scan a single-language root"
        ),
        "stderr: {}",
        common::stderr(&out)
    );

    let b = Fixture::new();
    b.write("Cargo.toml", "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    b.write("src/lib.rs", "pub fn f() {}\n");
    b.write("package.json", "{}\n");
    let out = b.run(&["scan", "."]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", common::stderr(&out));
    assert!(
        common::stderr(&out).contains("ambiguous root markers: Cargo.toml (rust), package.json (ts)"),
        "stderr: {}",
        common::stderr(&out)
    );
}

#[test]
fn an_ambiguous_root_with_a_pinned_spec_runs_the_pinned_driver() {
    // DET-ROOT-AMBIGUOUS-PINNED: the pin resolves the collision on both
    // verbs; the D14 agreement counts still apply to the foreign sources.
    let tree = Fixture::new();
    tree.write("Cargo.toml", "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    tree.write("src/lib.rs", "pub fn f() {}\n");
    tree.write("package.json", "{}\n");
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"tool\"\nmatches = { units = [\"tool\"] }\n",
    );
    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "scan failed: {}", common::stderr(&scan));
    assert_eq!(model_language(&scan), "rust");
    let verify = tree.run(&["verify", "."]);
    assert!(verify.status.success(), "verify failed: {}", common::stderr(&verify));
}

#[test]
fn a_pin_without_its_root_marker_reds_with_the_mismatch_prefix() {
    // DET-PIN-NO-ROOT-MARKER: ts pin over a Cargo.toml-only root — the
    // branch keeps the `language mismatch:` byte prefix and names Cargo.toml.
    let tree = Fixture::new();
    tree.write("Cargo.toml", "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    tree.write("src/lib.rs", "pub fn f() {}\n");
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"app\"\nmatches = { paths = [\"src/**\"] }\n",
    );
    let out = tree.run(&["verify", "."]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", common::stderr(&out));
    let line = common::stderr(&out)
        .lines()
        .find(|line| line.contains("language mismatch:"))
        .expect("mismatch line")
        .to_string();
    assert!(
        line.contains("no root marker package.json for the declared language, found: Cargo.toml (rust)"),
        "line: {line}"
    );
}

#[test]
fn a_root_marker_selects_its_driver_and_a_stray_source_cannot_outrank_it() {
    // DET-ROOT-PIN-TS + DET-STRAY-FLIP: root package.json + nested .rs —
    // the ts driver runs (the flipped `rust_outranks_the_ts_fallback_in_both_orders`);
    // the rust driver could never extract this tree.
    let tree = Fixture::new();
    tree.write("package.json", "{ \"name\": \"ui\" }\n");
    tree.write("src/app.ts", "export const a = 1;\n");
    tree.write("tools/helper.rs", "pub fn h() {}\n");
    let out = tree.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", common::stderr(&out));
    assert_eq!(model_language(&out), "ts");
}

#[test]
fn a_markerless_root_keeps_the_chain_fallback() {
    // DET-FALLBACK-CHAIN: the no-walk-down truth stays honest — no root
    // marker, so the unchanged chain answers from below the root.
    let tree = Fixture::new();
    tree.write("web/package.json", "{}\n");
    tree.write("web/app.ts", "export const a = 1;\n");
    let out = tree.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", common::stderr(&out));
    assert_eq!(model_language(&out), "ts");
    let empty = Fixture::new();
    empty.write("README.md", "nothing here\n");
    let out = empty.run(&["scan", "."]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        common::stderr(&out).contains("no supported-language sources found under:"),
        "stderr: {}",
        common::stderr(&out)
    );
}

#[test]
fn a_marker_under_an_excluded_tree_stays_silent() {
    // DET-NESTED-EXCLUDED-SILENT: the DETECT_EXCLUDE_DIRS scoping fix —
    // a dependency tree's marker earns no report line.
    let tree = Fixture::new();
    tree.write("go.mod", "module example.com/tool\n\ngo 1.21\n");
    tree.write("main.go", "package main\n\nfunc main() {}\n");
    tree.write("node_modules/pkg/package.json", "{}\n");
    let out = tree.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", common::stderr(&out));
    assert!(
        !common::stderr(&out).contains("nested foreign marker"),
        "stderr: {}",
        common::stderr(&out)
    );
}

#[test]
fn go_markers_outrank_ts_plugin_sources() {
    // The opencode-plugin-shaped hazard (re-anchored): a go tree with a .ts
    // subtree stays go — the root go.mod decides — AND the nested foreign
    // marker earns its report-only line.
    let tree = Fixture::new();
    tree.write("go.mod", "module example.com/tool\n\ngo 1.21\n");
    tree.write("cmd/tool/main.go", "package main\n\nfunc main() {}\n");
    tree.write("opencode-plugin/package.json", "{\"name\": \"plugin\"}\n");
    tree.write("opencode-plugin/index.ts", "export const p = 1;\n");
    let out = tree.run(&["scan", "."]);
    assert!(out.status.success(), "scan failed: {}", common::stderr(&out));
    assert_eq!(model_language(&out), "go");
    assert!(
        common::stderr(&out)
            .contains("nested foreign marker: opencode-plugin/package.json (ts) — not modelled"),
        "stderr: {}",
        common::stderr(&out)
    );
}

#[test]
fn dependency_tree_markers_alone_select_no_driver() {
    // node_modules is behind the exclusion set: its package.json and its .js
    // sources are not first-party markers; the run states no-driver loudly.
    let tree = Fixture::new();
    tree.write("node_modules/left-pad/package.json", "{\"name\": \"left-pad\"}\n");
    tree.write("node_modules/left-pad/index.js", "module.exports = 1;\n");
    let out = tree.run(&["scan", "."]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        common::stderr(&out)
            .contains("no supported-language sources found under:"),
        "stderr: {}",
        common::stderr(&out)
    );
}

#[test]
fn build_output_markers_do_not_outrank_the_fallback() {
    // .rs under target/ is build output: excluded. What survives is the
    // package.json marker, so the tree detects the ts fallback; the scan-
    // able set is empty and the extraction arm states that loud.
    let tree = Fixture::new();
    tree.write("target/gen/out.rs", "");
    tree.write("package.json", "{}\n");
    let out = tree.run(&["scan", "."]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        common::stderr(&out).contains("no ts sources found"),
        "stderr: {}",
        common::stderr(&out)
    );
}
