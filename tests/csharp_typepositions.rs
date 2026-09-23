//! The C# type-position tier (workplan archspec_syntax_backends,
//! US 08 "module edges from type positions").
//!
//! CLI-level behaviour of `scan` on C# trees: every type
//! mentioned in a TYPE POSITION — base list, `new`, cast, attribute, method
//! parameter/return, generic argument, field/property type, dotted chain
//! anchored at a declared namespace — is a source fact: it records the module
//! edge to the owner of the deepest DECLARED prefix of the reference and
//! carries the tail segments as symbols, exactly like the US 06 using path.
//! Bare identifiers additionally expand into the file's visibility candidates
//! (each `using`d namespace + the enclosing namespace, dotted with the
//! identifier); a candidate survives iff the identifier is a TYPE that the
//! candidate's declared prefix actually declares (case-sensitive), so local
//! variables and instance chains produce nothing. The external tier
//! stays using-driven: a type-position reference to an
//! undeclared (BCL/NuGet) name records NOTHING, never a module_external
//! entry.
//!
//! Canonical acceptance rows: `docs/archspec/commands/scan/acceptance.md`
//! row 52 (type positions produce owner-addressed edges with type symbols)
//! and row 53 (bare names resolve
//! through the file's visibility with a declared-type anchor; locals,
//! instance chains and string content stay silent; the external tier stays
//! using-driven).

mod common;

use common::{stderr, stdout, Fixture};
use serde_json::Value;

fn csproj(references: &[&str]) -> String {
    let mut csproj = String::from(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
         <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n",
    );
    if !references.is_empty() {
        csproj.push_str("  <ItemGroup>\n");
        for reference in references {
            csproj.push_str(&format!(
                "    <ProjectReference Include=\"..\\{reference}\\{reference}.csproj\" />\n"
            ));
        }
        csproj.push_str("  </ItemGroup>\n");
    }
    csproj.push_str("</Project>\n");
    csproj
}

/// The Gherkin tree of US 08: `App.Api` (referencing `App.Core`) names
/// `App.Core` types in every implemented position WITHOUT any using directive
/// — base list, `new`, cast, attribute, parameter, return, generic arguments,
/// field type. `App.Core` declares each of them.
fn positions_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&["App.Core"]));
    fixture.write(
        "App.Api/Controller.cs",
        "namespace App.Api;\n\
         [App.Core.Attr]\n\
         public class Controller : App.Core.IBase\n\
         {\n\
         \x20   private App.Core.Engine _engine = new App.Core.Engine();\n\
         \x20   public App.Core.Widget Make(App.Core.Part part) => (App.Core.Widget)part;\n\
         \x20   public App.Core.Wrapper<App.Core.Inner> Wrap() => new App.Core.Wrapper<App.Core.Inner>();\n\
         }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Types.cs",
        "namespace App.Core;\n\
         public interface IBase { }\n\
         public class Engine { }\n\
         public class Widget { }\n\
         public class Part { }\n\
         public class Attr { }\n\
         public class Inner { }\n\
         public class Wrapper<T> { }\n",
    );
    fixture
}

/// The module edges of a model as comparable tuples.
fn edges(model: &Value) -> Vec<(String, String, String, Vec<String>)> {
    model["module_edges"]
        .as_array()
        .expect("module_edges must be an array")
        .iter()
        .map(|edge| {
            (
                edge["unit"].as_str().expect("unit").to_string(),
                edge["from"].as_str().expect("from").to_string(),
                edge["to"].as_str().expect("to").to_string(),
                edge["symbols"]
                    .as_array()
                    .expect("symbols")
                    .iter()
                    .map(|s| s.as_str().expect("symbol").to_string())
                    .collect(),
            )
        })
        .collect()
}

fn scan(fixture: &Fixture) -> Value {
    let output = fixture.run(&["scan"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`scan` must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "stderr must be empty");
    serde_json::from_str(&stdout(&output)).expect("stdout must be JSON")
}

fn scan_bytes(fixture: &Fixture) -> Vec<u8> {
    let output = fixture.run(&["scan"]);
    assert_eq!(output.status.code(), Some(0), "scan must exit 0");
    output.stdout
}

/// Gherkin clause 1: every type position on the same tree produces ONE edge to
/// the owner of the deepest declared prefix (`App::Core`) carrying every
/// referenced type name as a symbol, owned by the importing unit.
#[test]
fn type_positions_record_owner_addressed_edges_with_type_symbols() {
    let fixture = positions_fixture();
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec![
                "Attr".to_string(),
                "Engine".to_string(),
                "IBase".to_string(),
                "Inner".to_string(),
                "Part".to_string(),
                "Widget".to_string(),
                "Wrapper".to_string(),
            ]
        )],
        "one edge per (from, owner): symbols = referenced type names, sorted"
    );
}

/// Every implemented position contributes on one unit in one project: field
/// type, property type, method return and parameter, local declaration type,
/// cast, `default(..)`, `typeof(..)`, `is`, `as`, `new` and generic arguments
/// — all dedup into one edge with the exact symbol set.
#[test]
fn every_position_kind_contributes_to_the_edge() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj(&[]));
    fixture.write(
        "App/Api.cs",
        "namespace App.Api;\n\
         public class Api\n\
         {\n\
         \x20   private App.Core.Store _store;\n\
         \x20   public App.Core.Engine Field { get; set; }\n\
         \x20   public App.Core.Widget Do(App.Core.Part part)\n\
         \x20   {\n\
         \x20       App.Core.Local local = default(App.Core.Local);\n\
         \x20       object o = (App.Core.Widget)part;\n\
         \x20       var flag = o is App.Core.Part;\n\
         \x20       var store = o as App.Core.Store;\n\
         \x20       var t = typeof(App.Core.Widget);\n\
         \x20       var gen = new App.Core.Wrapper<App.Core.Inner>();\n\
         \x20       return (App.Core.Widget)part;\n\
         \x20   }\n\
         }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\n\
         public class Store { }\n\
         public class Engine { }\n\
         public class Widget { }\n\
         public class Part { }\n\
         public class Local { }\n\
         public class Inner { }\n\
         public class Wrapper<T> { }\n",
    );
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec![
                "Engine".to_string(),
                "Inner".to_string(),
                "Local".to_string(),
                "Part".to_string(),
                "Store".to_string(),
                "Widget".to_string(),
                "Wrapper".to_string(),
            ]
        )],
        "every position feeds the SAME deduped edge; symbols sorted"
    );
    assert_eq!(
        scan_bytes(&fixture),
        scan_bytes(&fixture),
        "the type-position scan is byte-identical across runs"
    );
}

/// Bare identifiers resolve through exactly THIS file's visibility: a plain
/// namespace using (`using App.Core;`) plus the enclosing namespace generate
/// the candidates, and only the candidate whose prefix DECLARES the
/// identifier as a TYPE survives. Cross-unit, the using already addresses the
/// owner and the type symbols ride that edge.
#[test]
fn bare_names_resolve_through_the_files_usings() {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&["App.Core"]));
    fixture.write(
        "App.Api/Controller.cs",
        "using App.Core;\n\
         namespace App.Api;\n\
         public class Controller\n\
         {\n\
         \x20   private Engine _engine = new Engine();\n\
         \x20   public Widget Make(Part part) => (Widget)part;\n\
         }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Types.cs",
        "namespace App.Core;\n\
         public class Engine { }\n\
         public class Widget { }\n\
         public class Part { }\n",
    );
    let model_edges = edges(&scan(&fixture));
    assert_eq!(
        model_edges,
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec![
                "Engine".to_string(),
                "Part".to_string(),
                "Widget".to_string(),
            ]
        )],
        "bare types resolve to the using'd namespace with type symbols"
    );
}

/// A fully-qualified reference and a bare reference to the same type (and a
/// type-targeted using of it) collapse into ONE edge with each symbol once.
#[test]
fn qualified_and_bare_and_using_references_dedup_into_one_edge() {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&["App.Core"]));
    fixture.write(
        "App.Api/Controller.cs",
        "using App.Core;\n\
         using App.Core.Store;\n\
         namespace App.Api;\n\
         public class Controller\n\
         {\n\
         \x20   private App.Core.Engine _a = new Engine();\n\
         \x20   private Store _b = new App.Core.Store();\n\
         }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Types.cs",
        "namespace App.Core;\npublic class Engine { }\npublic class Store { }\n",
    );
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["Engine".to_string(), "Store".to_string()]
        )],
        "qualified + bare + the type-targeted using share one edge, symbols deduped"
    );
}

/// The noise guard at the CLI level: local variables, instance-method chains,
/// static calls through undeclared types and using-directive text inside a
/// STRING produce no module edges; `var` alone adds no
/// type reference. The external tier stays using-driven —
/// type positions never add module_external entries (the BCL types here are
/// used without a using, and the framework using attributes nothing).
#[test]
fn locals_and_instance_chains_and_string_content_stay_silent() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj(&[]));
    fixture.write(
        "App/Api.cs",
        "using System;\n\
         namespace App.Api;\n\
         public class Api\n\
         {\n\
         \x20   public void Run()\n\
         \x20   {\n\
         \x20       var timer = new Timer();\n\
         \x20       timer.Tick();\n\
         \x20       Timer.Wake();\n\
         \x20       Console.WriteLine(\"using App.Core.Engine;\");\n\
         \x20   }\n\
         }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Registry { }\n",
    );
    let model = scan(&fixture);
    assert!(edges(&model).is_empty(), "no module edges");
    let rendered_external =
        serde_json::to_string(&model["module_external"]).expect("external");
    assert!(
        !rendered_external.contains("System"),
        "the using-driven external tier records no BCL fact from type positions \
         or framework usings: {rendered_external}"
    );
    let rendered = serde_json::to_string(&model).expect("model");
    assert!(
        !rendered.contains("Registry"),
        "the unreferenced App.Core type appears nowhere: {rendered}"
    );
}

/// The no-csproj fallback (one unit for the tree) takes the same
/// type-position facts: parallel behavior to the csproj shape.
#[test]
fn no_csproj_single_unit_shares_the_type_position_behavior() {
    let fixture = Fixture::new();
    fixture.write(
        "Controller.cs",
        "namespace App.Api;\npublic class Controller : App.Core.IBase { }\n",
    );
    fixture.write("Engine.cs", "namespace App.Core;\npublic class IBase { }\n");
    let model_edges = edges(&scan(&fixture));
    let (unit, from, to, symbols) = model_edges
        .first()
        .expect("the single edge must be recorded")
        .clone();
    assert_eq!(model_edges.len(), 1);
    assert!(!unit.is_empty(), "the unit is the tree's directory name");
    assert_eq!(
        (from.as_str(), to.as_str(), symbols.as_slice()),
        ("App::Api", "App::Core", ["IBase".to_string()].as_slice())
    );
}
