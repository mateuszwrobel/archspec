//! Workplan `visibility_phase2` scenario 03 — csharp manifest path-items and
//! the csharp literal-candidate feed. The classification arms of the include
//! walk each print their named row (or state nothing, for the silent
//! under-self arm), the dedup law keeps a path item and a `ProjectReference`
//! to one unit at ONE edge, the shared-source census states numbers, never
//! units, and the candidate feed joins two csharp units through the declared
//! `[[pattern_pair]]` / `[[shared_constant]]` grammar with zero engine-side
//! vocabulary. Malformed XML refuses loud — never a silent zero.
//!
//! Names are synthetic (`App.Api`, `App.Contracts`, `App.Worker`, `Shared`);
//! no corpus, vendor or account string appears in this file.

mod common;

use common::Fixture;
use serde_json::Value;

/// The census forms pinned by the claim registry (`csharp-manifest-path-items`
/// and `csharp-shared-source-census`, owner surface: `help diagnostics`). The
/// dedicated guard for those placeholder bytes is this file: the exact-bytes
/// cells below render them.
#[allow(dead_code)]
const MANIFEST_FORMS: [&str; 4] = [
    "manifest path item in target: <csproj>:<line>",
    "manifest path item under unevaluated condition: <csproj>:<line>",
    "manifest path item unresolved: <csproj>:<line>",
    "manifest path item under dropped project: <csproj>:<line> — <resolved path>",
];

#[allow(dead_code)]
const SHARED_FORMS: [&str; 1] = ["shared source: <path> — host units: <n>"];

/// Minimal SDK-style csproj carrying `items` verbatim between the root tags.
fn csproj(items: &str) -> String {
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n{items}</Project>\n"
    )
}

/// One referencing project with a compile item pointing at `rel`.
fn unit_with_item(name: &str, rel: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            Box::leak(format!("{name}/{name}.csproj").into_boxed_str()),
            csproj(&format!(
                "  <ItemGroup>\n    <Compile Include=\"{rel}\" />\n  </ItemGroup>\n"
            )),
        ),
        (
            Box::leak(format!("{name}/Program.cs").into_boxed_str()),
            format!("namespace {name};\npublic class Program {{ public static void Main() {{}} }}\n"),
        ),
    ]
}

fn write_all(fx: &Fixture, files: &[(&str, String)]) {
    for (path, content) in files {
        fx.write(path, content);
    }
}

fn manifest_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| {
            line.starts_with("manifest path item ") || line.starts_with("shared source: ")
        })
        .map(str::to_string)
        .collect()
}

fn edge_pairs(model: &Value) -> Vec<String> {
    model["edges"]
        .as_array()
        .expect("edges is a list")
        .iter()
        .map(|edge| {
            format!(
                "{} -> {}",
                edge["from"].as_str().unwrap_or_default(),
                edge["to"].as_str().unwrap_or_default()
            )
        })
        .collect()
}

fn unit_names(model: &Value) -> Vec<String> {
    model["units"]
        .as_array()
        .expect("units is a list")
        .iter()
        .filter_map(|unit| unit["name"].as_str().map(String::from))
        .collect()
}

fn scan(fx: &Fixture) -> std::process::Output {
    let output = fx.run(&["scan", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        common::stderr(&output)
    );
    output
}

fn model_of(output: &std::process::Output) -> Value {
    serde_json::from_str(&common::stdout(output)).expect("scan stdout is JSON")
}

// ---------------------------------------------------------------------------
// cell A — the under-self arm is silent (SDK default content, no row, no edge)
// ---------------------------------------------------------------------------

#[test]
fn self_items_state_no_rows_and_no_edges() {
    let fx = Fixture::new();
    write_all(
        &fx,
        &unit_with_item("App", r"Program.cs"),
    );
    let output = scan(&fx);
    assert!(
        manifest_lines(&output).is_empty(),
        "an item under self is SDK-default content: no row (stderr: {})",
        common::stderr(&output)
    );
    assert_eq!(edge_pairs(&model_of(&output)), Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// cell B — cross-unit item = unit edge, deduped with the ProjectReference
// ---------------------------------------------------------------------------

#[test]
fn cross_unit_item_and_reference_state_one_edge() {
    let fx = Fixture::new();
    fx.write(
        "App.Api/App.Api.csproj",
        &csproj(
            "  <ItemGroup>\n    <ProjectReference Include=\"..\\App.Contracts\\App.Contracts.csproj\" />\n  </ItemGroup>\n  <ItemGroup>\n    <Compile Include=\"..\\App.Contracts\\Models.cs\" />\n  </ItemGroup>\n",
        ),
    );
    fx.write(
        "App.Api/Program.cs",
        "namespace App.Api;\npublic class Program { public static void Main() {} }\n",
    );
    fx.write(
        "App.Contracts/App.Contracts.csproj",
        &csproj(""),
    );
    fx.write(
        "App.Contracts/Models.cs",
        "namespace App.Contracts;\npublic record OrderPlaced;\n",
    );
    let output = scan(&fx);
    let model = model_of(&output);
    assert_eq!(
        edge_pairs(&model),
        vec!["App.Api -> App.Contracts".to_string()],
        "a path item and a ProjectReference to one unit state ONE edge (F2 dedup):\n{}",
        common::stderr(&output)
    );
    assert!(
        manifest_lines(&output).is_empty(),
        "a resolved cross-unit item is an edge, never a skip row (stderr: {})",
        common::stderr(&output)
    );
}

// ---------------------------------------------------------------------------
// cell C — shared sources: a census row, NOT a unit and not an edge (S7, A2)
// ---------------------------------------------------------------------------

#[test]
fn shared_source_prints_the_census_row_and_gains_no_unit() {
    let fx = Fixture::new();
    write_all(
        &fx,
        &unit_with_item("App.Api", r"..\Shared\Shared.cs"),
    );
    write_all(
        &fx,
        &unit_with_item("App.Worker", r"..\Shared\Shared.cs"),
    );
    fx.write(
        "Shared/Shared.cs",
        "namespace Shared;\npublic static class Clock { public static string Now() => string.Empty; }\n",
    );
    let output = scan(&fx);
    let model = model_of(&output);
    assert_eq!(
        manifest_lines(&output),
        vec!["shared source: Shared/Shared.cs — host units: 2".to_string()],
        "one census row, exact bytes, the host count stated as a number (SP4):\n{}",
        common::stderr(&output)
    );
    assert!(
        !unit_names(&model).contains(&"Shared".to_string()),
        "the census creates no unit: {:?}",
        unit_names(&model)
    );
    assert_eq!(edge_pairs(&model), Vec::<String>::new());
    // A2 in force: the shared file sits outside every kept project dir, so
    // no source fact of it is scanned and no namespace row appears anywhere.
    let soft = serde_json::to_string(&model["soft_structure"]).expect("soft_structure");
    assert!(
        !soft.contains("Shared"),
        "the census adds no namespace rows to the module tier: {soft}"
    );
}

/// SP4's proven arm: exactly ONE host prints host-count 1 — the census states
/// numbers, never thresholds.
#[test]
fn single_host_shared_source_states_count_one() {
    let fx = Fixture::new();
    write_all(
        &fx,
        &unit_with_item("App.Api", r"..\Shared\Shared.cs"),
    );
    fx.write(
        "Shared/Shared.cs",
        "namespace Shared;\npublic static class Clock { public static string Now() => string.Empty; }\n",
    );
    let output = scan(&fx);
    assert_eq!(
        manifest_lines(&output),
        vec!["shared source: Shared/Shared.cs — host units: 1".to_string()],
        "one host states count 1, never silence (stderr: {})",
        common::stderr(&output)
    );
}

// ---------------------------------------------------------------------------
// cell D — items resolving under a dropped project: named no-edge row (SP3)
// ---------------------------------------------------------------------------

#[test]
fn dropped_project_item_prints_the_named_no_edge_row() {
    let fx = Fixture::new();
    fx.write(
        "App/App.csproj",
        &csproj(
            "  <ItemGroup>\n    <Compile Include=\"..\\App.Tests\\Helper.cs\" />\n  </ItemGroup>\n",
        ),
    );
    fx.write(
        "App/Program.cs",
        "namespace App;\npublic class Program { public static void Main() {} }\n",
    );
    fx.write(
        "App.Tests/App.Tests.csproj",
        &csproj(
            "  <ItemGroup>\n    <PackageReference Include=\"xunit\" Version=\"2.9.0\" />\n  </ItemGroup>\n",
        ),
    );
    fx.write(
        "App.Tests/Helper.cs",
        "namespace App.Tests;\npublic static class Helper {}\n",
    );
    let output = scan(&fx);
    let lines = manifest_lines(&output);
    assert_eq!(
        lines.len(),
        1,
        "exactly one manifest row (stderr: {})",
        common::stderr(&output)
    );
    assert!(
        lines[0].starts_with("manifest path item under dropped project: App/App.csproj:")
            && lines[0].contains("App.Tests/Helper.cs"),
        "the dropped-project row names item and project: {:?}",
        lines
    );
    assert_eq!(
        edge_pairs(&model_of(&output)),
        Vec::<String>::new(),
        "the dropped project states no edge"
    );
    assert!(
        common::stderr(&output)
            .lines()
            .any(|line| line.starts_with("test-gated unit: App.Tests")),
        "the project-tier row stays printed (no double-report means no dropped-project \
         item row without it): {}",
        common::stderr(&output)
    );
}

// ---------------------------------------------------------------------------
// cell E — the SP2 arm: placeholders, globs and non-path values each name
//          themselves, zero edges
// ---------------------------------------------------------------------------

#[test]
fn unresolvable_items_each_print_the_named_row() {
    let fx = Fixture::new();
    fx.write(
        "App/App.csproj",
        &csproj(
            "  <ItemGroup>\n    <InternalsVisibleTo Include=\"App.UnitTests\" />\n    <Compile Include=\"$(Generated)\\Gen.cs\" />\n    <None Include=\"**\\*.txt\" />\n  </ItemGroup>\n",
        ),
    );
    fx.write(
        "App/Program.cs",
        "namespace App;\npublic class Program { public static void Main() {} }\n",
    );
    let output = scan(&fx);
    let lines = manifest_lines(&output);
    assert_eq!(
        lines.len(),
        3,
        "one row per unresolvable Include (stderr: {})",
        common::stderr(&output)
    );
    for line in &lines {
        assert!(
            line.starts_with("manifest path item unresolved: App/App.csproj:"),
            "the SP2 row shape is `manifest path item unresolved: <csproj>:<line>`: {line}"
        );
    }
    assert_eq!(edge_pairs(&model_of(&output)), Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// cell F — the condition posture: marker row, never an edge
// ---------------------------------------------------------------------------

#[test]
fn conditioned_item_prints_the_condition_row_not_an_edge() {
    let fx = Fixture::new();
    fx.write(
        "App/App.csproj",
        &csproj(
            "  <ItemGroup Condition=\"'$(Configuration)' == 'Debug'\">\n    <Compile Include=\"..\\App.Contracts\\Models.cs\" />\n  </ItemGroup>\n",
        ),
    );
    fx.write(
        "App/Program.cs",
        "namespace App;\npublic class Program { public static void Main() {} }\n",
    );
    fx.write(
        "App.Contracts/App.Contracts.csproj",
        &csproj(""),
    );
    fx.write(
        "App.Contracts/Models.cs",
        "namespace App.Contracts;\npublic record OrderPlaced;\n",
    );
    let output = scan(&fx);
    let lines = manifest_lines(&output);
    assert_eq!(
        lines,
        vec!["manifest path item under unevaluated condition: App/App.csproj:6".to_string()],
        "the condition row states once (stderr: {})",
        common::stderr(&output)
    );
    assert_eq!(
        edge_pairs(&model_of(&output)),
        Vec::<String>::new(),
        "a conditioned item is a marker, never an edge"
    );
}

// ---------------------------------------------------------------------------
// cell G — the target arm: executed content is a named row, never an edge
// ---------------------------------------------------------------------------

#[test]
fn target_item_prints_the_target_row_not_an_edge() {
    let fx = Fixture::new();
    fx.write(
        "App/App.csproj",
        &csproj(
            "  <Target Name=\"CopyStuff\">\n    <Content Include=\"..\\App.Contracts\\Models.cs\" />\n  </Target>\n",
        ),
    );
    fx.write(
        "App/Program.cs",
        "namespace App;\npublic class Program { public static void Main() {} }\n",
    );
    fx.write(
        "App.Contracts/App.Contracts.csproj",
        &csproj(""),
    );
    fx.write(
        "App.Contracts/Models.cs",
        "namespace App.Contracts;\npublic record OrderPlaced;\n",
    );
    let output = scan(&fx);
    let lines = manifest_lines(&output);
    assert_eq!(
        lines.len(),
        1,
        "one target row (stderr: {})",
        common::stderr(&output)
    );
    assert!(
        lines[0].starts_with("manifest path item in target: App/App.csproj:"),
        "the target row names itself: {:?}",
        lines
    );
    assert_eq!(
        edge_pairs(&model_of(&output)),
        Vec::<String>::new(),
        "target content derives no edge"
    );
}

// ---------------------------------------------------------------------------
// cell H — SP1: a malformed csproj refuses loud, never a silent zero
// ---------------------------------------------------------------------------

#[test]
fn malformed_csproj_refuses_with_the_loud_named_error() {
    let fx = Fixture::new();
    fx.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <ItemGroup>\n    <Compile Include=\"Program.cs\"\n",
    );
    fx.write(
        "App/Program.cs",
        "namespace App;\npublic class Program { public static void Main() {} }\n",
    );
    let output = fx.run(&["scan", "."]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "a broken manifest must not answer a silent zero (stderr: {})",
        common::stderr(&output)
    );
    let error = common::stderr(&output);
    assert!(
        error.contains("failed to parse") && error.contains("App.csproj"),
        "the refusal names the file: {error}"
    );
}

// ---------------------------------------------------------------------------
// cell I — the candidate feed: two csharp units join through `pattern_pair`,
//          and a declared `shared_constant` class states constant rows
// ---------------------------------------------------------------------------

#[test]
fn csharp_candidates_join_through_pattern_pair_and_shared_constant() {
    let fx = Fixture::new();
    fx.write("App.Publisher/App.Publisher.csproj", &csproj(""));
    fx.write(
        "App.Publisher/Program.cs",
        "namespace App.Publisher;\npublic static class Program\n{\n    public static void Main()\n    {\n        Builder.AddCache(\"orders-cache\");\n        Bus.Emit(\"orders.created\");\n    }\n}\nstatic class Builder { public static void AddCache(string _name) {} }\nstatic class Bus { public static void Emit(string _e) {} }\n",
    );
    fx.write("App.Subscriber/App.Subscriber.csproj", &csproj(""));
    fx.write(
        "App.Subscriber/Handler.cs",
        "namespace App.Subscriber;\npublic static class Handler\n{\n    public static void Handle()\n    {\n        Bus.Handle(\"orders.created\");\n    }\n}\nstatic class Bus { public static void Handle(string _e) {} }\n",
    );
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[pattern_pair]]\nid = \"demo-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"csharp\"\nrole = \"publish\"\ncall = \"Emit\"\narg = 0\n\n\
         [pattern_pair.subscribe]\nlanguage = \"csharp\"\nrole = \"subscribe\"\ncall = \"Handle\"\narg = 0\n\n\
         [[pattern_pair]]\nid = \"resource-name\"\n\n\
         [pattern_pair.emit]\nlanguage = \"csharp\"\nrole = \"publish\"\ncall = \"AddCache\"\narg = 0\n\n\
         [[shared_constant]]\nclass = \"resource-name\"\nproducer = \"App.Publisher\"\n",
    );
    let output = scan(&fx);
    let error = common::stderr(&output);
    assert!(
        error.contains(
            "literal sites: pattern \"demo-bus\" language \"csharp\" anchor call-name+arg-position sites=2"
        ),
        "the funded pair states its census count (stderr: {error})"
    );
    let model = model_of(&output);
    let joined = serde_json::to_string(&model["async_edges"]).expect("async_edges");
    assert!(
        joined.contains("\"App.Publisher\"") && joined.contains("\"App.Subscriber\"")
            && joined.contains("orders.created"),
        "the two units' shared literal joins through the declaration: {joined}"
    );
    assert!(
        error.contains("shared-constant pattern=\"resource-name\" value=\"orders-cache\""),
        "a pair id naming a declared constant class feeds the constant row, \
         never an async row (stderr: {error})"
    );
    let _ = error;
}

/// The declaration gate holds the other way too: without a `[[pattern_pair]]`
/// table the csharp scan serializes no literal bytes and prints no census
/// line (4.2 arm 1 — the byte identity, not a special case).
#[test]
fn non_declaring_csharp_scan_collects_no_candidate_bytes() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", &csproj(""));
    fx.write(
        "App/Program.cs",
        "namespace App;\npublic class Program\n{\n    public static void Main() { Bus.Emit(\"orders.created\"); }\n}\nstatic class Bus { public static void Emit(string _e) {} }\n",
    );
    let output = scan(&fx);
    assert!(
        !common::stdout(&output).contains("literal_sites")
            && !common::stdout(&output).contains("literal_candidates"),
        "the not-declared arm serializes no literal bytes"
    );
    assert!(
        !common::stderr(&output).contains("literal sites:"),
        "the not-declared arm prints no census line (stderr: {})",
        common::stderr(&output)
    );
}
