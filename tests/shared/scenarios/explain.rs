//! `explain` scenarios (GitHub issue #26): the verb renders the verify path
//! per pair. The shared scenarios state the two slices the surface promises:
//! a clean tree traces with an empty findings section, and a tree whose cycle
//! verify reports has that finding named in the trace — the parity claim in
//! the shape the matrix can run per driver.

use crate::common;
use crate::shared::driver::Driver;
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{
    boundary_spec, expect_success, not_ts_or_py, scenario, scenario_when,
};

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::ExplainPairTrace,
            "explain_traces_a_clean_pair_with_no_findings",
            "explain over a clean boundary spec exits 0, states the pair's model row and the renderer note, and names no finding",
            explain_traces_a_clean_pair_with_no_findings,
        ),
        scenario_when(
            Feature::ExplainPairTrace,
            "explain_names_the_cycle_findings_verify_reports",
            "a cycle verify reports is listed by explain's findings section for the pair, and the trace states the pair is adjudicated",
            not_ts_or_py,
            explain_names_the_cycle_findings_verify_reports,
        ),
    ]
}

fn explain_traces_a_clean_pair_with_no_findings(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    driver.materialize(fx, &driver.probe_tree());
    fx.write("architecture.spec.toml", &boundary_spec(driver));
    let verify = expect_success(driver, fx, &["verify"])?;
    let verify_out = common::stdout(&verify);
    if !verify_out.contains("matches source model") {
        return Err(format!("verify must confirm the match first:\n{verify_out}"));
    }
    // The pair names are the declared boundary names of `boundary_spec`:
    // the unit boundaries are app and shared; the single-package dialects
    // (ts, python) declare the core file module allowed onto the ui one.
    let (source, target) = if crate::shared::scenarios::ts_driver(driver)
        || crate::shared::scenarios::py_driver(driver)
    {
        (
            driver.module_path("app", "core"),
            driver.module_path("app", "ui"),
        )
    } else {
        ("app".to_string(), "shared".to_string())
    };
    let output = expect_success(driver, fx, &["explain", &source, &target])?;
    let stdout = common::stdout(&output);
    for needle in [
        &format!("pair: {source} -> {target}"),
        "model rows:",
        &format!("module-graph row {source} -> {target}: present"),
        "findings naming the pair:",
        "note: explain renders the same load, extract and compare calls `archspec verify` runs",
    ] {
        if !stdout.contains(needle) {
            return Err(format!("trace must state `{needle}`:\n{stdout}"));
        }
    }
    if stdout.contains("disallowed cross-component dependency:") {
        return Err(format!("a clean tree must name no finding:\n{stdout}"));
    }
    Ok(())
}

fn explain_names_the_cycle_findings_verify_reports(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = crate::shared::driver::LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    tree.hard_edges = vec![("a".into(), "b".into()), ("b".into(), "a".into())];
    driver.materialize(fx, &tree);
    crate::shared::scenarios::verify::write_cycle_spec(driver, fx, true);
    let verify = driver.run(fx, &["verify"]);
    if verify.status.code() == Some(0) {
        return Err("verify must fail on a dependency cycle".into());
    }
    let verify_out = common::stdout(&verify);
    let cycle_body = verify_out
        .lines()
        .find_map(|line| {
            line.find("cycle: ")
                .map(|at| line[at + "cycle: ".len()..].trim().to_string())
        })
        .ok_or_else(|| format!("verify must list the cycle:\n{verify_out}"))?;
    let output = expect_success(driver, fx, &["explain", "a", "b"])?;
    let stdout = common::stdout(&output);
    let wanted = format!("cycle detected in: {cycle_body}");
    if !stdout.contains(&wanted) {
        return Err(format!(
            "the findings section must list the cycle verify reported, rendered under the catalog category:\n{stdout}"
        ));
    }
    if !stdout.contains("pair adjudicated: yes") {
        return Err(format!(
            "both endpoints are inside the declared territory, so the trace must say the pair is adjudicated:\n{stdout}"
        ));
    }
    Ok(())
}
