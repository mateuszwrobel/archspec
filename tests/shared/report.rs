use crate::common;
use crate::shared::driver::Driver;
use crate::shared::feature::Feature;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The feature-matrix report: languages × features × behaviors with statuses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixReport {
    pub languages: Vec<String>,
    pub features: Vec<FeatureReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureReport {
    /// feature.as_str()
    pub name: String,
    /// lang -> "implemented" | "not-implemented"
    pub language_capability: BTreeMap<String, String>,
    pub behaviors: Vec<BehaviorReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorReport {
    pub name: String,
    pub description: String,
    /// lang -> "pass" | "fail" | "skipped"
    pub status: BTreeMap<String, String>,
    /// lang -> why the cell is skipped. Capability skips quote the
    /// capability table row verbatim; there is no second list to drift from.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skip_reasons: BTreeMap<String, String>,
}

/// An empty matrix report: languages from `Driver::all`, one `FeatureReport`
/// per feature with capabilities filled by running each probe against a fresh
/// fixture, and no behaviors.
pub fn empty_report() -> MatrixReport {
    let languages: Vec<String> = Driver::all()
        .iter()
        .map(|driver| driver.language.as_str().to_string())
        .collect();
    let features = Feature::ALL
        .iter()
        .map(|feature| {
            let mut language_capability = BTreeMap::new();
            for driver in Driver::all() {
                let fx = common::Fixture::new();
                let implemented = feature.probe(&driver, &fx);
                language_capability.insert(
                    driver.language.as_str().to_string(),
                    if implemented {
                        "implemented".to_string()
                    } else {
                        "not-implemented".to_string()
                    },
                );
            }
            FeatureReport {
                name: feature.as_str().to_string(),
                language_capability,
                behaviors: Vec::new(),
            }
        })
        .collect();
    MatrixReport {
        languages,
        features,
    }
}

/// Deterministic markdown, grouped by feature: a header line per feature with
/// per-language capability, then a bullet per behavior with per-language status.
/// Language order follows `report.languages`; feature order follows
/// `report.features`; behavior order follows each feature's behaviors.
pub fn report_to_markdown(report: &MatrixReport) -> String {
    let mut out = String::from("# Feature Matrix\n\n");
    for feature in &report.features {
        out.push_str(&format!("## {}\n", feature.name));
        let caps: Vec<String> = report
            .languages
            .iter()
            .map(|lang| {
                let capability = feature
                    .language_capability
                    .get(lang)
                    .map(String::as_str)
                    .unwrap_or("unknown");
                format!("{lang}={capability}")
            })
            .collect();
        out.push_str(&format!("**Capability:** {}\n\n", caps.join(", ")));
        if feature.behaviors.is_empty() {
            out.push_str("_no behaviors yet_\n\n");
            continue;
        }
        out.push_str("**Behaviors:**\n\n");
        for behavior in &feature.behaviors {
            out.push_str(&format!(
                "- **{}** — {}\n",
                behavior.name, behavior.description
            ));
            for lang in &report.languages {
                let status = behavior
                    .status
                    .get(lang)
                    .map(String::as_str)
                    .unwrap_or("unknown");
                let detail = if status == "skipped" {
                    match behavior.skip_reasons.get(lang) {
                        Some(reason) => format!(" ({reason})"),
                        None => String::new(),
                    }
                } else {
                    String::new()
                };
                out.push_str(&format!("  - {lang}: {status}{detail}\n"));
            }
        }
        out.push('\n');
    }
    out
}

/// Pretty JSON serialization of the report.
pub fn report_to_json(report: &MatrixReport) -> String {
    serde_json::to_string_pretty(report).expect("serialize report to JSON")
}
