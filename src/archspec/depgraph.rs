//! Dependency-graph projections of the extracted model, consumed by the
//! `depgraph` command. The model stores module edges at full dotted granularity
//! (`unit::parent::child`); these projections collapse those paths onto the
//! vocabulary an auto-generated dependency doc expects: top-level module names
//! (section a), an API-usage table grouped by target top-level module (section
//! b), and immediate-child submodule names scoped to one parent (section c).
//!
//! The projections are pure functions of the model — no spec, no rules — so the
//! output is a current-state picture, matching `scan`/`inspect` rather than
//! `verify`. Rendering reuses the shared `render` primitives so escaping cannot
//! drift from the other diagram commands.

use crate::archspec::model::Model;
use rust_arch_test_kit::render::{
    markdown_table_escape, mermaid_edge, mermaid_node, plantuml_component, plantuml_edge,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// Node name a unit-root module path folds to (a crate's `lib.rs`/`main.rs`).
const ROOT_NODE: &str = "root";
/// Node name a parent module's own file folds to in the submodule view.
const MOD_NODE: &str = "mod";

/// The capability sentence every `depgraph` view emits when the model carries
/// no module tier (`Model::has_module_tier` is false). Each view projects the
/// module tier and nothing language-specific beyond it, so a tier-less model —
/// the go driver emits neither `soft_structure` nor `module_edges` — has no
/// module structure to render and is refused up front rather than with an
/// internal-shape complaint. The remedy names how a Go tree acquires the tier:
/// a tree WITH the tier renders, a tree WITHOUT it gets this error. Kept as the
/// single source of this wording — the command's `--help` line and the depgraph
/// docs quote it verbatim (locked by the
/// `depgraph_help_and_docs_quote_module_tier_sentence` test).
pub const MODULE_TIER_REQUIREMENT: &str = "depgraph needs the module tier, which this model has none of; a Go tree acquires one through go.work members (2+)";

/// A directed module graph at a single granularity: node names and the distinct
/// edges between them. Canonical ordering is inherent (`BTreeSet`).
#[derive(Debug, Clone, Default)]
pub struct ModuleGraph {
    pub nodes: BTreeSet<String>,
    pub edges: BTreeSet<(String, String)>,
}

impl ModuleGraph {
    fn add_node(&mut self, node: &str) {
        if !node.is_empty() {
            self.nodes.insert(node.to_string());
        }
    }
}

/// The top-level module a dotted module path projects to: the segment right
/// after the unit name; a unit-root path (one segment) folds to `root`.
fn top_node(dotted: &str) -> String {
    let segments: Vec<&str> = dotted.split("::").collect();
    if segments.len() <= 1 {
        ROOT_NODE.to_string()
    } else {
        segments[1].to_string()
    }
}

/// The submodule a dotted module path projects to within `parent`, or `None`
/// when the path does not sit under a `parent` top-level module. The parent's
/// own module (two segments) folds to `mod`; a deeper path folds to the child
/// segment directly under the parent.
fn child_node(dotted: &str, parent: &str) -> Option<String> {
    let segments: Vec<&str> = dotted.split("::").collect();
    if !parent_matches(&segments, parent) {
        return None;
    }
    Some(if segments.len() >= 3 {
        segments[2].to_string()
    } else {
        MOD_NODE.to_string()
    })
}

/// True when `segments` sit under the top-level module named by `parent`.
/// A full module name (unit + first segment, any separator) matches first;
/// the bare first module segment matches as a compatibility fallback for
/// callers written against the old bare-only lookup.
fn parent_matches(segments: &[&str], parent: &str) -> bool {
    if segments.len() < 2 {
        return false;
    }
    let full = format!("{}::{}", segments[0], segments[1]);
    full.replace("::", ".") == parent.replace("::", ".") || segments[1] == parent
}

/// The full top-level module name of a dotted path (`unit::first-segment`),
/// or `None` for a unit-root path.
fn top_full_name(dotted: &str) -> Option<String> {
    let mut segments = dotted.split("::");
    let unit = segments.next()?;
    let top = segments.next()?;
    Some(format!("{unit}::{top}"))
}

/// Project the model's module edges and soft structure onto top-level modules.
/// Every top-level module present in the soft tier becomes a node; every module
/// edge contributes its two endpoints and, when they differ, an edge between the
/// two top-level modules (edges that stay inside one top-level module collapse
/// to a node and emit no edge). Unit (dependency) edges are projected through
/// module membership: each endpoint unit contributes the SET of top-level nodes
/// its soft paths fold to, and the unit edge becomes the cross product of the
/// two sets with self-edges suppressed. A unit folding to one node IS that
/// module (a c# project per module), so singleton×singleton projections
/// reproduce the previous membership rule exactly; a multi-node unit (a rust
/// bin sharing its package's module tree, a crate with several top modules)
/// no longer drops its cross-unit edges — unit-tier evidence renders as the
/// coarsest honest module picture. Units with no soft paths contribute
/// nothing. The projection is one code path for every driver, keyed on
/// membership, not on language.
pub fn modules_graph(model: &Model) -> ModuleGraph {
    let mut graph = ModuleGraph::default();
    for paths in model.soft_structure.values() {
        for path in paths {
            graph.add_node(&top_node(path));
        }
    }
    for edge in &model.module_edges {
        let from = top_node(&edge.from);
        let to = top_node(&edge.to);
        graph.add_node(&from);
        graph.add_node(&to);
        if from != to {
            graph.edges.insert((from, to));
        }
    }
    project_unit_edges(model, &mut graph);
    graph
}

/// Map each unit to the SET of top-level nodes its soft paths fold to, then
/// draw every pair of the cross product for each unit edge whose endpoints are
/// both present, skipping self-edges (pairs naming the same node) and unit
/// self-references (a unit edge from a unit to itself says nothing about
/// cross-unit structure; its internal shape already reads from module edges).
fn project_unit_edges(model: &Model, graph: &mut ModuleGraph) {
    if model.edges.is_empty() {
        return;
    }
    let mut unit_tops: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (unit, paths) in &model.soft_structure {
        unit_tops.insert(unit.as_str(), paths.iter().map(|path| top_node(path)).collect());
    }
    for edge in &model.edges {
        if edge.from == edge.to {
            continue;
        }
        let (Some(from_tops), Some(to_tops)) = (
            unit_tops.get(edge.from.as_str()),
            unit_tops.get(edge.to.as_str()),
        ) else {
            continue;
        };
        for from in from_tops {
            for to in to_tops {
                if from == to {
                    continue;
                }
                graph.add_node(from);
                graph.add_node(to);
                graph.edges.insert((from.clone(), to.clone()));
            }
        }
    }
}

/// Project the model onto the immediate-child submodules of `parent`. Only
/// endpoints under `unit::parent` are kept; a dependency leaving the parent's
/// subtree is excluded. `parent` resolves by full module name (any separator)
/// or by bare first module segment. Errors when no top-level module matches,
/// listing the module names the model does know.
pub fn submodules_graph(model: &Model, parent: &str) -> Result<ModuleGraph, String> {
    let mut graph = ModuleGraph::default();
    let mut found = false;
    for paths in model.soft_structure.values() {
        for path in paths {
            if let Some(child) = child_node(path, parent) {
                found = true;
                graph.add_node(&child);
            }
        }
    }
    for edge in &model.module_edges {
        let from = child_node(&edge.from, parent);
        let to = child_node(&edge.to, parent);
        if let (Some(from), Some(to)) = (from, to) {
            found = true;
            graph.add_node(&from);
            graph.add_node(&to);
            if from != to {
                graph.edges.insert((from, to));
            }
        }
    }
    if !found {
        return Err(parent_not_found(model, parent));
    }
    Ok(graph)
}

/// The failed-lookup error: the requested name plus every top-level module
/// name the model carries, so the user can pick one that exists.
fn parent_not_found(model: &Model, parent: &str) -> String {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for paths in model.soft_structure.values() {
        for path in paths {
            if let Some(full) = top_full_name(path) {
                names.insert(full);
            }
        }
    }
    for edge in &model.module_edges {
        for endpoint in [&edge.from, &edge.to] {
            if let Some(full) = top_full_name(endpoint) {
                names.insert(full);
            }
        }
    }
    if names.is_empty() {
        format!("parent module not found in model: {parent}")
    } else {
        format!(
            "parent module not found in model: {parent} (known top-level modules: {})",
            names.into_iter().collect::<Vec<_>>().join(", ")
        )
    }
}

/// Group the symbols used on module edges by target top-level module, then by
/// using top-level module. An edge whose endpoints project to the same top-level
/// module contributes nothing, and a pair with no recorded symbol contributes
/// nothing (it never opens a group).
pub fn api_usage_by_target(model: &Model) -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let mut grouped: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for edge in &model.module_edges {
        let from = top_node(&edge.from);
        let to = top_node(&edge.to);
        if from == to {
            continue;
        }
        for symbol in &edge.symbols {
            grouped
                .entry(to.clone())
                .or_default()
                .entry(from.clone())
                .or_default()
                .insert(symbol.clone());
        }
    }
    grouped
}

/// Render a module graph as a Mermaid `graph TD`: one node line per node, then
/// one solid arrow per edge, in canonical order.
pub fn render_mermaid(graph: &ModuleGraph) -> String {
    let mut out = String::from("graph TD\n");
    for node in &graph.nodes {
        let _ = writeln!(out, "  {}", mermaid_node(node));
    }
    for (from, to) in &graph.edges {
        let _ = writeln!(out, "  {}", mermaid_edge(from, to));
    }
    out
}

/// Render a module graph as a PlantUML component diagram.
pub fn render_plantuml(graph: &ModuleGraph) -> String {
    let mut out = String::from("@startuml\n");
    for node in &graph.nodes {
        let _ = writeln!(out, "{}", plantuml_component(node));
    }
    for (from, to) in &graph.edges {
        let _ = writeln!(out, "{}", plantuml_edge(from, to));
    }
    let _ = writeln!(out, "@enduml");
    out
}

/// The reason sentence an empty api-usage result carries when the emptiness is
/// a fact of **where** the driver records usage rather than an absence of
/// usage. The c# driver records no symbols on module edges at all (it fills
/// edge symbols from `using` targets and emits none). The rust driver records
/// symbols, but its module edges only ever connect modules **within one unit**
/// — a tree whose cross-target links all live at the unit tier (the `edges`)
/// groups to nothing at module granularity while the usage exists, so an empty
/// result there states the fact placement, not a capability gap. A rust tree
/// with no cross-unit links, and any other driver whose empty grouping means
/// genuinely no recorded cross-module usage, keeps the bare statement.
pub fn api_usage_empty_reason(model: &Model) -> Option<&str> {
    match model.language.as_str() {
        "csharp" => Some("Symbol-level facts are not emitted by the csharp driver."),
        "rust" if model.edges.iter().any(|edge| edge.from != edge.to) => Some(
            "Module edges record usage between modules of a single unit; cross-target links are recorded at unit granularity.",
        ),
        _ => None,
    }
}

/// The api-usage body for a model: the grouped table, or the empty statement
/// with the reason appended when the emptiness is a fact of where usage is
/// recorded rather than an absence of usage.
pub fn api_usage_markdown(model: &Model) -> String {
    render_api_usage_markdown(
        &api_usage_by_target(model),
        api_usage_empty_reason(model),
    )
}

/// Render the API-usage grouping as the Markdown table the dependency doc
/// expects: `Target module | Used by module | APIs used`, grouped by target then
/// using module. An empty grouping renders the plain statement the doc uses
/// when no internal API usage is recorded, extended with the driver's `reason`
/// sentence when the emptiness is a driver fact rather than an absence of
/// usage.
pub fn render_api_usage_markdown(
    grouped: &BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    empty_reason: Option<&str>,
) -> String {
    if grouped.is_empty() {
        return match empty_reason {
            Some(reason) => format!("No internal API usage details found. {reason}\n"),
            None => "No internal API usage details found.\n".to_string(),
        };
    }
    let mut out = String::from("| Target module | Used by module | APIs used |\n");
    out.push_str("| --- | --- | --- |\n");
    for (target, froms) in grouped {
        for (from, symbols) in froms {
            let api_list = symbols
                .iter()
                .map(|symbol| format!("`{}`", markdown_table_escape(symbol)))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                out,
                "| `{}` | `{}` | {} |",
                markdown_table_escape(target),
                markdown_table_escape(from),
                api_list
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archspec::model::{Edge, ModuleEdge, Unit};

    fn model_with(unit: &str, soft: Vec<&str>, edges: Vec<(&str, &str, &[&str])>) -> Model {
        Model {
            schema_version: 1,
            language: "rust".to_string(),
            units: vec![Unit {
                name: unit.to_string(),
                kind: "crate".to_string(),
                path: ".".to_string(),
                root: "lib.rs".to_string(),
                crate_ids: BTreeSet::from([unit.to_string()]),
            }],
            edges: Vec::new(),
            usage: Default::default(),
            soft_structure: BTreeMap::from([(
                unit.to_string(),
                soft.into_iter().map(String::from).collect(),
            )]),
            external: Vec::new(),
            module_edges: edges
                .into_iter()
                .map(|(from, to, syms)| ModuleEdge {
                    unit: unit.to_string(),
                    from: from.to_string(),
                    to: to.to_string(),
                    symbols: syms.iter().map(|s| s.to_string()).collect(),
                })
                .collect(),
            manifest: None,
            root_public_exports: Default::default(),
            root_glob_exports: Default::default(),
            root_empty_glob_exports: Default::default(),
            module_external: Default::default(),
            root_module_declarations: Default::default(),
            unit_manifests: Default::default(),
            unresolved_module_files: Default::default(),
            test_gated_modules: Default::default(),
            facade_roots: Default::default(),
        }
    }

    #[test]
    fn modules_graph_collapses_nested_paths_to_top_level() {
        let model = model_with(
            "app",
            vec!["app", "app::config", "app::orchestration::control_loop"],
            vec![("app::orchestration::control_loop", "app::config", &["Config"])],
        );
        let graph = modules_graph(&model);
        assert!(graph.nodes.contains("config"));
        assert!(graph.nodes.contains("orchestration"));
        assert!(graph.nodes.contains(ROOT_NODE));
        assert!(graph.edges.contains(&("orchestration".to_string(), "config".to_string())));
    }

    #[test]
    fn modules_graph_drops_edges_inside_one_top_level_module() {
        let model = model_with(
            "app",
            vec!["app::orchestration", "app::orchestration::a", "app::orchestration::b"],
            vec![("app::orchestration::a", "app::orchestration::b", &[])],
        );
        let graph = modules_graph(&model);
        assert!(!graph.edges.iter().any(|(f, t)| f == "orchestration" && t == "orchestration"));
        assert!(graph.nodes.contains("orchestration"));
    }

    #[test]
    fn submodules_graph_scopes_to_parent_and_folds_parent_to_mod() {
        let model = model_with(
            "app",
            vec![
                "app::orchestration",
                "app::orchestration::control_loop",
                "app::orchestration::progress",
                "app::config",
            ],
            vec![
                ("app::orchestration::control_loop", "app::orchestration::progress", &[]),
                ("app::orchestration::control_loop", "app::config", &[]),
            ],
        );
        let graph = submodules_graph(&model, "orchestration").expect("parent present");
        assert!(graph.nodes.contains("control_loop"));
        assert!(graph.nodes.contains("progress"));
        assert!(graph.nodes.contains(MOD_NODE));
        assert!(graph
            .edges
            .contains(&("control_loop".to_string(), "progress".to_string())));
        assert!(!graph.nodes.contains("config"));
        assert!(!graph.edges.iter().any(|(_, t)| t == "config"));
    }

    #[test]
    fn submodules_graph_errors_when_parent_absent() {
        let model = model_with("app", vec!["app::config"], vec![]);
        assert!(submodules_graph(&model, "orchestration").is_err());
    }

    #[test]
    fn api_usage_groups_symbols_by_target_then_source() {
        let model = model_with(
            "app",
            vec!["app::config", "app::agent_auth", "app::sessions"],
            vec![
                ("app::config", "app::agent_auth", &["AuthConfig"]),
                ("app::sessions", "app::agent_auth", &["CliAuthEntry"]),
            ],
        );
        let grouped = api_usage_by_target(&model);
        assert_eq!(
            grouped["agent_auth"]["config"],
            BTreeSet::from(["AuthConfig".to_string()])
        );
        assert_eq!(
            grouped["agent_auth"]["sessions"],
            BTreeSet::from(["CliAuthEntry".to_string()])
        );
    }

    #[test]
    fn api_usage_ignores_same_top_level_edges() {
        let model = model_with(
            "app",
            vec!["app::a", "app::a::b"],
            vec![("app::a::b", "app::a", &["Thing"])],
        );
        assert!(api_usage_by_target(&model).is_empty());
    }

    #[test]
    fn api_usage_markdown_has_stable_columns() {
        let model = model_with(
            "app",
            vec!["app::config", "app::agent_auth"],
            vec![("app::config", "app::agent_auth", &["AuthConfig", "CliAuthEntry"])],
        );
        let out = render_api_usage_markdown(&api_usage_by_target(&model), None);
        assert!(out.contains("| Target module | Used by module | APIs used |"));
        assert!(out.contains("| --- | --- | --- |"));
        assert!(out.contains("| `agent_auth` | `config` | `AuthConfig`, `CliAuthEntry` |"));
    }

    #[test]
    fn api_usage_markdown_states_none_found_when_empty() {
        let out = render_api_usage_markdown(&BTreeMap::new(), None);
        assert_eq!(out, "No internal API usage details found.\n");
    }

    #[test]
    fn api_usage_markdown_appends_reason_when_given() {
        let out = render_api_usage_markdown(
            &BTreeMap::new(),
            Some("Symbol-level facts are not emitted by the csharp driver."),
        );
        assert_eq!(
            out,
            "No internal API usage details found. Symbol-level facts are not emitted by the csharp driver.\n"
        );
    }

    #[test]
    fn api_usage_empty_reason_none_for_rust_without_unit_links() {
        let model = model_with("app", vec!["app::a", "app::a::b"], vec![]);
        assert_eq!(api_usage_empty_reason(&model), None);
    }

    #[test]
    fn api_usage_empty_reason_states_unit_tier_placement_for_rust() {
        let mut model = model_with("app", vec!["app::a", "app::b"], vec![]);
        model.edges = vec![Edge {
            from: "app-bin".to_string(),
            to: "app".to_string(),
        }];
        let reason = api_usage_empty_reason(&model).expect("unit-tier links must carry a reason");
        assert!(
            reason.contains("unit granularity"),
            "reason must state where the links are recorded: {reason}"
        );
        assert!(
            !reason.contains("not emitted"),
            "rust records symbols; the reason is placement, not capability: {reason}"
        );
    }

    fn unit(name: &str) -> Unit {
        Unit {
            name: name.to_string(),
            kind: "project".to_string(),
            path: ".".to_string(),
            root: String::new(),
            crate_ids: BTreeSet::new(),
        }
    }

    /// A c#-shaped model: one unit per project, every unit's namespaces folding
    /// to a single top-level node, references recorded as unit edges only.
    fn csharp_like_model() -> Model {
        let base = model_with("HomeBudget.Api", Vec::new(), Vec::new());
        Model {
            language: "csharp".to_string(),
            units: vec![
                unit("HomeBudget.Api"),
                unit("HomeBudget.Application"),
                unit("HomeBudget.Domain"),
            ],
            edges: vec![
                Edge {
                    from: "HomeBudget.Api".to_string(),
                    to: "HomeBudget.Application".to_string(),
                },
                Edge {
                    from: "HomeBudget.Application".to_string(),
                    to: "HomeBudget.Domain".to_string(),
                },
            ],
            soft_structure: BTreeMap::from([
                (
                    "HomeBudget.Api".to_string(),
                    vec!["HomeBudget::Api".to_string(), "HomeBudget::Api::Controllers".to_string()],
                ),
                (
                    "HomeBudget.Application".to_string(),
                    vec!["HomeBudget::Application::Services".to_string()],
                ),
                (
                    "HomeBudget.Domain".to_string(),
                    vec!["HomeBudget::Domain::Entities".to_string()],
                ),
            ]),
            ..base
        }
    }

    #[test]
    fn modules_graph_projects_unit_edges_of_single_module_units() {
        let graph = modules_graph(&csharp_like_model());
        assert!(graph.nodes.contains("Api"));
        assert!(graph.nodes.contains("Application"));
        assert!(graph.nodes.contains("Domain"));
        assert!(graph
            .edges
            .contains(&("Api".to_string(), "Application".to_string())));
        assert!(graph
            .edges
            .contains(&("Application".to_string(), "Domain".to_string())));
    }

    #[test]
    fn modules_graph_projects_unit_edges_of_multi_module_units_as_cross_product() {
        let base = model_with("app", Vec::new(), Vec::new());
        let model = Model {
            units: vec![unit("app"), unit("web")],
            edges: vec![Edge {
                from: "app".to_string(),
                to: "web".to_string(),
            }],
            soft_structure: BTreeMap::from([
                (
                    "app".to_string(),
                    vec!["app::x".to_string(), "app::y".to_string()],
                ),
                (
                    "web".to_string(),
                    vec!["web::p".to_string(), "web::q".to_string()],
                ),
            ]),
            ..base
        };
        let graph = modules_graph(&model);
        assert_eq!(
            graph.edges,
            BTreeSet::from([
                ("x".to_string(), "p".to_string()),
                ("x".to_string(), "q".to_string()),
                ("y".to_string(), "p".to_string()),
                ("y".to_string(), "q".to_string()),
            ]),
            "unit edge must project as the cross product of the endpoint top sets"
        );
    }

    #[test]
    fn modules_graph_projects_bin_unit_edge_onto_multi_module_lib() {
        let base = model_with("app", Vec::new(), Vec::new());
        let model = Model {
            units: vec![unit("app"), unit("app-bin")],
            edges: vec![Edge {
                from: "app-bin".to_string(),
                to: "app".to_string(),
            }],
            soft_structure: BTreeMap::from([
                (
                    "app".to_string(),
                    vec!["app::core_api".to_string(), "app::store".to_string()],
                ),
                ("app-bin".to_string(), vec!["app-bin::main".to_string()]),
            ]),
            ..base
        };
        let graph = modules_graph(&model);
        assert!(graph.edges.contains(&("main".to_string(), "core_api".to_string())));
        assert!(graph.edges.contains(&("main".to_string(), "store".to_string())));
    }

    #[test]
    fn modules_graph_skips_unit_self_reference_edges() {
        let base = model_with("app", Vec::new(), Vec::new());
        let model = Model {
            units: vec![unit("app")],
            edges: vec![Edge {
                from: "app".to_string(),
                to: "app".to_string(),
            }],
            soft_structure: BTreeMap::from([(
                "app".to_string(),
                vec!["app::x".to_string(), "app::y".to_string()],
            )]),
            ..base
        };
        let graph = modules_graph(&model);
        assert!(graph.edges.is_empty(), "unit self-reference must emit no edges");
    }

    #[test]
    fn submodules_graph_resolves_full_module_name_parent() {
        let model = csharp_like_model();
        let graph = submodules_graph(&model, "HomeBudget.Api").expect("full name resolves");
        assert!(graph.nodes.contains("Controllers"));
        assert!(graph.nodes.contains(MOD_NODE));
        let same = submodules_graph(&model, "HomeBudget::Api").expect("colon form resolves");
        assert_eq!(
            same.nodes.into_iter().collect::<Vec<_>>(),
            graph.nodes.into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn submodules_graph_unknown_parent_lists_known_names() {
        let model = csharp_like_model();
        let err = submodules_graph(&model, "HomeBudget.Ghost").expect_err("absent parent");
        assert!(err.contains("parent module not found in model: HomeBudget.Ghost"), "{err}");
        assert!(err.contains("HomeBudget::Api"), "must list names: {err}");
        assert!(err.contains("HomeBudget::Domain"), "must list names: {err}");
    }

    #[test]
    fn help_quotes_module_tier_sentence_verbatim() {
        assert!(
            crate::archspec::commands::depgraph::HELP.contains(MODULE_TIER_REQUIREMENT),
            "depgraph --help must quote the module-tier sentence verbatim"
        );
    }

    #[test]
    fn mermaid_nodes_then_edges_in_order() {
        let model = model_with(
            "app",
            vec!["app::core", "app::ui"],
            vec![("app::core", "app::ui", &[])],
        );
        let out = render_mermaid(&modules_graph(&model));
        assert!(out.starts_with("graph TD\n"));
        assert!(out.contains("core --> ui"));
    }
}
