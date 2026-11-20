//! Grammar source facts for python extraction (python driver record P2):
//! the two grammar nodes `import_statement` and `import_from_statement` are
//! the whole import surface; aliases never change the module path; the
//! `from a.b import c` form records the module `a.b` plus the NAME LIST the
//! resolution layer consults for the submodule-or-attribute arm.
//!
//! Grammar node kinds consumed (tree-sitter-python 0.23.6, exact pin,
//! probed against the runtime 0.24 per ADR-014):
//! - `import_statement` — children `dotted_name` (`import a.b`) or
//!   `aliased_import` (fields `name:`/`alias:` — the alias is a binding,
//!   never a path component).
//! - `import_from_statement` — field `module_name:` (`dotted_name` absolute,
//!   or `relative_import` = `import_prefix` dots plus optional `dotted_name`);
//!   the imported names are the remaining named children (`dotted_name`,
//!   `aliased_import`, or `wildcard_import` for `*` — a star carries no
//!   name, the module edge stands).
//! - `call` — the dynamic-import sites: `importlib.import_module` and
//!   `importlib.util.spec_from_file_location`, attribute-rooted or bound
//!   through a `from importlib import …` name (the measured pilot spellings).
//!   Literal string arguments ride the wildcard too (P2 literal-string
//!   clause): extraction of literal-to-fact is PARKED, the line names the
//!   site.
//!
//! `from __future__ import …` carries NO fact (compiler directive, record
//! P2): the statement is not counted and contributes no import form.
//! Error tolerance is the go/C# law: a tree damaged into `ERROR` wrappers
//! still contributes every import the parser located; only a file
//! tree-sitter cannot parse at all errors out.

use tree_sitter::Node;

use crate::language::Language;
use crate::model::LiteralCandidateAnchor;
use crate::parse::grammar::{node_text, parse_tree};

use std::collections::BTreeSet;
use std::path::Path;

/// One import statement's shape, as the resolution layer consumes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImportForm {
    /// `import a.b.c` (names empty) or `from a.b import x, y` (names the
    /// imported list; a star import lists nothing).
    Absolute {
        specifier: String,
        names: Vec<String>,
    },
    /// `from . import x` / `from ..p.q import x`: `level` is the dot count,
    /// the anchor is the importing file's directory climbed `level − 1`
    /// levels, then `module`'s segments (P3 relative law).
    Relative {
        level: usize,
        module: Option<String>,
        names: Vec<String>,
    },
}

/// The grammar facts of one `.py` file.
#[derive(Clone, Debug, Default)]
pub(crate) struct FileFacts {
    /// Every modeled import form (deduplicated by the assembly below).
    pub(crate) imports: Vec<ImportForm>,
    /// Names this file binds from `importlib` (P2 dynamic arm): a bare call
    /// through one of them is a wildcard site.
    pub(crate) importlib_bindings: BTreeSet<String>,
    /// Dynamic-import sites: 1-based line plus the call's spelled name,
    /// sorted and deduplicated (the report-only wildcard line, P2/P3).
    pub(crate) dynamic_sites: Vec<(usize, String)>,
    /// Count of modeled import statements — the tripwire's evidence side
    /// (the ts tripwire precedent).
    pub(crate) import_statements: usize,
}

/// Read and parse one `.py` file, returning its grammar facts.
pub(crate) fn scan_python_file(path: &Path) -> Result<FileFacts, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    scan_python_source(&raw)
}

pub(crate) fn scan_python_source(source: &str) -> Result<FileFacts, String> {
    let tree = parse_tree(Language::Python, source).map_err(|err| err.to_string())?;
    let mut facts = FileFacts::default();
    collect(&tree.root_node(), source, &mut facts);
    facts.imports.sort();
    facts.imports.dedup();
    facts.dynamic_sites.sort();
    facts.dynamic_sites.dedup();
    Ok(facts)
}

fn collect(node: &Node<'_>, source: &str, facts: &mut FileFacts) {
    match node.kind() {
        "import_statement" => {
            record_import(node, source, facts);
            return;
        }
        "import_from_statement" => {
            record_import_from(node, source, facts);
            return;
        }
        "call" => {
            record_dynamic_call(node, source, facts);
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(&child, source, facts);
    }
}

/// `import a.b.c [as m], d` — one entry per imported module path; the alias
/// is dropped (P2: aliases never change the module path).
fn record_import(node: &Node<'_>, source: &str, facts: &mut FileFacts) {
    let mut located = false;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let path = match child.kind() {
            "dotted_name" => node_text(&child, source).to_string(),
            "aliased_import" => child
                .child_by_field_name("name")
                .map(|name| node_text(&name, source).to_string())
                .unwrap_or_default(),
            _ => continue,
        };
        if path.is_empty() {
            continue;
        }
        located = true;
        facts.imports.push(ImportForm::Absolute {
            specifier: path,
            names: Vec::new(),
        });
    }
    if located {
        facts.import_statements += 1;
    }
}

/// `from a.b import x[, y as z]` / `from .mod import x` — the module and the
/// imported-name list (a name's alias never changes the module path either).
/// `from __future__ import …` carries no fact and is not counted.
fn record_import_from(node: &Node<'_>, source: &str, facts: &mut FileFacts) {
    let Some(module) = node.child_by_field_name("module_name") else {
        return;
    };
    let module_text = node_text(&module, source);
    if module.kind() == "relative_import" {
        let dots = module_text.chars().take_while(|c| *c == '.').count();
        let tail = &module_text[dots.min(module_text.len())..];
        let tail = tail.trim_start_matches('.');
        facts.imports.push(ImportForm::Relative {
            level: dots.max(1),
            module: (!tail.is_empty()).then(|| tail.to_string()),
            names: imported_names(node, module, source),
        });
        facts.import_statements += 1;
        return;
    }
    if module_text == "__future__" {
        return;
    }
    let names = imported_names(node, module, source);
    if module_text == "importlib" {
        // Bindings from importlib feed the bare-call wildcard arm.
        facts.importlib_bindings.extend(names.iter().cloned());
    }
    facts.imports.push(ImportForm::Absolute {
        specifier: module_text.to_string(),
        names,
    });
    facts.import_statements += 1;
}

/// The imported names of an `import_from_statement`: every named child that
/// is not the module (grammar-verified), `*` excluded, aliases resolved to
/// the bound name.
fn imported_names(node: &Node<'_>, module: Node<'_>, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.id() == module.id() {
            continue;
        }
        match child.kind() {
            "dotted_name" | "identifier" => names.push(node_text(&child, source).to_string()),
            "aliased_import" => {
                if let Some(name) = child.child_by_field_name("name") {
                    names.push(node_text(&name, source).to_string());
                }
            }
            // `wildcard_import` (`import *`) lists no name — the module
            // edge stands on its own.
            _ => {}
        }
    }
    names
}

/// The dynamic arm (P2): a call through `import_module` or
/// `spec_from_file_location` — attribute-rooted (`importlib.import_module`)
/// or bound through a file-local importlib binding. The site is recorded
/// with its 1-based line; NO edge, exit 0, the report line names it.
fn record_dynamic_call(node: &Node<'_>, source: &str, facts: &mut FileFacts) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    let matched = match function.kind() {
        "attribute" => {
            let name = function
                .child_by_field_name("attribute")
                .map(|part| node_text(&part, source).to_string())
                .unwrap_or_default();
            let mut object = function.child_by_field_name("object");
            let rooted = {
                let mut current = object.take();
                let mut text = String::new();
                while let Some(node) = current {
                    let chunk = node_text(&node, source);
                    text = if text.is_empty() {
                        chunk.to_string()
                    } else {
                        format!("{chunk}.{text}")
                    };
                    current = if node.kind() == "attribute" {
                        node.child_by_field_name("object")
                    } else {
                        None
                    };
                }
                text.starts_with("importlib")
            };
            rooted && is_importlib_site(&name)
        }
        "identifier" => {
            let name = node_text(&function, source).to_string();
            is_importlib_site(&name) && facts.importlib_bindings.contains(&name)
        }
        _ => false,
    };
    if matched {
        facts.dynamic_sites.push((
            function.start_position().row + 1,
            node_text(&function, source).to_string(),
        ));
    }
}

fn is_importlib_site(name: &str) -> bool {
    name == "import_module" || name == "spec_from_file_location"
}

/// The literal-candidate feed of one `.py` file (workplan events_value_join
/// scenario 09, decision-check F8 seam): vocabulary-free syntactic sites
/// carrying a string literal — every call site with a string-literal
/// argument (call anchor) and every string-literal value of a module-level
/// binding (exported-property anchor). NO declared name reaches this walk:
/// the callee's simple name and the binding's name ride as spelled; name
/// equality is the engine matcher verb's work at the resolution site
/// (`archspec::events::patterns`), so the driver owns no interpretation
/// (A6). The caller runs this only when a declared pattern names the
/// python driver — the declaration gate is the byte identity (4.2 arm 1).
/// A file that cannot be read or parsed states no candidate.
pub(crate) fn literal_candidates(path: &Path) -> Vec<(LiteralCandidateAnchor, String)> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    literal_candidates_source(&source)
}

/// The tree walk behind [`literal_candidates`], split out so the unit tests
/// feed source text directly (the `scan_python_source` precedent).
pub(crate) fn literal_candidates_source(source: &str) -> Vec<(LiteralCandidateAnchor, String)> {
    let Ok(tree) = parse_tree(Language::Python, source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_call_candidates(&tree.root_node(), source, &mut out);
    collect_module_property_candidates(&tree.root_node(), source, &mut out);
    out
}

/// The call-anchor walk: every `call` in the tree, wherever it sits (nested
/// calls each contribute their own candidates); one candidate per argument
/// position whose node is a plain string literal. Positions count ALL named
/// children of the argument list — a keyword argument occupies a spelled
/// position too (the ts enumeration law). The callee's simple name is the
/// spelled last name: a bare `identifier`, or the attribute of an
/// `attribute` target (`os.path.join` reads `join`).
fn collect_call_candidates<'a>(
    node: &Node<'a>,
    source: &'a str,
    out: &mut Vec<(LiteralCandidateAnchor, String)>,
) {
    if node.kind() == "call"
        && let (Some(callee), Some(arguments)) = (
            node.child_by_field_name("function")
                .and_then(|target| simple_callee_name(target, source)),
            node.child_by_field_name("arguments"),
        )
    {
        for (position, argument) in arguments.named_children(&mut arguments.walk()).enumerate() {
            if let Some(value) = string_literal_value(argument, source) {
                out.push((
                    LiteralCandidateAnchor::Call {
                        callee: callee.clone(),
                        position,
                    },
                    value,
                ));
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_call_candidates(&child, source, out);
    }
}

/// The simple name of a call target as the source spells it: a bare
/// `identifier` or the `attribute:` of an `attribute` chain (the LAST
/// segment — the name actually called). Subscripted and parenthesized
/// targets state no simple name this verb can read.
fn simple_callee_name<'a>(node: Node<'a>, source: &'a str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(&node, source).to_string()),
        "attribute" => node
            .child_by_field_name("attribute")
            .map(|attribute| node_text(&attribute, source).to_string()),
        _ => None,
    }
}

/// The exported-property-anchor walk: `assignment` statements that are
/// DIRECT children of the module's `expression_statement`s — the
/// module-level bindings. Function-body, class-body and conditional
/// assignments are not module surface and contribute nothing. One candidate
/// per assignment whose single `identifier` target is bound to a single
/// plain string literal; every other shape (tuple targets, subscripts,
/// attribute targets, list or call values) states no literal value for
/// this verb — a value nested in a collection is unmatched SHAPE, not a
/// gap.
fn collect_module_property_candidates<'a>(
    node: &Node<'a>,
    source: &'a str,
    out: &mut Vec<(LiteralCandidateAnchor, String)>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "expression_statement" {
            continue;
        }
        let mut statement_cursor = child.walk();
        for statement in child.named_children(&mut statement_cursor) {
            if statement.kind() != "assignment" {
                continue;
            }
            // The `left:` field is a single pattern; tuple and list targets
            // arrive as `pattern_list` and state no single name.
            let name = statement
                .child_by_field_name("left")
                .filter(|left| left.kind() == "identifier")
                .map(|name| node_text(&name, source).to_string());
            if statement.child_by_field_name("type").is_some() {
                continue;
            }
            let value = statement
                .child_by_field_name("right")
                .and_then(|value| string_literal_value(value, source));
            if let (Some(property), Some(value)) = (name, value) {
                out.push((
                    LiteralCandidateAnchor::ExportedProperty { property },
                    value,
                ));
            }
        }
    }
}

/// The value of a plain string-literal node: a `string` joins its content
/// fragments and decoded escapes IN ORDER (the ts `string_text` posture);
/// an interpolated (`f`-string) literal states no plain value and is
/// skipped. `concatenated_string`, calls, names — every other expression —
/// states no literal value.
fn string_literal_value(node: Node, source: &str) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    let mut text = String::new();
    let mut saw_content = false;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            // An interpolation (`f"…{x}…"`) makes the runtime value not a
            // literal (the kotlin interpolation law, same posture).
            "interpolation" => return None,
            "string_content" => {
                saw_content = true;
                text.push_str(node_text(&child, source));
            }
            "escape_sequence" => {
                saw_content = true;
                text.push_str(&decode_escape(node_text(&child, source)));
            }
            _ => {}
        }
    }
    if saw_content {
        return Some(text);
    }
    // Empty literal (`""`): the node text minus its quotes and any string
    // prefix letters, never the quoted raw (the ts fallback posture).
    let raw = node_text(&node, source);
    let open = raw.find(['"', '\''])?;
    let quote = raw.as_bytes()[open] as char;
    let inner = raw[open + 1..].strip_suffix(quote)?;
    Some(inner.to_string())
}

/// Decode the single-form Python escape sequences (the kotlin/ts subset:
/// the common control escapes plus quote and backslash); anything else
/// stays as spelled — exotic escapes (octal, `\u{…}`) are not this verb's
/// claim surface.
fn decode_escape(text: &str) -> String {
    match text {
        "\\n" => "\n".into(),
        "\\r" => "\r".into(),
        "\\t" => "\t".into(),
        "\\b" => "\u{8}".into(),
        "\\f" => "\u{c}".into(),
        "\\v" => "\u{b}".into(),
        "\\a" => "\u{7}".into(),
        "\\\"" => "\"".into(),
        "\\'" => "'".into(),
        "\\\\" => "\\".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ImportForm, scan_python_source};
    use std::collections::BTreeSet;

    #[test]
    fn absolute_forms_carry_module_and_names() {
        let source = "import a.b.c\nimport d.e as m\nfrom pkg.mod import x, y as z\nfrom pkg.star import *\n";
        let facts = scan_python_source(source).expect("parses");
        assert_eq!(
            facts.imports,
            vec![
                ImportForm::Absolute {
                    specifier: "a.b.c".into(),
                    names: vec![]
                },
                ImportForm::Absolute {
                    specifier: "d.e".into(),
                    names: vec![]
                },
                ImportForm::Absolute {
                    specifier: "pkg.mod".into(),
                    names: vec!["x".into(), "y".into()]
                },
                ImportForm::Absolute {
                    specifier: "pkg.star".into(),
                    names: vec![]
                },
            ],
            "aliases carry the path, never the alias name; a star lists nothing"
        );
        assert_eq!(facts.import_statements, 4);
    }

    #[test]
    fn relative_forms_carry_level_and_module() {
        let source = "from . import x\nfrom .mod import y\nfrom .. import z\nfrom ..p.q import w\n";
        let facts = scan_python_source(source).expect("parses");
        assert_eq!(
            facts.imports,
            vec![
                ImportForm::Relative {
                    level: 1,
                    module: None,
                    names: vec!["x".into()]
                },
                ImportForm::Relative {
                    level: 1,
                    module: Some("mod".into()),
                    names: vec!["y".into()]
                },
                ImportForm::Relative {
                    level: 2,
                    module: None,
                    names: vec!["z".into()]
                },
                ImportForm::Relative {
                    level: 2,
                    module: Some("p.q".into()),
                    names: vec!["w".into()]
                },
            ]
        );
    }

    #[test]
    fn future_import_carries_no_fact() {
        let source = "from __future__ import annotations\nimport os\n";
        let facts = scan_python_source(source).expect("parses");
        assert_eq!(
            facts.imports,
            vec![ImportForm::Absolute {
                specifier: "os".into(),
                names: vec![]
            }]
        );
        assert_eq!(facts.import_statements, 1, "the directive is not counted");
    }

    #[test]
    fn dynamic_sites_record_line_and_spelling() {
        let source = "import importlib\nimportlib.import_module(name)\nimportlib.util.spec_from_file_location(n, p)\n";
        let facts = scan_python_source(source).expect("parses");
        assert_eq!(
            facts.dynamic_sites,
            vec![
                (2, "importlib.import_module".to_string()),
                (3, "importlib.util.spec_from_file_location".to_string()),
            ]
        );
    }

    #[test]
    fn bound_importlib_names_enable_the_bare_call_arm() {
        let source = "from importlib import import_module\nimport_module(\"some.name\")\nimport_module(name)\n";
        let facts = scan_python_source(source).expect("parses");
        assert_eq!(facts.importlib_bindings, BTreeSet::from(["import_module".to_string()]));
        assert_eq!(
            facts.dynamic_sites,
            vec![(2, "import_module".to_string()), (3, "import_module".to_string())],
            "the literal-string arm rides the wildcard too (P2 literal-string clause)"
        );
    }

    #[test]
    fn calls_named_like_the_sites_without_importlib_are_quiet() {
        let source = "registry.import_module(name)\nimport_module(name)\n";
        let facts = scan_python_source(source).expect("parses");
        assert!(
            facts.dynamic_sites.is_empty(),
            "a foreign chain and an unbound name are no importlib site"
        );
    }

    #[test]
    fn damaged_tree_still_yields_located_imports() {
        let source = "import a.b\n\ndef broken(:\n";
        let facts = scan_python_source(source).expect("error-tolerant tree");
        assert!(facts.imports.contains(&ImportForm::Absolute {
            specifier: "a.b".into(),
            names: vec![]
        }));
    }

    #[test]
    fn indented_and_conditional_imports_are_located() {
        // The measured pilot shapes: function-body imports (310 sites) and
        // `except ImportError` guards — the try-body import stays static.
        let source = "def f():\n    from pkg.mod import x\ntry:\n    import optional_dep\nexcept ImportError:\n    optional_dep = None\n";
        let facts = scan_python_source(source).expect("parses");
        assert!(facts.imports.contains(&ImportForm::Absolute {
            specifier: "pkg.mod".into(),
            names: vec!["x".into()]
        }));
        assert!(facts.imports.contains(&ImportForm::Absolute {
            specifier: "optional_dep".into(),
            names: vec![]
        }));
    }
}
