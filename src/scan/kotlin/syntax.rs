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

use crate::language::Language;
use crate::model::LiteralCandidateAnchor;
use crate::parse::grammar::{node_text, parse_tree};

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

/// The literal-candidate feed of one `.kt` file (workplan
/// events_value_join scenario 03, decision-check F8): vocabulary-free
/// call sites carrying a plain string-literal argument — the callee's
/// SIMPLE name and the argument position ride as spelled, NO declared name
/// reaches this walk (A6); the engine matcher verb decides equality at the
/// resolution site. The caller runs this only when a declared pattern
/// names the kotlin driver (4.2 arm 1: the declaration gate is the byte
/// identity). A file that cannot be read or parsed states no candidate.
pub(crate) fn literal_candidates(path: &Path) -> Vec<(LiteralCandidateAnchor, String)> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(tree) = parse_tree(Language::Kotlin, &source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_call_candidates(tree.root_node(), &source, &mut out);
    collect_property_candidates(tree.root_node(), &source, &mut out);
    out
}

/// The exported-property-anchor walk (workplan events_value_join scenario
/// 09, decision (b) of the kotlin reading): the `property_declaration`s
/// that are DIRECT children of `source_file` — the top-level bindings,
/// `const val` included. `private` and `internal` bindings are skipped
/// through the grammar `modifiers` layer (they are not exported surface);
/// member properties (class or companion bodies) are not top level and
/// contribute nothing. One candidate per binding whose initializer is a
/// plain (no interpolation) string literal; every other shape states no
/// literal value for this verb.
fn collect_property_candidates<'a>(node: Node<'a>, source: &'a str, out: &mut Vec<(LiteralCandidateAnchor, String)>) {
    if node.kind() != "source_file" {
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "property_declaration" {
            continue;
        }
        let mut child_cursor = child.walk();
        let mut skipped = false;
        let mut name: Option<String> = None;
        let mut value: Option<String> = None;
        for part in child.named_children(&mut child_cursor) {
            match part.kind() {
                "modifiers" => {
                    let mut modifier_cursor = part.walk();
                    for modifier in part.named_children(&mut modifier_cursor) {
                        if modifier.kind() == "visibility_modifier" {
                            let text = node_text(&modifier, source);
                            if text == "private" || text == "internal" {
                                skipped = true;
                            }
                        }
                    }
                }
                // The binding name: a direct `simple_identifier`, or the
                // first one under `variable_declaration` (the grammar keeps
                // the binding kind and its name there).
                "simple_identifier" => {
                    if name.is_none() {
                        name = Some(node_text(&part, source).to_string());
                    }
                }
                "variable_declaration" => {
                    if name.is_none()
                        && let Some(identifier) = find_kind(part, "simple_identifier")
                    {
                        name = Some(node_text(&identifier, source).to_string());
                    }
                }
                // The initializer rides as a direct child: a plain string
                // literal carries the value, anything else states none. A
                // call initializer reaches this arm with no plain literal
                // INSIDE the walk's own rules (interpolation law reused:
                // `string_literal_value` requires the WHOLE initializer to
                // be one literal, no nesting through arbitrary expressions).
                _ => {
                    if value.is_none()
                        && part.kind() == "string_literal"
                        && let Some(literal) = string_literal_value(part, source)
                    {
                        value = Some(literal);
                    }
                }
            }
        }
        if let (false, Some(property), Some(literal)) = (skipped, name, value) {
            out.push((LiteralCandidateAnchor::ExportedProperty { property }, literal));
        }
    }
}

/// The call-anchor walk: every call site in the tree, wherever it sits;
/// one candidate per argument position whose node holds a plain (no
/// interpolation) string literal.
fn collect_call_candidates<'a>(
    node: Node<'a>,
    source: &'a str,
    out: &mut Vec<(LiteralCandidateAnchor, String)>,
) {
    if node.kind() == "call_expression" {
        let callee = node
            .named_children(&mut node.walk())
            .filter(|child| child.kind() != "value_arguments" && child.kind() != "call_suffix")
            .flat_map(|receiver| simple_identifiers(receiver, source))
            .last();
        let arguments = node
            .named_children(&mut node.walk())
            .find(|child| child.kind() == "value_arguments" || child.kind() == "call_suffix");
        if let (Some(callee), Some(arguments)) = (callee, arguments) {
            let mut position = 0usize;
            for argument in arguments.named_children(&mut arguments.walk()) {
                if argument.kind() != "value_argument" {
                    continue;
                }
                if let Some(value) = string_literal_value(argument, source) {
                    out.push((
                        LiteralCandidateAnchor::Call {
                            callee: callee.clone(),
                            position,
                        },
                        value,
                    ));
                }
                position += 1;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_candidates(child, source, out);
    }
}

/// Every `simple_identifier` under `node`, in order — the last one is the
/// name the source calls (a navigation chain's method name, a bare call's
/// function name).
fn simple_identifiers<'a>(node: Node<'a>, source: &'a str) -> Vec<String> {
    let mut names = Vec::new();
    if node.kind() == "simple_identifier" {
        names.push(node_text(&node, source).to_string());
        return names;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        names.extend(simple_identifiers(child, source));
    }
    names
}

/// The plain value of a `value_argument`'s string literal, if it holds
/// exactly one and it carries no interpolation: content fragments and
/// escape sequences join in order, escapes decoded (the ts `decode_escape`
/// posture, kotlin's single-char subset).
fn string_literal_value(node: Node, source: &str) -> Option<String> {
    let literal = find_kind(node, "string_literal")?;
    if find_kind(literal, "string_interpolation").is_some() {
        return None;
    }
    let mut text = String::new();
    let mut saw_content = false;
    let mut cursor = literal.walk();
    for child in literal.named_children(&mut cursor) {
        collect_string_parts(child, source, &mut text, &mut saw_content);
    }
    if saw_content {
        return Some(text);
    }
    // An empty literal (`""`) has no content child and carries the empty value.
    let raw = node_text(&literal, source);
    let inner = raw.trim_start_matches('"').trim_end_matches('"');
    if inner.is_empty() {
        Some(String::new())
    } else {
        Some(inner.to_string())
    }
}

/// Join the content fragments of a kotlin string literal, decoding the
/// single-char escape forms (the ts `decode_escape` posture, kotlin subset).
fn collect_string_parts(node: Node, source: &str, text: &mut String, saw_content: &mut bool) {
    match node.kind() {
        "string_content" => {
            *saw_content = true;
            text.push_str(node_text(&node, source));
        }
        "escape_sequence" => {
            *saw_content = true;
            text.push_str(match node_text(&node, source) {
                "\\n" => "\n",
                "\\r" => "\r",
                "\\t" => "\t",
                "\\b" => "\u{8}",
                "\\\"" => "\"",
                "\\'" => "'",
                "\\\\" => "\\",
                "\\$" => "$",
                other => other,
            });
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_string_parts(child, source, text, saw_content);
            }
        }
    }
}

/// Depth-first search for the first node of `kind` at or below `node`.
fn find_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_kind(child, kind) {
            return Some(found);
        }
    }
    None
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
