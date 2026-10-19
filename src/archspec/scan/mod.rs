use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::language::{self, Language};
use crate::archspec::model::Model;
use crate::archspec::spec;
use std::path::Path;

pub(crate) mod csharp;
pub(crate) mod go;
pub(crate) mod kotlin;
mod leg;
pub(crate) mod python;
mod rust;
pub(crate) mod ts;

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "output",
                takes_value: true,
            },
            FlagSpec {
                name: "check",
                takes_value: false,
            },
            FlagSpec {
                name: "pretty",
                takes_value: false,
            },
        ],
    )?;
    cli::exactly_one_positional(&parsed)?;

    let root = parsed
        .positionals
        .first()
        .map(String::as_str)
        .unwrap_or(".");
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("path does not exist: {root}"));
    }
    if !root_path.is_dir() {
        return Err(format!("path is not a directory: {root}"));
    }

    // Root-only spec discovery (D14(4)): read the spec when it sits at the
    // scanned root — an invalid spec fails the run with verify's class; a
    // missing spec keeps scan spec-free. Active exclusions are named on the
    // report surface (stderr) before the work runs — every run, R3-7.
    let loaded = spec::load_scan(root_path)?;
    if let Some(spec) = &loaded {
        eprint!("{}", spec::excluded_sources_report(spec));
    }

    // Root-marker selection (detection record GC): with a spec at the root
    // the pinned language SELECTS the driver (its root marker wins, a foreign
    // marker root answers the mismatch branch); without one exactly one root
    // marker selects, several answer the AMBIGUITY error, none keeps the
    // unchanged chain fallback. The nested-marker report line prints BEFORE
    // the agreement precondition so a red run carries it too.
    let pin = match &loaded {
        Some(spec) => Some(language::from_name(&spec.project.language).ok_or_else(|| {
            format!("invalid spec: unknown language \"{}\"", spec.project.language)
        })?),
        None => None,
    };
    let language = language::select(root_path, pin)?
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    eprint!("{}", language::marker_report(root_path, language));

    // The agreement check is a precondition (D14): with a spec at the root,
    // the declared language is the truth and any un-excluded foreign source
    // fails the run BEFORE any artifact or model body. Without a spec, chain
    // detection stays the whole truth — nothing is checked (D14(2)).
    if let (Some(spec), Some(declared)) = (loaded.as_ref(), pin)
        && let Some(finding) =
            language::agreement_finding(root_path, declared, &spec::exclusion_paths(spec))
    {
        return Err(finding);
    }

    if !driver_available(language) {
        return Err(driver_unavailable(language));
    }

    let model = extract(language, root_path)?;
    // The wildcard facts are report-only (R2-6): one stderr line per fact,
    // never part of the machine body, never a verdict.
    eprint!("{}", wildcard_report(&model));
    // The `external not installed` lines are report-only the same way
    // (ts-resolution record R7b).
    eprint!("{}", report_only_report(&model));
    // `--pretty` is a serialization switch only: same facts, indented layout
    // for readers and diffs; the default bytes are untouched (blind4 D03).
    let json = if parsed.switches.contains("pretty") {
        format!("{}\n", model.to_json_pretty()?)
    } else {
        format!("{}\n", model.to_json()?)
    };

    let config = config::load(root_path)?;
    // The model JSON is a machine body: a body on stdout carries no status
    // line (wave-3 D02 purity); a destination write echoes the universal
    // `wrote <path>` line like every path write (blind4 D01).
    let route = config::route(
        parsed.values.get("output").map(String::as_str),
        config.scan.as_deref(),
        root_path,
        false,
    );
    let check = parsed.switches.contains("check");
    config::emit(&json, route, check, "model", "scan", None)
}

pub fn extract(language: Language, root: &Path) -> Result<Model, String> {
    match language {
        Language::Rust => rust::extract(root),
        Language::Csharp => csharp::extract(root),
        Language::Go => go::extract(root),
        Language::Kotlin => kotlin::extract(root),
        Language::Python => python::extract(root),
        Language::Ts => ts::extract(root),
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

/// Extract with the driver-availability gate applied. Every consumer that
/// extracts through a language driver must go through this so the
/// env-controlled test invariant cannot be silently bypassed.
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

fn driver_unavailable(language: Language) -> String {
    format!(
        "detected language '{}' but the {} driver is not available (run 'archspec doctor')",
        language.as_str(),
        language.as_str()
    )
}

