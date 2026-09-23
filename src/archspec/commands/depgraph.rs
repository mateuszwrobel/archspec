use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::depgraph;
use crate::archspec::language;
use crate::archspec::scan;
use std::path::Path;

const GRAPH_FORMATS: &[&str] = &["mermaid", "plantuml"];

pub const HELP: &str = "usage: archspec depgraph <modules|api-usage|submodules> [path] [--format <fmt>] [--parent <module>] [--output <path>] [--check]\ncurrent-state dependency views of a project, straight from the extracted model\n\n  modules        top-level module dependency graph; one level deeper when that projection folds to a single node (default format: mermaid)\n  api-usage      Markdown table of used APIs grouped by target module (markdown only)\n  submodules     graph of one module's immediate children (requires --parent)\n  path           directory to scan (default: current directory)\n  --format <fmt> mermaid or plantuml for graphs; markdown for api-usage\n  --parent <m>   parent top-level module to expand (submodules view only)\n  --output <p>   write the body to that file (default: stdout); `--output -` writes to stdout and creates no file — never a path positional\n  --check        compare the body to its destination without writing; exit non-zero if stale or missing; a fresh check prints `ok: <path> up to date`\n\nlanguages      rust and csharp render directly; when the model has no module tier the command refuses with: depgraph needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of\n\ndepgraph does not read archspec.toml [output] — an artefact path comes only from an explicit --output.\n\nNode labels fold units — unit ownership lives in the `root_module_declarations` map that `archspec scan` emits in its scan JSON.\n\nThe `mod` label is a unit's root-module bucket: the namespace-less root where the composition/wiring code lives.\n\nTest-gated/soft edges (test namespaces and their consumers) can appear in depgraph projections while being excluded from `depend_on` comparison.\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "format",
                takes_value: true,
            },
            FlagSpec {
                name: "parent",
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

    let view = parsed
        .positionals
        .first()
        .map(String::as_str)
        .ok_or_else(|| {
            "usage: archspec depgraph <modules|api-usage|submodules> [path] [--format <fmt>] [--parent <module>] [--output <path>]".to_string()
        })?;
    if !matches!(view, "modules" | "api-usage" | "submodules") {
        return Err(format!(
            "unknown depgraph view: {view} (supported: modules, api-usage, submodules)"
        ));
    }

    let rest: Vec<&String> = parsed.positionals.iter().skip(1).collect();
    if rest.len() > 1 {
        return Err("expected at most one path argument".to_string());
    }
    let root = rest.first().map(|path| path.as_str()).unwrap_or(".");
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("path does not exist: {root}"));
    }
    if !root_path.is_dir() {
        return Err(format!("path is not a directory: {root}"));
    }

    let default_format = if view == "api-usage" { "markdown" } else { "mermaid" };
    let format = parsed
        .values
        .get("format")
        .map(String::as_str)
        .unwrap_or(default_format);
    if view == "api-usage" {
        if format != "markdown" {
            return Err(format!(
                "unsupported format for api-usage: {format} (supported: markdown)"
            ));
        }
    } else if !GRAPH_FORMATS.contains(&format) {
        return Err(format!(
            "unsupported format: {format} (supported: mermaid, plantuml)"
        ));
    }

    let parent = parsed.values.get("parent").map(String::as_str);
    if view == "submodules" && parent.is_none() {
        return Err("submodules view requires --parent <module>".to_string());
    }
    if view != "submodules" && parent.is_some() {
        return Err("--parent is only valid for the submodules view".to_string());
    }

    let detected = language::detect(root_path)
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    if !scan::driver_available(detected) {
        return Err(format!(
            "detected language '{}' but the {} driver is not available (run 'archspec doctor')",
            detected.as_str(),
            detected.as_str()
        ));
    }
    let model = scan::extract(detected, root_path)?;

    // Capability guard, keyed on the model's module tier (not a hardcoded
    // language): every view projects the module tier, so a model without one is
    // refused up front with a Go-actionable sentence instead of an internal-shape
    // complaint. A rust/csharp model always carries the tier and is untouched; a
    // Go tree acquires the tier through its package references.
    if !model.has_module_tier() {
        return Err(depgraph::MODULE_TIER_REQUIREMENT.to_string());
    }

    let body = match view {
        "api-usage" => depgraph::api_usage_markdown(&model),
        "modules" => {
            let graph = depgraph::modules_graph(&model);
            render_graph(&graph, format)?
        }
        "submodules" => {
            let parent = parent.expect("checked above");
            let graph = depgraph::submodules_graph(&model, parent)?;
            render_graph(&graph, format)?
        }
        _ => unreachable!("view validated above"),
    };

    // `depgraph` has no `[output]` key: its only destination is `--output`,
    // which echoes the universal `wrote` line like every path write (blind4
    // D01); with no destination at all the body prints with no echo.
    let route = config::route(
        parsed.values.get("output").map(String::as_str),
        None,
        root_path,
        config::is_human(format),
    );
    let check = parsed.switches.contains("check");
    config::emit(
        &body,
        route,
        check,
        "dependency graph",
        "depgraph",
        None,
    )
}

fn render_graph(graph: &depgraph::ModuleGraph, format: &str) -> Result<String, String> {
    Ok(match format {
        "mermaid" => depgraph::render_mermaid(graph),
        "plantuml" => depgraph::render_plantuml(graph),
        _ => unreachable!("format validated above"),
    })
}
