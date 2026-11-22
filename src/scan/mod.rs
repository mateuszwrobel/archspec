use crate::driver::Driver;
use crate::language::Language;
use crate::model::Model;
use std::path::Path;

pub mod csharp;
pub mod go;
pub mod kotlin;
mod leg;
pub mod python;
pub mod rust;
pub mod ts;

/// The declared facts ride the model vocabulary (the scan facts
/// seam): re-exported here so the `scan::DeclaredFacts` path keeps
/// resolving for consumers that address them through the scan engine.
pub use crate::model::{DeclaredFacts, DeclaredPattern, DeclaredPatternVerb, DeclaredZone};

/// Extract the architecture model of `root` for `language`, spec-free
/// (mechanical exclusions only — the stateless fresh-walk entry the facts
/// library surfaces). A consumer holding a project spec passes its entries
/// through [`extract_with`].
pub fn extract(language: Language, root: &Path) -> Result<Model, String> {
    extract_with(language, root, &DeclaredFacts::default())
}

/// Extract with the declared facts applied: the same walk, adjudicated
/// against the entries the consumer loaded from its own spec. The
/// language-to-driver decision moves through the ONE compile-time table
/// below (workplan events_value_join scenario 10, ADR-069 Decision 5's
/// ports rung) — every serialized surface is unchanged.
pub fn extract_with(
    language: Language,
    root: &Path,
    declared: &DeclaredFacts,
) -> Result<Model, String> {
    driver(language).extract(root, declared)
}

/// The compile-time driver table (workplan events_value_join scenario 10):
/// the ONE exhaustive `Language` match that decides extraction, returning
/// the driver as a `&'static dyn Driver` over unit structs. Exhaustive by
/// construction: a new `Language` variant without a driver fails to
/// compile. There is no runtime lookup and none may ship (decision-check
/// A3) — the availability gate [`driver_available`] stays OUTSIDE the
/// table, keyed on `as_str()`, exactly where it was.
fn driver(language: Language) -> &'static dyn Driver {
    match language {
        Language::Rust => &rust::RustDriver,
        Language::Csharp => &csharp::CsharpDriver,
        Language::Go => &go::GoDriver,
        Language::Kotlin => &kotlin::KotlinDriver,
        Language::Python => &python::PythonDriver,
        Language::Ts => &ts::TsDriver,
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

/// The unresolvable-imports census line (visibility phase 2, scenario 01).
///
/// ONE implementation owned by the scan engine, read by `scan` and
/// `verify`. Line shape — `unresolvable imports census: <N> specifiers
/// across <M> files — top: <spec> ×<k>, …` — where `<N>` counts DISTINCT
/// specifiers and `<M>` DISTINCT files (a repeated edge counts once per
/// dimension); the top list is complete, sorted count-descending then
/// specifier-ascending (the MINOR-1 determinism law). `None` when the
/// census is empty: an empty census is ZERO bytes (the shipped-bytes
/// guard), and the line is never a tripwire arm — it is ABSENCE computed
/// from the walk, the D7/B1 ordering law the orphan census restates. The
/// anchored miss lines print separately, from the same list, in walk
/// order.
pub fn unresolvable_census_line(entries: &[crate::model::UnresolvableImport]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut files: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for entry in entries {
        *counts.entry(entry.specifier.as_str()).or_insert(0) += 1;
        files.insert(entry.file.as_str());
    }
    let mut rows: Vec<(&str, usize)> = counts.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let top: Vec<String> = rows
        .iter()
        .map(|(spec, count)| format!("{spec} ×{count}"))
        .collect();
    Some(format!(
        "unresolvable imports census: {} specifiers across {} files — top: {}",
        rows.len(),
        files.len(),
        top.join(", ")
    ))
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
