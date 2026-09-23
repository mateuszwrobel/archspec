//! Dependency-graph projections of the extracted model, consumed by the
//! `depgraph` command. The model stores module edges at full dotted granularity
//! (`unit::parent::child`); these projections collapse those paths onto the
//! vocabulary an auto-generated dependency doc expects: top-level module names
//! (section a), an API-usage table grouped by target module at the same
//! granularity the modules graph renders (section b), and immediate-child
//! submodule names scoped to one parent (section c).
//!
//! The projections are pure functions of the model — no spec, no rules — so the
//! output is a current-state picture, matching `scan`/`inspect` rather than
//! `verify`. Rendering reuses the shared `render` primitives so escaping cannot
//! drift from the other diagram commands. The granularity is one decision for
//! every view (see [`projection_node`]): top-level modules, or one level deeper
//! when the top-level projection folds the whole tier onto a single node — a
//! structural rule, not a language branch.

use crate::archspec::model::{Model, Role};
use rust_arch_test_kit::render::{
    markdown_table_escape, mermaid_edge, mermaid_node, mermaid_node_marked, plantuml_component,
    plantuml_component_marked, plantuml_edge,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// Node name a unit-root module path folds to (a crate's `lib.rs`/`main.rs`).
const ROOT_NODE: &str = "root";
/// Node name a parent module's own file folds to in the submodule view.
const MOD_NODE: &str = "mod";

/// The capability sentence every `depgraph` view emits when the model carries
/// no module tier (`Model::has_module_tier` is false). Each view projects the
/// module tier and nothing language-specific
/// beyond it, so a tier-less model has no module structure to render and is
/// refused up front rather than with an internal-shape complaint. The remedy
/// states how the sole extraction path acquires the tier for a Go tree: a
/// tree WITH the tier renders, a tree WITHOUT it gets this error.
/// Kept as the single source of this wording — the command's `--help` line and
/// the depgraph docs quote it verbatim (locked by the
/// `depgraph_help_and_docs_quote_module_tier_sentence` test).
pub const MODULE_TIER_REQUIREMENT: &str = "depgraph needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of";

/// A directed module graph at a single granularity: node names, the distinct
/// edges between them, and the role markers the model states for its nodes.
/// Canonical ordering is inherent (`BTreeSet`, `BTreeMap`). The markers map
/// is populated by [`modules_graph`] / [`submodules_graph`] through
/// [`fold_markers`] (roles-views US 02) and stays empty wherever roles are
/// absent or too coarse to state — a graph built directly, or a model with
/// an empty roles map, renders exactly its pre-marker bytes.
#[derive(Debug, Clone, Default)]
pub struct ModuleGraph {
    pub nodes: BTreeSet<String>,
    pub edges: BTreeSet<(String, String)>,
    pub markers: BTreeMap<String, Role>,
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

/// The node a dotted module path projects to one level below the top tier: the
/// path's own final segment — the module's identity beneath whatever prefix it
/// shares with its siblings; a unit-root path (one segment) folds to `root`
/// exactly as in the top view. This is what sits below a trivially folded
/// projection: a single-module go tree addresses its packages below the module
/// path (host, organisation, repository segments), so its paths below the fold
/// are its packages, while a rust/csharp path below a top module is the child
/// module itself.
fn deep_node(dotted: &str) -> String {
    let segments: Vec<&str> = dotted.split("::").collect();
    if segments.len() <= 1 {
        ROOT_NODE.to_string()
    } else {
        segments[segments.len() - 1].to_string()
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
///
/// **Trivial fold**: when that projection collapses the whole tier onto ONE node
/// and the tier holds ≥ 2 paths beneath it, the one node says nothing about the
/// structure the model carries (a single-`go.mod` tree's 14-package graph renders
/// as a lone label). The view then renders one level deeper — the paths below the
/// fold become the nodes, labelled by their own final segment, and the edges
/// reproject at that level. Keyed on the shape of the projection, not on a
/// language: a rust or csharp tree whose projection has more than one node is
/// rendered by the top-level rule byte-for-byte unchanged, and a one-node
/// projection with nothing distinct below it keeps its single node.
pub fn modules_graph(model: &Model) -> ModuleGraph {
    let node = projection_node(model);
    let mut graph = project(model, node);
    let markers = fold_markers(model, |path| Some(node(path)));
    graph.markers = markers
        .into_iter()
        .filter(|(name, _)| graph.nodes.contains(name))
        .collect();
    graph
}

/// The granularity every depgraph view projects through: `top_node`, or
/// `deep_node` when the top-level projection trivially folds — collapses the
/// whole tier onto one node while the tier holds more distinct paths below it
/// (the rule [`modules_graph`] renders by, kept here as one decision so the
/// modules graph and the api-usage grouping cannot read different
/// granularities). Structural, not language-keyed, exactly as before.
fn projection_node(model: &Model) -> fn(&str) -> String {
    let top = project(model, top_node);
    if top.nodes.len() > 1 {
        return top_node;
    }
    let deeper = project(model, deep_node);
    if deeper.nodes.len() > top.nodes.len() {
        deep_node
    } else {
        top_node
    }
}

/// The node markers of one view (roles-views US 02): project every key of
/// the model's `roles` map through the view's own node function and collect
/// the roles landing on each rendered node. A node whose collected set
/// holds exactly one distinct role carries that role; a node onto which two
/// different roles fold stays silent — the fold is too coarse to state one
/// role, and silence beats an aggregate claim (no node ever carries several
/// markers). A key whose projection names no node (outside the view's
/// parent scope) or names a node the view does not render contributes
/// nothing — the rule never invents a node to carry a role. Roles keys may
/// carry the driver's native path spelling (go import paths spell
/// separators `/`), lifted here to the model's `::` path grammar before
/// projection — the same separator tolerance `parent_matches` applies to
/// identity lookups, kept in this one place so no second lookup diverges.
/// Pure lookup of serialized model facts, no derivation, no language branch.
fn fold_markers(model: &Model, node: impl Fn(&str) -> Option<String>) -> BTreeMap<String, Role> {
    let mut claimed: BTreeMap<String, Option<Role>> = BTreeMap::new();
    for (key, role) in &model.roles {
        let Some(name) = node(&key.replace('/', "::")) else {
            continue;
        };
        let next = match claimed.get(&name) {
            None => Some(*role),
            Some(Some(existing)) if *existing == *role => Some(*role),
            // A different role folded onto this node, or a node already
            // silenced by an earlier conflict: it stays silent.
            Some(_) => None,
        };
        claimed.insert(name, next);
    }
    claimed
        .into_iter()
        .filter_map(|(name, role)| role.map(|role| (name, role)))
        .collect()
}

/// Project the model through one node function: soft paths contribute nodes,
/// module edges contribute their endpoints and (when they differ) an edge,
/// unit edges project through module membership. Shared by every granularity
/// the modules view reads.
fn project(model: &Model, node: fn(&str) -> String) -> ModuleGraph {
    let mut graph = ModuleGraph::default();
    for paths in model.soft_structure.values() {
        for path in paths {
            graph.add_node(&node(path));
        }
    }
    for edge in &model.module_edges {
        let from = node(&edge.from);
        let to = node(&edge.to);
        graph.add_node(&from);
        graph.add_node(&to);
        if from != to {
            graph.edges.insert((from, to));
        }
    }
    project_unit_edges(model, &mut graph, node);
    graph
}

/// Map each unit to the SET of nodes its soft paths fold to under `node`, then
/// draw every pair of the cross product for each unit edge whose endpoints are
/// both present, skipping self-edges (pairs naming the same node) and unit
/// self-references (a unit edge from a unit to itself says nothing about
/// cross-unit structure; its internal shape already reads from module edges).
fn project_unit_edges(model: &Model, graph: &mut ModuleGraph, node: fn(&str) -> String) {
    if model.edges.is_empty() {
        return;
    }
    let mut unit_nodes: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (unit, paths) in &model.soft_structure {
        unit_nodes.insert(unit.as_str(), paths.iter().map(|path| node(path)).collect());
    }
    for edge in &model.edges {
        if edge.from == edge.to {
            continue;
        }
        let (Some(from_nodes), Some(to_nodes)) = (
            unit_nodes.get(edge.from.as_str()),
            unit_nodes.get(edge.to.as_str()),
        ) else {
            continue;
        };
        for from in from_nodes {
            for to in to_nodes {
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
    let markers = fold_markers(model, |path| child_node(path, parent));
    graph.markers = markers
        .into_iter()
        .filter(|(name, _)| graph.nodes.contains(name))
        .collect();
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

/// Group the symbols used on module edges by target module, then by using
/// module, at the granularity the modules view renders: `top_node`, or the
/// paths below the fold when the top projection trivially folds (shared rule
/// with [`modules_graph`], so a folded tree's table names the same nodes its
/// graph shows). An edge whose endpoints project to the same node contributes
/// nothing, and a pair with no recorded symbol contributes nothing (it never
/// opens a group).
pub fn api_usage_by_target(model: &Model) -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let node = projection_node(model);
    let mut grouped: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for edge in &model.module_edges {
        let from = node(&edge.from);
        let to = node(&edge.to);
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
/// one solid arrow per edge, in canonical order. A node the graph carries a
/// marker for renders the roles US 06 label suffix (`mermaid_node_marked`);
/// every other node line keeps `mermaid_node`'s exact bytes, so ids and all
/// edge references are the unmarked render's ids.
pub fn render_mermaid(graph: &ModuleGraph) -> String {
    let mut out = String::from("graph TD\n");
    for node in &graph.nodes {
        let line = match graph.markers.get(node) {
            Some(role) => mermaid_node_marked(node, &format!(" [{}]", role.as_str())),
            None => mermaid_node(node),
        };
        let _ = writeln!(out, "  {line}");
    }
    for (from, to) in &graph.edges {
        let _ = writeln!(out, "  {}", mermaid_edge(from, to));
    }
    out
}

/// Render a module graph as a PlantUML component diagram. A marked node
/// carries the role stereotype on its bare declaration identifier
/// (`plantuml_component_marked`); every other line — unmarked declarations
/// and all edges — keeps its exact pre-marker bytes.
pub fn render_plantuml(graph: &ModuleGraph) -> String {
    let mut out = String::from("@startuml\n");
    for node in &graph.nodes {
        let line = match graph.markers.get(node) {
            Some(role) => plantuml_component_marked(node, role.as_str()),
            None => plantuml_component(node),
        };
        let _ = writeln!(out, "{line}");
    }
    for (from, to) in &graph.edges {
        let _ = writeln!(out, "{}", plantuml_edge(from, to));
    }
    let _ = writeln!(out, "@enduml");
    out
}

/// The reason sentence an empty api-usage result carries when the emptiness is
/// a fact of **where** the extractor records usage rather than an
/// absence of usage. The two language arms state their tree's placement as
/// before: for csharp an empty grouping states that no symbol facts were
/// emitted for this tree, because the extractor does emit symbol facts
/// wherever the tree carries cross-boundary references; the rust driver
/// records symbols, but its module edges only ever connect modules **within
/// one unit** — a tree whose cross-target links all live at the unit tier
/// (the `edges`) groups to nothing while the usage exists there, so an empty
/// result states that placement, not a capability gap. Everything else —
/// formerly the go hole, where no reason could ever be said — is routed
/// structurally (audit D4): a model whose module edges carry symbols can
/// never print the bare statement, so if the grouping came out empty anyway
/// (every symbol-carrying edge lands on one rendered node, the usage sitting
/// below the granularity the view renders) the statement carries the
/// fold-placement sentence. The bare statement survives exactly where it is
/// honest: a model with no symbol facts on its module edges and nothing else
/// to say — genuinely no recorded cross-module usage — for any language.
pub fn api_usage_empty_reason(model: &Model) -> Option<&str> {
    match model.language.as_str() {
        "csharp" => Some(NO_SYMBOL_FACTS_REASON),
        "rust" if model.edges.iter().any(|edge| edge.from != edge.to) => Some(
            RUST_UNIT_TIER_PLACEMENT_REASON,
        ),
        _ if model.module_edges.iter().any(|edge| !edge.symbols.is_empty()) => {
            Some(MODULE_TIER_FOLD_PLACEMENT_REASON)
        }
        _ => None,
    }
}

/// The emptiness statement for a c# tree whose module edges carry no symbols:
/// a fact of this tree, not a driver-wide absence.
pub const NO_SYMBOL_FACTS_REASON: &str =
    "No symbol facts were emitted for this tree.";
/// The rust placement sentence.
pub const RUST_UNIT_TIER_PLACEMENT_REASON: &str =
    "Module edges record usage between modules of a single unit; cross-target links are recorded at unit granularity.";
/// The placement sentence for a model that DOES carry symbol facts on module
/// edges while the grouping stayed empty: the symbols exist, but every
/// symbol-carrying edge projects onto a single node at the granularity the
/// views render (the trivial-fold condition one level too deep to help), so
/// the emptiness is a fact of tier placement, not of absence.
pub const MODULE_TIER_FOLD_PLACEMENT_REASON: &str =
    "Module edges carry symbols, but every symbol-carrying edge stays inside one rendered module; the usage sits between modules below the granularity this view renders.";

/// The api-usage view's self-stated silence (roles plan, US 06): roles never
/// appear in this output **by decision** — a role is not a usage fact — and
/// the view says so in one line so the silence can never be misread as
/// breakage. The line is unconditional on the human form (its subject is the
/// view's design, not the model's state), uses the established `note: `
/// prefix, and is welded to the `none by decision` cell of the views ×
/// markers matrix (`tests/shared/roles_matrix.rs`): one surface cannot
/// change without the guard naming the other. The canonical sentence carries
/// a `help roles` pointer suffix (blind-4 follow-up US 04) so the note routes
/// the reader to where the decision is explained; the matrix weld pins the
/// sentence itself, byte-intact, as the emitted line's prefix.
pub const ROLE_LESS_NOTE: &str = "note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'";

/// The api-usage body for a model: the grouped table, or the empty statement
/// with the driver's reason appended when the emptiness is a fact of where
/// usage is recorded rather than an absence of usage. Both forms end with the
/// role-less note line stating the view's designed silence.
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
/// usage. The table's and the statement's bytes are untouched by the role-less
/// note, which trails them on its own line.
pub fn render_api_usage_markdown(
    grouped: &BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    empty_reason: Option<&str>,
) -> String {
    if grouped.is_empty() {
        let statement = match empty_reason {
            Some(reason) => format!("No internal API usage details found. {reason}\n"),
            None => "No internal API usage details found.\n".to_string(),
        };
        return format!("{statement}{ROLE_LESS_NOTE}\n");
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
    out.push_str(ROLE_LESS_NOTE);
    out.push('\n');
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
            roles: Default::default(),
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

    /// The trivial-fold rule: a model whose module tier addresses its members
    /// below a shared multi-segment prefix (the shape a single-`go.mod` tree
    /// records — host, organisation and repository segments above the package)
    /// projects to exactly one top node while the tier holds the whole graph
    /// below it. The view renders that graph instead: one node per path below the
    /// fold, labelled by the path's own final segment, edges reprojected there.
    #[test]
    fn modules_graph_renders_children_when_top_projection_folds_to_one_node() {
        let model = model_with(
            "example.com::demo",
            Vec::new(),
            vec![
                ("example.com::demo::app", "example.com::demo::shared", &[] as &[&str]),
                ("example.com::demo::shared", "example.com::demo::store", &[]),
            ],
        );
        let graph = modules_graph(&model);
        assert_eq!(
            graph.nodes.iter().cloned().collect::<Vec<_>>(),
            vec!["app".to_string(), "shared".to_string(), "store".to_string()],
            "a trivially folded projection must render the paths below the fold"
        );
        assert!(graph.edges.contains(&("app".to_string(), "shared".to_string())));
        assert!(graph.edges.contains(&("shared".to_string(), "store".to_string())));
        assert!(
            !graph.edges.iter().any(|(from, to)| from == to),
            "the deeper render must not emit self-edges"
        );
    }

    /// Guard for the rule above: the fold fires on a ONE-node projection only. A
    /// tree whose top projection already carries several nodes keeps the top-level
    /// vocabulary, however deep its paths go (rust/csharp trees).
    #[test]
    fn modules_graph_keeps_top_vocabulary_when_projection_has_several_nodes() {
        let model = model_with(
            "app",
            vec!["app::core", "app::core::a", "app::ui"],
            vec![("app::core::a", "app::ui", &[] as &[&str])],
        );
        let graph = modules_graph(&model);
        assert!(
            graph.nodes.contains("core"),
            "top-level node must survive: {:?}",
            graph.nodes
        );
        assert!(
            !graph.nodes.contains("a"),
            "deeper names must not leak into a multi-node render: {:?}",
            graph.nodes
        );
        assert!(graph.edges.contains(&("core".to_string(), "ui".to_string())));
    }

    /// Second guard: the rule needs something BELOW the fold. A one-node
    /// projection with no distinct path under it stays that one node — no descent,
    /// no invented children, no self-edge.
    #[test]
    fn modules_graph_keeps_one_node_when_nothing_sits_below_the_fold() {
        let model = model_with(
            "example.com::demo",
            Vec::new(),
            vec![("example.com::demo::app", "example.com::demo::app", &[] as &[&str])],
        );
        let graph = modules_graph(&model);
        assert_eq!(
            graph.nodes.iter().cloned().collect::<Vec<_>>(),
            vec!["demo".to_string()]
        );
        assert!(graph.edges.is_empty(), "a path depending on itself emits no edge");
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
    fn api_usage_ignores_same_node_edges_when_projection_has_several_nodes() {
        // Same-node edges never open a group — pinned on a multi-node
        // projection, where the grouping granularity stays at the top.
        // Folded shapes group one level deeper (see
        // `api_usage_groups_symbols_at_folded_granularity`).
        let model = model_with(
            "app",
            vec!["app::a", "app::a::b", "app::ui"],
            vec![("app::a::b", "app::a", &["Thing"])],
        );
        assert!(api_usage_by_target(&model).is_empty());
    }

    #[test]
    fn api_usage_groups_symbols_at_folded_granularity() {
        // The trivial fold (shared with modules_graph): all paths share a
        // multi-segment prefix, so the grouping reads the paths below the
        // fold — the symbols on the package edge land in child-level rows.
        let model = model_with(
            "example.com::demo",
            Vec::new(),
            vec![("example.com::demo::store", "example.com::demo::format", &["Title"])],
        );
        let grouped = api_usage_by_target(&model);
        assert_eq!(
            grouped["format"]["store"],
            BTreeSet::from(["Title".to_string()]),
            "a folded tree's symbols must group at the depth the graph renders"
        );
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
        assert_eq!(
            out,
            format!("No internal API usage details found.\n{ROLE_LESS_NOTE}\n")
        );
    }

    #[test]
    fn api_usage_markdown_appends_reason_when_given() {
        let out = render_api_usage_markdown(&BTreeMap::new(), Some(NO_SYMBOL_FACTS_REASON));
        assert_eq!(
            out,
            format!(
                "No internal API usage details found. No symbol facts were \
                 emitted for this tree.\n{ROLE_LESS_NOTE}\n"
            )
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
        let reason = api_usage_empty_reason(&model)
            .expect("unit-tier links must carry a reason");
        assert!(
            reason.contains("unit granularity"),
            "reason must state where the links are recorded: {reason}"
        );
        assert!(
            !reason.contains("not emitted"),
            "rust records symbols; the reason is placement, not capability: {reason}"
        );
    }

    #[test]
    fn api_usage_empty_reason_csharp_states_tree_fact_not_driver_absence() {
        let mut model = model_with("App", vec!["App::Api", "App::Core"], vec![]);
        model.language = "csharp".to_string();
        let reason = api_usage_empty_reason(&model)
            .expect("the emptiness carries the tree statement");
        assert_eq!(reason, "No symbol facts were emitted for this tree.");
        assert!(!reason.contains("driver"), "not a driver-wide claim: {reason}");
    }

    #[test]
    fn api_usage_empty_reason_go_keeps_bare_statement() {
        // A symbol-free model (no module edges carry a symbol): the
        // structural arm stays silent and the bare statement is honest —
        // the routing keys on model facts, not on the go label.
        let mut model = model_with("example.com/demo", vec![], vec![]);
        model.language = "go".to_string();
        assert_eq!(api_usage_empty_reason(&model), None);
    }

    #[test]
    fn api_usage_empty_reason_symbol_bearing_fold_states_placement() {
        // The D4 rule: a language arm says nothing, but the model has symbol
        // facts on module edges while the grouping is empty — the sentence
        // names the tier placement, never an absence.
        let mut model = model_with(
            "example.com::tools",
            Vec::new(),
            vec![(
                "example.com::tools::alpha::util",
                "example.com::tools::beta::util",
                &["Title"] as &[&str],
            )],
        );
        model.language = "go".to_string();
        let reason = api_usage_empty_reason(&model)
            .expect("symbols on edges forbid the bare statement");
        assert_eq!(reason, MODULE_TIER_FOLD_PLACEMENT_REASON);
        assert!(reason.contains("symbols"), "the reason must admit the symbols: {reason}");
        assert!(
            !reason.contains("not emitted") && !reason.contains("no symbol facts"),
            "the model has symbol facts; absence wording would lie: {reason}"
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
        let base = model_with("Shop.Api", Vec::new(), Vec::new());
        Model {
            language: "csharp".to_string(),
            units: vec![
                unit("Shop.Api"),
                unit("Shop.Application"),
                unit("Shop.Domain"),
            ],
            edges: vec![
                Edge {
                    from: "Shop.Api".to_string(),
                    to: "Shop.Application".to_string(),
                },
                Edge {
                    from: "Shop.Application".to_string(),
                    to: "Shop.Domain".to_string(),
                },
            ],
            soft_structure: BTreeMap::from([
                (
                    "Shop.Api".to_string(),
                    vec!["Shop::Api".to_string(), "Shop::Api::Controllers".to_string()],
                ),
                (
                    "Shop.Application".to_string(),
                    vec!["Shop::Application::Services".to_string()],
                ),
                (
                    "Shop.Domain".to_string(),
                    vec!["Shop::Domain::Entities".to_string()],
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
        let graph = submodules_graph(&model, "Shop.Api").expect("full name resolves");
        assert!(graph.nodes.contains("Controllers"));
        assert!(graph.nodes.contains(MOD_NODE));
        let same = submodules_graph(&model, "Shop::Api").expect("colon form resolves");
        assert_eq!(
            same.nodes.into_iter().collect::<Vec<_>>(),
            graph.nodes.into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn submodules_graph_unknown_parent_lists_known_names() {
        let model = csharp_like_model();
        let err = submodules_graph(&model, "Shop.Ghost").expect_err("absent parent");
        assert!(err.contains("parent module not found in model: Shop.Ghost"), "{err}");
        assert!(err.contains("Shop::Api"), "must list names: {err}");
        assert!(err.contains("Shop::Domain"), "must list names: {err}");
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

    // --- node-marker projection (roles-views US 02) -------------------------

    fn model_with_roles(
        unit: &str,
        soft: Vec<&str>,
        edges: Vec<(&str, &str)>,
        roles: &[(&str, Role)],
    ) -> Model {
        let mut model = model_with(
            unit,
            soft,
            edges
                .iter()
                .map(|(from, to)| (*from, *to, &[] as &[&str]))
                .collect(),
        );
        model.roles = roles
            .iter()
            .map(|(key, role)| (key.to_string(), *role))
            .collect();
        model
    }

    /// A single role-carrying path marks the node it projects onto; the
    /// nodes the map leaves unaddressed keep their plain bytes.
    #[test]
    fn modules_graph_marks_the_node_a_single_role_path_projects_onto() {
        let model = model_with_roles(
            "app",
            vec!["app::core", "app::ui"],
            vec![("app::core", "app::ui")],
            &[("app::core", Role::Facade)],
        );
        let graph = modules_graph(&model);
        assert_eq!(
            graph.markers,
            BTreeMap::from([("core".to_string(), Role::Facade)])
        );
        let out = render_mermaid(&graph);
        assert!(out.contains("core[\"core [facade]\"]"), "{out}");
        assert!(out.contains("  ui\n"), "unmarked node keeps bare bytes: {out}");
        assert!(out.contains("core --> ui"), "edges unchanged: {out}");
    }

    /// The go spelling: the key is an import path (`/` separators), the
    /// projection lifts it to the model path grammar, and the trivial fold
    /// reads the paths below it — so the key marks the package's own node.
    #[test]
    fn modules_graph_marks_go_composition_key_in_native_separator_spelling() {
        let base = model_with("example.com::demo", Vec::new(), Vec::new());
        let mut model = Model {
            language: "go".to_string(),
            module_edges: vec![
                ModuleEdge {
                    unit: "example.com::demo".to_string(),
                    from: "example.com::demo::app".to_string(),
                    to: "example.com::demo::shared".to_string(),
                    symbols: Default::default(),
                },
            ],
            ..base
        };
        model.roles = BTreeMap::from([("example.com/demo/app".to_string(), Role::Composition)]);
        let graph = modules_graph(&model);
        assert_eq!(
            graph.markers,
            BTreeMap::from([("app".to_string(), Role::Composition)])
        );
        assert!(render_plantuml(&graph).contains("component app <<composition>>"));
    }

    /// Two roles folding onto one rendered node state no single role: the
    /// node stays silent (and never grows two markers).
    #[test]
    fn modules_graph_conflicting_fold_states_no_role() {
        let base = model_with("X.Api", Vec::new(), Vec::new());
        let model = Model {
            language: "csharp".to_string(),
            units: vec![unit("X.Api"), unit("Y.Api")],
            soft_structure: BTreeMap::from([
                (
                    "X.Api".to_string(),
                    vec!["X::Api::Wiring".to_string()],
                ),
                (
                    "Y.Api".to_string(),
                    vec!["Y::Api::Program".to_string()],
                ),
                // A second top module keeps this model off the trivial-fold
                // path: the fold under test is the roles-key fold, not the
                // projection's descent.
                (
                    "Shared.Lib".to_string(),
                    vec!["Shared::Lib::Core".to_string()],
                ),
            ]),
            roles: BTreeMap::from([
                ("X::Api".to_string(), Role::Facade),
                ("Y::Api".to_string(), Role::Composition),
            ]),
            ..base
        };
        let graph = modules_graph(&model);
        assert!(graph.nodes.contains("Api"), "{:?}", graph.nodes);
        assert!(
            graph.markers.is_empty(),
            "distinct folded roles must silence the node: {:?}",
            graph.markers
        );
        // Contrast: the same two paths agreeing on one role mark their node.
        let model = Model {
            roles: BTreeMap::from([
                ("X::Api".to_string(), Role::Facade),
                ("Y::Api".to_string(), Role::Facade),
            ]),
            ..model
        };
        let graph = modules_graph(&model);
        assert_eq!(
            graph.markers,
            BTreeMap::from([("Api".to_string(), Role::Facade)]),
            "agreeing folded roles state that role, once"
        );
        assert_eq!(
            render_mermaid(&graph).matches("Api[\"Api [facade]\"]").count(),
            1,
            "one node, one marker"
        );
    }

    /// A key whose projection names a node the view does not render (a
    /// unit-tier rust key folding to `root` with no unit-root path in the
    /// tier) contributes nothing: no node is invented, no marker printed.
    #[test]
    fn modules_graph_key_folding_to_unrendered_node_contributes_nothing() {
        let model = model_with_roles(
            "app",
            vec!["app::core", "app::ui"],
            vec![("app::core", "app::ui")],
            &[("app", Role::Facade)],
        );
        let graph = modules_graph(&model);
        assert!(!graph.nodes.contains(ROOT_NODE), "no node invented: {:?}", graph.nodes);
        assert!(graph.markers.is_empty(), "{:?}", graph.markers);
    }

    /// The same key does mark `root` when the tier addresses the unit root
    /// and the node IS rendered — the rule is about the projection's target,
    /// not about a name blacklist.
    #[test]
    fn modules_graph_marks_unit_root_key_when_the_root_node_is_rendered() {
        let model = model_with_roles(
            "app",
            vec!["app", "app::core"],
            Vec::new(),
            &[("app", Role::Facade)],
        );
        let graph = modules_graph(&model);
        assert_eq!(
            graph.markers,
            BTreeMap::from([(ROOT_NODE.to_string(), Role::Facade)])
        );
    }

    /// Insertion order of the roles map cannot change the fold: the marker
    /// set and the rendered bytes are a function of the map's contents only.
    #[test]
    fn modules_graph_markers_ignore_roles_map_insertion_order() {
        let mut first = model_with("app", vec!["app::a", "app::b"], Vec::new());
        for (key, role) in [
            ("app::a", Role::Facade),
            ("app::b", Role::Facade),
            ("ghost::gone", Role::Composition),
        ] {
            first.roles.insert(key.to_string(), role);
        }
        let mut second = model_with("app", vec!["app::a", "app::b"], Vec::new());
        for (key, role) in [
            ("ghost::gone", Role::Composition),
            ("app::b", Role::Facade),
            ("app::a", Role::Facade),
        ] {
            second.roles.insert(key.to_string(), role);
        }
        let a = render_mermaid(&modules_graph(&first));
        let b = render_mermaid(&modules_graph(&second));
        assert_eq!(a, b, "fold markers must not depend on insertion order");
        assert_eq!(
            modules_graph(&first).markers,
            BTreeMap::from([
                ("a".to_string(), Role::Facade),
                ("b".to_string(), Role::Facade),
            ]),
        );
    }

    /// The zero-theater byte pin: a role-free model renders the exact
    /// pre-marker graph — markers default-empty, every line as before.
    #[test]
    fn modules_graph_with_empty_roles_renders_pre_marker_bytes() {
        let model = model_with("app", vec!["app::core", "app::ui"], vec![("app::core", "app::ui", &["Thing"])]);
        let mermaid = render_mermaid(&modules_graph(&model));
        assert_eq!(mermaid, "graph TD\n  core\n  ui\n  core --> ui\n");
        let plantuml = render_plantuml(&modules_graph(&model));
        assert_eq!(
            plantuml,
            "@startuml\ncomponent core\ncomponent ui\ncore --> ui\n@enduml\n"
        );
    }

    /// The submodule view marks through its own `child_node` projection:
    /// children by name, the parent's own file via the `mod` fold, and keys
    /// outside the parent's subtree contribute nothing.
    #[test]
    fn submodules_graph_marks_children_through_the_child_projection() {
        let mut model = model_with(
            "app",
            vec![
                "app::orchestration",
                "app::orchestration::control_loop",
                "app::orchestration::progress",
                "app::config",
            ],
            Vec::new(),
        );
        model.roles = BTreeMap::from([
            ("app::orchestration::control_loop".to_string(), Role::Facade),
            ("app::orchestration".to_string(), Role::Composition),
            ("app::config".to_string(), Role::Facade),
        ]);
        let graph = submodules_graph(&model, "orchestration").expect("parent present");
        assert_eq!(
            graph.markers,
            BTreeMap::from([
                ("control_loop".to_string(), Role::Facade),
                (MOD_NODE.to_string(), Role::Composition),
            ]),
            "the key under another parent must project to nothing"
        );
    }

    /// Conflicting children under one parent fold silently, exactly like the
    /// modules view's conflicting fold: two paths below the same child fold
    /// onto one node with different roles.
    #[test]
    fn submodules_graph_conflicting_children_stay_silent() {
        let mut model = model_with(
            "app",
            vec!["app::orchestration::a::one", "app::orchestration::a::two"],
            Vec::new(),
        );
        model.roles = BTreeMap::from([
            ("app::orchestration::a::one".to_string(), Role::Facade),
            ("app::orchestration::a::two".to_string(), Role::Composition),
        ]);
        let graph = submodules_graph(&model, "orchestration").expect("parent present");
        assert!(graph.nodes.contains("a"), "{:?}", graph.nodes);
        assert!(graph.markers.is_empty(), "{:?}", graph.markers);
    }
}
