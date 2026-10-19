use crate::archspec::inspect::file_graph::FileGraph;
use crate::archspec::scan::ts as ts_scan;
use std::collections::BTreeSet;
use std::path::Path;

/// File-level import graph for a TypeScript tree: nodes are source files,
/// edges are the resolved file-to-file import edges the scan driver states
/// (capability `file-grained-dependency-facts`: ts `module_edges` rows carry
/// the file identity by construction, D10). Wiring only, per the
/// ts-resolution record C5 — there is no second resolver here: extraction
/// runs the same resolution seam `archspec scan` runs (tsconfig maps,
/// workspace precedence, the external tier), so inspect and scan can never
/// disagree on what resolves where.
///
/// The test-tier rule reads C2's facts: a production edge never TARGETS a
/// test-tier file (the same exclusion the module-tier checks apply through
/// `is_test_gated`), while test files stay nodes and import sources — their
/// own imports render, exactly like the go arm's law, with the go file-tier
/// drop replaced by the ts serialized tier.
pub fn scan(root: &Path) -> Result<FileGraph, String> {
    let model = ts_scan::extract(root)?;
    let mut nodes: BTreeSet<String> = BTreeSet::new();
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for edge in &model.module_edges {
        // The graph is directed and kind-collapsed at render entry: value
        // and type rows are one edge, same-file rows state nothing.
        if edge.from == edge.to {
            nodes.insert(edge.from.clone());
            continue;
        }
        if model.is_test_gated(&edge.to) {
            // A production import cannot target the test tier; the row
            // renders nothing. Test files below stay nodes and sources.
            nodes.insert(edge.from.clone());
            continue;
        }
        nodes.insert(edge.from.clone());
        nodes.insert(edge.to.clone());
        edges.insert((edge.from.clone(), edge.to.clone()));
    }
    // Files that carry facts without edges (a leaf exporting and importing
    // nothing) are source files of the tree too: the per-file fact maps are
    // their witness, mirroring how every file-grained row lists them.
    nodes.extend(model.module_external.keys().cloned());
    nodes.extend(model.module_public_types.keys().cloned());
    // Test-tier files that carry no edge at all stay invisible exactly like
    // edge-free production leaves elsewhere: the graph renders facts, never
    // a directory walk (the go arm's discovery exceeds the guard model; the
    // ts facts already include the tier).
    Ok(FileGraph { nodes, edges })
}
