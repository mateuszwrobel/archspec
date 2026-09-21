use crate::archspec::inspect::FileGraph;
use rust_arch_test_kit::render::{
    mermaid_edge, mermaid_layout_link, mermaid_node, mermaid_subgraph,
};
use crate::archspec::inspect::tree::{build_tree, Dir};
use std::fmt::Write;

pub fn render(graph: &FileGraph) -> String {
    let mut out = String::from("graph TD\n");
    let tree = build_tree(&graph.nodes);
    emit_dir(&mut out, &tree, 1, "");
    for (from, to) in &graph.edges {
        let _ = writeln!(out, "  {}", mermaid_edge(from, to));
    }
    out
}

fn emit_dir(out: &mut String, dir: &Dir, indent: usize, subpath: &str) {
    let pad = "  ".repeat(indent);
    let files: Vec<&String> = dir.files.iter().collect();
    for file in &files {
        let _ = writeln!(out, "{pad}{}", mermaid_node(file));
    }
    for pair in files.windows(2) {
        let _ = writeln!(out, "{pad}{}", mermaid_layout_link(pair[0], pair[1]));
    }
    let subdirs: Vec<String> = dir.dirs.keys().cloned().collect();
    for name in &subdirs {
        let sub = &dir.dirs[name];
        let sub_full = if subpath.is_empty() {
            name.clone()
        } else {
            format!("{subpath}/{name}")
        };
        let _ = writeln!(out, "{pad}{}", mermaid_subgraph(&sub_full));
        emit_dir(out, sub, indent + 1, &sub_full);
        let _ = writeln!(out, "{pad}end");
    }
    if !files.is_empty() && !subdirs.is_empty() {
        let from = files[files.len() - 1];
        let to = first_node(&dir.dirs[&subdirs[0]]);
        let _ = writeln!(out, "{pad}{}", mermaid_layout_link(from, &to));
    }
    for pair in subdirs.windows(2) {
        let from = last_node(&dir.dirs[&pair[0]]);
        let to = first_node(&dir.dirs[&pair[1]]);
        let _ = writeln!(out, "{pad}{}", mermaid_layout_link(&from, &to));
    }
}

fn first_node(dir: &Dir) -> String {
    if let Some(file) = dir.files.iter().next() {
        return file.clone();
    }
    first_node(dir.dirs.values().next().unwrap())
}

fn last_node(dir: &Dir) -> String {
    if let Some((_, sub)) = dir.dirs.iter().next_back() {
        return last_node(sub);
    }
    dir.files.iter().next_back().unwrap().clone()
}
