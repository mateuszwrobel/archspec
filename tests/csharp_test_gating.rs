//! GitHub issue #39 cells — `Sdk="MSTest.Sdk"` projects and
//! `Microsoft.Testing.Platform`- / `MSTest`-family package ids join the C#
//! test tier exactly like xunit/nunit projects, and the scan never drops a
//! test project silently: the report surface carries one
//! `test-gated unit: <path> — <marker>` census line per dropped project and
//! the verdict/report surfaces add the `test-gated units: <n>` census note.
//!
//! The fixture is the issue's repro tree verbatim: a `Microsoft.NET.Sdk` lib,
//! an `MSTest.Sdk` project (project-shape evidence, no runner package at
//! all), and an xunit project whose marker is the `xunit.v3.mtp-v2` package
//! id. The machine body stays test-blind: the model bytes equal the tree
//! with the test projects removed, and the census lives on stderr.

mod common;

use common::Fixture;
use serde_json::Value;

/// The census forms pinned by the claim registry (`csharp-test-gated-units`,
/// owner surface: `help diagnostics`). The dedicated guard for those
/// placeholder bytes is this file: the exact-bytes cells below render them.
#[allow(dead_code)]
const CENSUS_FORMS: [&str; 2] = [
    "test-gated unit: <path> — <marker>",
    "test-gated units: <n>",
];

fn lib_csproj() -> &'static str {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n</Project>\n"
}

/// The issue repro tree: lib plus the two test shapes (MSTest project-SDK
/// evidence, xunit package evidence), files copied from the reported layout.
fn repro_tree() -> Fixture {
    let fx = Fixture::new();
    fx.write("lib/Lib.csproj", lib_csproj());
    fx.write("lib/Lib.cs", "namespace Repro.Lib { public class Greeter { } }\n");
    fx.write(
        "tests/App.UnitTests/App.UnitTests.csproj",
        "<Project Sdk=\"MSTest.Sdk\">\n<ItemGroup><ProjectReference Include=\"../../lib/Lib.csproj\"/></ItemGroup>\n</Project>\n",
    );
    fx.write(
        "tests/App.UnitTests/Tests.cs",
        "using Repro.Lib; namespace Repro.UnitTests { class T { } }\n",
    );
    fx.write(
        "tests/App.XunitTests/App.XunitTests.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n<ItemGroup><PackageReference Include=\"xunit.v3.mtp-v2\" Version=\"0.2.0\"/>\n<ProjectReference Include=\"../../lib/Lib.csproj\"/></ItemGroup>\n</Project>\n",
    );
    fx.write(
        "tests/App.XunitTests/Tests.cs",
        "using Repro.Lib; namespace Repro.XunitTests { class T { } }\n",
    );
    fx
}

/// The repro tree without the test projects — the machine-body equal.
fn production_tree() -> Fixture {
    let fx = Fixture::new();
    fx.write("lib/Lib.csproj", lib_csproj());
    fx.write("lib/Lib.cs", "namespace Repro.Lib { public class Greeter { } }\n");
    fx
}

fn spec() -> &'static str {
    "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Lib\"\nmatches = { units = [\"Lib\"] }\n"
}

fn census_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("test-gated unit: "))
        .map(str::to_string)
        .collect()
}

fn unit_names(model: &Value) -> Vec<String> {
    model["units"]
        .as_array()
        .map(|units| {
            units
                .iter()
                .filter_map(|unit| unit["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn mstest_sdk_and_package_marker_projects_land_in_the_test_tier() {
    let fx = repro_tree();
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(
        unit_names(&model),
        vec!["Lib".to_string()],
        "the lib is the only production unit — the MSTest.Sdk project and the \
         xunit project are both test tier (issue #39)"
    );
    for marker in ["MSTest.Sdk", "xunit.v3.mtp-v2"] {
        assert!(
            !model["external"]
                .as_object()
                .is_some_and(|external| external.contains_key(marker)),
            "dropped test projects leave no marker in the external tier: {:?}",
            model["external"]
        );
    }
}

#[test]
fn scan_names_every_dropped_project_in_sorted_order_exact_bytes() {
    let fx = repro_tree();
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        census_lines(&scan),
        vec![
            "test-gated unit: tests/App.UnitTests — MSTest.Sdk",
            "test-gated unit: tests/App.XunitTests — xunit.v3.mtp-v2",
        ],
        "one census line per dropped project, path = the project dir, marker = \
         the matched package id else the Sdk attribute, sorted project order"
    );
}

#[test]
fn machine_body_equals_the_tree_without_test_projects() {
    let with_tests = repro_tree();
    let without = production_tree();
    let a = with_tests.run(&["scan", "."]);
    let b = without.run(&["scan", "."]);
    assert!(
        a.status.success() && b.status.success(),
        "scans failed: {} | {}",
        common::stderr(&a),
        common::stderr(&b)
    );
    assert_eq!(
        common::stdout(&a),
        common::stdout(&b),
        "the model bytes are test-blind: the census lives on stderr only"
    );
    assert!(
        !common::stdout(&a).contains("test-gated"),
        "the machine body carries no test field"
    );
}

#[test]
fn verify_surfaces_carry_the_census_note_and_class() {
    let fx = repro_tree();
    fx.write("architecture.spec.toml", spec());
    let verify = fx.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "verify failed: {}",
        common::stderr(&verify)
    );
    let out = common::stdout(&verify);
    assert!(out.starts_with("ok: "), "pass confirmation first:\n{out}");
    assert!(
        out.contains("test-gated units: 2\n"),
        "the census note rides the verdict surface:\n{out}"
    );
    let row = out
        .lines()
        .find(|line| line.starts_with("test-gated unit ") && !line.contains("units"))
        .unwrap_or_else(|| panic!("the class table names the census:\n{out}"));
    assert!(row.trim_end().ends_with('2'), "class row counts the lines: {row}");
    assert_eq!(
        census_lines(&verify),
        vec![
            "test-gated unit: tests/App.UnitTests — MSTest.Sdk",
            "test-gated unit: tests/App.XunitTests — xunit.v3.mtp-v2",
        ],
        "verify prints the same census lines on the report surface"
    );

    let json = fx.run(&["verify", "--json", "."]);
    assert!(
        json.status.success(),
        "verify --json failed: {}",
        common::stderr(&json)
    );
    let out = common::stdout(&json);
    assert!(
        !out.contains("test-gated units:"),
        "the json verdict stays byte-clean of the note rendering"
    );
    let value: Value = serde_json::from_str(&out).expect("json verdict");
    assert_eq!(
        value["environment"]["classes"]["test-gated unit"].as_u64(),
        Some(2),
        "the json environment states the census through its classes field"
    );
}

#[test]
fn report_text_and_markdown_state_the_note() {
    let fx = repro_tree();
    fx.write("architecture.spec.toml", spec());
    for args in [
        ["report", "."].as_slice(),
        ["report", ".", "--format", "markdown"].as_slice(),
    ] {
        let report = fx.run(args);
        assert!(
            report.status.success(),
            "`{}` failed: {}",
            args.join(" "),
            common::stderr(&report)
        );
        assert!(
            common::stdout(&report).contains("test-gated units: 2"),
            "the report surface states the census:\n{}",
            common::stdout(&report)
        );
    }
}

#[test]
fn versioned_mstest_sdk_spelling_is_test_tier_with_verbatim_marker() {
    let fx = Fixture::new();
    fx.write(
        "Feed/Feed.csproj",
        "<Project Sdk=\"MSTest.Sdk/3.6.4\">\n  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>\n</Project>\n",
    );
    fx.write(
        "Feed/Tests.cs",
        "namespace Feed.Tests; public class T { }\n",
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(unit_names(&model), Vec::<String>::new());
    assert_eq!(
        census_lines(&scan),
        vec!["test-gated unit: Feed — MSTest.Sdk/3.6.4"],
        "the versioned Sdk attribute is a marker and its value is the verbatim marker"
    );
}

#[test]
fn trees_without_test_tier_stay_silent_and_note_free() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>\n  <ItemGroup><PackageReference Include=\"FluentAssertions\" Version=\"6.0.0\"/></ItemGroup>\n</Project>\n");
    fx.write("App/App.cs", "namespace App; public class A { }\n");
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n",
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(
        unit_names(&model),
        vec!["App".to_string()],
        "an assertion library alone is production tooling, not a runner marker"
    );
    assert!(
        common::stderr(&scan).is_empty(),
        "no dropped project, no census line: {}",
        common::stderr(&scan)
    );
    let verify = fx.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "verify failed: {}",
        common::stderr(&verify)
    );
    let out = common::stdout(&verify);
    assert!(
        !out.contains("test-gated units:"),
        "zero-inert: no test tier, no note:\n{out}"
    );
}
