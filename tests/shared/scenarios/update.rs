use crate::common;
use crate::shared::driver::{Driver, LogicalTree};
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{
    expect_success, scenario, scenario_when, scenario_when_capability, scenario_when_inert,
    csharp_project_files,
};

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
        scenario_when_capability(
            Feature::UpdateSeed,
            "seed_test_tier_follows_cfg_gating_not_module_name",
            "the rust dialect of the seed test-tier provenance: a #[cfg(test)] module (a cfg on a root declaration, the `root-module-declarations` row this leg is keyed on) seeds no boundary while a module merely named tests seeds one with its production edge",
            "root-module-declarations",
            seed_test_tier_follows_cfg_gating_not_module_name,
        ),
        scenario_when_inert(
            Feature::UpdateSeed,
            "seed_omits_go_test_file_imports_and_marks_no_module",
            "the go dialect of the same provenance: imports carried only by *_test.go files seed no dependency and no module carries a test mark — keyed inert on `test-tier` because the table states go applies the exclusion at the file tier with no serialized mark",
            "test-tier",
            seed_omits_go_test_file_imports_and_marks_no_module,
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
        return Err(format!(
            "update summary must name the generated file:\n{stdout}"
        ));
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
/// `.Tests` suffix play no part: the tier fact is the same runner SDK and
/// test-runner package ids the scan excludes from every production tier, so the
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

/// UPD-2's rust leg (workplan archspec_cvd_precision, US 05): the seed follows
/// the cfg-based rule, not a name-based one. `test_gated` materializes the
/// declaration as `#[cfg(test)] mod tests;` — the cfg rides the root module
/// declaration, which is why this leg keys on the `root-module-declarations`
/// row (the table's feature-gating fact, granular only where a cfg exists to
/// read) rather than a driver list. Mirrors the language-local pins
/// `tests/update.rs::update_seed_{includes,excludes}_*` semantics through the
/// shared tree: the gated arm seeds no boundary for `tests` and keeps the
/// production one; the same tree minus the gate is production by name and
/// seeds its boundary with the real edge.
fn seed_test_tier_follows_cfg_gating_not_module_name(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let prod = driver.module_path("app", "prod");
    let tests = driver.module_path("app", "tests");
    let mut gated = LogicalTree::new();
    gated.units = vec!["app".into()];
    gated
        .modules
        .insert("app".into(), vec!["prod".into(), "tests".into()]);
    gated
        .module_usings
        .push(("app".into(), "tests".into(), "prod".into()));
    gated.test_gated.insert("app".into(), vec!["tests".into()]);
    driver.materialize(fx, &gated);
    expect_success(driver, fx, &["update"])?;
    let spec = fx.read("architecture.spec.toml");
    if !spec.contains(&format!("name = \"{prod}\"")) {
        return Err(format!("the production boundary must still seed:\n{spec}"));
    }
    if spec.contains(&tests) {
        return Err(format!("a cfg(test) tests module must seed no boundary:\n{spec}"));
    }
    expect_success(driver, fx, &["verify"])?;

    // The name-only arm: the same tree with the gate removed. `update`
    // refuses an existing spec, so the seed file goes first — the tree
    // itself materializes identically otherwise.
    std::fs::remove_file(fx.path("architecture.spec.toml")).expect("clear the seed between arms");
    let mut named = gated;
    named.test_gated.clear();
    driver.materialize(fx, &named);
    expect_success(driver, fx, &["update"])?;
    let spec = fx.read("architecture.spec.toml");
    if !spec.contains(&format!("name = \"{tests}\"")) {
        return Err(format!(
            "a production (non-cfg) tests module must seed a boundary:\n{spec}"
        ));
    }
    if !spec.contains(&format!("allowed = {{ depend_on = [\"{prod}\"] }}")) {
        return Err(format!(
            "the tests boundary must record its production dependency:\n{spec}"
        ));
    }
    expect_success(driver, fx, &["verify"])?;
    Ok(())
}

/// UPD-2's go leg (workplan archspec_cvd_precision, US 05): production wires
/// `a -> b` in regular files while `a`'s `*_test.go` imports a package
/// production never touches. The scan drops that file (the `is_test_file`
/// rule behind the `test-tier` row's `file tier only` cell), so the seeded
/// matches and edges name nothing from its imports and no module carries any
/// test mark. Keyed inert on `test-tier`: the leg runs exactly where the table
/// says there is no granular mark to consult. The language-local seed-shape
/// pin stays where it is: `tests/verify.rs::verify_go_cycle_only_through_test_file_is_clean`.
fn seed_omits_go_test_file_imports_and_marks_no_module(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let a = driver.unit_name("a");
    let b = driver.unit_name("b");
    const TEST_IMPORT: &str = "example.com/never/in/production";
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    tree.hard_edges.push(("a".into(), "b".into()));
    tree.module_usings
        .push(("a".into(), "a".into(), TEST_IMPORT.into()));
    tree.test_gated.insert("a".into(), vec!["a".into()]);
    driver.materialize(fx, &tree);
    expect_success(driver, fx, &["update"])?;
    let spec = fx.read("architecture.spec.toml");
    if !spec.contains(&format!("name = \"{a}\"")) || !spec.contains(&format!("name = \"{b}\"")) {
        return Err(format!("both production modules must be declared:\n{spec}"));
    }
    if !spec.contains(&format!("depend_on = [\"{b}\"]")) {
        return Err(format!("the real production edge a -> b must seed:\n{spec}"));
    }
    if spec.contains(TEST_IMPORT) || spec.contains("_test") {
        return Err(format!(
            "nothing a *_test.go file imports may seed, and no module may carry a test mark:\n{spec}"
        ));
    }
    let model = driver.scan(fx);
    if model.to_string().contains(TEST_IMPORT) {
        return Err(format!(
            "the scan model may not even see the test file's import: {model}"
        ));
    }
    expect_success(driver, fx, &["verify"])?;
    Ok(())
}
