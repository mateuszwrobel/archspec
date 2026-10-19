use crate::archspec::inspect::file_graph::FileGraph;
use crate::archspec::scan::kotlin as kotlin_scan;
use std::collections::BTreeSet;
use std::path::Path;

/// File-level import graph for a Kotlin tree: nodes are `.kt` source files,
/// edges are the resolved file-to-file import edges the scan driver states
/// (the kotlin driver record K-P3 — `file_edges` rows carry the file
/// identity by construction). Wiring only, mirroring the ts and python
/// arms: there is no second resolver here. Extraction runs the same
/// census-resolution seam `archspec scan` runs (the package census, the
/// (package, basename) key, the external tier), so inspect and scan can
/// never disagree on what resolves where.
///
/// The test-tier law is the ts arm's: a production edge never TARGETS a
/// test-tier file, while test files stay nodes and import sources. The
/// K-P6 blind spot holds here too: a same-package reference carries no
/// import and therefore no edge — the model states what imports state.
pub fn scan(root: &Path) -> Result<FileGraph, String> {
    let model = kotlin_scan::extract(root)?;
    let mut nodes: BTreeSet<String> = BTreeSet::new();
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for edge in &model.file_edges {
        nodes.insert(edge.from.clone());
        // A test-gated endpoint stays a NODE — only the edge is dropped
        // (the ts arm's law: production edges never target test files,
        // the files themselves stay visible and import sources).
        nodes.insert(edge.to.clone());
        if model.is_test_gated(&edge.to) {
            continue;
        }
        edges.insert((edge.from.clone(), edge.to.clone()));
    }
    // Edge-free leaves are source files of the tree too — the per-file
    // external map is their witness (the ts arm's law).
    nodes.extend(model.module_external.keys().cloned());
    Ok(FileGraph { nodes, edges })
}
