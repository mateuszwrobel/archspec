//! Grammar registry and syntax-tree helpers shared by the syntax scanners.
//!
//! Grammars are compiled in-binary, pinned to tree-sitter 0.24 with grammar
//! crates 0.23 (ADR-014): no runtime toolchain dependency. The registry maps
//! the engine's [`Language`] to a grammar handle; Rust answers `None` because
//! it has no tree-sitter path — `syn` is its syntax substrate (alias rule).

use tree_sitter::{Node, Parser, Tree};

use crate::archspec::language::Language;

/// Why a syntax-tree parse could not start or produced no tree.
#[derive(Debug)]
pub enum ParseError {
    /// No grammar is compiled in for this language (rust has no tree-sitter
    /// path; unknown grammar languages cannot reach this layer).
    NoGrammar(&'static str),
    /// tree-sitter rejected the grammar or returned no tree.
    Parser(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NoGrammar(language) => write!(f, "no grammar for language: {language}"),
            ParseError::Parser(reason) => write!(f, "tree-sitter failed to parse: {reason}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Handle returning the compiled-in grammar for one language.
type GrammarHandle = fn() -> tree_sitter::Language;

fn csharp_grammar() -> tree_sitter::Language {
    tree_sitter_c_sharp::LANGUAGE.into()
}

fn go_grammar() -> tree_sitter::Language {
    tree_sitter_go::LANGUAGE.into()
}

/// Grammar registry: language → compiled-in grammar. Rust (the only language
/// without a tree-sitter path) and any future language without a registered
/// handle answer `None`.
pub fn grammar_for(language: Language) -> Option<tree_sitter::Language> {
    let handle: GrammarHandle = match language {
        Language::Csharp => csharp_grammar,
        Language::Go => go_grammar,
        Language::Rust => return None,
    };
    Some(handle())
}

/// Parses `source` with the grammar registered for `language` and returns the
/// syntax tree. The tree is error-tolerant: incomplete input still yields a
/// tree, so callers probe it with [`first_error_range`] before trusting it.
/// Errors when no grammar exists for `language` or tree-sitter refuses the
/// grammar / returns no tree.
pub fn parse_tree(language: Language, source: &str) -> Result<Tree, ParseError> {
    let grammar = grammar_for(language).ok_or(ParseError::NoGrammar(language.as_str()))?;
    let mut parser = Parser::new();
    parser
        .set_language(&grammar)
        .map_err(|err| ParseError::Parser(err.to_string()))?;
    parser
        .parse(source, None)
        .ok_or_else(|| ParseError::Parser("parser returned no tree".to_string()))
}

/// Error-tolerance probe: byte range of the first `ERROR` or `MISSING` node
/// in the tree, scanning in document order (preorder over named and
/// anonymous nodes alike). `None` means the tree parsed clean. Syntax
/// scanners use this to decide between extraction and fallback/reporting.
pub fn first_error_range(tree: &Tree) -> Option<(usize, usize)> {
    error_range_in(&tree.root_node())
}

fn error_range_in(node: &Node<'_>) -> Option<(usize, usize)> {
    if node.kind() == "ERROR" || node.is_missing() {
        return Some((node.start_byte(), node.end_byte()));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let found = error_range_in(&child);
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Source text a node covers, sliced by its byte range. `source` must be the
/// exact text the tree was parsed from (the invariant at every call site:
/// [`parse_tree`] never copies or re-encodes it).
pub fn node_text<'a>(node: &Node<'_>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

#[cfg(test)]
mod tests {
    // Import through the same path the scanners will call: archspec::parse::grammar.
    use super::{first_error_range, grammar_for, node_text, parse_tree, ParseError};
    use crate::archspec::language::Language;

    #[test]
    fn registry_answers_compiled_grammars_per_language() {
        assert!(grammar_for(Language::Csharp).is_some());
        assert!(grammar_for(Language::Go).is_some());
        // Rust has no tree-sitter path (syn alias rule) — never a handle.
        assert!(grammar_for(Language::Rust).is_none());
    }

    #[test]
    fn parse_tree_rejects_languages_without_a_grammar() {
        let error = parse_tree(Language::Rust, "fn main() {}")
            .expect_err("rust must not parse through the grammar layer");
        assert!(
            matches!(error, ParseError::NoGrammar("rust")),
            "expected NoGrammar(rust), got {error:?}"
        );
    }

    #[test]
    fn csharp_snippet_parses_to_clean_compilation_unit() {
        let source = "namespace App.Core\n{\n    public class Engine\n    {\n    }\n}\n";
        let tree = parse_tree(Language::Csharp, source).expect("csharp parses");
        assert_eq!(tree.root_node().kind(), "compilation_unit");
        assert!(!tree.root_node().has_error());
        assert_eq!(first_error_range(&tree), None);
    }

    #[test]
    fn truncated_csharp_snippet_flags_first_error_region() {
        let source = "namespace App.Core\n{\n    public class Engine\n    {\n";
        let tree = parse_tree(Language::Csharp, source).expect("error-tolerant tree");
        assert!(tree.root_node().has_error());
        let (start, end) = first_error_range(&tree).expect("error region reported");
        assert!(start < source.len(), "error start inside source: {start}");
        assert!(end <= source.len(), "error end inside source: {end}");
    }

    #[test]
    fn go_hello_world_parses_clean() {
        let source = "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hello\")\n}\n";
        let tree = parse_tree(Language::Go, source).expect("go parses");
        assert_eq!(tree.root_node().kind(), "source_file");
        assert_eq!(first_error_range(&tree), None);
    }

    #[test]
    fn node_text_slices_the_node_byte_range() {
        let source = "package main\n\nfunc main() {}\n";
        let tree = parse_tree(Language::Go, source).expect("go parses");
        let root = tree.root_node();
        assert_eq!(node_text(&root, source), source);
        let first_line = root
            .named_child(0)
            .expect("package clause is the first named child");
        assert_eq!(node_text(&first_line, source), "package main");
    }
}
