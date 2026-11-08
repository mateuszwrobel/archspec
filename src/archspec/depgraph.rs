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
    /// Role facts that folded onto one node with a DIFFERENT role and so
    /// state no marker (roles reality US 05): the renderers state this count
    /// in one note line instead of hiding the drop in silence. Claims that
    /// fold outside the rendered vocabulary are a placement fact of the
    /// view, not a drop, and never count here.
    pub unplaced_claims: usize,
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
///
/// **Collapse drop** (GitHub issue #20): when EVERY module-tier fold — top,
/// deeper, identity — still projects the whole tier onto ONE node while the
/// model's units tier renders more than one node (a monorepo whose packages
/// address each other at the module tier through their bare unit names, so
/// every endpoint folds to `root` and every merged self-edge is suppressed),
/// the one node is a contentless render of a model that demonstrably carries a
/// graph. The view then drops to the units tier — the projection
/// `--aggregate units` draws, node set and edges identical — so a single-node
/// render never escapes while the units tier shows more. Structural, keyed on
/// the two projections' node counts, not on a language. Where the units tier
/// also carries one node, the single node IS the honest render, and the
/// automatic path states the collapse loudly (see [`automatic_projection_note`]).
pub fn modules_graph(model: &Model) -> ModuleGraph {
    let graph = project_marked(model, projection_node(model));
    if graph.nodes.len() <= 1 {
        let units = units_graph(model);
        if units.nodes.len() > 1 {
            return units;
        }
    }
    graph
}

/// Project through one node function and attach the markers of that same
/// fold. The shared body of every whole-tier graph view (automatic and
/// forced), so a forced level cannot drift from the automatic one in
/// marker or drop-count handling.
fn project_marked(model: &Model, node: fn(&str) -> String) -> ModuleGraph {
    let mut graph = project(model, node);
    let markers = fold_markers(model, |path| Some(node(path)), &graph.nodes);
    graph.markers = markers
        .0
        .into_iter()
        .filter(|(name, _)| graph.nodes.contains(name))
        .collect();
    graph.unplaced_claims = markers.1;
    graph
}

/// The module-tier projection pinned to the top level: the automatic rule's
/// first trial with no fold and no descent (`--aggregate modules`). A tree
/// whose whole tier folds onto one node renders that one node honestly on
/// the dotted tiers; a file-grained tier (GitHub issue 15) does not fold
/// onto `root` at all — its module tier addresses files, so its top level is
/// the module roots above them and the view projects there through
/// [`file_modules_graph`]. The view never reads the deeper arms here.
pub fn top_modules_graph(model: &Model) -> ModuleGraph {
    if module_tier_is_file_grained(model) {
        return file_modules_graph(model);
    }
    project_marked(model, top_node)
}

/// True when the model's module edges name files: an endpoint carries a
/// directory separator (the ts driver keys its `module_edges` rows by file
/// identity — the split `Model::file_edges` documents), so the dotted fold
/// of [`top_node`] collapses every endpoint onto `root`. The go driver
/// lifts `/` to `::` in its paths, so no other driver's endpoints trip this.
/// Structural, not language-keyed — the same spelling discriminator
/// [`unit_of_path`] applies to its endpoints.
fn module_tier_is_file_grained(model: &Model) -> bool {
    model
        .module_edges
        .iter()
        .any(|edge| edge.from.contains('/') || edge.to.contains('/'))
}

/// The module root of one file endpoint: the module-tier node above a file
/// in a tier whose module identity IS the path. A file under a member unit's
/// directory folds to that unit — a workspace member is its own module root,
/// the fold [`units_graph`] already attributes to. A file the root unit owns
/// folds to its first path segment (the root package's module root, the
/// `src` of a single-package tree); a file with no directory below the root
/// folds to `root`, the fold a unit-root dotted path already performs in
/// [`top_node`]. The rule mirrors [`top_node`] across the separator spelling
/// and never invents an owner: an endpoint no unit claims still folds by its
/// path, because the fold states a path fact, not a membership.
fn module_root_node(model: &Model, path: &str) -> String {
    if let Some(unit) = unit_of_path(model, path)
        .as_deref()
        .and_then(|name| model.units.iter().find(|unit| unit.name == name))
        && unit.path.trim_end_matches('/') != "."
    {
        return unit.name.clone();
    }
    match path.replace("::", "/").split_once('/') {
        Some((head, _)) if !head.is_empty() => head.to_string(),
        _ => ROOT_NODE.to_string(),
    }
}

/// The module-tier projection of a file-grained tier (`--aggregate modules`
/// where the module edges name files, GitHub issue 15): module edges fold
/// through [`module_root_node`], an edge whose endpoints land on one module
/// root is a self-edge and emits nothing, and unit-tier dependency edges
/// project through the fold of their endpoint units' module roots exactly as
/// [`project_unit_edges`] draws them on the dotted tiers — a unit without
/// module-tier facts contributes nothing. Shared with [`units_graph`] down
/// to the containers: the same attribution, the same `BTreeSet` merge
/// (deduplicated, never counted), and the markers of the same fold.
fn file_modules_graph(model: &Model) -> ModuleGraph {
    let mut graph = ModuleGraph::default();
    let mut unit_nodes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in &model.module_edges {
        let from = module_root_node(model, &edge.from);
        let to = module_root_node(model, &edge.to);
        graph.add_node(&from);
        graph.add_node(&to);
        if from != to {
            graph.edges.insert((from.clone(), to.clone()));
        }
        for (path, node) in [(&edge.from, &from), (&edge.to, &to)] {
            if let Some(unit) = unit_of_path(model, path) {
                unit_nodes.entry(unit).or_default().insert(node.clone());
            }
        }
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
    let markers = fold_markers(model, |path| Some(module_root_node(model, path)), &graph.nodes);
    graph.markers = markers
        .0
        .into_iter()
        .filter(|(name, _)| graph.nodes.contains(name))
        .collect();
    graph.unplaced_claims = markers.1;
    graph
}

/// The upper bound on what the AUTOMATIC projection may render before the
/// view calls itself degraded. Justification of the number: the vocabulary
/// the automatic rule aims at is a project's module list, and a module list
/// a dependency doc can print is hundreds of entries at most; the reported
/// incident renders 68,563 lines — one node per file. A graph over 200 nodes
/// therefore cannot be read as a module picture (the largest healthy trees
/// this tool's goldens and probes carry stay an order of magnitude below
/// the cap), and the automatic rule has landed below module granularity —
/// file-level nodes wherever the module tier is file-grained. The cap is a
/// note threshold, not a render limit: the graph still prints in full. It
/// guards the automatic path only; an explicit `--aggregate` level is never
/// capped.
pub const AUTOMATIC_PROJECTION_NODE_CAP: usize = 200;

/// The one degraded-render note (stderr, automatic path only): names the
/// degradation and the count, stays honest about the condition (file-level
/// nodes WHERE the module tier is file-grained), and points at the flag
/// that forces a coarser level. It never replaces or trims the render.
pub fn degraded_projection_note(count: usize) -> String {
    format!("note: the automatic projection rendered {count} nodes below module granularity (file-level nodes where the module tier is one module per file) — pass --aggregate units to force the unit tier or --aggregate modules to pin the top level")
}

/// The single-node collapse note (stderr, automatic path only, GitHub issue
/// #20): the same mechanism as [`degraded_projection_note`] — it names the
/// degradation and the count, states the condition honestly (no projection
/// level separates the paths AND the units tier names no more), points at the
/// flag, and never replaces or trims the render. It fires exactly where the
/// [`modules_graph`] collapse drop could not: the fold projected one node and
/// the units tier shows one node too, so no coarser render exists to drop to
/// — the render keeps its one node and says so.
pub fn collapsed_projection_note(count: usize) -> String {
    format!("note: the automatic projection folded {count} module paths onto one node (no projection level separates them and the units tier names no more) — pass --aggregate units to force the unit tier or --aggregate modules to pin the top level")
}

/// The note the AUTOMATIC modules path owes its render, if any (stderr only,
/// an explicit `--aggregate` level is never noted):
/// * above the node cap and the render IS the module-tier projection → the
///   degraded-render note, bytes unchanged; a units-tier collapse-drop render
///   is not a below-module-granularity render, so the cap note stays silent
///   on it (its file-grained claim would be false there);
/// * a single node whose tier holds more than one distinct module path → the
///   collapse note (the drop to the units tier already replaced every render
///   the units tier could draw honestly, so reaching this arm means no
///   coarser level separates the paths);
/// * anything else → nothing, and the render keeps its exact bytes.
pub fn automatic_projection_note(model: &Model, graph: &ModuleGraph) -> Option<String> {
    if graph.nodes.len() > AUTOMATIC_PROJECTION_NODE_CAP {
        if project_marked(model, projection_node(model)).nodes.len() > 1 {
            return Some(degraded_projection_note(graph.nodes.len()));
        }
        return None;
    }
    if graph.nodes.len() <= 1 {
        let paths = distinct_module_paths(model);
        if paths > 1 {
            return Some(collapsed_projection_note(paths));
        }
    }
    None
}

/// How many distinct module paths the model's module tier names (soft paths
/// and module-edge endpoints, deduplicated): more than one with a one-node
/// render is the fold losing every distinction the tier states.
fn distinct_module_paths(model: &Model) -> usize {
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    for unit_paths in model.soft_structure.values() {
        for path in unit_paths {
            paths.insert(path.as_str());
        }
    }
    for edge in &model.module_edges {
        paths.insert(edge.from.as_str());
        paths.insert(edge.to.as_str());
    }
    paths.len()
}

/// The unit a module path or file path belongs to, as a pure function of the
/// serialized model facts — the scan-side directory-ownership rule read
/// backwards from `units` (name and path) with no driver, no filesystem,
/// and no language branch. A dotted module path (`unit::a::b`) addresses
/// its unit by name prefix: the path equals the unit name or continues it
/// with one more segment (rust/csharp/go units, whose names carry the whole
/// dotted prefix). A file-path endpoint (`dir/file.ts`, the ts driver's
/// module identity) addresses its unit by the LONGEST unit-directory path
/// that is a directory prefix — the root unit (path `.`) owns every file no
/// member directory owns, exactly as the ts scan's own ownership rule does.
/// A key that matches nothing (a lifted separator spelling under no member
/// directory) attributes to no unit and contributes no edge — the rule
/// never invents an owner.
pub fn unit_of_path(model: &Model, path: &str) -> Option<String> {
    if path.contains('/') {
        return directory_owner(model, path, true)
            .or_else(|| name_prefix_owner(model, path));
    }
    name_prefix_owner(model, path).or_else(|| {
        // A native `/` key lifted to the `::` spelling by fold_markers'
        // separator lift: retry as a directory path, but WITHOUT the root
        // catch-all — a lifted spelling says nothing about root membership.
        let lifted = path.replace("::", "/");
        (lifted != *path).then(|| directory_owner(model, &lifted, false)).flatten()
    })
}

/// The unit whose `path` is the longest directory prefix of `slashed`.
/// `root_wins` lets the root unit (path `.`) claim files no member owns —
/// granted only for a path spelled with real `/` separators.
fn directory_owner(model: &Model, slashed: &str, root_wins: bool) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    let mut root: Option<&str> = None;
    for unit in &model.units {
        let dir = unit.path.trim_end_matches('/');
        if dir == "." {
            root = Some(unit.name.as_str());
            continue;
        }
        if (slashed == dir || slashed.starts_with(&format!("{dir}/")))
            && best.is_none_or(|(len, _)| dir.len() > len)
        {
            best = Some((dir.len(), unit.name.as_str()));
        }
    }
    best.map(|(_, name)| name.to_string())
        .or_else(|| root.filter(|_| root_wins).map(String::from))
}

/// The unit whose name (or path) is the dotted prefix of `path`, longest
/// match first. Both spellings normalize `::` and `/` to `.` — the same
/// separator tolerance `parent_matches` applies to identity lookups.
fn name_prefix_owner(model: &Model, path: &str) -> Option<String> {
    let key = normalize_dotted(path);
    let mut best: Option<(usize, &str)> = None;
    for unit in &model.units {
        for candidate in [unit.name.as_str(), unit.path.as_str()] {
            let candidate = normalize_dotted(candidate);
            if candidate.is_empty() || candidate == "." {
                continue;
            }
            if (key == candidate || key.starts_with(&format!("{candidate}.")))
                && best.is_none_or(|(len, _)| candidate.len() > len)
            {
                best = Some((candidate.len(), unit.name.as_str()));
            }
        }
    }
    best.map(|(_, name)| name.to_string())
}

fn normalize_dotted(path: &str) -> String {
    path.replace("::", ".").replace('/', ".")
}

/// The unit-tier projection (`--aggregate units`): one node per model unit;
/// unit-tier dependency edges (`edges`) appear directly, and module-tier
/// edges (`module_edges`) appear as the edge between the two endpoint units
/// under [`unit_of_path`] — an edge whose endpoints attribute to one unit
/// (or to no unit) is tier-internal or unnamed and emits nothing. This is
/// the coarser honest picture the degraded-render note points at. Roles
/// fold through the same attribution: a key landing on a unit the graph
/// renders carries its role, a key outside it names no node and states
/// nothing, exactly as in the module views.
pub fn units_graph(model: &Model) -> ModuleGraph {
    let mut graph = ModuleGraph::default();
    for unit in &model.units {
        graph.add_node(&unit.name);
    }
    for edge in &model.edges {
        if edge.from == edge.to {
            continue;
        }
        graph.add_node(&edge.from);
        graph.add_node(&edge.to);
        graph.edges.insert((edge.from.clone(), edge.to.clone()));
    }
    for edge in &model.module_edges {
        let (Some(from), Some(to)) = (
            unit_of_path(model, &edge.from),
            unit_of_path(model, &edge.to),
        ) else {
            continue;
        };
        if from == to {
            continue;
        }
        graph.add_node(&from);
        graph.add_node(&to);
        graph.edges.insert((from, to));
    }
    let markers = fold_markers(model, |path| unit_of_path(model, path), &graph.nodes);
    graph.markers = markers
        .0
        .into_iter()
        .filter(|(name, _)| graph.nodes.contains(name))
        .collect();
    graph.unplaced_claims = markers.1;
    graph
}

/// The granularity every depgraph view projects through: `top_node`, or
/// `deep_node` when the top-level projection trivially folds — collapses the
/// whole tier onto one node while the tier holds more distinct paths below it
/// (the rule [`modules_graph`] renders by, kept here as one decision so the
/// modules graph and the api-usage grouping cannot read different
/// granularities). Structural, not language-keyed, exactly as before.
/// The modules GRAPH carries one further step this function cannot express as
/// a node fold: when every fold here still projects one node and the units
/// tier renders more, [`modules_graph`] drops to [`units_graph`] outright
/// (GitHub issue #20); the api-usage grouping keeps this granularity, and its
/// emptiness statement [`api_usage_empty_reason`] already names the fold
/// placement that collapse leaves behind.
fn projection_node(model: &Model) -> fn(&str) -> String {
    let top = project(model, top_node);
    if top.nodes.len() > 1 {
        return top_node;
    }
    let deeper = project(model, deep_node);
    if deeper.nodes.len() > top.nodes.len() {
        return deep_node;
    }
    // Neither prefix fold separates the endpoints: the model states no
    // containment under a shared prefix at all, and its module edges name
    // their endpoints directly (the kotlin driver's module tier is its gradle
    // units, so an endpoint is the whole identity, not a path below a unit).
    // The fold that keeps distinct endpoints distinct is the identity — under
    // either prefix fold the endpoints collide into `root` and the graph
    // disappears. Structural, not language-keyed, exactly as the two trials
    // above; a model whose paths carry a separating prefix never reaches it.
    if model.soft_structure.is_empty() {
        let direct = project(model, |path| path.to_string());
        if direct.nodes.len() > top.nodes.len() {
            return |path| path.to_string();
        }
    }
    top_node
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
///
/// The second return value states the fold's DROP (roles reality US 05): how
/// many role claims landed on a RENDERED node whose markers were silenced by
/// disagreement between different roles. Silence about such a drop would
/// present a lost fact as an absent one, so the renderers turn this count
/// into a conditional note; claims without roles, agreeing folds, and folds
/// onto nodes the view never draws are no drop and never count — the zero
/// keeps the view's bytes exact.
fn fold_markers(
    model: &Model,
    node: impl Fn(&str) -> Option<String>,
    rendered: &BTreeSet<String>,
) -> (BTreeMap<String, Role>, usize) {
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
    // The conflict count the renderers state: every claim that landed on a
    // node silenced by disagreement is a fact this fold could not place
    // (roles reality US 05) — silence about the DROP is what the note ends.
    let mut landed: BTreeMap<String, Vec<Role>> = BTreeMap::new();
    for (key, role) in &model.roles {
        if let Some(name) = node(&key.replace('/', "::")) {
            landed.entry(name).or_default().push(*role);
        }
    }
    let unplaced = landed
        .iter()
        .filter(|(name, roles)| {
            let distinct: BTreeSet<&'static str> = roles.iter().map(|role| role.as_str()).collect();
            rendered.contains(*name)
                && distinct.len() > 1
                && claimed.get(*name).is_none_or(|slot| slot.is_none())
        })
        .map(|(_, roles)| roles.len())
        .sum();
    (
        claimed
            .into_iter()
            .filter_map(|(name, role)| role.map(|role| (name, role)))
            .collect(),
        unplaced,
    )
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
    let markers = fold_markers(model, |path| child_node(path, parent), &graph.nodes);
    graph.markers = markers
        .0
        .into_iter()
        .filter(|(name, _)| graph.nodes.contains(name))
        .collect();
    graph.unplaced_claims = markers.1;
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

/// The stated-fold note (roles reality US 05): when a fold silences a node
/// because DIFFERENT roles landed on it, the graph states how many role
/// facts it could not place instead of dropping them in silence. Conditional
/// by design — a fold that places every claim emits nothing extra and keeps
/// its exact bytes (the zero-theater goldens prove it). The sentence shares
/// the `note: ` prefix and the `help roles` pointer of [`ROLE_LESS_NOTE`].
pub fn fold_note(count: usize) -> String {
    format!("note: {count} role facts folded onto one node with different roles, so the node states nothing — explained in 'archspec help roles'")
}

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

// ---------------------------------------------------------------------------
// Issue #21 — the inversion-debt ledger.
//
// The ledger is a VIEW over facts the model already carries, never a new
// adjudication: the same file/module-tier import evidence `verify`'s
// `used but undeclared` finding reports (issue #19, re-derived here at unit
// territory without a spec), the same per-unit manifest declarations
// (`unit_manifests`), and the same unit-tier edges (`edges`). The pair
// vocabulary is unit names — the tier the manifest declares legs at.

/// Section header for the class of pairs the code imports but the importer's
/// manifest omits. The bytes are the `used but undeclared` finding category
/// label (issue #19), reused unchanged: one word, one meaning.
pub const LEDGER_SECTION_UNDECLARED: &str = "## used but undeclared";
/// Section header for the class of pairs the manifest declares while the
/// model holds no code evidence for them.
pub const LEDGER_SECTION_UNUSED: &str = "## declared but unused";
/// The statement the ledger prints when both sections are empty. A clean
/// ledger says so by name — never a silent nothing (false-zero contract).
pub const LEDGER_CLEAN_STATEMENT: &str =
    "Ledger clean: no used-but-undeclared and no declared-but-unused pairs.";
/// The statement the ledger prints when the model carries no per-unit
/// manifest facts at all: without a declaration tier NEITHER class can be
/// answered — the absence is stated, not laundered into a clean verdict.
pub const LEDGER_NO_MANIFEST_FACTS_STATEMENT: &str =
    "The ledger cannot answer: the model carries no per-unit manifest facts.";
/// The placement statement for a model whose driver records the manifest
/// declaration and the code import of a unit pair in the SAME unit-edge tier
/// (the rust driver's law: every `[dependencies]` leg naming a member joins
/// `edges` beside the code-derived legs). A declared pair in such a model
/// carries an edge whatever its code says, so its code status is not
/// separable there; the section states the placement instead of printing a
/// census the model cannot support.
pub const LEDGER_UNIT_EDGE_FOLD_STATEMENT: &str =
    "The model records manifest declarations and code imports in one unit-edge tier, so a declared pair's code status is not separable in this model.";
/// The word printed under a section header when that class is empty while
/// the other class is not: every section of a census answers, an empty one
/// answers `none`.
const LEDGER_SECTION_EMPTY: &str = "none";

/// Dash/underscore crate-name equivalence (the same normalization the rust
/// scan applies when it resolves a dependency name to a member).
fn ledger_name_key(name: &str) -> String {
    name.replace('-', "_")
}

/// The declared legs of the model: pairs `unit -> target` where `unit`'s own
/// manifest declares a dependency naming workspace unit `target`. Dependency
/// names resolve to units exactly first (npm names are exact), then under
/// the dash/underscore crate-name law.
pub fn ledger_declared_pairs(model: &Model) -> BTreeSet<(String, String)> {
    let units_by_key: BTreeMap<String, String> = model
        .units
        .iter()
        .flat_map(|unit| {
            std::iter::once(unit.name.clone())
                .chain(unit.crate_ids.iter().cloned())
                .map(move |id| (ledger_name_key(&id), unit.name.clone()))
        })
        .collect();
    let mut declared = BTreeSet::new();
    for (unit, info) in &model.unit_manifests {
        if !model.units.iter().any(|known| &known.name == unit) {
            continue;
        }
        for dependency in &info.dependencies {
            let Some(target) = units_by_key
                .get(dependency)
                .or_else(|| units_by_key.get(&ledger_name_key(dependency)))
            else {
                continue;
            };
            if target != unit {
                declared.insert((unit.clone(), target.clone()));
            }
        }
    }
    declared
}

/// The unit a module-path or repo-file path belongs to: first by path
/// territory (a unit's directory owns its files, longest path wins — the
/// `containing_unit` rule `verify`'s file-tier evidence attributes with),
/// then by unit-name prefix (the dotted `unit::module` / `unit.module`
/// spelling the dotted-tier drivers record). A root unit's territory (path
/// `.` or empty) attributes nothing: it is the whole tree, so containment
/// against it would swallow foreign paths (the file-tier evidence law).
fn ledger_path_owner(model: &Model, path: &str) -> Option<String> {
    model
        .units
        .iter()
        .filter(|unit| {
            let territory = unit.path.trim_end_matches('/');
            !territory.is_empty()
                && territory != "."
                && (path == territory || path.starts_with(&format!("{territory}/")))
        })
        .max_by_key(|unit| unit.path.trim_end_matches('/').chars().count())
        .map(|unit| unit.name.clone())
        .or_else(|| {
            model
                .units
                .iter()
                .filter(|unit| {
                    unit.name == path
                        || path.starts_with(&format!("{}::", unit.name))
                        || path.starts_with(&format!("{}.", unit.name))
                })
                .max_by_key(|unit| unit.name.chars().count())
                .map(|unit| unit.name.clone())
        })
}

/// Code-evidenced unit pairs with their evidence locs, folded from the
/// model's code tiers — the same cross-unit import evidence issue #19's
/// `file_tier_evidence` reads, attributed at unit territory without a spec:
/// every non-self module edge whose endpoints attribute to different units
/// contributes the pair with the importing endpoint as its evidence loc.
/// Test-gated endpoints are excluded (the same one-case-one-verdict law).
/// A model whose driver ALSO records code-derived imports directly as unit
/// edges (the rust driver) contributes those legs too — an edge with no
/// declaration can only have come from the code arm of such a model.
fn ledger_code_evidence(model: &Model) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut evidence: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for edge in &model.module_edges {
        if model.is_test_gated(&edge.from) || model.is_test_gated(&edge.to) {
            continue;
        }
        let from = ledger_path_owner(model, &edge.from).or_else(|| {
            model
                .units
                .iter()
                .find(|unit| unit.name == edge.unit)
                .map(|unit| unit.name.clone())
        });
        let (Some(from), Some(to)) = (from, ledger_path_owner(model, &edge.to)) else {
            continue;
        };
        if from == to {
            continue;
        }
        evidence
            .entry((from, to))
            .or_default()
            .insert(edge.from.clone());
    }
    evidence
}

/// Unit-tier pairs the code evidences while no declaration covers them: in
/// a model whose unit edges are the union of both provenances (the rust
/// driver), a pair that carries an edge but no declaration can only have
/// arrived through the code arm. In a model whose unit edges carry only
/// declarations (every other driver), this set is empty by construction and
/// the module-tier evidence owns the class.
fn ledger_unit_tier_undeclared(model: &Model) -> BTreeMap<(String, String), BTreeSet<String>> {
    if model.language != "rust" {
        return BTreeMap::new();
    }
    let declared = ledger_declared_pairs(model);
    let mut rows: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for edge in &model.edges {
        if edge.from == edge.to || declared.contains(&(edge.from.clone(), edge.to.clone())) {
            continue;
        }
        if !model.units.iter().any(|unit| unit.name == edge.from)
            || !model.units.iter().any(|unit| unit.name == edge.to)
        {
            continue;
        }
        rows.entry((edge.from.clone(), edge.to.clone())).or_default();
    }
    rows
}

/// The ledger's first section: code evidence without a declaration, each
/// pair with its sorted distinct evidence locs (empty when the pair's only
/// evidence is the model's own unit-tier edge, which carries no loc).
pub fn ledger_used_but_undeclared(model: &Model) -> BTreeMap<(String, String), BTreeSet<String>> {
    let declared = ledger_declared_pairs(model);
    let mut rows = ledger_code_evidence(model);
    for (pair, locs) in ledger_unit_tier_undeclared(model) {
        rows.entry(pair).or_default().extend(locs);
    }
    rows.retain(|pair, _| !declared.contains(pair));
    rows
}

/// The ledger's second section: declarations with no code evidence. A model
/// that separates the tiers answers by name; a model whose driver folds the
/// declaration leg and the code leg into one unit-edge tier cannot support a
/// census — the emptiness is a placement fact, so the renderer prints
/// [`LEDGER_UNIT_EDGE_FOLD_STATEMENT`] instead of these pairs (a declared
/// pair there carries an edge whatever its code says, and listing every
/// declared pair would call clean pairs debt).
pub fn ledger_declared_but_unused(model: &Model) -> BTreeSet<(String, String)> {
    if ledger_folds_unit_edges(model) {
        return BTreeSet::new();
    }
    let mut used: BTreeSet<(String, String)> =
        ledger_code_evidence(model).into_keys().collect();
    used.extend(ledger_unit_tier_undeclared(model).into_keys());
    ledger_declared_pairs(model)
        .into_iter()
        .filter(|pair| !used.contains(pair))
        .collect()
}

/// True when the model's driver records a declared unit leg as a unit edge
/// whatever its code says (the rust driver's union law: every manifest leg
/// naming a member joins `edges` beside the code-derived legs). Other
/// drivers record declared legs at the unit tier and code legs at the module
/// tier, so a pair's two statuses stay separable there.
fn ledger_folds_unit_edges(model: &Model) -> bool {
    model.language == "rust"
}

/// One ledger row: `from -> to`, with the sorted evidence locs in
/// parentheses when the model carries them — the same row shape `verify`'s
/// `used but undeclared` finding prints (issue #19).
fn ledger_row(from: &str, to: &str, locs: &BTreeSet<String>) -> String {
    if locs.is_empty() {
        format!("{from} -> {to}")
    } else {
        format!(
            "{from} -> {to} ({})",
            locs.iter().cloned().collect::<Vec<_>>().join(", ")
        )
    }
}

/// The inversion-debt ledger body: the two sections over the model's own
/// facts. Both empty states cleanliness by name; a model without a
/// declaration tier states that it cannot answer; a declared-but-unused
/// census on a fold model prints the placement statement instead.
pub fn ledger_markdown(model: &Model) -> String {
    if model.unit_manifests.is_empty() {
        return format!("{LEDGER_NO_MANIFEST_FACTS_STATEMENT}\n");
    }
    let used_undeclared = ledger_used_but_undeclared(model);
    let declared_unused = ledger_declared_but_unused(model);
    let fold_statement =
        ledger_folds_unit_edges(model) && !ledger_declared_pairs(model).is_empty();
    if used_undeclared.is_empty() && declared_unused.is_empty() && !fold_statement {
        return format!("{LEDGER_CLEAN_STATEMENT}\n");
    }
    let mut out = String::new();
    let _ = writeln!(out, "{LEDGER_SECTION_UNDECLARED}\n");
    if used_undeclared.is_empty() {
        let _ = writeln!(out, "{LEDGER_SECTION_EMPTY}");
    } else {
        for ((from, to), locs) in &used_undeclared {
            let _ = writeln!(out, "{}", ledger_row(from, to, locs));
        }
    }
    out.push('\n');
    let _ = writeln!(out, "{LEDGER_SECTION_UNUSED}\n");
    if fold_statement {
        let _ = writeln!(out, "{LEDGER_UNIT_EDGE_FOLD_STATEMENT}");
    } else if declared_unused.is_empty() {
        let _ = writeln!(out, "{LEDGER_SECTION_EMPTY}");
    } else {
        for (from, to) in &declared_unused {
            let _ = writeln!(out, "{from} -> {to}");
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
                dynamic_import_wildcards: Default::default(),
            schema_version: 1,
            report_only_lines: Vec::new(),
            warnings: Vec::new(),
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
                    type_only: false,
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
            module_public_types: Default::default(),
            module_external: Default::default(),
            root_module_declarations: Default::default(),
            unit_manifests: Default::default(),
            unresolved_module_files: Default::default(),
            test_gated_modules: Default::default(),
            file_edges: Vec::new(),
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

    /// The issue #20 shape: a monorepo whose module tier addresses its packages
    /// by BARE unit names (the csharp scan spells a single-segment root
    /// namespace this way) with per-package claims at those same bare paths.
    /// Every endpoint is one segment: the top and deeper folds collapse the
    /// whole graph onto `root`, unit edges project to suppressed self-edges,
    /// and the identity arm is gated by the non-empty soft tier — the fold
    /// that used to render `graph TD` + one `root` with zero edges.
    fn issue20_monorepo_model() -> Model {
        let mut model = Model::new("csharp");
        model.units = ["Alpha", "Beta", "Gamma"]
            .into_iter()
            .map(|name| Unit {
                name: name.to_string(),
                kind: "project".to_string(),
                path: name.to_string(),
                root: format!("{name}.csproj"),
                crate_ids: BTreeSet::from([name.to_string()]),
            })
            .collect();
        model.edges = [
            ("Beta", "Alpha"),
            ("Gamma", "Alpha"),
            ("Gamma", "Beta"),
        ]
        .into_iter()
        .map(|(from, to)| Edge {
            from: from.to_string(),
            to: to.to_string(),
        })
        .collect();
        model.soft_structure = ["Alpha", "Beta", "Gamma"]
            .into_iter()
            .map(|name| (name.to_string(), vec![name.to_string()]))
            .collect();
        model.module_edges = [
            ("Beta", "Alpha", &["Token"][..]),
            ("Gamma", "Alpha", &["Token"]),
            ("Gamma", "Beta", &["Channel"]),
        ]
        .into_iter()
        .map(|(from, to, symbols)| ModuleEdge {
            type_only: false,
            unit: from.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            symbols: symbols.iter().map(|s| s.to_string()).collect(),
        })
        .collect();
        // Per-package claims: every package claims facade at its own root path.
        model.roles = ["Alpha", "Beta", "Gamma"]
            .into_iter()
            .map(|name| (name.to_string(), Role::Facade))
            .collect();
        model
    }

    /// Precondition for the tests below: on this model every module-tier fold
    /// projects one node — the collapse the drop replaces.
    #[test]
    fn issue20_model_folds_to_one_node_before_the_drop() {
        let model = issue20_monorepo_model();
        let folded = project_marked(&model, projection_node(&model));
        assert_eq!(
            folded.nodes.iter().cloned().collect::<Vec<_>>(),
            vec![ROOT_NODE.to_string()],
            "the repro must be a one-node fold before the drop"
        );
        assert!(folded.edges.is_empty(), "a one-node fold carries no edges");
    }

    /// The fix: the collapsed fold drops to the units tier — the render equals
    /// what `--aggregate units` draws, so no single-node render escapes on a
    /// model whose units tier shows more.
    #[test]
    fn issue20_collapsed_fold_drops_to_the_units_tier() {
        let model = issue20_monorepo_model();
        let graph = modules_graph(&model);
        let units = units_graph(&model);
        assert_eq!(graph.nodes, units.nodes, "the dropped render names the units tier");
        assert_eq!(graph.edges, units.edges, "the dropped render draws the unit edges");
        assert_eq!(graph.nodes.len(), 3, "{:?}", graph.nodes);
        assert_eq!(graph.edges.len(), 3, "{:?}", graph.edges);
        assert!(!graph.nodes.contains(ROOT_NODE), "the lone `root` was the bug");
        // The per-package claims survive the drop: each claim lands on its own
        // unit node through the units fold's attribution.
        assert_eq!(graph.markers.len(), 3, "{:?}", graph.markers);
        for name in ["Alpha", "Beta", "Gamma"] {
            assert_eq!(graph.markers.get(name), Some(&Role::Facade), "{name} keeps its claim");
        }
        assert_eq!(graph.unplaced_claims, 0, "the units fold places every claim");
    }

    /// The residual case: one unit, several module paths, no fold separates
    /// them, and no units tier to drop to — the render keeps its one node and
    /// states the collapse loudly (same mechanism as the degraded-render note).
    #[test]
    fn issue20_residual_single_node_render_names_its_collapse() {
        let model = model_with(
            "app",
            vec!["app::api", "app::api::api"],
            vec![("app::api", "app::api::api", &[] as &[&str])],
        );
        let graph = modules_graph(&model);
        assert_eq!(graph.nodes.iter().collect::<Vec<_>>().len(), 1, "{:?}", graph.nodes);
        let note = automatic_projection_note(&model, &graph)
            .expect("a one-node render of a multi-path tier must name its collapse");
        assert!(note.starts_with("note: the automatic projection folded 2 module paths"), "{note}");
        assert!(note.contains("--aggregate units"), "{note}");
    }

    /// The escape law: NO single-node render of a module tier that names more
    /// than one path leaves the automatic path without the loud note, and no
    /// healthy render gains a note or changes bytes.
    #[test]
    fn issue20_no_single_node_render_escapes_without_the_note() {
        let sweep: Vec<Model> = vec![
            issue20_monorepo_model(),
            model_with("app", vec!["app"], vec![("app", "app", &[] as &[&str])]),
            model_with(
                "app",
                vec!["app::api", "app::api::api"],
                vec![("app::api", "app::api::api", &[])],
            ),
            model_with(
                "example.com::demo",
                Vec::new(),
                vec![
                    ("example.com::demo::app", "example.com::demo::shared", &[]),
                    ("example.com::demo::shared", "example.com::demo::store", &[]),
                ],
            ),
            model_with(
                "example.com::demo",
                Vec::new(),
                vec![("example.com::demo::app", "example.com::demo::app", &[])],
            ),
            model_with(
                "app",
                vec!["app::core", "app::ui"],
                vec![("app::core", "app::ui", &[])],
            ),
        ];
        for model in sweep {
            let graph = modules_graph(&model);
            let paths = distinct_module_paths(&model);
            if graph.nodes.len() <= 1 && paths > 1 {
                assert!(
                    automatic_projection_note(&model, &graph).is_some(),
                    "single-node render of a {paths}-path tier escaped without a note: {:?}",
                    graph.nodes
                );
            }
            if graph.nodes.len() > 1 {
                assert!(
                    automatic_projection_note(&model, &graph).is_none(),
                    "a multi-node render must stay silent: {:?}",
                    graph.nodes
                );
            }
        }
        // The faithful single node (one path under the fold) keeps its silence.
        let faithful = model_with(
            "example.com::demo",
            Vec::new(),
            vec![("example.com::demo::app", "example.com::demo::app", &[] as &[&str])],
        );
        assert!(automatic_projection_note(&faithful, &modules_graph(&faithful)).is_none());
    }

    /// The cap note keeps its exact bytes on the file-grained render, and the
    /// collapse note never double-fires with it.
    #[test]
    fn issue20_cap_note_bytes_are_unchanged() {
        let mut model = Model::new("ts");
        model.units = vec![Unit {
            name: "root".to_string(),
            kind: "package".to_string(),
            path: ".".to_string(),
            root: String::new(),
            crate_ids: BTreeSet::from(["root".to_string()]),
        }];
        model.module_edges = (0..=AUTOMATIC_PROJECTION_NODE_CAP)
            .map(|index| ModuleEdge {
                type_only: false,
                unit: "root".to_string(),
                from: format!("packages/a/f{index}.ts"),
                to: format!("packages/b/m{index}.ts"),
                symbols: Vec::new(),
            })
            .collect();
        let graph = modules_graph(&model);
        assert!(graph.nodes.len() > AUTOMATIC_PROJECTION_NODE_CAP, "{:?}", graph.nodes.len());
        let note = automatic_projection_note(&model, &graph).expect("cap crossing notes");
        assert_eq!(note, degraded_projection_note(graph.nodes.len()), "cap note bytes");
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
                dynamic_import_wildcards: Default::default(),
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
                dynamic_import_wildcards: Default::default(),
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
                dynamic_import_wildcards: Default::default(),
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
                dynamic_import_wildcards: Default::default(),
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
        assert_eq!(graph.unplaced_claims, 0, "a placed fold states no note");
    }

    /// The go spelling: the key is an import path (`/` separators), the
    /// projection lifts it to the model path grammar, and the trivial fold
    /// reads the paths below it — so the key marks the package's own node.
    #[test]
    fn modules_graph_marks_go_composition_key_in_native_separator_spelling() {
        let base = model_with("example.com::demo", Vec::new(), Vec::new());
        let mut model = Model {
                dynamic_import_wildcards: Default::default(),
            language: "go".to_string(),
            module_edges: vec![
                ModuleEdge {
                    type_only: false,
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
                dynamic_import_wildcards: Default::default(),
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
        assert_eq!(
            graph.unplaced_claims, 2,
            "the silenced node's two role facts are the stated drop: {:?}",
            graph.markers
        );
        // Contrast: the same two paths agreeing on one role mark their node.
        let model = Model {
                dynamic_import_wildcards: Default::default(),
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
        assert_eq!(
            graph.unplaced_claims, 0,
            "an agreeing fold places every claim and states no note"
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
        assert_eq!(
            graph.unplaced_claims, 0,
            "a key folded to an unrendered node states nothing — the view's \
             bytes stay exact (rust/go zero-theater criterion)"
        );
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
        assert_eq!(
            graph.unplaced_claims, 2,
            "the submodules view counts the same stated drop as modules"
        );
    }

    // --- forced aggregation levels + the degraded-projection note (issue 5) --

    fn unit_at(name: &str, path: &str) -> Unit {
        Unit {
            name: name.to_string(),
            kind: "package".to_string(),
            path: path.to_string(),
            root: String::new(),
            crate_ids: BTreeSet::new(),
        }
    }

    /// A ts-workspace-shaped model: units carry directory paths, the module
    /// tier IS file paths (the capability matrix's file-grained row), and the
    /// model carries no soft tier at all — the exact shape whose automatic
    /// projection renders one node per file.
    fn ts_shape_model() -> Model {
        let base = model_with("mono", Vec::new(), Vec::new());
        Model {
            dynamic_import_wildcards: Default::default(),
            language: "ts".to_string(),
            soft_structure: Default::default(),
            units: vec![
                unit_at("mono", "."),
                unit_at("@mono/a", "packages/a"),
                unit_at("@mono/b", "packages/b"),
            ],
            module_edges: vec![ModuleEdge {
                unit: "@mono/a".to_string(),
                from: "packages/a/src/index.ts".to_string(),
                to: "packages/b/src/index.ts".to_string(),
                symbols: Default::default(),
                type_only: false,
            }],
            ..base
        }
    }

    /// The mechanism pin (issue 5's verified code truth): on a file-grained
    /// module tier the automatic rule reaches the identity arm — the fold
    /// trials fold every file path onto `root`, so the identity projection IS
    /// the file graph. This is what the degraded note guards and `--aggregate`
    /// bypasses.
    #[test]
    fn identity_fold_renders_one_node_per_file_on_a_file_grained_tier() {
        let graph = modules_graph(&ts_shape_model());
        assert_eq!(
            graph.nodes.iter().cloned().collect::<Vec<_>>(),
            vec![
                "packages/a/src/index.ts".to_string(),
                "packages/b/src/index.ts".to_string()
            ],
            "the automatic rule must land on the file identities"
        );
        assert!(graph
            .edges
            .contains(&("packages/a/src/index.ts".to_string(), "packages/b/src/index.ts".to_string())));
    }

    #[test]
    fn unit_of_path_attributes_every_driver_spelling() {
        // Dotted module paths: the unit name is the dotted prefix.
        let rust = model_with("app", vec!["app::core"], Vec::new());
        assert_eq!(unit_of_path(&rust, "app::core").as_deref(), Some("app"));
        assert_eq!(unit_of_path(&rust, "app").as_deref(), Some("app"));
        let go = model_with("example.com::demo", Vec::new(), Vec::new());
        assert_eq!(
            unit_of_path(&go, "example.com::demo::app").as_deref(),
            Some("example.com::demo")
        );
        let mut csharp = csharp_like_model();
        csharp.units = vec![unit("Shop.Api"), unit("Shop.Application"), unit("Shop.Domain")];
        assert_eq!(
            unit_of_path(&csharp, "Shop::Api::Controllers").as_deref(),
            Some("Shop.Api"),
            "the namespace prefix attributes to its project"
        );
        // File paths: the longest owning directory; the root unit owns what
        // no member directory claims; the `::`-lifted spelling resolves too.
        let ts = ts_shape_model();
        assert_eq!(
            unit_of_path(&ts, "packages/a/src/index.ts").as_deref(),
            Some("@mono/a")
        );
        assert_eq!(
            unit_of_path(&ts, "packages::a::src::index.ts").as_deref(),
            Some("@mono/a")
        );
        assert_eq!(unit_of_path(&ts, "src/x.ts").as_deref(), Some("mono"));
        // No candidate, no claim: an unattributable dotted path invents no owner.
        assert_eq!(unit_of_path(&ts, "ghost::thing"), None);
    }

    #[test]
    fn units_graph_projects_file_tier_edges_onto_their_units() {
        let graph = units_graph(&ts_shape_model());
        assert_eq!(
            graph.nodes.iter().cloned().collect::<Vec<_>>(),
            vec![
                "@mono/a".to_string(),
                "@mono/b".to_string(),
                "mono".to_string()
            ],
            "the unit tier is one node per model unit"
        );
        assert_eq!(
            graph.edges.iter().cloned().collect::<Vec<_>>(),
            vec![(
                "@mono/a".to_string(),
                "@mono/b".to_string()
            )],
            "a cross-package import edge lands on the two owning units"
        );
    }

    #[test]
    fn units_graph_keeps_edges_inside_one_unit_tier_internal() {
        let base = ts_shape_model();
        let model = Model {
            module_edges: vec![
                ModuleEdge {
                    unit: "@mono/a".to_string(),
                    from: "packages/a/src/index.ts".to_string(),
                    to: "packages/a/src/util.ts".to_string(),
                    symbols: Default::default(),
                    type_only: false,
                },
                ModuleEdge {
                    unit: "@mono/a".to_string(),
                    from: "packages/a/src/index.ts".to_string(),
                    to: "packages/b/src/index.ts".to_string(),
                    symbols: Default::default(),
                    type_only: false,
                },
            ],
            ..base
        };
        let graph = units_graph(&model);
        assert_eq!(
            graph.edges.len(),
            1,
            "a same-unit file edge must not surface as a unit edge: {:?}",
            graph.edges
        );
    }

    #[test]
    fn units_graph_adds_unit_edges_from_the_unit_tier_directly() {
        let base = model_with("app", vec!["app::core"], Vec::new());
        let mut model = Model {
            units: vec![unit_at("app", "."), unit_at("web", "crates/web")],
            edges: vec![Edge {
                from: "web".to_string(),
                to: "app".to_string(),
            }],
            ..base
        };
        model.module_edges.clear();
        let graph = units_graph(&model);
        assert!(graph
            .edges
            .contains(&("web".to_string(), "app".to_string())));
    }

    #[test]
    fn top_modules_graph_pins_the_top_projection_past_the_fold() {
        // The go-fold shape: the automatic rule descends to the packages
        // below the fold; the forced top level renders the fold itself.
        let model = model_with(
            "example.com::demo",
            Vec::new(),
            vec![("example.com::demo::app", "example.com::demo::shared", &[] as &[&str])],
        );
        let automatic = modules_graph(&model);
        assert_eq!(automatic.nodes.len(), 2, "the automatic rule descends");
        let pinned = top_modules_graph(&model);
        assert_eq!(
            pinned.nodes.iter().cloned().collect::<Vec<_>>(),
            vec!["demo".to_string()],
            "the pinned top level never descends"
        );
        assert!(pinned.edges.is_empty(), "a one-node projection emits no edge");
    }

    #[test]
    fn top_modules_graph_matches_the_automatic_bytes_on_a_healthy_tree() {
        let model = model_with(
            "app",
            vec!["app::core", "app::ui"],
            vec![("app::core", "app::ui", &[] as &[&str])],
        );
        let auto = render_mermaid(&modules_graph(&model));
        let pinned = render_mermaid(&top_modules_graph(&model));
        assert_eq!(auto, pinned, "a multi-node projection has nothing to bypass");
    }

    /// GitHub issue 15: the `--aggregate modules` arm of a file-grained tier
    /// projects the file endpoints onto their per-unit module roots instead of
    /// collapsing every endpoint onto `root`. The ts-shape model's two member
    /// units own the two files, so the two module roots are the two units.
    #[test]
    fn top_modules_graph_projects_the_file_tier_onto_the_module_roots() {
        let graph = top_modules_graph(&ts_shape_model());
        assert_eq!(
            graph.nodes.iter().cloned().collect::<Vec<_>>(),
            vec!["@mono/a".to_string(), "@mono/b".to_string()],
            "the module roots of a member-owned file tier are the owning units"
        );
        assert_eq!(
            graph.edges.iter().cloned().collect::<Vec<_>>(),
            vec![(
                "@mono/a".to_string(),
                "@mono/b".to_string()
            )],
            "the cross-package file edge survives the fold once, merged"
        );
    }

    /// Files the root unit owns fold to their first path segment — the root
    /// package's module root — mirroring how `top_node` takes the segment
    /// after the unit in the dotted spelling. A file directly under the root
    /// has no segment below it and folds to `root`, exactly as a unit-root
    /// dotted path folds.
    #[test]
    fn top_modules_graph_folds_root_owned_files_onto_their_first_segment() {
        let base = ts_shape_model();
        let model = Model {
            module_edges: vec![
                ModuleEdge {
                    unit: "mono".to_string(),
                    from: "src/app.ts".to_string(),
                    to: "packages/b/src/index.ts".to_string(),
                    symbols: Default::default(),
                    type_only: false,
                },
                ModuleEdge {
                    unit: "mono".to_string(),
                    from: "index.ts".to_string(),
                    to: "src/app.ts".to_string(),
                    symbols: Default::default(),
                    type_only: false,
                },
            ],
            ..base
        };
        let graph = top_modules_graph(&model);
        assert_eq!(
            graph.nodes.iter().cloned().collect::<Vec<_>>(),
            vec![
                "@mono/b".to_string(),
                "root".to_string(),
                "src".to_string()
            ],
            "`src/app.ts` folds to its first segment, `index.ts` folds to root"
        );
        assert_eq!(
            graph.edges.iter().cloned().collect::<Vec<_>>(),
            vec![
                (
                    "root".to_string(),
                    "src".to_string()
                ),
                (
                    "src".to_string(),
                    "@mono/b".to_string()
                )
            ],
            "the folds re-emit the edges at module-root granularity"
        );
    }

    /// Mirror of `units_graph_keeps_edges_inside_one_unit_tier_internal`: two
    /// files of one member fold onto the same module root, so their edge is a
    /// self-edge and vanishes — multiplicity never survives the merge.
    #[test]
    fn top_modules_graph_merges_file_edges_inside_one_unit() {
        let base = ts_shape_model();
        let model = Model {
            module_edges: vec![
                ModuleEdge {
                    unit: "@mono/a".to_string(),
                    from: "packages/a/src/index.ts".to_string(),
                    to: "packages/a/src/util.ts".to_string(),
                    symbols: Default::default(),
                    type_only: false,
                },
                ModuleEdge {
                    unit: "@mono/a".to_string(),
                    from: "packages/a/src/index.ts".to_string(),
                    to: "packages/b/src/index.ts".to_string(),
                    symbols: Default::default(),
                    type_only: false,
                },
            ],
            ..base
        };
        let graph = top_modules_graph(&model);
        assert_eq!(
            graph.edges.len(),
            1,
            "a same-unit file edge folds to a self-edge and emits nothing: {:?}",
            graph.edges
        );
    }

    /// Unit (dependency) edges project through the fold of their endpoint
    /// units' module roots exactly as `project_unit_edges` draws them on the
    /// dotted tiers: the cross product, self-pairs suppressed, a unit with no
    /// module-tier facts contributing nothing.
    #[test]
    fn top_modules_graph_projects_unit_edges_through_the_module_roots() {
        let base = ts_shape_model();
        let mut model = Model {
            edges: vec![Edge {
                from: "@mono/b".to_string(),
                to: "@mono/a".to_string(),
            }],
            ..base
        };
        model.edges.push(Edge {
            from: "@mono/a".to_string(),
            to: "@mono/a".to_string(),
        });
        let graph = top_modules_graph(&model);
        assert!(
            graph
                .edges
                .contains(&("@mono/b".to_string(), "@mono/a".to_string())),
            "a declared cross-unit dependency folds onto the two module roots: {:?}",
            graph.edges
        );
        assert_eq!(
            graph.edges.len(),
            2,
            "the module edge plus the projected unit edge, the unit self-edge gone: {:?}",
            graph.edges
        );
    }

    /// The dispatch pin: `top_modules_graph` on a file-grained tier projects
    /// instead of collapsing — the issue's one-node `root` graph is gone —
    /// while the dotted tiers keep the pre-fix fold byte-for-byte (the
    /// go-fold pin above stays green).
    #[test]
    fn top_modules_graph_projects_a_file_tier_instead_of_collapsing_it() {
        let graph = top_modules_graph(&ts_shape_model());
        assert_eq!(
            graph.nodes.len(),
            2,
            "a file tier with two owning units never collapses to one node: {:?}",
            graph.nodes
        );
        assert!(
            !graph.nodes.contains("root"),
            "the collapse to `root` was the bug: {:?}",
            graph.nodes
        );
    }

    #[test]
    fn degraded_note_names_the_count_and_the_flag() {
        let note = degraded_projection_note(68563);
        assert!(note.contains("68563"), "{note}");
        assert!(note.contains("--aggregate units"), "{note}");
        assert!(note.starts_with("note: "), "{note}");
    }

    // --- issue #21: the inversion-debt ledger -----------------------------

    /// A unit-tier fixture model: units by (name, path territory), declared
    /// manifest legs per unit, and module-tier edges as (unit, from, to).
    fn ledger_model(
        language: &str,
        units: &[(&str, &str)],
        declared: &[(&str, &[&str])],
        edges: &[(&str, &str, &str)],
    ) -> Model {
        let mut model = Model::new(language);
        model.units = units
            .iter()
            .map(|(name, path)| Unit {
                name: (*name).to_string(),
                kind: "package".to_string(),
                path: (*path).to_string(),
                root: String::new(),
                crate_ids: BTreeSet::from([(*name).to_string()]),
            })
            .collect();
        model.unit_manifests = units
            .iter()
            .map(|(name, _)| {
                let dependencies = declared
                    .iter()
                    .find(|(unit, _)| unit == name)
                    .map(|(_, deps)| deps.iter().map(|dep| (*dep).to_string()).collect())
                    .unwrap_or_default();
                (
                    (*name).to_string(),
                    crate::archspec::model::ManifestInfo {
                        publish: None,
                        dependencies,
                        features: Vec::new(),
                    },
                )
            })
            .collect();
        model.module_edges = edges
            .iter()
            .map(|(unit, from, to)| crate::archspec::model::ModuleEdge {
                unit: (*unit).to_string(),
                from: (*from).to_string(),
                to: (*to).to_string(),
                symbols: Vec::new(),
                type_only: false,
            })
            .collect();
        model
    }

    /// The both-classes fixture: `beta` imports a file of `alpha` while its
    /// manifest omits the declaration (used but undeclared), and `root`
    /// declares `beta` with no import evidence (declared but unused).
    fn issue21_debt_model() -> Model {
        ledger_model(
            "ts",
            &[
                ("root", "."),
                ("alpha", "packages/alpha"),
                ("beta", "packages/beta"),
            ],
            &[("root", &["beta"]), ("alpha", &[]), ("beta", &[])],
            &[(
                "beta",
                "packages/beta/src/index.ts",
                "packages/alpha/src/index.ts",
            )],
        )
    }

    #[test]
    fn issue21_ledger_names_both_classes_in_their_sections() {
        let body = ledger_markdown(&issue21_debt_model());
        let used_at = body
            .find(LEDGER_SECTION_UNDECLARED)
            .expect("the used-but-undeclared section must print");
        let unused_at = body
            .find(LEDGER_SECTION_UNUSED)
            .expect("the declared-but-unused section must print");
        let debt_at = body
            .find("beta -> alpha (packages/beta/src/index.ts)")
            .expect("the undeclared import must print with its evidence loc");
        assert!(used_at < debt_at && debt_at < unused_at, "{body}");
        let unused_pair_at = body
            .find("root -> beta")
            .expect("the unused declaration must print as a pair");
        assert!(unused_at < unused_pair_at, "{body}");
    }

    #[test]
    fn issue21_clean_ledger_states_cleanliness_by_name() {
        // The same workspace reconciled: beta both imports and declares the
        // alpha leg — neither class has a row, and the body says so by name.
        let model = ledger_model(
            "ts",
            &[("alpha", "packages/alpha"), ("beta", "packages/beta")],
            &[("alpha", &[]), ("beta", &["alpha"])],
            &[(
                "beta",
                "packages/beta/src/index.ts",
                "packages/alpha/src/index.ts",
            )],
        );
        let body = ledger_markdown(&model);
        assert_eq!(body, format!("{LEDGER_CLEAN_STATEMENT}\n"));
    }

    /// The rust driver's unit-edge fold (its manifest legs join `edges`
    /// whatever the code says): the used-but-undeclared census still answers
    /// (an edge with no declaration can only come from the code arm), while
    /// the declared section states the fold instead of printing a census the
    /// model cannot support.
    #[test]
    fn issue21_rust_ledger_answers_class_one_and_names_the_fold() {
        let mut model = ledger_model(
            "rust",
            &[("app", "app"), ("shared", "shared")],
            &[("app", &["shared"]), ("shared", &[])],
            &[],
        );
        model.edges = vec![
            crate::archspec::model::Edge {
                from: "app".to_string(),
                to: "shared".to_string(),
            },
            crate::archspec::model::Edge {
                from: "shared".to_string(),
                to: "app".to_string(),
            },
        ];
        let body = ledger_markdown(&model);
        let used_at = body.find(LEDGER_SECTION_UNDECLARED).expect(&body);
        let unused_at = body.find(LEDGER_SECTION_UNUSED).expect(&body);
        let debt_at = body
            .find("shared -> app")
            .expect("the code-only unit edge must print as used but undeclared");
        assert!(used_at < debt_at && debt_at < unused_at, "{body}");
        assert!(
            body.contains(LEDGER_UNIT_EDGE_FOLD_STATEMENT),
            "the fold model must state the fold, not a silent census: {body}"
        );
    }

    #[test]
    fn issue21_ledger_without_manifest_facts_names_its_refusal() {
        let mut model = ledger_model("ts", &[("pkg", ".")], &[], &[]);
        model.unit_manifests = BTreeMap::new();
        let body = ledger_markdown(&model);
        assert_eq!(body, format!("{LEDGER_NO_MANIFEST_FACTS_STATEMENT}\n"));
    }

    /// A pair whose only module-edge evidence is test-gated stays out of the
    /// census (the one-case-one-verdict law `verify`'s file-tier evidence
    /// applies), so a test-only import states no debt.
    #[test]
    fn issue21_test_gated_module_edge_states_no_debt() {
        let mut model = ledger_model(
            "ts",
            &[("alpha", "packages/alpha"), ("beta", "packages/beta")],
            &[("alpha", &[]), ("beta", &[])],
            &[(
                "beta",
                "packages/beta/src/tests.ts",
                "packages/alpha/src/index.ts",
            )],
        );
        model.test_gated_modules =
            BTreeSet::from(["packages/beta/src/tests.ts".to_string()]);
        let body = ledger_markdown(&model);
        assert_eq!(body, format!("{LEDGER_CLEAN_STATEMENT}\n"));
    }
}
