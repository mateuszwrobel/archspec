use crate::common;
use crate::shared::driver::Driver;
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{expect_success, scenario};

pub fn all() -> Vec<Scenario> {
    vec![scenario(
        Feature::HelpDiagnostics,
        "prints_catalog_with_facade_and_laundered_entries",
        "`help diagnostics` exits 0 and quotes the facade and laundered-edge catalog entries",
        prints_catalog_with_facade_and_laundered_entries,
    )]
}

fn prints_catalog_with_facade_and_laundered_entries(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let output = expect_success(driver, fx, &["help", "diagnostics"])?;
    let stdout = common::stdout(&output);
    let stderr = common::stderr(&output);
    if !stderr.is_empty() {
        return Err(format!("help diagnostics must not write stderr:\n{stderr}"));
    }
    for fragment in [
        "facade dependency",
        "laundered forbidden edge",
        "contract leak",
    ] {
        if !stdout.contains(fragment) {
            return Err(format!(
                "catalog must quote the entry \"{fragment}\":\n{stdout}"
            ));
        }
    }
    Ok(())
}
