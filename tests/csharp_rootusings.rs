//! C# root `using` directives in multi-namespace files (workplan
//! archspec_syntax_backends, US 10 "root usings in multi-namespace files").
//!
//! A `using` placed ABOVE the first block-scoped `namespace` of a file lives
//! at compilation-unit scope: C# visibility makes it apply to EVERY
//! namespace in that file. The scanner attributes such a root-scope using to
//! EVERY namespace the file declares, so every `from` module of the
//! resulting edges is a real module in the soft tier (attributing to a
//! synthetic unit-root module instead would invent a module no file of that
//! shape belongs to and pollute the inspect model view — the decision
//! recorded in the US 10 `__log__`). A file with exactly one namespace folds
//! the using into that namespace; a namespace-less file keeps the unit-root
//! sentinel.
//!
//! Canonical acceptance row: `docs/archspec/commands/scan/acceptance.md`
//! row 55 (root usings above multiple block namespaces emit edges from each
//! namespace).

mod common;

use common::{stderr, stdout, Fixture};
use serde_json::Value;

fn csproj() -> String {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
     <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n"
        .to_string()
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

/// The Gherkin tree: ONE file declares TWO block-scoped namespaces with the
/// using directives placed ABOVE the first namespace. One project holds all
/// four namespaces (the shape the drop is orthogonal to).
fn multi_namespace_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    fixture.write(
        "App/Mixed.cs",
        "using App.Core;\n\
         using App.Shared.Registry;\n\
         \n\
         namespace App.Api\n\
         {\n\
             public class Api { }\n\
         }\n\
         \n\
         namespace App.Model\n\
         {\n\
             public class Model { }\n\
         }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    fixture.write(
        "App/Shared.cs",
        "namespace App.Shared;\npublic class Registry { }\n",
    );
    fixture
}

/// Gherkin clause 1: a root using above multiple block namespaces is visible
/// to EACH of them, so the scanner attributes it to each namespace the file
/// declares — one edge per (namespace, target) pair, addressed like every
/// other using (deepest declared prefix, tail segments as symbols).
#[test]
fn root_usings_above_block_namespaces_emit_edges_from_each_namespace() {
    let fixture = multi_namespace_fixture();
    assert_eq!(
        edges(&scan(&fixture)),
        vec![
            (
                "App".to_string(),
                "App::Api".to_string(),
                "App::Core".to_string(),
                Vec::new()
            ),
            (
                "App".to_string(),
                "App::Api".to_string(),
                "App::Shared".to_string(),
                vec!["Registry".to_string()]
            ),
            (
                "App".to_string(),
                "App::Model".to_string(),
                "App::Core".to_string(),
                Vec::new()
            ),
            (
                "App".to_string(),
                "App::Model".to_string(),
                "App::Shared".to_string(),
                vec!["Registry".to_string()]
            ),
        ],
        "the compilation-unit usings reach BOTH block namespaces; the pure \
         namespace using carries no symbols, the type-targeted one the tail"
    );
}

/// The namespaces of a multi-namespace file are declarations, not usings:
/// they join the soft tier regardless of what the root-scope directives do.
#[test]
fn block_namespaces_of_a_multi_namespace_file_join_the_soft_tier() {
    let fixture = multi_namespace_fixture();
    let model = scan(&fixture);
    assert_eq!(
        model["soft_structure"]["App"],
        serde_json::json!(["App::Api", "App::Core", "App::Model", "App::Shared"]),
        "every declared namespace of the file joins the soft tier"
    );
}

/// The other shape: root usings above exactly ONE block namespace fold INTO
/// that namespace — the using joins the namespace, addressed at the deepest
/// declared prefix with the tail segment as the symbol.
#[test]
fn single_namespace_root_usings_fold_into_that_namespace() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    fixture.write(
        "App/Single.cs",
        "using App.Core.Engine;\n\
         \n\
         namespace App.Api\n\
         {\n\
             public class Api { }\n\
         }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["Engine".to_string()]
        )],
        "one edge from the single namespace, tail as symbol"
    );
}

/// The distributed using joins the namespace's OWN visibility picture: a
/// bare type name in the SECOND block namespace resolves through the root
/// using (distribution happens before candidate generation), so the edge
/// from that namespace carries the type symbol — and the anchor still filters
/// an unrelated root-scope noise identifier into nothing.
#[test]
fn root_usings_are_visible_to_bare_names_in_later_namespaces() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    fixture.write(
        "App/Mixed.cs",
        "using App.Core;\n\
         \n\
         namespace App.Api\n\
         {\n\
             public class Api { }\n\
         }\n\
         \n\
         namespace App.Model\n\
         {\n\
             public class Model\n\
             {\n\
                 private Engine _engine;\n\
             }\n\
         }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    let model_edges = edges(&scan(&fixture));
    assert_eq!(
        model_edges,
        vec![
            (
                "App".to_string(),
                "App::Api".to_string(),
                "App::Core".to_string(),
                Vec::new()
            ),
            (
                "App".to_string(),
                "App::Model".to_string(),
                "App::Core".to_string(),
                vec!["Engine".to_string()]
            ),
        ],
        "the second namespace's bare Engine resolves through the root using"
    );
}
