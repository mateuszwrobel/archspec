//! The inspect model view agrees with scan (workplan
//! archspec_syntax_backends, US 14).
//!
//! The documented invariant is that `inspect tree|scanner` render the
//! scan-phase model, so diagram and model never disagree: `inspect <mode>`
//! must show exactly the module-level boundaries `scan` records, and never a
//! boundary the extraction does not state. The dispatch hands the model to
//! `run_structural`, so these tests pin the AGREEMENT, not the plumbing:
//! every `-->` line of the model view is reconstructed from the `scan` model
//! JSON and compared as an exact set — inspect may show neither more nor
//! fewer module boundaries than the model it claims to render.
//!
//! The fixtures exercise the extraction's real boundary shapes (the csharp
//! tree mixes a type-targeted using with a type-position reference; the
//! go trees are the intra-module and go.work shapes of rows 56–58), so a
//! renderer reading a different model than the scanner would be caught.
//!
//! The file-level map stays extraction-free by decision: its bytes are
//! file-granular discovery facts rendered WITHOUT model extraction, pinned
//! on all three trees.
//!
//! Canonical acceptance rows: `docs/archspec/commands/inspect/acceptance.md`
//! rows 28 (csharp agreement), 29 (go agreement, single
//! module and workspace) and 30 (file-level map stability).

mod common;

use common::{stderr, stdout, Fixture};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Fixture builders (same shapes the scan-side suites pin, kept local so the
// agreement runs cannot drift from the scan-side fixtures).
// ---------------------------------------------------------------------------

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

/// Multi-project csharp tree whose boundaries mix both extraction paths:
/// `App.Api` reaches `App.Core` through a TYPE-TARGETED using (`Engine` is a
/// type inside the declared namespace `App.Core` — the edge is addressed at
/// the owner `App::Core`) and reaches
/// `App.Shared` ONLY through a type position (base list, no using — the
/// scanner records `App::Shared` from the base-list reference).
fn csharp_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "App.Api/App.Api.csproj",
        &csproj(&["App.Core", "App.Shared"]),
    );
    fixture.write(
        "App.Api/Controller.cs",
        "using App.Core.Engine;\nnamespace App.Api;\n\
         public class Controller : App.Shared.Base\n{\n    public Engine Make() => new Engine();\n}\n",
    );
    fixture.write("App.Core/App.Core.csproj", &csproj(&[]));
    fixture.write(
        "App.Core/Engine.cs",
        "namespace App.Core;\npublic class Engine { }\n",
    );
    fixture.write("App.Shared/App.Shared.csproj", &csproj(&[]));
    fixture.write(
        "App.Shared/Base.cs",
        "namespace App.Shared;\npublic class Base { }\n",
    );
    fixture
}

/// The canonical single-module go tree: package `store` imports sibling
/// package `format` and calls the exported selector (one package-to-package
/// module edge from the package reference).
fn go_single_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "store/store.go",
        "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() { format.Title() }\n",
    );
    fixture.write("format/format.go", "package format\n\nfunc Title() {}\n");
    fixture
}

/// The canonical `go.work` tree of row 58: members `example.com/api` and
/// `example.com/store`; `api` imports `store` cross-member with selector call
/// sites, `api/internal/handler` imports `store` cross-member (no selectors)
/// and the member root `api` WITHIN the member (a projected module edge).
fn go_workspace_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write(
        "api/api.go",
        "package api\n\nimport \"example.com/store\"\n\nfunc Api() {\n\tstore.Get()\n}\n",
    );
    fixture.write(
        "api/internal/handler/handler.go",
        "package handler\n\nimport (\n\t\"example.com/api\"\n\t\"example.com/store\"\n)\n\nfunc Handle() { api.Render() }\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write(
        "store/store.go",
        "package store\n\nfunc Get() {}\n\nfunc Render() {}\n",
    );
    fixture
}

// ---------------------------------------------------------------------------
// Comparable views of the two artefacts: the scan model JSON and the
// rendered model view.
// ---------------------------------------------------------------------------

/// The mermaid sanitizer (mirrors `render::mermaid_id`: non-alphanumeric
/// names get an FNV-suffixed id), so expected edge lines can be reconstructed
/// from raw model names exactly as the renderer writes them.
fn mermaid_id(name: &str) -> String {
    let mut sanitized = String::new();
    let mut changed = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            sanitized.push(c);
        } else {
            sanitized.push('_');
            changed = true;
        }
    }
    if !changed {
        return sanitized;
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let hex = format!("{hash:016x}");
    format!("{sanitized}_{}", &hex[..8])
}

/// The plantuml quoting rule (mirrors `render::quoted_escape`): model names
/// stay raw inside quotes — no id sanitization is involved — so expected
/// edge lines can be reconstructed from raw model names exactly as the
/// renderer writes them.
fn plantuml_name(name: &str) -> String {
    format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
}

fn scan_model(fixture: &Fixture) -> Value {
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

/// The (unit, from, to) entries of a model's module edges, in JSON order.
fn module_entries(model: &Value) -> Vec<(String, String, String)> {
    model["module_edges"]
        .as_array()
        .expect("module_edges must be an array")
        .iter()
        .map(|edge| {
            (
                edge["unit"].as_str().expect("unit").to_string(),
                edge["from"].as_str().expect("from").to_string(),
                edge["to"].as_str().expect("to").to_string(),
            )
        })
        .collect()
}

/// The (from, to) endpoints of a model's unit-tier edges, in JSON order.
fn unit_entries(model: &Value) -> Vec<(String, String)> {
    model["edges"]
        .as_array()
        .expect("edges must be an array")
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().expect("from").to_string(),
                edge["to"].as_str().expect("to").to_string(),
            )
        })
        .collect()
}

/// Edge lines a diagram draws: every line carrying the visible link operator,
/// trimmed. The structural renderers emit `id --> id` lines for exactly the
/// module edges (`tree`) plus the unit edges (`scanner`) and nothing else.
fn diagram_edges(diagram: &str) -> Vec<String> {
    let mut lines: Vec<String> = diagram
        .lines()
        .filter(|line| line.contains(" --> "))
        .map(str::trim)
        .map(str::to_string)
        .collect();
    lines.sort();
    lines
}

fn module_lines(entries: &[(String, String, String)]) -> Vec<String> {
    let mut lines: Vec<String> = entries
        .iter()
        .map(|(_, from, to)| format!("{} --> {}", mermaid_id(from), mermaid_id(to)))
        .collect();
    lines.sort();
    lines
}

fn unit_lines(entries: &[(String, String)]) -> Vec<String> {
    let mut lines: Vec<String> = entries
        .iter()
        .map(|(from, to)| format!("{} --> {}", mermaid_id(from), mermaid_id(to)))
        .collect();
    lines.sort();
    lines
}

/// The same lines in plantuml quoting (raw names inside quotes).
fn plantuml_module_lines(entries: &[(String, String, String)]) -> Vec<String> {
    let mut lines: Vec<String> = entries
        .iter()
        .map(|(_, from, to)| format!("{} --> {}", plantuml_name(from), plantuml_name(to)))
        .collect();
    lines.sort();
    lines
}

fn plantuml_unit_lines(entries: &[(String, String)]) -> Vec<String> {
    let mut lines: Vec<String> = entries
        .iter()
        .map(|(from, to)| format!("{} --> {}", plantuml_name(from), plantuml_name(to)))
        .collect();
    lines.sort();
    lines
}

fn inspect(fixture: &Fixture, mode: &str) -> Vec<u8> {
    let output = fixture.run(&["inspect", mode]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`inspect {mode}` must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "stderr must be empty");
    output.stdout
}

/// Run `inspect <mode> --format <format>` with the same exit/stderr contract,
/// returning stdout as text. The model views honour `--format` like the
/// file-level map does: mermaid is the default, plantuml emits the `@startuml`
/// diagram of the same model content.
fn inspect_format(fixture: &Fixture, mode: &str, format: &str) -> String {
    let output = fixture.run(&["inspect", mode, "--format", format]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`inspect {mode} --format {format}` must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "stderr must be empty");
    stdout(&output)
}

/// Run `inspect tree` in both formats (the mermaid default and
/// `--format plantuml`) and assert each draws EXACTLY the module
/// boundaries the scan model records: none dropped, none invented.
/// `expected` restates the fixture contract so the agreement cannot be
/// vacuously true through both commands reading the wrong model.
fn assert_tree_agrees(fixture: &Fixture, expected: &[(&str, &str)]) {
    let model = scan_model(fixture);
    let entries = module_entries(&model);
    let pairs: Vec<(String, String)> = entries
        .iter()
        .map(|(_, from, to)| (from.clone(), to.clone()))
        .collect();
    let mut expected: Vec<(String, String)> = expected
        .iter()
        .map(|(from, to)| (from.to_string(), to.to_string()))
        .collect();
    expected.sort();
    let mut pairs = pairs;
    pairs.sort();
    assert_eq!(
        pairs, expected,
        "scan module_edges boundary set"
    );
    let diagram = inspect(fixture, "tree");
    assert_eq!(
        diagram_edges(&String::from_utf8(diagram.clone()).expect("utf-8")),
        module_lines(&entries),
        "inspect tree boundaries must equal the scan model"
    );
    // Both formats are exercised: `inspect tree --format plantuml` must
    // honour the flag (it once silently emitted Mermaid) and draw exactly the
    // same boundary set, named by raw quoted endpoints.
    let plantuml = inspect_format(fixture, "tree", "plantuml");
    assert!(
        plantuml.starts_with("@startuml"),
        "inspect tree --format plantuml must emit a plantuml diagram:\n{plantuml}"
    );
    assert!(
        !plantuml.contains("graph TD"),
        "inspect tree --format plantuml must not emit mermaid syntax:\n{plantuml}"
    );
    assert_eq!(
        diagram_edges(&plantuml),
        plantuml_module_lines(&entries),
        "inspect tree --format plantuml boundaries must equal the scan model"
    );
}

/// The same agreement for the scanner view, which also draws the unit-tier
/// edges: the diagram's edge multiset must equal module lines plus unit
/// lines, both rebuilt from the model.
fn assert_scanner_agrees(fixture: &Fixture, expected: &[(&str, &str)]) {
    let model = scan_model(fixture);
    let entries = module_entries(&model);
    let pairs: Vec<(String, String)> = entries
        .iter()
        .map(|(_, from, to)| (from.clone(), to.clone()))
        .collect();
    let mut expected: Vec<(String, String)> = expected
        .iter()
        .map(|(from, to)| (from.to_string(), to.to_string()))
        .collect();
    expected.sort();
    let mut pairs = pairs;
    pairs.sort();
    assert_eq!(
        pairs, expected,
        "scan module_edges boundary set"
    );
    let diagram = inspect(fixture, "scanner");
    let mut expected_lines = module_lines(&entries);
    expected_lines.extend(unit_lines(&unit_entries(&model)));
    expected_lines.sort();
    assert_eq!(
        diagram_edges(&String::from_utf8(diagram.clone()).expect("utf-8")),
        expected_lines,
        "inspect scanner boundaries must equal the scan model"
    );
    // The plantuml sibling of the same view, same agreement over the same
    // model: module edges plus unit edges, quoted raw names.
    let plantuml = inspect_format(fixture, "scanner", "plantuml");
    assert!(
        plantuml.starts_with("@startuml"),
        "inspect scanner --format plantuml must emit a plantuml diagram:\n{plantuml}"
    );
    assert!(
        !plantuml.contains("graph TD"),
        "inspect scanner --format plantuml must not emit mermaid syntax:\n{plantuml}"
    );
    let mut expected_plantuml = plantuml_module_lines(&entries);
    expected_plantuml.extend(plantuml_unit_lines(&unit_entries(&model)));
    expected_plantuml.sort();
    assert_eq!(
        diagram_edges(&plantuml),
        expected_plantuml,
        "inspect scanner --format plantuml boundaries must equal the scan model"
    );
}

// ---------------------------------------------------------------------------
// csharp (acceptance row 28)
// ---------------------------------------------------------------------------

/// Row 28, tree view: the diagram draws the owner-addressed
/// `App::Api -> App::Core` (type-targeted using) and
/// `App::Api -> App::Shared` (type position) and nothing else — it equals
/// the scan model exactly.
#[test]
fn inspect_tree_module_boundaries_match_scan_on_csharp() {
    let fixture = csharp_fixture();
    assert_tree_agrees(
        &fixture,
        &[("App::Api", "App::Core"), ("App::Api", "App::Shared")],
    );
}

/// Row 28, scanner view: module boundaries plus the unit-tier edges (the two
/// project references) equal the model.
#[test]
fn inspect_scanner_model_view_matches_scan_on_csharp() {
    let fixture = csharp_fixture();
    assert_scanner_agrees(
        &fixture,
        &[("App::Api", "App::Core"), ("App::Api", "App::Shared")],
    );
}

// ---------------------------------------------------------------------------
// go single module (acceptance row 29, clause 1)
// ---------------------------------------------------------------------------

/// Row 29 clause 1: the single-module tree derives its module tier from
/// the package reference, so BOTH views draw exactly the one
/// package→package boundary the scan model records — `inspect tree` renders
/// rather than refusing.
#[test]
fn inspect_model_view_matches_scan_on_go_single_module() {
    let fixture = go_single_fixture();
    assert_tree_agrees(
        &fixture,
        &[("example.com::demo::store", "example.com::demo::format")],
    );
    assert_scanner_agrees(
        &fixture,
        &[("example.com::demo::store", "example.com::demo::format")],
    );
}

/// The tree-view refusal is a model fact (acceptance row 31): a Go tree with
/// no module facts refuses, naming the missing fact and the tier rule (the
/// tree's package references). The multi-package single-module tree above
/// proves the refusal is not a language verdict: a tree that records package
/// references RENDERS.
#[test]
fn inspect_tree_refusal_names_the_missing_fact() {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("store/store.go", "package store\n\nfunc Save() {}\n");

    let refused = fixture.run(&["inspect", "tree"]);
    assert_ne!(refused.status.code(), Some(0), "refusal exits non-zero");
    assert!(
        stderr(&refused).contains(
            "inspect tree needs the module tier, which this model has none of; the module tier is derived from the tree's package references, which this tree records none of, 'inspect scanner' renders the unit-tier model"
        ),
        "the refusal must name the missing fact and the tier rule: {}",
        stderr(&refused)
    );
}

/// Row 31's second half (cvd US 04): the remedy tail of the refusal sentence
/// is a behavior claim — on the very tree the tree view refuses, `inspect
/// scanner` renders the unit-tier model and exits 0. The refusal is a
/// model-content verdict, not a language or tree verdict, and the exit code
/// and stream the acceptance row quotes (exit 1, stderr) are pinned here.
#[test]
fn inspect_scanner_renders_where_tree_refuses_on_tier_less_go() {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("store/store.go", "package store\n\nfunc Save() {}\n");

    let refused = fixture.run(&["inspect", "tree"]);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "the tree view refuses a tier-less model with exit 1: {}",
        stderr(&refused)
    );

    let rendered = fixture.run(&["inspect", "scanner"]);
    assert_eq!(
        rendered.status.code(),
        Some(0),
        "the scanner view must render where the tree view refuses: {}",
        stderr(&rendered)
    );
    let out = stdout(&rendered);
    assert!(out.contains("graph TD"), "scanner must emit a diagram:\n{out}");
    assert!(
        out.contains("example.com/demo/store"),
        "the refused tree's unit must appear in the scanner diagram:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// go workspace (acceptance row 29, clause 2 — US 13 workspace facts)
// ---------------------------------------------------------------------------

/// Row 29 clause 2: on a `go.work` tree the tree view draws the cross-member
/// pair PLUS the within-member edge the extraction projects (row 58) — equal
/// to the scan model.
#[test]
fn inspect_model_view_matches_scan_on_go_workspace() {
    let fixture = go_workspace_fixture();
    assert_tree_agrees(
        &fixture,
        &[
            ("example.com::api", "example.com::store"),
            ("example.com::api::internal::handler", "example.com::api"),
            ("example.com::api::internal::handler", "example.com::store"),
        ],
    );
    assert_scanner_agrees(
        &fixture,
        &[
            ("example.com::api", "example.com::store"),
            ("example.com::api::internal::handler", "example.com::api"),
            ("example.com::api::internal::handler", "example.com::store"),
        ],
    );
}

// ---------------------------------------------------------------------------
// File-level map stays extraction-free (acceptance row 30)
// ---------------------------------------------------------------------------

/// Decision "Inspect's file-level import map stays extraction-free": the
/// default file-level map renders file-granular discovery facts produced
/// WITHOUT model extraction, so no extraction choice can move its bytes —
/// pinned on every fixture tree here (csharp with the type-position
/// boundaries, single-module go, go.work tree) as a deterministic render.
#[test]
fn inspect_file_level_map_is_deterministic_and_extraction_free() {
    for fixture in [
        csharp_fixture(),
        go_single_fixture(),
        go_workspace_fixture(),
    ] {
        let default = fixture.run(&["inspect"]);
        assert_eq!(default.status.code(), Some(0), "file-level map must exit 0");
        let repeat = fixture.run(&["inspect"]);
        assert_eq!(
            default.stdout, repeat.stdout,
            "the file-level map renders deterministically from discovery facts"
        );
    }
}

// ---------------------------------------------------------------------------
// Format threading into the model views (bug guard).
// ---------------------------------------------------------------------------

/// `inspect tree --format plantuml` emits PlantUML — `@startuml` header, no
/// mermaid `graph TD` — while the format-free invocation stays the Mermaid
/// model diagram: the mermaid default is unchanged and plantuml no longer
/// silently renders mermaid.
#[test]
fn inspect_tree_honours_the_plantuml_format_and_keeps_mermaid_default() {
    let fixture = go_single_fixture();

    let plantuml = inspect_format(&fixture, "tree", "plantuml");
    assert!(
        plantuml.starts_with("@startuml"),
        "missing @startuml header:\n{plantuml}"
    );
    assert!(
        plantuml.contains("@enduml"),
        "missing @enduml footer:\n{plantuml}"
    );
    assert!(
        !plantuml.contains("graph TD"),
        "mermaid syntax leaked into plantuml output:\n{plantuml}"
    );
    assert!(
        plantuml.contains("\"example.com::demo::store\" --> \"example.com::demo::format\""),
        "missing plantuml module edge:\n{plantuml}"
    );

    let default = inspect(&fixture, "tree");
    let default = String::from_utf8(default).expect("utf-8");
    assert!(
        default.starts_with("graph TD"),
        "the mermaid default must stay mermaid:\n{default}"
    );
    assert!(
        !default.contains("@startuml"),
        "the mermaid default must not emit plantuml:\n{default}"
    );
}

// ---------------------------------------------------------------------------
// Declared, labelled mermaid nodes (audit defect D3, workplan
// archspec_audit_defects US 03).
// ---------------------------------------------------------------------------

/// A go model whose module-edge endpoints carry no soft-tier declaration
/// must still declare every id before an arrow references it: each endpoint
/// id appears on its own declaration line above the arrows, labelled with
/// the raw model name (the labelling semantics `diagram --source scan` uses
/// for the same names). Undeclared ids render detached from their
/// subgraphs, unreadable — the defect this pins away.
#[test]
fn inspect_tree_declares_and_labels_every_referenced_id() {
    let fixture = go_single_fixture();
    let diagram = inspect(&fixture, "tree");
    let diagram = String::from_utf8(diagram).expect("utf-8");
    common::assert_declared_before_reference("inspect tree go single", &diagram);
    for name in ["example.com::demo::store", "example.com::demo::format"] {
        let declaration = format!("{}[\"{name}\"]", mermaid_id(name));
        assert!(
            diagram.contains(&declaration),
            "endpoint {name:?} must be declared with its raw model name as label:\n{diagram}"
        );
    }
}

// ---------------------------------------------------------------------------
// Role markers on the model views (workplan archspec_roles, US 06): the
// markers attach to the nodes the model STATES (roles map path -> role) —
// the tree computes nothing.
// ---------------------------------------------------------------------------

/// A marker-bearing string from either output language: role facts reach the
/// diagram only through these two syntaxes (mermaid label suffix, plantuml
/// stereotype) — an absent roles map must leak none of them.
const ROLE_MARKER_STRINGS: &[&str] = &[
    " [facade]",
    " [composition]",
    "<<facade>>",
    "<<composition>>",
];

fn assert_no_role_markers(label: &str, diagram: &str) {
    for marker in ROLE_MARKER_STRINGS {
        assert!(
            !diagram.contains(marker),
            "{label} must carry no role marker {marker:?} without role facts:\n{diagram}"
        );
    }
}

/// rust lib+bin tree whose model states both roles: `facade` at the unit path
/// `kit` (declaration-only lib root) and `composition` at the wiring main
/// root `kit-bin::main`. Same shape the scan roles scenario pins.
fn rust_roles_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod engine;\npub use engine::Thing;\n");
    fixture.write("src/engine.rs", "pub struct Thing;\n");
    fixture.write(
        "src/main.rs",
        "mod wire;\nuse crate::wire::glue;\nfn main() { glue(); }\n",
    );
    fixture.write("src/wire.rs", "pub fn glue() {}\n");
    fixture
}

/// The csharp DI-entrypoint tree: the model states `composition` at the
/// root module path `App::Api` and nothing else (the exclusivity pin).
fn csharp_di_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "App.Api/App.Api.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
         <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  \
         <ItemGroup>\n    <ProjectReference Include=\"..\\App.Core\\App.Core.csproj\" />\n  \
         </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App.Api/Program.cs",
        "using App.Core;\n\
         var builder = WebApplication.CreateBuilder(args);\n\
         builder.Services.AddScoped<IEngine, Engine>();\n\
         var app = builder.Build();\n\
         app.Run();\n",
    );
    fixture.write(
        "App.Core/App.Core.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
         <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "App.Core/Core.cs",
        "namespace App.Core;\npublic interface IEngine { }\npublic class Engine { }\n",
    );
    fixture
}

/// A single-module go tree whose root directory holds package main wiring an
/// internal package: the model states `composition` at the unit path
/// `example.com/demo` (the same identity the unit-tier edges use).
fn go_main_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "main.go",
        "package main\n\nimport \"example.com/demo/core\"\n\nfunc main() { core.Setup() }\n",
    );
    fixture.write("core/core.go", "package core\n\nfunc Setup() {}\n");
    fixture
}

#[test]
fn inspect_tree_marks_role_carrying_units_and_modules_on_rust() {
    let fixture = rust_roles_fixture();
    let diagram = String::from_utf8(inspect(&fixture, "tree")).expect("utf-8");
    assert!(
        diagram.contains("subgraph kit[\"kit [facade]\"]"),
        "the facade unit must be marked in its subgraph declaration:\n{diagram}"
    );
    assert!(
        diagram.contains("[\"kit-bin::main [composition]\"]"),
        "the composition root must be marked wherever it is declared:\n{diagram}"
    );
    // Nodes the model leaves unmarked keep their exact D3 declaration bytes.
    assert!(
        diagram.contains("[\"kit::engine\"]"),
        "unmarked module declarations must not shift:\n{diagram}"
    );
    // The declaration-first property survives the markers (audit D3).
    common::assert_declared_before_reference("inspect tree rust roles", &diagram);

    // The scanner view draws the same marked declarations.
    let scanner = String::from_utf8(inspect(&fixture, "scanner")).expect("utf-8");
    assert!(
        scanner.contains("subgraph kit[\"kit [facade]\"]"),
        "scanner view must mark the facade unit too:\n{scanner}"
    );
}

#[test]
fn inspect_tree_marks_role_declarations_in_plantuml_on_rust() {
    let fixture = rust_roles_fixture();
    let plantuml = inspect_format(&fixture, "tree", "plantuml");
    assert!(
        plantuml.contains("package \"kit\" <<facade>> {"),
        "facade unit package missing its stereotype:\n{plantuml}"
    );
    assert!(
        plantuml.contains("component \"kit-bin::main\" <<composition>>"),
        "composition root component missing its stereotype:\n{plantuml}"
    );
    // Identity-preserving markers: edges still quote the RAW model names, so
    // the inspect==scan agreement is untouched by the markers.
    assert!(
        plantuml.contains("\"kit-bin::main\" --> \"kit-bin::wire\""),
        "edge endpoints must stay unmarked raw names:\n{plantuml}"
    );
    let unmarked = inspect_format(&csharp_fixture(), "tree", "plantuml");
    assert_no_role_markers("plantuml tree without role facts", &unmarked);
}

#[test]
fn inspect_tree_marks_the_csharp_composition_root_by_module_path() {
    let fixture = csharp_di_fixture();
    let diagram = String::from_utf8(inspect(&fixture, "tree")).expect("utf-8");
    assert!(
        diagram.contains("[\"App::Api [composition]\"]"),
        "the DI composition root must be marked by its module path:\n{diagram}"
    );
    // The exclusivity of the derivation reaches the diagram: no facade
    // marker appears anywhere in this tree's render.
    assert!(
        !diagram.contains("[facade]"),
        "the DI entrypoint never also reads as a facade:\n{diagram}"
    );
}

#[test]
fn inspect_tree_marks_the_go_main_package_composition_root() {
    let fixture = go_main_fixture();
    let diagram = String::from_utf8(inspect(&fixture, "tree")).expect("utf-8");
    // The roles key is the unit path (slash identity) — the marker attaches
    // to the declaration that names that path, the unit subgraph header.
    assert!(
        diagram.contains("[\"example.com/demo [composition]\"]"),
        "the main-package unit must be marked in its declaration:\n{diagram}"
    );
    let plantuml = inspect_format(&fixture, "tree", "plantuml");
    assert!(
        plantuml.contains("package \"example.com/demo\" <<composition>> {"),
        "plantuml marks the same unit through its package declaration:\n{plantuml}"
    );
}

/// Guard (roles US 06): markers appear IFF the model's roles map has entries.
/// The trees whose scans state no roles (the agreement fixtures) must render
/// byte-identical to pre-marker output — no marker noise.
#[test]
fn inspect_views_carry_no_role_markers_without_role_facts() {
    for (label, fixture) in [
        ("csharp tree", csharp_fixture()),
        ("go single", go_single_fixture()),
        ("go workspace", go_workspace_fixture()),
    ] {
        let model = scan_model(&fixture);
        assert!(
            model.get("roles").is_none()
                || model["roles"]
                    .as_object()
                    .is_some_and(|map| map.is_empty()),
            "fixture {label} must state no roles for this guard to mean anything:\n{model}"
        );
        for mode in ["tree", "scanner"] {
            let mermaid = String::from_utf8(inspect(&fixture, mode)).expect("utf-8");
            assert_no_role_markers(&format!("{label} mermaid {mode}"), &mermaid);
            let plantuml = inspect_format(&fixture, mode, "plantuml");
            assert_no_role_markers(&format!("{label} plantuml {mode}"), &plantuml);
        }
    }
}

/// Determinism (US 06: the determinism suite pins marker ordering): markers
/// come from the roles map in model order, so repeated renders — and renders
/// of the same model — are byte-identical.
#[test]
fn role_markers_are_byte_stable_across_repeated_renders() {
    let fixture = rust_roles_fixture();
    let first = inspect(&fixture, "tree");
    for _ in 0..4 {
        assert_eq!(
            first,
            inspect(&fixture, "tree"),
            "role markers must render byte-identically across runs"
        );
    }
}
