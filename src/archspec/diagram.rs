use crate::archspec::model::{Model, Role, Unit};
use rust_arch_test_kit::render::{
    mermaid_edge, mermaid_labeled_edge, mermaid_module_edge, mermaid_module_labeled_edge,
    mermaid_module_node, mermaid_node, mermaid_node_marked, mermaid_subgraph,
    plantuml_component, plantuml_component_marked, plantuml_edge, plantuml_quoted_component,
    plantuml_quoted_violation_edge, plantuml_violation_edge,
};
use crate::archspec::spec::{Module, Spec, Stereotype};
use crate::archspec::verify::compare::{Adjudicator, ModuleRender};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

struct SpecGraph {
    nodes: BTreeSet<String>,
    clusters: Vec<(String, Vec<String>)>,
    allowed_edges: BTreeSet<(String, String)>,
    forbidden_edges: BTreeSet<(String, String)>,
}

fn build_graph(spec: &Spec) -> SpecGraph {
    let mut nodes = BTreeSet::new();
    let mut clusters = Vec::new();
    let mut allowed_edges = BTreeSet::new();
    let mut forbidden_edges = BTreeSet::new();
    let mut descendants: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    fn collect(
        module: &Module,
        depth: usize,
        nodes: &mut BTreeSet<String>,
        clusters: &mut Vec<(String, Vec<String>)>,
        allowed_edges: &mut BTreeSet<(String, String)>,
        forbidden_edges: &mut BTreeSet<(String, String)>,
        descendants: &mut BTreeMap<String, BTreeSet<String>>,
    ) -> BTreeSet<String> {
        let mut subtree = BTreeSet::new();
        let mut subs: Vec<String> = Vec::new();
        for submodule in &module.submodules {
            subs.push(submodule.name.clone());
            let sub_subtree = collect(
                submodule,
                depth + 1,
                nodes,
                clusters,
                allowed_edges,
                forbidden_edges,
                descendants,
            );
            subtree.extend(sub_subtree);
        }
        if depth == 0 {
            nodes.insert(module.name.clone());
        }
        if !subs.is_empty() {
            subs.sort();
            clusters.push((module.name.clone(), subs));
        }
        for depend_on in &module.allowed.depend_on {
            allowed_edges.insert((module.name.clone(), depend_on.clone()));
        }
        for forbidden in &module.allowed.forbidden {
            forbidden_edges.insert((module.name.clone(), forbidden.clone()));
        }
        descendants.insert(module.name.clone(), subtree.clone());
        subtree.insert(module.name.clone());
        subtree
    }

    for module in &spec.modules {
        collect(
            module,
            0,
            &mut nodes,
            &mut clusters,
            &mut allowed_edges,
            &mut forbidden_edges,
            &mut descendants,
        );
    }
    let contained = |from: &str, to: &str| -> bool {
        descendants
            .get(from)
            .is_some_and(|subtree| subtree.contains(to))
            || descendants
                .get(to)
                .is_some_and(|subtree| subtree.contains(from))
    };
    allowed_edges.retain(|(from, to)| !contained(from, to));
    forbidden_edges.retain(|(from, to)| !contained(from, to));
    clusters.sort_by(|a, b| a.0.cmp(&b.0));

    SpecGraph {
        nodes,
        clusters,
        allowed_edges,
        forbidden_edges,
    }
}

pub fn render_spec_mermaid(spec: &Spec) -> String {
    let graph = build_graph(spec);
    let mut out = String::from("graph TD\n");
    for node in &graph.nodes {
        let _ = writeln!(out, "  {}", mermaid_node(node));
    }
    for (parent, subs) in &graph.clusters {
        let _ = writeln!(out, "  {}", mermaid_subgraph(parent));
        for sub in subs {
            let _ = writeln!(out, "    {}", mermaid_node(sub));
        }
        let _ = writeln!(out, "  end");
    }
    for (from, to) in &graph.allowed_edges {
        let _ = writeln!(out, "  {}", mermaid_edge(from, to));
    }
    for (from, to) in &graph.forbidden_edges {
        let _ = writeln!(out, "  {}", mermaid_labeled_edge(from, to, "forbidden"));
    }
    out
}

pub fn render_spec_plantuml(spec: &Spec) -> String {
    let graph = build_graph(spec);
    let mut out = String::from("@startuml\n");
    for node in &graph.nodes {
        let _ = writeln!(out, "{}", plantuml_component(node));
    }
    for (parent, subs) in &graph.clusters {
        let _ = writeln!(out, "package {parent} {{");
        for sub in subs {
            let _ = writeln!(out, "  {}", plantuml_component(sub));
        }
        let _ = writeln!(out, "}}");
    }
    for (from, to) in &graph.allowed_edges {
        let _ = writeln!(out, "{}", plantuml_edge(from, to));
    }
    for (from, to) in &graph.forbidden_edges {
        let _ = writeln!(out, "{}", plantuml_violation_edge(from, to, "forbidden"));
    }
    let _ = writeln!(out, "@enduml");
    out
}

struct ScanGraph {
    project_nodes: BTreeSet<String>,
    external_nodes: BTreeSet<String>,
    edges: BTreeSet<(String, String)>,
    violating: BTreeSet<(String, String)>,
    external_edges: BTreeSet<(String, String)>,
    /// Module-tier nodes: identity -> visible label (claim name + raw path,
    /// or the raw path alone when no claim owns the endpoint). Present only
    /// under `--tiers modules`; BTreeMap iteration is the canonical
    /// node order (by name).
    module_nodes: BTreeMap<String, String>,
    /// Module-tier arrows: (from identity, to identity, violates). One entry
    /// per distinct resolved endpoint pair; symbols multiplicity collapses.
    module_arrows: BTreeSet<(String, String, bool)>,
}

fn build_scan_graph(model: &Model, governing: Option<&Spec>, tiers: &TierSelection) -> ScanGraph {
    let mut project_nodes: BTreeSet<String> = model.units.iter().map(|u| u.name.clone()).collect();
    let external_nodes: BTreeSet<String> = model.external.iter().cloned().collect();
    project_nodes.retain(|name| !external_nodes.contains(name));

    // Marks and module names come from the shared adjudication seam — the
    // same pipeline `verify` runs — never a diagram-side re-derivation: a
    // unit edge is dashed exactly when `verify` reports its pair, and a
    // specless artefact (empty modules) marks nothing.
    let modules: &[Module] = governing.map(|spec| spec.modules.as_slice()).unwrap_or(&[]);
    let stereotypes: &[Stereotype] = governing.map(|spec| spec.stereotype.as_slice()).unwrap_or(&[]);
    let adjudicator = Adjudicator::new(model, modules, stereotypes);
    let constraints = governing.map(|spec| build_constraints(spec, model));

    let mut edges = BTreeSet::new();
    let mut violating = BTreeSet::new();
    let mut external_edges = BTreeSet::new();
    for edge in &model.edges {
        let touches_external =
            external_nodes.contains(&edge.from) || external_nodes.contains(&edge.to);
        if touches_external {
            external_edges.insert((edge.from.clone(), edge.to.clone()));
        } else if constraints
            .as_ref()
            .is_some_and(|constraints| edge_violates(constraints, model, &edge.from, &edge.to))
            || adjudicator.unit_edge_violates(&edge.from, &edge.to)
        {
            violating.insert((edge.from.clone(), edge.to.clone()));
        } else {
            edges.insert((edge.from.clone(), edge.to.clone()));
        }
    }

    let mut module_nodes: BTreeMap<String, String> = BTreeMap::new();
    let mut module_arrows: BTreeSet<(String, String, bool)> = BTreeSet::new();
    if tiers.modules {
        for edge in &model.module_edges {
            let placement = adjudicator.module_edge_placement(edge);
            let (from, from_path, to, to_path, draws_arrow, violates) = match &placement {
                ModuleRender::Arrow {
                    from,
                    from_path,
                    to,
                    to_path,
                    violates,
                    api_mark,
                } => (
                    *from,
                    *from_path,
                    *to,
                    *to_path,
                    true,
                    *violates || *api_mark,
                ),
                ModuleRender::Nesting {
                    from,
                    from_path,
                    to,
                    to_path,
                } => (*from, *from_path, *to, *to_path, false, false),
            };
            register_module_node(&mut module_nodes, &adjudicator, from, from_path);
            register_module_node(&mut module_nodes, &adjudicator, to, to_path);
            if draws_arrow && from != to {
                module_arrows.insert((from.to_string(), to.to_string(), violates));
            }
        }
    }

    ScanGraph {
        project_nodes,
        external_nodes,
        edges,
        violating,
        external_edges,
        module_nodes,
        module_arrows,
    }
}

/// Record one module-tier endpoint node: identity is the owning claim name
/// where a claim owns the path, else the raw path; the label keeps identity
/// and path visible together so verify findings grep into the diagram. The
/// first registered label for an identity wins (deterministic under the
/// canonical `module_edges` iteration).
fn register_module_node(
    nodes: &mut BTreeMap<String, String>,
    adjudicator: &Adjudicator<'_>,
    identity: &str,
    path: &str,
) {
    let label = if identity == path || adjudicator.claim_naming(path).is_none() {
        path.to_string()
    } else {
        format!("{identity} ({path})")
    };
    nodes.entry(identity.to_string()).or_insert(label);
}

/// The per-module subgraph of one named claim (workplan
/// rust-arch-test-kit/worklog/workplan_archspec_module_public_api_diagrams.md
/// D11): the claim's territory nodes (own files union descendant paths, unit
/// roots excluded per D10), the nested-submodule nesting of its subclaims,
/// and its first-party boundary edges. Marks — violating and public-API —
/// read the shared adjudication seam, never a re-derivation; a governing
/// spec that declares no expose anywhere carries no API marks, so the view
/// never claims a green public API it cannot see.
/// A rendered module view: nodes (id → label) and arrows (from, to, marked).
type ModuleRenderGraph = (BTreeMap<String, String>, BTreeSet<(String, String, bool)>);

fn module_subgraph(
    model: &Model,
    spec: &Spec,
    claim: &str,
) -> ModuleRenderGraph {
    let adjudicator = Adjudicator::new(model, &spec.modules, &spec.stereotype);
    let mut nodes: BTreeMap<String, String> = BTreeMap::new();
    register_module_node(&mut nodes, &adjudicator, claim, claim);
    for path in adjudicator.territory_paths(claim) {
        let identity = adjudicator.claim_naming(&path).unwrap_or(path.as_str());
        register_module_node(&mut nodes, &adjudicator, identity, &path);
    }
    let mut arrows: BTreeSet<(String, String, bool)> = BTreeSet::new();
    let territory = adjudicator.territory_paths(claim);
    for edge in &model.module_edges {
        match adjudicator.module_edge_placement(edge) {
            ModuleRender::Arrow {
                from,
                from_path,
                to,
                to_path,
                violates,
                api_mark,
            } => {
                let touches = from == claim
                    || to == claim
                    || territory.contains(from_path)
                    || territory.contains(to_path);
                if touches {
                    register_module_node(&mut nodes, &adjudicator, from, from_path);
                    register_module_node(&mut nodes, &adjudicator, to, to_path);
                    if from != to {
                        arrows.insert((from.to_string(), to.to_string(), violates || api_mark));
                    }
                }
            }
            ModuleRender::Nesting {
                from,
                from_path,
                to,
                to_path,
            } => {
                if from == claim || to == claim {
                    register_module_node(&mut nodes, &adjudicator, from, from_path);
                    register_module_node(&mut nodes, &adjudicator, to, to_path);
                }
            }
        }
    }
    for sub in adjudicator.subclaims_of(claim) {
        register_module_node(&mut nodes, &adjudicator, sub, sub);
    }
    (nodes, arrows)
}

/// Render one claim's module subgraph (mermaid) — the `--module` body.
pub fn render_module_mermaid(model: &Model, spec: &Spec, claim: &str) -> String {
    let (nodes, arrows) = module_subgraph(model, spec, claim);
    let mut out = String::from("graph TD\n");
    let _ = writeln!(out, "  {}", mermaid_subgraph(claim));
    for (identity, label) in &nodes {
        let _ = writeln!(out, "    {}", mermaid_module_node(identity, label));
    }
    for (from, to, violates) in &arrows {
        if *violates {
            let _ = writeln!(
                out,
                "    {}",
                mermaid_module_labeled_edge(from, to, "violates spec")
            );
        } else {
            let _ = writeln!(out, "    {}", mermaid_module_edge(from, to));
        }
    }
    let _ = writeln!(out, "  end");
    out
}

/// Render one claim's module subgraph (plantuml) — the `--module` body.
pub fn render_module_plantuml(model: &Model, spec: &Spec, claim: &str) -> String {
    let (nodes, arrows) = module_subgraph(model, spec, claim);
    let mut out = String::from("@startuml\n");
    let _ = writeln!(out, "package {claim} {{");
    for label in nodes.values() {
        let _ = writeln!(out, "  {}", plantuml_quoted_component(label));
    }
    for (from, to, violates) in &arrows {
        let from_label = nodes.get(from).map(String::as_str).unwrap_or(from.as_str());
        let to_label = nodes.get(to).map(String::as_str).unwrap_or(to.as_str());
        if *violates {
            let _ = writeln!(
                out,
                "{}",
                plantuml_quoted_violation_edge(from_label, to_label, "violates spec")
            );
        } else {
            let _ = writeln!(out, r#""{from_label}" --> "{to_label}""#);
        }
    }
    let _ = writeln!(out, "}}");
    let _ = writeln!(out, "@enduml");
    out
}

/// The batch filename escape (D12): split the claim name on `.`, percent-
/// escape every byte outside `[A-Za-z0-9._-]` (lowercase hex) and every `%`
/// inside a segment, encode an empty segment as `%2e`, and rejoin with
/// single dots. `.` and `..` are unrepresentable, the map is injective (`%`
/// escapes itself), and decoding recovers exactly one claim name.
pub fn claim_filename(claim: &str) -> String {
    claim
        .split('.')
        .map(|segment| {
            if segment.is_empty() {
                "%2e".to_string()
            } else {
                segment
                    .bytes()
                    .map(|byte| match byte {
                        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' => {
                            (byte as char).to_string()
                        }
                        b'.' => "%2e".to_string(),
                        other => format!("%{other:02x}"),
                    })
                    .collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Inverse of `claim_filename`.
pub fn claim_from_filename(name: &str) -> String {
    name.split('.')
        .map(|segment| {
            if segment == "%2e" {
                String::new()
            } else {
                let bytes = segment.as_bytes();
                let mut out = Vec::with_capacity(bytes.len());
                let mut i = 0;
                while i < bytes.len() {
                    if bytes[i] == b'%'
                        && i + 2 < bytes.len()
                        && let Ok(byte) =
                            u8::from_str_radix(&segment[i + 1..i + 3], 16)
                    {
                        out.push(byte);
                        i += 3;
                        continue;
                    }
                    out.push(bytes[i]);
                    i += 1;
                }
                String::from_utf8_lossy(&out).into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Resolve which unit names a governing spec forbids for each matching module.
struct ScanConstraints {
    /// unit name -> names of modules that match the unit
    modules_by_unit: BTreeMap<String, Vec<String>>,
    /// module name -> unit names the module forbids depending on
    forbidden: BTreeMap<String, BTreeSet<String>>,
}
fn build_constraints(spec: &Spec, model: &Model) -> ScanConstraints {
    let mut modules_by_unit: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for module in &spec.modules {
        for unit in &model.units {
            if unit_matches_module(unit, module) {
                modules_by_unit
                    .entry(unit.name.clone())
                    .or_default()
                    .push(module.name.clone());
            }
        }
    }
    for modules in modules_by_unit.values_mut() {
        modules.sort();
        modules.dedup();
    }

    let forbidden = spec
        .modules
        .iter()
        .map(|module| (module.name.clone(), forbidden_units(module, model, spec)))
        .collect();

    ScanConstraints {
        modules_by_unit,
        forbidden,
    }
}

fn edge_violates(constraints: &ScanConstraints, model: &Model, from: &str, to: &str) -> bool {
    if model.units.iter().all(|u| u.name != from) {
        return false;
    }
    let Some(modules) = constraints.modules_by_unit.get(from) else {
        return false;
    };
    modules.iter().any(|module| {
        constraints
            .forbidden
            .get(module)
            .map(|forbidden| forbidden.contains(to))
            .unwrap_or(false)
    })
}

/// Unit names a module may not depend on: its `allowed.forbidden` entries
/// (module names and direct name/path globs) plus `contract.forbid`
/// stereotypes matched by the target unit.
fn forbidden_units(module: &Module, model: &Model, spec: &Spec) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in &module.allowed.forbidden {
        let mut resolved_by_module = false;
        for candidate in &spec.modules {
            if glob_match(entry, &candidate.name) {
                resolved_by_module = true;
                for unit in &model.units {
                    if unit_matches_module(unit, candidate) {
                        out.insert(unit.name.clone());
                    }
                }
            }
        }
        if !resolved_by_module {
            for unit in &model.units {
                if glob_match(entry, &unit.name) || glob_match(entry, &unit.path) {
                    out.insert(unit.name.clone());
                }
            }
        }
    }
    for stereotype in &module.contract.forbid {
        for definition in &spec.stereotype {
            if glob_match(stereotype, &definition.name) {
                for unit in &model.units {
                    if stereotype_matches(unit, definition) {
                        out.insert(unit.name.clone());
                    }
                }
            }
        }
    }
    out
}

fn unit_matches_module(unit: &Unit, module: &Module) -> bool {
    module
        .matches
        .units
        .iter()
        .any(|rule| glob_match(rule, &unit.name))
        || module
            .matches
            .names
            .iter()
            .any(|rule| glob_match(rule, &unit.name))
        || module
            .matches
            .paths
            .iter()
            .any(|rule| glob_match(rule, &unit.path))
        || module
            .submodules
            .iter()
            .any(|sub| unit_matches_module(unit, sub))
}

fn stereotype_matches(unit: &Unit, stereotype: &Stereotype) -> bool {
    stereotype
        .match_set
        .units
        .iter()
        .any(|rule| glob_match(rule, &unit.name))
        || stereotype
            .match_set
            .names
            .iter()
            .any(|rule| glob_match(rule, &unit.name))
        || stereotype
            .match_set
            .paths
            .iter()
            .any(|rule| glob_match(rule, &unit.path))
}

/// Match a `*` glob against text; `*` matches any (possibly empty) run.
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let mut row = vec![false; text.len() + 1];
    row[0] = true;
    for &p in &pattern {
        let mut next = vec![false; text.len() + 1];
        if p == '*' {
            next[0] = row[0];
            for j in 0..text.len() {
                next[j + 1] = row[j + 1] || next[j];
            }
        } else {
            for j in 0..text.len() {
                if row[j] && p == text[j] {
                    next[j + 1] = true;
                }
            }
        }
        row = next;
    }
    row[text.len()]
}

/// The role a scan-sourced node carries (roles-views US 01): the node is
/// marked iff the model's `roles` map holds a key equal to the node's name
/// under the `.`/`::` separator equivalence `depgraph::parent_matches` uses
/// (rust keys at `kit`, c# at `Shop::Api`, the node stands for unit
/// `Shop.Api`). Exact identity — no subtree containment, no fold — so a node
/// the map does not address keeps its exact unmarked bytes, and presentation
/// stays a pure lookup like `inspect structural`'s marker.
fn scan_role_marker(model: &Model, name: &str) -> Option<Role> {
    let normalized = name.replace("::", ".");
    model
        .roles
        .iter()
        .find(|(key, _)| key.replace("::", ".") == normalized)
        .map(|(_, role)| *role)
}

/// Which tiers a scan render draws (`--tiers`). The selection is a render
/// selector only — no spec, model or config field stores it — and the
/// default (flag absent) is units-only, whose bytes are byte-identical to
/// the historical scan surface (workplan D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierSelection {
    pub units: bool,
    pub modules: bool,
}

impl TierSelection {
    /// The supported tiers, stated in every `--tiers` usage error (D7).
    const SUPPORTED: &'static str = "units, modules";

    /// The default selection: the unit render alone.
    pub const fn units_only() -> Self {
        Self {
            units: true,
            modules: false,
        }
    }

    /// Parse a comma-separated tier list: every entry names a supported
    /// tier, duplicates are rejected, an empty or blank list is rejected;
    /// every error names the offending value and the supported tiers (D7).
    pub fn parse(spec: &str) -> Result<Self, String> {
        let mut units = false;
        let mut modules = false;
        for entry in spec.split(',') {
            match entry {
                "units" | "modules" => {}
                other => {
                    return Err(format!(
                        "invalid value {other:?} for --tiers — supported tiers: {}",
                        Self::SUPPORTED
                    ));
                }
            }
            if (entry == "units" && units) || (entry == "modules" && modules) {
                return Err(format!(
                    "duplicate value {entry:?} for --tiers — supported tiers: {}",
                    Self::SUPPORTED
                ));
            }
            if entry == "units" {
                units = true;
            } else {
                modules = true;
            }
        }
        // `modules` renders on top of the unchanged unit render: the unit
        // tier is never dropped, `--tiers modules` equals `--tiers
        // units,modules` (D1).
        Ok(Self {
            units: true,
            modules,
        })
    }
}

pub fn render_scan_mermaid(
    model: &Model,
    governing: Option<&Spec>,
    tiers: &TierSelection,
) -> String {
    let graph = build_scan_graph(model, governing, tiers);
    let mut out = String::from("graph TD\n");
    if !graph.project_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph project");
        for node in &graph.project_nodes {
            let line = match scan_role_marker(model, node) {
                Some(role) => mermaid_node_marked(node, &format!(" [{}]", role.as_str())),
                None => mermaid_node(node),
            };
            let _ = writeln!(out, "    {line}");
        }
        for (from, to) in &graph.edges {
            let _ = writeln!(out, "    {}", mermaid_edge(from, to));
        }
        for (from, to) in &graph.violating {
            let _ = writeln!(
                out,
                "    {}",
                mermaid_labeled_edge(from, to, "violates spec")
            );
        }
        let _ = writeln!(out, "  end");
    }
    if !graph.external_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph external");
        for node in &graph.external_nodes {
            let line = match scan_role_marker(model, node) {
                Some(role) => mermaid_node_marked(node, &format!(" [{}]", role.as_str())),
                None => mermaid_node(node),
            };
            let _ = writeln!(out, "    {line}");
        }
        let _ = writeln!(out, "  end");
        for (from, to) in &graph.external_edges {
            let _ = writeln!(out, "  {}", mermaid_edge(from, to));
        }
    }
    if !graph.module_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph modules");
        for (identity, label) in &graph.module_nodes {
            let _ = writeln!(out, "    {}", mermaid_module_node(identity, label));
        }
        for (from, to, violates) in &graph.module_arrows {
            if *violates {
                let _ = writeln!(
                    out,
                    "    {}",
                    mermaid_module_labeled_edge(from, to, "violates spec")
                );
            } else {
                let _ = writeln!(out, "    {}", mermaid_module_edge(from, to));
            }
        }
        let _ = writeln!(out, "  end");
    }
    out
}

pub fn render_scan_plantuml(
    model: &Model,
    governing: Option<&Spec>,
    tiers: &TierSelection,
) -> String {
    let graph = build_scan_graph(model, governing, tiers);
    let mut out = String::from("@startuml\n");
    if !graph.project_nodes.is_empty() {
        let _ = writeln!(out, "package project {{");
        for node in &graph.project_nodes {
            let line = match scan_role_marker(model, node) {
                Some(role) => plantuml_component_marked(node, role.as_str()),
                None => plantuml_component(node),
            };
            let _ = writeln!(out, "  {line}");
        }
        for (from, to) in &graph.edges {
            let _ = writeln!(out, "{}", plantuml_edge(from, to));
        }
        for (from, to) in &graph.violating {
            let _ = writeln!(
                out,
                "{}",
                plantuml_violation_edge(from, to, "violates spec")
            );
        }
        let _ = writeln!(out, "}}");
    }
    if !graph.external_nodes.is_empty() {
        let _ = writeln!(out, "package external {{");
        for node in &graph.external_nodes {
            let line = match scan_role_marker(model, node) {
                Some(role) => plantuml_component_marked(node, role.as_str()),
                None => plantuml_component(node),
            };
            let _ = writeln!(out, "  {line}");
        }
        let _ = writeln!(out, "}}");
        for (from, to) in &graph.external_edges {
            let _ = writeln!(out, "{}", plantuml_edge(from, to));
        }
    }
    if !graph.module_nodes.is_empty() {
        let _ = writeln!(out, "package modules {{");
        for label in graph.module_nodes.values() {
            let _ = writeln!(out, "  {}", plantuml_quoted_component(label));
        }
        for (from, to, violates) in &graph.module_arrows {
            let from_label = graph
                .module_nodes
                .get(from)
                .map(String::as_str)
                .unwrap_or(from.as_str());
            let to_label = graph
                .module_nodes
                .get(to)
                .map(String::as_str)
                .unwrap_or(to.as_str());
            if *violates {
                let _ = writeln!(
                    out,
                    "{}",
                    plantuml_quoted_violation_edge(from_label, to_label, "violates spec")
                );
            } else {
                let _ = writeln!(out, r#""{from_label}" --> "{to_label}""#);
            }
        }
        let _ = writeln!(out, "}}");
    }
    let _ = writeln!(out, "@enduml");
    out
}
