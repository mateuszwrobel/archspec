//! workplan csharp_condition_eval scenario 04 — the F2 never-silent cure:
//! condition-face sources the csharp driver does not walk name themselves.
//! A test marker declared only in a root `Directory.Build.targets`, or in
//! props brought in by an `<Import>`, reaches no tier (the walk extension
//! stays the named non-goal) and the scan never lets that gap be a silent
//! zero: every such file states itself once as a report-only
//! `condition face not walked:` row — machine body byte-identical, exit 0.
//!
//! The trees are hermetic synthetics (`App.Core`, `App.Tests`); shapes only
//! come from the reported origin layout, no private content rides here.

mod common;

use common::Fixture;

/// The row grammars pinned by the claim registry
/// (`csharp-condition-face-unwalked`, owner surface: `help diagnostics`).
/// Welded by `guard_pin` (`equals-fragments`): the registry fragments and
/// these bytes must move together or the fan-out weld fails.
#[allow(dead_code)]
const GAP_FORMS: [&str; 2] = [
    "condition face not walked: <file>",
    "condition face not walked: <file> — imported by <importing-file>:<line>",
];

fn lib_csproj() -> &'static str {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n</Project>\n"
}

/// A lib csproj importing the wiring props LITERALLY: the resolved target
/// on line 3 (path-relative to `lib/`), the property-residue target
/// verbatim on line 4.
fn importing_csproj() -> &'static str {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n<Import Project=\"../Shared/Testwiring.props\"/>\n<Import Project=\"$(RepoRoot)/props/Extras.props\"/>\n</Project>\n"
}

fn lib_source() -> &'static str {
    "namespace App.Core { public class Greeter { } }\n"
}

/// The marker shape: a test-runner `PackageReference` under an unevaluated
/// `Condition` — in the targets/props variant this file is the ONLY
/// condition-bearing content in the tree.
fn marker_wiring() -> &'static str {
    "<Project>\n<ItemGroup Condition=\"'$(IsTestProject)'=='true'\">\n<PackageReference Include=\"xunit\" Version=\"2.9.0\"/>\n</ItemGroup>\n</Project>\n"
}

fn gap_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("condition face not walked: "))
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

fn gated_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("test-gated unit: "))
        .map(str::to_string)
        .collect()
}

fn assert_scan_ok(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "scan failed: {}",
        common::stderr(output)
    );
}

/// The targets-variant tree: the ONLY marker sits in the root
/// `Directory.Build.targets`. `with_targets = false` is the marker-less
/// equal for the machine-body pin.
fn targets_tree(with_targets: bool) -> Fixture {
    let fx = Fixture::new();
    fx.write("lib/App.Core.csproj", lib_csproj());
    fx.write("lib/App.Core.cs", lib_source());
    if with_targets {
        fx.write("Directory.Build.targets", marker_wiring());
    }
    fx
}

/// The F2 proving cell: markers only in `Directory.Build.targets` print
/// exactly the one named row — a zero-row tree here is a landing failure —
/// and nothing else moves: no suppression line, no gating line, the model
/// bytes equal the marker-less tree, exit 0.
#[test]
fn markers_only_in_directory_build_targets_print_the_named_row() {
    let with = targets_tree(true);
    let without = targets_tree(false);
    let scan = with.run(&["scan", "."]);
    let baseline = without.run(&["scan", "."]);
    assert_scan_ok(&scan);
    assert_scan_ok(&baseline);
    assert_eq!(
        gap_lines(&scan),
        vec!["condition face not walked: Directory.Build.targets".to_string()],
        "the unwalked targets file states itself exactly once (F2 — zero rows here is a landing failure)"
    );
    assert!(
        marker_lines(&scan).is_empty(),
        "the targets contents are never read, so no suppression line may appear: {:?}",
        marker_lines(&scan)
    );
    assert!(
        gated_lines(&scan).is_empty(),
        "the marker gates nothing: no project drops: {:?}",
        gated_lines(&scan)
    );
    assert_eq!(
        common::stdout(&scan),
        common::stdout(&baseline),
        "the machine body is byte-equal to the marker-less tree"
    );
}

/// The Import cells: a resolved target prints its root-relative resolved
/// path at the `Import` element's line; a glob/`$(...)`-residue attribute
/// prints its text verbatim in the file position. The imported contents are
/// never read, so the marker inside states no suppression line.
#[test]
fn import_props_print_the_named_row() {
    let fx = Fixture::new();
    fx.write("lib/App.Core.csproj", importing_csproj());
    fx.write("lib/App.Core.cs", lib_source());
    fx.write("Shared/Testwiring.props", marker_wiring());
    let scan = fx.run(&["scan", "."]);
    assert_scan_ok(&scan);
    assert_eq!(
        gap_lines(&scan),
        vec![
            "condition face not walked: $(RepoRoot)/props/Extras.props — imported by lib/App.Core.csproj:4".to_string(),
            "condition face not walked: Shared/Testwiring.props — imported by lib/App.Core.csproj:3".to_string(),
        ],
        "one row per distinct (target, importing site): resolved target root-relative at the element's line, residue verbatim, sorted by line bytes"
    );
    assert!(
        marker_lines(&scan).is_empty(),
        "Import-target contents are never walked: {:?}",
        marker_lines(&scan)
    );
}

/// The walked-props control: trees holding only the walked names print
/// ZERO gap rows — the existing suppression census stays theirs alone, and
/// an import of a walked name states nothing new.
#[test]
fn walked_props_presence_prints_no_gap_row() {
    let props = Fixture::new();
    props.write("lib/App.Core.csproj", lib_csproj());
    props.write("lib/App.Core.cs", lib_source());
    props.write("Directory.Build.props", marker_wiring());
    let build_props = props.run(&["scan", "."]);
    assert_scan_ok(&build_props);
    assert!(
        gap_lines(&build_props).is_empty(),
        "Directory.Build.props is walked — no gap row: {:?}",
        gap_lines(&build_props)
    );
    assert_eq!(
        marker_lines(&build_props),
        vec!["test marker under unevaluated condition: Directory.Build.props:3".to_string()],
        "the walked-props suppression census keeps its existing bytes"
    );

    let packages = Fixture::new();
    packages.write("lib/App.Core.csproj", lib_csproj());
    packages.write("lib/App.Core.cs", lib_source());
    packages.write("Directory.Packages.props", marker_wiring());
    let packages = packages.run(&["scan", "."]);
    assert_scan_ok(&packages);
    assert!(
        gap_lines(&packages).is_empty(),
        "Directory.Packages.props is walked — no gap row: {:?}",
        gap_lines(&packages)
    );

    let import_walked = Fixture::new();
    import_walked.write(
        "lib/App.Core.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n<Import Project=\"Directory.Build.props\"/>\n</Project>\n",
    );
    import_walked.write("lib/App.Core.cs", lib_source());
    let import_walked = import_walked.run(&["scan", "."]);
    assert_scan_ok(&import_walked);
    assert!(
        gap_lines(&import_walked).is_empty(),
        "an import of a walked name states nothing new: {:?}",
        gap_lines(&import_walked)
    );
}

/// Dedupe law: two projects under one root `Directory.Build.targets` print
/// exactly ONE row — the row set dedupes on the row bytes.
#[test]
fn targets_row_dedupes_across_projects_under_one_targets_file() {
    let fx = Fixture::new();
    fx.write("lib/App.Core.csproj", lib_csproj());
    fx.write("lib/App.Core.cs", lib_source());
    fx.write(
        "lib/App.Data.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n</Project>\n",
    );
    fx.write(
        "lib/App.Data.cs",
        "namespace App.Data { public class Store { } }\n",
    );
    fx.write("Directory.Build.targets", marker_wiring());
    let scan = fx.run(&["scan", "."]);
    assert_scan_ok(&scan);
    assert_eq!(
        gap_lines(&scan),
        vec!["condition face not walked: Directory.Build.targets".to_string()],
        "one row per distinct targets file, never one per project"
    );
}

/// Determinism law: two runs print byte-equal, sorted gap rows; the stdout
/// model bytes are stable and equal the control tree without the gap
/// sources; the scan exits 0 with rows present — report-only, never a
/// verdict.
#[test]
fn gap_rows_are_sorted_and_stable_and_the_machine_body_is_unchanged() {
    let fx = Fixture::new();
    fx.write("lib/App.Core.csproj", importing_csproj());
    fx.write("lib/App.Core.cs", lib_source());
    fx.write("Shared/Testwiring.props", marker_wiring());
    fx.write("Directory.Build.targets", marker_wiring());
    let control = Fixture::new();
    control.write("lib/App.Core.csproj", lib_csproj());
    control.write("lib/App.Core.cs", lib_source());
    let first = fx.run(&["scan", "."]);
    let second = fx.run(&["scan", "."]);
    let baseline = control.run(&["scan", "."]);
    assert_scan_ok(&first);
    assert_scan_ok(&second);
    assert_scan_ok(&baseline);
    let rows = gap_lines(&first);
    assert_eq!(
        rows,
        vec![
            "condition face not walked: $(RepoRoot)/props/Extras.props — imported by lib/App.Core.csproj:4".to_string(),
            "condition face not walked: Directory.Build.targets".to_string(),
            "condition face not walked: Shared/Testwiring.props — imported by lib/App.Core.csproj:3".to_string(),
        ],
        "the gap families, sorted by line bytes"
    );
    assert_eq!(rows, gap_lines(&second), "two runs print byte-equal rows");
    let mut sorted = rows.clone();
    sorted.sort();
    assert_eq!(rows, sorted, "the row set is sorted by line bytes");
    assert_eq!(
        common::stdout(&first),
        common::stdout(&baseline),
        "the machine body is unchanged by the gap rows"
    );
}

/// The fan-out weld, cell side: `archspec help diagnostics` renders both
/// registered grammars byte-identical to `GAP_FORMS` (whitespace-flattened,
/// the registry's comparison shape).
#[test]
fn help_diagnostics_renders_the_gap_forms() {
    let fx = Fixture::new();
    let help = fx.run(&["help", "diagnostics"]);
    assert_scan_ok(&help);
    let rendered = common::stdout(&help)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for grammar in GAP_FORMS {
        let wanted = grammar.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            rendered.contains(&wanted),
            "help diagnostics must render the registered grammar {grammar:?}:\n{}",
            common::stdout(&help)
        );
    }
}
