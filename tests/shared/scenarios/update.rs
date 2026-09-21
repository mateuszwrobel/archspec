use crate::common;
use crate::shared::driver::{Driver, LogicalTree};
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{csharp_project_files, expect_success, scenario, scenario_when};

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::UpdateSeed,
            "update_creates_spec_at_project_root",
            "update seeds an architecture.spec.toml at the project root",
            update_creates_spec_at_project_root,
        ),
        scenario(
            Feature::UpdateSeed,
            "seed_verifies_clean_against_itself",
            "verify against the just-generated seed reports no violations",
            seed_verifies_clean_against_itself,
        ),
        scenario_when(
            Feature::UpdateSeed,
            "seed_omits_dotnet_test_projects_with_runner_evidence",
            "update omits a dotnet test project proven by runner-package evidence; the packable flag and the name play no part",
            csharp_project_files,
            seed_omits_dotnet_test_projects_with_runner_evidence,
        ),
    ]
}

fn update_creates_spec_at_project_root(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    let output = expect_success(driver, fx, &["update"])?;
    if !fx.path("architecture.spec.toml").exists() {
        return Err("update must create architecture.spec.toml at the project root".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("generated") {
        return Err(format!("update summary must name the generated file:\n{stdout}"));
    }
    Ok(())
}

fn seed_verifies_clean_against_itself(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    expect_success(driver, fx, &["update"])?;
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("matches source model") {
        return Err(format!("seed must verify clean against itself:\n{stdout}"));
    }
    Ok(())
}
/// Workplan csharp_test_projects_tier, scenario "seed omits test projects":
/// the seed's test-tier evidence is dependency evidence — the project's
/// package set naming a runner (xunit here). `IsPackable=false` and the
/// `.Tests` suffix play no part: the tier fact is the same runner SDK/classic
/// runner package ids the scan excludes from every production tier, so the
/// seeded spec declares no boundary for the test project and verifies clean
/// against production boundaries only.
fn seed_omits_dotnet_test_projects_with_runner_evidence(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into(), "app.Tests".into()];
    tree.is_packable.insert("app.Tests".into(), false);
    tree.packages
        .insert("app.Tests".into(), vec!["xunit".into()]);
    driver.materialize(fx, &tree);
    expect_success(driver, fx, &["update"])?;
    let spec = fx.read("architecture.spec.toml");
    if spec.contains("app.Tests") {
        return Err(format!(
            "the seed must declare no boundary for the test project:\n{spec}"
        ));
    }
    if !spec.contains("name = \"app\"") {
        return Err(format!(
            "production boundaries must still be stated as today:\n{spec}"
        ));
    }
    expect_success(driver, fx, &["verify"])?;
    Ok(())
}
