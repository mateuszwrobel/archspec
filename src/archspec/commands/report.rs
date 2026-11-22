use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::depgraph;
use crate::archspec::inspect;
use crate::archspec::language;
use crate::archspec::report;
use crate::archspec::scan;
use crate::archspec::spec;
use crate::archspec::verify::compare;
use super::verify::RULE_VIOLATION_REPORTED;
use std::path::Path;

pub const HELP: &str = "usage: archspec report [path] [--format <text|markdown|json>] [--diagram <files|modules|none>] [--output <path>] [--check] [--pretty]\nproduce a text, markdown, or json architecture report (diff + metrics)\n\n  path             project directory to report on (default: current directory)\n  --format <fmt>   text, markdown, or json (default: text)\n  --diagram <mode> diagram embedded in a markdown report: files (default, the file-level import map), modules (the graph archspec depgraph modules draws), none (omit the section); archspec.toml [report] diagram sets it for runs without the flag\n  --output <path>  write the report to that file (default: stdout unless archspec.toml [output] sets a destination); `--output -` writes to stdout and creates no file — never a path positional\n  --check          compare the report to its destination without writing; exit non-zero if stale, missing, or violations are present; a fresh check prints `ok: <path> up to date`. A `--check` compares the canonical rendering unless `--format` names another; to verify a `--format`-generated artefact, pass the same `--format` to `--check`.\n  --pretty         accepted for symmetry with `scan`; report json is already indented and no body changes\n\nA configured report destination is provisioned for the default text rendering: a run naming another `--format` renders to stdout and never writes the destination.\n\nThe diagram embedded in a markdown report is file-granular by default; the `--diagram` flag or `[report] diagram` in `archspec.toml` selects the module-granular graph or omits the section, and the standalone module-granular graph is `archspec depgraph modules`.\n\nexit 0 when the code satisfies its spec; exit 1 on rule violations; exit 70 when no spec is found (operational)\n";

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

/// `archspec report [path] [--format <text|markdown|json>] [--diagram
/// <files|modules|none>] [--output <path>]` (commands/report/cli.md).
/// Spec-driven: the spec's `[project] language`
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
                // The diagram-scale knob (GitHub issue #52): `none|modules|
                // files`, overriding the committable `[report] diagram` key
                // for one-offs. V1 is the knob alone — no automatic
                // degrade: a cap rewrite would break `--check` on committed
                // artefacts (the deferred rung, named follow-up).
                name: "diagram",
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
        "report",
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
    let language = language::from_name(&loaded.project.language).ok_or_else(|| {
        format!(
            "invalid spec: {}: unknown language \"{}\"",
            spec_file.display(),
            loaded.project.language
        )
    })?;

    // The agreement check is a precondition (D14, GitHub issue #36): the
    // declared language is the truth of the tree, and any un-excluded foreign
    // source file fails the run loud BEFORE any artifact is computed. Report
    // is the verb that only produces artifacts, so the guard must weld here
    // too: the same shared `agreement_finding` call verify and scan make —
    // one call site, same message bytes, same untagged exit class — never a
    // report-local copy of the logic.
    if let Some(finding) =
        language::agreement_finding(root_path, language, &spec::exclusion_paths(&loaded))
    {
        return Err(finding);
    }

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
    // The environment declaration (GitHub issue #24): the census counts exactly
    // the report-only lines this scan produced — a count over facts already
    // emitted, never a second parse of the tree. Every renderer states it;
    // none computes a second adjudication.
    let environment = report::environment(&model);

    let mapping = compare::map_units(&model.units, &loaded.modules);
    let diff = compare::compare(
        &mapping,
        &model,
        &loaded.modules,
        &loaded.stereotype,
        &loaded.constraint,
        &loaded.exempt,
        &loaded.async_depends,
        &loaded.canonical_producer,
        &loaded.shared_constant,
    );
    let metrics = report::compute_metrics(&model, &mapping, &loaded.modules, &diff);
    // The async PAIR census (workplan archspec_async_depends, D9/D10):
    // declared rows with all three keys resolved; an absent or empty table
    // computes the empty census and every renderer skips the section (D8).
    let async_census = report::AsyncCensus::from_declared(&loaded.async_depends);
    // The declaration census (workplan events_value_join scenario 02): the
    // declared entries of both declaration tables; neither declaring
    // computes the empty census and every renderer skips the section (D8).
    let declaration_census = report::DeclarationCensus::from_declared(
        &loaded.canonical_producer,
        &loaded.shared_constant,
    );

    let config = config::load(root_path)?;
    // The diagram scale (GitHub issue #52): the CLI flag overrides the
    // committable `[report] diagram` key; the key is what `--check` consumes,
    // so a write-with-flag / check-without-flag pair resolves the SAME diagram
    // from the SAME file instead of stale-red-ing CI. Default `files`.
    let diagram_mode = parsed
        .values
        .get("diagram")
        .map(String::as_str)
        .or(config.report.diagram.as_deref())
        .unwrap_or("files");
    if diagram_mode != "files" && diagram_mode != "modules" && diagram_mode != "none" {
        return Err(format!(
            "unsupported diagram mode: {diagram_mode} (supported: files, modules, none)"
        ));
    }
    let diagram = if format == "markdown" {
        match diagram_mode {
            "none" => None,
            "files" => inspect::render_import_map(root_path, language)?,
            "modules" => Some(render_module_diagram(&model, &loaded.async_depends)?),
            _ => unreachable!("diagram mode validated above"),
        }
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
            &environment,
            &async_census,
            &declaration_census,
        ),
        "json" => report::render_json(
            &loaded.project.language,
            &metrics,
            &diff,
            &model.roles,
            &environment,
            &async_census,
            &declaration_census,
        ),
        _ => report::render_text(
            &loaded.project.language,
            &metrics,
            &diff,
            &model.roles,
            &environment,
            &async_census,
            &declaration_census,
        ),
    };

    let output_flag = parsed.values.get("output");
    let check = parsed.switches.contains("check");
    let mut route = config::route(
        output_flag.map(String::as_str),
        config.output.report.as_deref(),
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
        && config.output.report.is_some()
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

/// The `--diagram modules` refusal on a model without a module tier — the
/// report member of the module-tier refusal family (`depgraph`'s sentence,
/// `inspect tree`'s sentence): the same clause, its own remedy (GitHub
/// issue #52).
pub const MODULE_TIER_REQUIREMENT: &str = "report needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of, 'report --diagram files' renders the file-level import map";

/// The graph the `modules` diagram mode embeds — exactly what
/// `archspec depgraph modules` renders automatically (GitHub issue #52): the
/// same projection, the same automatic-projection note, the same async-row
/// fold and the same fold note, so the embedded diagram cannot drift from the
/// standalone view's bytes.
fn render_module_diagram(
    model: &crate::archspec::model::Model,
    declared: &[spec::AsyncDepend],
) -> Result<String, String> {
    if !model.has_module_tier() {
        return Err(MODULE_TIER_REQUIREMENT.to_string());
    }
    let mut graph = depgraph::modules_graph(model);
    // The automatic path's stderr notes ride with the projection — the same
    // never-silent renders `depgraph modules` owes, body and exit unchanged.
    if let Some(note) = depgraph::automatic_projection_note(model, &graph) {
        eprintln!("{note}");
    }
    // The dotted-name collision census (issue #61): the embedded diagram
    // shares the standalone view's fold, so it shares its census rows too.
    let node = depgraph::projection_node(model);
    for note in depgraph::module_fold_collision_note(model, node) {
        eprintln!("{note}");
    }
    // The async rows ride BOTH sources exactly as the `depgraph modules`
    // view builds them (workplan archspec_async_depends N2: derived
    // `async_edges` plus the declared table): one set, deduped and ordered
    // by `(from, to, event)`, so the embedded diagram and the standalone
    // view cannot drift.
    let mut async_rows: std::collections::BTreeSet<(String, String, String)> =
        depgraph::derived_render_rows(model).into_iter().collect();
    for entry in declared {
        if let (Some(from), Some(to), Some(event)) =
            (entry.from.as_deref(), entry.to.as_deref(), entry.event.as_deref())
            && !from.is_empty()
            && !to.is_empty()
            && !event.is_empty()
        {
            async_rows.insert((from.to_string(), to.to_string(), event.to_string()));
        }
    }
    let projection = depgraph::project_async_rows(
        model,
        &graph,
        move |path| Some(depgraph::fold_named(model, node, path)),
        async_rows.iter().cloned(),
    );
    if let Some(note) = depgraph::async_fold_note(&projection) {
        eprintln!("{note}");
    }
    depgraph::attach_async_rows(&mut graph, projection.rows);
    let mut body = depgraph::render_mermaid(&graph, depgraph::LayerSelection::Both);
    // The stated fold rides the fence exactly as the depgraph view's mermaid
    // body carries it (roles reality US 05): one fact, one byte spelling.
    if graph.unplaced_claims > 0 {
        body.push_str(&format!(
            "  %% {}\n",
            depgraph::fold_note(graph.unplaced_claims)
        ));
    }
    Ok(body)
}
