use crate::common;
use crate::shared::driver::Driver;
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{
    expect_success, mermaid_node_markers, scenario, scenario_when_capability,
};

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::Diagram,
            "renders_a_node_per_declared_module_and_edge",
            "diagram renders a node per declared module and the declared dependency edge",
            renders_a_node_per_declared_module_and_edge,
        ),
        scenario(
            Feature::Diagram,
            "marks_forbidden_edge_dashed",
            "diagram renders a forbidden edge dashed and keeps allowed edges solid",
            marks_forbidden_edge_dashed,
        ),
        scenario_when_capability(
            Feature::Diagram,
            "scan_mode_marks_role_carriers",
            "diagram --source scan marks exactly the nodes the model's roles map addresses under the `.`/`::` separator equivalence, and nothing else, so a node looks special in the diagram exactly when the model says it is",
            "module-tier",
            scan_mode_marks_role_carriers,
        ),
    ]
}

fn renders_a_node_per_declared_module_and_edge(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"Billing\"\nmatches = {{ units = [\"Billing*\"] }}\n\
             [module.allowed]\ndepend_on = [\"Shared\"]\n\n\
             [[module]]\nname = \"Shared\"\nmatches = {{ units = [\"Shared*\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["diagram"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("graph TD") {
        return Err(format!("diagram must render a mermaid graph:\n{stdout}"));
    }
    if !stdout.contains("  Billing") {
        return Err(format!("module Billing missing:\n{stdout}"));
    }
    if !stdout.contains("  Shared") {
        return Err(format!("module Shared missing:\n{stdout}"));
    }
    if !stdout.contains("  Billing --> Shared") {
        return Err(format!("edge Billing --> Shared missing:\n{stdout}"));
    }
    Ok(())
}

fn marks_forbidden_edge_dashed(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"Billing\"\nmatches = {{ units = [\"Billing*\"] }}\n\
             [module.allowed]\ndepend_on = [\"Shared\"]\nforbidden = [\"Portal\"]\n\n\
             [[module]]\nname = \"Portal\"\nmatches = {{ units = [\"Portal*\"] }}\n\n\
             [[module]]\nname = \"Shared\"\nmatches = {{ units = [\"Shared*\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["diagram"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("  Billing -.->|forbidden| Portal") {
        return Err(format!("forbidden edge must be dashed:\n{stdout}"));
    }
    if !stdout.contains("  Billing --> Shared") {
        return Err(format!("allowed edge must stay solid:\n{stdout}"));
    }
    Ok(())
}

/// The scan-mode lookup's separator equivalence: exact identity under `::` ↔
/// `.`, the same tolerance `depgraph::parent_matches` applies — every driver
/// keys its roles at the unit tier whose spelling equals the unit name modulo
/// separators (plan Assumptions), so this scenario needs no language branch.
fn separated(name: &str) -> String {
    name.replace("::", ".")
}

/// The bidirectional roles-in-views rule for `diagram --source scan`
/// (roles-views US 03, acceptance rows 28–30): every roles entry addressing a node of the scan graph
/// marks that node, and every marked node is addressed by an entry — a view
/// can neither mark what the model omits nor omit what the model states at its
/// vocabulary tier. Written from the model (the scan artefact): where a driver
/// keys below the diagram's unit vocabulary (a rust bin composition
/// `<unit>::main`), no entry addresses the node, the node honestly stays
/// unmarked, and the scenario records that absence as a vocabulary fact rather
/// than skipping — the expectation of no marker comes from the absence of an
/// addressing entry, never from a per-language list.
fn scan_mode_marks_role_carriers(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    fx.write("model.json", &model.to_string());
    let output = expect_success(driver, fx, &["diagram", "--source", "scan", "model.json"])?;
    let stdout = common::stdout(&output);
    let marked = mermaid_node_markers(&stdout)?;
    if marked.is_empty() {
        return Err(format!("the scan diagram renders no node declarations:\n{stdout}"));
    }
    let roles = model["roles"]
        .as_object()
        .ok_or_else(|| "the canonical tree derives roles, so the model must serialize a map".to_string())?;
    for (name, marker) in &marked {
        // The view's lookup mirrored: the first entry in map order addressing
        // this node (roles keys iterate sorted, as the renderer's map does).
        let expected = roles
            .iter()
            .find(|(key, _)| separated(key) == separated(name))
            .and_then(|(_, role)| role.as_str())
            .map(|role| role.to_string());
        if marker == &expected {
            continue;
        }
        return Err(match (&expected, marker) {
            (Some(role), None) => format!(
                "the roles map addresses {name} with {role}, yet the node renders unmarked:\n{stdout}"
            ),
            (None, Some(role)) => format!(
                "the node {name} carries marker {role} that no roles entry states:\n{stdout}"
            ),
            (Some(want), Some(got)) => format!(
                "the node {name} carries marker {got} but the roles map states {want}:\n{stdout}"
            ),
            (None, None) => unreachable!("equal markers never reach this branch"),
        });
    }
    Ok(())
}
