use crate::common;
use crate::shared::driver::{Driver, Language, LogicalTree};
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{expect_success, scenario_when, scenario_when_capability};

/// Rust-only: logical `::` module paths materialize as literal file names on
/// the c# materializer, so this nested tree cannot be expressed for c# through
/// the shared harness. The c# driver's real nested-submodule rendering
/// (submodules view expands; modules view folds to a bare node) is pinned
/// against a natural `.csproj` layout in `tests/depgraph.rs`.
/// contract-plan: the materializer will encode nested paths per language so
/// this scenario can run on every soft-tier driver.
fn rust_only(driver: &Driver) -> bool {
    matches!(driver.language, Language::Rust)
}

pub fn all() -> Vec<Scenario> {
    vec![
        scenario_when_capability(
            Feature::Depgraph,
            "modules_projects_top_level_graph",
            "depgraph modules collapses nested module paths to top-level nodes and may draw several (cross product of folded top-nodes)",
            "module-tier",
            modules_projects_top_level_graph,
        ),
        scenario_when_capability(
            Feature::Depgraph,
            "api_usage_view_emits_table_contract",
            "depgraph api-usage emits the fixed-column table or the explicit no-usage statement",
            "module-tier",
            api_usage_view_emits_table_contract,
        ),
        scenario_when(
            Feature::Depgraph,
            "submodules_view_scopes_to_parent",
            "depgraph submodules expands one parent into its children (plus mod) and the edges between them",
            rust_only,
            submodules_view_scopes_to_parent,
        ),
    ]
}

/// The top-level module node a concrete dotted path projects to (segment after
/// the unit), mirroring the command's projection so scenarios stay
/// language-agnostic.
fn top_segment(path: &str) -> String {
    path.split("::").nth(1).unwrap_or("root").to_string()
}

/// The child submodule a concrete dotted path projects to within a parent
/// (segment two past the unit), mirroring the command's submodule projection.
fn child_segment(path: &str) -> String {
    path.split("::").nth(2).unwrap_or("mod").to_string()
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

fn nested_module_tree() -> LogicalTree {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert(
        "app".into(),
        vec!["parent".into(), "parent::a".into(), "parent::b".into()],
    );
    tree.module_usings
        .push(("app".into(), "parent::a".into(), "parent::b".into()));
    tree
}

fn modules_projects_top_level_graph(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    driver.materialize(fx, &soft_module_tree());
    let output = expect_success(driver, fx, &["depgraph", "modules"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("graph TD") {
        return Err(format!("depgraph modules must render a mermaid graph:\n{stdout}"));
    }
    let core = top_segment(&driver.module_path("app", "core"));
    let ui = top_segment(&driver.module_path("app", "ui"));
    let edge = format!("{core} --> {ui}");
    if !stdout.contains(&edge) {
        return Err(format!("expected top-level edge {edge}:\n{stdout}"));
    }
    Ok(())
}

fn api_usage_view_emits_table_contract(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    driver.materialize(fx, &soft_module_tree());
    let output = expect_success(driver, fx, &["depgraph", "api-usage"])?;
    let stdout = common::stdout(&output);
    let has_table = stdout.contains("| Target module | Used by module | APIs used |");
    let has_none = stdout.contains("No internal API usage details found.");
    if !has_table && !has_none {
        return Err(format!("api-usage must emit the fixed-column table or the no-usage statement:\n{stdout}"));
    }
    Ok(())
}

fn submodules_view_scopes_to_parent(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    driver.materialize(fx, &nested_module_tree());
    let output = expect_success(
        driver,
        fx,
        &["depgraph", "submodules", "--parent", "parent"],
    )?;
    let stdout = common::stdout(&output);
    if !stdout.contains("graph TD") {
        return Err(format!("depgraph submodules must render a mermaid graph:\n{stdout}"));
    }
    if !stdout.contains("mod") {
        return Err(format!("submodule view must include the parent's own `mod` node:\n{stdout}"));
    }
    let a = child_segment(&driver.module_path("app", "parent::a"));
    let b = child_segment(&driver.module_path("app", "parent::b"));
    let edge = format!("{a} --> {b}");
    if !stdout.contains(&edge) {
        return Err(format!("expected child edge {edge}:\n{stdout}"));
    }
    Ok(())
}
