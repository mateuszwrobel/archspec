mod common;

use common::{stderr, stdout};

/// A Rust workspace crate whose spec declares components matching every
/// extracted unit, plus allowed/forbidden matching the real edge.
fn matching_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n",
    );
    fixture
}

/// A single-crate `auth` workspace with no spec written, for the contract
/// schema tests that attach a `contract` and observe the loader's decision.
fn auth_crate_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture
}

#[test]
fn verify_scenario_one_matching_spec_exits_zero_with_confirmation() {
    let fixture = matching_fixture();
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "matching spec must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.starts_with("ok: "),
        "pass confirmation on stdout:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

#[test]
fn verify_scenario_two_is_byte_identical_across_runs() {
    let fixture = matching_fixture();
    let first = fixture.run(&["verify"]);
    let second = fixture.run(&["verify"]);

    assert_eq!(first.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "verify output must be byte-identical across runs"
    );
}

#[test]
fn verify_accepts_strict_flag_and_explicit_path_on_matching_project() {
    let fixture = matching_fixture();
    let strict = fixture.run(&["verify", "--strict"]);
    let explicit = fixture.run(&["verify", "."]);

    assert_eq!(
        strict.status.code(),
        Some(0),
        "--strict must not break a pass"
    );
    assert_eq!(
        explicit.status.code(),
        Some(0),
        "explicit path to the project root must work"
    );
}

#[test]
fn verify_rejects_unknown_flag() {
    let fixture = matching_fixture();
    let output = fixture.run(&["verify", "--format", "mermaid"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("unknown flag: --format"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_rejects_two_positional_paths() {
    let fixture = matching_fixture();
    let output = fixture.run(&["verify", "crates/auth", "crates/billing"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("expected at most one path argument"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_rejects_missing_path() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["verify", "/no/such/dir"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path does not exist: /no/such/dir"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_rejects_regular_file_path() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\n");
    let output = fixture.run(&["verify", "Cargo.toml"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path is not a directory: Cargo.toml"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_rejects_directory_without_spec() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = []\n");
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("no architecture.spec.toml found in:"));
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_invalid_toml_names_file_and_parse_error() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = []\n");
    fixture.write("architecture.spec.toml", "[project\nlanguage = \"rust\"\n");
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains("architecture.spec.toml"),
        "must name the spec file and the parse error:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_unknown_constraint_type_names_field_and_why() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = []\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[constraint]]\ntype = \"no_loops\"\nmodules = [\"auth\"]\nseverity = \"error\"\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("unknown constraint type \"no_loops\"")
            && err.contains("[constraint] #1"),
        "must name the file, field, and why:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_divergent_spec_reports_on_stdout_with_empty_stderr() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    // Spec declares a component the code does not contain.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"ghost\"\nmatches = { units = [\"ghost\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(
        output.status.code(),
        Some(0),
        "divergence must exit non-zero"
    );
    let out = stdout(&output);
    assert!(
        out.contains("architecture.spec.toml does not match source model"),
        "report header on stdout:\n{out}"
    );
    assert!(
        out.contains("missing component: ghost"),
        "report names the missing component:\n{out}"
    );
    assert!(
        stderr(&output).is_empty(),
        "rule violations are report content, not stderr"
    );
}

/// A divergence must exit non-zero, print the report header plus every expected
/// report line on stdout, and keep stderr empty (output.md, errors.md).
fn assert_divergent(output: &std::process::Output, expected_lines: &[&str]) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "divergence must exit non-zero (stderr: {})",
        stderr(output)
    );
    let out = stdout(output);
    assert!(
        out.contains("architecture.spec.toml does not match source model"),
        "report header on stdout:\n{out}"
    );
    for line in expected_lines {
        assert!(out.contains(line), "report must contain `{line}`:\n{out}");
    }
    assert!(
        stderr(output).is_empty(),
        "rule violations are report content, not stderr"
    );
}

/// Write a single-package Go fixture file. The path is the package dir under
/// the module root; imports are matched verbatim against extracted unit names.
fn write_go_package(fixture: &common::Fixture, dir: &str, package: &str, imports: &[&str]) {
    let mut src = format!("package {package}\n");
    if !imports.is_empty() {
        src.push_str("\nimport (\n");
        for import in imports {
            src.push_str(&format!("    {import:?}\n"));
        }
        src.push_str(")\n");
    }
    src.push('\n');
    fixture.write(&format!("{dir}/{package}.go"), &src);
}

#[test]
fn verify_scenario_three_reports_added_component() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\", \"crates/portal\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    // Code grew a crate the spec does not declare.
    fixture.write(
        "crates/portal/Cargo.toml",
        "[package]\nname = \"portal\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/portal/src/lib.rs", "pub fn portal() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n",
    );
    assert_divergent(&fixture.run(&["verify"]), &["unexpected component: portal"]);
}

#[test]
fn verify_scenario_four_reports_missing_component() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    // Spec declares a component the code no longer contains.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[[module]]\nname = \"ghost\"\nmatches = { units = [\"ghost\"] }\n",
    );
    assert_divergent(&fixture.run(&["verify"]), &["missing component: ghost"]);
}

#[test]
fn verify_scenario_five_reports_forbidden_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\", \"crates/portal\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nportal = { path = \"../portal\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "crates/portal/Cargo.toml",
        "[package]\nname = \"portal\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/portal/src/lib.rs", "pub fn portal() {}\n");
    // The extracted billing -> portal edge is explicitly banned by the spec.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\nforbidden = [\"portal\"]\n\n[[module]]\nname = \"portal\"\nmatches = { units = [\"portal\"] }\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["forbidden edge: billing -> portal"],
    );
}

#[test]
fn verify_scenario_six_reports_disallowed_cross_component_dependency() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\", \"crates/portal\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\nportal = { path = \"../portal\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "crates/portal/Cargo.toml",
        "[package]\nname = \"portal\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/portal/src/lib.rs", "pub fn portal() {}\n");
    // billing may depend on auth, but portal is not in depend_on: the
    // billing -> portal edge crosses a boundary the spec does not allow.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n\n[[module]]\nname = \"portal\"\nmatches = { units = [\"portal\"] }\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["disallowed cross-component dependency: billing -> portal"],
    );
}

#[test]
fn verify_scenario_seven_reports_unassigned_unit() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\n");
    // The store package nests under the module-declared auth namespace but is
    // itself matched by no declared component: it is an unassigned sub-unit.
    write_go_package(&fixture, "auth", "auth", &[]);
    write_go_package(&fixture, "auth/store", "store", &[]);
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\"] }\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["unassigned unit: example.com/demo/auth/store"],
    );
}

#[test]
fn verify_scenario_eight_reports_contract_leak() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/billing\", \"crates/billing_entity\"]\n",
    );
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "crates/billing_entity/Cargo.toml",
        "[package]\nname = \"billing_entity\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/billing_entity/src/lib.rs", "pub fn entity() {}\n");
    // The billing module owns an entity-stereotype unit but its contract
    // forbids exposing the entity stereotype.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[stereotype]]\nname = \"entity\"\nmatch = { names = [\"billing_entity\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\", \"billing_entity\"] }\n\n[module.contract]\nforbid = [\"entity\"]\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["contract leak: billing exposes entity (forbidden)"],
    );
}

#[test]
fn verify_scenario_nine_reports_cycle() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[module.allowed]\ndepend_on = [\"billing\"]\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"auth\", \"billing\"]\nseverity = \"error\"\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["cycle: auth -> billing -> auth"],
    );
}

#[test]
fn verify_scenario_ten_lists_all_divergences_in_one_run() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\n");
    write_go_package(&fixture, "auth", "auth", &[]);
    write_go_package(&fixture, "auth/store", "store", &[]);
    write_go_package(
        &fixture,
        "billing",
        "billing",
        &["example.com/demo/auth", "example.com/demo/portal"],
    );
    write_go_package(&fixture, "portal", "portal", &[]);
    write_go_package(&fixture, "extra", "extra", &[]);
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"example.com/demo/billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\nforbidden = [\"portal\"]\n\n[[module]]\nname = \"portal\"\nmatches = { units = [\"example.com/demo/portal\"] }\n\n[[module]]\nname = \"shared\"\nmatches = { units = [\"example.com/demo/shared\"] }\n",
    );
    let expected = [
        "missing component: shared",
        "unexpected component: example.com/demo/extra",
        "unassigned unit: example.com/demo/auth/store",
        "forbidden edge: billing -> portal",
    ];
    assert_divergent(&fixture.run(&["verify"]), &expected);

    // Determinism: unchanged project, two runs, byte-identical diff (acceptance #15).
    let first = fixture.run(&["verify"]);
    let second = fixture.run(&["verify"]);
    assert_eq!(first.status.code(), Some(1), "first run must fail");
    assert_eq!(second.status.code(), Some(1), "second run must fail");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "diff report must be byte-identical across runs"
    );
}

#[test]
fn verify_scenario_eleven_reports_missing_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    // billing does NOT depend on auth, though the spec requires it.
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["missing edge: billing -> auth"],
    );
}

#[test]
fn verify_constraint_referencing_undeclared_module_is_invalid() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"ghost\"]\nseverity = \"error\"\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains("undeclared module \"ghost\""),
        "must reject a constraint over an undeclared module:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

/// Two crates in a mutual dependency cycle, spec declaring both components and
/// a `no_cycles` constraint at warning severity. The ONLY divergence is the
/// warning-level cycle (edges are declared in `allowed.depend_on`, so no other
/// category fires). Shared by scenarios #12/#13.
fn warning_cycle_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[module.allowed]\ndepend_on = [\"billing\"]\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"auth\", \"billing\"]\nseverity = \"warning\"\n",
    );
    fixture
}

/// Two disjoint mutual-dependency cycles (auth<->billing, portal<->shared), one
/// warning-severity `no_cycles` constraint over all four components. Every
/// divergence is warning-level; used by scenario #14.
fn two_warning_cycles_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\", \"crates/portal\", \"crates/shared\"]\n",
    );
    for (name, deps) in [
        ("auth", vec!["billing"]),
        ("billing", vec!["auth"]),
        ("portal", vec!["shared"]),
        ("shared", vec!["portal"]),
    ] {
        let mut manifest =
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
        if !deps.is_empty() {
            manifest.push_str("\n[dependencies]\n");
            for dep in &deps {
                manifest.push_str(&format!("{dep} = {{ path = \"../{dep}\" }}\n"));
            }
        }
        fixture.write(&format!("crates/{name}/Cargo.toml"), &manifest);
        fixture.write(
            &format!("crates/{name}/src/lib.rs"),
            &format!("pub fn {}() {{}}\n", name.replace("shared", "shared_fn")),
        );
    }
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[module.allowed]\ndepend_on = [\"billing\"]\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n\n[[module]]\nname = \"portal\"\nmatches = { units = [\"portal\"] }\n\n[module.allowed]\ndepend_on = [\"shared\"]\n\n[[module]]\nname = \"shared\"\nmatches = { units = [\"shared\"] }\n\n[module.allowed]\ndepend_on = [\"portal\"]\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"auth\", \"billing\", \"portal\", \"shared\"]\nseverity = \"warning\"\n",
    );
    fixture
}

#[test]
fn verify_scenario_twelve_warning_cycle_is_listed_but_does_not_fail() {
    let fixture = warning_cycle_fixture();
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "a warning-level divergence must not fail the run (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("warning: cycle: auth -> billing -> auth"),
        "report must list the cycle as a warning:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on a warning run");
}

#[test]
fn verify_scenario_thirteen_strict_promotes_warning_cycle_to_error() {
    let fixture = warning_cycle_fixture();
    let output = fixture.run(&["verify", "--strict"]);

    assert_ne!(
        output.status.code(),
        Some(0),
        "--strict must promote the warning to a failure"
    );
    let out = stdout(&output);
    assert!(
        out.contains("cycle: auth -> billing -> auth"),
        "report must list the cycle:\n{out}"
    );
    assert!(
        !out.contains("warning:"),
        "promoted warning must render as an error line, not a warning:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on a strict failure");
}

#[test]
fn verify_scenario_fourteen_strict_reports_each_promoted_warning() {
    let fixture = two_warning_cycles_fixture();

    let lax = fixture.run(&["verify"]);
    assert_eq!(
        lax.status.code(),
        Some(0),
        "warning-only run must exit 0 (stderr: {})",
        stderr(&lax)
    );
    let lax_out = stdout(&lax);
    assert!(
        lax_out.contains("warning: cycle: auth -> billing -> auth"),
        "must list the auth/billing cycle as a warning:\n{lax_out}"
    );
    assert!(
        lax_out.contains("warning: cycle: portal -> shared -> portal"),
        "must list the portal/shared cycle as a warning:\n{lax_out}"
    );

    let strict = fixture.run(&["verify", "--strict"]);
    assert_ne!(
        strict.status.code(),
        Some(0),
        "--strict must fail when only warnings are present"
    );
    let strict_out = stdout(&strict);
    assert!(
        strict_out.contains("cycle: auth -> billing -> auth"),
        "must list the auth/billing cycle as an error:\n{strict_out}"
    );
    assert!(
        strict_out.contains("cycle: portal -> shared -> portal"),
        "must list the portal/shared cycle as an error:\n{strict_out}"
    );
    assert!(
        !strict_out.contains("warning:"),
        "each promoted warning must render as an error line:\n{strict_out}"
    );
    assert!(stderr(&strict).is_empty(), "no stderr on a strict failure");
}

#[test]
fn verify_leading_wildcard_glob_matches() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/portal\"]\n");
    fixture.write(
        "crates/portal/Cargo.toml",
        "[package]\nname = \"portal\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/portal/src/lib.rs", "pub fn portal() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"frontend\"\nmatches = { units = [\"*portal\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "must exit 0, got stderr: {}",
        stderr(&output)
    );
    assert!(
        stdout(&output).starts_with("ok: "),
        "leading-wildcard glob must match"
    );
}

#[test]
fn verify_no_cycles_without_severity_defaults_to_error() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"auth\", \"billing\"]\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(
        output.status.code(),
        Some(0),
        "default severity must be error"
    );
    let out = stdout(&output);
    assert!(
        out.contains("cycle: auth -> billing -> auth")
            || out.contains("cycle: billing -> auth -> billing"),
        "cycle listed as an error (no warning prefix):\n{out}"
    );
    assert!(
        !out.contains("warning: cycle"),
        "default severity must not warn:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on rule violation");
}

/// `contract.expose` is parsed but enforced by no check; declaring it would
/// give false comfort, so the loader rejects its presence until an
/// enforcement exists. `contract.forbid` is enforced and stays accepted.
#[test]
fn verify_contract_expose_is_a_schema_error() {
    let fixture = auth_crate_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\ncontract = { expose = [\"domain\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains("contract.expose"),
        "must reject the unenforced field:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

/// Presence — not emptiness — signals an unenforced contract, so an empty
/// `expose` list is rejected exactly like a populated one.
#[test]
fn verify_contract_expose_empty_list_is_a_schema_error() {
    let fixture = auth_crate_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\ncontract = { expose = [] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains("contract.expose"),
        "presence of expose must be rejected even when empty:\n{err}"
    );
}

/// The rejection recurses: a submodule cannot launder `expose` past the
/// top-level check.
#[test]
fn verify_submodule_contract_expose_is_a_schema_error() {
    let fixture = auth_crate_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module.submodules]]\nname = \"auth::internal\"\nmatches = { modules = [\"auth::internal\"] }\ncontract = { expose = [\"domain\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains("contract.expose"),
        "nested expose must be rejected:\n{err}"
    );
}

/// Presence triggers at the submodule level too: an empty nested list is a
/// schema error like its populated sibling.
#[test]
fn verify_submodule_contract_expose_empty_list_is_a_schema_error() {
    let fixture = auth_crate_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module.submodules]]\nname = \"auth::internal\"\nmatches = { modules = [\"auth::internal\"] }\ncontract = { expose = [] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains("contract.expose"),
        "nested empty expose must be rejected:\n{err}"
    );
}

/// `contract.forbid` is enforced (contract leaks), so it stays accepted at the
/// top level.
#[test]
fn verify_contract_forbid_is_accepted() {
    let fixture = auth_crate_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[stereotype]]\nname = \"entity\"\nmatch = { names = [\"nothing\"] }\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\ncontract = { forbid = [\"entity\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    let err = stderr(&output);
    assert!(
        !err.contains("invalid spec:") && !err.contains("contract.expose"),
        "enforced forbid must load cleanly:\n{err}"
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "clean forbid verifies:\n{err}"
    );
}

/// Enforced `forbid` inside a submodule is accepted by the loader.
#[test]
fn verify_submodule_contract_forbid_is_accepted() {
    let fixture = auth_crate_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module.submodules]]\nname = \"auth::internal\"\nmatches = { modules = [\"auth::internal\"] }\ncontract = { forbid = [\"entity\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    let err = stderr(&output);
    assert!(
        !err.contains("invalid spec:") && !err.contains("contract.expose"),
        "enforced submodule forbid must load cleanly:\n{err}"
    );
}

/// A single-package crate `a` exposing module `b`; the caller supplies the
/// body of `b` and any extra files, for the submodule contract enforcement
/// scenarios (#90-#93).
fn submodule_contract_fixture(
    b_body: &str,
    extra_files: &[(&str, &str)],
    spec_body: &str,
) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod b;\n");
    fixture.write("src/b.rs", b_body);
    for (path, content) in extra_files {
        fixture.write(path, content);
    }
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = {{ units = [\"a\"] }}\n\n{spec_body}"
        ),
    );
    fixture
}

/// #90: a submodule's `contract.forbid` is enforced on the boundary path.
/// Module `c` lives under `a::b`, so the forbidden stereotype bleeds across
/// the submodule boundary and the leak must be reported, not pass silently.
#[test]
fn verify_submodule_contract_forbid_reports_leak_crossing_boundary() {
    let fixture = submodule_contract_fixture(
        "pub mod c;\n",
        &[("src/b/c.rs", "pub fn c() {}\n")],
        "[[stereotype]]\nname = \"c\"\nmatch = { names = [\"c\"] }\n\n[[module.submodules]]\nname = \"a::b\"\nmatches = { modules = [\"a::b\"] }\ncontract = { forbid = [\"c\"] }\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["contract leak: a::b exposes c (forbidden)"],
    );
}

/// #91: the same contract stays clean while nothing matching the forbidden
/// stereotype lives under the submodule boundary.
#[test]
fn verify_submodule_contract_forbid_clean_when_boundary_untouched() {
    let fixture = submodule_contract_fixture(
        "pub fn b() {}\n",
        &[("src/c.rs", "pub fn c() {}\n")],
        "[[stereotype]]\nname = \"c\"\nmatch = { names = [\"c\"] }\n\n[[module.submodules]]\nname = \"a::b\"\nmatches = { modules = [\"a::b\"] }\ncontract = { forbid = [\"c\"] }\n",
    );
    fixture.write("src/lib.rs", "pub mod b;\npub mod c;\n");
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "untouched boundary must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "pass confirmation:\n{out}");
    assert!(!out.contains("contract leak"), "no leak expected:\n{out}");
}

/// #92: the recursion reaches nested submodules: a contract on `a::b::c`
/// applies to the `a::b::c` boundary path.
#[test]
fn verify_nested_submodule_contract_forbid_is_enforced() {
    let fixture = submodule_contract_fixture(
        "pub mod c;\n",
        &[
            ("src/b/c.rs", "pub mod legacy;\n"),
            ("src/b/c/legacy.rs", "pub fn legacy() {}\n"),
        ],
        "[[stereotype]]\nname = \"legacy\"\nmatch = { names = [\"legacy\"] }\n\n[[module.submodules]]\nname = \"a::b\"\nmatches = { modules = [\"a::b\"] }\n\n[[module.submodules.submodules]]\nname = \"a::b::c\"\nmatches = { modules = [\"a::b::c\"] }\ncontract = { forbid = [\"legacy\"] }\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["contract leak: a::b::c exposes legacy (forbidden)"],
    );
}

/// #93: a submodule `contract.forbid` naming no declared stereotype can never
/// engage, so it is a dead reference — the same warning-severity treatment a
/// top-level contract reference gets, worded identically.
#[test]
fn verify_submodule_contract_forbid_naming_no_stereotype_is_dead_reference() {
    let fixture = submodule_contract_fixture(
        "pub fn b() {}\n",
        &[],
        "[[module.submodules]]\nname = \"a::b\"\nmatches = { modules = [\"a::b\"] }\ncontract = { forbid = [\"ghost\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "dead reference is warning severity (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("dead reference:"), "report:\n{out}");
    assert!(
        out.contains("'a::b'"),
        "report names the submodule path:\n{out}"
    );
    assert!(out.contains("\"ghost\""), "report:\n{out}");
    assert!(out.contains("names no stereotype"), "report:\n{out}");
    assert!(
        !out.contains("contract leak"),
        "no leak without stereotype:\n{out}"
    );

    let strict = fixture.run(&["verify", "--strict"]);
    assert_ne!(
        strict.status.code(),
        Some(0),
        "--strict promotes the dead reference (stderr: {})",
        stderr(&strict)
    );
}

#[test]
fn verify_invalid_severity_is_a_schema_error() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"auth\"]\nseverity = \"warn\"\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains("invalid severity \"warn\""),
        "must reject unknown severity:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_scenario_fifteen_divergent_report_is_byte_identical() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"ghost\"\nmatches = { units = [\"ghost\"] }\n",
    );
    let first = fixture.run(&["verify"]);
    let second = fixture.run(&["verify"]);

    assert_ne!(first.status.code(), Some(0), "divergence must fail");
    assert_ne!(second.status.code(), Some(0), "divergence must fail");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "diff report must be byte-identical across runs"
    );
}

#[test]
fn verify_scenario_twenty_one_no_sources_names_where() {
    let fixture = common::Fixture::new();
    fixture.write("architecture.spec.toml", "[project]\nlanguage = \"rust\"\n");
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("no supported-language sources found under:"),
        "must say no sources and where:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no report on operational error");
}

#[test]
fn verify_scenario_twenty_two_parse_failure_names_file_no_partial_diff() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "fn broken( {\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("failed to parse source:") && stderr(&output).contains("lib.rs"),
        "must name the failing file:\n{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "no partial diff on parse failure"
    );
}

#[test]
fn verify_scenario_twenty_three_missing_driver_suggests_doctor() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"example.com/demo\"] }\n",
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_archspec"))
        .arg("verify")
        .current_dir(&fixture.root)
        .env("ARCHSPEC_DISABLE_DRIVERS", "go")
        .output()
        .expect("run archspec");

    assert_ne!(output.status.code(), Some(0));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("no Go toolchain found (driver missing)")
            && err.contains("run 'archspec doctor' to diagnose"),
        "must name the language and suggest doctor:\n{err}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).is_empty(),
        "no report on operational error"
    );
}

/// A single-package Rust crate with two internal modules. Shared by the
/// shape-agnostic module-tier scenarios (behaviors 3 and 6).
fn single_crate_module_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod billing;\nmod config;\n");
    fixture.write("src/billing.rs", "pub fn bill() {}\n");
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture
}

#[test]
fn verify_single_crate_module_boundary_accounts_for_the_unit() {
    let fixture = single_crate_module_fixture();
    // The ONLY boundary matches internal modules via `matches.modules`; the
    // one unit (`app`) contains a matching module path, so it must be
    // accounted for (behavior 3).
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\", \"app::config\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "single-crate module boundary must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "pass confirmation:\n{out}");
    assert!(
        !out.contains("unexpected component: app"),
        "the unit app must not be reported as unexpected:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

#[test]
fn verify_module_boundary_does_not_cover_present_unit_reports_it_unexpected() {
    let fixture = single_crate_module_fixture();
    // A `matches.modules` boundary that does not cover the extracted `app`
    // unit: the unit matches neither tier and must be reported (behavior 6).
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { modules = [\"app::other\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("unexpected component: app"),
        "a unit with no matching boundary is reported unexpected:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on rule violation");
}

#[test]
fn verify_hybrid_unit_and_module_boundaries_together() {
    // Multi-crate workspace where one crate has internal modules; spec
    // declares a unit boundary and a module boundary together (behavior 5).
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/app\", \"crates/auth\"]\n",
    );
    fixture.write(
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/app/src/lib.rs", "mod billing;\nmod config;\n");
    fixture.write(
        "crates/app/src/billing.rs",
        "use crate::config;\npub fn bill() {}\n",
    );
    fixture.write("crates/app/src/config.rs", "pub fn config() {}\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n[module.allowed]\ndepend_on = [\"config\"]\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"app::config\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "hybrid boundaries must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "pass confirmation:\n{out}");
    assert!(
        !out.contains("unexpected component:") && !out.contains("missing component:"),
        "no spurious added/missing components:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

#[test]
fn verify_ignores_build_artifacts_under_target() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn ok() {}\n");
    // A broken generated .rs under target/ must not fail the run.
    fixture.write("target/debug/build/generated.rs", "fn broken( {\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "target/ artifacts must be ignored (stderr: {})",
        stderr(&output)
    );
    assert!(stdout(&output).starts_with("ok: "));
}

/// A single-package crate with a `commands` subtree and a `config` module.
/// Shared by the subtree-matching scenarios (behaviors 6a/6b).
fn commands_subtree_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod commands;\nmod config;\n");
    fixture.write(
        "src/commands/mod.rs",
        "pub mod init;\npub mod project_resolution;\npub mod start;\n",
    );
    fixture.write(
        "src/commands/start.rs",
        "use crate::config;\npub fn start() {}\n",
    );
    fixture.write("src/commands/init.rs", "pub fn init() {}\n");
    fixture.write(
        "src/commands/project_resolution.rs",
        "pub fn project_resolution() {}\n",
    );
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture
}

/// A `matches.modules = ["commands"]` boundary must cover every descendant of
/// the `commands` module (`commands::start`, `commands::init`,
/// `commands::project_resolution`), so the whole subtree satisfies the boundary
/// and the owning crate unit is accounted for (behavior 6a).
#[test]
fn verify_module_pattern_covers_the_whole_subtree() {
    let fixture = commands_subtree_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"commands\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "subtree pattern must cover every descendant (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.starts_with("ok: ")
            || (out.contains("warning: unowned module edge endpoint: app::config")
                && !out.contains("forbidden edge:")
                && !out.contains("disallowed cross-component dependency:")),
        "pass, or warning-only report for the unclaimed sibling module:\n{out}"
    );
    assert!(
        !out.contains("unexpected component: app"),
        "the unit owning the subtree must not be reported unexpected:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

/// A `forbidden = ["config"]` edge declared on a `commands` boundary must flag a
/// real `commands::start -> config` edge, resolved to the owning `commands`
/// boundary via the ancestor/subtree match (behavior 6b).
#[test]
fn verify_forbidden_edge_from_descendant_submodule_flags_boundary() {
    let fixture = commands_subtree_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"commands\"] }\n\n[module.allowed]\nforbidden = [\"config\"]\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"config\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_divergent(&output, &["forbidden edge: commands -> config"]);
}

/// A leaf pattern (`commands::start`) still matches exactly that module; the
/// subtree semantics are additive and never change what a leaf addresses
/// (behavior 6c).
#[test]
fn verify_leaf_module_pattern_matches_exactly_the_leaf() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod commands;\n");
    fixture.write("src/commands/mod.rs", "pub mod start;\n");
    fixture.write("src/commands/start.rs", "pub fn start() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"start\"\nmatches = { modules = [\"commands::start\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "leaf pattern must match exactly the leaf (stderr: {})",
        stderr(&output)
    );
    assert!(stdout(&output).starts_with("ok: "), "pass confirmation");
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

/// A trailing-`*` entry is a path-prefix claim: one `commands*` boundary covers
/// the whole `commands` subtree in a single entry, exactly like the bare
/// subtree form, and the unit owning it is accounted for.
#[test]
fn verify_trailing_star_prefix_group_covers_the_whole_subtree() {
    let fixture = commands_subtree_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"commands*\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "prefix entry must cover every descendant (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.starts_with("ok: ")
            || (out.contains("warning: unowned module edge endpoint: app::config")
                && !out.contains("forbidden edge:")
                && !out.contains("disallowed cross-component dependency:")),
        "pass, or warning-only report for the unclaimed sibling module:\n{out}"
    );
    assert!(
        !out.contains("unexpected component: app"),
        "the unit owning the prefix group must not be reported unexpected:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

/// An edge from ANY submodule inside a prefix group resolves to the group
/// boundary, so a `forbidden` on the group flags it.
#[test]
fn verify_prefix_group_flags_edge_from_any_submodule() {
    let fixture = commands_subtree_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"commands*\"] }\n\n[module.allowed]\nforbidden = [\"config\"]\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"config\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_divergent(&output, &["forbidden edge: commands -> config"]);
}

/// Specificity, not declaration order, decides ownership: the more-specific
/// prefix (`commands::start*`) claims `commands::start` over the broader one
/// (`commands*`), so the edge violation names the specific boundary.
#[test]
fn verify_more_specific_prefix_wins_over_broader_prefix() {
    let fixture = commands_subtree_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"commands*\"] }\n\n[[module]]\nname = \"start\"\nmatches = { modules = [\"commands::start*\"] }\n\n[module.allowed]\nforbidden = [\"config\"]\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"config\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_divergent(&output, &["forbidden edge: start -> config"]);
}

/// Two different boundaries claiming the same modules with equal specificity is
/// an ambiguity error naming the module path and both claiming entries — not a
/// silent declaration-order pick.
#[test]
fn verify_equal_specificity_prefixes_report_ambiguity() {
    let fixture = commands_subtree_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"commands*\"] }\n\n[[module]]\nname = \"commands_alias\"\nmatches = { modules = [\"commands*\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(
        output.status.code(),
        Some(0),
        "equal-specificity claims must fail (stdout: {})",
        stdout(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("ambiguous module match: app::commands::start claimed by both commands entry 'commands*' and commands_alias entry 'commands*'"),
        "ambiguity must name the module and both entries:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on rule violation");
}

/// Many module edges between the same pair of prefix groups aggregate into one
/// grouped dependency: the diff lists the pair exactly once.
#[test]
fn verify_grouped_edges_between_same_boundaries_are_deduplicated() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod commands;\nmod config;\n");
    fixture.write("src/commands/mod.rs", "pub mod start;\npub mod init;\n");
    fixture.write(
        "src/commands/start.rs",
        "use crate::config;\npub fn start() {}\n",
    );
    fixture.write(
        "src/commands/init.rs",
        "use crate::config;\npub fn init() {}\n",
    );
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"commands*\"] }\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"config\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let out = stdout(&output);
    let count = out
        .lines()
        .filter(|line| line.trim() == "disallowed cross-component dependency: commands -> config")
        .count();
    assert_eq!(
        count, 1,
        "the two module edges must aggregate into one grouped pair:\n{out}"
    );
}

/// A unit whose modules are claimed by no prefix group keeps behaving exactly
/// as before: the unit is reported, prefix grouping never hides it.
#[test]
fn verify_modules_outside_every_prefix_group_are_still_reported() {
    let fixture = commands_subtree_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"other*\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_ne!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("unexpected component: app"),
        "a unit outside every prefix group is still reported:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on rule violation");
}

/// A single crate with a `commands` subtree and a `config` module whose
/// submodule references the crate-root type, producing a crate-root module edge
/// (`app::commands::index -> app`). Shared by the crate-root endpoint scenarios.
fn crate_root_edge_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub struct App;\nmod commands;\nmod config;\n",
    );
    fixture.write("src/commands/mod.rs", "pub mod index;\n");
    fixture.write(
        "src/commands/index.rs",
        "use crate::App;\nuse crate::config;\npub fn index() {}\n",
    );
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture
}

/// #49: an internal edge whose endpoint is the crate root. The spec declares a
/// unit boundary via `matches.units` (`app`) and module boundaries via
/// `matches.modules`. The crate-root endpoint (`app`) resolves to the unit
/// boundary, so the edge is counted and a satisfied spec exits 0 with no
/// spurious missing-edge finding.
#[test]
fn verify_crate_root_edge_resolves_to_unit_boundary_and_passes() {
    let fixture = crate_root_edge_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"app::commands\"] }\n\n[module.allowed]\ndepend_on = [\"app\", \"config\"]\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"app::config\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "satisfied spec with a crate-root edge must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "pass confirmation:\n{out}");
    assert!(
        !out.contains("missing edge:") && !out.contains("disallowed cross-component dependency:"),
        "the crate-root edge must be counted, not dropped, and not flagged:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

/// #49 forbidden variant: a root-touching edge that is actually forbidden must
/// be CHECKED, not skipped. The `app::commands::index -> app` edge resolves to
/// `commands -> app`, which `commands` forbids.
#[test]
fn verify_forbidden_crate_root_edge_is_reported() {
    let fixture = crate_root_edge_fixture();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"commands\"\nmatches = { modules = [\"app::commands\"] }\n\n[module.allowed]\ndepend_on = [\"config\"]\nforbidden = [\"app\"]\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"app::config\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_divergent(&output, &["forbidden edge: commands -> app"]);
}

/// A single crate named `app` whose root lib.rs declares production modules
/// `prod` and `scanner`, plus a root-level `#[cfg(test)] mod tests` whose tests
/// use both production modules. This mirrors the common bin+lib tool shape: the
/// `app::tests` module produces edges into `app::prod` and `app::scanner`.
fn test_module_edge_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "mod prod;\nmod scanner;\n#[cfg(test)]\nmod tests;\n",
    );
    fixture.write("src/prod.rs", "pub fn prod() {}\n");
    fixture.write("src/scanner.rs", "pub fn scanner() {}\n");
    fixture.write(
        "src/tests.rs",
        "use crate::prod;\nuse crate::scanner;\n#[test]\nfn exercise() {}\n",
    );
    fixture
}

/// Design intent (#16/#18): `::tests` modules are test scaffolding that fold
/// into their top-level boundary and are not real boundaries. Edges FROM or TO
/// a `::tests` module must not produce boundary-level architecture findings —
/// a test module depending on production modules is normal.
#[test]
fn verify_test_module_edges_do_not_produce_disallowed_findings() {
    let fixture = test_module_edge_fixture();
    // Unit boundary + two production module boundaries. The `tests` module is
    // intentionally NOT declared; its edges into prod/scanner are scaffolding.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"app::prod\"\nmatches = { modules = [\"app::prod\"] }\n\n[[module]]\nname = \"app::scanner\"\nmatches = { modules = [\"app::scanner\"] }\n",
    );
    let output = fixture.run(&["verify"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "test-module edges must not fail verify (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "pass confirmation:\n{out}");
    assert!(
        !out.contains("disallowed cross-component dependency:"),
        "no test-module edge may be reported disallowed:\n{out}"
    );
    assert!(
        !out.contains("::tests"),
        "no boundary-level finding may name the test module:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

/// Excluding `::tests` scaffolding must NOT hide real violations: a production
/// module (not a test module) with a disallowed edge is still reported.
#[test]
fn verify_production_module_edges_still_checked() {
    let fixture = test_module_edge_fixture();
    // `prod` reaches `scanner` in production code but is not allowed to, while
    // the (undisallowed) `tests` module also reaches both. Only the production
    // violation may be reported.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"app::prod\"\nmatches = { modules = [\"app::prod\"] }\n\n[[module]]\nname = \"app::scanner\"\nmatches = { modules = [\"app::scanner\"] }\n",
    );
    // Override prod.rs to add the disallowed production edge.
    fixture.write("src/prod.rs", "use crate::scanner;\npub fn prod() {}\n");

    let output = fixture.run(&["verify"]);

    assert_ne!(
        output.status.code(),
        Some(0),
        "production disallowed edge must fail verify"
    );
    let out = stdout(&output);
    assert!(
        out.contains("disallowed cross-component dependency: app::prod -> app::scanner"),
        "production violation must still be reported:\n{out}"
    );
    assert!(
        !out.contains("::tests"),
        "test-module edges must still be excluded:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on rule violation");
}

/// A bin's `use <lib>::…` reference becomes a bin->lib unit edge. `update --force`
/// must seed a spec that declares that edge; a spec omitting the lib from the
/// bin's `depend_on` must then report it as a disallowed cross-component dep.
#[test]
fn verify_use_based_bin_to_lib_edge_is_modeled_and_enforceable() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn run() {}\n");
    fixture.write("src/main.rs", "use voice_app::run;\nfn main() { run(); }\n");

    // Prove the edge is modeled: `update --force` seeds the bin->lib depend_on.
    let updated = fixture.run(&["update", "--force"]);
    assert_eq!(updated.status.code(), Some(0), "update --force must exit 0");
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("voice-app-bin") && spec.contains("depend_on = [\"voice-app\"]"),
        "update --force must seed the use-based bin->lib edge in the spec:\n{spec}"
    );

    // Now omit the lib from the bin's depend_on; verify must flag the edge.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"voice-app\"\nmatches = { units = [\"voice-app\"] }\n\n[[module]]\nname = \"voice-app-bin\"\nmatches = { units = [\"voice-app-bin\"] }\n\n[module.allowed]\ndepend_on = []\n",
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["disallowed cross-component dependency: voice-app-bin -> voice-app"],
    );
}

/// Absence of the constraint must stay silent about cycles (no implicit
/// engine-level cycle check). Cyclic crates whose edges are all declared in
/// `allowed.depend_on` verify clean when the spec has no `no_cycles`
/// constraint anywhere. Pins the "existing specs unchanged" guarantee.
#[test]
fn cycle_without_no_cycles_constraint_verifies_clean() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[module.allowed]\ndepend_on = [\"billing\"]\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n",
    );

    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "no constraint must mean cycles are unmonitored: {}",
        stdout(&output)
    );
    let out = stdout(&output);
    assert!(!out.contains("cycle"), "no cycle finding allowed:\n{out}");
}

// ---------------------------------------------------------------------------
// F8: test-module exclusion is cfg-gated, not name-gated.
//
// A module literally named `tests` that is NOT `#[cfg(test)]` is production
// architecture and must participate in the boundary/cycle graph. Only modules
// the scan PROVED test-gated (a `#[cfg(test)]` declaration, or an equivalent
// test-only `cfg(all(..., test, ...))`) and their descendants are scaffolding.
// ---------------------------------------------------------------------------

/// One crate `app` with a production `prod` and a `tests` module that reference
// each other. `gate` selects whether `mod tests` carries `#[cfg(test)]`
// (scaffolding) or no cfg (production). `tests.rs` uses `crate::prod`;
// `prod.rs` uses `crate::tests` — a genuine mutual dependency.
fn tests_cycle_fixture(gated: bool) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let lib = if gated {
        "mod prod;\n#[cfg(test)]\nmod tests;\n"
    } else {
        "mod prod;\nmod tests;\n"
    };
    fixture.write("src/lib.rs", lib);
    fixture.write("src/prod.rs", "use crate::tests;\npub fn prod() {}\n");
    fixture.write("src/tests.rs", "use crate::prod;\npub fn t() {}\n");
    // Both production boundaries; no `depend_on`, so a real mutual edge shows up
    // as a cycle rather than a merely-disallowed dependency.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"app::prod\"\nmatches = { modules = [\"app::prod\"] }\n\n[[module]]\nname = \"app::tests\"\nmatches = { modules = [\"app::tests\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
    );
    fixture
}

/// A non-cfg `mod tests` is production architecture: a mutual dependency between
/// it and a production module is a REAL cycle and must fail `verify --strict`,
/// not be hidden by a name-based `::tests` exclusion.
#[test]
fn verify_non_cfg_tests_module_cycle_is_detected() {
    let fixture = tests_cycle_fixture(false);
    let output = fixture.run(&["verify", "--strict"]);

    assert_ne!(
        output.status.code(),
        Some(0),
        "a production (non-cfg) tests cycle must fail verify (stdout: {})",
        stdout(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("cycle: app::prod -> app::tests -> app::prod"),
        "the hidden cycle must be reported:\n{out}"
    );
}

/// The same mutual dependency through a `#[cfg(test)] mod tests` remains test
/// scaffolding: excluded from the graph, folding into the design as before. The
/// graph still carries a real production edge (`prod -> config`) so the cycle
/// check is engaged (not vacuous) yet must report no cycle — the tests module's
/// edge into production must not create one.
#[test]
fn verify_cfg_test_module_cycle_remains_excluded() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "mod prod;\nmod config;\n#[cfg(test)]\nmod tests;\n",
    );
    fixture.write("src/prod.rs", "use crate::config;\npub fn prod() {}\n");
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture.write("src/tests.rs", "use crate::prod;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"app::prod\"\nmatches = { modules = [\"app::prod\"] }\n\n[module.allowed]\ndepend_on = [\"app::config\"]\n\n[[module]]\nname = \"app::config\"\nmatches = { modules = [\"app::config\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
    );

    let output = fixture.run(&["verify", "--strict"]);
    let out = stdout(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "cfg(test) scaffolding must not fail verify (stderr: {})",
        stderr(&output)
    );
    assert!(
        !out.contains("cycle"),
        "cfg(test) scaffolding must not surface a cycle (nor vacuous cycle \
         warning):\n{out}"
    );
    assert!(
        !out.contains("::tests"),
        "no finding may name the cfg(test) tests module:\n{out}"
    );
}

/// A nested module under a `#[cfg(test)] mod tests` is still scaffolding: its
/// edges must be excluded by ancestry (a nested `tests::helpers` does not end in
/// `::tests`, so a name-only rule would leak it into the graph).
#[test]
fn verify_nested_module_under_cfg_test_tests_remains_excluded() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod prod;\n#[cfg(test)]\nmod tests;\n");
    fixture.write("src/prod.rs", "pub fn prod() {}\n");
    fixture.write("src/tests/mod.rs", "pub mod helpers;\n");
    fixture.write("src/tests/helpers.rs", "use crate::prod;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"app::prod\"\nmatches = { modules = [\"app::prod\"] }\n",
    );

    let output = fixture.run(&["verify", "--strict"]);
    let out = stdout(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a descendant of a cfg(test) tests module must not leak a finding \
         (disallowed/unowned edge):\n{out}{}\n",
        stderr(&output)
    );
    assert!(
        !out.contains("::tests"),
        "no finding may name the nested cfg(test) tests subtree:\n{out}"
    );
}

/// Workplan go_test_files_tier, scenario "test-only cross-package import
/// cannot create a cycle": package `a` imports `b` in production and `b`
/// imports `a` only from `b/a_test.go`. Seeding (`update`) then `verify
/// --strict` must report no cycle and exit 0 — the exact contract the rust
/// driver gives for `cfg(test)`-only back edges.
#[test]
fn verify_go_cycle_only_through_test_file_is_clean() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "a/a.go",
        "package a\n\nimport \"example.com/demo/b\"\n\nfunc A() { b.B() }\n",
    );
    fixture.write("b/b.go", "package b\n\nfunc B() {}\n");
    fixture.write(
        "b/a_test.go",
        "package b\n\nimport (\n\t\"testing\"\n\t\"example.com/demo/a\"\n)\n\nfunc TestA(t *testing.T) { a.A() }\n",
    );

    let seeded = fixture.run(&["update"]);
    assert_eq!(
        seeded.status.code(),
        Some(0),
        "update must seed (stderr: {})",
        stderr(&seeded)
    );

    let output = fixture.run(&["verify", "--strict"]);
    let out = stdout(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a cycle carried only by a *_test.go file must not fail verify \
         (rust cfg(test) parity), and the seeded diff must stay clean \
         (stderr: {}):\n{out}",
        stderr(&output)
    );
    assert!(
        !out.contains("cycle"),
        "no cycle may be reported through a test file:\n{out}"
    );
}

/// A single-go.mod Go tree whose spec declares one module per package: the
/// declared-grouping derivation gives the module-granularity checks a tier to
/// run on. Shared by the go declared-grouping scenarios below.
fn go_declared_grouping_fixture(spec: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    write_go_package(&fixture, "a", "a", &["example.com/demo/shell"]);
    write_go_package(&fixture, "shell", "shell", &["example.com/demo/store"]);
    write_go_package(&fixture, "store", "store", &[]);
    fixture.write("architecture.spec.toml", spec);
    fixture
}

/// Declared grouping makes laundered-edge detection run on go: the ban
/// `a -> store` has no direct edge, but the route rides the shell unit, which
/// no boundary claims through `matches.modules` — the same conduit shape the
/// rust driver reports. Pre-derivation the tier was absent and the checks
/// were silent on go.
#[test]
fn verify_go_declared_grouping_flags_laundered_forbidden_edge() {
    let fixture = go_declared_grouping_fixture(
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\n[module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"store\"]\n\n[[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/demo/shell\"] }\n\n[module.allowed]\ndepend_on = [\"store\"]\n\n[[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "laundering is a warning without --strict (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("warning: laundered forbidden edge: a -> store via shell"),
        "warning listed on stdout:\n{}",
        stdout(&output)
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_divergent(&strict, &["laundered forbidden edge: a -> store via shell"]);
}

/// A go.work workspace carries the module tier natively, so the
/// laundered-edge check enforces over workspace members the way it does for
/// csharp projects: the members are the modules, and the spec just names them.
#[test]
fn verify_go_work_members_enforce_laundered_forbidden_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "go.work",
        "go 1.21\n\nuse (\n\t./a\n\t./shell\n\t./store\n)\n",
    );
    fixture.write("a/go.mod", "module example.com/a\ngo 1.21\n");
    fixture.write(
        "a/a.go",
        "package a\n\nimport \"example.com/shell\"\n\nfunc A() {}\n",
    );
    fixture.write("shell/go.mod", "module example.com/shell\ngo 1.21\n");
    fixture.write(
        "shell/shell.go",
        "package shell\n\nimport \"example.com/store\"\n\nfunc Call() {}\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"example.com/a\"] }\n\n[module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"store\"]\n\n[[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/shell\"] }\n\n[module.allowed]\ndepend_on = [\"store\"]\n\n[[module]]\nname = \"store\"\nmatches = { units = [\"example.com/store\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "laundering is a warning without --strict (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("warning: laundered forbidden edge: a -> store via shell"),
        "workspace members enforce laundering:\n{}",
        stdout(&output)
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_divergent(&strict, &["laundered forbidden edge: a -> store via shell"]);
}

/// Declared grouping makes submodule contracts enforce on go: the submodule
/// claims the entity package through `matches.units`, so its `contract.forbid`
/// engages the package the parent imports and the leak is reported. A
/// single-`go.mod` tree now carries the module tier natively (derived from its
/// package references), which skips the declared-grouping derivation entirely,
/// and the contract's module-path branch reads soft module paths — which a
/// single-module go tree records none of — so a boundary declared only through
/// `matches.modules` stays inert (pinned below as the contrast leg).
#[test]
fn verify_go_declared_grouping_enforces_submodule_contract() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    write_go_package(&fixture, "auth", "auth", &["example.com/demo/auth/entity"]);
    write_go_package(&fixture, "auth/entity", "entity", &[]);
    let base = "[project]\nlanguage = \"go\"\n\n[[stereotype]]\nname = \"entity\"\nmatch = { paths = [\"example.com/demo/auth/entity\"] }\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\", \"example.com/demo/auth/entity\"] }\n";
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "{base}\n[[module.submodules]]\nname = \"auth::entity\"\nmatches = {{ units = [\"example.com/demo/auth/entity\"] }}\n\n[module.submodules.contract]\nforbid = [\"entity\"]\n"
        ),
    );
    assert_divergent(
        &fixture.run(&["verify"]),
        &["contract leak: auth::entity exposes entity (forbidden)"],
    );

    // Contrast leg: the same contract with the boundary declared through
    // `matches.modules` naming the package path. The native single-module tier
    // has no soft paths, so the boundary matches nothing, no constraint runs,
    // and the run is clean — the enforcement above must come from the unit
    // claim, not from this vocabulary.
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "{base}\n[[module.submodules]]\nname = \"auth::entity\"\nmatches = {{ modules = [\"example.com/demo/auth/entity\"] }}\n\n[module.submodules.contract]\nforbid = [\"entity\"]\n"
        ),
    );
    let inert = fixture.run(&["verify"]);
    assert_eq!(
        inert.status.code(),
        Some(0),
        "module-path boundary must stay inert on a single-module go tree (stderr: {})",
        stderr(&inert)
    );
    assert!(
        !stdout(&inert).contains("contract leak"),
        "the inert vocabulary must not report a leak:\n{}",
        stdout(&inert)
    );
}

/// A package no declared module covers is loud, not silently excluded from
/// the derived tier: the existing component classification names it, and
/// mapping it into a module turns the run clean.
#[test]
fn verify_go_declared_grouping_reports_uncovered_package_loudly() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    write_go_package(&fixture, "auth", "auth", &["example.com/demo/core"]);
    write_go_package(&fixture, "core", "core", &[]);
    write_go_package(&fixture, "util", "util", &[]);
    let base = "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\"] }\n\n[module.allowed]\ndepend_on = [\"core\"]\n\n[[module]]\nname = \"core\"\nmatches = { units = [\"example.com/demo/core\"] }\n";
    fixture.write("architecture.spec.toml", base);
    assert_divergent(
        &fixture.run(&["verify"]),
        &["unexpected component: example.com/demo/util"],
    );
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "{base}\n[[module]]\nname = \"util\"\nmatches = {{ units = [\"example.com/demo/util\"] }}\n"
        ),
    );
    let mapped = fixture.run(&["verify"]);
    assert_eq!(
        mapped.status.code(),
        Some(0),
        "mapping the package into a module clears the finding (stderr: {})",
        stderr(&mapped)
    );
    assert!(
        stdout(&mapped).starts_with("ok: "),
        "pass confirmation after mapping:\n{}",
        stdout(&mapped)
    );
}

/// The clean shape of declared grouping: every package mapped, every real
/// cross-module edge declared — the derived tier adds no findings.
#[test]
fn verify_go_declared_grouping_clean_spec_passes_with_checks_running() {
    let fixture = go_declared_grouping_fixture(
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\n[module.allowed]\ndepend_on = [\"shell\"]\n\n[[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/demo/shell\"] }\n\n[module.allowed]\ndepend_on = [\"store\"]\n\n[[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a mapped-clean tree passes even under --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "pass confirmation:\n{out}");
    assert!(
        !out.contains("laundered forbidden edge:") && !out.contains("disallowed cross-component"),
        "the derived tier must add no findings on a mapped-clean tree (the informational \
         inert-rule notes are not findings):\n{out}"
    );
}

/// The composition root of the go driver is the main package (US 03): its file
/// imports wire the graph without type positions. A ban whose route rides the
/// main package's legal hop is sanctioned wiring, not laundering — under
/// `--strict` the verify passes. The same tree with a named package (no
/// composition role anywhere) is the pure conduit again, still reported.
#[test]
fn verify_go_main_package_wiring_sanctions_the_composition_bridge() {
    let tree = |main_pkg: &str| {
        let fixture = common::Fixture::new();
        fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
        write_go_package(&fixture, "a", "a", &["example.com/demo/store"]);
        write_go_package(&fixture, "store", "store", &[]);
        write_go_package(&fixture, "cmd/server", main_pkg, &["example.com/demo/a"]);
        fixture.write(
            "architecture.spec.toml",
            "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\n[module.allowed]\ndepend_on = [\"store\"]\n\n[[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n\n[[module]]\nname = \"wiring\"\nmatches = { units = [\"example.com/demo/cmd/server\"] }\n\n[module.allowed]\ndepend_on = [\"a\"]\nforbidden = [\"store\"]\n",
        );
        fixture
    };
    // sanity: the main package carries the composition role
    let wired = tree("main");
    let model: serde_json::Value =
        serde_json::from_str(&stdout(&wired.run(&["scan"]))).expect("scan json");
    assert_eq!(
        model["roles"]["example.com/demo/cmd/server"].as_str(),
        Some("composition"),
        "the go composition root is keyed at the main package's import path:\n{}",
        model["roles"]
    );
    let output = wired.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the composition root's wiring hop is sanctioned, not laundering (stdout: {})",
        stdout(&output)
    );
    // same edges, no main package: no composition entry, conduit reported
    let conduit = tree("server");
    let output = conduit.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "strict laundering must exit 1"
    );
    assert!(
        stdout(&output).contains("laundered forbidden edge"),
        "the named-package route stays a conduit:\n{}",
        stdout(&output)
    );
}

/// The identity-split geometry of the C# audit (workplan
/// archspec_audit_defects US 05 / ADR-018), in neutral names: three projects —
/// `Shop.Api` referencing `Shop.Data` directly, `Shop.Data` referencing
/// `Shop.Domain` (so `Shop.Domain` is TRANSITIVELY reachable from the api),
/// each project's namespace mirrored as a second, allowance-less boundary
/// identity (`Shop::Api` etc., `matches.modules`) beside the unit boundary
/// (`matches.units`) that carries the allowances. The api project's
/// composition-root file (`Program.cs`, no namespace declaration) wires the
/// Data services and a controller uses + injects a Data interface: every
/// dependency is stated once as a hard unit edge (ProjectReference) and once
/// attributed under the namespace identity. `with_domain_wiring` adds the
/// companion shape: the composition root also wires a `Shop.Domain` type that
/// no rule declares for the api unit.
fn identity_split_fixture(with_domain_wiring: bool) -> common::Fixture {
    let fixture = common::Fixture::new();
    let csproj = |references: &[&str]| {
        let mut text = String::from(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n",
        );
        if !references.is_empty() {
            text.push_str("  <ItemGroup>\n");
            for reference in references {
                text.push_str(&format!(
                    "    <ProjectReference Include=\"..\\{reference}\\{reference}.csproj\" />\n"
                ));
            }
            text.push_str("  </ItemGroup>\n");
        }
        text.push_str("</Project>\n");
        text
    };
    fixture.write("Shop.Api/Shop.Api.csproj", &csproj(&["Shop.Data"]));
    fixture.write(
        "Shop.Api/Program.cs",
        &format!(
            "using Shop.Data;\n{}\nvar builder = WebApplication.CreateBuilder(args);\nbuilder.Build().Run();\n",
            if with_domain_wiring {
                "using Shop.Domain;\nbuilder.Services.AddScoped<IOrderService, OrderRepository>();\nbuilder.Services.AddScoped<IOrderRepository, OrderRepository>();"
            } else {
                "builder.Services.AddScoped<IOrderService, OrderService>();"
            }
        ),
    );
    fixture.write(
        "Shop.Api/Controllers/OrderController.cs",
        "using Shop.Data;\nnamespace Shop.Api.Controllers;\npublic class OrderController\n{\n    private readonly IOrderService _service;\n    public OrderController(IOrderService service)\n    {\n        _service = service;\n    }\n}\n",
    );
    fixture.write("Shop.Data/Shop.Data.csproj", &csproj(&["Shop.Domain"]));
    fixture.write(
        "Shop.Data/OrderService.cs",
        "using Shop.Domain;\nnamespace Shop.Data;\npublic class OrderService { private readonly IOrderRepository _repo; public OrderService(IOrderRepository repo) { _repo = repo; } }\npublic class IOrderService { }\npublic class OrderRepository : IOrderRepository { }\n",
    );
    fixture.write("Shop.Domain/Shop.Domain.csproj", &csproj(&[]));
    fixture.write(
        "Shop.Domain/Contracts.cs",
        "namespace Shop.Domain;\npublic interface IOrderRepository { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Shop.Api\"\nmatches = { units = [\"Shop.Api\"] }\nallowed = { depend_on = [\"Shop.Data\"] }\n\n[[module]]\nname = \"Shop.Data\"\nmatches = { units = [\"Shop.Data\"] }\nallowed = { depend_on = [\"Shop.Domain\"] }\n\n[[module]]\nname = \"Shop.Domain\"\nmatches = { units = [\"Shop.Domain\"] }\n\n[[module]]\nname = \"Shop::Api\"\nmatches = { modules = [\"Shop::Api\"] }\n\n[[module]]\nname = \"Shop::Data\"\nmatches = { modules = [\"Shop::Data\"] }\n\n[[module]]\nname = \"Shop::Domain\"\nmatches = { modules = [\"Shop::Domain\"] }\n",
    );
    fixture
}

/// ADR-018 / audit defect D5, the audit's finding shape: every dependency is
/// stated once as a unit edge (allowed by the unit boundary's `depend_on`)
/// and once attributed under the namespace identity, where the mirrored
/// boundary grants nothing. The allowances are owned by the unit boundary, so
/// a unit-tier allowance survives namespace attribution: zero `disallowed
/// cross-component dependency` findings. Before the fix this failed with the
/// audit's warnings: the namespace pair was re-adjudicated where no allowance
/// was visible.
#[test]
fn unit_tier_allowance_survives_namespace_attribution() {
    let fixture = identity_split_fixture(false);
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "namespace-attributed twins of allowed unit edges must not void the grant (stdout: {})",
        stdout(&output)
    );
    let out = stdout(&output);
    assert!(
        !out.contains("disallowed cross-component dependency:"),
        "no identity-split dependency may be reported disallowed:\n{out}"
    );
    assert!(stderr(&output).is_empty(), "no stderr on pass");
}

/// The fix must not become an exemption hole (ADR-018 Consequences): a
/// dependency attributed under the namespace identity that the owning unit
/// boundary does not name is still reported. The composition-root file wires
/// a `Shop.Domain` type reached through the reference chain only — no unit
/// edge states it, no `depend_on` names it — so the owning unit boundary's
/// default-deny governs and the pair stays a finding, text unchanged.
#[test]
fn undeclared_target_under_namespace_attribution_stays_disallowed() {
    let fixture = identity_split_fixture(true);
    let output = fixture.run(&["verify"]);
    assert_divergent(
        &output,
        &["disallowed cross-component dependency: Shop::Api -> Shop::Domain"],
    );
}
