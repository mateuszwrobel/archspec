use crate::archspec::capability;
use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::language::{self, Language};
use crate::archspec::model::Model;
use crate::archspec::scan;
use crate::archspec::spec;
use crate::archspec::verify::compare;
use std::path::Path;

/// Sentinel returned as the Err payload when the full diff report (the product)
/// was already printed to stdout. Dispatch must exit non-zero without echoing
/// it to stderr — a rule violation is not an operational error.
pub const RULE_VIOLATION_REPORTED: &str = "\u{1}archspec-rule-violation-reported";

pub const HELP: &str = "usage: archspec verify [path] [--strict]\nextract the model and compare against architecture.spec.toml; exit 1 on violations\n\n  path       project directory to extract and compare (default: current directory)\n  --strict   promote every warning to an error (CI gate); a green run means zero\n             findings of any kind\n\nsee 'archspec help workflow' for the audit recipe and 'archspec help diagnostics'\nfor what each finding category means\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[FlagSpec {
            name: "strict",
            takes_value: false,
        }],
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
    let language = language_from_str(&loaded.project.language)?;

    if language::detect(root_path).is_none() {
        return Err(format!("no supported-language sources found under: {root}"));
    }
    if !scan::driver_available(language) {
        return Err(format!(
            "no {} toolchain found (driver missing); run 'archspec doctor' to diagnose",
            language_display(language)
        ));
    }

    let model = scan::extract_checked(language, root_path)?;

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
    let notes = inert_rule_notes(language, &model, &diff);

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
fn inert_rule_notes(language: Language, model: &Model, diff: &compare::Diff) -> Vec<String> {
    let mut notes = Vec::new();
    for (rule, fact) in capability::RULES {
        if rule_reported(rule, diff) {
            continue;
        }
        match capability::emission(language, fact) {
            Some(capability::NOT_EMITTED) => notes.push(format!(
                "note: {rule} rule inert for {}: driver emits no {fact} fact",
                language.as_str()
            )),
            Some(capability::WORKTIER_ONLY) if !model.has_module_tier() => notes.push(format!(
                "note: {rule} rule: no native module tier this run \
                 (grouping derived from spec declarations)"
            )),
            _ => {}
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
    match value {
        "rust" => Ok(Language::Rust),
        "csharp" => Ok(Language::Csharp),
        "go" => Ok(Language::Go),
        other => Err(format!("invalid spec: unknown language \"{other}\"")),
    }
}

fn language_display(language: Language) -> String {
    let value = language.as_str();
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => value.to_string(),
    }
}
