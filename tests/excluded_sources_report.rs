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

// ——— Phase 2: the `mode` dial, specificity and liveness (decision-check
// BLOCKER-2, MAJOR-1, MAJOR-2). The dial rides entries only, the mechanical
// set never; `abort` keeps every shipped byte, `warn` states the hit on the
// warnings channel — green without `--strict`, red with it, never the
// report-only channel (which can never go red). ———

fn ts_tree_with_entries(entries: &str, files: &[(&str, &str)]) -> Fixture {
    let tree = Fixture::new();
    tree.write("package.json", "{ \"name\": \"demo-ui\" }\n");
    for (path, content) in files {
        tree.write(path, content);
    }
    tree.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"ts\"\n{entries}\n[[module]]\nname = \"demo-ui\"\nmatches = {{ units = [\"demo-ui\"] }}\n"
        ),
    );
    tree
}

fn liveness_lines(out: &std::process::Output) -> Vec<String> {
    common::stderr(out)
        .lines()
        .filter(|line| line.starts_with("excluded source matched nothing: "))
        .map(str::to_string)
        .collect()
}

/// BLOCKER-2 honesty: a `warn` hit is a WARNING-SEVERITY finding naming
/// entry and target. Without `--strict` the run passes and the finding is
/// labelled `warning:`; under `--strict` the SAME finding fails the run.
/// It never rides the report-only channel and never enters the machine body.
#[test]
fn warn_hit_is_green_without_strict_and_red_under_it() {
    let tree = ts_tree_with_entries(
        "\nexcluded_sources = [ { path = \"src/vendor\", reason = \"vendored legacy code\", mode = \"warn\" } ]\n",
        &[
            (
                "src/a.ts",
                "import { legacy } from \"./vendor/legacy\";\nexport const a = legacy;\n",
            ),
            ("src/vendor/legacy.ts", "export const legacy = 1;\n"),
        ],
    );

    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "warn never aborts a scan: {}", common::stderr(&scan));
    let err = common::stderr(&scan);
    assert!(
        err.contains(
            "warning: excluded source: src/a.ts -> \"./vendor/legacy\" — resolves only into excluded path src/vendor"
        ),
        "scan must state the finding with evidence (file, specifier, entry):\n{err}"
    );
    assert!(
        !err.contains("unresolvable import"),
        "a warn hit never renders the abort line:\n{err}"
    );
    let body: serde_json::Value =
        serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is the model JSON");
    assert!(
        body.get("warnings").is_none(),
        "the warnings channel is transient — never the machine body:\n{body}"
    );

    let verify = tree.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "without `--strict` a warning-severity finding passes (exit 0):\n{}",
        common::stdout(&verify)
    );
    assert!(
        common::stdout(&verify).contains(
            "warning: excluded source: src/a.ts -> \"./vendor/legacy\" — resolves only into excluded path src/vendor"
        ),
        "the report must list the warning:\n{}",
        common::stdout(&verify)
    );

    let strict = tree.run(&["verify", ".", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(1),
        "`--strict` promotes the warning to a failing finding:\n{}",
        common::stdout(&strict)
    );
    assert!(
        common::stdout(&strict).contains(
            "excluded source: src/a.ts -> \"./vendor/legacy\" — resolves only into excluded path src/vendor"
        ),
        "the finding keeps its evidence under `--strict`:\n{}",
        common::stdout(&strict)
    );
}

/// The default stays the shipped law byte-for-byte: an explicit
/// `mode = "abort"` and no mode at all answer identically — same exit code,
/// same stderr, same stdout (the dial cannot shift the abort bytes).
#[test]
fn explicit_abort_is_byte_identical_to_the_default_mode() {
    let files = [
        (
            "src/a.ts",
            "import { legacy } from \"./vendor/legacy\";\nexport const a = legacy;\n",
        ),
        ("src/vendor/legacy.ts", "export const legacy = 1;\n"),
    ];
    let default = ts_tree_with_entries(
        "\nexcluded_sources = [ { path = \"src/vendor\", reason = \"vendored legacy code\" } ]\n",
        &files,
    );
    let explicit = ts_tree_with_entries(
        "\nexcluded_sources = [ { path = \"src/vendor\", reason = \"vendored legacy code\", mode = \"abort\" } ]\n",
        &files,
    );

    for verb in ["verify", "scan"] {
        let a = default.run(&[verb, "."]);
        let b = explicit.run(&[verb, "."]);
        assert_eq!(a.status.code(), b.status.code(), "{verb}: exit must match");
        assert_eq!(common::stdout(&a), common::stdout(&b), "{verb}: stdout must match");
        assert_eq!(common::stderr(&a), common::stderr(&b), "{verb}: stderr must match");
    }
}

/// MAJOR-1: precedence is specificity, not TOML order. The exact-path entry
/// (warn) beats the directory prefix (abort) in BOTH declaration orders. The
/// entry echo lists entries as declared, so it is excluded from the byte
/// comparison; everything else — exit code, findings, other lines — must be
/// identical either way, so the dial can never be order-sensitive.
#[test]
fn specificity_beats_declaration_order_in_both_orders() {
    let files = [
        (
            "src/a.ts",
            "import { legacy } from \"./vendor/legacy\";\nexport const a = legacy;\n",
        ),
        ("src/vendor/legacy.ts", "export const legacy = 1;\n"),
    ];
    let prefix_first = ts_tree_with_entries(
        "\nexcluded_sources = [\n  { path = \"src/vendor\", reason = \"directory exclusion\" },\n  { path = \"src/vendor/legacy.ts\", reason = \"the exact file\", mode = \"warn\" },\n]\n",
        &files,
    );
    let exact_first = ts_tree_with_entries(
        "\nexcluded_sources = [\n  { path = \"src/vendor/legacy.ts\", reason = \"the exact file\", mode = \"warn\" },\n  { path = \"src/vendor\", reason = \"directory exclusion\" },\n]\n",
        &files,
    );

    let first = prefix_first.run(&["verify", "."]);
    let second = exact_first.run(&["verify", "."]);
    for run in [&first, &second] {
        assert!(
            run.status.success(),
            "the exact warn entry must win over the abort prefix:\n{}",
            common::stdout(run)
        );
        assert!(
            common::stdout(run)
                .contains("excluded source: src/a.ts -> \"./vendor/legacy\" — resolves only into excluded path src/vendor/legacy.ts"),
            "the finding must name the SPECIFICITY-WINNING entry:\n{}",
            common::stdout(run)
        );
    }
    assert_eq!(common::stdout(&first), common::stdout(&second));
    let echo_free = |s: String| -> String {
        s.lines()
            .filter(|line| !line.starts_with("excluded sources: "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        echo_free(common::stderr(&first)),
        echo_free(common::stderr(&second)),
        "every line but the entry echo must be byte-identical across declaration orders"
    );
}

/// The tie law end-to-end: two entries declaring the same path would match
/// every shared file with equal specificity — refused at load as an
/// invalid-spec error naming BOTH entries, not decided by TOML order.
#[test]
fn duplicate_entry_paths_are_refused_naming_both_entries() {
    let tree = ts_tree_with_entries(
        "\nexcluded_sources = [\n  { path = \"src/vendor\", reason = \"first\" },\n  { path = \"src/vendor\", reason = \"second\", mode = \"warn\" },\n]\n",
        &[("src/a.ts", "export const a = 1;\n")],
    );
    let out = tree.run(&["verify", "."]);
    assert_eq!(out.status.code(), Some(1), "a tie is an invalid-spec error");
    let err = common::stderr(&out);
    assert!(err.contains("invalid spec:"), "stderr: {err}");
    assert!(err.contains("#1") && err.contains("#2"), "both entries are named: {err}");
    assert!(err.contains("src/vendor"), "the tied path is named: {err}");
}

/// MAJOR-2: an entry matching nothing is a silent pass no longer. One
/// liveness line per dead entry, exact bytes, printed once per run by BOTH
/// verbs, right after the active-entry lines; live entries print none.
#[test]
fn liveness_line_names_each_dead_entry_once_per_run() {
    let tree = ts_tree_with_entries(
        "\nexcluded_sources = [\n  { path = \"src/vendor\", reason = \"vendored legacy code\" },\n  { path = \"web/generated\", reason = \"generated sources\" },\n]\n",
        &[
            ("src/a.ts", "export const a = 1;\n"),
            ("src/vendor/legacy.ts", "export const legacy = 1;\n"),
        ],
    );
    for verb in ["scan", "verify"] {
        let out = tree.run(&[verb, "."]);
        assert_eq!(
            liveness_lines(&out),
            vec!["excluded source matched nothing: web/generated — generated sources"],
            "`{verb}` must print exactly one liveness line for the dead entry:\n{}",
            common::stderr(&out)
        );
    }
}

/// The liveness half of the mechanical law: a subtree the walk never visits
/// cannot keep a dead entry alive — an entry whose only content lives under
/// a mechanical exclusion directory is DEAD and prints its line.
#[test]
fn mechanical_subtrees_do_not_keep_an_entry_alive() {
    let tree = ts_tree_with_entries(
        "\nexcluded_sources = [ { path = \"web/pkg\", reason = \"vendored package\" } ]\n",
        &[
            ("src/a.ts", "export const a = 1;\n"),
            ("web/pkg/node_modules/dep/index.js", "module.exports = 1;\n"),
        ],
    );
    let out = tree.run(&["scan", "."]);
    assert_eq!(
        liveness_lines(&out),
        vec!["excluded source matched nothing: web/pkg — vendored package"],
        "content only behind the mechanical set is no match:\n{}",
        common::stderr(&out)
    );
}
