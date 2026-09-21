use crate::common;
use crate::shared::driver::{Driver, Language};
use crate::shared::feature::Feature;
use crate::shared::scenario::{Applies, Scenario};
use serde_json::Value;

pub mod cli;
pub mod diagram;
pub mod depgraph;
pub mod help;
pub mod init;
pub mod inspect;
pub mod report;
pub mod scan;
pub mod update;
pub mod verify;

/// The full shared behavior suite. Each scenario is a granular behavior
/// assertion for one feature, run against every driver; the runner skips a
/// scenario on drivers whose capability probe reports the feature
/// not-implemented. Order: scan tiers, verify constraints, then the
/// workflow commands (update, report, inspect, diagram, init), then the
/// CLI surfaces (help, artefact freshness).
pub fn all() -> Vec<Scenario> {
    let mut out = Vec::new();
    out.extend(scan::all());
    out.extend(verify::all());
    out.extend(update::all());
    out.extend(report::all());
    out.extend(inspect::all());
    out.extend(depgraph::all());
    out.extend(diagram::all());
    out.extend(init::all());
    out.extend(help::all());
    out.extend(cli::all());
    out
}

pub(crate) fn scenario(
    feature: Feature,
    name: &'static str,
    description: &'static str,
    run: fn(&Driver, &common::Fixture) -> Result<(), String>,
) -> Scenario {
    Scenario {
        feature,
        name,
        description,
        run,
        applies: None,
    }
}

/// Like `scenario`, but the behavior only applies where `applies` returns true.
/// Drivers where it does not apply render `skipped`.
pub(crate) fn scenario_when(
    feature: Feature,
    name: &'static str,
    description: &'static str,
    applies: fn(&Driver) -> bool,
    run: fn(&Driver, &common::Fixture) -> Result<(), String>,
) -> Scenario {
    Scenario {
        feature,
        name,
        description,
        run,
        applies: Some(Applies::Custom(applies)),
    }
}

/// A capability-driven behavior: it runs exactly where the driver's capability
/// table row for `fact` says `granular`. The skip is table-driven — there is
/// no per-language list to keep in sync, and the prose guard fails the suite
/// if a cited fact disappears from the table.
pub(crate) fn scenario_when_capability(
    feature: Feature,
    name: &'static str,
    description: &'static str,
    fact: &'static str,
    run: fn(&Driver, &common::Fixture) -> Result<(), String>,
) -> Scenario {
    Scenario {
        feature,
        name,
        description,
        run,
        applies: Some(Applies::Capability(fact)),
    }
}

/// The complement of `scenario_when_capability`: the behavior runs exactly
/// where the table says the driver does NOT emit `fact` at full granularity
/// (the honest-vacuity side: inert-rule notes, "not verifiable" wordings).
pub(crate) fn scenario_when_inert(
    feature: Feature,
    name: &'static str,
    description: &'static str,
    fact: &'static str,
    run: fn(&Driver, &common::Fixture) -> Result<(), String>,
) -> Scenario {
    Scenario {
        feature,
        name,
        description,
        run,
        applies: Some(Applies::Inert(fact)),
    }
}

/// Evaluate a scenario's applicability for one driver. Capability variants
/// consult the capability table through its machine projection (see
/// `shared::capability`); `Custom` runs the predicate.
pub(crate) fn applies_here(driver: &Driver, applies: &Applies) -> bool {
    let language = driver.language.as_str();
    match applies {
        Applies::Custom(predicate) => predicate(driver),
        Applies::Capability(fact) => crate::shared::capability::table().granular(language, fact),
        Applies::Inert(fact) => !crate::shared::capability::table().granular(language, fact),
    }
}

/// A per-language external package/crate referenced by the fixture and the
/// model's `external`/`module_external` tiers.
pub(crate) fn external_dep(driver: &Driver) -> &'static str {
    match driver.language {
        Language::Rust => "serde",
        Language::Csharp => "Newtonsoft.Json",
        Language::Go => "example.com/third/party",
    }
}

/// A second external package/crate that the fixture never references, used to
/// assert a constraint that must pass.
pub(crate) fn other_dep(driver: &Driver) -> &'static str {
    match driver.language {
        Language::Rust => "tokio",
        Language::Csharp => "Microsoft.EntityFrameworkCore",
        Language::Go => "example.com/other/third",
    }
}

/// True when a dotted unit stem materializes as a nested unit of the declared
/// component (rust/c# project files). Go scopes packages by directory path,
/// so a dotted stem is not a sub-unit there — it is a separate component.
pub(crate) fn dot_stems_nest_units(driver: &Driver) -> bool {
    matches!(driver.language, Language::Rust | Language::Csharp)
}

/// True when the materializer writes real project files (csproj) alongside
/// sources, so project-level update behaviors (test-project seeding) apply.
pub(crate) fn csharp_project_files(driver: &Driver) -> bool {
    matches!(driver.language, Language::Csharp)
}

/// True only for the go driver — behaviors pinned to what a go SCAN emits
/// (module-tier conditional on go.work, no root-facade fact).
pub(crate) fn go_driver(driver: &Driver) -> bool {
    matches!(driver.language, Language::Go)
}

/// The mermaid id the renderers emit for a raw name: pure `[A-Za-z0-9_]`
/// names keep their bare form; names that sanitization would alter get every
/// unparseable character mapped to `_` plus an 8-hex FNV-1a suffix of the raw
/// name, so distinct raws never share an id. Scenarios assert on *rendered*
/// edges, so they must build ids through the same mapping the render boundary
/// applies.
pub(crate) fn mermaid_id(name: &str) -> String {
    let mut sanitized = String::new();
    let mut changed = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            sanitized.push(c);
        } else {
            sanitized.push('_');
            changed = true;
        }
    }
    if !changed {
        return sanitized;
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let hex = format!("{hash:016x}");
    format!("{sanitized}_{}", &hex[..8])
}

/// Run `archspec` in the fixture, requiring exit 0.
pub(crate) fn expect_success(
    driver: &Driver,
    fx: &common::Fixture,
    args: &[&str],
) -> Result<std::process::Output, String> {
    let output = driver.run(fx, args);
    if output.status.code() != Some(0) {
        return Err(format!(
            "`archspec {}` failed (stderr: {})",
            args.join(" "),
            common::stderr(&output)
        ));
    }
    Ok(output)
}

/// Run `archspec` from a subdirectory of the fixture, requiring exit 0.
pub(crate) fn expect_success_in(
    fx: &common::Fixture,
    dir: &str,
    args: &[&str],
) -> Result<std::process::Output, String> {
    let output = fx.run_in(dir, args);
    if output.status.code() != Some(0) {
        return Err(format!(
            "`archspec {}` in {dir} failed (stderr: {})",
            args.join(" "),
            common::stderr(&output)
        ));
    }
    Ok(output)
}

/// True when a JSON string array contains `needle`.
pub(crate) fn array_contains(value: &Value, needle: &str) -> bool {
    value
        .as_array()
        .map(|array| array.iter().any(|item| item.as_str() == Some(needle)))
        .unwrap_or(false)
}

/// Count occurrences of `needle` in a JSON string array.
pub(crate) fn array_count(value: &Value, needle: &str) -> usize {
    value
        .as_array()
        .map(|array| {
            array
                .iter()
                .filter(|item| item.as_str() == Some(needle))
                .count()
        })
        .unwrap_or(0)
}

/// True when every element of a JSON string array is a string and the array is
/// in ascending order.
pub(crate) fn is_sorted(value: &Value) -> bool {
    value
        .as_array()
        .map(|array| {
            let names: Vec<&str> = array.iter().filter_map(|item| item.as_str()).collect();
            names.len() == array.len() && names.windows(2).all(|pair| pair[0] <= pair[1])
        })
        .unwrap_or(false)
}

/// The unit-boundary spec exercising the canonical tree: one component per unit
/// with the hard edge declared allowed. Verifies clean for every driver.
pub(crate) fn boundary_spec(driver: &Driver) -> String {
    format!(
        "[project]\nlanguage = \"{}\"\n\n\
         [[module]]\nname = \"app\"\nmatches = {{ units = [\"{}\"] }}\n\
         [module.allowed]\ndepend_on = [\"shared\"]\n\n\
         [[module]]\nname = \"shared\"\nmatches = {{ units = [\"{}\"] }}\n",
        driver.language.as_str(),
        driver.unit_name("app"),
        driver.unit_name("shared")
    )
}

/// Extract a metric value from a report line like `components:          2`.
pub(crate) fn metric_value(text: &str, name: &str) -> Option<String> {
    text.lines()
        .find(|line| line.starts_with(&format!("{name}:")))
        .and_then(|line| line.split(':').nth(1))
        .map(|value| value.trim().to_string())
}