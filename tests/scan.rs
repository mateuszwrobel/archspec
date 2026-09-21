mod common;
#[allow(dead_code)]
mod shared;

use common::{stderr, stdout};
use serde_json::Value;

fn workspace_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture
}

#[test]
fn scan_scenario_one_lists_workspace_crates_and_hard_edge() {
    let fixture = workspace_fixture();
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);
    assert!(stderr(&output).is_empty(), "stderr should be empty");

    let model: Value = serde_json::from_str(&out).expect("stdout must be JSON");
    assert_eq!(model["schema_version"].as_u64(), Some(1));
    assert_eq!(model["language"].as_str(), Some("rust"));

    let units = model["units"].as_array().expect("units must be an array");
    let names: Vec<&str> = units
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    assert_eq!(
        names,
        ["auth", "billing"],
        "both crates listed as units, sorted"
    );
    for unit in units {
        assert_eq!(unit["kind"].as_str(), Some("crate"), "unit kind is crate");
    }
    let auth = units
        .iter()
        .find(|unit| unit["name"].as_str() == Some("auth"))
        .expect("auth unit present");
    assert_eq!(auth["path"].as_str(), Some("crates/auth"));
    let billing = units
        .iter()
        .find(|unit| unit["name"].as_str() == Some("billing"))
        .expect("billing unit present");
    assert_eq!(billing["path"].as_str(), Some("crates/billing"));

    let edges = model["edges"].as_array().expect("edges must be an array");
    assert_eq!(edges.len(), 1, "exactly one hard edge");
    assert_eq!(edges[0]["from"].as_str(), Some("billing"));
    assert_eq!(edges[0]["to"].as_str(), Some("auth"));

    assert!(model.get("usage").is_some(), "usage field must be present");
    assert!(
        model.get("soft_structure").is_some(),
        "soft_structure field must be present"
    );
}

#[test]
fn scan_is_deterministic_across_runs() {
    let fixture = workspace_fixture();
    let first = fixture.run(&["scan"]);
    let second = fixture.run(&["scan"]);

    assert_eq!(first.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "scan output must be byte-identical across runs"
    );
}

#[test]
fn scan_output_flag_writes_file_and_keeps_stdout_empty() {
    let fixture = workspace_fixture();
    let output = fixture.run(&["scan", "--output", "model.json"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(
        stdout(&output).is_empty(),
        "stdout must be empty when --output is given"
    );
    let written = fixture.read("model.json");
    let model: Value = serde_json::from_str(&written).expect("file must be JSON");
    assert_eq!(model["language"].as_str(), Some("rust"));
    let edges = model["edges"].as_array().expect("edges must be an array");
    assert_eq!(edges.len(), 1, "hard edge billing -> auth in file");
}

#[test]
fn scan_rejects_missing_path() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["scan", "/no/such/dir"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path does not exist: /no/such/dir"));
    assert!(stdout(&output).is_empty(), "no partial model on error");
}

#[test]
fn scan_rejects_regular_file_path() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\n");
    let output = fixture.run(&["scan", "Cargo.toml"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path is not a directory: Cargo.toml"));
    assert!(stdout(&output).is_empty(), "no partial model on error");
}

#[test]
fn scan_rejects_directory_without_supported_language() {
    let fixture = common::Fixture::new();
    fixture.write("docs/readme.md", "hello");
    let output = fixture.run(&["scan", "docs"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("no supported-language sources found under: docs"));
    assert!(stdout(&output).is_empty(), "no partial model on error");
}

#[test]
fn scan_parse_error_names_failing_manifest_and_writes_no_partial_model() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace\nmembers = [\n");
    let output = fixture.run(&["scan"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("failed to parse source:") && err.contains("Cargo.toml"),
        "stderr must name the failing manifest:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no partial model on error");
}

#[test]
fn scan_parse_error_with_output_flag_creates_no_file() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace\nmembers = [\n");
    let output = fixture.run(&["scan", "--output", "model.json"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("failed to parse source:"));
    assert!(stdout(&output).is_empty(), "stdout should be empty");
    assert!(
        !fixture.path("model.json").exists(),
        "no partial model file on parse error"
    );
}

#[test]
fn scan_rejects_two_positional_paths() {
    let fixture = workspace_fixture();
    let output = fixture.run(&["scan", "crates/auth", "crates/billing"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("expected at most one path argument"));
    assert!(stdout(&output).is_empty(), "no partial model on error");
}

#[test]
fn scan_expands_glob_workspace_members() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "");
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    let names: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|unit| unit["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, ["auth", "billing"], "glob members must expand");
    let edges = model["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "billing -> auth edge via glob members");
}

#[test]
fn scan_resolves_renamed_dependency_to_real_crate() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nmyauth = { path = \"../auth\", package = \"auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "");
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    let edges = model["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "renamed dep must resolve to real crate");
    assert_eq!(edges[0]["from"].as_str(), Some("billing"));
    assert_eq!(edges[0]["to"].as_str(), Some("auth"));
}

#[test]
fn scan_resolves_workspace_dependency_edges() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n\n[workspace.dependencies]\nauth = { path = \"crates/auth\" }\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { workspace = true }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "");
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    let edges = model["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 1, "workspace=true dep must produce edge");
    assert_eq!(edges[0]["from"].as_str(), Some("billing"));
    assert_eq!(edges[0]["to"].as_str(), Some("auth"));
}

// acceptance #20: a member inheriting a dep via `{ workspace = true }` where the
// workspace `[workspace.dependencies]` entry renames to a real member (package =
// "...") resolves the inherited dep to that member unit, not external.
#[test]
fn scan_workspace_resolves_inherited_dep_to_unit() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.dependencies]\nlib-b = { path = \"b\", package = \"b\" }\n",
    );
    fixture.write(
        "a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nlib-b = { workspace = true }\n",
    );
    fixture.write("a/src/lib.rs", "pub fn a() {}\n");
    fixture.write(
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("b/src/lib.rs", "pub fn b() {}\n");
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    let edges = model["edges"].as_array().expect("edges");
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        pairs.contains(&("a", "b")),
        "inherited workspace dep must resolve to member unit edge: {pairs:?}"
    );
    let external: Vec<&str> = model["external"]
        .as_array()
        .expect("external")
        .iter()
        .map(|n| n.as_str().unwrap_or_default())
        .collect();
    assert!(
        !external.contains(&"b") && !external.contains(&"lib_b"),
        "resolved member must not be external: {external:?}"
    );
}

#[test]
fn scan_missing_manifest_names_the_absence() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub fn f() {}\n");
    let output = fixture.run(&["scan"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("no Cargo.toml found under:"),
        "must say no manifest, not a parse failure:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial model on error");
}

fn csharp_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Orders/Orders.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n\
         \x20 <PropertyGroup>\n\
         \x20\x20 <TargetFramework>net8.0</TargetFramework>\n\
         \x20 </PropertyGroup>\n\
         \x20 <ItemGroup>\n\
         \x20\x20 <PackageReference Include=\"Newtonsoft.Json\" Version=\"13.0.3\" />\n\
         \x20\x20 <ProjectReference Include=\"..\\Orders.Abstractions\\Orders.Abstractions.csproj\" />\n\
         \x20 </ItemGroup>\n\
         </Project>\n",
    );
    fixture.write(
        "Orders/Orders.cs",
        "namespace Orders;\npublic class Order {}\n",
    );
    fixture.write(
        "Orders.Abstractions/Orders.Abstractions.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "Orders.Abstractions/IOrdersService.cs",
        "namespace Orders.Abstractions;\npublic interface IOrdersService {}\n",
    );
    fixture
}

#[test]
fn scan_scenario_two_lists_csharp_projects_and_project_reference_edge() {
    let fixture = csharp_fixture();
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);
    assert!(stderr(&output).is_empty(), "stderr should be empty");

    let model: Value = serde_json::from_str(&out).expect("stdout must be JSON");
    assert_eq!(model["schema_version"].as_u64(), Some(1));
    assert_eq!(model["language"].as_str(), Some("csharp"));

    let units = model["units"].as_array().expect("units must be an array");
    let names: Vec<&str> = units
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    assert_eq!(
        names,
        ["Orders", "Orders.Abstractions"],
        "both projects listed as units, sorted"
    );
    for unit in units {
        assert_eq!(
            unit["kind"].as_str(),
            Some("project"),
            "unit kind is project"
        );
    }
    let orders = units
        .iter()
        .find(|unit| unit["name"].as_str() == Some("Orders"))
        .expect("Orders unit present");
    assert_eq!(orders["path"].as_str(), Some("Orders"));
    let abstractions = units
        .iter()
        .find(|unit| unit["name"].as_str() == Some("Orders.Abstractions"))
        .expect("Orders.Abstractions unit present");
    assert_eq!(abstractions["path"].as_str(), Some("Orders.Abstractions"));

    let edges = model["edges"].as_array().expect("edges must be an array");
    assert_eq!(
        edges.len(),
        1,
        "exactly one hard edge (PackageReference ignored)"
    );
    assert_eq!(edges[0]["from"].as_str(), Some("Orders"));
    assert_eq!(edges[0]["to"].as_str(), Some("Orders.Abstractions"));
}

#[test]
fn scan_csharp_is_deterministic_across_runs() {
    let fixture = csharp_fixture();
    let first = fixture.run(&["scan"]);
    let second = fixture.run(&["scan"]);

    assert_eq!(first.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "csharp scan output must be byte-identical across runs"
    );
}

#[test]
fn scan_csharp_without_csproj_emits_single_root_unit() {
    let fixture = common::Fixture::new();
    fixture.write(
        "src/Program.cs",
        "namespace Demo;\npublic static class Program {}\n",
    );
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    assert_eq!(model["language"].as_str(), Some("csharp"));

    let units = model["units"].as_array().expect("units must be an array");
    assert_eq!(units.len(), 1, "one unit for the whole root");
    assert_eq!(
        units[0]["name"].as_str(),
        fixture.root.file_name().and_then(|n| n.to_str()),
        "unit name is the root folder name"
    );
    assert_eq!(units[0]["path"].as_str(), Some("."));
    assert_eq!(units[0]["kind"].as_str(), Some("project"));
    assert!(
        model["edges"].as_array().expect("edges").is_empty(),
        "no edges without project references"
    );
}

#[test]
fn scan_csharp_ignores_self_and_outside_tree_references() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Orders/Orders.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\Orders\\Orders.csproj\" />\n    <ProjectReference Include=\"..\\External\\External.csproj\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "Orders/Orders.cs",
        "namespace Orders;\npublic class Order {}\n",
    );
    fixture.write(
        "Orders.Abstractions/Orders.Abstractions.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "Orders.Abstractions/IOrdersService.cs",
        "namespace Orders.Abstractions;\npublic interface IOrdersService {}\n",
    );
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    let edges = model["edges"].as_array().expect("edges must be an array");
    assert!(
        edges.is_empty(),
        "self-reference and outside-tree references must not produce edges: {edges:?}"
    );
}

#[test]
fn scan_csharp_resolves_absolute_project_reference() {
    let fixture = csharp_fixture();
    // Rewrite Orders.csproj with an absolute Include.
    let abs = fixture
        .path("Orders.Abstractions/Orders.Abstractions.csproj")
        .to_string_lossy()
        .into_owned();
    fixture.write(
        "Orders/Orders.csproj",
        &format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"{abs}\" />\n  </ItemGroup>\n</Project>\n"
        ),
    );
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    let edges = model["edges"].as_array().expect("edges must be an array");
    assert_eq!(edges.len(), 1, "absolute Include must resolve");
    assert_eq!(edges[0]["from"].as_str(), Some("Orders"));
    assert_eq!(edges[0]["to"].as_str(), Some("Orders.Abstractions"));
}

#[test]
fn scan_csharp_projects_under_build_output_directories_are_not_units() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write("App/App.cs", "namespace App;\npublic class App {}\n");
    // Build-output locations: a restore placeholder inside the project's obj/
    // and a generated project in a root-level obj/.
    fixture.write(
        "App/obj/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "App/obj/Generated.cs",
        "namespace App.Generated;\npublic class Generated {}\n",
    );
    fixture.write(
        "obj/Restored.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "obj/Restored.cs",
        "namespace Restored;\npublic class Restored {}\n",
    );
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");

    let units = model["units"].as_array().expect("units must be an array");
    let names: Vec<&str> = units
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    assert_eq!(
        names,
        ["App"],
        "projects under bin/obj must never contribute units"
    );
    for unit in units {
        assert_eq!(
            unit["path"].as_str(),
            Some("App"),
            "no unit may point into build output"
        );
    }
    let modules: Vec<&str> = model["soft_structure"]["App"]
        .as_array()
        .map(|arr| arr.iter().map(|v| v.as_str().unwrap_or_default()).collect())
        .unwrap_or_default();
    assert_eq!(
        modules,
        ["App"],
        "sources under obj/ must appear nowhere in the soft tier"
    );
    assert!(
        model["soft_structure"]
            .as_object()
            .expect("soft_structure must be an object")
            .keys()
            .all(|unit| unit == "App"),
        "excluded projects must own no soft tier"
    );
}

#[test]
fn scan_csharp_projects_under_dot_directories_are_not_units() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write("App/App.cs", "namespace App;\npublic class App {}\n");
    fixture.write(
        ".templates/TemplateEngine.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        ".templates/Template.cs",
        "namespace TemplateEngine;\npublic class Template {}\n",
    );
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");

    let units = model["units"].as_array().expect("units must be an array");
    let names: Vec<&str> = units
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    assert_eq!(
        names,
        ["App"],
        "projects under dot-directories must never contribute units"
    );
    assert!(
        model["soft_structure"]
            .as_object()
            .expect("soft_structure must be an object")
            .keys()
            .all(|unit| unit == "App"),
        "dot-directory project must own no soft tier"
    );
}

#[test]
fn scan_scenario_three_lists_go_packages_and_internal_structure() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "cmd/app/main.go",
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
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);
    assert!(stderr(&output).is_empty(), "stderr should be empty");

    let model: Value = serde_json::from_str(&out).expect("stdout must be JSON");
    assert_eq!(model["language"].as_str(), Some("go"));

    let units = model["units"].as_array().expect("units must be an array");
    let names: Vec<&str> = units
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    assert_eq!(
        names,
        [
            "example.com/demo/cmd/app",
            "example.com/demo/internal/service",
            "example.com/demo/internal/store"
        ],
        "all packages listed as units, sorted"
    );
    for unit in units {
        assert_eq!(
            unit["kind"].as_str(),
            Some("package"),
            "unit kind is package"
        );
    }

    let edges = model["edges"].as_array().expect("edges must be an array");
    let edge_pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().unwrap_or_default(),
                edge["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        edge_pairs.contains(&(
            "example.com/demo/cmd/app",
            "example.com/demo/internal/service"
        )),
        "cmd/app -> internal/service edge missing: {edge_pairs:?}"
    );
    assert!(
        edge_pairs.contains(&(
            "example.com/demo/internal/service",
            "example.com/demo/internal/store"
        )),
        "service -> store edge missing: {edge_pairs:?}"
    );
    assert_eq!(edges.len(), 2, "no other edges (external imports excluded)");
}

#[test]
fn scan_go_is_deterministic_across_runs() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "internal/a/a.go",
        "package a\n\nimport \"example.com/demo/internal/b\"\n\nfunc A() {}\n",
    );
    fixture.write("internal/b/b.go", "package b\n\nfunc B() {}\n");
    let first = fixture.run(&["scan"]);
    let second = fixture.run(&["scan"]);

    assert_eq!(first.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "go scan output must be byte-identical across runs"
    );
}

#[test]
fn scan_go_module_root_package_and_import_variants() {
    let fixture = common::Fixture::new();
    fixture.write(
        "go.mod",
        "module example.com/demo // internal demo\ngo 1.21\n",
    );
    fixture.write(
        "main.go",
        "package main\n\nimport (\n\t_ \"example.com/demo/internal/store\"\n\tf \"example.com/demo/internal/service\"\n\t. \"example.com/demo/internal/util\"\n)\n\nfunc main() {}\n",
    );
    fixture.write(
        "internal/service/service.go",
        "package service\n\nfunc Do() {}\n",
    );
    fixture.write(
        "internal/store/store.go",
        "package store\n\nfunc Get() {}\n",
    );
    fixture.write("internal/util/util.go", "package util\n\nfunc U() {}\n");
    fixture.write(
        "internal/comment/comment.go",
        "package comment\n\n// import \"example.com/demo/internal/store\"\nfunc C() {}\n",
    );
    fixture.write("testdata/fix/fix.go", "package fix\n\nfunc F() {}\n");
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);
    assert!(stderr(&output).is_empty(), "stderr should be empty");

    let model: Value = serde_json::from_str(&out).expect("stdout must be JSON");
    let units = model["units"].as_array().expect("units must be an array");
    let names: Vec<&str> = units
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    // Root package present; testdata/ excluded; comment package present (no bogus edge).
    assert!(
        names.contains(&"example.com/demo"),
        "module-root package missing: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n.contains("testdata")),
        "testdata dirs must be excluded: {names:?}"
    );
    assert!(
        names.contains(&"example.com/demo/internal/comment"),
        "comment package missing: {names:?}"
    );
    // Module path must not include the trailing comment.
    assert!(
        names.contains(&"example.com/demo/internal/store"),
        "module path must have trailing comment stripped: {names:?}"
    );

    let edges = model["edges"].as_array().expect("edges must be an array");
    let edge_pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().unwrap_or_default(),
                edge["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    // Blank, aliased, and dot imports all produce edges.
    assert!(
        edge_pairs.contains(&("example.com/demo", "example.com/demo/internal/store")),
        "blank import edge missing: {edge_pairs:?}"
    );
    assert!(
        edge_pairs.contains(&("example.com/demo", "example.com/demo/internal/service")),
        "aliased import edge missing: {edge_pairs:?}"
    );
    assert!(
        edge_pairs.contains(&("example.com/demo", "example.com/demo/internal/util")),
        "dot import edge missing: {edge_pairs:?}"
    );
    // Commented-out import must NOT produce an edge.
    assert!(
        !edge_pairs.contains(&(
            "example.com/demo/internal/comment",
            "example.com/demo/internal/store"
        )),
        "comment import must not produce an edge: {edge_pairs:?}"
    );
}

#[test]
fn scan_go_workspace_lists_members_and_populates_module_tier() {
    let fixture = common::Fixture::new();
    shared::driver::materialize_go_workspace(&fixture);
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");

    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    assert_eq!(model["language"].as_str(), Some("go"));

    let units = model["units"].as_array().expect("units must be an array");
    let names: Vec<&str> = units
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    assert_eq!(
        names,
        [
            "example.com/api",
            "example.com/api/internal/handler",
            "example.com/store"
        ],
        "every member package is a unit, named by its member module path, sorted"
    );
    for unit in units {
        assert_eq!(
            unit["kind"].as_str(),
            Some("package"),
            "unit kind is package"
        );
    }

    let edges = model["edges"].as_array().expect("edges must be an array");
    let edge_pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().unwrap_or_default(),
                edge["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        edge_pairs.contains(&("example.com/api", "example.com/store")),
        "cross-member import must be an edge: {edge_pairs:?}"
    );
    assert!(
        edge_pairs.contains(&(
            "example.com/api/internal/handler",
            "example.com/store"
        )),
        "nested cross-member import must be an edge: {edge_pairs:?}"
    );
    assert_eq!(edges.len(), 2, "no other edges");

    assert_eq!(
        model["soft_structure"],
        serde_json::json!({
            "example.com/api": ["example.com::api", "example.com::api::internal::handler"],
            "example.com/store": ["example.com::store"],
        }),
        "module tier: member modules keyed by module path, members are their packages"
    );
    assert_eq!(
        model["module_edges"],
        serde_json::json!([
            {
                "unit": "example.com/api",
                "from": "example.com::api",
                "to": "example.com::store",
                "symbols": []
            },
            {
                "unit": "example.com/api",
                "from": "example.com::api::internal::handler",
                "to": "example.com::store",
                "symbols": []
            },
        ]),
        "module edges from real cross-member imports"
    );
    assert_eq!(
        model["external"],
        serde_json::json!([]),
        "member modules are internal, stdlib is not external"
    );
}

/// A cross-member import to a member SUBPATH (not the member's root package)
/// is still a module fact: the unit edge names the package and the module
/// edge carries the dotted package path, which the `::` projections fold onto
/// the member node (moda -> modb). Silence here would be fact loss.
#[test]
fn scan_go_workspace_cross_member_import_to_member_subpath() {
    let fixture = common::Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./moda\n\t./modb\n)\n");
    fixture.write("moda/go.mod", "module example.com/moda\ngo 1.21\n");
    fixture.write(
        "moda/moda.go",
        "package moda\n\nimport \"example.com/modb/internal/x\"\n\nfunc A() { x.X() }\n",
    );
    fixture.write("modb/go.mod", "module example.com/modb\ngo 1.21\n");
    fixture.write(
        "modb/internal/x/x.go",
        "package x\n\nfunc X() {}\n",
    );
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");

    let edge_pairs: Vec<(&str, &str)> = model["edges"]
        .as_array()
        .expect("edges must be an array")
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().unwrap_or_default(),
                edge["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        edge_pairs.contains(&("example.com/moda", "example.com/modb/internal/x")),
        "cross-member import to a member subpath must be a unit edge: {edge_pairs:?}"
    );
    assert_eq!(
        model["module_edges"],
        serde_json::json!([
            {
                "unit": "example.com/moda",
                "from": "example.com::moda",
                "to": "example.com::modb::internal::x",
                "symbols": []
            },
        ]),
        "cross-member import to a member subpath must fold to a module edge (moda -> modb)"
    );
    assert_eq!(
        model["external"],
        serde_json::json!([]),
        "a member subpath import is internal, not external"
    );
}

#[test]
fn scan_go_single_module_keeps_module_tier_empty() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "main.go",
        "package main\n\nimport \"example.com/demo/internal/store\"\n\nfunc main() {}\n",
    );
    fixture.write("internal/store/store.go", "package store\n\nfunc Get() {}\n");
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");

    assert_eq!(
        model["soft_structure"],
        serde_json::json!({}),
        "no go.work: the module tier stays empty exactly as before"
    );
    assert_eq!(
        model["module_edges"],
        serde_json::json!([]),
        "no go.work: no module edges"
    );
}

/// A `go.work` naming a single member is the single-module path: fewer than
/// two members derive no module tier (the capability table's go module-tier
/// row is exactly this condition).
#[test]
fn scan_go_single_member_work_takes_single_module_path() {
    let fixture = common::Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse .\n");
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "main.go",
        "package main\n\nimport \"example.com/demo/internal/store\"\n\nfunc main() {}\n",
    );
    fixture.write("internal/store/store.go", "package store\n\nfunc Get() {}\n");
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let model: Value = serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");

    assert_eq!(
        model["soft_structure"],
        serde_json::json!({}),
        "one-member go.work: the module tier stays absent as on a plain single module"
    );
    assert_eq!(
        model["module_edges"],
        serde_json::json!([]),
        "one-member go.work: no module edges"
    );
}

#[test]
fn scan_csharp_parse_error_names_csproj() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Orders/Orders.csproj",
        "<Project><ItemGroup><ProjectReference Include=\"broken",
    );
    fixture.write("Orders/Orders.cs", "namespace Orders;\n");
    let output = fixture.run(&["scan"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("Orders.csproj"),
        "must name the failing csproj:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial model on error");
}

#[test]
fn scan_detected_language_without_driver_reports_doctor() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    // Simulate a machine without the go toolchain: acceptance #11 describes a
    // controlled environment, so the driver availability is env-controlled.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_archspec"))
        .arg("scan")
        .current_dir(&fixture.root)
        .env("ARCHSPEC_DISABLE_DRIVERS", "go")
        .output()
        .expect("run archspec");

    assert_ne!(output.status.code(), Some(0));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains(
            "detected language 'go' but the go driver is not available (run 'archspec doctor')"
        ),
        "stderr must name the language and point at doctor:\n{err}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).is_empty(),
        "no partial model on error"
    );
}

#[test]
fn scan_rust_source_parse_error_names_file() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "fn broken( {\n");
    let output = fixture.run(&["scan"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("failed to parse source:") && err.contains("src/lib.rs"),
        "must name the failing source file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no partial model on error");
}

/// Workplan go_test_files_tier, scenario "test-only external import is not
/// production external usage": an external module imported ONLY from
/// `main_test.go` must not appear in `external` or `module_external`. This
/// mirrors the rust driver's cfg(test) exclusion contract at the file tier.
#[test]
fn scan_go_test_only_external_import_is_not_production_external_usage() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("main.go", "package main\n\nfunc main() {}\n");
    fixture.write(
        "main_test.go",
        "package main\n\nimport (\n\t\"testing\"\n\t\"github.com/stretchr/testify/assert\"\n)\n\nfunc TestNothing(t *testing.T) {}\n",
    );
    let output = fixture.run(&["scan"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value =
        serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");
    assert_eq!(
        model["external"],
        Value::Array(Vec::new()),
        "test-only external import must not reach the production external tier:\n{}",
        stdout(&output)
    );
    assert_eq!(
        model["module_external"],
        Value::Object(serde_json::Map::new()),
        "test-only external import must not reach module_external:\n{}",
        stdout(&output)
    );
}

/// Workplan go_test_files_tier, scenario "external test package belongs to no
/// production unit": a `package foo_test` file with third-party and intra-module
/// imports must leave foo's model identical to the same tree without the test
/// file — same units, same edges, no new externals.
#[test]
fn scan_go_external_test_package_leaves_production_model_unchanged() {
    let with_test = common::Fixture::new();
    with_test.write("go.mod", "module example.com/demo\ngo 1.21\n");
    with_test.write(
        "foo/foo.go",
        "package foo\n\nimport \"example.com/demo/bar\"\n\nfunc F() {}\n",
    );
    with_test.write(
        "foo/x_test.go",
        "package foo_test\n\nimport (\n\t\"testing\"\n\t\"github.com/x/third\"\n\t\"example.com/demo/baz\"\n)\n\nfunc TestF(t *testing.T) {}\n",
    );
    with_test.write("bar/bar.go", "package bar\n\nfunc B() {}\n");
    with_test.write("baz/baz.go", "package baz\n\nfunc Z() {}\n");

    let without_test = common::Fixture::new();
    without_test.write("go.mod", "module example.com/demo\ngo 1.21\n");
    without_test.write(
        "foo/foo.go",
        "package foo\n\nimport \"example.com/demo/bar\"\n\nfunc F() {}\n",
    );
    without_test.write("bar/bar.go", "package bar\n\nfunc B() {}\n");
    without_test.write("baz/baz.go", "package baz\n\nfunc Z() {}\n");

    let first = with_test.run(&["scan"]);
    let second = without_test.run(&["scan"]);
    assert_eq!(first.status.code(), Some(0), "with-test scan must exit 0");
    assert_eq!(
        second.status.code(),
        Some(0),
        "without-test scan must exit 0"
    );

    let model: Value =
        serde_json::from_str(&stdout(&first)).expect("with-test stdout must be JSON");
    let edges: Vec<(&str, &str)> = model["edges"]
        .as_array()
        .expect("edges must be an array")
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().unwrap_or_default(),
                edge["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        edges,
        [(
            "example.com/demo/foo",
            "example.com/demo/bar"
        )],
        "foo's production edges must stay exactly the production-file imports:\n{edges:?}"
    );
    assert_eq!(
        model["external"],
        Value::Array(Vec::new()),
        "`package foo_test` external imports are not production external usage"
    );
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "model of a tree with a `package foo_test` file must equal the same tree without it"
    );
}

/// Workplan go_test_files_tier, scenario "test-gating evidence visible to spec
/// tooling as in Rust": the visibility contract is the rust driver's — the
/// gating fact is model-internal (the rust `test_gated_modules` field is not
/// serialized either), so a scanned tree with test files must emit no test-tier
/// fields and no test-file paths in the model, and keep the exact same
/// serialized shape as a test-free tree.
#[test]
fn scan_go_test_files_add_no_serialized_test_tier_fields() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "foo/foo.go",
        "package foo\n\nimport \"github.com/x/third\"\n\nfunc F() {}\n",
    );
    fixture.write(
        "foo/x_test.go",
        "package foo\n\nimport (\n\t\"testing\"\n\t\"github.com/y/testonly\"\n)\n\nfunc TestF(t *testing.T) {}\n",
    );
    let output = fixture.run(&["scan"]);
    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);

    let model: Value = serde_json::from_str(&out).expect("stdout must be JSON");
    let keys: Vec<&str> = model
        .as_object()
        .expect("model must be an object")
        .keys()
        .map(|key| key.as_str())
        .collect();
    assert!(
        !keys.iter().any(|key| key.contains("test")),
        "no test-tier field may join the serialized model shape: {keys:?}"
    );
    assert!(
        !out.contains("test_gated") && !out.contains("_test.go"),
        "test-gating is a model-internal fact (rust precedent: test_gated_modules \
         is not serialized): no report noise:\n{out}"
    );
    assert_eq!(
        model["external"],
        Value::Array(vec![serde_json::json!("github.com/x/third")]),
        "production external usage from non-test files must stay untouched"
    );
}

/// Workplan go_test_files_tier, scenario "production behavior unchanged for
/// test-free trees": the model of the existing test-free go tree (scan scenario
/// three fixture) is pinned so the file-tier change cannot silently shift
/// units, edges, or external tiers.
#[test]
fn scan_go_test_free_tree_model_is_byte_identical_to_before() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "cmd/app/main.go",
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
    let output = fixture.run(&["scan"]);
    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let model: Value =
        serde_json::from_str(&stdout(&output)).expect("stdout must be JSON");

    let expected = serde_json::json!({
        "units": [
            {"name": "example.com/demo/cmd/app", "kind": "package", "path": "cmd/app"},
            {"name": "example.com/demo/internal/service", "kind": "package", "path": "internal/service"},
            {"name": "example.com/demo/internal/store", "kind": "package", "path": "internal/store"}
        ],
        "edges": [
            {"from": "example.com/demo/cmd/app", "to": "example.com/demo/internal/service"},
            {"from": "example.com/demo/internal/service", "to": "example.com/demo/internal/store"}
        ],
        "external": [],
        "module_external": {},
        "soft_structure": {},
        "module_edges": [],
        "usage": {}
    });
    for key in expected.as_object().expect("expected must be object").keys() {
        assert_eq!(
            model[key], expected[key],
            "test-free go model must stay byte-identical in tier `{key}`:\n{}",
            stdout(&output)
        );
    }
}
