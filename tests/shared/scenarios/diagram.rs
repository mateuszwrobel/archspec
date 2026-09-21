use crate::common;
use crate::shared::driver::Driver;
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{expect_success, scenario};

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