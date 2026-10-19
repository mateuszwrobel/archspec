use crate::archspec::inspect::file_graph::FileGraph;
use crate::archspec::scan::python as python_scan;
use std::collections::BTreeSet;
use std::path::Path;

/// File-level import graph for a Python tree: nodes are source files, edges
/// are the resolved file-to-file import edges the scan driver states (the
/// python driver record P2 — `module_edges` rows carry the file identity by
/// construction). Wiring only, mirroring the ts arm: there is no second
/// resolver here. Extraction runs the same resolution seam `archspec scan`
/// runs (the file-set package-root derivation, the longest-existing-prefix
/// referent, the external tier), so inspect and scan can never disagree on
/// what resolves where.
///
/// A NAMESPACE-DIR target (module-grain, P3) renders no file-to-file edge —
/// it has no file to point at, exactly like an external tier row. The
/// test-tier law is the ts arm's: a production edge never TARGETS a
/// test-tier file, while test files stay nodes and import sources.
pub fn scan(root: &Path) -> Result<FileGraph, String> {
    let model = python_scan::extract(root)?;
    let mut nodes: BTreeSet<String> = BTreeSet::new();
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for edge in &model.module_edges {
        // Same-file rows state nothing; a directory target is module-grain.
        if edge.from == edge.to || !edge.to.ends_with(".py") {
            nodes.insert(edge.from.clone());
            continue;
        }
        if model.is_test_gated(&edge.to) {
            nodes.insert(edge.from.clone());
            continue;
        }
        nodes.insert(edge.from.clone());
        nodes.insert(edge.to.clone());
        edges.insert((edge.from.clone(), edge.to.clone()));
    }
    // Edge-free leaves are source files of the tree too — the per-file
    // external map is their witness (the ts arm's law).
    nodes.extend(model.module_external.keys().cloned());
    Ok(FileGraph { nodes, edges })
}
