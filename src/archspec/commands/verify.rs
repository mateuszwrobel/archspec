use crate::archspec::capability;
use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::language::{self, Language};
use crate::archspec::scan;
use crate::archspec::spec;
use crate::archspec::verify::compare;
use std::path::Path;

/// Sentinel returned as the Err payload when the full diff report (the product)
/// was already printed to stdout. Dispatch must exit non-zero without echoing
/// it to stderr — a rule violation is not an operational error.
pub const RULE_VIOLATION_REPORTED: &str = "\u{1}archspec-rule-violation-reported";

pub const HELP: &str = "usage: archspec verify [path] [--strict]\nextract the model and compare against architecture.spec.toml; exit 1 on violations\n\n  path          project directory to extract and compare (default: current directory)\n  --strict      promote every warning to an error (CI gate); a green run means zero\n                findings of any kind\n\nsee 'archspec help workflow' for the audit recipe and 'archspec help diagnostics'\nfor what each finding category means\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "strict",
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

    let loaded = spec::load_verify(root_path)?;
    // Every run names every active exclusion on the report surface (stderr)
    // before the verdict prints — an exclusion can never shrink the analyzed
    // set in silence (D13/R3-7).
    eprint!("{}", spec::excluded_sources_report(&loaded));
    let language = language_from_str(&loaded.project.language)?;

    // Root-marker selection (detection record GC): the pin SELECTS the
    // DRIVER — its root marker wins outright, a root carrying only foreign
    // markers answers the `language mismatch:` branch (byte prefix kept),
    // a marker-less root keeps the unchanged zero-hits guard. The
    // nested-marker report line prints BEFORE the agreement precondition
    // so a red run carries it too.
    let language = language::select(root_path, Some(language))?
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    eprint!("{}", language::marker_report(root_path, language));

    // The agreement check is a precondition (D14): the declared language is
    // the truth of the tree, and any un-excluded foreign source file fails
    // the run loud BEFORE any verdict, note or artifact is computed.
    if let Some(finding) = language::agreement_finding(
        root_path,
        language,
        &spec::exclusion_paths(&loaded),
    ) {
        return Err(finding);
    }
    if !scan::driver_available(language) {
        return Err(format!(
            "no {} toolchain found (driver missing); run 'archspec doctor' to diagnose",
            language_display(language)
        ));
    }

    let model = scan::extract_checked(language, root_path)?;
    // The scan report surface covers both verbs (D13): wildcard facts print
    // here exactly as they do on `scan`, report-only (R2-6), and so do the
    // `external not installed` lines (ts-resolution record R7b).
    eprint!("{}", scan::wildcard_report(&model));
    eprint!("{}", scan::report_only_report(&model));

    let mapping = compare::map_units(&model.units, &loaded.modules);
    let diff = compare::compare(
        &mapping,
        &model,
        &loaded.modules,
        &loaded.stereotype,
        &loaded.constraint,
    );
    // `--strict` promotes every warning-level divergence to an error so any
    // finding fails the run (design §9, cli.md: the CI gate).
    let strict = parsed.switches.contains("strict");
    let notes = inert_rule_notes(language, &diff);

    if diff.is_empty() {
        print!(
            "{}",
            compare::pass_confirmation(&loaded.modules, loaded.constraint.len())
        );
        print_notes(&notes);
        Ok(())
    } else if diff.fails(strict) {
        print!("{}", compare::render_report(&diff, strict));
        print_notes(&notes);
        Err(RULE_VIOLATION_REPORTED.to_string())
    } else {
        // Warning-only run: report still listed on stdout, but warnings are
        // tolerated without --strict, so the run passes (exit 0).
        print!("{}", compare::render_report(&diff, strict));
        print_notes(&notes);
        Ok(())
    }
}

/// Informational notes stating which capability-table-listed rules can never
/// fire for this language (or, for a conditional fact, fired this run from no
/// native scan fact). A note is suppressed whenever the rule's own finding is
/// present — a rule that just fired is not inert, the finding states what it
/// saw. Notes are output, not findings: they never enter the diff, never
/// change the verdict or exit status, and keep rust runs byte-identical (rust
/// emits every rule fact).
fn inert_rule_notes(language: Language, diff: &compare::Diff) -> Vec<String> {
    let mut notes = Vec::new();
    for (rule, fact) in capability::RULES {
        if rule_reported(rule, diff) {
            continue;
        }
        if let Some(capability::NOT_EMITTED) = capability::emission(language, fact) {
            notes.push(format!(
                "note: {rule} rule inert for {}: no derivable {fact} idiom — alias \
                 umbrella, public-package and root delegation forms investigated, \
                 recorded in ADR-017",
                language.as_str()
            ));
        }
    }
    notes
}

/// True when the run's diff carries at least one finding of this rule, so an
/// accompanying "inert"-style note would contradict the report.
fn rule_reported(rule: &str, diff: &compare::Diff) -> bool {
    match rule {
        capability::RULE_FACADE_DEPENDENCY => !diff.facade_dependencies.is_empty(),
        capability::RULE_LAUNDERED_FORBIDDEN_EDGE => diff
            .warnings
            .iter()
            .any(|entry| entry.starts_with(&format!("{rule}: "))),
        _ => false,
    }
}

fn print_notes(notes: &[String]) {
    for note in notes {
        println!("{note}");
    }
}

fn language_from_str(value: &str) -> Result<Language, String> {
    // The shared parser (round-8 F11): the hand-copied third mapping is
    // deleted — the red bytes stay byte-identical.
    language::from_name(value)
        .ok_or_else(|| format!("invalid spec: unknown language \"{value}\""))
}

fn language_display(language: Language) -> String {
    let value = language.as_str();
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => value.to_string(),
    }
}
