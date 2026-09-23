//! The C# module tier (workplan archspec_syntax_backends,
//! US 06 "C# symbols from type-targeted usings").
//!
//! CLI-level behaviour of `scan` on C# trees: a
//! type-targeted using (`using App.Core.Engine;`, `Engine` a TYPE inside the
//! declared namespace `App.Core`) records the module edge addressed at the
//! deepest DECLARED namespace (`App::Core`) carrying the type tail as a
//! symbol. Attribution of units and from-modules — same-unit vs cross-unit
//! branches, the ambiguity and reachability guards, the external tier — is
//! shared code; the addressing rule decides the endpoint and symbols of every
//! resolved module edge.
//!
//! Canonical acceptance rows: `docs/archspec/commands/scan/acceptance.md`
//! row 47 (type-targeted using: endpoint = deepest declared namespace,
//! symbol = type tail) and row 48 (namespace-less, alias,
//! static and `global::` usings share one addressing rule; `var`
//! resources and lambdas add no edges).
//!
//! US 07 ("C# externals stay external") extends the file with the external
//! tier probes: a `using` whose target no declared namespace owns resolves
//! through the SAME dotted-prefix miss and is attributed
//! through the external tier (`module_external` + `external`), never as a
//! module edge — row 49.

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

/// csproj with project references AND NuGet `PackageReference` items (the
/// external-tier fixtures need referenced packages; the `csproj` helper above
/// stays untouched so the US 06 fixtures keep their exact bytes).
fn csproj_with(references: &[&str], packages: &[&str]) -> String {
    let mut csproj = String::from(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
         <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n",
    );
    if !references.is_empty() || !packages.is_empty() {
        csproj.push_str("  <ItemGroup>\n");
        for reference in references {
            csproj.push_str(&format!(
                "    <ProjectReference Include=\"..\\{reference}\\{reference}.csproj\" />\n"
            ));
        }
        for package in packages {
            csproj.push_str(&format!("    <PackageReference Include=\"{package}\" />\n"));
        }
        csproj.push_str("  </ItemGroup>\n");
    }
    csproj.push_str("</Project>\n");
    csproj
}

/// One `using App.Core.Engine;` from namespace `App.Api` into the declared
/// namespace `App.Core` (which owns the TYPE `Engine`) across a referenced
/// project — the Gherkin tree: importing unit `App.Api`, referenced unit
/// `App.Core`.
fn gherkin_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&["App.Core"]));
    fixture.write(
        "App.Api/Controller.cs",
        "using App.Core.Engine;\nnamespace App.Api;\npublic class Controller { }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Engine.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    fixture
}

/// One project `App` with a `using` whose target is the declared namespace
/// `App.Core` itself — a pure namespace using (no type tail).
fn namespace_using_fixture(target: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj(&[]));
    fixture.write(
        "App/Api.cs",
        &format!("using {target};\nnamespace App.Api;\npublic class Api {{ }}\n"),
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Registry { }\n",
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

/// Gherkin clause 1: `scan` addresses the edge at the
/// deepest declared namespace (`App::Core`, owned by unit `App.Core`) and
/// carries the type tail (`Engine`) as a symbol, owned by the importing unit.
#[test]
fn edge_addresses_declared_namespace_with_type_symbol() {
    let fixture = gherkin_fixture();
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["Engine".to_string()]
        )],
        "one module edge: unit = importing unit, to = owner namespace, symbols = type tail"
    );
}

/// A namespace-targeted `using App.Core;` carries no tail: the edge endpoint
/// is the target itself and the symbols stay empty.
#[test]
fn namespace_targeted_using_keeps_the_target_endpoint_without_symbols() {
    let fixture = namespace_using_fixture("App.Core");
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            Vec::new(),
        )],
        "edge to App::Core, symbols empty"
    );
}

/// Alias (`using Engine = App.Core.Engine;`), `using static` and
/// `global using` forms resolve through the same rule as plain usings: the
/// target (right-hand side) decides owner and symbols.
#[test]
fn alias_static_and_global_forms_resolve_through_the_same_rule() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj(&[]));
    fixture.write(
        "App/Api.cs",
        "using Engine = App.Core.Engine;\nusing static App.Core.Clock;\nglobal using App.Core.Vehicle;\nnamespace App.Api;\npublic class Api { }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Engine { }\npublic static class Clock { }\npublic class Vehicle { }\n",
    );
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec![
                "Clock".to_string(),
                "Engine".to_string(),
                "Vehicle".to_string()
            ]
        )],
        "one edge to App::Core carrying every referenced type name, sorted"
    );
}

/// Symbols dedup per edge across files/usings and serialize sorted; two
/// identical runs are byte-identical (determinism clause).
#[test]
fn symbols_dedup_per_edge_sorted_and_runs_are_deterministic() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj(&[]));
    fixture.write(
        "App/Api.cs",
        "using App.Core.Store;\nusing App.Core.Engine;\nnamespace App.Api;\npublic class Api { }\n",
    );
    fixture.write(
        "App/Home.cs",
        "using App.Core.Engine;\nnamespace App.Api;\npublic class Home { }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Engine { }\npublic class Store { }\n",
    );
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "App".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["Engine".to_string(), "Store".to_string()]
        )],
        "the repeated Engine collapses; symbols serialize sorted"
    );
    assert_eq!(
        scan_bytes(&fixture),
        scan_bytes(&fixture),
        "the scan is byte-identical across runs"
    );
}

/// Error tolerance: a file whose tail the grammar cannot parse (ERROR region)
/// still contributes its located usings and namespaces — no error exit.
#[test]
fn error_tolerant_tree_still_yields_edges() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj(&[]));
    fixture.write(
        "App/Api.cs",
        "using App.Core.Engine;\nnamespace App.Api;\npublic class Api\n{\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    let model = scan(&fixture);
    assert_eq!(
        edges(&model),
        vec![(
            "App".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["Engine".to_string()]
        )],
        "the damaged file's surviving using still produces the owner-addressed edge"
    );
}

/// The cross-unit guards are shared code: a using into a project
/// that is NOT reference-reachable resolves to no edge (a using that cannot
/// compile is no source fact).
#[test]
fn unreachable_cross_unit_using_emits_no_edge() {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&[]));
    fixture.write(
        "App.Api/Controller.cs",
        "using App.Core.Engine;\nnamespace App.Api;\npublic class Controller { }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Engine.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    assert!(
        edges(&scan(&fixture)).is_empty(),
        "the reachability guard drops the uncompilable using"
    );
}

/// The no-csproj fallback (one unit for the tree) takes the same
/// source facts and addressing: parallel behavior to the csproj shape.
#[test]
fn no_csproj_single_unit_shares_the_addressing() {
    let fixture = Fixture::new();
    fixture.write(
        "Controller.cs",
        "using App.Core.Engine;\nnamespace App.Api;\npublic class Controller { }\n",
    );
    fixture.write(
        "Engine.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    let model_edges = edges(&scan(&fixture));
    let (unit, from, to, symbols) = model_edges
        .first()
        .expect("the single edge must be recorded")
        .clone();
    assert_eq!(model_edges.len(), 1);
    assert!(!unit.is_empty(), "the unit is the tree's directory name");
    assert_eq!(
        (from.as_str(), to.as_str(), symbols.as_slice()),
        ("App::Api", "App::Core", ["Engine".to_string()].as_slice())
    );
}

// ---- US 07: C# externals stay external ------------------------------------

/// The package ids `module_external` records for one module key (empty when
/// the key is absent — an absent key and an empty list are the same fact:
/// this module attributes nothing external).
fn attributed_packages(model: &Value, module: &str) -> Vec<String> {
    model["module_external"][module]
        .as_array()
        .map(|crates| {
            crates
                .iter()
                .map(|c| c.as_str().expect("package id").to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// The Gherkin tree of the workplan scenario: two projects whose only
/// cross-namespace usings target the BCL (`System.Text`) and a referenced
/// NuGet package (`Microsoft.Extensions.Logging`); the referenced project's
/// declared namespace is used by nobody.
fn externals_only_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "App.Api/App.Api.csproj",
        &csproj_with(&["App.Core"], &["Microsoft.Extensions.Logging"]),
    );
    fixture.write(
        "App.Api/Controller.cs",
        "using System.Text;\nusing Microsoft.Extensions.Logging;\nnamespace App.Api;\npublic class Controller { }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj_with(&[], &[]));
    fixture.write(
        "App.Core/Engine.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    fixture
}

/// Gherkin clauses 1 + 2 (row 49): cross-namespace usings that target only
/// BCL and NuGet namespaces yield NO module edges — the
/// declared-namespace ownership miss is the dotted-prefix rule — and the
/// external tier carries the usage instead.
#[test]
fn externals_only_tree_records_no_module_edges_and_attributes_externals() {
    let fixture = externals_only_fixture();
    let model = scan(&fixture);
    assert!(
        edges(&model).is_empty(),
        "no declared namespace owns System.Text or Microsoft.Extensions.Logging, so no module edge"
    );
    assert_eq!(
        attributed_packages(&model, "App::Api"),
        ["Microsoft.Extensions.Logging".to_string()],
        "the NuGet using is the dependency fact; the unreferenced BCL root System.* attributes nothing (the scenario-6 rule)"
    );
    let external: Vec<&str> = model["external"]
        .as_array()
        .expect("external")
        .iter()
        .map(|n| n.as_str().expect("package name"))
        .collect();
    assert_eq!(
        external,
        ["Microsoft.Extensions.Logging"],
        "the package is the csproj-referenced external, not a phantom module"
    );
}

/// Alias usings whose RIGHT-HAND side is external — the BCL shape
/// (`using Foo = System.Text.StringBuilder;`) and the NuGet shape
/// (`using LogLevel = Microsoft.Extensions.Logging.LogLevel;`) — are no
/// different from plain usings: the alias name is dropped, the
/// target flows through the SAME dotted resolver, attributes through the
/// external tier, and records no module edge.
#[test]
fn alias_targets_with_external_right_hand_side_attribute_and_edge_nothing() {
    let fixture = Fixture::new();
    fixture.write(
        "App/App.csproj",
        &csproj_with(&[], &["Microsoft.Extensions.Logging", "System.Text.Json"]),
    );
    fixture.write(
        "App/Api.cs",
        "using Foo = System.Text.StringBuilder;\nusing LogLevel = Microsoft.Extensions.Logging.LogLevel;\nnamespace App.Api;\npublic class Api { }\n",
    );
    let model = scan(&fixture);
    assert!(
        edges(&model).is_empty(),
        "an alias to an external type is no module edge"
    );
    // The BCL alias hits `System.Text.Json` through the documented
    // two-shared-segment prefix approximation; the NuGet alias matches its
    // package id exactly. Same rule, same result.
    assert_eq!(
        attributed_packages(&model, "App::Api"),
        [
            "Microsoft.Extensions.Logging".to_string(),
            "System.Text.Json".to_string()
        ],
        "both alias targets attribute their packages"
    );
}

/// `global::`-anchored targets aimed at external namespaces (plain and alias
/// forms): the anchor is dropped and the normalized target
/// flows through the SAME external rule — external tier, never a module
/// edge; nothing invents an edge for a target no declared namespace owns.
#[test]
fn global_anchored_external_targets_never_become_module_edges() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj_with(&[], &["System.Text.Json"]));
    fixture.write(
        "App/Api.cs",
        "using Bar = global::System.Text.StringBuilder;\nusing global::System.Text.Encoding;\nnamespace App.Api;\npublic class Api { }\n",
    );
    let model = scan(&fixture);
    assert!(
        edges(&model).is_empty(),
        "external stays external — the anchor is dropped, not turned into a module path"
    );
    assert_eq!(
        attributed_packages(&model, "App::Api"),
        ["System.Text.Json".to_string()],
        "both anchors' targets attribute the package through the shared prefix rule"
    );
}

/// Regression guard: a file mixing a using into a DECLARED namespace (which
/// takes the edge-targeting addressing path) with usings into
/// external namespaces. The edge path must not disturb the external tier:
/// `module_external` and `external` stay exactly the documented facts while
/// the resolved edge keeps its documented addressing and the
/// external targets contribute no edge.
#[test]
fn external_attribution_is_unaffected_by_the_edge_path() {
    let fixture = Fixture::new();
    fixture.write(
        "App.Api/App.Api.csproj",
        &csproj_with(&["App.Core"], &["Microsoft.Extensions.Logging"]),
    );
    fixture.write(
        "App.Api/Controller.cs",
        "using App.Core.Engine;\nusing Microsoft.Extensions.Logging;\nusing System.Text;\nnamespace App.Api;\npublic class Controller { }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj_with(&[], &[]));
    fixture.write(
        "App.Core/Engine.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    let model = scan(&fixture);
    assert_eq!(
        edges(&model),
        vec![(
            "App.Api".to_string(),
            "App::Api".to_string(),
            "App::Core".to_string(),
            vec!["Engine".to_string()]
        )],
        "exactly ONE module edge — the declared-namespace using; the external usings contribute none"
    );
    let resolved = edges(&model);
    let to_modules: Vec<&str> = resolved.iter().map(|edge| edge.2.as_str()).collect();
    assert!(
        !to_modules
            .iter()
            .any(|to| to.starts_with("Microsoft::") || to.starts_with("System::")),
        "no external namespace is ever addressed as a module endpoint: {to_modules:?}"
    );
    assert_eq!(
        attributed_packages(&model, "App::Api"),
        ["Microsoft.Extensions.Logging".to_string()],
        "external attribution identical with the edge path present"
    );
    assert_eq!(
        model["external"]
            .as_array()
            .expect("external")
            .iter()
            .map(|n| n.as_str().expect("package name"))
            .collect::<Vec<_>>(),
        ["Microsoft.Extensions.Logging"],
        "the external tier is the csproj vocabulary"
    );
}
