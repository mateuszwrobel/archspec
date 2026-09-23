//! The C# injected-field tier (workplan archspec_syntax_backends,
//! US 09 "edges through injected fields").
//!
//! CLI-level behaviour of `scan` on dependency-injection
//! shaped C#: a class declares a field of a type from another namespace
//! (qualified, or bare through a namespace using) and calls through that
//! field. The edge comes from the FIELD TYPE POSITION (US 08 already collects
//! `field_declaration` / `parameter` type positions), addressed at the owner
//! of the deepest declared prefix and carrying the interface type as symbol.
//! The member-access chains through the field (`_engine.Run()`) reconstruct to
//! `_engine.Run` — a chain whose leading segment is NOT a declared namespace,
//! so the anchored-prefix rule contributes NOTHING: neither an edge nor a
//! symbol. No identifier-to-field link step is needed: fields are file-scoped
//! and their declared type is already the type-position fact, so the chain is
//! anchored noise by design. The edge map is a set keyed by (unit, from, to):
//! the edge is recorded ONCE regardless of how many calls traverse the field,
//! and constructor-parameter / field / using references to the same type
//! dedup into one edge with the interface as the only symbol — method names
//! never pollute the symbol set.
//!
//! Canonical acceptance row: `docs/archspec/commands/scan/acceptance.md`
//! row 54 (injected-field type positions produce one owner-addressed edge
//! with the interface symbol, call-count invariant, no method-name symbols).

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

/// The core of the Gherkin tree: `App.Api` (referencing `App.Core`) declares
/// `private readonly App.Core.IEngine _engine;` — NO using — and the `App.Core`
/// project declares `IEngine`. `calls` controls how many calls traverse the
/// field (the once-only clause).
fn di_fixture(calls: usize) -> Fixture {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&["App.Core"]));
    let mut body = String::new();
    for _ in 0..calls {
        body.push_str("        _engine.Run();\n");
    }
    fixture.write(
        "App.Api/Api.cs",
        &format!(
            "namespace App.Api;\n\
             public class Api\n\
             {{\n\
             \x20   private readonly App.Core.IEngine _engine;\n\
             \x20   public void Handle()\n\
             \x20   {{\n{body}\x20   }}\n\
             }}\n"
        ),
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Types.cs",
        "namespace App.Core;\npublic interface IEngine { void Run(); }\n",
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

/// Gherkin clause 1: the declared field type is the edge — `App::Api ->
/// App::Core` carrying `IEngine`, with NO using anywhere and no help from the
/// call sites. Clause 2 (once-only) is covered by the call-count tests below.
#[test]
fn injected_field_type_records_the_edge_with_the_interface_symbol() {
    let fixture = di_fixture(1);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["IEngine".to_string()]
        )],
        "the field type position alone yields the edge + interface symbol"
    );
}

/// Gherkin clause 2: the edge is recorded ONCE regardless of how many calls
/// traverse the field — one call and five calls give byte-identical module
/// edge sets, and repeated scans of the five-call tree are deterministic.
#[test]
fn calls_through_the_field_never_multiply_the_edge() {
    let one = di_fixture(1);
    let five = di_fixture(5);
    assert_eq!(
        edges(&scan(&one)),
        edges(&scan(&five)),
        "five calls through the field emit exactly what one call emits"
    );
    assert_eq!(
        edges(&scan(&five)).len(),
        1,
        "the five-call tree still carries ONE edge"
    );
    assert_eq!(
        scan_bytes(&five),
        scan_bytes(&five),
        "the injected-field scan is byte-identical across runs"
    );
}

/// The chains `_engine.Run()` are anchored noise: the reconstructed reference
/// is rooted at `_engine` (not a declared namespace), so it contributes no
/// edge of its own AND no method-name symbol — the symbol set is EXACTLY the
/// interface type, never [IEngine, Run].
#[test]
fn chains_through_the_field_add_no_method_name_symbols() {
    let fixture = di_fixture(3);
    let model = scan(&fixture);
    assert_eq!(
        edges(&model),
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["IEngine".to_string()]
        )],
        "symbols are exactly [IEngine]"
    );
    let rendered = serde_json::to_string(&model).expect("model");
    assert!(
        !rendered.contains("\"Run\""),
        "the method name never enters the model: {rendered}"
    );
}

/// The bare-field form: `using App.Core;` + `private readonly IEngine
/// _engine;` — the field type resolves through the file's visibility (the
/// declared-type anchor makes `App.Core.IEngine` the ONE real candidate;
/// `App.Api.IEngine` and `App.Core._engine` are not declared types). The
/// scanner adds the symbol to the edge the using already produces.
#[test]
fn bare_interface_via_using_carries_the_interface_symbol() {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&["App.Core"]));
    fixture.write(
        "App.Api/Api.cs",
        "using App.Core;\n\
         namespace App.Api;\n\
         public class Api\n\
         {\n\
         \x20   private readonly IEngine _engine;\n\
         \x20   public void Handle()\n\
         \x20   {\n\
         \x20       _engine.Run();\n\
         \x20       _engine.Run();\n\
         \x20   }\n\
         }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Types.cs",
        "namespace App.Core;\npublic interface IEngine { void Run(); }\n",
    );
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["IEngine".to_string()]
        )],
        "the using's edge gains the interface symbol from the field position"
    );
}

/// Constructor injection: the parameter type position and the assigned field
/// type position name the SAME type — the parameter resolves through the
/// file's using (`App.Core.IEngine` is the one declared-type-anchored
/// candidate). Field position + parameter position + using all dedup into ONE
/// edge with the interface as the only symbol.
#[test]
fn constructor_parameter_and_assigned_field_dedup_into_one_edge() {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&["App.Core"]));
    fixture.write(
        "App.Api/Api.cs",
        "using App.Core;\n\
         namespace App.Api;\n\
         public class Api\n\
         {\n\
         \x20   private readonly IEngine _engine;\n\
         \x20   public Api(IEngine e)\n\
         \x20   {\n\
         \x20       _engine = e;\n\
         \x20   }\n\
         \x20   public void Handle()\n\
         \x20   {\n\
         \x20       _engine.Run();\n\
         \x20       _engine.Stop();\n\
         \x20       _engine.Reset();\n\
         \x20   }\n\
         }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Types.cs",
        "namespace App.Core;\npublic interface IEngine { }\n",
    );
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["IEngine".to_string()]
        )],
        "field + parameter + using references share ONE edge, symbol [IEngine]"
    );
    let rendered = serde_json::to_string(&scan(&fixture)).expect("model");
    for method in ["Run", "Stop", "Reset"] {
        assert!(
            !rendered.contains(&format!("\"{method}\"")),
            "call-site method {method} never becomes a symbol: {rendered}"
        );
    }
}

/// The no-csproj fallback (one unit for the tree) takes the same
/// injected-field facts as the csproj shape.
#[test]
fn no_csproj_single_unit_shares_the_injected_field_behavior() {
    let fixture = Fixture::new();
    fixture.write(
        "Api.cs",
        "namespace App.Api;\n\
         public class Api\n\
         {\n\
         \x20   private readonly App.Core.IEngine _engine;\n\
         \x20   public void Handle()\n\
         \x20   {\n\
         \x20       _engine.Run();\n\
         \x20       _engine.Run();\n\
         \x20   }\n\
         }\n",
    );
    fixture.write(
        "Core.cs",
        "namespace App.Core;\npublic interface IEngine { }\n",
    );
    let model_edges = edges(&scan(&fixture));
    assert_eq!(model_edges.len(), 1);
    let (unit, from, to, symbols) = model_edges.first().expect("one edge").clone();
    assert!(!unit.is_empty(), "the unit is the tree's directory name");
    assert_eq!(
        (from.as_str(), to.as_str(), symbols.as_slice()),
        ("App::Api", "App::Core", ["IEngine".to_string()].as_slice())
    );
}
