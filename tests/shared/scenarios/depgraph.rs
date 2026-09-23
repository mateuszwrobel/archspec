use crate::common;
use crate::shared::driver::{
    materialize_go_workspace, Driver, Language, LogicalTree,
};
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{
    expect_success, mermaid_node_markers, scenario_when, scenario_when_capability,
};
use serde_json::Value;

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
        scenario_when_capability(
            Feature::Depgraph,
            "modules_mark_folded_role_carriers",
            "depgraph modules marks a rendered node exactly when the roles keys folded through the view's own projection state one distinct role there, so no marker appears without an entry and no entry the view can honestly name is dropped",
            "module-tier",
            modules_mark_folded_role_carriers,
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
    if driver.language == Language::Go {
        // A single-module go tree folds the whole graph onto one node: the
        // modules view projects one node per module and both endpoints of an
        // in-module edge share their module. A two-member go.work is the
        // shape that renders an inter-module edge: member api imports member
        // store, so the modules view draws api --> store.
        materialize_go_workspace(fx);
        let output = expect_success(driver, fx, &["depgraph", "modules"])?;
        let stdout = common::stdout(&output);
        if !stdout.contains("graph TD") {
            return Err(format!(
                "depgraph modules must render a mermaid graph:\n{stdout}"
            ));
        }
        let api = top_segment("example.com::api");
        let store = top_segment("example.com::store");
        let edge = format!("{api} --> {store}");
        if !stdout.contains(&edge) {
            return Err(format!("expected top-level edge {edge}:\n{stdout}"));
        }
        return Ok(());
    }
    driver.materialize(fx, &soft_module_tree());
    let output = expect_success(driver, fx, &["depgraph", "modules"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("graph TD") {
        return Err(format!(
            "depgraph modules must render a mermaid graph:\n{stdout}"
        ));
    }
    let core = top_segment(&driver.module_path("app", "core"));
    let ui = top_segment(&driver.module_path("app", "ui"));
    let edge = format!("{core} --> {ui}");
    if !stdout.contains(&edge) {
        return Err(format!("expected top-level edge {edge}:\n{stdout}"));
    }
    Ok(())
}

fn api_usage_view_emits_table_contract(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    driver.materialize(fx, &soft_module_tree());
    let output = expect_success(driver, fx, &["depgraph", "api-usage"])?;
    let stdout = common::stdout(&output);
    let has_table = stdout.contains("| Target module | Used by module | APIs used |");
    let has_none = stdout.contains("No internal API usage details found.");
    if !has_table && !has_none {
        return Err(format!(
            "api-usage must emit the fixed-column table or the no-usage statement:\n{stdout}"
        ));
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
        return Err(format!(
            "depgraph submodules must render a mermaid graph:\n{stdout}"
        ));
    }
    if !stdout.contains("mod") {
        return Err(format!(
            "submodule view must include the parent's own `mod` node:\n{stdout}"
        ));
    }
    let a = child_segment(&driver.module_path("app", "parent::a"));
    let b = child_segment(&driver.module_path("app", "parent::b"));
    let edge = format!("{a} --> {b}");
    if !stdout.contains(&edge) {
        return Err(format!("expected child edge {edge}:\n{stdout}"));
    }
    Ok(())
}

/// The node names one model projects through a tier function (soft paths and
/// module-edge endpoints, the two sources that decide the view's node set;
/// unit edges add only pairs of nodes those paths already named).
fn projected_nodes(model: &Value, deep: bool) -> std::collections::BTreeSet<String> {
    let mut nodes = std::collections::BTreeSet::new();
    if let Some(map) = model["soft_structure"].as_object() {
        for paths in map.values() {
            for path in paths.as_array().into_iter().flatten() {
                if let Some(path) = path.as_str() {
                    nodes.insert(project_path(path, deep));
                }
            }
        }
    }
    if let Some(edges) = model["module_edges"].as_array() {
        for edge in edges {
            for key in ["from", "to"] {
                if let Some(path) = edge[key].as_str() {
                    nodes.insert(project_path(path, deep));
                }
            }
        }
    }
    nodes
}

/// Project one model path the way the modules view projects at one tier: the
/// segment right after the unit at the top tier, the path's own final segment
/// one level below the fold, and `root` for a unit-root path at either tier
/// (the view's `top_node` / `deep_node`).
fn project_path(path: &str, deep: bool) -> String {
    let segments: Vec<&str> = path.split("::").collect();
    if segments.len() <= 1 {
        "root".to_string()
    } else if deep {
        segments[segments.len() - 1].to_string()
    } else {
        segments[1].to_string()
    }
}

/// The view's granularity decision, read from the model rather than the
/// language: deep when the top-tier projection trivially folds — one node
/// while the tier holds more distinct paths below the fold (mirrors
/// `projection_node`).
fn projects_deep(model: &Value) -> bool {
    let top = projected_nodes(model, false);
    if top.len() > 1 {
        return false;
    }
    projected_nodes(model, true).len() > top.len()
}

/// The bidirectional roles-in-views rule for `depgraph modules`
/// (roles-views US 02 markers, US 03 pin, acceptance rows 34–37): every roles key is projected the
/// way the view projects (separator lift, then the view's own tier function),
/// roles landing on one node fold to that node's marker exactly when they
/// agree, and the marker shows exactly where the fold names a rendered node
/// with one honest role. No marker without an entry, no entry silently dropped
/// where the view can name it, and a conflict-silenced or unrendered
/// projection stays unmarked — the whole rule read from the model, so a driver
/// keying below this view's vocabulary (rust/c# unit-tier keys folding to the
/// unrendered `root`) is an honest absence, not a skip or a red suite.
fn modules_mark_folded_role_carriers(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let output = expect_success(driver, fx, &["depgraph", "modules"])?;
    let stdout = common::stdout(&output);
    let rendered = mermaid_node_markers(&stdout)?;
    if rendered.is_empty() {
        return Err(format!("the modules view renders no node declarations:\n{stdout}"));
    }
    let roles = model["roles"]
        .as_object()
        .ok_or_else(|| "the canonical tree derives roles, so the model must serialize a map".to_string())?;
    let deep = projects_deep(&model);
    // The view's fold, mirrored: lift driver-native separators to the model's
    // `::` grammar, project, and collect one honest role per node — a node
    // onto which two different roles fold stays silent.
    let mut folded: std::collections::BTreeMap<String, Option<String>> = std::collections::BTreeMap::new();
    for (key, role) in roles {
        let Some(role) = role.as_str() else {
            continue;
        };
        let name = project_path(&key.replace('/', "::"), deep);
        let next = match folded.get(&name) {
            None => Some(role.to_string()),
            Some(Some(existing)) if existing == role => Some(role.to_string()),
            Some(_) => None,
        };
        folded.insert(name, next);
    }
    for (node, marker) in &rendered {
        let expected = folded.get(node).cloned().unwrap_or(None);
        if marker == &expected {
            continue;
        }
        return Err(match (&expected, marker) {
            (Some(role), None) => format!(
                "the roles fold names {node} as {role}, yet the node renders unmarked:\n{stdout}"
            ),
            (None, Some(role)) => format!(
                "the node {node} carries marker {role} that no roles entry folds onto it:\n{stdout}"
            ),
            (Some(want), Some(got)) => format!(
                "the node {node} carries marker {got} but the roles fold states {want}:\n{stdout}"
            ),
            (None, None) => unreachable!("equal markers never reach this branch"),
        });
    }
    Ok(())
}
