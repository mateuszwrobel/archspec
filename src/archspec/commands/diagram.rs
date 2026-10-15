use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::diagram as render;
use crate::archspec::language::{self, Language};
use crate::archspec::model;
use crate::archspec::scan;
use crate::archspec::spec;
use std::collections::BTreeSet;
use std::path::Path;

const FORMATS: &[&str] = &["mermaid", "plantuml"];
const SOURCES: &[&str] = &["spec", "scan"];
/// The render-extension vocabulary the batch orphan scan owns: files with
/// these extensions in a modules dir are the writer's territory; anything
/// else is foreign and never an orphan (D12).
const RENDER_EXTENSIONS: &[&str] = &["mmd", "puml"];

pub const HELP: &str = "usage: archspec diagram [path] [--source <spec|scan> <scan-path>] [--format <mermaid|plantuml>] [--output <path>] [--check]\nrender the model (declared spec or scan artefact) as a diagram\n\n  path                  project directory (default: current directory)\n  --source <spec|scan>  model source; with scan, bind the artefact path next (default: spec)\n  --format <fmt>        mermaid or plantuml (default: mermaid)\n  --output <path>       write the diagram to that file (default: stdout unless archspec.toml [output] sets a destination); `--output -` writes to stdout and creates no file — never a path positional\n  --tiers <list>        scan source only: additive comma-separated tiers to render, from units, modules (default: units); modules adds the module-tier cluster on top of the unchanged unit render\n  --module <claim>      spec source only: render that one module claim's territory (its members, edges and contract marks, adjudicated like verify); an unknown claim is a usage error (exit 64)\n  --modules-dir <dir>   spec source only: write one stable claim-named diagram per module claim into <dir>; the batch writes atomically; --check flags stale, missing, and orphan files without writing\n  --check               compare the diagram to its destination without writing; exit non-zero if stale or missing; a fresh check prints `ok: <path> up to date`\n\nArtefact form: `archspec diagram --source scan docs/archspec/scan.json` — with `--source scan`, the argument is a scan-JSON artefact file written by `archspec scan`, not a project directory.\n\nThe scan source is exploratory: `archspec diagram --source scan <path>` with no `--output` renders to stdout, states the skip with `note: destination <path> not written`, and never touches the configured destination; pass `--output <path>` to write that render.\n";

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
            if value == "scan"
                && let Some(next) = it.peek()
                    && !next.starts_with("--") {
                        artefact = Some(next.clone());
                        it.next();
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
                name: "tiers",
                takes_value: true,
            },
            FlagSpec {
                name: "check",
                takes_value: false,
            },
            FlagSpec {
                name: "module",
                takes_value: true,
            },
            FlagSpec {
                name: "modules-dir",
                takes_value: true,
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
    // `--tiers` selects rendered tiers on the scan surface only; every
    // rejected value is a usage error naming what it broke and the
    // supported tiers (workplan D7).
    let tiers = match parsed.values.get("tiers").map(String::as_str) {
        Some(selection) => {
            let tiers = render::TierSelection::parse(selection).map_err(cli::usage_error)?;
            if source != "scan" {
                return Err(cli::usage_error(
                    "--tiers requires --source scan — the spec source renders declared components at full depth (supported tiers: units, modules)",
                ));
            }
            tiers
        }
        None => render::TierSelection::units_only(),
    };

    let project_root = Path::new(project_dir(&parsed));
    if !project_root.exists() {
        return Err(format!("path does not exist: {}", project_root.display()));
    }
    if !project_root.is_dir() {
        return Err(format!("path is not a directory: {}", project_root.display()));
    }
    let config = config::load(project_root)?;

    // Per-module views (workplan archspec_module_public_api_diagrams, D11/D12):
    // both flags require the governing spec — the scan artefact carries no
    // expose verdicts — and the pair is mutually exclusive (one claim or the
    // whole batch, never both in one run). Each rejection is a usage error
    // naming the rule it broke.
    let module = parsed.values.get("module");
    let modules_dir = parsed.values.get("modules-dir");
    if module.is_some() && modules_dir.is_some() {
        return Err(cli::usage_error(
            "--module and --modules-dir are mutually exclusive — render one claim or write the batch, not both",
        ));
    }
    if source == "scan" && (module.is_some() || modules_dir.is_some()) {
        return Err(cli::usage_error(
            "--module and --modules-dir require the spec source — a scan artefact carries no expose verdicts",
        ));
    }
    if let Some(claim) = module {
        let (loaded, model) = load_and_scan(project_root)?;
        if !spec::claim_names(&loaded.modules).iter().any(|name| name == claim) {
            return Err(cli::usage_error(format!(
                "unknown module: no claim of the spec carries that name: {claim}"
            )));
        }
        let diagram = match format {
            "mermaid" => render::render_module_mermaid(&model, &loaded, claim),
            "plantuml" => render::render_module_plantuml(&model, &loaded, claim),
            _ => unreachable!(),
        };
        let regen = format!("diagram --module {claim}");
        let route = config::route(
            parsed.values.get("output").map(String::as_str),
            config.diagram.as_deref(),
            project_root,
            config::is_human(format),
        );
        return config::emit(
            &diagram,
            route,
            parsed.switches.contains("check"),
            "diagram",
            &regen,
            None,
        );
    }
    if let Some(dir) = modules_dir {
        let (loaded, model) = load_and_scan(project_root)?;
        return write_modules_dir(
            Path::new(dir),
            &loaded,
            &model,
            format,
            parsed.switches.contains("check"),
        );
    }

    let diagram = match source {
        "scan" => {
            let artefact = scan_artefact
                .ok_or_else(|| "--source scan requires an artefact path".to_string())?;
            render_scan(&artefact, project_root, format, &tiers)?
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

    let mut route = config::route(
        parsed.values.get("output").map(String::as_str),
        config.diagram.as_deref(),
        project_root,
        config::is_human(format),
    );
    let check = parsed.switches.contains("check");
    // blind4 D02: the destination belongs to the canonical source (spec mode).
    // An exploratory `--source scan` render with no explicit `--output` prints
    // to stdout and never touches the configured destination — the rule
    // `report` follows on its format axis, applied to the source axis — so a
    // committed spec-mode artefact cannot be clobbered by an exploratory run.
    // With `--output Q` the old capability stays: the body goes to Q (and the
    // destination stays untouched). `--check` never writes, so its destination
    // resolution is unchanged.
    if source == "scan" && !check && !parsed.values.contains_key("output") {
        route.destination = None;
    }
    config::emit(&diagram, route, check, "diagram", "diagram", None)
}

fn project_dir(parsed: &cli::Args) -> &str {
    parsed
        .positionals
        .first()
        .map(String::as_str)
        .unwrap_or(".")
}

/// Governing spec plus the scanned model — the per-module views read the
/// same facts `verify` does (the pair law), so a dashed crossing in the
/// render is exactly a red crossing in the report.
fn load_and_scan(root: &Path) -> Result<(spec::Spec, model::Model), String> {
    let loaded = spec::load(root)?;
    if loaded.modules.is_empty() {
        return Err("no model content to render".to_string());
    }
    let language = language::from_name(&loaded.project.language).ok_or_else(|| {
        format!("unsupported language: {}", loaded.project.language)
    })?;
    if !scan::driver_available(language) {
        return Err(format!(
            "no {} toolchain found (driver missing); run 'archspec doctor' to diagnose",
            language_display(language)
        ));
    }
    let model = scan::extract_checked(language, root)?;
    Ok((loaded, model))
}

fn language_display(language: Language) -> &'static str {
    match language {
        Language::Rust => "Rust",
        Language::Go => "Go",
        Language::Csharp => "C#",
    }
}

/// The batch write law (D12): one file per declared claim, named by the
/// escape, written temp-plus-rename in the destination directory so a failed
/// run leaves every file at previous-or-complete bytes and no temp residue.
/// `--check` compares bytes, flags missing or stale files, and flags orphans
/// — render-extension entries whose decoded name no claim declares.
fn write_modules_dir(
    dir: &Path,
    loaded: &spec::Spec,
    model: &model::Model,
    format: &str,
    check: bool,
) -> Result<(), String> {
    let extension = if format == "plantuml" { "puml" } else { "mmd" };
    let claims = spec::claim_names(&loaded.modules);
    if check {
        let mut findings: Vec<String> = Vec::new();
        for claim in &claims {
            let path = dir.join(format!("{}.{extension}", render::claim_filename(claim)));
            let expected = render_one(model, loaded, format, claim);
            match std::fs::read(&path) {
                Ok(existing) if existing.as_slice() == expected.as_bytes() => {}
                Ok(_) => findings.push(format!(
                    "out of date diagram: {} differs from generated output (regenerate with: archspec diagram --modules-dir {})",
                    path.display(),
                    dir.display()
                )),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => findings.push(format!(
                    "out of date diagram: {} is missing (generate with: archspec diagram --modules-dir {})",
                    path.display(),
                    dir.display()
                )),
                Err(err) => {
                    return Err(format!("failed to read diagram {}: {err}", path.display()));
                }
            }
        }
        let declared: BTreeSet<String> = claims.iter().cloned().collect();
        let entries = read_dir_entries(dir)?;
        for name in entries {
            if !has_render_extension(&name) {
                continue; // foreign files are not the writer's territory
            }
            let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&name);
            let decoded = render::claim_from_filename(stem);
            if !declared.contains(&decoded) {
                findings.push(format!(
                    "orphan diagram: {} decodes to \"{decoded}\" which no claim of the spec declares",
                    dir.join(&name).display()
                ));
            }
        }
        if !findings.is_empty() {
            return Err(findings.join("\n"));
        }
        println!("ok: {} up to date", dir.display());
        return Ok(());
    }
    std::fs::create_dir_all(dir)
        .map_err(|err| cli::operational_error(format!("cannot create {}: {err}", dir.display())))?;
    let mut failures: Vec<String> = Vec::new();
    for claim in &claims {
        let path = dir.join(format!("{}.{extension}", render::claim_filename(claim)));
        let content = render_one(model, loaded, format, claim);
        match atomic_write(dir, &path, content.as_bytes()) {
            Ok(()) => {
                println!("wrote {}", path.display());
            }
            Err(err) => failures.push(format!("{}: {err}", path.display())),
        }
    }
    if !failures.is_empty() {
        return Err(cli::operational_error(format!(
            "failed to write diagram batch to {}: {}",
            dir.display(),
            failures.join("; ")
        )));
    }
    Ok(())
}

fn render_one(model: &model::Model, loaded: &spec::Spec, format: &str, claim: &str) -> String {
    match format {
        "plantuml" => render::render_module_plantuml(model, loaded, claim),
        _ => render::render_module_mermaid(model, loaded, claim),
    }
}

fn read_dir_entries(dir: &Path) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(names),
        Err(err) => {
            return Err(format!("failed to read directory {}: {err}", dir.display()));
        }
    };
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read directory {}: {err}", dir.display()))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(names)
}

fn has_render_extension(name: &str) -> bool {
    RENDER_EXTENSIONS
        .iter()
        .any(|extension| name.ends_with(&format!(".{extension}")))
}

/// One atomic file write: temp in the destination directory (hidden so the
/// orphan scan never sees it), then rename over the target. A failed run
/// leaves the target at previous-or-complete bytes and no temp residue.
fn atomic_write(dir: &Path, path: &Path, content: &[u8]) -> Result<(), String> {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = dir.join(format!(".{file_name}.tmp"));
    let result = std::fs::write(&temp, content).and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|err| format!("write failed: {err}"))
}

fn render_scan(
    artefact: &str,
    project_path: &Path,
    format: &str,
    tiers: &render::TierSelection,
) -> Result<String, String> {
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
        "mermaid" => Ok(render::render_scan_mermaid(&loaded, governing.as_ref(), tiers)),
        "plantuml" => Ok(render::render_scan_plantuml(&loaded, governing.as_ref(), tiers)),
        _ => unreachable!(),
    }
}
