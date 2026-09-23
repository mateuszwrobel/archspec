use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::language;
use crate::archspec::scan;
use std::path::Path;

mod csharp;
mod go;
mod mermaid;
mod plantuml;
mod scanner;
mod structural;
mod tree;

/// File-level dependency graph shared by all language file scanners (Rust,
/// C#, Go). `nodes` are source-file paths relative to the scanned root;
/// `edges` are resolved file-to-file dependency edges: imports plus, for Rust,
/// module declarations (declaring file → declared child file, collapsed per
/// pair). Renderers (mermaid/plantuml) group nodes into folder subgraphs and
/// draw the edges.
pub struct FileGraph {
    pub nodes: std::collections::BTreeSet<String>,
    pub edges: std::collections::BTreeSet<(String, String)>,
}

const FORMATS: &[&str] = &["mermaid", "plantuml"];

/// The `tree` refusal when the scanned model carries no module tier: the view
/// projects containment and nothing else, so there is nothing to render.
/// Mirrors the depgraph tier sentence (`depgraph::MODULE_TIER_REQUIREMENT`);
/// the refusal states a model fact, not a language verdict.
pub const TREE_MODULE_TIER_REQUIREMENT: &str = "inspect tree needs the module tier, which this model has none of; the module tier is derived from the tree's package references, which this tree records none of, 'inspect scanner' renders the unit-tier model";

pub const HELP: &str = "usage: archspec inspect [tree|scanner] [path] [--format <mermaid|plantuml>] [--output <path>] [--check]\nzero-config file-level import map of a project\n\n  tree|scanner      structural model mode (default: file-level graph)\n  path              directory to scan (default: current directory)\n  --format <fmt>    mermaid or plantuml (default: mermaid)\n  --output <path>   write the diagram to that file (default: stdout unless archspec.toml [output] sets a destination); `--output -` writes to stdout and creates no file — never a path positional\n  --check           compare the diagram to its destination without writing; exit non-zero if stale or missing; a fresh check prints `ok: <path> up to date`\n";

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

    let format = parsed
        .values
        .get("format")
        .map(String::as_str)
        .unwrap_or("mermaid");
    if !FORMATS.contains(&format) {
        return Err(format!(
            "unsupported format: {format} (supported: mermaid, plantuml)"
        ));
    }
    let check = parsed.switches.contains("check");

    match parsed.positionals.first().map(String::as_str) {
        Some("tree") => run_structural(false, &parsed, format, check),
        Some("scanner") => run_structural(true, &parsed, format, check),
        _ => run_file_level(&parsed, format, check),
    }
}

/// The file-instead-of-directory refusal. Every inspect mode scans a
/// directory; this names the modes that exist (file-level import maps for
/// rust/csharp/go, structural `tree`/`scanner` model views) and points at the
/// directory form instead of the bare "not a directory".
fn not_a_directory(root: &str) -> String {
    let dir = match Path::new(root).parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.display().to_string(),
        _ => ".".to_string(),
    };
    format!(
        "path is not a directory: {root}; every inspect mode takes a directory — \
         file-level import map modes (rust, csharp, go; default mermaid/plantuml output) and \
         structural modes 'inspect tree|scanner' (model views; guard surfaces live in \
         scan/verify/report/update/diagram); pass a directory, e.g. archspec inspect {dir}"
    )
}

fn run_structural(
    include_unit_edges: bool,
    parsed: &cli::Args,
    format: &str,
    check: bool,
) -> Result<(), String> {
    let positionals: Vec<&String> = parsed.positionals.iter().skip(1).collect();
    if positionals.len() > 1 {
        return Err("expected at most one path argument".to_string());
    }
    let root = positionals
        .first()
        .map(|path| path.as_str())
        .unwrap_or(".");
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("path does not exist: {root}"));
    }
    if !root_path.is_dir() {
        return Err(not_a_directory(root));
    }

    let language = language::detect(root_path)
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    if !scan::driver_available(language) {
        return Err(format!(
            "detected language '{}' but the {} driver is not available (run 'archspec doctor')",
            language.as_str(),
            language.as_str()
        ));
    }
    // Structural branching is content-based, not language-keyed: the tree
    // view projects only the module tier, so it renders containment wherever
    // the model carries one and is refused by that
    // missing fact where it does not. The scanner view
    // renders whatever units, edges and soft structure the model carries and
    // is never refused for being tier-less.
    // The tree view extracts the model exactly as `archspec scan` does, so
    // the documented inspect==scan agreement holds. The file-level map below
    // stays model-free — those are file-granular discovery facts produced
    // without model extraction.
    let model = scan::extract(language, root_path)?;
    if !include_unit_edges && !model.has_module_tier() {
        return Err(TREE_MODULE_TIER_REQUIREMENT.to_string());
    }
    // `--format` selects the model renderer exactly as it selects the
    // file-level renderer: mermaid stays the default, plantuml emits the
    // `@startuml` diagram of the same model content.
    let diagram = match format {
        "mermaid" => structural::render_model(&model, include_unit_edges),
        "plantuml" => structural::render_model_plantuml(&model, include_unit_edges),
        _ => unreachable!(),
    };
    let config = config::load(root_path)?;
    let route = config::route(
        parsed.values.get("output").map(String::as_str),
        config.inspect.as_deref(),
        root_path,
        config::is_human(format),
    );
    config::emit(&diagram, route, check, "diagram", "inspect", None)
}

fn run_file_level(parsed: &cli::Args, format: &str, check: bool) -> Result<(), String> {
    cli::exactly_one_positional(parsed)?;

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
        return Err(not_a_directory(root));
    }

    let language = language::detect(root_path)
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    if !scan::driver_available(language) {
        return Err(format!(
            "detected language '{}' but the {} driver is not available (run 'archspec doctor')",
            language.as_str(),
            language.as_str()
        ));
    }

    let graph = match language {
        language::Language::Csharp => csharp::scan(root_path)?,
        language::Language::Rust => scanner::scan(root_path)?,
        language::Language::Go => go::scan(root_path)?,
    };

    let diagram = match format {
        "mermaid" => mermaid::render(&graph),
        "plantuml" => plantuml::render(&graph),
        _ => unreachable!(),
    };
    let config = config::load(root_path)?;
    let route = config::route(
        parsed.values.get("output").map(String::as_str),
        config.inspect.as_deref(),
        root_path,
        config::is_human(format),
    );
    config::emit(&diagram, route, check, "diagram", "inspect", None)
}

/// Render the file-level import map for a language as a mermaid diagram.
/// Every supported language has a file scanner; `None` means "no diagram"
/// (report omits it) and is reserved for languages without one.
pub fn render_import_map(
    root: &std::path::Path,
    language: crate::archspec::language::Language,
) -> Result<Option<String>, String> {
    let graph = match language {
        crate::archspec::language::Language::Csharp => csharp::scan(root)?,
        crate::archspec::language::Language::Rust => scanner::scan(root)?,
        crate::archspec::language::Language::Go => go::scan(root)?,
    };
    Ok(Some(mermaid::render(&graph)))
}
