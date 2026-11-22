//! GitHub issue #39 cells — `Sdk="MSTest.Sdk"` projects and
//! `Microsoft.Testing.Platform`- / `MSTest`-family package ids join the C#
//! test tier exactly like xunit/nunit projects, and the scan never drops a
//! test project silently: the report surface carries one
//! `test-gated unit: <path> — <marker>` census line per dropped project and
//! the verdict/report surfaces add the `test-gated units: <n>` census note.
//!
//! GitHub issue #45 rung 1 cells — a test-runner `PackageReference` under an
//! unevaluated MSBuild `Condition` (on the item, its `ItemGroup`, or inside
//! a `Choose`/`When`/`Otherwise` subtree) is NOT gating evidence: it reaches
//! no tier and states itself once as the report-only
//! `test marker under unevaluated condition: <file>:<line>` line. Condition
//! evaluation ships the designed subset (workplan `csharp_condition_eval`
//! scenarios 01–02): a TRUE marker gates and its project states the
//! `test-gated unit` census row, a FALSE marker leaves the item absent, and
//! only an unevaluated condition keeps this rung-1 row;
//! `Directory.Build.targets` and props brought in by `<Import>` are
//! non-goals — named at runtime by the `condition face not walked:` row
//! (workplan csharp_condition_eval scenario 04).
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

/// The suppression form pinned by the claim registry
/// (`csharp-conditioned-test-markers`, owner surface: `help diagnostics`):
/// the rung-1 diagnostic for a test marker under an unevaluated MSBuild
/// `Condition`. Welded by its own const — `CENSUS_FORMS` stays welded to
/// the issue #39 claim and the two families never merge.
#[allow(dead_code)]
const CONDITION_FORMS: [&str; 1] = ["test marker under unevaluated condition: <file>:<line>"];

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

fn marker_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("test marker under unevaluated condition: "))
        .map(str::to_string)
        .collect()
}

fn assert_machine_body_clean(output: &std::process::Output, label: &str) {
    assert!(
        output.status.success(),
        "{label}: scan failed: {}",
        common::stderr(output)
    );
    let out = common::stdout(output);
    let _: Value = serde_json::from_str(&out).expect("scan stdout is JSON");
    assert!(
        !out.contains("test marker under unevaluated condition"),
        "{label}: the diagnostics never join the machine body:\n{out}"
    );
}

/// Cell 1 (issue #45): a central props `Choose` with a `When` arm and an
/// `Otherwise` arm, each ItemGroup wiring an xunit-family marker. The lib
/// stays production — `Choose` arms are unevaluated, so no marker gates —
/// and each arm gets exactly one diagnostic line.
#[test]
fn choose_when_otherwise_markers_diagnose_and_never_gate() {
    let fx = Fixture::new();
    fx.write(
        "Directory.Packages.props",
        "<Project>\n<Choose>\n  <When Condition=\"'$(IsTestProject)'=='true'\">\n    <ItemGroup>\n      <PackageReference Include=\"xunit\" Version=\"2.9.0\" />\n    </ItemGroup>\n  </When>\n  <Otherwise>\n    <ItemGroup>\n      <PackageReference Include=\"xunit.core\" Version=\"2.9.0\" />\n    </ItemGroup>\n  </Otherwise>\n</Choose>\n</Project>\n",
    );
    fx.write("lib/Lib.csproj", lib_csproj());
    fx.write("lib/Lib.cs", "namespace Repro.Lib { public class Greeter { } }\n");
    let scan = fx.run(&["scan", "."]);
    assert_machine_body_clean(&scan, "choose-arms");
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(
        unit_names(&model),
        vec!["Lib".to_string()],
        "a Choose-subtree marker must not test-gate the project below the props"
    );
    assert_eq!(
        marker_lines(&scan),
        vec![
            "test marker under unevaluated condition: Directory.Packages.props:5".to_string(),
            "test marker under unevaluated condition: Directory.Packages.props:10".to_string(),
        ],
        "one diagnostic per marker arm, at the line holding the item"
    );
    assert_eq!(
        census_lines(&scan),
        Vec::<String>::new(),
        "no project is dropped — the conditioned markers are not gating evidence"
    );
    assert!(
        !common::stdout(&scan).contains("xunit"),
        "conditioned markers reach no tier of the machine body"
    );
}

/// Cell 2: the props variants — an `ItemGroup Condition` and an item-level
/// `Condition` on the `PackageReference`. The marker is excluded from
/// gating and each variant prints exactly one diagnostic.
#[test]
fn props_item_group_and_item_level_conditions_diagnose_once() {
    for (label, props) in [
        (
            "item-group-condition",
            "<Project>\n<ItemGroup Condition=\"'$(IsTestProject)'=='true'\">\n  <PackageReference Include=\"xunit\" Version=\"2.9.0\" />\n</ItemGroup>\n</Project>\n",
        ),
        (
            "item-level-condition",
            "<Project>\n<ItemGroup>\n  <PackageReference Include=\"xunit\" Version=\"2.9.0\" Condition=\"'$(IsTestProject)'=='true'\" />\n</ItemGroup>\n</Project>\n",
        ),
    ] {
        let fx = Fixture::new();
        fx.write("Directory.Packages.props", props);
        fx.write("lib/Lib.csproj", lib_csproj());
        fx.write("lib/Lib.cs", "namespace Repro.Lib { public class Greeter { } }\n");
        let scan = fx.run(&["scan", "."]);
        assert_machine_body_clean(&scan, label);
        let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
        assert_eq!(
            unit_names(&model),
            vec!["Lib".to_string()],
            "{label}: the conditioned marker must not erase the production model"
        );
        assert_eq!(
            marker_lines(&scan),
            vec!["test marker under unevaluated condition: Directory.Packages.props:3".to_string()],
            "{label}: exactly one diagnostic, at the line holding the item"
        );
        assert!(
            !common::stdout(&scan).contains("xunit"),
            "{label}: the suppressed marker reaches no tier"
        );
    }
}

/// Cell 3 (control): the same trees WITHOUT conditions keep today's
/// behavior byte-identical — the test project is dropped, the
/// `test-gated unit:` census and `test-gated units:` note stand, the
/// machine body equals the tree with the test project removed, and no
/// marker diagnostic is printed.
#[test]
fn unconditional_markers_keep_the_test_gated_behavior() {
    let fx = Fixture::new();
    fx.write("lib/Lib.csproj", lib_csproj());
    fx.write("lib/Lib.cs", "namespace Repro.Lib { public class Greeter { } }\n");
    fx.write(
        "tests/Tests.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n<ItemGroup><PackageReference Include=\"xunit\" Version=\"2.9.0\"/></ItemGroup>\n</Project>\n",
    );
    fx.write(
        "tests/Tests.cs",
        "using Repro.Lib; namespace Repro.Tests { class T { } }\n",
    );
    let scan = fx.run(&["scan", "."]);
    assert_machine_body_clean(&scan, "control");
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(unit_names(&model), vec!["Lib".to_string()]);
    assert_eq!(
        census_lines(&scan),
        vec!["test-gated unit: tests — xunit".to_string()],
        "the unconditional marker still drops its project with the issue #39 census"
    );
    assert_eq!(
        marker_lines(&scan),
        Vec::<String>::new(),
        "no condition, no marker diagnostic"
    );
    let without = production_tree();
    let equal = without.run(&["scan", "."]);
    assert!(
        equal.status.success(),
        "control-equal scan failed: {}",
        common::stderr(&equal)
    );
    assert_eq!(
        common::stdout(&scan),
        common::stdout(&equal),
        "the machine body equals the tree with the test project removed"
    );
    fx.write("architecture.spec.toml", spec());
    let verify = fx.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "verify failed: {}",
        common::stderr(&verify)
    );
    assert!(
        common::stdout(&verify).contains("test-gated units: 1\n"),
        "the verdict note still counts the dropped project:\n{}",
        common::stdout(&verify)
    );
}

/// Cell 4 (bug b): a PRODUCTION csproj wiring `Microsoft.NET.Test.Sdk`
/// behind an item-level `Condition` — the project stays production and the
/// run prints exactly one diagnostic line.
#[test]
fn csproj_conditioned_test_sdk_stays_production_with_one_diagnostic() {
    let fx = Fixture::new();
    fx.write(
        "Api/Api.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n<ItemGroup><PackageReference Include=\"Microsoft.NET.Test.Sdk\" Condition=\"'$(Cfg)'=='Debug'\"/></ItemGroup>\n</Project>\n",
    );
    fx.write("Api/Api.cs", "namespace Api; public class A { }\n");
    let scan = fx.run(&["scan", "."]);
    assert_machine_body_clean(&scan, "csproj-condition");
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(
        unit_names(&model),
        vec!["Api".to_string()],
        "a conditioned Test.Sdk must not mark a production project test-gated"
    );
    assert_eq!(
        marker_lines(&scan),
        vec![
            "test marker under unevaluated condition: Api/Api.csproj:2".to_string(),
        ],
        "exactly one diagnostic for the conditioned csproj marker"
    );
    assert_eq!(census_lines(&scan), Vec::<String>::new());
    assert!(
        !common::stdout(&scan).contains("Microsoft.NET.Test.Sdk"),
        "the conditioned test marker is excluded from the dependency tier"
    );
}

/// Cell 5: a `Choose` with two `When` arms, each wiring a marker — both
/// arms are diagnosed and neither gates anywhere.
#[test]
fn multiple_when_arms_are_both_diagnosed_and_never_gate() {
    let fx = Fixture::new();
    fx.write(
        "Directory.Packages.props",
        "<Project>\n<Choose>\n  <When Condition=\"'$(IsDotNet)'=='true'\">\n    <ItemGroup><PackageReference Include=\"xunit\" Version=\"2.9.0\" /></ItemGroup>\n  </When>\n  <When Condition=\"'$(IsDotNet)'=='false'\">\n    <ItemGroup><PackageReference Include=\"NUnit\" Version=\"4.2.0\" /></ItemGroup>\n  </When>\n</Choose>\n</Project>\n",
    );
    fx.write("lib/Lib.csproj", lib_csproj());
    fx.write("lib/Lib.cs", "namespace Repro.Lib { public class Greeter { } }\n");
    let scan = fx.run(&["scan", "."]);
    assert_machine_body_clean(&scan, "two-when-arms");
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(unit_names(&model), vec!["Lib".to_string()]);
    assert_eq!(
        marker_lines(&scan),
        vec![
            "test marker under unevaluated condition: Directory.Packages.props:4".to_string(),
            "test marker under unevaluated condition: Directory.Packages.props:7".to_string(),
        ],
        "both When arms state their marker; neither gates the project"
    );
    assert_eq!(census_lines(&scan), Vec::<String>::new());
    for package in ["xunit", "NUnit"] {
        assert!(
            !common::stdout(&scan).contains(package),
            "conditioned markers reach no tier: {package}"
        );
    }
}

/// Cell 6: two projects sharing ONE central props that carries markers —
/// each diagnostic line prints exactly once per run and the bytes are
/// identical across two runs (dedup on `(file, line, include)`, sorted).
#[test]
fn shared_props_markers_dedup_and_stay_deterministic() {
    let fx = Fixture::new();
    fx.write(
        "Directory.Packages.props",
        "<Project>\n<Choose>\n  <When Condition=\"'$(IsTestProject)'=='true'\">\n    <ItemGroup>\n      <PackageReference Include=\"xunit\" Version=\"2.9.0\" />\n    </ItemGroup>\n  </When>\n  <Otherwise>\n    <ItemGroup>\n      <PackageReference Include=\"xunit.core\" Version=\"2.9.0\" />\n    </ItemGroup>\n  </Otherwise>\n</Choose>\n</Project>\n",
    );
    fx.write("a/A/A.csproj", lib_csproj());
    fx.write("a/A/A.cs", "namespace Demo.A; public class A { }\n");
    fx.write("b/B/B.csproj", lib_csproj());
    fx.write("b/B/B.cs", "namespace Demo.B; public class B { }\n");
    let first = fx.run(&["scan", "."]);
    assert_machine_body_clean(&first, "shared-props-run-1");
    let second = fx.run(&["scan", "."]);
    assert_machine_body_clean(&second, "shared-props-run-2");
    let expected = [
        "test marker under unevaluated condition: Directory.Packages.props:5".to_string(),
        "test marker under unevaluated condition: Directory.Packages.props:10".to_string(),
    ];
    for run in [&first, &second] {
        assert_eq!(
            marker_lines(run),
            expected,
            "each diagnostic line prints exactly once per run, not once per project"
        );
    }
    assert_eq!(
        common::stderr(&first),
        common::stderr(&second),
        "the report surface is byte-identical across runs"
    );
    assert_eq!(
        common::stdout(&first),
        common::stdout(&second),
        "the machine body is byte-identical across runs"
    );
}

/// Cell 7: the non-goal doc pin — the scan input doc states the
/// `Directory.Build.targets` / `<Import>` posture with the exact sentence
/// (superseded 2026-11-21 by workplan csharp_condition_eval scenario 04:
/// the face is still not walked, and now names itself on the report
/// surface through the catalog entry this sentence cites).
#[test]
fn non_goal_sentence_is_documented() {
    let doc = std::fs::read_to_string(format!(
        "{}/docs/archspec/commands/scan/input.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the scan input doc ships in the payload");
    assert!(
        doc.contains(
            "Markers declared only in `Directory.Build.targets` or props brought in by `<Import>` are not walked and stay unmodeled — the named non-goal stands — and every such file a run encounters states itself on the report surface through the `condition face not walked` entry of `archspec help diagnostics`."
        ),
        "the non-goal sentence must state the Directory.Build.targets and <Import> posture verbatim"
    );
}

/// The line-number law: the diagnostic names the line HOLDING the
/// `<PackageReference>` — a blank line before the item never shifts the
/// count, and the law holds for LF and CRLF fixtures alike (item on line 3
/// with a blank line 2 reports line 3).
#[test]
fn marker_line_numbers_survive_blank_lines_and_crlf() {
    let lf = "<Project><ItemGroup Condition=\"'$(IsTestProject)'=='true'\">\n\n  <PackageReference Include=\"xunit\" Version=\"2.9.0\" />\n</ItemGroup></Project>\n";
    let crlf = lf.replace('\n', "\r\n");
    for (label, props) in [("lf", lf), ("crlf", crlf.as_str())] {
        let fx = Fixture::new();
        fx.write("Directory.Packages.props", props);
        fx.write("lib/Lib.csproj", lib_csproj());
        fx.write("lib/Lib.cs", "namespace Repro.Lib { public class Greeter { } }\n");
        let scan = fx.run(&["scan", "."]);
        assert_machine_body_clean(&scan, label);
        assert_eq!(
            marker_lines(&scan),
            vec![
                "test marker under unevaluated condition: Directory.Packages.props:3".to_string(),
            ],
            "{label}: the conditioned PackageReference sits on line 3 with a blank line before it"
        );
    }
}
