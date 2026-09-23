//! `depgraph api-usage` symbol grouping and empty-tree reasons (workplan
//! archspec_syntax_backends, US 15 "depgraph api-usage shows symbols").
//!
//! The api-usage renderer is data-driven: it reads `symbols` off the
//! `module_edges` of the scanned model — `commands::depgraph` runs
//! `scan::extract`, and `depgraph::api_usage_by_target`
//! groups those symbols per (target top-level module, using top-level
//! module) with no language conditional (backticked, comma-separated,
//! sorted — the rust shape pinned by `api_usage_locks_symbol_rows`). These
//! tests prove the data-driven path holds per language: on trees whose
//! model carries symbols (C# type-targeted usings + type positions,
//! Go exported selectors), api-usage prints exactly the grouping the
//! scan model contains.
//!
//! Two projections of api-usage are pinned honestly rather than pretending
//! otherwise: grouping runs at the granularity the modules view renders — the
//! trivial-fold rule shared with `depgraph modules` (D1), so a single-module
//! Go tree's sibling-package edge with symbols lands in the package-level
//! table — and an empty grouping carries the reason sentence of the model's
//! facts, never the bare statement, whenever the model has symbol facts on
//! its module edges (D4: c# states that no symbol facts were emitted for this
//! tree — `depgraph::NO_SYMBOL_FACTS_REASON`; rust states its placement
//! sentence; a tree whose symbol-carrying edges still fold onto one rendered
//! node states the fold placement; the bare sentence is legal only when no
//! module edge carries a symbol).
//!
//! Canonical acceptance rows: `docs/archspec/commands/depgraph/acceptance.md`
//! row 27 (api-usage prints the symbols carried by the model, grouped like
//! the rust shape) and row 28 (the empty statements on symbolless trees
//! state the documented reasons).

mod common;
#[allow(dead_code)]
mod shared;

use common::{stderr, stdout, Fixture};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The role-less note line every api-usage body trails with (roles plan
/// US 06): the view states its designed silence, welded to the matrix cell.
const ROLE_LESS_NOTE: &str =
    "note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'";

/// The exact body of `depgraph api-usage` on a c# tree whose
/// module edges carry no symbols: the empty statement plus the reason that
/// no symbol facts were emitted for this tree — the c# arm of
/// `depgraph::api_usage_empty_reason`.
const CSHARP_EMPTY_BODY: &str = concat!(
    "No internal API usage details found. No symbol facts were emitted for this tree.\n",
    "note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'\n"
);

/// The bare empty statement (D4): legal only when the model's module edges
/// carry no symbol facts at all — every fixture below that has symbols must
/// print a table or a reasoned sentence instead.
const BARE_EMPTY_BODY: &str = concat!(
    "No internal API usage details found.\n",
    "note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'\n"
);

/// The exact body on a model whose module edges DO carry symbols but whose
/// grouping stays empty even at the granularity the modules view renders
/// (every symbol-carrying edge lands on one node): the fold-placement arm of
/// `depgraph::api_usage_empty_reason`.
const FOLD_PLACEMENT_EMPTY_BODY: &str = concat!(
    "No internal API usage details found. Module edges carry symbols, but every \
     symbol-carrying edge stays inside one rendered module; the usage sits between \
     modules below the granularity this view renders.\n",
    "note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'\n"
);

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

/// The Gherkin tree: a c# solution whose model carries edges
/// with symbols. `App.Api` uses the type `App.Core.Engine` (type-targeted
/// using, US 06) and names `App.Core.Widget` in a type position (US 08);
/// `App.Worker` uses `App.Core.Registry`. The edges carry
/// {Engine, Widget} and {Registry}.
fn csharp_symbols_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("App.Api/App.Api.csproj", &csproj(&["App.Core"]));
    fixture.write(
        "App.Api/Controller.cs",
        "using App.Core.Engine;\n\
         namespace App.Api;\n\
         public class Controller\n\
         {\n\
         \x20   private App.Core.Widget _widget = new App.Core.Widget();\n\
         }\n",
    );
    fixture.write("App.Worker/App.Worker.csproj", &csproj(&["App.Core"]));
    fixture.write(
        "App.Worker/Worker.cs",
        "using App.Core.Registry;\nnamespace App.Worker;\npublic class Worker { }\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Types.cs",
        "namespace App.Core;\n\
         public class Engine { }\n\
         public class Widget { }\n\
         public class Registry { }\n",
    );
    fixture
}

/// A c# tree whose model carries NO symbol facts at all: only BCL
/// usings and instance chains (locals_and_instance_chains shape), so no
/// module edges are recorded while namespaces still give the module tier.
fn csharp_no_symbols_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj(&[]));
    fixture.write(
        "App/Api.cs",
        "using System;\n\
         namespace App.Api;\n\
         public class Api\n\
         {\n\
         \x20   public void Run() => Console.WriteLine(\"hello\");\n\
         }\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Registry { }\n",
    );
    fixture
}

/// A go.work workspace whose member `api` imports member `store` and calls
/// the exported selector `store.Title()` (plus the unexported `helper()`,
/// excluded by the casing rule): the cross-member edge carries {Title}.
fn go_workspace_symbols_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write(
        "api/api.go",
        "package api\n\nimport \"example.com/store\"\n\nfunc Api() {\n\tstore.Title()\n\tstore.helper()\n}\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write(
        "store/store.go",
        "package store\n\nfunc Title() {}\n\nfunc helper() {}\n",
    );
    fixture
}

/// The single-module Gherkin tree of the Go selector scenario: package
/// `store` imports sibling `format` and calls `format.Title()`; the package
/// edge carries {Title}.
fn go_single_module_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "store/store.go",
        "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n\tformat.Title()\n}\n",
    );
    fixture.write("format/format.go", "package format\n\nfunc Title() {}\n");
    fixture
}

/// A rust crate where `billing` imports `auth::Token` — the shape behind
/// `api_usage_locks_symbol_rows`.
fn rust_symbols_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");
    fixture
}

/// `depgraph api-usage [path]` on the fixture, asserting exit
/// 0 and returning stdout.
fn api_usage(fixture: &Fixture) -> String {
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`depgraph api-usage` must exit 0 (stderr: {})",
        stderr(&output)
    );
    stdout(&output)
}

/// The model of `scan` on the fixture (exit 0 asserted).
fn scan(fixture: &Fixture) -> Value {
    let output = fixture.run(&["scan"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "scan must exit 0 (stderr: {})",
        stderr(&output)
    );
    serde_json::from_str(&stdout(&output)).expect("stdout must be JSON")
}

/// The top-level projection `depgraph` applies to a dotted module path: the
/// segment right after the unit; a unit-root path folds to `root` (the
/// `depgraph::top_node` rule, rebuilt here from the model JSON only).
fn top_node(dotted: &str) -> String {
    match dotted.split("::").nth(1) {
        Some(top) => top.to_string(),
        None => "root".to_string(),
    }
}

/// The one-level-deeper projection (`depgraph::deep_node`): the path's own
/// final segment — what sits below a trivially folded projection; a
/// unit-root path folds to `root` exactly as in the top view.
fn deep_node(dotted: &str) -> String {
    match dotted.split("::").last().filter(|_| dotted.contains("::")) {
        Some(deep) => deep.to_string(),
        None => "root".to_string(),
    }
}

/// The granularity api-usage groups at: `top_node`, or `deep_node` when the
/// top-level projection collapses the module tier onto one node while the
/// tier holds more distinct paths below it — the trivial-fold rule
/// `modules_graph` applies, rebuilt from model data only (unit-edge
/// projection can only ever add nodes already derived from soft paths, so
/// these node sets match `project`'s).
fn grouping_node(model: &Value) -> fn(&str) -> String {
    let mut top_nodes: BTreeSet<String> = BTreeSet::new();
    let mut deep_nodes: BTreeSet<String> = BTreeSet::new();
    for paths in model["soft_structure"]
        .as_object()
        .expect("soft_structure")
        .values()
    {
        for path in paths.as_array().expect("paths") {
            let path = path.as_str().expect("path");
            top_nodes.insert(top_node(path));
            deep_nodes.insert(deep_node(path));
        }
    }
    for edge in model["module_edges"].as_array().expect("module_edges") {
        for endpoint in ["from", "to"] {
            let path = edge[endpoint].as_str().expect(endpoint);
            top_nodes.insert(top_node(path));
            deep_nodes.insert(deep_node(path));
        }
    }
    if top_nodes.len() == 1 && deep_nodes.len() > top_nodes.len() {
        return deep_node;
    }
    top_node
}

/// The api-usage grouping the selected model implies, rebuilt from the scan
/// JSON: symbols per (target node, using node) at the granularity the views
/// render, edges inside one node and symbolless pairs contributing nothing —
/// the very rule set `api_usage_by_target` applies, expressed only through
/// model data.
fn grouped_from_model(model: &Value) -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let node = grouping_node(model);
    let mut grouped: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for edge in model["module_edges"].as_array().expect("module_edges") {
        let from = node(edge["from"].as_str().expect("from"));
        let to = node(edge["to"].as_str().expect("to"));
        if from == to {
            continue;
        }
        for symbol in edge["symbols"].as_array().expect("symbols") {
            grouped
                .entry(to.clone())
                .or_default()
                .entry(from.clone())
                .or_default()
                .insert(symbol.as_str().expect("symbol").to_string());
        }
    }
    grouped
}

/// The Markdown table the grouping above renders (the documented column
/// shape: backticked sorted symbols, comma-separated).
fn render_expected(grouped: &BTreeMap<String, BTreeMap<String, BTreeSet<String>>>) -> String {
    let mut out =
        String::from("| Target module | Used by module | APIs used |\n| --- | --- | --- |\n");
    for (target, froms) in grouped {
        for (from, symbols) in froms {
            let api_list = symbols
                .iter()
                .map(|symbol| format!("`{symbol}`"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("| `{target}` | `{from}` | {api_list} |\n"));
        }
    }
    out.push_str(ROLE_LESS_NOTE);
    out.push('\n');
    out
}

/// Api-usage on the c# tree prints exactly the grouping the scan
/// model contains (data-flow pin: extraction ignored inside the
/// api-usage rendering would render a table this comparison rejects), and
/// the rows carry the symbols grouped per (target, source) in the rust shape
/// — backticked, comma-separated, sorted.
#[test]
fn csharp_api_usage_prints_model_grouped_symbols_per_pair() {
    let fixture = csharp_symbols_fixture();
    let expected = render_expected(&grouped_from_model(&scan(&fixture)));
    let out = api_usage(&fixture);
    assert_eq!(
        out, expected,
        "the api-usage table must equal the model's grouping"
    );
    assert_eq!(
        out,
        "| Target module | Used by module | APIs used |\n\
         | --- | --- | --- |\n\
         | `Core` | `Api` | `Engine`, `Widget` |\n\
         | `Core` | `Worker` | `Registry` |\n\
         note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'\n",
        "grouped per (target, source) exactly like the rust shape"
    );
}

/// Api-usage on the go workspace prints the exported selector symbol
/// on the cross-member (package) edge; the unexported selector stays out.
#[test]
fn go_workspace_api_usage_prints_selector_symbol() {
    let fixture = go_workspace_symbols_fixture();
    assert_eq!(
        api_usage(&fixture),
        "| Target module | Used by module | APIs used |\n\
         | --- | --- | --- |\n\
         | `store` | `api` | `Title` |\n\
         note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'\n",
        "the selector symbol rides the member edge exactly like rust symbols"
    );
}

/// The single-module Go tree renders (D4 reads D1's projection): the package
/// edge with {Title} has both endpoints folding to one top-level node, so
/// the modules view already renders one node per package — api-usage groups
/// at that same granularity and the symbol lands in a package-level row.
/// The bare statement is gone on this shape (it used to print with the
/// symbols sitting in the model unnoticed).
#[test]
fn go_single_module_api_usage_renders_folded_package_rows() {
    let fixture = go_single_module_fixture();
    let expected = render_expected(&grouped_from_model(&scan(&fixture)));
    let out = api_usage(&fixture);
    assert_eq!(out, expected, "the table must equal the model's grouping");
    assert_eq!(
        out,
        "| Target module | Used by module | APIs used |\n\
         | --- | --- | --- |\n\
         | `format` | `store` | `Title` |\n\
         note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'\n",
        "the folded edge's symbol renders at package granularity"
    );
    assert_ne!(out, BARE_EMPTY_BODY, "symbols exist; the bare statement is illegal");
}

/// Two go modules addressed under one shared prefix whose packages share a
/// final segment: the fold finds nothing distinct below the top node, every
/// symbol-carrying edge stays inside the one rendered module, and the empty
/// statement carries the fold-placement reason (D4) — never the bare
/// sentence, because the model has symbol facts.
fn go_same_final_segment_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "go.work",
        "go 1.21\n\nuse (\n\t./alpha\n\t./beta\n)\n",
    );
    fixture.write("alpha/go.mod", "module example.com/tools/alpha\ngo 1.21\n");
    fixture.write(
        "alpha/util/util.go",
        "package util\n\nimport \"example.com/tools/beta/util\"\n\nfunc Use() {\n\tutil.Title()\n}\n",
    );
    fixture.write("beta/go.mod", "module example.com/tools/beta\ngo 1.21\n");
    fixture.write("beta/util/util.go", "package util\n\nfunc Title() {}\n");
    fixture
}

#[test]
fn symbols_inside_one_rendered_node_carry_the_fold_placement_reason() {
    let fixture = go_same_final_segment_fixture();
    let out = api_usage(&fixture);
    assert_eq!(
        out, FOLD_PLACEMENT_EMPTY_BODY,
        "the empty statement must state the fold placement verbatim"
    );
    assert_ne!(out, BARE_EMPTY_BODY, "symbols exist; the bare statement is illegal");
}

/// The cross-language D4 invariant: no tree whose model carries symbol facts
/// on module edges prints the bare empty statement — the output is a table
/// or a sentence that says why the grouping is empty. Runs over every
/// symbol-bearing fixture shape (rust, csharp, go single-module, go
/// workspace, folded go) against the `scan` model's own facts.
#[test]
fn symbol_bearing_models_never_print_the_bare_empty_statement() {
    let cases: Vec<(&str, Fixture)> = vec![
        ("rust", rust_symbols_fixture()),
        ("csharp", csharp_symbols_fixture()),
        ("go single-module", go_single_module_fixture()),
        ("go workspace", go_workspace_symbols_fixture()),
        ("go folded", go_same_final_segment_fixture()),
    ];
    for (name, fixture) in cases {
        let model = scan(&fixture);
        let has_symbols = model["module_edges"]
            .as_array()
            .expect("module_edges")
            .iter()
            .any(|edge| !edge["symbols"].as_array().expect("symbols").is_empty());
        let out = api_usage(&fixture);
        assert!(
            !has_symbols || out != BARE_EMPTY_BODY,
            "{name}: model carries symbol facts on module edges, yet api-usage \
             printed the bare statement:\n{out}"
        );
    }
}

/// A go module whose single package imports nothing — the model carries
/// no module tier, so depgraph refuses; the refusal names the missing model
/// fact and the tier rule (package references).
fn go_single_package_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("store/store.go", "package store\n\nfunc Save() {}\n");
    fixture
}

/// `depgraph modules [path]` returning (stdout, stderr).
fn depgraph_modules(fixture: &Fixture) -> (String, String) {
    let output = fixture.run(&["depgraph", "modules"]);
    (stdout(&output), stderr(&output))
}

/// The multi-package single-module Go tree RENDERS (the model
/// carries the package edges, so the module-tier guard clears — the refusal
/// is a model fact, not a language verdict).
#[test]
fn go_module_tier_renders_from_package_references() {
    let fixture = go_single_module_fixture();
    let (out, err) = depgraph_modules(&fixture);
    assert!(
        out.contains("graph TD"),
        "the module graph renders, no refusal: {err}"
    );
}

/// A Go tree whose model carries no module facts refuses, and the refusal
/// names the missing model fact and the tier rule (package references)
/// rather than any particular tree shape.
#[test]
fn go_tier_less_refusal_names_the_missing_fact() {
    let fixture = go_single_package_fixture();
    let (out, err) = depgraph_modules(&fixture);
    assert!(out.is_empty(), "refusal keeps stdout empty");
    assert!(
        err.contains(
            "depgraph needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of"
        ),
        "the refusal must name the missing fact and the tier rule: {err}"
    );
}

/// A c# tree with no symbol facts at all (external-only c#): the empty
/// body states that no symbol facts were emitted for this tree — not that
/// the c# driver never emits them.
#[test]
fn csharp_api_usage_without_symbol_facts_states_the_tree_not_a_driver_rule() {
    let fixture = csharp_no_symbols_fixture();
    let out = api_usage(&fixture);
    assert_eq!(
        out, CSHARP_EMPTY_BODY,
        "the empty reason must name this tree's facts, verbatim"
    );
    assert!(
        !out.contains("driver"),
        "the body must not state a driver-wide absence: {out}"
    );
}

/// Rust api-usage symbol rows come straight off the model's
/// symbols, extending the existing `api_usage_locks_symbol_rows` pin.
#[test]
fn rust_api_usage_symbol_rows_come_from_the_model() {
    let fixture = rust_symbols_fixture();
    let out = api_usage(&fixture);
    assert!(
        out.contains("| `auth` | `billing` | `Token` |"),
        "the rust symbol row must be present:\n{out}"
    );
}

/// The c# tree behind the stated fold shape (cvd US 03, DEP-3): a parent
/// namespace `Demo.Parent` with a class of its own and two child namespaces
/// that reference each other — the layout `tests/depgraph.rs`'s
/// `csharp_nested_fixture` pins resolution for.
fn csharp_nested_parent_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "src/Parent.cs",
        "namespace Demo.Parent;\npublic class Mod { }\n",
    );
    fixture.write(
        "src/Parent/A.cs",
        "using Demo.Parent.B;\nnamespace Demo.Parent.A;\npublic class A { }\n",
    );
    fixture.write(
        "src/Parent/B.cs",
        "namespace Demo.Parent.B;\npublic class B { }\n",
    );
    fixture
}

/// The c# `submodules` fold shape as rendered bytes (acceptance rows 9
/// and 20): the parent's own module folds to the lone `mod` node, the child
/// namespaces render as nodes and the edge between children renders — the
/// shape rust row 4 states, shared across drivers as a projection choice
/// (`worklog/done/workplan_depgraph_csharp_views_honest.md`). The existing
/// c# parent pins are contains-asserts (resolution + edge); this is the
/// verbatim node-set pin. The tree carries no roles fact, so the bytes are
/// the pre-marker render (row 37's stated no-node case).
#[test]
fn csharp_submodules_fold_prints_child_nodes_mod_and_edge_verbatim() {
    let fixture = csharp_nested_parent_fixture();
    let expected = "graph TD\n  A\n  B\n  mod\n  A --> B\n";
    let output = fixture.run(&["depgraph", "submodules", "--parent", "Demo.Parent"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), expected, "the fold's node set, verbatim");
    let fallback = fixture.run(&["depgraph", "submodules", "--parent", "Parent"]);
    assert_eq!(
        fallback.status.code(),
        Some(0),
        "{}",
        stderr(&fallback)
    );
    assert_eq!(
        stdout(&fallback),
        expected,
        "the bare-segment compatibility fallback renders the same node set"
    );
}

/// The csharp-first arm as a decision rule (cvd US 03 reachability probe):
/// a c# tree whose model DOES carry symbol facts while every
/// symbol-carrying edge lands inside one rendered node (the
/// `Demo.Core.A`→`Demo.Core.B` type using folds onto the single top node
/// `Core` under a two-node top projection) prints the tree-fact sentence —
/// the language arm precedes the structural symbol arm, first match wins
/// (`api_usage_empty_reason`). The bare statement still never prints on a
/// symbol-bearing model (the never-bare invariant above); the sentence's
/// absence wording misstates such a tree and is recorded as a reported
/// wording defect, pinned here as current behavior, not endorsed as the
/// sentence a fixed arm order would print.
#[test]
fn csharp_arm_prints_the_tree_fact_sentence_even_when_edges_carry_symbols() {
    let fixture = Fixture::new();
    fixture.write("Demo.Core/Demo.Core.csproj", &csproj(&[]));
    fixture.write(
        "Demo.Core/A.cs",
        "using Demo.Core.B.Widget;\nnamespace Demo.Core.A;\npublic class Use { }\n",
    );
    fixture.write(
        "Demo.Core/B.cs",
        "namespace Demo.Core.B;\npublic class Widget { }\n",
    );
    fixture.write("Demo.Api/Demo.Api.csproj", &csproj(&[]));
    fixture.write(
        "Demo.Api/Api.cs",
        "namespace Demo.Api;\npublic class Api { }\n",
    );
    let model = scan(&fixture);
    let has_symbols = model["module_edges"]
        .as_array()
        .expect("module_edges")
        .iter()
        .any(|edge| !edge["symbols"].as_array().expect("symbols").is_empty());
    assert!(
        has_symbols,
        "the probe tree's model must carry symbol facts: {model}"
    );
    let out = api_usage(&fixture);
    assert_eq!(
        out, CSHARP_EMPTY_BODY,
        "arm 1 decides by language order, verbatim, whatever the symbol facts"
    );
    assert_ne!(out, FOLD_PLACEMENT_EMPTY_BODY, "the csharp arm precedes the structural arm");
    assert_ne!(out, BARE_EMPTY_BODY, "symbols exist; the bare statement is illegal");
}
