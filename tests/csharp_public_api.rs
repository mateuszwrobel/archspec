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
/// declared-type anchor), so the canonical body form is welded here. Records
/// are NOT bound by that shape: `record_declaration` carries a
/// `declaration_list | ';'` body, so record declarations land in every body
/// shape (GitHub issue #2 investigation — see
/// `record_declarations_emit_facts_in_every_body_shape`).
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
    // Grammar note (plan US 01, refined by the GitHub issue #2 investigation):
    // a bodiless `public sealed class Trailing;` is NOT a type declaration
    // node in this grammar version (the pre-existing declared-type anchor
    // misses it too), so that spelling emits no fact; the same bound covers
    // `;`-bodied `interface`/`struct`/`enum` spellings (workplan
    // csharp_declaration_bodies). Record declarations carry a
    // `declaration_list | ';'` body in the grammar and land in EVERY shape —
    // welded by `record_declarations_emit_facts_in_every_body_shape`.
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

/// GitHub issue #2 verbatim scan repro: the brace-bodied record control in
/// the contracts namespace and the SEMICOLON-BODIED positional record in the
/// implementation namespace. The report claimed the second emits no fact;
/// the pinned grammar gives the record family its own `record_declaration`
/// node whose body is `declaration_list | ';'`, so both shapes emit facts
/// (the enforcement half of the repro lives in
/// `csharp_public_api_allowlist::issue_two_semicolon_bodied_record...`).
#[test]
fn issue_two_record_body_shapes_both_emit_facts() {
    let fixture = common::Fixture::new();
    fixture.write("Sample/Sample.csproj", &csproj("net8.0"));
    fixture.write(
        "Sample/Contracts.cs",
        "namespace Sample.Contracts;\npublic sealed record BraceBodiedContract(string Id) { }\n",
    );
    fixture.write(
        "Sample/Implementation.cs",
        "namespace Sample.Implementation;\npublic sealed record SemicolonBodiedLeak(string Id);\n",
    );
    let facts = public_type_facts(&fixture);
    assert_eq!(
        facts,
        serde_json::json!({
            "Sample::Contracts": ["BraceBodiedContract"],
            "Sample::Implementation": ["SemicolonBodiedLeak"],
        }),
        "both record body shapes emit public-API facts — body shape never \
         decides detection: {facts}"
    );
}

/// The record-family matrix (GitHub issue #2 asks the regression to cover
/// every declaration shape): positional with and without trailing semicolon,
/// braced, bodiless `record X;`, positional `record struct`, positional
/// `record class`, a partial split across a semicolon and a braced part,
/// generic positional, `where`-claused, attributed, and one nested in a
/// public class. Every shape dumps to `record_declaration` in the pinned
/// grammar, so every shape emits its fact; the partial dedupes and the
/// nested type qualifies through its parent like every other kind.
#[test]
fn record_declarations_emit_facts_in_every_body_shape() {
    let fixture = common::Fixture::new();
    fixture.write("Lib/Lib.csproj", &csproj("net8.0"));
    fixture.write(
        "Lib/Records.cs",
        "namespace Lib.Records;\npublic sealed record PositionalSemi(string Id);\npublic record PositionalBraced(int X) { }\npublic record Braced { }\npublic record BodilessRecord;\npublic record struct PositionalStruct(int X, int Y);\npublic record class PositionalClass(int X);\npublic partial record SplitPart(int X);\npublic partial record SplitPart { }\npublic record Generic<T>(T V);\npublic record WhereClaused<T>(T V) where T : class;\n[Obsolete]\npublic record Attributed(int X);\npublic class Outer { public sealed record Nested(int X); }\n",
    );
    let facts = public_type_facts(&fixture);
    assert_eq!(
        facts["Lib::Records"],
        serde_json::json!([
            "Attributed",
            "BodilessRecord",
            "Braced",
            "Generic",
            "Outer",
            "Outer.Nested",
            "PositionalBraced",
            "PositionalClass",
            "PositionalSemi",
            "PositionalStruct",
            "SplitPart",
            "WhereClaused",
        ]),
        "every public record spelling lands regardless of body shape, the \
         partial parts dedupe and the nested record qualifies through its \
         parent: {facts}"
    );
}

/// The real boundary the GitHub issue #2 investigation found. The grammar
/// offers the `;` body alternative to RECORDS only: `class_declaration`,
/// `struct_declaration` and `interface_declaration` REQUIRE a braced
/// `declaration_list` body and `enum_declaration` a braced
/// `enum_member_declaration_list`, so `class X;`, `class X(int A);`,
/// `struct X(int A);`, `interface I;` and `enum E;` produce NO declaration
/// node — they parse to ERROR fragments (children: `modifier`* plus a bare
/// `identifier` name, optionally a `parameter_list`; the `;` reparses as a
/// sibling `empty_statement`). Facts are never invented from error recovery:
/// garbage like a stray `public Foo;` dumps to the same fragment shape, so
/// declaring types from ERROR children would attribute public API to
/// unparsable code. The weld doubles as the adjacency proof — an ERROR
/// fragment never swallows a neighbouring record's fact — and as the
/// grammar-upgrade tripwire: should a future grammar emit declaration nodes
/// for these spellings, this weld flips first and the bounded-gap notes in
/// the workplan csharp_declaration_bodies family must be re-verified.
#[test]
fn semicolon_bodied_non_record_declarations_are_grammar_bounded() {
    let fixture = common::Fixture::new();
    fixture.write("Bound/Bound.csproj", &csproj("net8.0"));
    fixture.write(
        "Bound/Bounded.cs",
        "namespace Bound.Shapes;\npublic sealed class SemicolonClass;\npublic sealed record RecordBetween1(int A);\npublic class CtorLikeClass(int B);\npublic interface IBodiless;\npublic record struct RecordBetween2(int C);\npublic struct CtorLikeStruct(int D);\npublic enum EBodiless;\npublic record RecordBetween3(int E);\n",
    );
    let facts = public_type_facts(&fixture);
    assert_eq!(
        facts["Bound::Shapes"],
        serde_json::json!(["RecordBetween1", "RecordBetween2", "RecordBetween3"]),
        "records land between grammar ERROR fragments; the `;`-bodied \
         class/interface/struct/enum spellings emit no fact (no declaration \
         node exists for them): {facts}"
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
