//! The binary's CLI glue over the library scan engine (discovery-extraction
//! rung one). The engine walks through `rust_arch_test_kit::scan`; this file
//! owns the root-spec discovery (D14(4)) the engine no longer performs: the
//! verbs keep the shipped behavior — the root `architecture.spec.toml` is
//! read once, its declared entries are handed to the walk as facts, and the
//! report lines print on the report surface exactly as before.

use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::language::Language;
use crate::archspec::model::Model;
use crate::archspec::spec;
use rust_arch_test_kit::scan::{DeclaredFacts, DeclaredZone};
use std::path::Path;

// The engine surface the verbs consume unchanged: the driver-availability
// gate, the report-surface helpers, and the driver modules the `inspect`
// renderers walk directly.
pub use rust_arch_test_kit::scan::{csharp, driver_available, driver_unavailable, go, report_only_report, wildcard_report};

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
        // A dead entry (zero matches under the mechanical set) is stated on
        // the same surface right after the active-entry lines (issue-8
        // phase 2, decision-check MAJOR-2): once per entry per run.
        eprint!("{}", spec::exclusion_liveness_report(root_path, spec));
    }

    // Root-marker selection (detection record GC): with a spec at the root
    // the pinned language SELECTS the driver (its root marker wins, a foreign
    // marker root answers the mismatch branch); without one exactly one root
    // marker selects, several answer the AMBIGUITY error, none keeps the
    // unchanged chain fallback. The nested-marker report line prints BEFORE
    // the agreement precondition so a red run carries it too.
    let pin = match &loaded {
        Some(spec) => Some(crate::archspec::language::from_name(&spec.project.language).ok_or_else(|| {
            format!("invalid spec: unknown language \"{}\"", spec.project.language)
        })?),
        None => None,
    };
    let language = crate::archspec::language::select(root_path, pin)?
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    eprint!("{}", crate::archspec::language::marker_report(root_path, language));

    // The agreement check is a precondition (D14): with a spec at the root,
    // the declared language is the truth and any un-excluded foreign source
    // fails the run BEFORE any artifact or model body. Without a spec, chain
    // detection stays the whole truth — nothing is checked (D14(2)).
    if let (Some(spec), Some(declared)) = (loaded.as_ref(), pin)
        && let Some(finding) = crate::archspec::language::agreement_finding(
            root_path,
            declared,
            &spec::exclusion_paths(spec),
        )
    {
        return Err(finding);
    }

    if !driver_available(language) {
        return Err(driver_unavailable(language));
    }

    let model = extract_with_spec(language, root_path, loaded.as_ref())?;
    // The wildcard facts are report-only (R2-6): one stderr line per fact,
    // never part of the machine body, never a verdict.
    eprint!("{}", wildcard_report(&model));
    // The `external not installed` lines are report-only the same way
    // (ts-resolution record R7b).
    eprint!("{}", report_only_report(&model));
    // The warn-mode exclusion findings (issue-8 phase 2, BLOCKER-2): one
    // `warning:` line per finding on the report surface — stated, never a
    // verdict, never the machine body (verify fails the same finding under
    // `--strict`; scan carries no verdict).
    for warning in &model.warnings {
        eprintln!("warning: {warning}");
    }
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
        config.output.scan.as_deref(),
        root_path,
        false,
    );
    let check = parsed.switches.contains("check");
    config::emit(&json, route, check, "model", "scan", None)
}

/// The verbs' entry: load the root spec the way the drivers used to
/// (silently — an invalid spec failed the verb earlier where the verb
/// loads it; a missing spec answers the spec-free walk) and extract with
/// the declared facts. Byte-identical to the pre-extraction
/// `scan::extract(language, root)`.
pub fn extract(language: Language, root: &Path) -> Result<Model, String> {
    let loaded = spec::load_scan(&canonical(root)).ok().flatten();
    extract_with_spec(language, root, loaded.as_ref())
}

/// Extract with the driver-availability gate applied and the root-spec
/// facts loaded (the shipped `extract_checked` contract, preserved).
pub fn extract_checked(language: Language, root: &Path) -> Result<Model, String> {
    if !driver_available(language) {
        return Err(driver_unavailable(language));
    }
    extract(language, root)
}

fn extract_with_spec(
    language: Language,
    root: &Path,
    loaded: Option<&spec::Spec>,
) -> Result<Model, String> {
    let declared = loaded.map(declared_facts).unwrap_or_default();
    rust_arch_test_kit::scan::extract_with(language, root, &declared)
}

/// Map the loaded spec onto the engine's declared facts: the entries the
/// adjudicating drivers consume, in the same order and values the drivers
/// derived from the spec before the extraction.
fn declared_facts(loaded: &spec::Spec) -> DeclaredFacts {
    DeclaredFacts {
        exclusions: spec::exclusion_entries(loaded),
        zones: loaded
            .modules
            .iter()
            .map(|zone| DeclaredZone {
                name: zone.name.clone(),
                units: zone.matches.units.clone(),
                paths: zone.matches.paths.clone(),
                modules: zone.matches.modules.clone(),
            })
            .collect(),
        framework: loaded.project.framework.clone(),
    }
}

/// The canonicalization every spec-reading driver applied to its root
/// before the spec load; the glue reproduces it at the same point.
fn canonical(root: &Path) -> std::path::PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}
