//! GitHub issue #10 — two-layer `depend_on` adjudication and the named
//! reason classes of the `missing edge` verdict. The floor reads the pair
//! graph (unit-tier manifest edges unioned with attributed module-tier import
//! edges); the ceiling engages exactly when an attributed pair crosses a
//! boundary. Test-gated evidence satisfies neither. Each cell pins one
//! reason class end to end on raw fixtures; the report cell pins the two
//! sub-counts of the edges category that split the same finding families.

mod common;

use common::{stderr, stdout};

/// cell 1 — rust, zero evidence: `a` declares `depend_on b` while neither a
/// manifest edge nor an import evidences the pair. The verdict must name the
/// reason class `(no import evidence for this pair)`.
#[test]
fn missing_edge_without_evidence_names_the_no_evidence_reason() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\n",
    );
    fixture.write(
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/a/src/lib.rs", "pub fn a() {}\n");
    fixture.write(
        "crates/b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/b/src/lib.rs", "pub fn b() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[module.allowed]\ndepend_on = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\n",
    );

    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the phantom edge must fail (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("missing edge: a -> b (no import evidence for this pair)"),
        "the verdict must name the zero-evidence reason class:\n{out}"
    );
}

/// cell 2 — ts, test-gated evidence only: the only import from `a`'s
/// territory into `b` sits in a test file of a package that declares vitest,
/// so the pair is exercised by nothing but test-gated code. The verdict must
/// name `(test-gated import evidence only)`.
#[test]
fn missing_edge_with_only_test_gated_evidence_names_the_test_gated_reason() {
    let fixture = common::Fixture::new();
    fixture.write(
        "package.json",
        "{\n  \"name\": \"app\",\n  \"devDependencies\": { \"vitest\": \"^1.0.0\" }\n}\n",
    );
    fixture.write("src/a.ts", "export const a = 1;\n");
    fixture.write("src/b.ts", "export const b = 1;\n");
    fixture.write("src/a.test.ts", "import { b } from './b';\nexport const t = b;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"app\"], modules = [\"src/a.ts\"] }\n\n[module.allowed]\ndepend_on = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"src/b.ts\"] }\n",
    );

    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "test-gated evidence must not satisfy the floor (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("missing edge: a -> b (test-gated import evidence only)"),
        "the verdict must name the test-gated reason class:\n{out}"
    );
}

/// cell 3 — ts workspace, unowned import evidence: package `c` (claimed by
/// no module) links `b`, so manifest import evidence touches the target from
/// territory no claim owns while `a -> b` itself stays unevidenced. The
/// verdict must name `(import evidence exists but no module claim owns this
/// pair)`; adding the pair to package `a`'s devDependencies exercises the
/// floor and the verdict goes away.
#[test]
fn missing_edge_with_unowned_evidence_names_the_unowned_reason() {
    let fixture = common::Fixture::new();
    fixture.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    fixture.write(
        "packages/a/package.json",
        "{\n  \"name\": \"a\",\n  \"version\": \"1.0.0\"\n}\n",
    );
    fixture.write("packages/a/src/index.ts", "export const a = 1;\n");
    fixture.write(
        "packages/b/package.json",
        "{\n  \"name\": \"b\",\n  \"version\": \"1.0.0\"\n}\n",
    );
    fixture.write("packages/b/src/index.ts", "export const b = 1;\n");
    fixture.write(
        "packages/c/package.json",
        "{\n  \"name\": \"c\",\n  \"version\": \"1.0.0\",\n  \"dependencies\": { \"b\": \"1.0.0\" }\n}\n",
    );
    fixture.write("packages/c/src/index.ts", "export const c = 1;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[module.allowed]\ndepend_on = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\n",
    );

    let output = fixture.run(&["verify"]);
    let out = stdout(&output);
    assert!(
        out.contains("missing edge: a -> b (import evidence exists but no module claim owns this pair)"),
        "the verdict must name the unowned-evidence reason class:\n{out}"
    );

    // The unit tier reads devDependencies: declaring the pair in package a's
    // manifest exercises the floor, and the verdict is gone.
    fixture.write(
        "packages/a/package.json",
        "{\n  \"name\": \"a\",\n  \"version\": \"1.0.0\",\n  \"devDependencies\": { \"b\": \"1.0.0\" }\n}\n",
    );
    let second = fixture.run(&["verify"]);
    let out = stdout(&second);
    assert!(
        !out.contains("missing edge: a -> b"),
        "devDependencies satisfy the floor — the verdict must be gone:\n{out}"
    );
}

/// cell 4 — ts, the ceiling side: the same test file is NOT test-gated when
/// the package manifest declares no vitest/jest runner, so the attributed
/// pair engages the ceiling: `disallowed cross-component dependency` fires
/// for the undeclared pair (and the floor, which declares nothing here,
/// stays silent).
#[test]
fn ungated_test_file_evidence_engages_the_ceiling() {
    let fixture = common::Fixture::new();
    fixture.write(
        "package.json",
        "{\n  \"name\": \"app\",\n  \"devDependencies\": {}\n}\n",
    );
    fixture.write("src/a.ts", "export const a = 1;\n");
    fixture.write("src/b.ts", "export const b = 1;\n");
    fixture.write("src/a.test.ts", "import { b } from './b';\nexport const t = b;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"app\"], modules = [\"src/a.ts\"] }\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"src/b.ts\"] }\n",
    );

    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an ungated attributed pair must engage the ceiling (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("disallowed cross-component dependency: a -> b"),
        "the ceiling fires for the pair the test gate no longer hides:\n{out}"
    );
    assert!(
        !out.contains("missing edge: a -> b"),
        "nothing is declared, so the floor engages nothing:\n{out}"
    );
}

/// cell 5 — report sub-counts: the edges category splits exactly into
/// undeclared dependencies (missing + disallowed) and boundary violations
/// (forbidden + facade), and both renderers state the arithmetic.
#[test]
fn report_edges_row_splits_into_the_two_sub_counts() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/api\", \"crates/dom\", \"crates/ext\"]\n",
    );
    for (name, deps, lib) in [
        ("api", "ext = { path = \"../ext\" }", "pub fn api() {}\n"),
        ("dom", "api = { path = \"../api\" }", "use api::api;\npub fn dom() { api(); }\n"),
        ("ext", "", "pub fn ext() {}\n"),
    ] {
        fixture.write(
            &format!("crates/{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}\n"
            ),
        );
        fixture.write(&format!("crates/{name}/src/lib.rs"), lib);
    }
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"api\"\nmatches = { units = [\"api\"] }\n\n[[module]]\nname = \"dom\"\nmatches = { units = [\"dom\"] }\n\n[module.allowed]\nforbidden = [\"api\"]\ndepend_on = [\"ext\"]\n\n[[module]]\nname = \"ext\"\nmatches = { units = [\"ext\"] }\n",
    );

    let output = fixture.run(&["report"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "violations demand a non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("forbidden edge present: dom -> api"),
        "boundary class:\n{out}"
    );
    assert!(
        out.contains("disallowed cross-component dependency: api -> ext"),
        "undeclared class:\n{out}"
    );
    assert!(
        out.contains("missing edge: dom -> ext (no import evidence for this pair)"),
        "undeclared class with the zero-evidence reason:\n{out}"
    );
    let categories = |name: &str| -> usize {
        out.lines()
            .skip_while(|line| !line.starts_with("Violations by category"))
            .skip(2)
            .find(|line| line.starts_with(&format!("{name}:")))
            .and_then(|line| line.split(':').nth(1))
            .map(str::trim)
            .unwrap_or("0")
            .parse()
            .expect("category count")
    };
    let edges = categories("edges");
    let undeclared = categories("undeclared-dependency");
    let boundary = categories("boundary-violation");
    assert_eq!(edges, 3, "edges row:\n{out}");
    assert_eq!(undeclared, 2, "undeclared-dependency sub-count:\n{out}");
    assert_eq!(boundary, 1, "boundary-violation sub-count:\n{out}");
    assert_eq!(
        undeclared + boundary,
        edges,
        "the sub-counts must sum to the edges row"
    );

    let json = fixture.run(&["report", "--format", "json"]);
    assert_eq!(
        json.status.code(),
        Some(1),
        "violations fail the run in every format (stderr: {})",
        stderr(&json)
    );
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&json)).expect("report json parses");
    let metrics = &value["metrics"];
    assert_eq!(metrics["violations_edges"].as_u64().unwrap(), 3);
    assert_eq!(metrics["violations_undeclared_dependency"].as_u64().unwrap(), 2);
    assert_eq!(metrics["violations_boundary_violation"].as_u64().unwrap(), 1);
    assert_eq!(
        metrics["violations_undeclared_dependency"].as_u64().unwrap()
            + metrics["violations_boundary_violation"].as_u64().unwrap(),
        metrics["violations_edges"].as_u64().unwrap(),
        "the json sub-counts sum to the edges count"
    );
}
