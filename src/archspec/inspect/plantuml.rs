use crate::archspec::inspect::FileGraph;
use rust_arch_test_kit::render::{plantuml_entity, plantuml_quoted_edge};
use crate::archspec::inspect::tree::build_tree;
use std::fmt::Write;

pub fn render(graph: &FileGraph) -> String {
    let mut out = String::from("@startuml\n");
    let tree = build_tree(&graph.nodes);
    emit_dir(&mut out, &tree, 1, "");
    for (from, to) in &graph.edges {
        let _ = writeln!(out, "{}", plantuml_quoted_edge(from, to));
    }
    let _ = writeln!(out, "@enduml");
    out
}

fn emit_dir(
    out: &mut String,
    dir: &crate::archspec::inspect::tree::Dir,
    indent: usize,
    subpath: &str,
) {
    let pad = "  ".repeat(indent);
    for file in &dir.files {
        let _ = writeln!(out, "{pad}{}", plantuml_entity(file));
    }
    for (name, sub) in &dir.dirs {
        let sub_full = if subpath.is_empty() {
            name.clone()
        } else {
            format!("{subpath}/{name}")
        };
        let _ = writeln!(out, "{pad}package \"{sub_full}\" {{");
        emit_dir(out, sub, indent + 1, &sub_full);
        let _ = writeln!(out, "{pad}}}");
    }
}
