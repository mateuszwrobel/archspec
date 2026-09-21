use crate::archspec::model::{Model, Unit};
use rust_arch_test_kit::render::{
    mermaid_edge, mermaid_labeled_edge, mermaid_node, mermaid_subgraph, plantuml_component,
    plantuml_edge, plantuml_violation_edge,
};
use crate::archspec::spec::{Module, Spec, Stereotype};
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

    for module in &spec.modules {
        nodes.insert(module.name.clone());
        if !module.submodules.is_empty() {
            let mut subs: Vec<String> = module
                .submodules
                .iter()
                .map(|sub| sub.name.clone())
                .collect();
            subs.sort();
            clusters.push((module.name.clone(), subs));
        }
        for depend_on in &module.allowed.depend_on {
            allowed_edges.insert((module.name.clone(), depend_on.clone()));
        }
        for forbidden in &module.allowed.forbidden {
            forbidden_edges.insert((module.name.clone(), forbidden.clone()));
        }
    }
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
}

/// Resolve which unit names a governing spec forbids for each matching module.
struct ScanConstraints {
    /// unit name -> names of modules that match the unit
    modules_by_unit: BTreeMap<String, Vec<String>>,
    /// module name -> unit names the module forbids depending on
    forbidden: BTreeMap<String, BTreeSet<String>>,
}

fn build_scan_graph(model: &Model, governing: Option<&Spec>) -> ScanGraph {
    let mut project_nodes: BTreeSet<String> = model.units.iter().map(|u| u.name.clone()).collect();
    let external_nodes: BTreeSet<String> = model.external.iter().cloned().collect();
    project_nodes.retain(|name| !external_nodes.contains(name));

    let constraints = governing.map(|spec| build_constraints(spec, model));

    let mut edges = BTreeSet::new();
    let mut violating = BTreeSet::new();
    let mut external_edges = BTreeSet::new();
    for edge in &model.edges {
        let touches_external =
            external_nodes.contains(&edge.from) || external_nodes.contains(&edge.to);
        if touches_external {
            external_edges.insert((edge.from.clone(), edge.to.clone()));
        } else if let Some(constraints) = &constraints {
            if edge_violates(constraints, model, &edge.from, &edge.to) {
                violating.insert((edge.from.clone(), edge.to.clone()));
            } else {
                edges.insert((edge.from.clone(), edge.to.clone()));
            }
        } else {
            edges.insert((edge.from.clone(), edge.to.clone()));
        }
    }

    ScanGraph {
        project_nodes,
        external_nodes,
        edges,
        violating,
        external_edges,
    }
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

pub fn render_scan_mermaid(model: &Model, governing: Option<&Spec>) -> String {
    let graph = build_scan_graph(model, governing);
    let mut out = String::from("graph TD\n");
    if !graph.project_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph project");
        for node in &graph.project_nodes {
            let _ = writeln!(out, "    {}", mermaid_node(node));
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
            let _ = writeln!(out, "    {}", mermaid_node(node));
        }
        let _ = writeln!(out, "  end");
        for (from, to) in &graph.external_edges {
            let _ = writeln!(out, "  {}", mermaid_edge(from, to));
        }
    }
    out
}

pub fn render_scan_plantuml(model: &Model, governing: Option<&Spec>) -> String {
    let graph = build_scan_graph(model, governing);
    let mut out = String::from("@startuml\n");
    if !graph.project_nodes.is_empty() {
        let _ = writeln!(out, "package project {{");
        for node in &graph.project_nodes {
            let _ = writeln!(out, "  {}", plantuml_component(node));
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
            let _ = writeln!(out, "  {}", plantuml_component(node));
        }
        let _ = writeln!(out, "}}");
        for (from, to) in &graph.external_edges {
            let _ = writeln!(out, "{}", plantuml_edge(from, to));
        }
    }
    let _ = writeln!(out, "@enduml");
    out
}
