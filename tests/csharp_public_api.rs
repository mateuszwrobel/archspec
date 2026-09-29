//! C# public API surface facts (csharp public-api plan, US 01): the scan
//! emits `module_public_types` — public types attributed to their
//! namespace-derived module, production tier only, partials deduped.
//! Enforcement by `public_api_allowlist` is tested in
//! `csharp_public_api_allowlist.rs`. Rust trees serialize no such key (their
//! fact source stays the crate-root exports).

mod common;

use common::{stderr, stdout};
use serde_json::Value;

fn csproj(tf: &str) -> String {
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>{tf}</TargetFramework>\n  </PropertyGroup>\n</Project>\n"
    )
}

/// Test-tier csproj: the xunit runner package marks the project (the
/// `is_test_project` predicate), so its sources join no production fact.
fn xunit_csproj() -> &'static str {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"xunit\" Version=\"2.6.0\" />\n  </ItemGroup>\n</Project>\n"
}

/// The issue repro tree, welded as the regression fixture: one
/// `Microsoft.NET.Sdk` project (net8.0) whose sources split a public
/// contract namespace from a leaked implementation namespace. The class
/// bodies use `{ }`: the grammar produces no type declaration node for a
/// bodiless `class X;` spelling (same pre-existing shape as the
/// declared-type anchor), so the canonical body form is welded here.
fn issue_repro_tree() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("Sample/Sample.csproj", &csproj("net8.0"));
    fixture.write(
        "Sample/Contracts.cs",
        "namespace Sample.Contracts;\npublic sealed class PublicContract { }\n",
    );
    fixture.write(
        "Sample/Implementation.cs",
        "namespace Sample.Implementation;\npublic sealed class LeakedImplementation { }\n",
    );
    fixture
}

fn model_of(fixture: &common::Fixture) -> Value {
    let output = fixture.run(&["scan"]);
    assert_eq!(output.status.code(), Some(0), "scan must exit 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    serde_json::from_str(&stdout(&output)).expect("stdout must be JSON")
}

fn public_type_facts(fixture: &common::Fixture) -> Value {
    let model = model_of(fixture);
    model["module_public_types"].clone()
}

#[test]
fn issue_repro_scan_emits_public_type_facts_per_module() {
    let facts = public_type_facts(&issue_repro_tree());
    assert_eq!(
        facts,
        serde_json::json!({
            "Sample::Contracts": ["PublicContract"],
            "Sample::Implementation": ["LeakedImplementation"],
        }),
        "the repro tree must state the public types of both namespaces"
    );
}

#[test]
fn every_namespace_spelling_attributes_the_same_way() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/FileScoped.cs",
        "namespace App.FileFirst;\npublic class FromFileScoped { }\n",
    );
    fixture.write(
        "App/Block.cs",
        "namespace App.BlockFirst\n{\n    public class FromBlock { }\n}\n",
    );
    fixture.write(
        "App/Nested.cs",
        "namespace App\n{\n    namespace Outer\n    {\n        namespace Inner\n        {\n            public class FromNested { }\n        }\n    }\n}\n",
    );
    let facts = public_type_facts(&fixture);
    assert_eq!(
        facts["App::FileFirst"],
        serde_json::json!(["FromFileScoped"]),
        "file-scoped namespaces own their file"
    );
    assert_eq!(
        facts["App::BlockFirst"],
        serde_json::json!(["FromBlock"]),
        "block namespaces own their body"
    );
    // The innermost namespace context attributes the declaration — the very
    // spelling the soft tier and the declared-type anchor already use (the
    // driver records nested namespace declarations by their own name; facts
    // must not disagree with the soft tier).
    assert_eq!(
        facts["Inner"],
        serde_json::json!(["FromNested"]),
        "nested block namespaces attribute like the soft tier spells them"
    );
}

#[test]
fn global_namespace_files_attribute_to_the_unit_root() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write("App/Global.cs", "public class GlobalApi { }\n");
    let facts = public_type_facts(&fixture);
    // The unit root key mirrors `resolve_root_namespace`: with no csproj
    // RootNamespace/AssemblyName, the root is the common namespace prefix
    // of the unit's files truncated to the unit-name segment count, else
    // the full unit name — for App (a one-segment unit, namespace-less
    // file) all three ladder legs answer the unit name itself.
    assert_eq!(
        facts["App"],
        serde_json::json!(["GlobalApi"]),
        "a namespace-less file contributes to the unit root module"
    );
}

#[test]
fn kinds_partials_and_nested_types_are_the_public_surface() {
    let fixture = common::Fixture::new();
    fixture.write("Lib/Lib.csproj", &csproj("net8.0"));
    fixture.write(
        "Lib/Types.cs",
        "namespace Lib.Api;\npublic class Klass { }\npublic interface IFace { }\npublic struct Val { }\npublic enum Kind { A }\npublic record Point(int X);\npublic partial class Widget { }\ninternal class NotApi { }\nclass AlsoNotApi { }\n",
    );
    fixture.write(
        "Lib/WidgetMore.cs",
        "namespace Lib.Api;\npublic partial class Widget { }\n",
    );
    fixture.write(
        "Lib/Nested.cs",
        "namespace Lib.Api;\npublic class Outer { public class Inner { } internal class HiddenInner { } }\n",
    );
    // Grammar note (plan US 01): a bodiless `public sealed class Trailing;`
    // is NOT a type declaration node in this grammar version (the
    // pre-existing declared-type anchor misses it too), so that spelling
    // emits no fact; this test welds the forms the grammar resolves.
    let facts = public_type_facts(&fixture);
    assert_eq!(
        facts["Lib::Api"],
        serde_json::json!([
            "IFace",
            "Kind",
            "Klass",
            "Outer",
            "Outer.Inner",
            "Point",
            "Val",
            "Widget",
        ]),
        "public kinds with partials deduped and nested types named through \
         the parent; facts: {facts}"
    );
    assert!(
        facts["Lib::Api"]
            .as_array()
            .map(|types| !types
                .iter()
                .any(|v| v.as_str().is_some_and(|s| s.contains("NotApi"))))
            .unwrap_or(false),
        "modifier-less and internal types are not API: {facts}"
    );
}

#[test]
fn test_tier_projects_contribute_no_facts() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    fixture.write("App.Tests/App.Tests.csproj", xunit_csproj());
    fixture.write(
        "App.Tests/Specs.cs",
        "namespace App.Tests.Specs;\npublic class SpecHarness { }\n",
    );
    let model = model_of(&fixture);
    let facts = &model["module_public_types"];
    assert_eq!(
        facts["App::Core"],
        serde_json::json!(["Engine"]),
        "the production module states its public type"
    );
    assert!(
        facts.get("App::Tests::Specs").is_none(),
        "a dropped test project states no public-API fact: {facts}"
    );
    assert_eq!(
        model["units"]
            .as_array()
            .map(|units| units.len())
            .unwrap_or(0),
        1,
        "the test project is not a unit either"
    );
}

#[test]
fn rust_trees_serialize_no_public_type_facts_key() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn serve() {}\n");
    let model = model_of(&fixture);
    assert!(
        model.get("module_public_types").is_none(),
        "empty maps are omitted — rust scan bytes unchanged: {model}"
    );
}
