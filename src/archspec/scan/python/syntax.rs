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

use crate::archspec::language::Language;
use crate::archspec::parse::grammar::{node_text, parse_tree};

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
