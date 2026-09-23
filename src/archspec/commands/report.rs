use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::inspect;
use crate::archspec::language::{self, Language};
use crate::archspec::report;
use crate::archspec::scan;
use crate::archspec::spec;
use crate::archspec::verify::compare;
use super::verify::RULE_VIOLATION_REPORTED;
use std::path::Path;

pub const HELP: &str = "usage: archspec report [path] [--format <text|markdown|json>] [--output <path>] [--check] [--pretty]\nproduce a text, markdown, or json architecture report (diff + metrics)\n\n  path             project directory to report on (default: current directory)\n  --format <fmt>   text, markdown, or json (default: text)\n  --output <path>  write the report to that file (default: stdout unless archspec.toml [output] sets a destination); `--output -` writes to stdout and creates no file — never a path positional\n  --check          compare the report to its destination without writing; exit non-zero if stale, missing, or violations are present; a fresh check prints `ok: <path> up to date`. A `--check` compares the canonical rendering unless `--format` names another; to verify a `--format`-generated artefact, pass the same `--format` to `--check`.\n  --pretty         accepted for symmetry with `scan`; report json is already indented and no body changes\n\nA configured report destination is provisioned for the default text rendering: a run naming another `--format` renders to stdout and never writes the destination.\n\nThe diagram embedded in a markdown report is file-granular; the module-granular graph is `archspec depgraph modules`.\n\nexit 0 when the code satisfies its spec; exit 1 on rule violations or operational errors\n";

/// The advice half of the guidance sentence (owner: the `--check` line of
/// `report --help`, welded to `docs/archspec/config.md`) stated where the
/// mistake happens: appended to a stale finding that compared the canonical
/// text rendering, absent when a format-named rendering was compared —
/// there, the exact-form regenerate hint already is the advice (blind5 D01).
const CANONICAL_ADVICE: &str =
    "; pass the same --format to --check to verify a --format-generated artefact";

/// The parenthetical a stale report `--check` prints: it names the rendering
/// the comparison actually made and hints that rendering's exact command
/// form, so following the hint regenerates the compared body byte-exactly
/// instead of rewriting it in another format (blind5 D01). The rendering is
/// the one already resolved for this run — the naming states no new routing
/// decision; the canonical text rendering regenerates bare and carries the
/// advice clause, a format-named rendering carries the `--format` suffix and
/// no clause.
fn stale_finding(format: &str) -> String {
    if format == "text" {
        format!(
            "compared the text rendering; regenerate with: archspec report{CANONICAL_ADVICE}"
        )
    } else {
        format!(
            "compared the {format} rendering; regenerate with: archspec report --format {format}"
        )
    }
}

/// `archspec report [path] [--format <text|markdown|json>] [--output <path>]`
/// (commands/report/cli.md). Spec-driven: the spec's `[project] language`
/// selects the extractor and its declared model is the comparison side.
/// Rule violations are report content and exit non-zero after the report is
/// emitted; only operational failures prevent the report (commands/report/errors.md).
pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "format",
                takes_value: true,
            },
            FlagSpec {
                name: "output",
                takes_value: true,
            },
            FlagSpec {
                name: "check",
                takes_value: false,
            },
            FlagSpec {
                // Accepted for one-flag-set symmetry with `scan`; report
                // json is already indented and other formats have no json
                // layout to change (blind4 D03).
                name: "pretty",
                takes_value: false,
            },
        ],
    )?;
    cli::exactly_one_positional(&parsed)?;

    let format = parsed
        .values
        .get("format")
        .map(String::as_str)
        .unwrap_or("text");
    if format != "text" && format != "markdown" && format != "json" {
        return Err(format!(
            "unsupported format: {format} (supported: text, markdown, json)"
        ));
    }

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

    let loaded = spec::load_report(root_path)?;
    let spec_file = root_path.join(spec::SPEC_FILE);
    let language = language_from_str(&loaded.project.language, &spec_file)?;

    if !scan::driver_available(language) {
        return Err(format!(
            "{} driver unavailable (toolchain not found); run 'archspec doctor' to diagnose",
            language.as_str()
        ));
    }
    if !language::has_sources(root_path, language) {
        return Err(format!(
            "no {} sources found under: {root}",
            language.as_str()
        ));
    }

    let model = scan::extract(language, root_path)?;

    let mapping = compare::map_units(&model.units, &loaded.modules);
    let diff = compare::compare(
        &mapping,
        &model,
        &loaded.modules,
        &loaded.stereotype,
        &loaded.constraint,
    );
    let metrics = report::compute_metrics(&model, &mapping, &loaded.modules, &diff);

    let config = config::load(root_path)?;
    let diagram = if format == "markdown" {
        inspect::render_import_map(root_path, language)?
    } else {
        None
    };
    let rendered = match format {
        "markdown" => report::render_markdown(
            &loaded.project.language,
            &metrics,
            &diff,
            &model.roles,
            diagram.as_deref(),
        ),
        "json" => report::render_json(&loaded.project.language, &metrics, &diff, &model.roles),
        _ => report::render_text(&loaded.project.language, &metrics, &diff, &model.roles),
    };

    let output_flag = parsed.values.get("output");
    let check = parsed.switches.contains("check");
    let mut route = config::route(
        output_flag.map(String::as_str),
        config.report.as_deref(),
        root_path,
        config::is_human(format),
    );
    // Precedence: `--output` always wins. Otherwise the configured destination
    // receives only the format it is provisioned for — the default text report
    // (what a plain `report` writes and `--check` compares). An explicit
    // `--format` that differs from it goes to stdout instead of being written
    // into the text artefact (serializing JSON into `report.md` would clobber
    // the committed report and make `--check` flag it stale), and stdout names
    // the destination it skipped — in a human body only; a machine body stays
    // byte-clean (D02). `--check` never
    // writes, so its destination resolution is unchanged.
    if output_flag.is_none()
        && !check
        && config.report.is_some()
        && parsed.values.contains_key("format")
        && format != "text"
    {
        route.destination = None;
    }
    config::emit(
        &rendered,
        route,
        check,
        "report",
        "report",
        Some(&stale_finding(format)),
    )?;
    if diff.fails(false) {
        return Err(RULE_VIOLATION_REPORTED.to_string());
    }
    Ok(())
}

fn language_from_str(value: &str, spec_file: &Path) -> Result<Language, String> {
    match value {
        "rust" => Ok(Language::Rust),
        "csharp" => Ok(Language::Csharp),
        "go" => Ok(Language::Go),
        other => Err(format!(
            "invalid spec: {}: unknown language \"{other}\"",
            spec_file.display()
        )),
    }
}
