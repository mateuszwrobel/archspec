// Reference engagement diagnostics
// (behaviors R1-R6): a module reference in `allowed.depend_on`,
// `allowed.forbidden`, or `contract.forbid` that can never engage a declared
// boundary (verify resolves references against declared top-level module names
// and stereotype names only) is reported as a warning-severity `dead reference`
// instead of passing silently. Classification: declared -> no finding; exists
// in source but undeclared -> "exists in source but undeclared"; absent ->
// "does not exist" plus a did-you-mean candidate where feasible. Warnings pass
// without --strict and are promoted under --strict; a run with warnings never
// claims "matches source model". `constraint.modules` undeclared references are
// untouched here (schema hard error).

mod common;

use common::{stderr, stdout};

// R1: a `depend_on` target that is not a declared module and matches nothing in
// the source is a dead reference warning; exit 0, never "matches source model".
#[test]
fn r1_dead_depend_on_target_is_warned() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [module.allowed]\ndepend_on = [\"storage\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "dead references must not fail without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(out.contains("allowed.depend_on"), "report:\n{out}");
    assert!(out.contains("\"storage\""), "report:\n{out}");
    assert!(out.contains("does not exist"), "report:\n{out}");
    assert!(
        !out.contains("matches source model"),
        "a run with warnings must not claim a match:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "dead references are report content");
}

// R2: a reference to a nested module path that exists in the source but is not
// a declared top-level module is classified "exists in source but undeclared".
#[test]
fn r2_nested_reference_exists_in_source_but_undeclared() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod storage;\n");
    fixture.write("src/storage.rs", "pub fn store() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [module.allowed]\ndepend_on = [\"app::storage\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(out.contains("\"app::storage\""), "report:\n{out}");
    assert!(
        out.contains("exists in source but undeclared"),
        "report:\n{out}"
    );
    assert!(!out.contains("matches source model"), "report:\n{out}");
}

// R3: an `allowed.forbidden` target matching no declared boundary and a
// `contract.forbid` naming no declared stereotype are both dead references.
#[test]
fn r3_dead_forbidden_target_and_stereotype_are_warned() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [module.allowed]\nforbidden = [\"ghost\"]\n\n\
         [module.contract]\nforbid = [\"external\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(out.contains("allowed.forbidden"), "report:\n{out}");
    assert!(out.contains("\"ghost\""), "report:\n{out}");
    assert!(out.contains("contract.forbid"), "report:\n{out}");
    assert!(
        out.contains("names no stereotype"),
        "report:\n{out}"
    );
    assert!(!out.contains("matches source model"), "report:\n{out}");
}

// R3b: an absent reference similar to a declared module name gets a
// did-you-mean candidate.
#[test]
fn r3b_absent_reference_gets_did_you_mean_candidate() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod core;\npub mod storage;\n");
    fixture.write(
        "src/core.rs",
        "use crate::storage::store;\npub fn core() { store(); }\n",
    );
    fixture.write("src/storage.rs", "pub fn store() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n\n\
         [module.allowed]\ndepend_on = [\"storage\", \"storge\"]\n\n\
         [[module]]\nname = \"storage\"\nmatches = { modules = [\"app::storage\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(out.contains("\"storage\""), "report:\n{out}");
    assert!(
        out.contains("did you mean \"storage\""),
        "report:\n{out}"
    );
}

// R4: a spec whose references all name declared modules produces no dead
// reference noise and still confirms the match.
#[test]
fn r4_declared_references_are_zero_noise() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n");
    fixture.write(
        "a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nb = { path = \"../b\" }\n",
    );
    fixture.write("a/src/lib.rs", "pub fn a() { b::b(); }\n");
    fixture.write(
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("b/src/lib.rs", "pub fn b() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n\
         [module.allowed]\ndepend_on = [\"b\"]\n\n\
         [[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("matches source model"), "report:\n{out}");
    assert!(!out.contains("dead reference"), "report:\n{out}");
}

// R5: --strict promotes dead reference warnings to errors; exit 1 and the
// `warning: ` prefix is dropped.
#[test]
fn r5_strict_promotes_dead_references() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [module.allowed]\ndepend_on = [\"storage\"]\n",
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "--strict must fail a dead-reference-only run (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(
        !out.contains("warning: dead reference:"),
        "promoted lines drop the warning prefix:\n{out}"
    );
}

// R6: the dead reference report is byte-stable across runs.
#[test]
fn r6_dead_reference_report_is_deterministic() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
         [module.allowed]\ndepend_on = [\"storage\", \"ghost\"]\nforbidden = [\"phantom\", \"storage\"]\n\n\
         [module.contract]\nforbid = [\"external\", \"external\"]\n",
    );
    let first = fixture.run(&["verify"]);
    let second = fixture.run(&["verify"]);
    assert_eq!(stdout(&first), stdout(&second), "output must be deterministic");
    assert_eq!(stderr(&first), stderr(&second));
}
