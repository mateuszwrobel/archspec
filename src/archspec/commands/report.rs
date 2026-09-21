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

pub const HELP: &str = "usage: archspec report [path] [--format <text|markdown|json>] [--output <path>] [--check]\nproduce a text, markdown, or json architecture report (diff + metrics)\n\n  path             project directory to report on (default: current directory)\n  --format <fmt>   text, markdown, or json (default: text)\n  --output <path>  write the report to that file (default: stdout unless archspec.toml [output] sets a destination)\n  --check          compare the report to its destination without writing; exit non-zero if stale, missing, or violations are present\n\nexit 0 when the code satisfies its spec; exit 1 on rule violations or operational errors\n";

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
            diagram.as_deref(),
        ),
        "json" => report::render_json(&loaded.project.language, &metrics, &diff),
        _ => report::render_text(&loaded.project.language, &metrics, &diff),
    };

    let output_flag = parsed.values.get("output");
    let check = parsed.switches.contains("check");
    let mut destination = config::resolve(
        output_flag.map(String::as_str),
        config.report.as_deref(),
        root_path,
    );
    // Precedence: `--output` always wins. Otherwise the configured destination
    // receives only the format it is provisioned for — the default text report
    // (what a plain `report` writes and `--check` compares). An explicit
    // `--format` that differs from it goes to stdout instead of being written
    // into the text artefact (serializing JSON into `report.md` would clobber
    // the committed report and make `--check` flag it stale). `--check` never
    // writes, so its destination resolution is unchanged.
    if output_flag.is_none()
        && !check
        && config.report.is_some()
        && parsed.values.contains_key("format")
        && format != "text"
    {
        destination = None;
    }
    config::emit(&rendered, destination, check, "report", "report")?;
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
