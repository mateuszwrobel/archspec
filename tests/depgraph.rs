mod common;
#[allow(dead_code)]
mod shared;

use common::{stderr, stdout, Fixture};

/// A single-unit crate where `billing` imports a named item from `auth`, so the
/// model carries one module edge with one symbol. Locks the api-usage symbol
/// column and the projected module edge precisely.
fn edge_fixture() -> Fixture {
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

/// A crate with a parent module and two children that reference each other, to
/// lock the submodule projection (children + `mod`, edge between children).
fn nested_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod parent;\n");
    fixture.write("src/parent.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/parent/a.rs", "use crate::parent::b::Thing;\npub fn use_it() -> b::Thing { b::Thing }\n");
    fixture.write("src/parent/b.rs", "pub struct Thing;\n");
    fixture
}

#[test]
fn api_usage_locks_symbol_rows() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("| Target module | Used by module | APIs used |"),
        "missing header:\n{out}"
    );
    assert!(
        out.contains("| --- | --- | --- |"),
        "missing separator:\n{out}"
    );
    assert!(
        out.contains("| `auth` | `billing` | `Token` |"),
        "missing symbol row:\n{out}"
    );
}

#[test]
fn modules_projects_the_import_edge() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a mermaid graph:\n{out}");
    assert!(out.contains("billing --> auth"), "missing edge:\n{out}");
}

#[test]
fn modules_output_is_deterministic() {
    let fixture = edge_fixture();
    let first = stdout(&fixture.run(&["depgraph", "modules"]));
    let second = stdout(&fixture.run(&["depgraph", "modules"]));
    assert_eq!(first, second, "depgraph modules output differs across runs");
}

#[test]
fn modules_plantuml_format() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "modules", "--format", "plantuml"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("@startuml") && out.contains("@enduml"), "not plantuml:\n{out}");
    assert!(out.contains("billing --> auth"), "missing plantuml edge:\n{out}");
}

#[test]
fn output_writes_file_and_leaves_stdout_empty() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "modules", "--output", "graph.mmd"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).is_empty(), "stdout must be empty when --output is set");
    assert!(
        fixture.read("graph.mmd").contains("billing --> auth"),
        "written file missing edge"
    );
}

#[test]
fn submodules_scopes_to_parent() {
    let fixture = nested_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "parent"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("graph TD"), "not a graph:\n{out}");
    assert!(out.contains("a --> b"), "missing child edge:\n{out}");
    assert!(out.contains("mod"), "parent's own `mod` node missing:\n{out}");
}

#[test]
fn unknown_view_errors() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "bogus"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("unknown depgraph view"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn submodules_requires_parent() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "submodules"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("requires --parent"), "{}", stderr(&output));
}

#[test]
fn parent_only_for_submodules() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "modules", "--parent", "auth"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("only valid for the submodules view"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn unknown_format_errors() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "modules", "--format", "bmp"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("unsupported format"), "{}", stderr(&output));
}

#[test]
fn api_usage_rejects_graph_format() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "api-usage", "--format", "mermaid"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("unsupported format"), "{}", stderr(&output));
}

/// A Go tree whose packages import each other but whose module tier is absent
/// (the go driver emits neither soft_structure nor module_edges), so every
/// depgraph view must refuse for the module-tier reason, not an internal one.
fn go_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "main.go",
        "package main\n\nimport \"example.com/demo/internal/service\"\n\nfunc main() {}\n",
    );
    fixture.write(
        "internal/service/service.go",
        "package service\n\nimport \"example.com/demo/internal/store\"\n\nfunc Do() {}\n",
    );
    fixture.write("internal/store/store.go", "package store\n\nfunc Get() {}\n");
    fixture
}

/// A single-unit csharp tree whose two namespaces reference each other, so the
/// module tier is present via namespaces/usings and views must render untouched.
fn csharp_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("src/A.cs", "namespace Demo.App;\npublic class A { }\n");
    fixture.write(
        "src/B.cs",
        "using Demo.App;\nnamespace Demo.Other;\npublic class B { }\n",
    );
    fixture
}

/// The exact capability sentence every depgraph view must emit for a
/// module-tier-less model, and that the docs/help must carry verbatim.
const MODULE_TIER_MESSAGE: &str = "depgraph needs the module tier, which this model has none of; a Go tree acquires one through go.work members (2+)";

#[test]
fn modules_refuses_go_for_module_tier_reason() {
    let fixture = go_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(1), "modules on go must exit 1");
    let err = stderr(&output);
    assert!(err.contains(MODULE_TIER_MESSAGE), "wrong refusal message:\n{err}");
    assert!(!err.contains("no model content to render"), "old opaque error leaked:\n{err}");
    assert!(stdout(&output).is_empty(), "no partial graph may reach stdout");
}

#[test]
fn api_usage_refuses_go_for_module_tier_reason() {
    let fixture = go_fixture();
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(output.status.code(), Some(1), "api-usage on go must exit 1");
    let err = stderr(&output);
    assert!(err.contains(MODULE_TIER_MESSAGE), "wrong refusal message:\n{err}");
    assert!(!err.contains("no model content to render"), "old opaque error leaked:\n{err}");
    assert!(stdout(&output).is_empty(), "no partial body may reach stdout");
}

#[test]
fn submodules_refuses_go_for_module_tier_reason() {
    let fixture = go_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "store"]);
    assert_eq!(output.status.code(), Some(1), "submodules on go must exit 1");
    let err = stderr(&output);
    assert!(err.contains(MODULE_TIER_MESSAGE), "wrong refusal message:\n{err}");
    assert!(
        !err.contains("parent module not found"),
        "old opaque error leaked:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no partial graph may reach stdout");
}

#[test]
fn csharp_modules_renders_untouched() {
    let fixture = csharp_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a mermaid graph:\n{out}");
    assert!(out.contains("Other --> App"), "missing csharp edge:\n{out}");
}

#[test]
fn unknown_parent_errors() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "ghost"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("parent module not found"),
        "{}",
        stderr(&output)
    );
}

/// The c# mirror of `nested_fixture`: parent namespace with two sibling child
/// namespaces referencing each other, laid out flat like a real .csproj tree
/// (`Parent.cs` for the parent, `Parent/A.cs` and `Parent/B.cs` for children).
fn csharp_nested_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("src/Parent.cs", "namespace Demo.Parent;\npublic class Mod { }\n");
    fixture.write(
        "src/Parent/A.cs",
        "using Demo.Parent.B;\nnamespace Demo.Parent.A;\npublic class A { }\n",
    );
    fixture.write("src/Parent/B.cs", "namespace Demo.Parent.B;\npublic class B { }\n");
    fixture
}

/// Current-shape pin: the c# modules view folds a nested parent into a single
/// node and the fold drops the intra-parent edge entirely — the folded graph
/// shows the parent alone. Shapes are identical across drivers: `modules_graph`
/// is one projection for all drivers.
#[test]
fn csharp_modules_view_folds_nested_parent_without_child_edges() {
    let fixture = csharp_nested_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a mermaid graph:\n{out}");
    assert!(out.contains("Parent"), "fold node missing:\n{out}");
    assert!(!out.contains("A --> B"), "child edge must not survive the fold:\n{out}");
    assert!(
        !out.contains("Parent --> Parent"),
        "self-loop must not appear on the fold node:\n{out}"
    );
}

/// Current-shape pin: the c# submodules view does not fold — it expands the
/// parent's children, its own `mod` node, and the child edge, matching the
/// rust submodules view.
#[test]
fn csharp_submodules_view_expands_nested_parent_with_child_edge() {
    let fixture = csharp_nested_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "Parent"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("graph TD"), "not a graph:\n{out}");
    assert!(out.contains("A --> B"), "missing child edge:\n{out}");
    assert!(out.contains("mod"), "parent's own `mod` node missing:\n{out}");
}

/// Current-shape pin (superseded by the honest parent lookup): the bare last
/// segment still resolves after full-name acceptance — compatibility fallback
/// for scripts written against the old behavior.
#[test]
fn csharp_submodules_parent_accepts_full_module_name() {
    let fixture = csharp_nested_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "Demo.Parent"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("A --> B"), "missing child edge:\n{out}");
    assert!(out.contains("mod"), "parent's own `mod` node missing:\n{out}");
}

#[test]
fn csharp_submodules_parent_falls_back_to_bare_segment() {
    let fixture = csharp_nested_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "Parent"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("A --> B"), "missing child edge:\n{out}");
    assert!(out.contains("mod"), "parent's own `mod` node missing:\n{out}");
}

#[test]
fn csharp_unknown_parent_lists_known_names() {
    let fixture = csharp_nested_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "Demo.Ghost"]);
    assert_eq!(output.status.code(), Some(1));
    let err = stderr(&output);
    assert!(err.contains("parent module not found"), "{err}");
    assert!(
        err.contains("Demo::Parent"),
        "error must list the known top-level modules:\n{err}"
    );
}

/// A c# solution whose projects reference each other (Api -> Application ->
/// Domain) through `ProjectReference` items: the unit graph carries every
/// edge, and the modules view must project them instead of rendering nodes
/// only.
fn csharp_solution_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "HomeBudget.Api/HomeBudget.Api.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\HomeBudget.Application\\HomeBudget.Application.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "HomeBudget.Application/HomeBudget.Application.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\HomeBudget.Domain\\HomeBudget.Domain.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "HomeBudget.Domain/HomeBudget.Domain.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "HomeBudget.Api/Controllers.cs",
        "using HomeBudget.Application;\nnamespace HomeBudget.Api.Controllers;\npublic class C { }\n",
    );
    fixture.write(
        "HomeBudget.Application/Services.cs",
        "using HomeBudget.Domain;\nnamespace HomeBudget.Application.Services;\npublic class S { }\n",
    );
    fixture.write(
        "HomeBudget.Domain/Entities.cs",
        "namespace HomeBudget.Domain.Entities;\npublic class E { }\n",
    );
    fixture
}

#[test]
fn csharp_solution_modules_view_draws_project_edges() {
    let fixture = csharp_solution_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("Api --> Application"),
        "ProjectReference edge must project onto the modules view:\n{out}"
    );
    assert!(
        out.contains("Application --> Domain"),
        "ProjectReference edge must project onto the modules view:\n{out}"
    );
}

#[test]
fn csharp_api_usage_empty_carries_reason_sentence() {
    let fixture = csharp_fixture();
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("No internal API usage details found"), "{out}");
    assert!(
        out.contains("csharp driver"),
        "empty csharp result must carry the driver reason:\n{out}"
    );
    assert_ne!(
        out,
        "No internal API usage details found.\n",
        "the bare sentence must not stand alone for csharp:\n{out}"
    );
}

#[test]
fn rust_api_usage_empty_keeps_bare_sentence() {
    let fixture = nested_fixture();
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "No internal API usage details found.\n");
}

/// A bin+lib crate whose binary consumes its library through `use`: the
/// dependency exists only at the unit tier (the bin tree references the lib
/// crate name, never a dotted lib module path), so the modules view must
/// project the unit edge onto the two units' top-node sets instead of
/// rendering bin and lib as edgeless islands.
fn bin_lib_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod core_api;\npub mod store;\n");
    fixture.write("src/core_api.rs", "pub fn run() {}\n");
    fixture.write("src/store.rs", "pub struct Db;\n");
    fixture.write("src/main.rs", "use app::{core_api, store};\nfn main() {}\n");
    fixture
}

#[test]
fn modules_projects_bin_to_lib_unit_edge_onto_top_nodes() {
    let fixture = bin_lib_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("main --> core_api"),
        "bin->lib unit edge must project onto the lib's top nodes:\n{out}"
    );
    assert!(
        out.contains("main --> store"),
        "bin->lib unit edge must project onto the lib's top nodes:\n{out}"
    );
    assert!(!out.contains("main --> main"), "self-edge must not appear:\n{out}");
}

#[test]
fn api_usage_empty_with_unit_links_carries_placement_reason() {
    let fixture = bin_lib_fixture();
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("No internal API usage details found"), "{out}");
    assert!(
        out.contains("unit granularity"),
        "empty rust result with unit-tier links must carry the placement reason:\n{out}"
    );
    assert_ne!(
        out,
        "No internal API usage details found.\n",
        "the bare sentence must not stand alone when unit-tier links exist:\n{out}"
    );
}

/// A two-member go.work workspace: the module tier is present (member modules
/// and their packages), so the module-tier guard — keyed on the structural
/// `has_module_tier`, not on language — must let views render instead of
/// refusing.
fn go_workspace_fixture() -> Fixture {
    let fixture = Fixture::new();
    shared::driver::materialize_go_workspace(&fixture);
    fixture
}

#[test]
fn modules_renders_go_workspace_members() {
    let fixture = go_workspace_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "workspace go must render, not be refused: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a mermaid graph:\n{out}");
    assert!(
        out.contains("api --> store"),
        "missing workspace member edge:\n{out}"
    );
}

#[test]
fn submodules_renders_go_workspace_member_children() {
    let fixture = go_workspace_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "api"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "workspace go submodules must render: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a mermaid graph:\n{out}");
    assert!(
        out.contains("internal"),
        "nested member package must surface:\n{out}"
    );
}
