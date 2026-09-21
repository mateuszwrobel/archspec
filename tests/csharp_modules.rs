mod common;

use common::{stderr, stdout};
use serde_json::Value;

fn csproj(tf: &str) -> String {
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>{tf}</TargetFramework>\n  </PropertyGroup>\n</Project>\n"
    )
}

/// A single project `App` with namespaces `App::Services` (which uses
/// `App::Models`) and `App::Models`. The using appears BEFORE the file-scoped
/// namespace declaration. Shared by the scan soft-tier scenarios.
fn csharp_app_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/ReceiptService.cs",
        "using App.Models;\nnamespace App.Services;\npublic class ReceiptService { }\n",
    );
    fixture.write(
        "App/ReceiptModels.cs",
        "namespace App.Models;\npublic class ReceiptModel { }\n",
    );
    fixture
}

fn model_of(fixture: &common::Fixture) -> Value {
    let output = fixture.run(&["scan"]);
    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    serde_json::from_str(&stdout(&output)).expect("stdout must be JSON")
}

fn module_edge_strings(model: &Value) -> Vec<String> {
    model["module_edges"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|edge| {
                    format!(
                        "{}:{}->{}:{}",
                        edge["unit"].as_str().unwrap_or_default(),
                        edge["from"].as_str().unwrap_or_default(),
                        edge["to"].as_str().unwrap_or_default(),
                        edge["symbols"]
                            .as_array()
                            .map(|s| s
                                .iter()
                                .map(|v| v.as_str().unwrap_or_default().to_string())
                                .collect::<Vec<_>>()
                                .join(","))
                            .unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn assert_pass(output: &std::process::Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0 (stderr: {})",
        stderr(output)
    );
    assert!(
        stdout(output).starts_with("ok: "),
        "pass confirmation on stdout:\n{}",
        stdout(output)
    );
}

fn assert_fail(output: &std::process::Output, expected_lines: &[&str]) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected non-zero exit (stderr: {})",
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

/// Extract a metric value from a text report line like `components:          2`.
fn metric_value(text: &str, name: &str) -> String {
    text.lines()
        .find(|line| line.starts_with(&format!("{name}:")))
        .and_then(|line| line.split(':').nth(1))
        .map(str::trim)
        .unwrap_or_else(|| panic!("metric {name:?} missing from report:\n{text}"))
        .to_string()
}

// === Scan soft tier ===

/// File-scoped namespaces are listed in soft_structure as `::`-converted module
/// paths, sorted, keyed by the owning project (unit stem).
#[test]
fn scan_scenario_1_lists_namespaces_as_soft_structure() {
    let fixture = csharp_app_fixture();
    let model = model_of(&fixture);

    let modules = model["soft_structure"]["App"]
        .as_array()
        .expect("App soft_structure must be an array");
    let paths: Vec<&str> = modules
        .iter()
        .map(|m| m.as_str().expect("module path"))
        .collect();
    assert_eq!(
        paths,
        ["App::Models", "App::Services"],
        "namespaces listed as `::`-converted module paths, sorted"
    );
}

/// A `using` whose target namespace is declared in the SAME project records a
/// soft module edge between the two namespaces.
#[test]
fn scan_scenario_2_records_module_edge_from_within_project_using() {
    let fixture = csharp_app_fixture();
    let model = model_of(&fixture);

    let edges = model["module_edges"].as_array().expect("module_edges");
    assert_eq!(edges.len(), 1, "exactly one module-level edge");
    assert_eq!(edges[0]["unit"].as_str(), Some("App"));
    assert_eq!(edges[0]["from"].as_str(), Some("App::Services"));
    assert_eq!(edges[0]["to"].as_str(), Some("App::Models"));
    assert_eq!(
        edges[0]["symbols"].as_array().expect("symbols").len(),
        0,
        "C# usings carry no symbols"
    );
    assert_eq!(
        model["edges"].as_array().expect("edges").len(),
        0,
        "no hard unit edges in single project"
    );
}

/// Usings placed BEFORE the file-scoped `namespace X;` declaration still belong
/// to that namespace (the whole file is the file-scoped namespace's scope).
#[test]
fn scan_scenario_3_attributes_usings_before_file_scoped_namespace() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/OrdersService.cs",
        "using App.Models;\nusing App.Models;\nnamespace App.Services;\npublic class S { }\n",
    );
    fixture.write(
        "App/Models.cs",
        "namespace App.Models;\npublic class Model { }\n",
    );
    let model = model_of(&fixture);

    let strings = module_edge_strings(&model);
    assert_eq!(
        strings,
        ["App:App::Services->App::Models:"],
        "pre-namespace usings fold into the file-scoped namespace (deduped): {strings:?}"
    );
}

/// Block-style `namespace X { ... }` scopes its usings to that namespace.
#[test]
fn scan_scenario_4_handles_block_style_namespace() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/Data.cs",
        "namespace App.Data {\n    using App.Models;\n    public class Data { }\n}\n",
    );
    fixture.write(
        "App/Models.cs",
        "namespace App.Models;\npublic class Model { }\n",
    );
    let model = model_of(&fixture);

    let strings = module_edge_strings(&model);
    assert!(
        strings.contains(&"App:App::Data->App::Models:".to_string()),
        "block-style namespace using must be a soft edge: {strings:?}"
    );
    let soft: Vec<&str> = model["soft_structure"]["App"]
        .as_array()
        .expect("App soft_structure")
        .iter()
        .map(|m| m.as_str().expect("module path"))
        .collect();
    assert!(
        soft.contains(&"App::Data") && soft.contains(&"App::Models"),
        "block-style namespace must be listed: {soft:?}"
    );
}

/// A `using` whose target namespace is declared in ANOTHER project that the
/// referencing project can reach through project references is a usage fact:
/// it records BOTH the hard unit edge (manifest truth, from the
/// ProjectReference) and a cross-unit soft module edge (code truth, from the
/// using). The unit edge stays csproj-derived; the module edge carries the
/// namespace-module crossing the reference graph alone would hide.
#[test]
fn scan_scenario_5_cross_project_using_emits_unit_and_module_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Orders/Orders.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Orders.Abstractions\\Orders.Abstractions.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "Orders/OrderService.cs",
        "using Orders.Abstractions;\nnamespace Orders.App;\npublic class OrderService { }\n",
    );
    fixture.write(
        "Orders.Abstractions/Orders.Abstractions.csproj",
        &csproj("net8.0"),
    );
    fixture.write(
        "Orders.Abstractions/IOrdersService.cs",
        "namespace Orders.Abstractions;\npublic interface IOrdersService { }\n",
    );
    let model = model_of(&fixture);

    let edges = model["edges"].as_array().expect("edges");
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        pairs.contains(&("Orders", "Orders.Abstractions")),
        "the ProjectReference must be the hard edge: {pairs:?}"
    );

    let strings = module_edge_strings(&model);
    assert_eq!(
        strings,
        ["Orders:Orders::App->Orders::Abstractions:"],
        "cross-project using of a reference-reachable namespace is a cross-unit module edge: {strings:?}"
    );
}

/// A cross-project `using` whose owning project is reachable only TRANSITIVELY
/// (no direct ProjectReference — MSBuild flows references down) is still a
/// compilable source fact and records a cross-unit module edge.
#[test]
fn scan_scenario_5a_transitively_reachable_using_emits_module_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Worker\\Worker.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Controller.cs",
        "using App.Core;\nusing Core.Entities;\nnamespace App.Controllers;\npublic class Controller { }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Helper { }\n",
    );
    fixture.write(
        "Worker/Worker.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Core\\Core.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "Worker/Job.cs",
        "using Core.Entities;\nnamespace Worker.Jobs;\npublic class Job { }\n",
    );
    fixture.write("Core/Core.csproj", &csproj("net8.0"));
    fixture.write(
        "Core/Order.cs",
        "namespace Core.Entities;\npublic class Order { }\n",
    );
    let model = model_of(&fixture);

    let strings = module_edge_strings(&model);
    assert!(
        strings.contains(&"App:App::Controllers->Core::Entities:".to_string()),
        "transitive reference reach makes the using a compilable source fact: {strings:?}"
    );
    assert!(
        strings.contains(&"Worker:Worker::Jobs->Core::Entities:".to_string()),
        "direct reach emits too: {strings:?}"
    );
    // The laundering fact: the usage edge exists even though the reference
    // graph never names App -> Core.
    let pairs: Vec<(&str, &str)> = model["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        !pairs.contains(&("App", "Core")),
        "no ProjectReference means no unit edge: {pairs:?}"
    );
}

/// A `using` of a namespace owned by a project that is neither referenced nor
/// transitively reachable cannot compile: it is not a source fact and records
/// no module edge (same stance as uncompilable-package usings).
#[test]
fn scan_scenario_5b_unreachable_project_using_emits_no_edge() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/Controller.cs",
        "using Orphan.Things;\nnamespace App.Controllers;\npublic class Controller { }\n",
    );
    fixture.write("Orphan/Orphan.csproj", &csproj("net8.0"));
    fixture.write(
        "Orphan/Thing.cs",
        "namespace Orphan.Things;\npublic class Thing { }\n",
    );
    let model = model_of(&fixture);

    assert!(
        model["module_edges"].as_array().expect("module_edges").is_empty(),
        "a using into a non-reachable project cannot compile and emits no module edge"
    );
    assert!(
        model["edges"].as_array().expect("edges").is_empty(),
        "no reference means no unit edge"
    );
    assert!(
        model["module_external"].is_null()
            || model["module_external"]
                .as_object()
                .map(|map| map.values().all(|v| v.as_array().map(|a| a.is_empty()).unwrap_or(true)))
                .unwrap_or(true),
        "the foreign using must not attribute packages either"
    );
}

/// A project reference that no source file uses stays a manifest fact only:
/// the unit edge exists (reference truth), no module edge is invented for it
/// (usage truth), so the reference and usage graphs may legitimately diverge.
#[test]
fn scan_scenario_5c_unused_reference_emits_unit_edge_only() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Util\\Util.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Service.cs",
        "namespace App.Services;\npublic class Service { }\n",
    );
    fixture.write("Util/Util.csproj", &csproj("net8.0"));
    fixture.write(
        "Util/Helper.cs",
        "namespace Util.Text;\npublic class Helper { }\n",
    );
    let model = model_of(&fixture);

    let pairs: Vec<(&str, &str)> = model["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        pairs.contains(&("App", "Util")),
        "the unused reference stays a unit edge (manifest truth): {pairs:?}"
    );
    assert!(
        model["module_edges"].as_array().expect("module_edges").is_empty(),
        "no using means no module edge — references and usings are different facts"
    );

    // Ceiling semantics unchanged: a spec permitting the reference verifies clean.
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\nallowed = { depend_on = [\"Util\"] }\n\n[[module]]\nname = \"Util\"\nmatches = { units = [\"Util\"] }\n",
    );
    assert_pass(&fixture.run(&["verify"]));
}

/// A `using` of a namespace no scanned unit owns (framework, NuGet package,
/// unknown) records no module edge and keeps flowing to the external tier
/// exactly as before, even when the tree carries cross-unit edges.
#[test]
fn scan_scenario_5d_framework_using_keeps_external_attribution() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Core\\Core.csproj\" />\n    <PackageReference Include=\"Newtonsoft.Json\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Service.cs",
        "using Newtonsoft.Json;\nusing System.Text;\nusing Core.Model;\nnamespace App.Services;\npublic class Service { }\n",
    );
    fixture.write("Core/Core.csproj", &csproj("net8.0"));
    fixture.write(
        "Core/Model.cs",
        "namespace Core.Model;\npublic class Model { }\n",
    );
    let model = model_of(&fixture);

    let strings = module_edge_strings(&model);
    assert_eq!(
        strings,
        ["App:App::Services->Core::Model:"],
        "only the unit-owned target becomes a module edge: {strings:?}"
    );
    let crates: Vec<&str> = model["module_external"]["App::Services"]
        .as_array()
        .expect("module_external for App::Services")
        .iter()
        .map(|n| n.as_str().expect("package id"))
        .collect();
    assert_eq!(
        crates,
        ["Newtonsoft.Json"],
        "external attribution of framework/package usings unchanged"
    );
}

/// Test-tier parity across the unit boundary: a test project's sources using
/// production namespaces emit no production edges, and a production using that
/// falls under a dropped test project's namespace prefix stays dead — no
/// owner, no edge, no resurrected phantom module.
#[test]
fn scan_scenario_5e_test_projects_source_no_production_edges() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/Service.cs",
        "using App.Models;\nusing App.Tests.Specs;\nnamespace App.Services;\npublic class Service { }\n",
    );
    fixture.write(
        "App/Model.cs",
        "namespace App.Models;\npublic class Model { }\n",
    );
    fixture.write(
        "App.Tests/App.Tests.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\App\\App.csproj\" />\n    <PackageReference Include=\"xunit\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App.Tests/ServiceTests.cs",
        "using App.Models;\nusing App.Services;\nnamespace App.Tests.Specs;\npublic class ServiceTests { }\n",
    );
    let model = model_of(&fixture);

    let strings = module_edge_strings(&model);
    assert_eq!(
        strings,
        ["App:App::Services->App::Models:"],
        "production using survives, test-project sourcing and dead-prefix targeting emit nothing: {strings:?}"
    );
    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(units, ["App"], "the test project is excluded from every tier");
}

/// Two production units declaring the SAME namespace make prefix ownership
/// ambiguous: a using of that namespace resolves to no honest owner and emits
/// no cross-unit edge (misresolution degrades to a missing edge, never a
/// wrong edge).
#[test]
fn scan_scenario_5f_ambiguous_namespace_ownership_emits_no_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "P1/P1.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\P2\\P2.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "P1/Use.cs",
        "using Shared.Ns.Deeper;\nnamespace P1.Callers;\npublic class Use { }\n",
    );
    fixture.write(
        "P1/Shared.cs",
        "namespace Shared.Ns;\npublic class InP1 { }\n",
    );
    fixture.write("P2/P2.csproj", &csproj("net8.0"));
    fixture.write(
        "P2/Shared.cs",
        "namespace Shared.Ns;\npublic class InP2 { }\n",
    );
    let model = model_of(&fixture);

    let strings = module_edge_strings(&model);
    assert!(
        !strings.iter().any(|e| e.contains("->Shared::Ns")),
        "an ambiguous prefix owner must never fabricate an edge: {strings:?}"
    );
}

/// Referenced packages surfaced by a module's usings are recorded in
/// module_external as PACKAGE ids (the vocabulary forbid rules are written in),
/// not namespaces. A `System` root with no matching reference and a namespace
/// mapping to no referenced package at all are not dependency facts and are
/// dropped.
#[test]
fn scan_scenario_6_records_module_external_skipping_system() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Microsoft.EntityFrameworkCore\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Repo.cs",
        "using Microsoft.EntityFrameworkCore;\nusing System.Linq;\nusing Microsoft.CodeAnalysis;\nnamespace App.Persistence;\npublic class Repo { }\n",
    );
    let model = model_of(&fixture);

    let crates = model["module_external"]["App::Persistence"]
        .as_array()
        .expect("module_external for App::Persistence")
        .iter()
        .map(|n| n.as_str().expect("package id"))
        .collect::<Vec<_>>();
    assert_eq!(
        crates,
        ["Microsoft.EntityFrameworkCore"],
        "referenced package recorded; unreferenced namespaces (BCL, uncompilable) dropped"
    );
}

/// NuGet packages referenced in csproj are listed in `external` and surfaced in
/// `unit_manifests.dependencies` for the owning project.
#[test]
fn scan_scenario_7_records_nuget_packages_external_and_dependencies() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Newtonsoft.Json\" />\n    <PackageReference Include=\"Microsoft.EntityFrameworkCore\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Service.cs",
        "namespace App.Services;\npublic class Service { }\n",
    );
    let model = model_of(&fixture);

    let external: Vec<&str> = model["external"]
        .as_array()
        .expect("external")
        .iter()
        .map(|n| n.as_str().expect("package name"))
        .collect();
    assert_eq!(
        external,
        ["Microsoft.EntityFrameworkCore", "Newtonsoft.Json"],
        "external lists NuGet packages, sorted"
    );

    let um = model["unit_manifests"]["App"]
        .as_object()
        .expect("App unit manifest facts");
    let deps: Vec<&str> = um["dependencies"]
        .as_array()
        .expect("App dependencies")
        .iter()
        .map(|d| d.as_str().expect("package"))
        .collect();
    assert_eq!(
        deps,
        ["Microsoft.EntityFrameworkCore", "Newtonsoft.Json"],
        "unit_manifests.dependencies surfaces the packages"
    );
}

/// `<IsPackable>false</IsPackable>` maps to publish=false in unit_manifests (and
/// the single-project manifest); an absent flag is null, distinct from false.
#[test]
fn scan_scenario_8_records_is_packable_false() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n    <IsPackable>false</IsPackable>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Service.cs",
        "namespace App.Services;\npublic class Service { }\n",
    );
    let model = model_of(&fixture);

    assert_eq!(
        model["unit_manifests"]["App"]["publish"],
        serde_json::Value::Bool(false),
        "IsPackable=false is publish=false"
    );
    assert_eq!(
        model["manifest"]["publish"],
        serde_json::Value::Bool(false),
        "single-project manifest reflects publish=false"
    );

    let plain = common::Fixture::new();
    plain.write("App/App.csproj", &csproj("net8.0"));
    plain.write(
        "App/Service.cs",
        "namespace App.Services;\npublic class Service { }\n",
    );
    let plain_model = model_of(&plain);
    assert_eq!(
        plain_model["unit_manifests"]["App"]["publish"],
        serde_json::Value::Null,
        "absent IsPackable is null (None), distinct from false"
    );
}

/// Comments, block comments, and string literals never produce false usings.
#[test]
fn scan_scenario_9_comments_and_strings_do_not_create_false_usings() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/Sneaky.cs",
        "// using Fake.Imports;\n/* using Another.Fake; */\nvar s = \"using NotAUsing;\";\nnamespace App.Services;\npublic class S { }\n",
    );
    let model = model_of(&fixture);

    assert!(
        model["module_edges"].as_array().expect("module_edges").is_empty(),
        "no fake module edges"
    );
    let module_external = model["module_external"]
        .as_object()
        .expect("module_external");
    assert!(
        module_external.values().all(|crates| {
            crates
                .as_array()
                .map(|arr| arr.is_empty())
                .unwrap_or(true)
        }),
        "no external usages from comments/strings"
    );
}

/// Scan output is byte-identical across runs for an unchanged C# tree.
#[test]
fn scan_scenario_10_deterministic_across_runs() {
    let fixture = csharp_app_fixture();
    let first = fixture.run(&["scan"]);
    let second = fixture.run(&["scan"]);

    assert_eq!(first.status.code(), Some(0), "first run exit 0");
    assert_eq!(second.status.code(), Some(0), "second run exit 0");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "csharp scan output must be byte-identical across runs"
    );
}

// === Verify constraints ===

/// A single project `Orders` with namespaces `Orders::App` (uses `Orders::Models`)
/// and `Orders::Models`. The spec body supplies the module boundaries and
/// `allowed.depend_on`.
fn module_boundary_fixture(spec_body: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("Orders/Orders.csproj", &csproj("net8.0"));
    fixture.write(
        "Orders/OrderService.cs",
        "using Orders.Models;\nnamespace Orders.App;\npublic class OrderService { }\n",
    );
    fixture.write(
        "Orders/OrderModels.cs",
        "namespace Orders.Models;\npublic class OrderModel { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        &format!("[project]\nlanguage = \"csharp\"\n\n{spec_body}"),
    );
    fixture
}

/// A spec with module boundaries (`matches.modules = ["Orders::App"]`) plus the
/// matching `allowed.depend_on` verifies clean.
#[test]
fn verify_scenario_11_module_boundaries_with_allowed_depend_on_verify_clean() {
    let fixture = module_boundary_fixture(
        "[[module]]\nname = \"Orders::App\"\nmatches = { modules = [\"Orders::App\"] }\n\n[module.allowed]\ndepend_on = [\"Orders::Models\"]\n\n[[module]]\nname = \"Orders::Models\"\nmatches = { modules = [\"Orders::Models\"] }\n",
    );
    assert_pass(&fixture.run(&["verify"]));
}

/// A module edge that violates `allowed.depend_on` is caught as a disallowed
/// cross-component dependency.
#[test]
fn verify_scenario_12_forbidden_module_dependency_is_caught() {
    let fixture = module_boundary_fixture(
        "[[module]]\nname = \"Orders::App\"\nmatches = { modules = [\"Orders::App\"] }\n\n[module.allowed]\ndepend_on = []\n\n[[module]]\nname = \"Orders::Models\"\nmatches = { modules = [\"Orders::Models\"] }\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["disallowed cross-component dependency: Orders::App -> Orders::Models"],
    );
}

/// `no_cycles` works over C# module boundaries: a namespace cycle A -> B -> A is
/// reported on the component names.
#[test]
fn verify_scenario_13_no_cycles_constraint_works_over_csharp_boundaries() {
    let fixture = common::Fixture::new();
    fixture.write("Orders/Orders.csproj", &csproj("net8.0"));
    fixture.write(
        "Orders/A.cs",
        "using Orders.B;\nnamespace Orders.A;\npublic class A { }\n",
    );
    fixture.write(
        "Orders/B.cs",
        "using Orders.A;\nnamespace Orders.B;\npublic class B { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Orders::A\"\nmatches = { modules = [\"Orders::A\"] }\n\n[module.allowed]\ndepend_on = [\"Orders::B\"]\n\n[[module]]\nname = \"Orders::B\"\nmatches = { modules = [\"Orders::B\"] }\n\n[module.allowed]\ndepend_on = [\"Orders::A\"]\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"Orders::A\", \"Orders::B\"]\nseverity = \"error\"\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["cycle: Orders::A -> Orders::B -> Orders::A"],
    );
}

/// `forbid_external_crates` matches the PACKAGE universe: a referenced package
/// whose namespace a module imports is caught (here the referenced
/// `Microsoft.EntityFrameworkCore` package banned via `Microsoft*`).
#[test]
fn verify_scenario_14_forbid_external_crates_catches_external_using() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Microsoft.EntityFrameworkCore\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Repo.cs",
        "using Microsoft.EntityFrameworkCore;\nnamespace App.Persistence;\npublic class Repo { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"App::Persistence\"]\nforbid = [\"Microsoft*\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden external crate: Persistence -> Microsoft.EntityFrameworkCore"],
    );

    let clean = common::Fixture::new();
    clean.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Microsoft.EntityFrameworkCore\" />\n  </ItemGroup>\n</Project>\n",
    );
    clean.write(
        "App/Repo.cs",
        "using Microsoft.EntityFrameworkCore;\nnamespace App.Persistence;\npublic class Repo { }\n",
    );
    clean.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"App::Persistence\"]\nforbid = [\"Newtonsoft*\"]\n",
    );
    assert_pass(&clean.run(&["verify"]));
}

/// `manifest_integrity` with `forbidden_dependencies` catches a PackageReference
/// declared in the project's csproj.
#[test]
fn verify_scenario_15_manifest_integrity_catches_package_reference() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Newtonsoft.Json\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Service.cs",
        "namespace App.Services;\npublic class Service { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"Newtonsoft.Json\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["has forbidden dependency Newtonsoft.Json"],
    );
}

/// `manifest_integrity` with `require_publish` catches an `<IsPackable>false`
/// project as publish=false (required true).
#[test]
fn verify_scenario_16_manifest_integrity_require_publish_catches_is_packable() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n    <IsPackable>false</IsPackable>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Service.cs",
        "namespace App.Services;\npublic class Service { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[constraint]]\ntype = \"manifest_integrity\"\nrequire_publish = true\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["publish=false (required true)"],
    );
}

// === Update + Report ===

/// `update` seeds a spec from a C# tree: a unit boundary per project plus a
/// module boundary per top-level namespace, with the module edge folded into the
/// source boundary's `allowed.depend_on`. The seed verifies clean against
/// itself.
#[test]
fn update_scenario_17_seeds_csharp_spec_and_verifies_clean() {
    let fixture = common::Fixture::new();
    fixture.write("Orders/Orders.csproj", &csproj("net8.0"));
    fixture.write(
        "Orders/OrderService.cs",
        "using Orders.Models;\nnamespace Orders.App;\npublic class OrderService { }\n",
    );
    fixture.write(
        "Orders/OrderModels.cs",
        "namespace Orders.Models;\npublic class OrderModel { }\n",
    );

    let update = fixture.run(&["update"]);
    assert_eq!(
        update.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&update)
    );

    let spec = fixture.read("architecture.spec.toml");
    let expected = "[project]\n\
        language = \"csharp\"\n\
        \n\
        [[module]]\n\
        name = \"Orders\"\n\
        matches = { units = [\"Orders\"] }\n\
        \n\
        [[module]]\n\
        name = \"Orders::App\"\n\
        matches = { modules = [\"Orders::App\"] }\n\
        allowed = { depend_on = [\"Orders::Models\"] }\n\
        \n\
        [[module]]\n\
        name = \"Orders::Models\"\n\
        matches = { modules = [\"Orders::Models\"] }\n\
        \n\
        [[constraint]]\n\
        type = \"no_cycles\"\n\
        \n";
    assert_eq!(spec, expected, "seed spec must be byte-exact:\n{spec}");

    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed must verify clean (exit 0): {}",
        stdout(&verify)
    );
    let out = stdout(&verify);
    assert!(
        out.contains("matches source model") && !out.contains("violation"),
        "seed must match source model, no violations:\n{out}"
    );
}

/// `report` shows metrics (components, units, edges) for a C# tree with a unit
/// boundary, two module boundaries, and one module edge.
#[test]
fn report_scenario_18_shows_csharp_metrics() {
    let fixture = common::Fixture::new();
    fixture.write("Orders/Orders.csproj", &csproj("net8.0"));
    fixture.write(
        "Orders/OrderService.cs",
        "using Orders.Models;\nnamespace Orders.App;\npublic class OrderService { }\n",
    );
    fixture.write(
        "Orders/OrderModels.cs",
        "namespace Orders.Models;\npublic class OrderModel { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Orders\"\nmatches = { units = [\"Orders\"] }\n\n[[module]]\nname = \"Orders::App\"\nmatches = { modules = [\"Orders::App\"] }\n\n[module.allowed]\ndepend_on = [\"Orders::Models\"]\n\n[[module]]\nname = \"Orders::Models\"\nmatches = { modules = [\"Orders::Models\"] }\n",
    );

    let output = fixture.run(&["report"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "report must exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert_eq!(
        metric_value(&out, "components"),
        "3",
        "components counts the unit + two module boundaries:\n{out}"
    );
    assert_eq!(metric_value(&out, "units"), "1");
    assert_eq!(
        metric_value(&out, "edges (internal)"),
        "1",
        "the module edge is unit-internal:\n{out}"
    );
    assert_eq!(metric_value(&out, "edges (external)"), "0");
    assert!(out.contains("Result: clean"), "summary:\n{out}");
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

// === Cross-unit using edges: verify, depgraph, update ===

/// A leaking tree: `Acme.Api` references `Acme.Core` (no direct reference to
/// `Acme.Domain`, reachable transitively through Core) and its controller
/// `using`s the Domain namespace directly. Shared by the cross-unit verify
/// scenarios; `leak` toggles the transitive using on/off.
fn layered_leak_fixture(spec_body: &str, leak: bool) -> common::Fixture {
    let fixture = common::Fixture::new();
    let api_csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Acme.Core\\Acme.Core.csproj\" />\n  </ItemGroup>\n</Project>\n";
    fixture.write("Acme.Api/Acme.Api.csproj", api_csproj);
    let mut controller = String::from("using Acme.Core.Services;\n");
    if leak {
        controller.push_str("using Acme.Domain.Entities;\n");
    }
    controller.push_str("namespace Acme.Api;\npublic class Controller { }\n");
    fixture.write("Acme.Api/Controller.cs", &controller);
    let core_csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Acme.Domain\\Acme.Domain.csproj\" />\n  </ItemGroup>\n</Project>\n";
    fixture.write("Acme.Core/Acme.Core.csproj", core_csproj);
    fixture.write(
        "Acme.Core/Service.cs",
        "using Acme.Domain.Entities;\nnamespace Acme.Core.Services;\npublic class Service { }\n",
    );
    fixture.write("Acme.Domain/Acme.Domain.csproj", &csproj("net8.0"));
    fixture.write(
        "Acme.Domain/Order.cs",
        "namespace Acme.Domain.Entities;\npublic class Order { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        &format!("[project]\nlanguage = \"csharp\"\n\n{spec_body}"),
    );
    fixture
}

/// The layered spec at NAMESPACE granularity: Api may depend only on Core.
/// The controller's direct using of the Domain namespace (transitively
/// reachable, no direct reference) is a cross-unit module edge, so verify
/// reports the disallowed dependency naming the modules — and the same tree
/// with the using removed verifies clean.
#[test]
fn verify_scenario_23_namespace_boundaries_catch_cross_unit_using_leak() {
    let namespace_spec = "[[module]]\nname = \"Acme::Api\"\nmatches = { modules = [\"Acme::Api\"] }\nallowed = { depend_on = [\"Acme::Core\"] }\n\n[[module]]\nname = \"Acme::Core\"\nmatches = { modules = [\"Acme::Core\"] }\nallowed = { depend_on = [\"Acme::Domain\"] }\n\n[[module]]\nname = \"Acme::Domain\"\nmatches = { modules = [\"Acme::Domain\"] }\n";
    assert_fail(
        &layered_leak_fixture(namespace_spec, true).run(&["verify"]),
        &["disallowed cross-component dependency: Acme::Api -> Acme::Domain"],
    );
    assert_pass(&layered_leak_fixture(namespace_spec, false).run(&["verify"]));
}

/// The same leaking tree with a UNITS-ONLY spec: the cross-unit edge endpoints
/// (`Acme::Api`, `Acme::Domain::Entities`, ...) are addressed by no boundary
/// and the unit fallback keys on the first path segment, which is the shared
/// namespace root — not a csproj name. The leak surfaces as loud unowned
/// endpoint warnings (never silence): verify passes without `--strict` and
/// fails with it. The audit-guide remedy — declare the namespace boundaries —
/// turns warnings into precise findings (scenario above).
#[test]
fn verify_scenario_24_units_only_spec_reports_cross_unit_endpoints_unowned() {
    let units_spec = "[[module]]\nname = \"Acme.Api\"\nmatches = { units = [\"Acme.Api\"] }\nallowed = { depend_on = [\"Acme.Core\"] }\n\n[[module]]\nname = \"Acme.Core\"\nmatches = { units = [\"Acme.Core\"] }\nallowed = { depend_on = [\"Acme.Domain\"] }\n\n[[module]]\nname = \"Acme.Domain\"\nmatches = { units = [\"Acme.Domain\"] }\n";
    let fixture = layered_leak_fixture(units_spec, true);

    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "warnings alone must stay warning-level (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("unowned module edge endpoint: Acme::Api"),
        "cross-unit edge endpoints outside declared boundaries must be reported:\n{out}"
    );
    assert!(
        out.contains("unowned module edge endpoint: Acme::Domain::Entities"),
        "the deep target endpoint must be reported:\n{out}"
    );

    let strict = fixture.run(&["verify", "--strict"]);
    assert_ne!(
        strict.status.code(),
        Some(0),
        "--strict must promote unowned-endpoint warnings to failures"
    );
}

/// `depgraph modules` renders the cross-unit boundary crossing: the using of
/// `Core.Entities` from `App::Controllers` (reachable only transitively, so no
/// unit edge projects this pair) appears as a module-view arrow.
#[test]
fn depgraph_scenario_25_modules_view_renders_cross_unit_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Worker\\Worker.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Controller.cs",
        "using Core.Entities;\nnamespace App.Controllers;\npublic class Controller { }\n",
    );
    fixture.write(
        "Worker/Worker.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Core\\Core.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "Worker/Job.cs",
        "using Core.Entities;\nnamespace Worker.Jobs;\npublic class Job { }\n",
    );
    fixture.write("Core/Core.csproj", &csproj("net8.0"));
    fixture.write(
        "Core/Order.cs",
        "namespace Core.Entities;\npublic class Order { }\n",
    );
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("Controllers --> Entities"),
        "the transitive usage must cross unit boundaries in the module view:\n{out}"
    );
}

/// `update` seeds cross-unit usage into the module boundary permissions: the
/// tree that launders a layer through a third project now shows the DIRECT
/// usage edge in the seed (the laundering path is no longer the only blessed
/// route), and the seed verifies clean against itself.
#[test]
fn update_scenario_26_seed_covers_cross_unit_usage_and_verifies_clean() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Worker\\Worker.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Controller.cs",
        "using Worker.Pipeline;\nusing Core.Entities;\nnamespace App.Controllers;\npublic class Controller { }\n",
    );
    fixture.write(
        "Worker/Worker.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Core\\Core.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "Worker/Pipeline.cs",
        "using Core.Entities;\nnamespace Worker.Pipeline;\npublic class Pipeline { }\n",
    );
    fixture.write("Core/Core.csproj", &csproj("net8.0"));
    fixture.write(
        "Core/Order.cs",
        "namespace Core.Entities;\npublic class Order { }\n",
    );

    let update = fixture.run(&["update"]);
    assert_eq!(
        update.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&update)
    );
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("depend_on = [\"Core::Entities\", \"Worker::Pipeline\"]"),
        "the seed must permission the direct cross-unit usage, not only the reference route:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    let out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed must verify clean against itself (stderr: {})",
        stderr(&verify)
    );
    assert!(
        out.contains("matches source model") && !out.contains("unowned module edge endpoint"),
        "cross-unit usage seed must match with no unowned warnings:\n{out}"
    );
}

/// A c# tree whose cross-unit USAGE forms a cycle (each project using the
/// other's namespace; mutual references make it compilable): `update` seeds
/// the permissions, and `verify` fails loudly on the cycle instead of the
/// seed silently blessing it — the same acyclic-only seed guarantee as today.
#[test]
fn update_scenario_27_usage_cycle_seed_fails_no_cycles_loudly() {
    let fixture = common::Fixture::new();
    let api_csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Core\\Core.csproj\" />\n  </ItemGroup>\n</Project>\n";
    let core_csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\App\\App.csproj\" />\n  </ItemGroup>\n</Project>\n";
    fixture.write("App/App.csproj", api_csproj);
    fixture.write(
        "App/Controller.cs",
        "using Core.Entities;\nnamespace App.Controllers;\npublic class Controller { }\n",
    );
    fixture.write("Core/Core.csproj", core_csproj);
    fixture.write(
        "Core/Order.cs",
        "using App.Utils;\nnamespace Core.Entities;\npublic class Order { }\n",
    );
    fixture.write(
        "App/Utils.cs",
        "namespace App.Utils;\npublic class Helper { }\n",
    );

    let update = fixture.run(&["update"]);
    assert_eq!(
        update.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&update)
    );
    let verify = fixture.run(&["verify"]);
    assert_ne!(
        verify.status.code(),
        Some(0),
        "a usage cycle must fail verify, not be blessed by the seed:\n{}",
        stdout(&verify)
    );
    assert!(
        stdout(&verify).contains("cycle"),
        "the cycle finding must name the cycle:\n{}",
        stdout(&verify)
    );
}

// === Namespace-less (composition-root) files ===

/// A file with no namespace declaration (top-level-statements composition
/// root) attributes its usings to the project's root module: the package shows
/// up in `module_external` under the root key, and the root module exists in
/// `soft_structure` even when the project contains only such files.
#[test]
fn scan_scenario_19_namespace_less_file_usings_land_on_root_module() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Microsoft.EntityFrameworkCore\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Program.cs",
        "using Microsoft.EntityFrameworkCore;\nusing System.Text;\nvar app = new object();\n",
    );
    let model = model_of(&fixture);

    let crates: Vec<&str> = model["module_external"]["App"]
        .as_array()
        .unwrap_or_else(|| panic!("root module must carry the usage:\n{model}"))
        .iter()
        .map(|n| n.as_str().expect("package id"))
        .collect();
    assert_eq!(
        crates,
        ["Microsoft.EntityFrameworkCore"],
        "namespace-less file usings attribute to the project root module"
    );

    let modules: Vec<&str> = model["soft_structure"]["App"]
        .as_array()
        .expect("App soft_structure must be an array")
        .iter()
        .map(|n| n.as_str().expect("module path"))
        .collect();
    assert_eq!(
        modules,
        ["App"],
        "root module exists in soft_structure when only root files exist"
    );
}

/// Mixed tree: namespaced files keep their attribution on their namespace
/// modules; namespace-less usings appear ONLY on the root module — never
/// duplicated onto children.
#[test]
fn scan_scenario_20_root_file_usings_do_not_duplicate_onto_children() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Microsoft.EntityFrameworkCore\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/ReceiptsController.cs",
        "using App.Models;\nnamespace App.Controllers;\npublic class ReceiptsController { }\n",
    );
    fixture.write(
        "App/Receipt.cs",
        "namespace App.Models;\npublic class Receipt { }\n",
    );
    fixture.write(
        "App/Program.cs",
        "using Microsoft.EntityFrameworkCore;\nusing App.Models;\nvar app = new object();\n",
    );
    let model = model_of(&fixture);

    let root_crates: Vec<&str> = model["module_external"]["App"]
        .as_array()
        .expect("root module carries the namespace-less file's package")
        .iter()
        .map(|n| n.as_str().expect("package id"))
        .collect();
    assert_eq!(root_crates, ["Microsoft.EntityFrameworkCore"]);

    let external_modules: Vec<&str> = model["module_external"]
        .as_object()
        .expect("module_external object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        external_modules,
        ["App"],
        "namespaced usings resolve in-project; the package must not duplicate onto children"
    );

    let edges = module_edge_strings(&model);
    assert!(
        edges.contains(&"App:App->App::Models:".to_string()),
        "root file using of an in-project namespace is a soft edge from the root module: {edges:?}"
    );
    assert!(
        edges.contains(&"App:App::Controllers->App::Models:".to_string()),
        "namespaced attribution unchanged: {edges:?}"
    );

    let modules: Vec<&str> = model["soft_structure"]["App"]
        .as_array()
        .expect("App soft_structure")
        .iter()
        .map(|n| n.as_str().expect("module path"))
        .collect();
    assert_eq!(
        modules,
        ["App", "App::Controllers", "App::Models"],
        "root module joins the declared namespaces exactly once"
    );
}

/// The observable hole end-to-end: a `forbid_external_crates` rule covering the
/// composition root's using now fires — namespace-less files are no longer
/// exempt from external-usage containment.
#[test]
fn verify_scenario_21_forbid_external_crates_fires_on_namespace_less_file() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Microsoft.EntityFrameworkCore\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Program.cs",
        "using Microsoft.EntityFrameworkCore;\nvar app = new object();\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"App\"]\nforbid = [\"Microsoft*\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden external crate: App -> Microsoft.EntityFrameworkCore"],
    );
}

/// Module keys are global while the attribution map is per-unit, and distinct
/// units can produce the SAME key: unit `A.B`'s namespace `A.B` and unit
/// `A.B.C`'s namespace-less composition root (root module = first two segments
/// of the unit name) both resolve to `A::B`. The cross-unit flatten must UNION
/// the package sets — last-write-wins silently drops one unit's fact.
#[test]
fn scan_scenario_22_shared_module_key_unions_packages_across_units() {
    let fixture = common::Fixture::new();
    fixture.write(
        "A.B/A.B.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Newtonsoft.Json\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "A.B/Thing.cs",
        "using Newtonsoft.Json;\nnamespace A.B;\npublic class Thing { }\n",
    );
    fixture.write(
        "A.B.C/A.B.C.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Serilog\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "A.B.C/Program.cs",
        "using Serilog;\nvar app = new object();\n",
    );
    let model = model_of(&fixture);

    let crates: Vec<&str> = model["module_external"]["A::B"]
        .as_array()
        .unwrap_or_else(|| panic!("shared module key A::B must carry both units' packages:\n{model}"))
        .iter()
        .map(|n| n.as_str().expect("package id"))
        .collect();
    assert_eq!(
        crates,
        ["Newtonsoft.Json", "Serilog"],
        "cross-unit flatten unions per shared module key, never last-write-wins"
    );
}
