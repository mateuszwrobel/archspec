use crate::language::Language;
use crate::model::Model;
use std::path::Path;

pub mod csharp;
pub mod go;
pub mod kotlin;
mod leg;
pub mod python;
mod rust;
pub mod ts;

/// The declared facts ride the model vocabulary (the scan facts
/// seam): re-exported here so the `scan::DeclaredFacts` path keeps
/// resolving for consumers that address them through the scan engine.
pub use crate::model::{DeclaredFacts, DeclaredZone};

/// Extract the architecture model of `root` for `language`, spec-free
/// (mechanical exclusions only — the stateless fresh-walk entry the facts
/// library surfaces). A consumer holding a project spec passes its entries
/// through [`extract_with`].
pub fn extract(language: Language, root: &Path) -> Result<Model, String> {
    extract_with(language, root, &DeclaredFacts::default())
}

/// Extract with the declared facts applied: the same walk, adjudicated
/// against the entries the consumer loaded from its own spec.
pub fn extract_with(
    language: Language,
    root: &Path,
    declared: &DeclaredFacts,
) -> Result<Model, String> {
    match language {
        Language::Rust => rust::extract(root),
        Language::Csharp => csharp::extract(root),
        Language::Go => go::extract(root),
        Language::Kotlin => kotlin::extract(root, declared),
        Language::Python => python::extract(root, declared),
        Language::Ts => ts::extract(root, declared),
    }
}

/// The `dynamic import wildcard` report line (D13 owner: the scan report
/// surface — printed by `scan` and `verify`, never by artifact-only verbs):
/// one line per fact, exact bytes
/// `dynamic import wildcard: <file> -> <expression>`. Report-only: the
/// fact states a blind spot, it never reddens a run (R2-6).
pub fn wildcard_report(model: &Model) -> String {
    let mut out = String::new();
    for (file, expressions) in &model.dynamic_import_wildcards {
        for expression in expressions {
            out.push_str(&format!("dynamic import wildcard: {file} -> {expression}\n"));
        }
    }
    out
}

/// The `external not installed` report line (D13 owner: the scan report
/// surface — printed by `scan` and `verify`, never by artifact-only verbs):
/// one line per package, exact bytes
/// `external not installed: <package> — declared in <manifest>, absent from
/// disk`. Report-only (ts-resolution record R7b law): the line states an
/// install gap, it never reddens a run and it never rides the machine body.
pub fn report_only_report(model: &Model) -> String {
    let mut out = String::new();
    for line in &model.report_only_lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Extract with the driver-availability gate applied, spec-free (see
/// [`extract`]). Every consumer that extracts through a language driver
/// must go through this so the env-controlled test invariant cannot be
/// silently bypassed.
pub fn extract_checked(language: Language, root: &Path) -> Result<Model, String> {
    if !driver_available(language) {
        return Err(driver_unavailable(language));
    }
    extract(language, root)
}

/// Phase-1 ships all drivers in the binary, so they are available unless a
/// test controls the environment (acceptance #11) by disabling one via
/// `ARCHSPEC_DISABLE_DRIVERS` (comma-separated, e.g. "go").
pub fn driver_available(language: Language) -> bool {
    std::env::var("ARCHSPEC_DISABLE_DRIVERS")
        .map(|value| {
            !value
                .split(',')
                .any(|part| part.trim() == language.as_str())
        })
        .unwrap_or(true)
}

pub fn driver_unavailable(language: Language) -> String {
    format!(
        "detected language '{}' but the {} driver is not available (run 'archspec doctor')",
        language.as_str(),
        language.as_str()
    )
}
