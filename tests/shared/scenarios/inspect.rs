use crate::common;
use crate::shared::driver::{
    materialize_go_declared_grouping, materialize_go_workspace, Driver, Language, LogicalTree,
};
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{expect_success, expect_success_in, mermaid_id, scenario};

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::InspectFileLevel,
            "renders_a_node_per_source_file",
            "inspect renders a node per source file and the resolved file-level edge",
            renders_a_node_per_source_file,
        ),
        scenario(
            Feature::InspectFileLevel,
            "file_level_diagram_is_deterministic",
            "an unchanged tree renders a byte-identical file-level diagram across runs",
            file_level_diagram_is_deterministic,
        ),
        scenario(
            Feature::InspectFileLevel,
            "production_imports_never_target_test_files",
            "no production file-level import targets a test-tier file; test files stay nodes and import sources",
            production_imports_never_target_test_files,
        ),
        scenario(
            Feature::InspectStructuralTree,
            "tree_view_renders_units_and_module_edges",
            "inspect tree renders one subgraph per unit with its module nodes and edges",
            tree_view_renders_units_and_module_edges,
        ),
        scenario(
            Feature::InspectStructuralScanner,
            "scanner_view_draws_unit_edges",
            "inspect scanner draws the cross-unit hard edges between unit subgraphs",
            scanner_view_draws_unit_edges,
        ),
        scenario(
            Feature::InspectStructuralTree,
            "inspect_tree_marks_role_carrying_nodes",
            "every node whose model path carries a role entry renders its visible label suffixed with that role's marker, so why a node looks important reads off the diagram alone",
            inspect_tree_marks_role_carrying_nodes,
        ),
    ]
}

/// Materialize the file-level inspect fixture and return the cwd to run
/// `inspect` from plus the two node names the edge must connect. The Rust
/// materializer nests crates under `app/src`, which the file scanner's
/// crate-root heuristic only understands when run from the crate's src dir;
/// the C# scanner resolves by namespace, so the fixture root works directly;
/// Go materializes its canonical single-module tree at the root.
fn materialize_file_level(driver: &Driver, fx: &common::Fixture) -> (&'static str, String, String) {
    match driver.language {
        Language::Rust => {
            driver.materialize(fx, &soft_module_tree());
            ("app/src", "core.rs".to_string(), "ui.rs".to_string())
        }
        Language::Csharp => {
            driver.materialize(fx, &soft_module_tree());
            (".", "app/core.cs".to_string(), "app/ui.cs".to_string())
        }
        Language::Go => {
            materialize_go_declared_grouping(fx);
            (".", "a/a.go".to_string(), "shell/shell.go".to_string())
        }
    }
}

fn soft_module_tree() -> LogicalTree {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["core".into(), "ui".into()]);
    tree.module_usings
        .push(("app".into(), "core".into(), "ui".into()));
    tree
}

fn renders_a_node_per_source_file(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let (cwd, core, ui) = materialize_file_level(driver, fx);
    let output = expect_success_in(fx, cwd, &["inspect"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("graph TD") {
        return Err(format!("inspect must render a mermaid graph:\n{stdout}"));
    }
    if !stdout.contains(&core) {
        return Err(format!("node {core} missing:\n{stdout}"));
    }
    if !stdout.contains(&ui) {
        return Err(format!("node {ui} missing:\n{stdout}"));
    }
    let edge = format!("{} --> {}", mermaid_id(&core), mermaid_id(&ui));
    if !stdout.contains(&edge) {
        return Err(format!("edge {edge} missing:\n{stdout}"));
    }
    Ok(())
}

fn file_level_diagram_is_deterministic(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let (cwd, _, _) = materialize_file_level(driver, fx);
    let first = expect_success_in(fx, cwd, &["inspect"])?;
    let second = expect_success_in(fx, cwd, &["inspect"])?;
    if common::stdout(&first) != common::stdout(&second) {
        return Err("inspect output differs across runs".into());
    }
    Ok(())
}

fn tree_view_renders_units_and_module_edges(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    // Structural applicability is per model content, not per language: Go
    // reaches the module tier through `go.work` members (2+), so the Go probe
    // of this behavior materializes the workspace shape — where containment
    // renders. A single-module Go tree has no tier and is refused by that
    // fact (pinned in `tests/inspect.rs`), which is why the feature probe
    // materializes the same shape the scenario does.
    if driver.language == Language::Go {
        materialize_go_workspace(fx);
        let output = expect_success(driver, fx, &["inspect", "tree"])?;
        let stdout = common::stdout(&output);
        if !stdout.contains("graph TD") {
            return Err(format!(
                "inspect tree must render a mermaid graph:\n{stdout}"
            ));
        }
        for member in ["example.com/api", "example.com/store"] {
            if !stdout.contains(member) {
                return Err(format!(
                    "member module {member} missing from tree view:\n{stdout}"
                ));
            }
        }
        let edge = format!(
            "{} --> {}",
            mermaid_id("example.com::api"),
            mermaid_id("example.com::store")
        );
        if !stdout.contains(&edge) {
            return Err(format!("module edge {edge} missing:\n{stdout}"));
        }
        return Ok(());
    }
    let tree = soft_module_tree();
    driver.materialize(fx, &tree);
    let output = expect_success(driver, fx, &["inspect", "tree"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("graph TD") {
        return Err(format!(
            "inspect tree must render a mermaid graph:\n{stdout}"
        ));
    }
    let unit = driver.unit_name("app");
    if !stdout.contains(&unit) {
        return Err(format!("unit {unit} missing from tree view:\n{stdout}"));
    }
    let core = driver.module_path("app", "core");
    if !stdout.contains(&core) {
        return Err(format!("module {core} missing from tree view:\n{stdout}"));
    }
    let ui = driver.module_path("app", "ui");
    let edge = format!("{} --> {}", mermaid_id(&core), mermaid_id(&ui));
    if !stdout.contains(&edge) {
        return Err(format!("module edge {edge} missing:\n{stdout}"));
    }
    Ok(())
}

fn scanner_view_draws_unit_edges(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into(), "shared".into()];
    tree.hard_edges.push(("app".into(), "shared".into()));
    driver.materialize(fx, &tree);
    let output = expect_success(driver, fx, &["inspect", "scanner"])?;
    let stdout = common::stdout(&output);
    let from = driver.unit_name("app");
    let to = driver.unit_name("shared");
    if !stdout.contains(&from) {
        return Err(format!("unit {from} missing from scanner view:\n{stdout}"));
    }
    if !stdout.contains(&to) {
        return Err(format!("unit {to} missing from scanner view:\n{stdout}"));
    }
    let edge = format!("{} --> {}", mermaid_id(&from), mermaid_id(&to));
    if !stdout.contains(&edge) {
        return Err(format!("unit edge {edge} missing:\n{stdout}"));
    }
    Ok(())
}

/// The language-neutral edge classifier for the test-tier scenario: an edge
/// line targeting a node is `--> <id>`, and node labels embed the raw
/// relative path, so the scenario asserts through the paths it wrote.
fn expect_node(diagram: &str, path: &str) -> Result<(), String> {
    if !diagram.contains(path) {
        return Err(format!("node {path} missing:\n{diagram}"));
    }
    Ok(())
}

fn expect_edge(diagram: &str, from: &str, to: &str) -> Result<(), String> {
    let edge = format!("{} --> {}", mermaid_id(from), mermaid_id(to));
    if !diagram.contains(&edge) {
        return Err(format!("edge {edge} missing:\n{diagram}"));
    }
    Ok(())
}

fn expect_never_targeted(diagram: &str, target: &str) -> Result<(), String> {
    let targeted = format!("--> {}", mermaid_id(target));
    if diagram.contains(&targeted) {
        return Err(format!(
            "production import targeted the test file {target}:\n{diagram}"
        ));
    }
    Ok(())
}

/// One rule, three honest identities: rust pins its structural cleanliness
/// (test targets live under ownership keys no production `use` probes), go
/// filters `*_test.go` from every target list by file name, c# excludes
/// test-project files by the scan driver's manifest predicate. Each branch
/// writes its adversarial shape on top of the shared base tree, asserts the
/// production fan-out it must keep, the test-file source edges it must keep,
/// and the absence of any production→test edge.
fn production_imports_never_target_test_files(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    match driver.language {
        Language::Rust => rust_imports_never_target_test_targets(driver, fx),
        Language::Csharp => csharp_imports_never_target_test_projects(driver, fx),
        Language::Go => go_imports_never_target_test_files(fx),
    }
}

/// A `tests/` integration target importing through the package name is a
/// source-side fact; an inline `#[cfg(test)]` module lives inside production
/// files that own it. No production path can name a test target's ownership
/// key, so the only assertions are: node present, source edge rendered,
/// never a target.
fn rust_imports_never_target_test_targets(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    driver.materialize(fx, &soft_module_tree());
    fx.write(
        "app/tests/it.rs",
        "use app::core;\n\n#[cfg(test)]\nmod helpers {\n    pub fn helper() {}\n}\n\n#[test]\nfn integration() { helpers::helper(); }\n",
    );
    let output = expect_success_in(fx, "app", &["inspect"])?;
    let diagram = common::stdout(&output);
    expect_node(&diagram, "tests/it.rs")?;
    expect_edge(&diagram, "tests/it.rs", "src/core.rs")?;
    expect_never_targeted(&diagram, "tests/it.rs")
}

/// A white-box test file in a test project (csproj naming a test runner)
/// declares the exact production namespace: same-namespace production files
/// keep being fanned into, the test-project file never is — while its own
/// `using` still renders as a source edge.
fn csharp_imports_never_target_test_projects(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    driver.materialize(fx, &soft_module_tree());
    fx.write("App/App.csproj", &csharp_project(&[]));
    fx.write(
        "App/Store.cs",
        "namespace App.Store;\npublic class Store { }\n",
    );
    fx.write(
        "App/Depot.cs",
        "namespace App.Store;\npublic class Depot { }\n",
    );
    fx.write("Consumer/Consumer.csproj", &csharp_project(&[]));
    fx.write(
        "Consumer/Client.cs",
        "using App.Store;\nnamespace Consumer;\npublic class Client { }\n",
    );
    fx.write("App.Tests/App.Tests.csproj", &csharp_project(&["xunit"]));
    fx.write(
        "App.Tests/SameNsTest.cs",
        "using App.Store;\nnamespace App.Store;\npublic class SameNsTest { }\n",
    );
    let output = expect_success_in(fx, ".", &["inspect"])?;
    let diagram = common::stdout(&output);
    expect_node(&diagram, "App.Tests/SameNsTest.cs")?;
    expect_edge(&diagram, "Consumer/Client.cs", "App/Store.cs")?;
    expect_edge(&diagram, "Consumer/Client.cs", "App/Depot.cs")?;
    expect_edge(&diagram, "App.Tests/SameNsTest.cs", "App/Store.cs")?;
    expect_never_targeted(&diagram, "App.Tests/SameNsTest.cs")
}

/// Both go test shapes sit in an imported package: the production importer
/// fans into the production file only, while each test file's own import of
/// a production package still renders.
fn go_imports_never_target_test_files(fx: &common::Fixture) -> Result<(), String> {
    materialize_go_declared_grouping(fx);
    fx.write(
        "store/store_test.go",
        "package store\n\nimport \"example.com/demo/a\"\n\nfunc TestStore(t *testing.T) { _ = a.A }\n",
    );
    fx.write(
        "store/external_test.go",
        "package store_test\n\nimport \"example.com/demo/a\"\n\nfunc TestExt(t *testing.T) { _ = a.A }\n",
    );
    let output = expect_success_in(fx, ".", &["inspect"])?;
    let diagram = common::stdout(&output);
    expect_edge(&diagram, "shell/shell.go", "store/store.go")?;
    expect_node(&diagram, "store/store_test.go")?;
    expect_node(&diagram, "store/external_test.go")?;
    expect_edge(&diagram, "store/store_test.go", "a/a.go")?;
    expect_edge(&diagram, "store/external_test.go", "a/a.go")?;
    expect_never_targeted(&diagram, "store/store_test.go")?;
    expect_never_targeted(&diagram, "store/external_test.go")
}

/// A minimal SDK-style csproj with the given package references — the shape
/// the c# materializer itself writes, carried here because the adversarial
/// projects are scenario-level additions, not base-tree units.
fn csharp_project(packages: &[&str]) -> String {
    let mut csproj = String::from(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n",
    );
    if !packages.is_empty() {
        csproj.push_str("  <ItemGroup>\n");
        for package in packages {
            csproj.push_str(&format!("    <PackageReference Include=\"{package}\" />\n"));
        }
        csproj.push_str("  </ItemGroup>\n");
    }
    csproj.push_str("</Project>\n");
    csproj
}

/// The tree renders the model's role facts as label markers (roles US 06):
/// every roles entry whose path addresses a node of this tree marks that
/// node's visible label — the marker is a lookup, so a node is special in
/// the diagram exactly when the model says it is, on every driver.
fn inspect_tree_marks_role_carrying_nodes(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let roles = model["roles"]
        .as_object()
        .ok_or_else(|| "the canonical tree derives roles, so the model must serialize a map".to_string())?;
    let output = expect_success(driver, fx, &["inspect", "tree"])?;
    let stdout = common::stdout(&output);
    for (path, role) in roles {
        let role = role.as_str().unwrap_or_default();
        let marked = format!("[\"{path} [{role}]\"]");
        if !stdout.contains(&marked) {
            return Err(format!(
                "the node for {path} must render the marker {marked}:\n{stdout}"
            ));
        }
    }
    Ok(())
}
