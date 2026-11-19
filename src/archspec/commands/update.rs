use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::language;
use crate::archspec::scan;
use crate::archspec::spec;
use crate::archspec::spec::SPEC_FILE;
use crate::archspec::update;
use std::path::Path;

pub const HELP: &str = "usage: archspec update [path] [--force]\nsnapshot the current model as a seed architecture.spec.toml\n\n  path          project directory to scan and snapshot (default: current directory)\n  --force       overwrite an existing architecture.spec.toml\n\n`--force` regenerates the whole spec from the source snapshot: only\n`[project]` `excluded_sources` survives regeneration — every other\nhand-written section is dropped and named on stdout with its entry count\n(recover dropped declarations from version control or a backup).\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "force",
                takes_value: false,
            },
        ],
    )?;
    cli::exactly_one_positional(&parsed)?;

    let force = parsed.switches.contains("force");
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

    let spec_path = root_path.join(SPEC_FILE);
    if spec_path.exists() && !force {
        return Err(format!(
            "spec already exists: {} (use --force to overwrite)",
            spec_path.display()
        ));
    }

    // Root-spec discovery BEFORE driver selection (GitHub issue #38, the
    // scan.rs shape): a present spec is validated first — a malformed spec
    // fails the verb with the shipped `invalid spec:` class before any
    // source scanning — and its [project] language pin SELECTS the driver
    // whether detection is ambiguous or not (the marker census only
    // validates). Without a spec the detection is the unchanged one.
    let loaded = spec::load_scan(root_path)?;
    let pin = loaded.as_ref().map(spec::declared_driver).transpose()?;
    let detected = language::select(root_path, pin)?
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;

    // The seed driver passes the SAME language-mismatch census scan and
    // verify apply (GitHub issue #43 part 1, the scan.rs / verify.rs call
    // shape): `agreement_finding` runs over the driver about to be seeded —
    // the [project] pin when a spec is present, the marker-detected language
    // when it is not — BEFORE the extraction and before anything is written.
    // On a specless polyglot root where the marker-led detection lands on
    // the MINORITY language the verb REFUSES with the shipped
    // `language mismatch:` class, never a silent exit-0 misseed (the loud
    // 0.6.69 contract). The pin law of issue #38 stands: the pin selects
    // the driver; the census only validates.
    let exclusions = loaded
        .as_ref()
        .map(spec::exclusion_paths)
        .unwrap_or_default();
    if let Some(finding) = language::agreement_finding(root_path, detected, &exclusions) {
        return Err(finding);
    }

    if !scan::driver_available(detected) {
        return Err(format!(
            "{} toolchain not found (run 'archspec doctor' to diagnose drivers)",
            detected.as_str()
        ));
    }

    let model = scan::extract(detected, root_path)?;

    // Both [project] keys the loaded spec carries are CONSUMER
    // declarations, not model-derived facts, so the reseed preserves the
    // `excluded_sources` entries EXACTLY as it preserves the language
    // (GitHub issue #43 part 2): the following `archspec scan` passes the
    // mismatch check clean instead of losing the entries to hand-restore.
    // Without a prior spec nothing is preserved and the seed bytes stay
    // identical to the shape shipped before.
    let preserved: &[spec::ExcludedSource] = loaded
        .as_ref()
        .map(|loaded_spec| loaded_spec.project.excluded_sources.as_slice())
        .unwrap_or_default();
    let content = update::seed_spec(&model, preserved);
    std::fs::write(&spec_path, content.as_bytes())
        .map_err(|err| format!("failed to write spec: {}: {err}", spec_path.display()))?;

    println!("generated {}", spec_path.display());
    // The reseed census (GitHub issue #57): regeneration preserves EXACTLY
    // ONE hand-written thing — the `[project]` `excluded_sources` entries —
    // and silently drops every other hand-written section. When a prior spec
    // was overwritten, enumerate RETAINED versus DROPPED on stdout: one line
    // per non-empty section class with its entry count, plus a recovery
    // pointer. The census never enters the written spec (report-only stdout),
    // and a purely generated prior spec prints no lines at all.
    if let Some(loaded_spec) = loaded.as_ref() {
        for line in update::reseed_census(loaded_spec, &content) {
            println!("{line}");
        }
    }
    println!("review and trim the seed spec, then run archspec verify");
    Ok(())
}
