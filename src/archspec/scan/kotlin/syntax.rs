//! Grammar source facts for kotlin extraction (kotlin driver record K-P3/K-P4):
//! the two grammar nodes `package_header` and `import_header` are the whole
//! package/import surface; the package declaration is IDENTITY (K-P3) and the
//! import list is the ONLY edge source (K-P6 blind spot — a same-package
//! reference needs no import and is therefore invisible here, stated).
//!
//! Grammar node kinds consumed (brokk-tree-sitter-kotlin 0.4.6, exact pin,
//! probed against the runtime 0.24 per K-P8):
//! - `package_header` — child `identifier` spanning the whole dotted chain.
//!   A file without one is the NAMED MISS CLASS (K-P3), never a directory
//!   guess: the assembly keeps its package `None`.
//! - `import_list` → `import_header` — child `identifier` spans the dotted
//!   chain (an alias is the `import_alias` child, a binding never a path
//!   component; a trailing `wildcard_import` marks the star form, which is
//!   PARKED — it carries no fact, K-P3).
//! - `class_declaration` / `object_declaration` / `function_declaration` /
//!   `type_alias` / `property_declaration` — the top-level declaration names
//!   (the symbol census beside each source row).
//!
//! Only `source_file`'s DIRECT children are read: packages, imports and
//! top-level declarations cannot appear elsewhere in Kotlin. Error
//! tolerance is the go/C#/python law: a tree damaged into `ERROR` wrappers
//! still contributes every import the parser located; only a file
//! tree-sitter cannot read at all errors out.

use tree_sitter::Node;

use crate::archspec::language::Language;
use crate::archspec::parse::grammar::{node_text, parse_tree};

use std::collections::BTreeSet;
use std::path::Path;

/// One import statement's shape, as the resolution layer consumes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Import {
    /// The dotted chain exactly as spelled (`pl.example.app.Thing`).
    pub(crate) path: String,
    /// 1-based line of the statement (the parked-star line would name it;
    /// today no line ships, the star simply carries no fact).
    #[allow(dead_code)]
    pub(crate) line: usize,
    /// True for `import a.b.*` — the PARKED star form (K-P3: wildcard
    /// expansion needs the dir↔package law Kotlin does not carry).
    pub(crate) wildcard: bool,
}

/// The grammar facts of one `.kt` file.
#[derive(Clone, Debug, Default)]
pub(crate) struct FileFacts {
    /// The `package` declaration; `None` is the named miss class (K-P3).
    pub(crate) package: Option<String>,
    /// Every import statement, star form included (the star resolves to
    /// nothing — parked — but is counted for the tripwire evidence side).
    pub(crate) imports: Vec<Import>,
    /// Top-level declaration names bound by this file (sorted, deduped).
    pub(crate) declarations: BTreeSet<String>,
    /// Count of import statements — the tripwire's evidence side (the ts
    /// and python tripwire precedent).
    pub(crate) import_statements: usize,
}

/// Read and parse one `.kt` file, returning its grammar facts.
pub(crate) fn scan_kotlin_file(path: &Path) -> Result<FileFacts, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    scan_kotlin_source(&raw)
}

pub(crate) fn scan_kotlin_source(source: &str) -> Result<FileFacts, String> {
    let tree = parse_tree(Language::Kotlin, source).map_err(|err| err.to_string())?;
    let mut facts = FileFacts::default();
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "package_header" => {
                if facts.package.is_none() {
                    facts.package = child
                        .named_child(0)
                        .map(|node| node_text(&node, source).to_string())
                        .filter(|text| !text.is_empty());
                }
            }
            "import_list" => {
                let mut inner = child.walk();
                for header in child.named_children(&mut inner) {
                    if header.kind() != "import_header" {
                        continue;
                    }
                    facts.import_statements += 1;
                    let mut path = String::new();
                    let mut wildcard = false;
                    let mut header_cursor = header.walk();
                    for part in header.named_children(&mut header_cursor) {
                        match part.kind() {
                            "identifier" => path = node_text(&part, source).to_string(),
                            "wildcard_import" => wildcard = true,
                            // `import_alias` is a binding, never a path
                            // component (the python alias law, same law).
                            _ => {}
                        }
                    }
                    if path.is_empty() {
                        continue;
                    }
                    facts.imports.push(Import {
                        path,
                        line: line_of(child_start(child, header), source),
                        wildcard,
                    });
                }
            }
            "class_declaration" | "object_declaration" | "type_alias" => {
                // `type_alias` aliases its name `simple_identifier` to
                // `type_identifier` (grammar 0.4.6), so the three share the
                // one name shape.
                if let Some(name) = first_child_kind(child, "type_identifier", source) {
                    facts.declarations.insert(name);
                }
            }
            "function_declaration" => {
                if let Some(name) = first_named_simple(child, source) {
                    facts.declarations.insert(name);
                }
            }
            "property_declaration" => {
                // `binding_pattern_kind` is the `val`/`var` KEYWORD, not a
                // name: the bound names live in the `variable_declaration`
                // (plain form) or under the `multi_variable_declaration`
                // destructuring entries (grammar 0.4.6).
                let mut inner = child.walk();
                for binding in child.named_children(&mut inner) {
                    match binding.kind() {
                        "variable_declaration" => {
                            if let Some(name) = first_named_simple(binding, source) {
                                facts.declarations.insert(name);
                            }
                        }
                        "multi_variable_declaration" => {
                            let mut entries = binding.walk();
                            for entry in binding.named_children(&mut entries) {
                                if entry.kind() == "variable_declaration"
                                    && let Some(name) = first_named_simple(entry, source)
                                {
                                    facts.declarations.insert(name);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    facts.imports.sort();
    facts.imports.dedup();
    Ok(facts)
}

/// The 1-based line of a byte offset.
fn line_of(offset: usize, source: &str) -> usize {
    source[..offset.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn child_start(parent: Node<'_>, child: Node<'_>) -> usize {
    // `parent.start_byte()` is unused: the header's own start is the line.
    let _ = parent;
    child.start_byte()
}

fn first_child_kind(node: Node<'_>, kind: &str, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
        .map(|child| node_text(&child, source).to_string())
}

/// The bound name of a `function_declaration` or `variable_declaration`:
/// the first direct `simple_identifier` child (the receiver, when present,
/// is reached through the `receiver` FIELD, never as a plain child — so the
/// first plain one is the name; a destructuring entry's optional `val`/`var`
/// is a `binding_pattern_kind`, a different kind).
fn first_named_simple(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == "simple_identifier")
        .map(|child| node_text(&child, source).to_string())
}

#[cfg(test)]
mod tests {
    use super::scan_kotlin_source;

    #[test]
    fn package_and_imports_are_the_direct_children() {
        let facts = scan_kotlin_source(
            "package pl.example.app\n\nimport android.content.Intent\nimport pl.example.core.Engine as E\nimport pl.example.lib.*\n\nclass Thing\n",
        )
        .expect("parses");
        assert_eq!(facts.package.as_deref(), Some("pl.example.app"));
        assert_eq!(facts.import_statements, 3);
        assert_eq!(facts.imports[0].path, "android.content.Intent");
        assert_eq!(facts.imports[1].path, "pl.example.core.Engine");
        assert!(
            !facts.imports[1].wildcard,
            "an alias is a binding, never the star"
        );
        assert!(facts.imports[2].wildcard, "the star form is marked");
        assert!(facts.declarations.contains("Thing"));
    }

    #[test]
    fn packageless_source_is_the_named_miss_class() {
        let facts = scan_kotlin_source("fun helper() {}\n").expect("parses");
        assert_eq!(facts.package, None);
    }

    #[test]
    fn declaration_names_carry_the_symbol_census() {
        let facts = scan_kotlin_source(
            "package p\n\nclass A\nobject B\nfun go() {}\nval flag = 1\ntypealias Fn = () -> Unit\n",
        )
        .expect("parses");
        for name in ["A", "B", "go", "flag", "Fn"] {
            assert!(facts.declarations.contains(name), "missing {name}");
        }
    }
}
