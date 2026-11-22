use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::depgraph;
use crate::archspec::language;
use crate::archspec::scan;
use crate::archspec::spec;
use std::path::Path;

const GRAPH_FORMATS: &[&str] = &["mermaid", "plantuml", "dot", "svg"];

pub const HELP: &str = "usage: archspec depgraph <modules|api-usage|submodules|ledger> [path] [--format <fmt>] [--layer <direct|indirect|both>] [--parent <module>] [--aggregate <units|modules>] [--output <path>] [--check]\ncurrent-state dependency views of a project, straight from the extracted model\n\n  modules        top-level module dependency graph; one level deeper when that projection folds to a single node (default format: mermaid); an automatic render above 200 nodes adds one stderr note naming the degraded projection — stdout and exit are unchanged\n  api-usage      Markdown table of used APIs grouped by target module (markdown only)\n  submodules     graph of one module's immediate children (requires --parent)\n  ledger         the inversion-debt ledger: two named sections over the model's own facts — `used but undeclared` pairs (the code imports across a unit pair the importer's manifest omits, with evidence locs) and `declared but unused` pairs (the manifest declares while the model holds no code evidence); a model without a declaration tier states it cannot answer; a clean model states the ledger is clean by name (markdown only)\n  path           directory to scan (default: current directory)\n  --format <fmt> mermaid, plantuml, dot or svg for graphs; markdown for api-usage and ledger; dot emits graphviz source and svg a hand-emitted layout — the SVG materializes from the dot body by piping it through `dot -Tsvg` in a container\n  --layer <layer> direct, indirect or both (default: both) for graphs: direct draws the structural arrows, indirect the dashed event-labelled async rows, both draws every row exactly as the shipped render — byte-identical; the dot and svg arms draw a GENERATED legend whenever both draws both families (the ceiling rationale rides `archspec diagram --help`)\n  --parent <m>   parent top-level module to expand (submodules view only)\n  --aggregate <level> modules view only: force the projection level, bypassing the automatic fold/descend rule — units renders the model's units and their edges; modules renders top-level modules with no descent\n  --output <p>   write the body to that file (default: stdout); `--output -` writes to stdout and creates no file — never a path positional\n  --check        compare the body to its destination without writing; exit non-zero if stale or missing; a fresh check prints `ok: <path> up to date`\n\nlanguages      rust and csharp render directly; when the model has no module tier the command refuses with: depgraph needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of\n\ndepgraph does not read archspec.toml [output] — an artefact path comes only from an explicit --output.\n\nNode labels fold units — unit ownership lives in the `root_module_declarations` map that `archspec scan` emits in its scan JSON.\n\nThe `mod` label is a unit's root-module bucket: the namespace-less root where the composition/wiring code lives.\n\nTest-gated/soft edges (test namespaces and their consumers) can appear in depgraph projections while being excluded from `depend_on` comparison exactly when the language's test gate marks them — rust `cfg(test)`; ts a test-file name in a package whose manifest declares vitest or jest — otherwise they adjudicate like any other edge.\n\nTo ask which edges are wrong, render the same model with `archspec diagram --source scan <scan.json> --tiers modules` — under a governing spec that view marks spec-violating edges.\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "format",
                takes_value: true,
            },
            FlagSpec {
                name: "layer",
                takes_value: true,
            },
            FlagSpec {
                name: "parent",
                takes_value: true,
            },
            FlagSpec {
                name: "aggregate",
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
            "usage: archspec depgraph <modules|api-usage|submodules|ledger> [path] [--format <fmt>] [--parent <module>] [--output <path>]".to_string()
        })?;
    if !matches!(view, "modules" | "api-usage" | "submodules" | "ledger") {
        return Err(format!(
            "unknown depgraph view: {view} (supported: modules, api-usage, submodules, ledger)"
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

    let default_format = if matches!(view, "api-usage" | "ledger") {
        "markdown"
    } else {
        "mermaid"
    };
    let format = parsed
        .values
        .get("format")
        .map(String::as_str)
        .unwrap_or(default_format);
    if matches!(view, "api-usage" | "ledger") {
        if format != "markdown" {
            return Err(format!(
                "unsupported format for {view}: {format} (supported: markdown)"
            ));
        }
    } else if !GRAPH_FORMATS.contains(&format) {
        return Err(format!(
            "unsupported format: {format} (supported: mermaid, plantuml, dot, svg)"
        ));
    }

    // `--layer` selects the edge layer of the graph views (workplan
    // visibility_phase2 scenario 05): the flag absent is `both`, whose
    // mermaid/plantuml bytes are the shipped render. api-usage and ledger
    // carry no arrows, so the flag there is a named usage error.
    let layer = match parsed.values.get("layer").map(String::as_str) {
        Some(selection) => {
            if matches!(view, "api-usage" | "ledger") {
                return Err(cli::usage_error(
                    "--layer selects an edge layer for the graph views — api-usage and ledger carry no arrows",
                ));
            }
            depgraph::LayerSelection::parse(selection).map_err(cli::usage_error)?
        }
        None => depgraph::LayerSelection::both(),
    };

    let parent = parsed.values.get("parent").map(String::as_str);
    if view == "submodules" && parent.is_none() {
        return Err("submodules view requires --parent <module>".to_string());
    }
    if view != "submodules" && parent.is_some() {
        return Err("--parent is only valid for the submodules view".to_string());
    }

    // `--aggregate` forces the projection level of the modules view; the
    // value is usage-checked before any scan so a bad level never costs a
    // tree walk, and the flag names the view that owns a projection.
    let aggregate = parsed.values.get("aggregate").map(String::as_str);
    if let Some(level) = aggregate {
        if !matches!(level, "units" | "modules") {
            return Err(cli::usage_error(format!(
                "invalid value {level:?} for --aggregate — supported levels: units, modules"
            )));
        }
        if view != "modules" {
            return Err(cli::usage_error(
                "--aggregate is only valid for the modules view".to_string(),
            ));
        }
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

    // The async rows ride BOTH sources (workplan archspec_async_depends, N2,
    // follow-up 6; scenario 08): the model's derived `async_edges` and,
    // wherever a governing spec is present, its declared `[[async_depends]]`
    // table. A set deduplicates and orders by `(from, to, event)` (N6); the
    // rows carry UNIT endpoints and the renderer draws one labelled arrow per
    // row (no projected-node fan-out). A non-declaring tree carries no rows,
    // so every graph renders its exact shipped bytes (D8). Validation stays
    // in `verify` — this view draws, never adjudicates (N2: a diagram is not
    // a second adjudicator).
    let mut async_rows: std::collections::BTreeSet<(String, String, String)> =
        depgraph::derived_render_rows(&model).into_iter().collect();
    if let Some(loaded_spec) = loaded.as_ref() {
        for entry in &loaded_spec.async_depends {
            if let (Some(from), Some(to), Some(event)) =
                (entry.from.as_deref(), entry.to.as_deref(), entry.event.as_deref())
                && !from.is_empty()
                && !to.is_empty()
                && !event.is_empty()
            {
                async_rows.insert((from.to_string(), to.to_string(), event.to_string()));
            }
        }
    }

    let body = match view {
        "api-usage" => depgraph::api_usage_markdown(&model),
        "ledger" => depgraph::ledger_markdown(&model),
        "modules" => {
            // Each graph pairs with the SAME node function its builder
            // projected through (issue #51), so async endpoints fold exactly
            // like the nodes do.
            type NodeFold<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;
            type CensusNode = Option<fn(&str) -> String>;
            // The automatic tier's node function, named BEFORE the match so
            // the fold closure can borrow it without borrowing the model's
            // ownership (issue #61: the closure now threads the model
            // through `fold_named`).
            let auto_node = depgraph::projection_node(&model);
            let ((mut graph, fold), census): (
                (depgraph::ModuleGraph, NodeFold<'_>),
                CensusNode,
            ) = match aggregate {
                // An explicit level is never capped: the user chose this
                // projection, so the automatic rule has nothing to report.
                // The fold paired with each level is the SAME node function
                // its graph builder used (issue #51), identity-preserving
                // like the builder's fold (issue #61); the units tier places
                // every unit endpoint by name, so its render keeps its bytes.
                Some("units") => (
                    (depgraph::units_graph(&model), Box::new(|_| None)),
                    None,
                ),
                Some("modules") => (
                    (
                        depgraph::top_modules_graph(&model),
                        Box::new(|path| Some(depgraph::fold_named(&model, depgraph::top_node, path))),
                    ),
                    Some(depgraph::top_node),
                ),
                _ => {
                    let graph = depgraph::modules_graph(&model);
                    // Degraded-projection guard: the automatic path only,
                    // stderr only — the graph renders unchanged and the
                    // exit code is unchanged (a note, not a refusal). The
                    // helper picks which note the render owes (cap crossing
                    // or single-node collapse, issue #20), or none.
                    if let Some(note) = depgraph::automatic_projection_note(&model, &graph) {
                        eprintln!("{note}");
                    }
                    (
                        (
                            graph,
                            Box::new(|path| Some(depgraph::fold_named(&model, auto_node, path))),
                        ),
                        Some(auto_node),
                    )
                }
            };
            // The dotted-name collision census (issue #61): where the fold
            // truncated several module names onto one segment, one stderr
            // row per colliding form names every full name the identity-
            // preserving render now shows. Conditional: no collision, no
            // bytes (D8).
            if let Some(census) = census {
                for note in depgraph::module_fold_collision_note(&model, census) {
                    eprintln!("{note}");
                }
            }
            // Async endpoints ride the view's fold, never their raw names
            // (issue #51): each row places on exactly one rendered node or
            // folds away with one counted stderr note — never silence.
            let projection =
                depgraph::project_async_rows(&model, &graph, fold.as_ref(), async_rows.iter().cloned());
            if let Some(note) = depgraph::async_fold_note(&projection) {
                eprintln!("{note}");
            }
            depgraph::attach_async_rows(&mut graph, projection.rows);
            render_graph(&graph, format, layer)?
        }
        "submodules" => {
            let parent = parent.expect("checked above");
            let mut graph = depgraph::submodules_graph(&model, parent)?;
            let parent_scope = parent.to_string();
            let fold = move |path: &str| depgraph::child_node(path, &parent_scope);
            let projection =
                depgraph::project_async_rows(&model, &graph, fold, async_rows.iter().cloned());
            if let Some(note) = depgraph::async_fold_note(&projection) {
                eprintln!("{note}");
            }
            depgraph::attach_async_rows(&mut graph, projection.rows);
            render_graph(&graph, format, layer)?
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

fn render_graph(
    graph: &depgraph::ModuleGraph,
    format: &str,
    layer: depgraph::LayerSelection,
) -> Result<String, String> {
    let mut body = match format {
        "mermaid" => depgraph::render_mermaid(graph, layer),
        "plantuml" => depgraph::render_plantuml(graph, layer),
        "dot" => depgraph::render_dot(graph, layer),
        "svg" => depgraph::render_svg(graph, layer),
        _ => unreachable!("format validated above"),
    };
    // The stated fold (roles reality US 05): a fold that silences markers on
    // a node because DIFFERENT roles landed there says so — conditional, so
    // views whose fold places every claim keep their exact bytes. Each
    // format carries the note in its own comment grammar (scenario 05 adds
    // the dot and svg grammars).
    if graph.unplaced_claims > 0 {
        let note = depgraph::fold_note(graph.unplaced_claims);
        match format {
            "mermaid" => body.push_str(&format!("  %% {note}\n")),
            "dot" => body.push_str(&format!("// {note}\n")),
            "svg" => body.push_str(&format!("<!-- {note} -->\n")),
            _ => body.push_str(&format!("'{note}\n")),
        }
    }
    // The ceiling law at the mermaid emit seam (claim
    // `layer-ceiling-rationale` armed by the sweep HIGH-1 cure, the same
    // arm the diagram command carries): the measured body is the exact
    // render — fold note included — so the counts are honest, and an
    // over-ceiling render refuses in the layer arms' usage class with
    // nothing emitted. The clear-arms name only flags this view accepts:
    // the dot and svg formats carry no mermaid-host ceiling, and the two
    // forced aggregation levels are the degraded note's own pair —
    // `--aggregate units` cures the file-granularity descent,
    // `--aggregate modules` pins the top level.
    if format == "mermaid"
        && let Some(msg) = crate::archspec::diagram::mermaid_ceiling_error(
            &body,
            "--format dot, --format svg, --aggregate units or --aggregate modules",
        )
    {
        return Err(cli::usage_error(msg));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archspec::depgraph::LayerSelection;
    use std::collections::{BTreeMap, BTreeSet};

    /// A module graph drawing `edges` structural arrows between distinct
    /// nodes — the seam measures what the mermaid body would carry.
    fn graph_with(edges: usize) -> depgraph::ModuleGraph {
        let mut graph = depgraph::ModuleGraph {
            nodes: BTreeSet::new(),
            edges: BTreeSet::new(),
            async_edges: BTreeSet::new(),
            markers: BTreeMap::new(),
            unplaced_claims: 0,
        };
        for i in 0..edges {
            graph.nodes.insert(format!("m{i}"));
            graph.nodes.insert(format!("n{i}"));
            graph.edges.insert((format!("m{i}"), format!("n{i}")));
        }
        graph
    }

    #[test]
    fn the_mermaid_arm_refuses_over_the_ceiling_and_keeps_dot_svg_and_plantuml() {
        // The sweep HIGH-1 shape on this seam: an over-ceiling projection
        // must refuse with the registered row, never print and exit 0 —
        // while the ceiling-free formats keep their shipped bytes.
        let refused = render_graph(&graph_with(501), "mermaid", LayerSelection::Both)
            .expect_err("a 501-edge projection refuses at the seam");
        let (code, bare) = cli::classify(&refused).expect("the refusal carries a class");
        assert_eq!(code, cli::EX_USAGE, "over-ceiling rides the layer arms' class");
        assert!(bare.starts_with("this render exceeds the mermaid ceilings of 500 edges and 50,000 characters — measured 501 edges and "), "{bare}");
        assert!(bare.contains("--aggregate units or --aggregate modules"), "{bare}");
        assert!(render_graph(&graph_with(500), "mermaid", LayerSelection::Both).is_ok());
        assert!(render_graph(&graph_with(501), "dot", LayerSelection::Both).is_ok());
        assert!(render_graph(&graph_with(501), "svg", LayerSelection::Both).is_ok());
        assert!(render_graph(&graph_with(501), "plantuml", LayerSelection::Both).is_ok());
    }
}
