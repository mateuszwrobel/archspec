//! GitHub issue #45 rung-2 cells (workplan `csharp_condition_eval` scenario
//! 02): the csharp driver consumes the shipped evaluator's verdicts. A test
//! marker under an evaluated TRUE condition is gating evidence and states
//! exactly one `test-gated unit: <path> — <marker>` census line naming the
//! promotion (F7); an evaluated FALSE marker condition leaves the item
//! absent with no diagnostic row and every unconditional fact unchanged
//! (F8 defense); an unevaluated condition keeps the byte-identical rung-1
//! row, deduped on `(file, line, Include)` and sorted (m1); a `Choose`
//! chain follows the B1 rule end-to-end — first match wins, an unevaluable
//! earlier arm vetoes later arms and the `Otherwise` to the diagnostic
//! path; truth is decided per CONSUMING project — one shared props file
//! flips one project and holds another (M4, no per-props-file cache); a
//! conditioned non-test reference keeps the rung-1 counting whatever the
//! verdict says (D6); repeated runs are byte-equal on both channels.
//!
//! The four issue repro variants ride these cells as hermetic fixture
//! trees with scrubbed synthetic names (`App.Core`, `App.Tests`,
//! `Widget.Lib`): the choose variant (props `Choose` with a `When` marker
//! arm and an `Otherwise` wiring no test marker, plus the multi-`When`
//! veto shape), the itemgroup variant (`ItemGroup Condition` in props and
//! in a csproj), the packageref variant (item-level `Condition` on the
//! `PackageReference`), and the control variant (conditions removed — the
//! rung-1 census behavior, asserted here as the evaluated-verdict sibling;
//! the control PINS live unchanged in `tests/csharp_test_gating.rs`). The
//! `$(IsTestProject)` spelling is the out-of-subset condition every
//! byte-identical rung-1 pin rides.

mod common;

use common::Fixture;
use serde_json::Value;

fn lib_csproj() -> String {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n</Project>\n".to_string()
}

fn lib_source() -> String {
    "namespace App.Core { public class Greeter { } }\n".to_string()
}

/// The consuming test project: a plain lib csproj shape plus its one
/// source; `extra_item_groups` is spliced in before the `ProjectReference`
/// group for the csproj-face variants.
fn tests_csproj(extra_item_groups: &str) -> String {
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n{extra_item_groups}<ItemGroup><ProjectReference Include=\"../../lib/App.Core.csproj\"/></ItemGroup>\n</Project>\n"
    )
}

fn tests_source() -> String {
    "using App.Core; namespace App.Tests { class T { } }\n".to_string()
}

/// The two-project tree: `lib/App.Core` plus `tests/App.Tests`, optionally
/// joined by a root central props file (`props`).
fn two_project_tree(props: Option<&str>) -> Fixture {
    let fx = Fixture::new();
    fx.write("lib/App.Core.csproj", &lib_csproj());
    fx.write("lib/App.Core.cs", &lib_source());
    fx.write("tests/App.Tests/App.Tests.csproj", &tests_csproj(""));
    fx.write("tests/App.Tests/Tests.cs", &tests_source());
    if let Some(props) = props {
        fx.write("Directory.Packages.props", props);
    }
    fx
}

// — the four #45 repro variants, names scrubbed to synthetics —

/// choose variant: a props `Choose` whose `When` arms the xunit-family
/// marker at `App.Tests`; the `Otherwise` wires NO test marker (the
/// MSBuild-real shape — the marker arms the matching project, the rest
/// stay production).
fn choose_props() -> String {
    "<Project>\n<Choose>\n  <When Condition=\"'$(MSBuildProjectName)'=='App.Tests'\">\n    <ItemGroup>\n      <PackageReference Include=\"xunit\" Version=\"2.9.0\"/>\n    </ItemGroup>\n  </When>\n  <Otherwise>\n    <ItemGroup/>\n  </Otherwise>\n</Choose>\n</Project>\n".to_string()
}

/// choose variant, the multi-`When` veto shape: the FIRST arm carries the
/// out-of-subset condition `$(IsTestProject)`; the later `When` and the
/// `Otherwise` carry markers — every arm rides the diagnostic path (B1).
fn veto_props() -> String {
    "<Project>\n<Choose>\n  <When Condition=\"'$(IsTestProject)'=='true'\">\n    <ItemGroup><PackageReference Include=\"xunit\" Version=\"2.9.0\"/></ItemGroup>\n  </When>\n  <When Condition=\"'$(MSBuildProjectName)'=='App.Tests'\">\n    <ItemGroup><PackageReference Include=\"xunit.runner.visualstudio\" Version=\"3.0.0\"/></ItemGroup>\n  </When>\n  <Otherwise>\n    <ItemGroup><PackageReference Include=\"xunit.extensibility.core\" Version=\"2.9.0\"/></ItemGroup>\n  </Otherwise>\n</Choose>\n</Project>\n".to_string()
}

/// choose variant, the evaluable chain shape: arm 1 matches `App.Core` and
/// wires a NON-test package; arm 2 wires the marker for `App.Tests`.
fn chain_first_props() -> String {
    "<Project>\n<Choose>\n  <When Condition=\"'$(MSBuildProjectName)'=='App.Core'\">\n    <ItemGroup><PackageReference Include=\"Serilog\" Version=\"4.0.0\"/></ItemGroup>\n  </When>\n  <When Condition=\"'$(MSBuildProjectName)'=='App.Tests'\">\n    <ItemGroup><PackageReference Include=\"xunit\" Version=\"2.9.0\"/></ItemGroup>\n  </When>\n  <Otherwise><ItemGroup/></Otherwise>\n</Choose>\n</Project>\n".to_string()
}

/// itemgroup variant, props face: the marker stands in a conditioned
/// `ItemGroup` keyed on the project-name method (case-sensitive `.EndsWith`,
/// M1).
fn itemgroup_props() -> String {
    "<Project>\n<ItemGroup Condition=\"$(MSBuildProjectName.EndsWith('.Tests'))\">\n  <PackageReference Include=\"xunit\" Version=\"2.9.0\"/>\n</ItemGroup>\n</Project>\n".to_string()
}

/// packageref variant, props face: the item-level `Condition` on the
/// xunit-family `PackageReference`.
fn packageref_props() -> String {
    "<Project>\n<ItemGroup>\n  <PackageReference Include=\"xunit\" Version=\"2.9.0\" Condition=\"'$(MSBuildProjectName)' != 'App.Core'\"/>\n</ItemGroup>\n</Project>\n".to_string()
}

/// The out-of-subset shape every rung-1 pin rides: `$(IsTestProject)`.
fn unevaluated_props() -> String {
    "<Project>\n<ItemGroup Condition=\"'$(IsTestProject)'=='true'\">\n  <PackageReference Include=\"xunit\" Version=\"2.9.0\"/>\n</ItemGroup>\n</Project>\n".to_string()
}

fn census_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("test-gated unit: "))
        .map(str::to_string)
        .collect()
}

fn marker_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("test marker under unevaluated condition: "))
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

fn run_scan(fx: &Fixture) -> std::process::Output {
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    scan
}

fn model_of(output: &std::process::Output) -> Value {
    serde_json::from_str(&common::stdout(output)).expect("scan stdout is JSON")
}

/// True when the model's external tier names the package id.
fn external_names(model: &Value, package: &str) -> bool {
    model["external"]
        .as_array()
        .is_some_and(|external| {
            external
                .iter()
                .any(|entry| entry.as_str() == Some(package))
        })
}

/// The same two-project tree with the test project removed: `lib/App.Core`
/// plus the same props file — the machine-body equal for promotion cells
/// (the props face is FALSE-inert for the surviving project).
fn lib_only_tree(props: Option<&str>) -> Fixture {
    let fx = Fixture::new();
    fx.write("lib/App.Core.csproj", &lib_csproj());
    fx.write("lib/App.Core.cs", &lib_source());
    if let Some(props) = props {
        fx.write("Directory.Packages.props", props);
    }
    fx
}

/// F7 cell: an evaluated TRUE test marker is gating evidence and states
/// exactly one `test-gated unit:` census line naming the promotion; the
/// machine body equals the tree with the promoted project removed. Drives
/// the choose, itemgroup (props face) and packageref repro variants, then
/// the csproj face of the itemgroup variant, then the control variant
/// (conditions removed — the SAME census bytes, the rung-1 behavior pinned
/// in `csharp_test_gating.rs`; this is the evaluated-verdict sibling
/// assertion, not a re-pin).
#[test]
fn evaluated_true_marker_promotes_with_the_census_row() {
    for (label, props) in [
        ("choose-props", choose_props()),
        ("itemgroup-props", itemgroup_props()),
        ("packageref-props", packageref_props()),
    ] {
        let fx = two_project_tree(Some(&props));
        let scan = run_scan(&fx);
        assert_eq!(
            census_lines(&scan),
            vec!["test-gated unit: tests/App.Tests — xunit".to_string()],
            "{label}: exactly one census line names the promotion (F7)"
        );
        assert_eq!(
            marker_lines(&scan),
            Vec::<String>::new(),
            "{label}: a promoted marker gates — it states no unevaluated diagnostic"
        );
        let without = lib_only_tree(Some(&props));
        assert_eq!(
            common::stdout(&scan),
            common::stdout(&run_scan(&without)),
            "{label}: the machine body equals the tree with the promoted project removed"
        );
    }
    // csproj face: the conditioned ItemGroup stands IN the consuming csproj.
    let fx = Fixture::new();
    fx.write("lib/App.Core.csproj", &lib_csproj());
    fx.write("lib/App.Core.cs", &lib_source());
    fx.write(
        "tests/App.Tests/App.Tests.csproj",
        &tests_csproj("<ItemGroup Condition=\"$(MSBuildProjectName.EndsWith('.Tests'))\"><PackageReference Include=\"xunit\" Version=\"2.9.0\"/></ItemGroup>\n"),
    );
    fx.write("tests/App.Tests/Tests.cs", &tests_source());
    let scan = run_scan(&fx);
    assert_eq!(
        census_lines(&scan),
        vec!["test-gated unit: tests/App.Tests — xunit".to_string()],
        "csproj face: the evaluated TRUE marker promotes with the one census line"
    );
    assert_eq!(marker_lines(&scan), Vec::<String>::new());
    // control variant: conditions removed, the census bytes identical —
    // the evaluated verdict and the unconditional fact state the same row.
    let control = Fixture::new();
    control.write("lib/App.Core.csproj", &lib_csproj());
    control.write("lib/App.Core.cs", &lib_source());
    control.write(
        "tests/App.Tests/App.Tests.csproj",
        &tests_csproj("<ItemGroup><PackageReference Include=\"xunit\" Version=\"2.9.0\"/></ItemGroup>\n"),
    );
    control.write("tests/App.Tests/Tests.cs", &tests_source());
    let control_scan = run_scan(&control);
    let choose_scan = run_scan(&two_project_tree(Some(&choose_props())));
    assert_eq!(
        census_lines(&control_scan),
        census_lines(&choose_scan),
        "the control variant states the identical census bytes to the evaluated promotion"
    );
    assert_eq!(
        marker_lines(&control_scan),
        Vec::<String>::new(),
        "control: no condition, no marker diagnostic"
    );
}

/// F8 defense cell: an evaluated FALSE test-marker condition leaves the
/// item ABSENT with zero `test marker under unevaluated condition` rows and
/// zero `test-gated unit:` rows naming it, and the project's facts are
/// byte-identical to the tree without the conditioned item. Drives the
/// choose, itemgroup and packageref faces for App.Core.
#[test]
fn evaluated_false_marker_is_absent_without_a_diagnostic() {
    for (label, fx) in [
        ("choose-props", two_project_tree(Some(&choose_props()))),
        (
            "itemgroup-props",
            two_project_tree(Some(&itemgroup_props())),
        ),
        (
            "packageref-props",
            two_project_tree(Some(&packageref_props())),
        ),
    ] {
        let scan = run_scan(&fx);
        assert!(
            marker_lines(&scan).is_empty(),
            "{label}: the FALSE-covered item states no diagnostic row"
        );
        assert!(
            !census_lines(&scan)
                .iter()
                .any(|line| line.contains("App.Core")),
            "{label}: App.Core is never promoted by a FALSE marker"
        );
    }
    // The F8 machine-body defense at the single-project face: App.Core
    // alone under the FALSE condition, byte-for-byte the tree without
    // the conditioned item, stderr free of both row families.
    let conditioned = Fixture::new();
    conditioned.write("lib/App.Core.csproj", &lib_csproj());
    conditioned.write("lib/App.Core.cs", &lib_source());
    conditioned.write("Directory.Packages.props", &packageref_props());
    let unconditioned = Fixture::new();
    unconditioned.write("lib/App.Core.csproj", &lib_csproj());
    unconditioned.write("lib/App.Core.cs", &lib_source());
    unconditioned.write("Directory.Packages.props", "<Project>\n<ItemGroup/>\n</Project>\n");
    let a = run_scan(&conditioned);
    let b = run_scan(&unconditioned);
    assert_eq!(
        common::stdout(&a),
        common::stdout(&b),
        "App.Core keeps every unconditional fact unchanged (F8)"
    );
    assert_eq!(common::stderr(&a), common::stderr(&b));
    assert_eq!(marker_lines(&a), Vec::<String>::new());
    assert_eq!(census_lines(&a), Vec::<String>::new());
}

/// m1 cell: an out-of-subset condition (`$(IsTestProject)`) on a test
/// marker keeps the byte-identical rung-1 row, deduped on
/// `(file, line, Include)` across the two consuming projects that read the
/// same props file, sorted; the projects stay production.
#[test]
fn unevaluated_condition_keeps_the_byte_identical_rung1_row() {
    let fx = two_project_tree(Some(&unevaluated_props()));
    let scan = run_scan(&fx);
    assert_eq!(
        marker_lines(&scan),
        vec!["test marker under unevaluated condition: Directory.Packages.props:3".to_string()],
        "one row for the distinct (file, line, Include) whatever the project count"
    );
    assert_eq!(
        census_lines(&scan),
        Vec::<String>::new(),
        "the unevaluated marker gates nothing"
    );
    let model = model_of(&scan);
    assert_eq!(
        unit_names(&model),
        vec!["App.Core".to_string(), "App.Tests".to_string()],
        "both projects stay production"
    );
}

/// B1 end-to-end cell: in an evaluable chain, arm 2 is effective exactly
/// when arm 1 evaluated FALSE (arm 1 matching App.Core wires a non-test
/// package there, arm 2 matching App.Tests wires the marker); an
/// unevaluable arm 1 routes arm 2 AND the `Otherwise` marker to the
/// diagnostic rows while every project stays production.
#[test]
fn choose_chain_first_match_wins_across_arms() {
    let fx = two_project_tree(Some(&chain_first_props()));
    let scan = run_scan(&fx);
    assert_eq!(
        census_lines(&scan),
        vec!["test-gated unit: tests/App.Tests — xunit".to_string()],
        "arm 2 is effective exactly when arm 1 evaluated FALSE"
    );
    assert_eq!(
        marker_lines(&scan),
        Vec::<String>::new(),
        "every arm evaluated — the veto never enters an evaluable chain"
    );
    let model = model_of(&scan);
    assert!(
        unit_names(&model).contains(&"App.Core".to_string()),
        "the arm-1 match keeps App.Core production with its non-test arm fact"
    );
    assert!(
        external_names(&model, "Serilog"),
        "the TRUE-matched arm's non-test central reference counts for App.Core"
    );

    let veto_tree = two_project_tree(Some(&veto_props()));
    let veto = run_scan(&veto_tree);
    assert_eq!(
        marker_lines(&veto),
        vec![
            "test marker under unevaluated condition: Directory.Packages.props:4".to_string(),
            "test marker under unevaluated condition: Directory.Packages.props:7".to_string(),
            "test marker under unevaluated condition: Directory.Packages.props:10".to_string(),
        ],
        "the unevaluable arm 1 vetoes arm 2 and the Otherwise to the sorted diagnostic rows"
    );
    assert_eq!(
        census_lines(&veto),
        Vec::<String>::new(),
        "a vetoed marker chain gates nothing — every project stays production"
    );
}

/// M4 cell: one shared props file, two consuming projects — exactly ONE
/// flip census row in one scan run, proving the per-consuming-project
/// context and the absence of a per-props-file cache. Drives both the
/// choose and the packageref (item-level `!=`) shapes.
#[test]
fn one_props_file_flips_one_project_and_holds_another() {
    for (label, props) in [("choose", choose_props()), ("packageref", packageref_props())] {
        let fx = two_project_tree(Some(&props));
        let scan = run_scan(&fx);
        assert_eq!(
            census_lines(&scan),
            vec!["test-gated unit: tests/App.Tests — xunit".to_string()],
            "{label}: exactly one project flips, the other holds every rung-1 fact"
        );
        let model = model_of(&scan);
        assert_eq!(
            unit_names(&model),
            vec!["App.Core".to_string()],
            "{label}: the held project is the model's only unit"
        );
        assert_eq!(
            marker_lines(&scan),
            Vec::<String>::new(),
            "{label}: flip and hold are both evaluated — no diagnostic row"
        );
    }
}

/// D6 cell: a FALSE-conditioned non-test `PackageReference` still appears
/// in the external-dependency facts exactly as rung 1 counts it — no
/// verdict erases a non-marker fact.
#[test]
fn conditioned_non_test_reference_counts_unchanged() {
    let conditioned = Fixture::new();
    conditioned.write(
        "lib/App.Core.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n<ItemGroup><PackageReference Include=\"Microsoft.Extensions.Logging\" Version=\"9.0.0\" Condition=\"'$(MSBuildProjectName)'=='Widget.Lib'\"/></ItemGroup>\n</Project>\n",
    );
    conditioned.write("lib/App.Core.cs", &lib_source());
    let unconditional = Fixture::new();
    unconditional.write(
        "lib/App.Core.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n<ItemGroup><PackageReference Include=\"Microsoft.Extensions.Logging\" Version=\"9.0.0\"/></ItemGroup>\n</Project>\n",
    );
    unconditional.write("lib/App.Core.cs", &lib_source());
    let a = run_scan(&conditioned);
    let b = run_scan(&unconditional);
    assert_eq!(
        common::stdout(&a),
        common::stdout(&b),
        "the FALSE-conditioned non-test reference keeps the rung-1 machine bytes"
    );
    let model = model_of(&a);
    assert!(
        external_names(&model, "Microsoft.Extensions.Logging"),
        "the non-test fact stays in the external tier (D6)"
    );
    assert_eq!(marker_lines(&a), Vec::<String>::new());
}

/// m1 determinism cell: two scans of one fixture answer byte-equal on
/// BOTH channels — stdout model and the stderr census/diagnostic set.
#[test]
fn repeated_runs_are_byte_equal_on_both_channels() {
    let fx = two_project_tree(Some(&veto_props()));
    let first = run_scan(&fx);
    let second = run_scan(&fx);
    assert_eq!(
        common::stdout(&first),
        common::stdout(&second),
        "the machine body is run-independent"
    );
    assert_eq!(
        common::stderr(&first),
        common::stderr(&second),
        "the census and diagnostic rows repeat byte-equal, sorted and deduped"
    );
}
