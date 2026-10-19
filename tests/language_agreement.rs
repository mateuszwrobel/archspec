//! C4 cells of the ts-driver design record — the D14 AGREEMENT law
//! (scenario half: the language-mismatch guard). The spec-declared language
//! is the truth of a declared tree; any source file outside the declared
//! language — after the mechanical exclusion set and the `excluded_sources`
//! entries — fails the run loud on BOTH verbs with the counts of both
//! languages, exit 1, before any verdict or artifact.

mod common;

use common::Fixture;

fn go_tree(spec_extra: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write("go.mod", "module example.com/tool\n\ngo 1.21\n");
    tree.write("cmd/tool/main.go", "package main\n\nfunc main() {}\n");
    tree.write("opencode-plugin/package.json", "{\"name\": \"plugin\"}\n");
    tree.write("opencode-plugin/index.ts", "export const p = 1;\n");
    tree.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"go\"\n{spec_extra}\n[[module]]\nname = \"tool\"\nmatches = {{ units = [\"example.com/tool/cmd/tool\"] }}\n"
        ),
    );
    tree
}

fn mismatch_line(out: &std::process::Output) -> Option<String> {
    common::stderr(out)
        .lines()
        .find(|line| line.contains("language mismatch:"))
        .map(str::to_string)
}

#[test]
fn ts_subtree_under_a_go_tree_fails_both_verbs_loud() {
    // The opencode-plugin-shaped cell: go stays the detected driver, so
    // detection alone would pass — it is the AGREEMENT check that reddens
    // the un-excluded foreign subtree on both verbs, both models.
    let tree = go_tree("");

    let scan = tree.run(&["scan", "."]);
    assert_eq!(scan.status.code(), Some(1), "the mismatch class is untagged — exit 1");
    let line = mismatch_line(&scan).expect("scan must fail loud with the mismatch line");
    assert!(line.contains("go vs ts"), "declared vs detected: {line}");
    assert!(line.contains("go: 1 files"), "counts of the declared language: {line}");
    assert!(line.contains("ts: 1 files"), "counts of the other language: {line}");
    // DET-NESTED-FOREIGN-REPORT (re-anchored): the nested marker earns its
    // report-only line AND the sources keep failing loud — the marker
    // downgrade was rejected, only the marker line is report-only.
    assert!(
        common::stderr(&scan)
            .contains("nested foreign marker: opencode-plugin/package.json (ts) — not modelled"),
        "the red scan must also carry the report-only marker line: {}",
        common::stderr(&scan)
    );
    assert!(
        common::stdout(&scan).is_empty(),
        "a mismatching scan computes no model body on stdout"
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(verify.status.code(), Some(1));
    let line = mismatch_line(&verify).expect("verify must fail loud with the mismatch line");
    assert!(line.contains("go vs ts"), "declared vs detected: {line}");
    assert!(
        !common::stdout(&verify).contains("ok"),
        "a wrong-language analysis must not print a verdict: {}",
        common::stdout(&verify)
    );
}

#[test]
fn an_exclusion_with_a_reason_silences_the_subtree_on_both_verbs() {
    // The companion cell: the exclusion (or a restructure) lands before the
    // guard could red it — excluded by path with a reason means green.
    let tree = go_tree(
        "\nexcluded_sources = [ { path = \"opencode-plugin\", reason = \"plugin subtree in its own language\" } ]\n",
    );
    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "scan failed: {}", common::stderr(&scan));
    let verify = tree.run(&["verify", "."]);
    assert!(verify.status.success(), "verify failed: {}", common::stderr(&verify));
}

#[test]
fn dependency_trees_never_produce_an_agreement_finding() {
    // node_modules/package.json under a go tree and under a csharp tree:
    // the mechanical set already removes it — no finding, both verbs green.
    let go = Fixture::new();
    go.write("go.mod", "module example.com/tool\n");
    go.write("main.go", "package main\n");
    go.write("node_modules/pkg/package.json", "{}\n");
    go.write("node_modules/pkg/index.js", "module.exports = 1;\n");
    go.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"tool\"\nmatches = { paths = [\"**\"] }\n",
    );
    let scan = go.run(&["scan", "."]);
    assert!(scan.status.success(), "go scan failed: {}", common::stderr(&scan));
    assert!(mismatch_line(&scan).is_none());

    let cs = Fixture::new();
    cs.write("tool.csproj", "<Project></Project>\n");
    cs.write("Program.cs", "class P {}\n");
    cs.write("node_modules/pkg/package.json", "{}\n");
    cs.write("node_modules/pkg/index.js", "module.exports = 1;\n");
    cs.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"tool\"\nmatches = { units = [\"tool\"] }\n",
    );
    let scan = cs.run(&["scan", "."]);
    assert!(scan.status.success(), "csharp scan failed: {}", common::stderr(&scan));
    assert!(mismatch_line(&scan).is_none());
}

#[test]
fn a_tree_without_a_spec_keeps_chain_detection_and_checks_nothing() {
    // D14(2): without a spec at the root, the detected language remains the
    // truth exactly as shipped — a stray foreign file changes no verdict.
    let tree = Fixture::new();
    tree.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    tree.write("src/lib.rs", "pub fn f() {}\n");
    tree.write("web/asset.js", "export const a = 1;\n");
    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "specless scan failed: {}", common::stderr(&scan));
    assert!(mismatch_line(&scan).is_none());
}

#[test]
fn a_declared_ts_tree_with_go_files_names_the_pair_and_counts() {
    // The foreign language is named by the fixed chain order; the line
    // carries the counts of both languages.
    let tree = Fixture::new();
    tree.write("package.json", "{}\n");
    tree.write("src/app.ts", "export const a = 1;\n");
    tree.write("src/main.ts", "export const m = 1;\n");
    tree.write("tools/helper.go", "package tools\n");
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"app\"\nmatches = { paths = [\"src/**\"] }\n",
    );
    let scan = tree.run(&["scan", "."]);
    assert_eq!(scan.status.code(), Some(1));
    let line = mismatch_line(&scan).expect("mismatch line");
    assert!(line.contains("ts vs go"), "declared vs detected: {line}");
    assert!(line.contains("ts: 2 files"), "declared counts: {line}");
    assert!(line.contains("go: 1 files"), "detected counts: {line}");
}

#[test]
fn a_mismatch_writes_no_artifact_and_scan_check_reports_nothing_stale() {
    // The check is a precondition: the mismatching run creates no artifact.
    let tree = go_tree("");
    let out = tree.run(&["scan", ".", "--output", "model.json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        !tree.path("model.json").exists(),
        "a mismatching scan must write no artifact"
    );
}

#[test]
fn the_catalog_names_the_language_mismatch_category() {
    let fixture = Fixture::new();
    let out = fixture.run(&["help", "diagnostics"]);
    assert!(out.status.success());
    assert!(
        common::stdout(&out).contains("language mismatch"),
        "the every-emitted-category guard carries the new fail-loud class (D13/D14)"
    );
}

#[test]
fn the_catalog_names_the_root_marker_classes() {
    // Detection-record GC fan-out: the AMBIGUITY class, the pin-vs-root-
    // marker mismatch branch and the nested-marker report-only line all
    // live in the diagnostics catalog with their verbatim patterns.
    let fixture = Fixture::new();
    let out = fixture.run(&["help", "diagnostics"]);
    assert!(out.status.success());
    let out = common::stdout(&out);
    for pattern in [
        "ambiguous root markers: <path> (<language>), … — pin [project] language or scan a single-language root",
        "language mismatch: <declared> — no root marker <marker> for the declared language, found: <markers>",
        "nested foreign marker: <path> (<language>) — not modelled",
    ] {
        assert!(out.contains(pattern), "catalog must name the pattern {pattern:?}:\n{out}");
    }
}
