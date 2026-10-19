// Reference engagement diagnostics
// (behaviors R1-R6): a module reference in `allowed.depend_on`,
// `allowed.forbidden`, or `contract.forbid` that can never engage a declared
// boundary (verify resolves references against declared claim names at any
// depth — top-level and submodule claims — and stereotype names only) is
// reported as a warning-severity `dead reference`
// instead of passing silently. Classification: declared -> no finding; exists
// in source but undeclared -> "exists in source but undeclared"; absent ->
// "does not exist" plus a did-you-mean candidate where feasible. Warnings pass
// without --strict and are promoted under --strict; a run with warnings never
// claims "matches source model". `constraint.modules` undeclared references are
// untouched here (schema hard error).

mod common;

use common::{stderr, stdout};

// R1 (phase B promotion, D10/R2-9): a `depend_on` target that names nothing —
// a removed claim, the retirement case — is an ERROR-class dead reference:
// exit 1 WITHOUT --strict, the line printed without the `warning:` prefix.
// The shipped shape was warning-severity rc-neutral (probe 2026-10-15,
// archspec 0.6.0: `warning: dead reference: module '<name>' allowed.depend_on
// target "<entry>" does not exist — create it or fix the reference`); this
// scenario is the declared promotion, never a restatement of that green.
#[test]
fn r1_dead_depend_on_target_is_an_error() {
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
        Some(1),
        "a naming-a-removed-claim dead reference must fail without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(
        !out.contains("warning: dead reference:"),
        "the promoted class prints without the warning prefix:\n{out}"
    );
    assert!(out.contains("allowed.depend_on"), "report:\n{out}");
    assert!(out.contains("\"storage\""), "report:\n{out}");
    assert!(out.contains("does not exist"), "report:\n{out}");
    assert!(
        !out.contains("matches source model"),
        "a failing run must not claim a match:\n{out}"
    );
    assert!(
        stderr(&output).is_empty(),
        "dead references are report content"
    );
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
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(out.contains("\"app::storage\""), "report:\n{out}");
    assert!(
        out.contains("exists in source but undeclared"),
        "report:\n{out}"
    );
    assert!(!out.contains("matches source model"), "report:\n{out}");
}

// R3 (phase B promotion, D10/R3-11): the `allowed.forbidden` ghost names
// nothing — the whole `does not exist` class is promoted to error, so the
// run exits 1 and that line prints without the warning prefix. The
// `contract.forbid` stereotype finding is a different class ("names no
// stereotype") and stays a warning entry.
#[test]
fn r3_dead_forbidden_target_is_an_error_stereotype_stays_warning() {
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
        Some(1),
        "a does-not-exist dead reference fails without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("dead reference: module 'app' allowed.forbidden target \"ghost\" does not exist"),
        "report:\n{out}"
    );
    assert!(
        !out.contains("warning: dead reference: module 'app' allowed.forbidden"),
        "the promoted class prints without the warning prefix:\n{out}"
    );
    assert!(
        out.contains("warning: dead reference: module 'app' contract.forbid"),
        "the stereotype class stays warning-severity:\n{out}"
    );
    assert!(out.contains("names no stereotype"), "report:\n{out}");
    assert!(!out.contains("matches source model"), "report:\n{out}");
}

// R3b (phase B promotion): a did-you-mean candidate rides the does-not-exist
// class — error severity, exit 1 without --strict.
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
        Some(1),
        "the does-not-exist class fails without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(
        !out.contains("warning: dead reference:"),
        "the promoted class prints without the warning prefix:\n{out}"
    );
    assert!(out.contains("\"storage\""), "report:\n{out}");
    assert!(out.contains("did you mean \"storage\""), "report:\n{out}");
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
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("matches source model"), "report:\n{out}");
    assert!(!out.contains("dead reference"), "report:\n{out}");
}

// R5: --strict promotes the WARNING-class dead references (the classifications
// not promoted by the phase B law) to errors; exit 1 and the `warning: `
// prefix is dropped. The warning-class fixture is r2's — a reference that
// exists in source but is undeclared.
#[test]
fn r5_strict_promotes_dead_references() {
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
    let plain = fixture.run(&["verify"]);
    assert_eq!(
        plain.status.code(),
        Some(0),
        "the warning class stays rc-neutral without --strict (stderr: {})",
        stderr(&plain)
    );
    assert!(
        stdout(&plain).contains("warning: dead reference:"),
        "the warning class keeps its prefix without --strict:\n{}",
        stdout(&plain)
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

// R7 (phase B, R3-11): the promotion covers the ENTIRE `does not exist`
// class — a forward-declared target named in an allowed entry but never
// created reddens verify WITHOUT --strict — and the error line is
// byte-identical with and without --strict (the severity is declared, not
// promoted).
#[test]
fn r7_forward_declared_target_reddens_and_the_line_is_strict_stable() {
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
         [module.allowed]\ndepend_on = [\"planned_reporting\"]\n",
    );
    let plain = fixture.run(&["verify"]);
    assert_eq!(
        plain.status.code(),
        Some(1),
        "a forward-declared allowed target fails without --strict (stderr: {})",
        stderr(&plain)
    );
    let plain_out = stdout(&plain);
    assert!(
        plain_out.contains(
            "dead reference: module 'app' allowed.depend_on target \"planned_reporting\" does not exist"
        ),
        "report:\n{plain_out}"
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_eq!(strict.status.code(), Some(1));
    let strict_out = stdout(&strict);
    for line in plain_out.lines().filter(|l| l.contains("dead reference:")) {
        assert!(
            strict_out.lines().any(|s| s == line),
            "the error line must be byte-identical under --strict:\n{line}"
        );
    }
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
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "output must be deterministic"
    );
    assert_eq!(stderr(&first), stderr(&second));
}
