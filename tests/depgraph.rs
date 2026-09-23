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
    fixture.write(
        "src/parent/a.rs",
        "use crate::parent::b::Thing;\npub fn use_it() -> b::Thing { b::Thing }\n",
    );
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
    assert!(
        out.contains("@startuml") && out.contains("@enduml"),
        "not plantuml:\n{out}"
    );
    assert!(
        out.contains("billing --> auth"),
        "missing plantuml edge:\n{out}"
    );
}

#[test]
fn output_writes_file_and_echoes_wrote_line() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "modules", "--output", "graph.mmd"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "wrote graph.mmd\n",
        "depgraph --output echoes the universal wrote line (blind4 D01)"
    );
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
    assert!(
        out.contains("mod"),
        "parent's own `mod` node missing:\n{out}"
    );
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
    assert!(
        stderr(&output).contains("requires --parent"),
        "{}",
        stderr(&output)
    );
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
    assert!(
        stderr(&output).contains("unsupported format"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn api_usage_rejects_graph_format() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "api-usage", "--format", "mermaid"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("unsupported format"),
        "{}",
        stderr(&output)
    );
}

/// A Go tree whose packages import each other inside a single `go.mod`: the
/// syntax extraction derives the module tier natively from those package
/// references, so every depgraph view renders instead of refusing. `modules`
/// renders the package graph (the top-level projection is one node, so the
/// trivial-fold rule reads the paths below it); `api-usage` groups at that
/// same granularity — on this fixture the edges carry no selectors, so the
/// symbol-free empty statement applies.
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
    fixture.write(
        "internal/store/store.go",
        "package store\n\nfunc Get() {}\n",
    );
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
const MODULE_TIER_MESSAGE: &str = "depgraph needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of";

/// `depgraph modules` on a single-module go tree whose packages reference each
/// other: the module tier is native (derived from the package references), and
/// every package sits under the one module, so the top-level projection is a
/// single node. The trivial-fold rule renders what is below that fold — the
/// package graph — one node per participating package and one arrow per
/// cross-package import.
#[test]
fn modules_renders_go_single_module_package_graph() {
    let fixture = go_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "modules on a tier-carrying go tree must render: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a mermaid graph:\n{out}");
    for package in ["demo", "service", "store"] {
        assert!(out.contains(package), "package node `{package}` missing:\n{out}");
    }
    assert!(out.contains("demo --> service"), "root-package arrow missing:\n{out}");
    assert!(out.contains("service --> store"), "cross-package arrow missing:\n{out}");
    assert!(
        !stderr(&output).contains(MODULE_TIER_MESSAGE),
        "a tier-carrying go tree must not be refused:\n{}",
        stderr(&output)
    );
}

/// The shape the audit found on a real single-module go tree: one `go.mod` whose
/// module path carries host and organisation segments, packages below it
/// importing each other. Descending one segment at a time would stop at the
/// organisation segment and print one node again, so the rule reads the paths
/// below the fold: `modules` renders ≥ 13 package nodes plus arrows, never a
/// lone folded node.
#[test]
fn modules_renders_package_graph_of_deeply_addressed_single_module() {
    const PACKAGES: usize = 13;
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/tools/pipeline\ngo 1.21\n");
    for index in 0..PACKAGES {
        let dir = format!("pkg{index:02}");
        let mut content = format!("package p{index:02}\n");
        if index + 1 < PACKAGES {
            content.push_str(&format!("\nimport \"example.com/tools/pipeline/pkg{:02}\"\n", index + 1));
        }
        fixture.write(&format!("{dir}/{dir}.go"), &content);
    }
    fixture.write(
        "cmd/server/server.go",
        "package main\n\nimport \"example.com/tools/pipeline/pkg00\"\n\nfunc main() {}\n",
    );

    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a mermaid graph:\n{out}");
    let nodes = out
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.contains("-->") && *line != "graph TD")
        .count();
    assert!(
        nodes >= 13,
        "expected >= 13 package nodes, got {nodes}:\n{out}"
    );
    assert!(out.contains("-->"), "cross-package arrows missing:\n{out}");
    assert!(
        !stderr(&output).contains(MODULE_TIER_MESSAGE),
        "a tier-carrying go tree must not be refused:\n{}",
        stderr(&output)
    );
}

/// `depgraph api-usage` on the same tree: the guard clears and the bare
/// statement stands — legitimately, because this tree's module edges record
/// plain package references with no symbol facts (D4 makes the bare sentence
/// legal exactly for symbol-free models; symbol-bearing shapes print a table
/// or a reasoned sentence, pinned in `tests/depgraph_reasons.rs`).
#[test]
fn api_usage_symbol_free_go_model_keeps_bare_empty_statement() {
    let fixture = go_fixture();
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "api-usage on a tier-carrying go tree must render: {}",
        stderr(&output)
    );
    assert_eq!(
        stdout(&output),
        "No internal API usage details found.\nnote: this view shows no roles \
         by decision (a role is not a usage fact) — explained in 'archspec help roles'\n",
        "a symbol-free model keeps the bare statement, and the role-less note \
         trails it (roles plan US 06)"
    );
}

/// `depgraph submodules` on the same tree: the module tier's vocabulary is the
/// dotted module path (`example.com::demo`), so that is the valid parent; the
/// view expands the module's children under it.
#[test]
fn submodules_renders_go_single_module_dotted_parent() {
    let fixture = go_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "example.com::demo"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "submodules on a tier-carrying go tree must render: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a graph:\n{out}");
    assert!(out.contains("internal"), "child package missing:\n{out}");
    assert!(
        !stderr(&output).contains(MODULE_TIER_MESSAGE),
        "a tier-carrying go tree must not be refused:\n{}",
        stderr(&output)
    );
}

/// A single-package go tree importing only the standard library records no
/// package reference at all, so the module tier derivation yields nothing and
/// every view refuses with the single capability sentence, exit 1, no partial
/// output.
fn go_tierless_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "main.go",
        "package main\n\nimport \"fmt\"\n\nfunc main() { fmt.Println() }\n",
    );
    fixture
}

#[test]
fn depgraph_views_refuse_tier_less_single_package_go() {
    let fixture = go_tierless_fixture();
    for view in [
        vec!["depgraph", "modules"],
        vec!["depgraph", "api-usage"],
        vec!["depgraph", "submodules", "--parent", "demo"],
    ] {
        let output = fixture.run(&view);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{view:?} on a tier-less go tree must exit 1"
        );
        let err = stderr(&output);
        assert!(
            err.contains(MODULE_TIER_MESSAGE),
            "{view:?} wrong refusal message:\n{err}"
        );
        assert!(
            !err.contains("no model content to render"),
            "old opaque error leaked:\n{err}"
        );
        assert!(
            stdout(&output).is_empty(),
            "{view:?}: no partial output may reach stdout"
        );
    }
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

/// The same trivial-fold rule as the go views, on the c# driver: a nested parent
/// namespace whose children reference each other projects to ONE top-level node,
/// so the modules view renders what sits below the fold — the child namespaces
/// and their edge — instead of the parent alone. The rule is keyed on the shape of
/// the projection, not on the driver (D1), which is why the c# picture matches
/// `submodules --parent Demo.Parent` here; a c# tree whose projection has several
/// nodes keeps the top-level vocabulary untouched (`tests/goldens/depgraph-modules`).
#[test]
fn csharp_modules_view_renders_children_of_trivially_folded_parent() {
    let fixture = csharp_nested_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "not a mermaid graph:\n{out}");
    assert!(out.contains("Parent"), "parent node missing:\n{out}");
    assert!(out.contains("A --> B"), "child edge missing:\n{out}");
    assert!(
        !out.contains("Parent --> Parent"),
        "self-loop must not appear on the parent node:\n{out}"
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
    assert!(
        out.contains("mod"),
        "parent's own `mod` node missing:\n{out}"
    );
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
    assert!(
        out.contains("mod"),
        "parent's own `mod` node missing:\n{out}"
    );
}

#[test]
fn csharp_submodules_parent_falls_back_to_bare_segment() {
    let fixture = csharp_nested_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "Parent"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("A --> B"), "missing child edge:\n{out}");
    assert!(
        out.contains("mod"),
        "parent's own `mod` node missing:\n{out}"
    );
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
        "Shop.Api/Shop.Api.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Shop.Application\\Shop.Application.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "Shop.Application/Shop.Application.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Shop.Domain\\Shop.Domain.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "Shop.Domain/Shop.Domain.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "Shop.Api/Controllers.cs",
        "using Shop.Application;\nnamespace Shop.Api.Controllers;\npublic class C { }\n",
    );
    fixture.write(
        "Shop.Application/Services.cs",
        "using Shop.Domain;\nnamespace Shop.Application.Services;\npublic class S { }\n",
    );
    fixture.write(
        "Shop.Domain/Entities.cs",
        "namespace Shop.Domain.Entities;\npublic class E { }\n",
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
        out.contains("No symbol facts were emitted for this tree."),
        "empty csharp result must carry the tree-level reason:\n{out}"
    );
    assert_ne!(
        out, "No internal API usage details found.\n",
        "the bare sentence must not stand alone for csharp:\n{out}"
    );
}

/// The nested rust crate of the submodule pin: its one module edge carries
/// the symbol `Thing` between sibling child modules, whose top-level
/// projection is a single node — the trivial fold. api-usage groups at the
/// granularity the modules view renders (D4 reads D1's rule), so the symbol
/// lands in a child-module row instead of the bare statement.
#[test]
fn rust_api_usage_renders_folded_child_module_rows() {
    let fixture = nested_fixture();
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("| `b` | `a` | `Thing` |"),
        "the folded edge's symbol must render at child granularity:\n{out}"
    );
    assert!(
        !out.starts_with("No internal API usage details found."),
        "symbols exist; the bare statement must not appear:\n{out}"
    );
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
    assert!(
        !out.contains("main --> main"),
        "self-edge must not appear:\n{out}"
    );
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
        out, "No internal API usage details found.\n",
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

// ---------------------------------------------------------------------------
// Role marks (roles-views US 02, acceptance rows 34–38): the modules view
// marks a node only when the roles paths folded into it agree on exactly one
// role; a conflicting fold, a key below/outside the rendered vocabulary, and
// an empty roles map all keep the exact pre-marker bytes.
// ---------------------------------------------------------------------------

/// A bin-only rust crate whose `main.rs` wires its own module: the rust
/// driver states `composition` at `tool::main`, which folds onto the `main`
/// node this view already renders (acceptance row 34).
fn rust_composition_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/main.rs", "mod store;\nuse store::Db;\nfn main() { let _ = Db; }\n");
    fixture.write("src/store.rs", "pub struct Db;\n");
    fixture
}

#[test]
fn modules_marks_rust_composition_root_on_the_main_node() {
    let fixture = rust_composition_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("main[\"main [composition]\"]"),
        "the composition root must carry its marker:\n{out}"
    );
    assert!(
        out.contains("main --> store"),
        "edge lines stay byte-identical:\n{out}"
    );
    assert!(
        !out.contains("store["),
        "the unaddressed node keeps its unmarked bytes:\n{out}"
    );
}

#[test]
fn modules_rust_composition_mark_is_a_bare_stereotype_in_plantuml() {
    let fixture = rust_composition_fixture();
    let output = fixture.run(&["depgraph", "modules", "--format", "plantuml"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("component main <<composition>>"),
        "plantuml stereotype must ride the bare identifier:\n{out}"
    );
    assert!(
        out.contains("main --> store"),
        "plantuml edge identity is untouched by the stereotype:\n{out}"
    );
}

/// `edge_fixture`'s model carries `"app": "facade"` — a unit-tier key the
/// rust driver spells as the unit name. It folds onto the `root` node this
/// view does not render (the rust soft tier addresses modules, not the unit
/// root), so the honest output is the exact pre-marker graph: no node is
/// invented to carry a role (acceptance row 37).
#[test]
fn modules_rust_facade_key_folds_to_no_rendered_node_and_prints_no_marker() {
    let fixture = edge_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(
        out,
        "graph TD\n  auth\n  billing\n  billing --> auth\n",
        "a role below this view's vocabulary must not move a single byte:\n{out}"
    );
}

#[test]
fn modules_marks_go_main_package_at_the_folded_granularity() {
    let fixture = go_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("demo[\"demo [composition]\"]"),
        "the go composition root must be marked at the granularity the \
         trivial fold renders (key spelling `example.com/demo`, node `demo`):\n{out}"
    );
    assert!(
        out.contains("demo --> service") && out.contains("service --> store"),
        "edge lines stay byte-identical:\n{out}"
    );
    assert!(
        !out.contains("service["),
        "packages the map does not address keep their bytes:\n{out}"
    );
}

/// The go marker key carries the import-path spelling
/// (`example.com/tools/pipeline/cmd/server`); the fold lifts it to the model
/// path grammar, and under the trivial fold the main package's node is its
/// final segment (acceptance row 34).
#[test]
fn modules_marks_deeply_addressed_go_main_package_below_the_fold() {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/tools/pipeline\ngo 1.21\n");
    fixture.write(
        "cmd/server/server.go",
        "package main\n\nimport \"example.com/tools/pipeline/pkg/a\"\n\nfunc main() { a.A() }\n",
    );
    fixture.write(
        "pkg/a/a.go",
        "package a\n\nimport \"example.com/tools/pipeline/pkg/b\"\n\nfunc A() { b.B() }\n",
    );
    fixture.write("pkg/b/b.go", "package b\n\nfunc B() {}\n");
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("server[\"server [composition]\"]"),
        "the deeply addressed main package must be marked by its folded name:\n{out}"
    );
    assert!(
        out.contains("server --> a"),
        "edge lines stay byte-identical:\n{out}"
    );
}

/// A c# solution whose composition root registers services in `Program.cs`:
/// the driver keys `composition` at `Shop::Api`, which the view folds onto
/// the `Api` node it renders (acceptance row 34).
fn csharp_composition_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Shop.Api/Shop.Api.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Shop.Domain\\Shop.Domain.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "Shop.Api/Program.cs",
        "using Microsoft.Extensions.DependencyInjection;\nusing Shop.Domain;\nnamespace Shop.Api;\npublic static class Program\n{\n    public static void Main()\n    {\n        var services = new ServiceCollection();\n        services.AddScoped<IStore, Store>();\n    }\n}\n",
    );
    fixture.write(
        "Shop.Domain/Shop.Domain.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "Shop.Domain/Domain.cs",
        "namespace Shop.Domain;\npublic interface IStore { }\npublic class Store : IStore { }\n",
    );
    fixture
}

#[test]
fn modules_marks_csharp_project_root_composition() {
    let fixture = csharp_composition_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("Api[\"Api [composition]\"]"),
        "the c# project root key `Shop::Api` must fold onto node `Api`:\n{out}"
    );
    assert!(
        out.contains("Api --> Domain"),
        "edge lines stay byte-identical:\n{out}"
    );
}

/// Two projects whose root module keys (`X::Api`, `Y::Api`) fold onto ONE
/// node whose roles differ: the fold is too coarse to state a role, so the
/// node carries no marker and nothing else moves (acceptance row 35). With
/// `second_facade` the two agree on one role and the node is marked — once
/// (acceptance row 36).
fn csharp_fold_fixture(second_facade: bool) -> Fixture {
    let fixture = Fixture::new();
    let csproj = |reference: &str| {
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n{}{}</Project>\n",
            if reference.is_empty() { "" } else { "  <ItemGroup>\n" },
            if reference.is_empty() {
                String::new()
            } else {
                format!("    <ProjectReference Include=\"{reference}\" />\n  </ItemGroup>\n")
            }
        )
    };
    fixture.write("X.Api/X.Api.csproj", &csproj("../Y.Shared/Y.Shared.csproj"));
    // Root-namespace wiring without declared types: the driver's facade
    // predicate (imports but defines nothing).
    fixture.write("X.Api/Wiring.cs", "using Y.Shared;\nnamespace X.Api;\n");
    if second_facade {
        fixture.write("Y.Api/Y.Api.csproj", &csproj(""));
        fixture.write("Y.Api/Wiring.cs", "using Y.Shared;\nnamespace Y.Api;\n");
    } else {
        fixture.write("Y.Api/Y.Api.csproj", &csproj("../Y.Shared/Y.Shared.csproj"));
        fixture.write(
            "Y.Api/Program.cs",
            "using Microsoft.Extensions.DependencyInjection;\nusing Y.Shared;\nnamespace Y.Api;\npublic static class Program\n{\n    public static void Main()\n    {\n        var services = new ServiceCollection();\n        services.AddScoped<ICache, Cache>();\n    }\n}\n",
        );
    }
    fixture.write("Y.Shared/Y.Shared.csproj", &csproj(""));
    fixture.write(
        "Y.Shared/Shared.cs",
        "namespace Y.Shared;\npublic interface ICache { }\npublic class Cache : ICache { }\n",
    );
    fixture
}

#[test]
fn modules_conflicting_fold_prints_no_marker() {
    let fixture = csharp_fold_fixture(false);
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.lines().any(|line| line.trim() == "Api"),
        "the folded node must still be rendered:\n{out}"
    );
    assert!(
        !out.contains('[') && !out.contains("<<"),
        "a conflicting fold must print no marker syntax (row 35):\n{out}"
    );
    assert!(
        out.contains("Api --> Shared"),
        "the fold's silence must not move an edge:\n{out}"
    );
}

#[test]
fn modules_agreeing_fold_marks_the_node_once() {
    let fixture = csharp_fold_fixture(true);
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    let marks = out.matches("Api[\"Api [facade]\"]").count();
    assert_eq!(
        marks, 1,
        "two agreeing roles paths must mark their folded node exactly once:\n{out}"
    );
    assert!(
        out.contains("Api --> Shared"),
        "edge lines stay byte-identical:\n{out}"
    );
}

/// The zero-theater byte anchor (US 02 TDD step 1): the rust probe tree's
/// model carries facade roles at the unit tier, every one of which folds
/// onto a node this view does not render — so the emitted bytes equal the
/// committed golden exactly, before and after the marker code exists.
#[test]
fn modules_bytes_on_the_rust_probe_tree_match_the_golden_that_carries_no_marks() {
    let fixture = common::Fixture::new();
    let driver = shared::driver::Driver {
        language: shared::driver::Language::Rust,
    };
    driver.materialize(&fixture, &driver.probe_tree());
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let golden = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens/depgraph-modules/rust.mmd"))
    .expect("rust depgraph golden");
    assert_eq!(
        stdout(&output).as_bytes(),
        golden.as_slice(),
        "a tree whose roles fold onto no rendered node must keep the golden bytes"
    );
}

#[test]
fn modules_marked_output_is_byte_identical_across_runs() {
    let fixture = go_fixture();
    let first = stdout(&fixture.run(&["depgraph", "modules"]));
    let second = stdout(&fixture.run(&["depgraph", "modules"]));
    assert_eq!(first, second, "marked depgraph output differs across runs");
}
