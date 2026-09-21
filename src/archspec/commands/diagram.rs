use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::diagram as render;
use crate::archspec::model;
use crate::archspec::spec;
use std::path::Path;

const FORMATS: &[&str] = &["mermaid", "plantuml"];
const SOURCES: &[&str] = &["spec", "scan"];

pub const HELP: &str = "usage: archspec diagram [path] [--source <spec|scan> <scan-path>] [--format <mermaid|plantuml>] [--output <path>] [--check]\nrender the model (declared spec or scan artefact) as a diagram\n\n  path                  project directory (default: current directory)\n  --source <spec|scan>  model source; with scan, bind the artefact path next (default: spec)\n  --format <fmt>        mermaid or plantuml (default: mermaid)\n  --output <path>       write the diagram to that file (default: stdout unless archspec.toml [output] sets a destination)\n  --check               compare the diagram to its destination without writing; exit non-zero if stale or missing\n";

/// `--source scan` binds the token immediately after `scan` as the artefact
/// path, so it never lands in the positional list (which stays the project
/// dir for the governing-spec lookup).
fn bind_scan_artefact(args: &[String]) -> Result<(Option<String>, Vec<String>), String> {
    let mut artefact = None;
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut it = args.iter().cloned().peekable();
    while let Some(arg) = it.next() {
        if arg == "--source" {
            out.push(arg);
            let value = it
                .next()
                .ok_or_else(|| "flag --source requires a value".to_string())?;
            out.push(value.clone());
            if value == "scan" {
                if let Some(next) = it.peek() {
                    if !next.starts_with("--") {
                        artefact = Some(next.clone());
                        it.next();
                    }
                }
            }
        } else {
            out.push(arg);
        }
    }
    Ok((artefact, out))
}

pub fn run(args: &[String]) -> Result<(), String> {
    let (scan_artefact, args) = bind_scan_artefact(args)?;
    let parsed = cli::parse(
        &args,
        &[
            FlagSpec {
                name: "source",
                takes_value: true,
            },
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
        .unwrap_or("mermaid");
    if !FORMATS.contains(&format) {
        return Err(format!(
            "unsupported format: {format} (supported: mermaid, plantuml)"
        ));
    }
    let source = parsed
        .values
        .get("source")
        .map(String::as_str)
        .unwrap_or("spec");
    if !SOURCES.contains(&source) {
        return Err(format!(
            "unsupported source: {source} (supported: spec, scan)"
        ));
    }

    let project_root = Path::new(project_dir(&parsed));
    if !project_root.exists() {
        return Err(format!("path does not exist: {}", project_root.display()));
    }
    if !project_root.is_dir() {
        return Err(format!("path is not a directory: {}", project_root.display()));
    }
    let config = config::load(project_root)?;

    let diagram = match source {
        "scan" => {
            let artefact = scan_artefact
                .ok_or_else(|| "--source scan requires an artefact path".to_string())?;
            render_scan(&artefact, project_root, format)?
        }
        "spec" => {
            let loaded = spec::load(project_root)?;
            if loaded.modules.is_empty() {
                return Err("no model content to render".to_string());
            }
            match format {
                "mermaid" => render::render_spec_mermaid(&loaded),
                "plantuml" => render::render_spec_plantuml(&loaded),
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    };

    let destination = config::resolve(
        parsed.values.get("output").map(String::as_str),
        config.diagram.as_deref(),
        project_root,
    );
    let check = parsed.switches.contains("check");
    config::emit(&diagram, destination, check, "diagram", "diagram")
}

fn project_dir(parsed: &cli::Args) -> &str {
    parsed
        .positionals
        .first()
        .map(String::as_str)
        .unwrap_or(".")
}

fn render_scan(artefact: &str, project_path: &Path, format: &str) -> Result<String, String> {
    let artefact_path = Path::new(artefact);
    if !artefact_path.exists() {
        return Err(format!("scan artefact not found: {artefact}"));
    }
    let content = std::fs::read_to_string(artefact_path)
        .map_err(|err| format!("invalid scan artefact: {artefact} ({err})"))?;
    let loaded = model::Model::from_json(&content)
        .map_err(|err| format!("invalid scan artefact: {artefact} ({err})"))?;
    if loaded.units.is_empty() && loaded.external.is_empty() {
        return Err("no model content to render".to_string());
    }

    let governing = if project_path.join(spec::SPEC_FILE).exists() {
        Some(spec::load(project_path)?)
    } else {
        None
    };

    match format {
        "mermaid" => Ok(render::render_scan_mermaid(&loaded, governing.as_ref())),
        "plantuml" => Ok(render::render_scan_plantuml(&loaded, governing.as_ref())),
        _ => unreachable!(),
    }
}
