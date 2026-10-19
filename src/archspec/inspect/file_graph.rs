/// File-level dependency graph shared by all language file scanners (Rust,
/// C#, Go). `nodes` are source-file paths relative to the scanned root;
/// `edges` are resolved file-to-file dependency edges: imports plus, for Rust,
/// module declarations (declaring file → declared child file, collapsed per
/// pair). Renderers (mermaid/plantuml) group nodes into folder subgraphs and
/// draw the edges.
pub struct FileGraph {
    pub nodes: std::collections::BTreeSet<String>,
    pub edges: std::collections::BTreeSet<(String, String)>,
}
