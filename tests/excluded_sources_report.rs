//! C3 cells of the ts-driver design record — the `[project] excluded_sources`
//! key (scenario "every excluded tree is named with its reason, every run").
//!
//! The key is an array of `{ path, reason }` tables — `path` mandatory,
//! `reason` mandatory, empty by default. Every run of BOTH verbs prints every
//! active entry as one report line, exact bytes
//! `excluded sources: <path> — <reason>` (owner: the scan/verify report
//! surface, D13/R3-7: a run with an active exclusion that prints no line is
//! red-by-test). Validation is parse-time, the shipped invalid-spec class,
//! untagged — exit 1. In Phase A the entries are reported, not consumed:
//! the detection-side consumption lands with the agreement guard.

mod common;

use common::Fixture;

fn rust_tree(spec: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    tree.write("src/lib.rs", "pub fn f() {}\n");
    tree.write("architecture.spec.toml", spec);
    tree
}

fn spec_with(entries: &str) -> String {
    format!(
        "[project]\nlanguage = \"rust\"\n{entries}\n[[module]]\nname = \"demo\"\nmatches = {{ units = [\"demo\"] }}\n"
    )
}

/// Report lines print on the stderr/report surface — the stdout machine body
/// stays pure (wave-3 D02) and rust stdout verdict bytes stay unchanged.
fn excluded_lines(out: &std::process::Output) -> Vec<String> {
    common::stderr(out)
        .lines()
        .filter(|line| line.starts_with("excluded sources: "))
        .map(str::to_string)
        .collect()
}

#[test]
fn both_verbs_print_every_active_entry_every_run() {
    let tree = rust_tree(&spec_with(
        "\nexcluded_sources = [\n  { path = \"web/vendor\", reason = \"vendored dependencies\" },\n  { path = \"tools/generated\", reason = \"generated sources\" },\n]\n",
    ));

    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "scan failed: {}", common::stderr(&scan));
    let lines = excluded_lines(&scan);
    assert_eq!(
        lines,
        vec![
            "excluded sources: web/vendor — vendored dependencies",
            "excluded sources: tools/generated — generated sources",
        ],
        "scan must print every active entry, one line per entry, exact bytes"
    );

    let verify = tree.run(&["verify", "."]);
    assert!(verify.status.success(), "verify failed: {}", common::stderr(&verify));
    let lines = excluded_lines(&verify);
    assert_eq!(
        lines,
        vec![
            "excluded sources: web/vendor — vendored dependencies",
            "excluded sources: tools/generated — generated sources",
        ],
        "verify must print every active entry, one line per entry, exact bytes"
    );
}

#[test]
fn a_failing_verify_run_still_prints_the_lines() {
    // "every run" — the red verdict path prints the exclusions too.
    let tree = rust_tree(&spec_with("\nexcluded_sources = [ { path = \"web\", reason = \"generated\" } ]\n"));
    tree.write("src/other.rs", "pub fn g() {}\n");
    let out = tree.run(&["verify", "."]);
    let lines = excluded_lines(&out);
    assert_eq!(
        lines,
        vec!["excluded sources: web — generated"],
        "a run with an active exclusion that prints no line is red-by-test (R3-7)"
    );
}

#[test]
fn absent_key_prints_no_line_and_changes_no_verdict_bytes() {
    // Empty by default: specs that never use the key stay byte-identical.
    let tree = rust_tree(&spec_with(""));
    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "scan failed: {}", common::stderr(&scan));
    assert!(excluded_lines(&scan).is_empty());
    let verify = tree.run(&["verify", "."]);
    assert!(verify.status.success(), "verify failed: {}", common::stderr(&verify));
    assert!(excluded_lines(&verify).is_empty());
}

#[test]
fn entry_missing_reason_is_an_invalid_spec_error_naming_the_entry_and_key() {
    let tree = rust_tree(&spec_with("\nexcluded_sources = [ { path = \"web\" } ]\n"));
    let out = tree.run(&["verify", "."]);
    assert_eq!(out.status.code(), Some(1), "the invalid-spec class is untagged — exit 1");
    let err = common::stderr(&out);
    assert!(err.contains("invalid spec:"), "stderr: {err}");
    assert!(err.contains("#1"), "message names the entry (1-indexed): {err}");
    assert!(err.contains("reason"), "message names the mandatory key: {err}");
}

#[test]
fn entry_with_empty_reason_is_an_invalid_spec_error() {
    let tree = rust_tree(&spec_with("\nexcluded_sources = [ { path = \"web\", reason = \"\" } ]\n"));
    let out = tree.run(&["verify", "."]);
    assert_eq!(out.status.code(), Some(1));
    let err = common::stderr(&out);
    assert!(err.contains("invalid spec:"), "stderr: {err}");
    assert!(err.contains("#1"), "message names the entry: {err}");
}

#[test]
fn entry_with_empty_path_is_an_invalid_spec_error() {
    let tree = rust_tree(&spec_with("\nexcluded_sources = [ { path = \"\", reason = \"why\" } ]\n"));
    let out = tree.run(&["verify", "."]);
    assert_eq!(out.status.code(), Some(1));
    let err = common::stderr(&out);
    assert!(err.contains("invalid spec:"), "stderr: {err}");
    assert!(err.contains("#1"), "message names the entry: {err}");
    assert!(err.contains("path"), "message names the mandatory key: {err}");
}

#[test]
fn non_table_entry_is_an_invalid_spec_error() {
    let tree = rust_tree(&spec_with("\nexcluded_sources = [ \"web\" ]\n"));
    let out = tree.run(&["verify", "."]);
    assert_eq!(out.status.code(), Some(1));
    let err = common::stderr(&out);
    assert!(err.contains("invalid spec:"), "stderr: {err}");
}

#[test]
fn scan_reads_the_root_spec_only_and_an_invalid_spec_fails_the_run() {
    // Root-only discovery: a spec at the scan root is read for validity and
    // the excluded set; an invalid spec fails scan with verify's message
    // class and the same historical exit 1.
    let tree = rust_tree(&spec_with("\nexcluded_sources = [ { path = \"web\", reason = \"\" } ]\n"));
    let out = tree.run(&["scan", "."]);
    assert_eq!(out.status.code(), Some(1), "invalid-spec class keeps exit 1");
    assert!(common::stderr(&out).contains("invalid spec:"));

    // A tree without a spec scans exactly as before.
    let plain = Fixture::new();
    plain.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    plain.write("src/lib.rs", "pub fn f() {}\n");
    let out = plain.run(&["scan", "."]);
    assert!(out.status.success(), "specless scan failed: {}", common::stderr(&out));
    assert!(excluded_lines(&out).is_empty());
}
