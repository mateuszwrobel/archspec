// Phase A vacuous-guard diagnostics (Feature 2, behaviors V1-V13):
// a constraint whose effective domain is empty —
// no module to check, no edges, no manifests, no exports — is reported as a
// warning-severity "vacuous constraint" instead of silently passing. A run whose
// ONLY findings are vacuous never claims "matches source model"; its header
// names the vacuous guard(s) instead. `--strict` promotes vacuous findings to
// errors (exit 1). The report command lists them too.

mod common;

use common::{stderr, stdout};

/// A single `app` crate with `core` and `ui` modules; `core` imports `serde`,
/// `ui` imports nothing.
fn forbid_external_app(spec_body: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod core;\npub mod ui;\n");
    fixture.write("src/core.rs", "use serde::Serialize;\npub fn core() {}\n");
    fixture.write("src/ui.rs", "pub fn ui() {}\n");
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = {{ units = [\"app\"] }}\n\n{spec_body}"
        ),
    );
    fixture
}

// V1: a `from` pattern matching no module with external dependencies is
// reported as a warning-level vacuous constraint; exit 0, never "matches".
#[test]
fn v1_unmatched_from_pattern_is_reported_not_passed() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::ui\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "vacuous findings must not fail without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(out.contains("forbid_external_crates"), "report:\n{out}");
    assert!(out.contains("'from' pattern \"app::ui\""), "report:\n{out}");
    assert!(
        !out.contains("matches source model"),
        "vacuous-only run must not claim a match:\n{out}"
    );
    assert!(
        stderr(&output).is_empty(),
        "vacuous findings are report content"
    );
}

// V2: --strict promotes a vacuous finding to an error; the line loses the
// `warning: ` prefix and the run exits 1.
#[test]
fn v2_strict_promotes_vacuous_to_error() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::ui\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "--strict must fail a vacuous-only run (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(
        !out.contains("warning: vacuous constraint:"),
        "promoted lines drop the warning prefix:\n{out}"
    );
    assert!(
        stderr(&output).is_empty(),
        "rule violations are report content"
    );
}

// V3: the vacuous report names the offending (here misspelled) `from` pattern.
#[test]
fn v3_vacuous_report_names_unmatched_pattern() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::persistnce\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(
        out.contains("app::persistnce"),
        "report must name the pattern:\n{out}"
    );
}

// V4: a no_cycles constraint whose modules have no dependency edges is vacuous.
#[test]
fn v4_no_cycles_over_edge_free_modules_is_vacuous() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n");
    fixture.write(
        "a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("a/src/lib.rs", "pub fn a() {}\n");
    fixture.write(
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("b/src/lib.rs", "pub fn b() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n\
         [[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\n\n\
         [[constraint]]\ntype = \"no_cycles\"\nmodules = [\"a\", \"b\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "vacuous no_cycles must not fail without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(out.contains("no_cycles"), "report:\n{out}");
    assert!(!out.contains("matches source model"), "report:\n{out}");
}

// V5: manifest_integrity with nothing to check (no manifests) is vacuous.
#[test]
fn v5_manifest_integrity_with_no_manifests_is_vacuous() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "app/app.go",
        "package app\n\nimport \"example.com/third/party\"\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"example.com/demo/app\"] }\n\n\
         [[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"example.com/some/dep\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "vacuous manifest_integrity must not fail without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(out.contains("no manifests to check"), "report:\n{out}");
}

// V6: public_api_allowlist with no crate-root exports is vacuous.
#[test]
fn v6_public_api_allowlist_with_no_exports_is_vacuous() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("app/app.go", "package app\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"example.com/demo/app\"] }\n\n\
         [[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"App\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "vacuous public_api_allowlist must not fail without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(
        out.contains("no crate-root public exports"),
        "report:\n{out}"
    );
}

// V7: forbid_submodule_dependency with no engaging intra-parent submodule edge
// is vacuous.
#[test]
fn v7_forbid_submodule_with_no_matching_edge_is_vacuous() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod orchestration;\n");
    fixture.write(
        "src/orchestration.rs",
        "pub mod common;\npub mod breakdown;\npub mod control_loop;\n",
    );
    fixture.write("src/orchestration/common.rs", "pub fn common() {}\n");
    fixture.write(
        "src/orchestration/breakdown.rs",
        "use crate::orchestration::control_loop::Loop;\npub fn breakdown() {}\n",
    );
    fixture.write("src/orchestration/control_loop.rs", "pub struct Loop;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [[constraint]]\ntype = \"forbid_submodule_dependency\"\nparent = \"orchestration\"\nfrom = [\"common\"]\nforbid = [\"control_loop\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "vacuous forbid_submodule must not fail without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(
        out.contains("no intra-parent submodule edges"),
        "report:\n{out}"
    );
}

// V8: an engaged forbid_external constraint whose from modules have external
// deps but none forbidden is clean — no vacuous warning, real pass.
#[test]
fn v8_engaged_forbid_external_is_not_vacuous() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"core\"]\nforbid = [\"clap\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("matches source model"), "report:\n{out}");
    assert!(
        !out.contains("vacuous constraint"),
        "no vacuous warning:\n{out}"
    );
}

// V9a: a run with a real error AND a vacuous constraint lists both; the vacuous
// one keeps its `warning: ` prefix without --strict, but the error dominates.
#[test]
fn v9_mixed_error_and_vacuous_lists_both_without_strict() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"core\"]\nforbid = [\"serde\"]\n\n\
         [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::ui\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_ne!(output.status.code(), Some(0), "error finding must fail");
    let out = stdout(&output);
    assert!(
        out.contains("forbidden external crate: core -> serde"),
        "report:\n{out}"
    );
    assert!(
        out.contains("warning: vacuous constraint:"),
        "report:\n{out}"
    );
    assert!(
        out.contains("does not match source model"),
        "report:\n{out}"
    );
}

// V9b: under --strict the same run promotes the vacuous line (no warning prefix).
#[test]
fn v9b_strict_promotes_vacuous_alongside_error() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"core\"]\nforbid = [\"serde\"]\n\n\
         [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::ui\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_ne!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("forbidden external crate: core -> serde"),
        "report:\n{out}"
    );
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(
        !out.contains("warning: vacuous constraint:"),
        "report:\n{out}"
    );
}

// V10: a genuinely clean, engaging manifest_integrity run still confirms the
// match and is never flagged vacuous.
#[test]
fn v10_engaging_manifest_integrity_confirms_match() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[features]\ntelemetry = []\n",
    );
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [[constraint]]\ntype = \"manifest_integrity\"\nrequired_features = [\"telemetry\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("matches source model"), "report:\n{out}");
    assert!(
        !out.contains("vacuous constraint"),
        "no vacuous warning:\n{out}"
    );
}

// V11: a vacuous-only run's header names each vacuous guard, not "does not
// match source model".
#[test]
fn v11_vacuous_only_header_names_the_guards() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::ui\"]\nforbid = [\"serde\"]\n\n\
         [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::persistnce\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("architecture.spec.toml has vacuous constraints:"),
        "header must name the guards:\n{out}"
    );
    assert!(
        out.contains("[constraint #1] forbid_external_crates"),
        "header must name constraint #1:\n{out}"
    );
    assert!(
        out.contains("[constraint #2] forbid_external_crates"),
        "header must name constraint #2:\n{out}"
    );
    assert!(
        !out.contains("does not match source model"),
        "report:\n{out}"
    );
}

// V12: the report command lists vacuous constraints as report content and exits
// 0 (informational, like every other finding).
#[test]
fn v12_report_lists_vacuous_constraint_and_exits_zero() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::ui\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["report"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "report violations are content, exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(out.contains("'from' pattern \"app::ui\""), "report:\n{out}");
}

// V13: verify output for a vacuous-only run is byte-stable across runs.
#[test]
fn v13_vacuous_output_is_byte_stable() {
    let fixture = forbid_external_app(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::ui\"]\nforbid = [\"serde\"]\n",
    );
    let first = fixture.run(&["verify"]);
    let second = fixture.run(&["verify"]);
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "output must be deterministic"
    );
    assert_eq!(stderr(&first), stderr(&second));
}

// Regression: a forbid_external constraint whose `from` module DOES have an
// external import (the fixed `module_outside_forbid_list_passes` shape) engages
// and verifies clean — never a false vacuous warning.
#[test]
fn engaged_from_module_with_external_import_verifies_clean() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod core;\npub mod ui;\n");
    fixture.write("src/core.rs", "use serde::Serialize;\npub fn core() {}\n");
    fixture.write("src/ui.rs", "use tokio;\npub fn ui() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::ui\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("matches source model"), "report:\n{out}");
    assert!(
        !out.contains("vacuous constraint"),
        "no vacuous warning:\n{out}"
    );
}
