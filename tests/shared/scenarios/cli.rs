use crate::common;
use crate::shared::driver::Driver;
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{expect_success, scenario};

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::CliArtefactFreshness,
            "scan_check_passes_on_fresh_artefact",
            "`scan --output` then `scan --output --check` exits 0 on the untouched file",
            scan_check_passes_on_fresh_artefact,
        ),
        scenario(
            Feature::CliArtefactFreshness,
            "scan_check_fails_on_stale_artefact",
            "a modified artefact makes `scan --check` exit non-zero with the out-of-date finding",
            scan_check_fails_on_stale_artefact,
        ),
    ]
}

fn scan_check_passes_on_fresh_artefact(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    driver.materialize(fx, &driver.probe_tree());
    expect_success(driver, fx, &["scan", "--output", "model.json"])?;
    let check = expect_success(driver, fx, &["scan", "--check", "--output", "model.json"])?;
    let stdout = common::stdout(&check);
    if stdout != "ok: model.json up to date\n" {
        return Err(format!("fresh check must print exactly the ok-line:\n{stdout}"));
    }
    Ok(())
}

fn scan_check_fails_on_stale_artefact(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    driver.materialize(fx, &driver.probe_tree());
    expect_success(driver, fx, &["scan", "--output", "model.json"])?;
    fx.write("model.json", "STALE ON PURPOSE\n");
    let check = driver.run(fx, &["scan", "--check", "--output", "model.json"]);
    if check.status.code() == Some(0) {
        return Err("a stale artefact must fail --check".into());
    }
    let stderr = common::stderr(&check);
    if !stderr.contains("out of date model") || !stderr.contains("differs from generated output") {
        return Err(format!(
            "report must carry the out-of-date model finding:\n{stderr}"
        ));
    }
    if fx.read("model.json") != "STALE ON PURPOSE\n" {
        return Err("--check must not overwrite the artefact".into());
    }
    Ok(())
}
