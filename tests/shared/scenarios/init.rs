use crate::common;
use crate::shared::driver::Driver;
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{expect_success, scenario};

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::Init,
            "init_creates_spec_declaring_language",
            "init writes an architecture.spec.toml that declares the detected language",
            init_creates_spec_declaring_language,
        ),
        scenario(
            Feature::Init,
            "init_refuses_to_overwrite_existing_spec",
            "init refuses to overwrite an existing architecture.spec.toml",
            init_refuses_to_overwrite_existing_spec,
        ),
    ]
}

fn init_creates_spec_declaring_language(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    let output = expect_success(driver, fx, &["init"])?;
    if !fx.path("architecture.spec.toml").exists() {
        return Err("init must create architecture.spec.toml".into());
    }
    let stdout = common::stdout(&output);
    let expected = format!("language: {}", driver.language.as_str());
    if !stdout.contains(&expected) {
        return Err(format!(
            "init summary must name the detected language ({expected}):\n{stdout}"
        ));
    }
    Ok(())
}

fn init_refuses_to_overwrite_existing_spec(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    fx.write("architecture.spec.toml", "existing content\n");
    let output = driver.run(fx, &["init"]);
    if output.status.code() == Some(0) {
        return Err("init must refuse to overwrite an existing spec".into());
    }
    if !common::stderr(&output).contains("already exists") {
        return Err(format!(
            "init error must mention the existing spec:\n{}",
            common::stderr(&output)
        ));
    }
    if !common::stdout(&output).is_empty() {
        return Err("init must not print to stdout on refusal".into());
    }
    Ok(())
}