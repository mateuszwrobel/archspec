use crate::common;
use crate::shared::driver::{Driver, LogicalTree};
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{boundary_spec, expect_success, metric_value, scenario};
use serde_json::Value;

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::ReportMetrics,
            "report_emits_metrics_for_clean_project",
            "report prints metric rows and a clean result for a spec that matches the tree",
            report_emits_metrics_for_clean_project,
        ),
        scenario(
            Feature::ReportMetrics,
            "report_counts_units_and_components",
            "report counts one component per declared unit and every extracted unit",
            report_counts_units_and_components,
        ),
        scenario(
            Feature::ReportMetrics,
            "report_json_findings_match_text_rendering",
            "report --format json parses, carries the same finding set as the text report, and exits 1 on violations",
            report_json_findings_match_text_rendering,
        ),
        scenario(
            Feature::ReportMetrics,
            "report_json_clean_project_has_empty_findings",
            "a clean project reports empty findings and exit 0 in the json format",
            report_json_clean_project_has_empty_findings,
        ),
    ]
}

fn report_emits_metrics_for_clean_project(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    fx.write("architecture.spec.toml", &boundary_spec(driver));
    let output = expect_success(driver, fx, &["report"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("components:") {
        return Err(format!("report must include metrics:\n{stdout}"));
    }
    if !stdout.contains("Result: clean") {
        return Err(format!("matching spec must report clean:\n{stdout}"));
    }
    Ok(())
}

fn report_counts_units_and_components(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    fx.write("architecture.spec.toml", &boundary_spec(driver));
    let output = expect_success(driver, fx, &["report"])?;
    let stdout = common::stdout(&output);
    let units = metric_value(&stdout, "units")
        .ok_or_else(|| format!("report must count units:\n{stdout}"))?;
    let components = metric_value(&stdout, "components")
        .ok_or_else(|| format!("report must count components:\n{stdout}"))?;
    if units != "2" {
        return Err(format!("expected 2 units, got {units}:\n{stdout}"));
    }
    if components != "2" {
        return Err(format!("expected 2 components, got {components}:\n{stdout}"));
    }
    Ok(())
}
/// The diff-section lines of a rendered text report as (category, message).
fn text_findings(text: &str) -> Vec<(String, String)> {
    text.lines()
        .skip_while(|line| !line.starts_with("Diff: extracted vs declared"))
        .skip(2)
        .take_while(|line| !line.trim().is_empty())
        .filter_map(|line| {
            line.split_once(": ")
                .map(|(category, body)| (category.to_string(), body.to_string()))
        })
        .collect()
}

fn json_findings(value: &Value) -> Result<Vec<(String, String)>, String> {
    let findings = value["findings"]
        .as_array()
        .ok_or_else(|| "json report must carry a findings array".to_string())?;
    findings
        .iter()
        .map(|finding| {
            let category = finding["category"]
                .as_str()
                .ok_or_else(|| "finding must carry a category".to_string())?;
            let severity = finding["severity"]
                .as_str()
                .ok_or_else(|| "finding must carry a severity".to_string())?;
            if !matches!(severity, "error" | "warning") {
                return Err(format!("unknown finding severity: {severity}"));
            }
            let message = finding["message"]
                .as_str()
                .ok_or_else(|| "finding must carry a message".to_string())?;
            Ok((category.to_string(), message.to_string()))
        })
        .collect()
}

fn report_json_findings_match_text_rendering(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    // A declared ghost component missing from the tree: every driver's diff
    // carries at least one error-level finding.
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    driver.materialize(fx, &tree);
    let ghost = driver.unit_name("ghost");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"ghost\"\nmatches = {{ units = [\"{ghost}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let text = driver.run(fx, &["report"]);
    if text.status.code() == Some(0) {
        return Err("report with violations must exit non-zero".into());
    }
    let json = driver.run(fx, &["report", "--format", "json"]);
    if json.status.code() == Some(0) {
        return Err("the json format must propagate the violation exit".into());
    }
    let value: Value = serde_json::from_str(&common::stdout(&json))
        .map_err(|error| format!("json report must parse: {error}"))?;
    if value["language"].as_str() != Some(driver.language.as_str()) {
        return Err(format!(
            "json must name the declared language:\n{}",
            common::stdout(&json)
        ));
    }
    let findings = json_findings(&value)?;
    if findings.is_empty() {
        return Err("a violating tree must yield at least one finding".into());
    }
    if findings != text_findings(&common::stdout(&text)) {
        return Err(format!(
            "json findings must equal the text-rendered set:\njson: {findings:?}\ntext:\n{}",
            common::stdout(&text)
        ));
    }
    Ok(())
}

fn report_json_clean_project_has_empty_findings(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    fx.write("architecture.spec.toml", &boundary_spec(driver));
    let output = expect_success(driver, fx, &["report", "--format", "json"])?;
    let value: Value = serde_json::from_str(&common::stdout(&output))
        .map_err(|error| format!("json report must parse: {error}"))?;
    let findings = value["findings"]
        .as_array()
        .ok_or_else(|| "json report must carry a findings array".to_string())?;
    if !findings.is_empty() {
        return Err(format!("a clean tree must report empty findings: {findings:?}"));
    }
    Ok(())
}
