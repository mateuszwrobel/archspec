use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::capability;
use crate::archspec::config;
use crate::archspec::diagram as render;
use crate::archspec::language::{self, Language};
use crate::archspec::model;
use crate::archspec::scan;
use crate::archspec::spec;
use std::collections::BTreeSet;
use std::path::Path;

const FORMATS: &[&str] = &["mermaid", "plantuml", "dot", "svg", "viewer"];
const SOURCES: &[&str] = &["spec", "scan"];
/// The render-extension vocabulary the batch orphan scan owns: files with
/// these extensions in a modules dir are the writer's territory; anything
/// else is foreign and never an orphan (D12).
const RENDER_EXTENSIONS: &[&str] = &["mmd", "puml"];

pub const HELP: &str = "usage: archspec diagram [path] [--source <spec|scan> <scan-path>] [--format <mermaid|plantuml|dot|svg|viewer>] [--layer <direct|indirect|both>] [--level <level>] [--output <path>] [--check]\nrender the model (declared spec or scan artefact) as a diagram\n\n  path                  project directory (default: current directory)\n  --source <spec|scan>  model source; with scan, bind the artefact path next (default: spec)\n  --format <fmt>        mermaid, plantuml, dot, svg or viewer (default: mermaid); dot emits graphviz source and svg a hand-emitted layout — the SVG materializes from the dot body by piping it through `dot -Tsvg` in a container, and the hand-emitted arm carries the same node/edge inventory; the dot and svg arms draw a GENERATED legend whenever --layer both draws both edge families; viewer emits ONE self-contained zero-dependency HTML document over the scan artefact and rides the scan source only — click a node to highlight its connections, a legend states the drawn classes, and the layer radios switch the direct and indirect views riding the --layer semantics\n  --layer <layer>       direct, indirect or both (default: both): direct draws the structural arrows, indirect the dashed event-labelled async rows, both draws every row exactly as the shipped render — byte-identical\n  --level <level>       scan source only: render granularity as a level fold over the artefact's module-edge rows — unit (the owning unit), dir:<k> (the first k directory segments relative to the tree root, k ≥ 1; dir:0 is not selectable), module (the module tier fold), file (one bucket per file); one aggregated edge row carries its raw-row count; intra-bucket rows are self-accounted in the self rows: tally and the accounting exact: <d> drawn + <s> self = <r> raw rows at level <lv> ledger row the run prints on stderr together with note: descriptive only — verify gates the module tier; a zero-row artefact prints empty model: 0 raw rows at level <lv> instead; the accounting block restates the scan completeness census row whenever the gap census is non-empty; at level unit the block additionally prints the declared-vs-projected unit census — a `unit delta model-only: <n> — <pairs>` row and a `unit delta projection-only: <n> — <pairs>` row naming where the model's declared unit edges and the projected unit graph differ (`unit delta model-only: none` when a class is empty) — advisory on stderr, never the render body, never the exit code; an over-ceiling level render emits nothing and refuses at exit 64, and the refusal names the fits — every fold up to the first OVER with cross-edges above 0 and its self share percent, or the named empty-fits row — then narrow with: archspec scan <bucket-path> --output <artefact>, then archspec diagram --source scan <artefact> [--level <fit>]; with --source spec or --tiers the flag is a usage error (exit 64)\n  --output <path>       write the diagram to that file (default: stdout unless archspec.toml [output] sets a destination); `--output -` writes to stdout and creates no file — never a path positional\n  --tiers <list>        additive comma-separated tiers to render, from units, modules (scan source) and files (spec source with --module/--modules-dir) (default: units); modules adds the module-tier cluster and files the file-tier cluster on top of the unchanged render — files with --source scan or without a territory flag is a usage error (exit 64)\n  --module <claim>      spec source only: render that one module claim's territory (its members, edges and contract marks, adjudicated like verify); an unknown claim is a usage error (exit 64)\n  --modules-dir <dir>   spec source only: write one stable claim-named diagram per module claim into <dir>; the batch writes atomically; --check flags stale, missing, and orphan files without writing\n  --check               compare the diagram to its destination without writing; exit non-zero if stale or missing; a fresh check prints `ok: <path> up to date`\n\nArtefact form: `archspec diagram --source scan docs/archspec/scan.json` — with `--source scan`, the argument is a scan-JSON artefact file written by `archspec scan`, not a project directory.\n\nThe scan source is exploratory: `archspec diagram --source scan <path>` with no `--output` renders to stdout, states the skip with `note: destination <path> not written`, and never touches the configured destination; pass `--output <path>` to write that render.\n\nWhy layers exist (the ceiling rationale): mermaid hosts pin ceilings of 500 edges and 50,000 characters, and a graph over either ceiling renders nothing — the corpus-scale witness measures 54,869 module-edge rows and 11,520 file nodes, proven unrenderable as one mermaid graph by that arithmetic. The layer split is mandatory for corpus legibility, not stylistic.\n\nViewer law: the emitted document is generated by the binary end to end — one hand-written SVG plus one hand-written script block — so no vendored library bytes enter the payload, the file carries no external asset reference, and every model string is HTML-escaped at build time and reaches the DOM via textContent; the payload ships no standalone JS asset. The emitted document carries an emitted-byte ceiling of 1048576 bytes — the measured constant (floor 1 MB; the pane-fold witness corpora emit 0.02 MB and 0.28 MB, both under the floor), the document's embedded refusal rows charge its own total, and a document over the ceiling ships nothing: the emit refuses at exit 64 naming the largest scope that fits, then the two-step narrowing.\n\nViewer channels: the mirror repository's GitHub Pages channel publishes the emitted file (the payload ships .github/workflows/pages.yml for it). gitea has no Pages — the feature is gone in the 28.x line and a 28.x instance answers 404 on pages paths — so on gitea the file has no pages channel. The gitea-side alternatives are named and NOT built here: hosting the same file on project infrastructure (a compose service or an artifact registry) is repository configuration outside the payload, and `archspec serve` is the named future local channel, not built.\n\nViewer interaction: the emitted viewer document draws one thin dotted grey declared edge per manifest-declared dependency leg from a unit to an external, a checkbox named declared deps toggles that class independent of the layer radios (default on), a click on the canvas background or the Escape key clears the selection, the mouse wheel zooms and a pointer drag pans the canvas through its viewBox only, never the emitted bytes, and every node is keyboard-focusable and selects on Enter or Space.\n";

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
                name: "layer",
                takes_value: true,
            },
            FlagSpec {
                name: "level",
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
        "diagram",
    )?;
    cli::exactly_one_positional(&parsed)?;

    let format = parsed
        .values
        .get("format")
        .map(String::as_str)
        .unwrap_or("mermaid");
    if !FORMATS.contains(&format) {
        return Err(format!(
            "unsupported format: {format} (supported: mermaid, plantuml, dot, svg, viewer)"
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
    // `--tiers` selects rendered tiers: `units`/`modules` on the scan
    // surface (the shipped law) and `files` on the spec surface under a
    // territory flag (design record D8); every rejected value is a usage
    // error naming what it broke and the supported tiers (workplan D7).
    let tiers = match parsed.values.get("tiers").map(String::as_str) {
        Some(selection) => {
            let tiers = render::TierSelection::parse(selection).map_err(cli::usage_error)?;
            if source == "scan" {
                if tiers.files {
                    return Err(cli::usage_error(
                        "--tiers files requires the spec source — the files tier marks through the adjudication seam, which needs the spec's facts a scan artefact does not carry",
                    ));
                }
            } else if tiers.named.contains("units") || tiers.named.contains("modules") {
                return Err(cli::usage_error(
                    "--tiers requires --source scan — the spec source renders declared components at full depth (supported tiers: units, modules)",
                ));
            }
            tiers
        }
        None => render::TierSelection::units_only(),
    };

    // `--layer` selects the edge layer (workplan visibility_phase2 scenario
    // 05): the flag absent is `both`, whose bytes are the shipped render;
    // every rejected value is a usage error naming the supported layers.
    let layer = match parsed.values.get("layer").map(String::as_str) {
        Some(selection) => render::LayerSelection::parse(selection).map_err(cli::usage_error)?,
        None => render::LayerSelection::both(),
    };

    // `--level` selects the level fold (workplan level_views scenario 01):
    // it rides the scan source (the fold projects the artefact's
    // module-edge rows — the spec source renders declared components, and
    // `--level` with it is a usage error) and it REPLACES the tier
    // selection (one fold draws one projected graph; pairing the two
    // selects two graphs at once). An unknown value is the usage-error
    // class naming the supported levels, mirroring `--tiers`.
    let level = match parsed.values.get("level").map(String::as_str) {
        Some(selection) => {
            let selection = render::LevelSelection::parse(selection).map_err(cli::usage_error)?;
            if source != "scan" {
                return Err(cli::usage_error(
                    "--level requires --source scan — the level fold projects the module-edge rows a scan artefact carries, and the spec source renders declared components",
                ));
            }
            if parsed.values.contains_key("tiers") {
                return Err(cli::usage_error(
                    "--level replaces the tier selection — one level fold draws one projected graph: drop --tiers to fold by level",
                ));
            }
            Some(selection)
        }
        None => None,
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
    // The size law (M13): the files tier renders one claim's territory,
    // never the whole tree — without a territory flag it is a usage error.
    if tiers.files && module.is_none() && modules_dir.is_none() {
        return Err(cli::usage_error(
            "--tiers files requires --module or --modules-dir — the files tier renders a claim's territory, never the whole tree",
        ));
    }
    // The territory views draw the module and file tiers — structural facts
    // only, the async rows live at the unit tier and in the declared table
    // (workplan visibility_phase2 scenario 05): `--layer indirect` there
    // would draw an arrowless graph, so it is a named usage error; and the
    // dot/svg arms render one whole-model graph, never a territory or a
    // batch, so the pair is named too.
    if (module.is_some() || modules_dir.is_some()) && layer == render::LayerSelection::Indirect {
        return Err(cli::usage_error(
            "--layer indirect has no arrows to draw in a territory render — the async rows live at the unit tier, never at module or file granularity",
        ));
    }
    if (module.is_some() || modules_dir.is_some())
        && matches!(format, "dot" | "svg" | "viewer")
    {
        return Err(cli::usage_error(
            "--format dot, --format svg and --format viewer render one whole-model graph — the module-territory views and the batch write mermaid and plantuml artefacts",
        ));
    }
    if let Some(claim) = module {
        let (loaded, model) = load_and_scan(project_root)?;
        if !spec::claim_names(&loaded.modules).iter().any(|name| name == claim) {
            return Err(cli::usage_error(format!(
                "unknown module: no claim of the spec carries that name: {claim}"
            )));
        }
        note_files_facts(&model, tiers.files);
        let diagram = match format {
            "mermaid" => render::render_module_mermaid(&model, &loaded, claim, &tiers),
            "plantuml" => render::render_module_plantuml(&model, &loaded, claim, &tiers),
            _ => unreachable!(),
        };
        // The ceiling law at the territory seam: the render is one claim's
        // territory, so the refusal names the claim, and the territory arms
        // accept only the mermaid and plantuml artefacts (dot/svg/viewer
        // were refused above), so the one flag that clears it is the format
        // switch to the host without these ceilings.
        if format == "mermaid"
            && let Some(msg) = render::mermaid_ceiling_error(&diagram, "--format plantuml")
        {
            return Err(cli::usage_error(format!("claim {claim:?}: {msg}")));
        }
        let regen = format!("diagram --module {claim}");
        let route = config::route(
            parsed.values.get("output").map(String::as_str),
            config.output.diagram.as_deref(),
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
        note_files_facts(&model, tiers.files);
        return write_modules_dir(
            Path::new(dir),
            &loaded,
            &model,
            format,
            parsed.switches.contains("check"),
            &tiers,
        );
    }

    let mut ledger_lines: Vec<String> = Vec::new();
    let diagram = match source {
        "scan" => {
            let artefact = scan_artefact
                .ok_or_else(|| "--source scan requires an artefact path".to_string())?;
            // The fit verdicts follow the ceilings parsed from these help
            // bytes (workplan level_views assumption); the ceiling-facts cell
            // pins the parse to the constants, so the fallback is unreachable
            // while the rationale sentence stands in the help.
            let ceilings = render::mermaid_ceilings_from_help(HELP)
                .unwrap_or((render::MERMAID_EDGE_CEILING, render::MERMAID_CHAR_CEILING));
            let (body, ledger) =
                render_scan(&artefact, project_root, format, &tiers, layer, level, ceilings)?;
            ledger_lines = ledger;
            body
        }
        "spec" => {
            let loaded = spec::load(project_root)?;
            if loaded.modules.is_empty() {
                return Err("no model content to render".to_string());
            }
            if format == "viewer" {
                return Err(cli::usage_error(
                    "--format viewer rides the scan source — the viewer document is emitted over a scan artefact: diagram --source scan <scan.json> --format viewer",
                ));
            }
            match format {
                "mermaid" => render::render_spec_mermaid(&loaded, layer),
                "plantuml" => render::render_spec_plantuml(&loaded, layer),
                "dot" => render::render_spec_dot(&loaded, layer),
                "svg" => render::render_spec_svg(&loaded, layer),
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    };

    // The ceiling law at the whole-model mermaid seam (claim
    // `layer-ceiling-rationale` armed by the sweep HIGH-1 cure): the body
    // has just been rendered in full, so the measured counts are the exact
    // bytes that would be emitted; over either ceiling the run refuses in
    // the layer arms' usage class and emits nothing — the failure never
    // waits for a third-party mermaid host to stay silent.
    if format == "mermaid" {
        let clear_with = if source == "scan" {
            "--format dot, --format svg or --format viewer, or fewer tiers with --tiers units"
        } else {
            "--format dot or --format svg"
        };
        if let Some(msg) = render::mermaid_ceiling_error(&diagram, clear_with) {
            return Err(cli::usage_error(msg));
        }
    }

    let mut route = config::route(
        parsed.values.get("output").map(String::as_str),
        config.output.diagram.as_deref(),
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
    config::emit(&diagram, route, check, "diagram", "diagram", None)?;
    // The accounting block (scenario 01): advisory stderr rows, printed
    // only once a render was emitted — descriptive rows never enter the
    // render body and never change rc.
    for line in &ledger_lines {
        eprintln!("{line}");
    }
    Ok(())
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
        Language::Ts => "TypeScript",
        Language::Python => "Python",
        Language::Kotlin => "Kotlin",
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
    tiers: &render::TierSelection,
) -> Result<(), String> {
    let extension = if format == "plantuml" { "puml" } else { "mmd" };
    let claims = spec::claim_names(&loaded.modules);
    if check {
        let mut findings: Vec<String> = Vec::new();
        for claim in &claims {
            let path = dir.join(format!("{}.{extension}", render::claim_filename(claim)));
            let expected = render_one(model, loaded, format, claim, tiers)?;
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
    // The batch renders EVERY claim before writing ANY file, so a refusal —
    // the ceiling arm included — leaves every file at previous-or-complete
    // bytes with no partial batch (the D12 atomicity law carried to the
    // seam arm).
    let mut rendered: Vec<(std::path::PathBuf, String)> = Vec::with_capacity(claims.len());
    for claim in &claims {
        let path = dir.join(format!("{}.{extension}", render::claim_filename(claim)));
        rendered.push((path, render_one(model, loaded, format, claim, tiers)?));
    }
    let mut failures: Vec<String> = Vec::new();
    for (path, content) in &rendered {
        match atomic_write(dir, path, content.as_bytes()) {
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

fn render_one(
    model: &model::Model,
    loaded: &spec::Spec,
    format: &str,
    claim: &str,
    tiers: &render::TierSelection,
) -> Result<String, String> {
    match format {
        "plantuml" => Ok(render::render_module_plantuml(model, loaded, claim, tiers)),
        _ => {
            // The ceiling law at the batch seam: the body is refused before
            // any file is written (the batch renders all, writes after),
            // and the one flag that clears a territory render is the format
            // switch — dot/svg/viewer are refused in territory mode.
            let body = render::render_module_mermaid(model, loaded, claim, tiers);
            if let Some(msg) = render::mermaid_ceiling_error(&body, "--format plantuml") {
                return Err(cli::usage_error(format!("claim {claim:?}: {msg}")));
            }
            Ok(body)
        }
    }
}

/// The vacuity law on the files tier (D7): a language whose driver emits
/// no file-grained facts renders no file vertices — and the run states the
/// vacuity instead of silence or a fabricated graph, pointing at the
/// capability table that owns the fact.
fn note_files_facts(model: &model::Model, files_tier: bool) {
    if !files_tier {
        return;
    }
    let granular = language::from_name(&model.language).is_some_and(|language| {
        capability::emission(language, capability::FACT_FILE_GRAINED_DEPENDENCY_FACTS)
            == Some(capability::GRANULAR)
    });
    if !granular {
        eprintln!(
            "note: driver emits no {} fact (capability {}): the files tier renders no file vertices — see `archspec help capabilities`",
            capability::FACT_FILE_GRAINED_DEPENDENCY_FACTS,
            capability::NOT_EMITTED
        );
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
    layer: render::LayerSelection,
    level: Option<render::LevelSelection>,
    ceilings: (usize, usize),
) -> Result<(String, Vec<String>), String> {
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

    // ONE fold site feeding every format arm (the one-assembly-site law):
    // the projection is computed once here; each arm draws from it and the
    // accounting block reads the same struct.
    let gap_census = scan::unresolvable_census_line(&loaded.unresolvable_imports);
    let projection = level.map(|level| render::level_projection(&loaded, level, gap_census.clone()));
    let body = match format {
        "mermaid" => render::render_scan_mermaid(
            &loaded,
            governing.as_ref(),
            tiers,
            layer,
            projection.as_ref(),
        ),
        "plantuml" => render::render_scan_plantuml(
            &loaded,
            governing.as_ref(),
            tiers,
            layer,
            projection.as_ref(),
        ),
        "dot" => render::render_scan_dot(&loaded, governing.as_ref(), tiers, layer, projection.as_ref()),
        "svg" => render::render_scan_svg(&loaded, governing.as_ref(), tiers, layer, projection.as_ref()),
        "viewer" => render::render_scan_viewer(
            &loaded,
            governing.as_ref(),
            tiers,
            layer,
            projection.as_ref(),
            ceilings,
        ),
        _ => unreachable!(),
    };
    // The level ceiling seam (workplan level_views scenario 02): a level
    // render over a parsed ceiling emits nothing and refuses in the usage
    // class naming the fits and the two-step narrowing — never a silent
    // auto-pick of a coarser level. The fit walk measures the emitter's own
    // bytes through the ONE fold site, stopping at the first OVER fold.
    if level.is_some() && format == "mermaid"
        && let Some(msg) =
            render::level_mermaid_ceiling_error(&body, &loaded, layer, ceilings.0, ceilings.1)
    {
        return Err(cli::usage_error(msg));
    }
    // The viewer emitted-byte ceiling seam (workplan level_views scenario
    // 05): an over-ceiling viewer document ships nothing and refuses in the
    // usage class, naming the largest scope that fits (emit-and-measure on
    // the emitter's own bytes) and the two-step narrowing. The emitted
    // bytes ARE the budget: the refusal rows a document embeds are charged
    // to its total once, so no refusal re-triggers this gate.
    if format == "viewer"
        && let Some(msg) =
            render::viewer_byte_ceiling_error(&body, &loaded, layer, level, ceilings, gap_census)
    {
        return Err(cli::usage_error(msg));
    }
    let ledger = projection
        .as_ref()
        .map(render::accounting_lines)
        .unwrap_or_default();
    Ok((body, ledger))
}
