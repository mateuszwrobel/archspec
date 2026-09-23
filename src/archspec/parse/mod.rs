//! Syntax-tree grammar layer shared by the tree-sitter scanners.
//!
//! Holds the grammar registry (language → in-binary grammar handle), parser
//! construction ([`grammar::parse_tree`]), the error-tolerance probe
//! ([`grammar::first_error_range`]) and the node text helper
//! ([`grammar::node_text`]) that the csharp/go syntax submodules walk trees
//! with. Procedural: pure functions,
//! no state, no domain entities — an engine in the same sense as `language`
//! and `spec`. Rust never goes through this path (the `syn` alias rule,
//! ADR-014), so the registry answers no grammar for it.
//!
//! The surface is consumed by the csharp and go `syntax` submodules (workplan
//! US 06, US 11). [`grammar::first_error_range`] is used
//! by the grammar module's own tests and documented as the error-tolerance
//! probe the scanners consult, but the csharp path deliberately extracts from
//! the tree regardless (see `scan::csharp::syntax`), so that one item stays
//! not-yet-called-from-a-command-path; the module keeps one honest
//! `allow(dead_code)` for it while unit tests exercise every item.
#![allow(dead_code)]

pub mod grammar;
