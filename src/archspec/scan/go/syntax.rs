//! Grammar source facts for Go extraction: the source-fact half of the go
//! driver (intra-module package edges US 11,
//! selector symbols US 12).
//!
//! Parses each `.go` file through the in-binary tree-sitter grammar
//! ([`crate::archspec::parse::grammar`]) and produces the import and selector
//! facts the shared scan layer feeds into its attribution logic — module-path
//! prefix, stdlib list and the discovered package set are shared rules; the
//! facts enter through grammar `import_spec` nodes, and the module-tier
//! projection lives in [`super`].
//!
//! Grammar node kinds consumed (tree-sitter-go 0.23.4, verified against
//! `node-types.json` and a node dump of every import and selector shape):
//! - `source_file` — root; children include `package_clause` and any number
//!   of `import_declaration`s (one per single spec or block).
//! - `package_clause` — the package name is a DIRECT `package_identifier`
//!   child (the grammar defines no field for it — verified dump).
//! - `import_declaration` — exactly one child: `import_spec` (single form
//!   `import "path"`) or `import_spec_list` (block form `import ( ... )`,
//!   children `import_spec` each).
//! - `import_spec` — field `path:` (required, `interpreted_string_literal`
//!   or `raw_string_literal` — the path is the node text minus its quotes);
//!   field `name:` (optional alias: `package_identifier`, or the named leaves
//!   `blank_identifier` (`_`) and `dot` (`.`) — all three read as their raw
//!   text).
//! - `selector_expression` — fields `operand:` (any expression) and `field:`
//!   (an `identifier` ALIASED to `field_identifier` — `_field_identifier` in
//!   the grammar). Chained access nests: `format.Title().String()` parses as
//!   `selector(operand: call(selector(operand: identifier, field)), field)`
//!   (verified dump), so requiring an `identifier` operand both anchors the
//!   qualifier lookup and excludes method-on-value and call-tail noise.
//!   Qualified TYPE references are NOT selectors — they parse as
//!   `qualified_type` nodes (fields `package:`/`name:`, kinds
//!   `package_identifier`/`type_identifier`) and struct-literal field keys as
//!   `keyed_element` `key:` literal elements (plain `identifier`), so
//!   `format.Options{Title: …}` contributes no selector at all.
//!
//! The cgo meta-import `import "C"` is NOT special-cased here: a bare `C`
//! path has no dotted first
//! segment, so the shared stdlib rule classifies it as stdlib and it
//! contributes neither an edge nor an external fact — reached through the
//! shared rule instead of a second rule.
//!
//! Per-file alias map: import path → the LOCAL qualifier the file uses for
//! it — the alias token when present (`f`, `_` or `.`) and the import path's
//! last segment otherwise (Go's default package name convention). US 12's
//! selector attribution resolves an import through this map: the exported
//! selector fields recorded under the import's qualifier become the symbols of
//! the edge that import created. The `_` (blank import) and `.` (dot import)
//! qualifiers are deliberately NOT referenceable: a blank import binds no
//! name (Go forbids selecting through `_`), and a dot import's exported names
//! appear as bare identifiers indistinguishable from local ones — both
//! contribute no symbols, exactly like the csharp bare-name rule keeps
//! unanchored identifiers out. The scan layer (`super`) owns that resolution;
//! this module only records the raw selector facts keyed by operand text.
//!
//! Attribution identity note: a package's IDENTITY
//! is its directory-derived import path ([`super::package_name`]), not the
//! `package_clause` name — the clause is a source fact recorded, never an
//! addressing input. `*_test.go` files never reach
//! this module at all: [`super::collect_packages`] excludes the test tier
//! (the shared inclusion rule).
//!
//! Error tolerance: identical to the C# side — the tree-sitter tree survives
//! broken constructs, so a file with `ERROR`/`MISSING` nodes still contributes
//! every import the parser located; there is deliberately NO alternate parse
//! path when [`crate::archspec::parse::grammar::first_error_range`]
//! would report damage, and only a file tree-sitter cannot parse at all yields
//! an error, propagated to the scan failure.

use tree_sitter::Node;

use crate::archspec::language::Language;
use crate::archspec::parse::grammar::{node_text, parse_tree};

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Grammar facts of one `.go` file (see the module docs for what feeds the
/// edge assembly and the selector attribution).
#[derive(Clone, Debug, Default)]
pub(crate) struct FileFacts {
    /// The `package_clause` name (e.g. `store`). Identity stays
    /// directory-derived in the scan layer; this is the declared clause,
    /// recorded as a source fact and pinned by the unit tests below.
    #[allow(dead_code)] // declared-clause fact only — never an addressing or symbol input
    pub(crate) package: Option<String>,
    /// Every imported path (deduplicated across specs and blocks; the cgo
    /// marker `"C"` included — the shared stdlib rule absorbs it).
    pub(crate) imports: BTreeSet<String>,
    /// Import path → local qualifier (alias / `_` / `.` / path last segment).
    /// The selector attribution keys on this map (US 12).
    pub(crate) qualifiers: BTreeMap<String, String>,
    /// Qualifier text → exported (uppercase-first) selector field names used
    /// with it anywhere in the file. A RAW source fact: operands that are
    /// local variables (e.g. `v.Method()`) are recorded under their text too —
    /// attribution keeps only the entries whose text is one of this file's
    /// qualifiers, so the grammar layer stays decision-free.
    pub(crate) selectors: BTreeMap<String, BTreeSet<String>>,
}

/// Read and parse one `.go` file, returning its grammar facts. The only
/// error paths are an unreadable file or a tree-sitter refusal.
pub(crate) fn scan_go_file(path: &Path) -> Result<FileFacts, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    scan_go_source(&raw)
}

/// Collects import paths, the
/// alias map and the selector facts from one source's syntax tree (document
/// order is irrelevant — the results are maps keyed by path or qualifier).
pub(crate) fn scan_go_source(source: &str) -> Result<FileFacts, String> {
    let tree = parse_tree(Language::Go, source).map_err(|err| err.to_string())?;
    let mut facts = FileFacts::default();
    collect(&tree.root_node(), source, &mut facts);
    Ok(facts)
}

/// Preorder walk: `package_clause` records the clause name (and is not
/// descended), `import_spec` records path + qualifier (and is not descended),
/// `selector_expression` records its qualified exported field and IS descended
/// (chained selectors nest deeper in `operand:`), everything else — including
/// `import_declaration` / `import_spec_list` and trees damaged into `ERROR`
/// wrappers — recurses by position so specs the parser located under a damaged
/// parent still count.
fn collect(node: &Node<'_>, source: &str, facts: &mut FileFacts) {
    match node.kind() {
        "package_clause" => {
            if let Some(name) = node.named_child(0) {
                facts.package = Some(node_text(&name, source).to_string());
            }
            return;
        }
        "import_spec" => {
            record_spec(node, source, facts);
            return;
        }
        "selector_expression" => {
            record_selector(node, source, facts);
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(&child, source, facts);
    }
}

/// One `import_spec`: the `path:` literal minus quotes (escape sequences stay
/// raw — a byte copy, and no import path in any
/// fixture uses one) joins the import set, and the qualifier rule of the
/// module docs joins the alias map. A spec without a literal (a `MISSING`
/// placeholder in a damaged tree) contributes nothing.
fn record_spec(spec: &Node<'_>, source: &str, facts: &mut FileFacts) {
    let Some(path_node) = spec.child_by_field_name("path") else {
        return;
    };
    let path = literal_value(node_text(&path_node, source));
    if path.is_empty() {
        return;
    }
    let qualifier = match spec.child_by_field_name("name") {
        Some(name) => node_text(&name, source).to_string(),
        None => path.rsplit('/').next().unwrap_or(&path).to_string(),
    };
    facts.imports.insert(path.clone());
    facts.qualifiers.insert(path, qualifier);
}

/// One `selector_expression` (fields `operand:`/`field:`, grammar-verified):
/// only an `operand` of kind `identifier` can name a package qualifier —
/// chained access nests `call_expression`/`selector_expression` operands, the
/// method-on-value noise guard falls out of this single test. The `field:`
/// (always a `field_identifier` by the grammar's `_field_identifier` alias —
/// qualified TYPES are separate `qualified_type` nodes) joins the operand's
/// selector map when its FIRST character is uppercase: Go's export rule is
/// casing, no configuration (workplan decision "Symbol vocabulary").
fn record_selector(node: &Node<'_>, source: &str, facts: &mut FileFacts) {
    let Some(operand) = node.child_by_field_name("operand") else {
        return;
    };
    if operand.kind() != "identifier" {
        return;
    }
    let Some(field) = node.child_by_field_name("field") else {
        return;
    };
    if field.kind() != "field_identifier" {
        return;
    }
    let name = node_text(&field, source);
    if !name.chars().next().is_some_and(char::is_uppercase) {
        return;
    }
    facts
        .selectors
        .entry(node_text(&operand, source).to_string())
        .or_default()
        .insert(name.to_string());
}

/// Strip the delimiters of a Go string literal node text (`"path"` or
/// `` `path` ``).
fn literal_value(text: &str) -> String {
    text.strip_prefix(['"', '`'])
        .and_then(|inner| inner.strip_suffix(['"', '`']))
        .unwrap_or(text)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::scan_go_source;
    use std::collections::{BTreeMap, BTreeSet};

    /// The selector-fact map from (qualifier text, exported names).
    fn selector_map(entries: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
        entries
            .iter()
            .map(|(qualifier, names)| {
                (
                    qualifier.to_string(),
                    names.iter().map(|n| n.to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn selectors_record_exported_field_names_per_operand_identifier() {
        // Raw source fact: EVERY identifier-qualified selector with an
        // exported (uppercase-first) field is recorded under its operand
        // text — local variables (`v`) included; the attribution layer later
        // keeps only operands that are file qualifiers.
        let source = "package store\n\nimport (\n\t\"example.com/demo/format\"\n\tf \"example.com/demo/f2\"\n\t\"strings\"\n)\n\nfunc Save() {\n\tformat.Title()\n\tformat.helper()\n\tf.Rename()\n\tv.Method()\n\tstrings.Join(nil, \"\")\n}\n";
        let facts = scan_go_source(source).expect("parses");
        assert_eq!(
            facts.selectors,
            selector_map(&[
                ("format", &["Title"][..]),
                ("f", &["Rename"]),
                ("strings", &["Join"]),
                ("v", &["Method"]),
            ]),
            "Title/Rename/Join/Method recorded, the lowercase `helper` never enters the map"
        );
    }

    #[test]
    fn chained_selectors_record_only_the_identifier_operand_layer() {
        // `format.Title().String()`: the trailing selector's `operand:` is a
        // `call_expression` (verified node dump) — skipped by the identifier
        // test — while recursion still reaches the inner qualified selector.
        // `v.Method()` (a local value) records under `v`, invisible to the
        // attribution.
        let source = "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n\tformat.Title().String()\n\tv.Method()\n}\n";
        let facts = scan_go_source(source).expect("parses");
        assert_eq!(
            facts.selectors,
            selector_map(&[("format", &["Title"][..]), ("v", &["Method"])]),
            "`String` is nowhere in the map — its operand is not an identifier"
        );
    }

    #[test]
    fn facts_collect_paths_from_every_import_shape() {
        let source = "package store\n\nimport (\n\t\"example.com/demo/format\"\n\tf \"example.com/demo/f2\"\n\t_ \"example.com/demo/blank\"\n\t. \"example.com/demo/dot\"\n)\n\nimport \"example.com/demo/single\"\n";
        let facts = scan_go_source(source).expect("parses");
        assert_eq!(
            facts.package.as_deref(),
            Some("store"),
            "the package_clause name is a direct child, not a field"
        );
        assert_eq!(
            facts.imports,
            [
                "example.com/demo/blank",
                "example.com/demo/dot",
                "example.com/demo/f2",
                "example.com/demo/format",
                "example.com/demo/single"
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<String>>(),
            "block specs, aliased, blank, dot and single forms all yield paths"
        );
    }

    #[test]
    fn qualifier_map_prefers_the_alias_and_defaults_to_the_path_tail() {
        let source = "package store\n\nimport (\n\tf \"example.com/demo/format\"\n\t_ \"example.com/demo/blank\"\n\t. \"example.com/demo/dot\"\n\t\"example.com/demo/plain\"\n\t\"C\"\n)\n";
        let facts = scan_go_source(source).expect("parses");
        assert_eq!(
            facts.qualifiers,
            [
                ("example.com/demo/format".to_string(), "f".to_string()),
                ("example.com/demo/blank".to_string(), "_".to_string()),
                ("example.com/demo/dot".to_string(), ".".to_string()),
                ("example.com/demo/plain".to_string(), "plain".to_string()),
                ("C".to_string(), "C".to_string()),
            ]
            .into_iter()
            .collect::<BTreeMap<String, String>>(),
            "the selector attribution keys on this map"
        );
    }

    #[test]
    fn duplicate_specs_collapse_and_cgo_marker_survives_into_facts() {
        let source = "package store\n\nimport (\n\t\"example.com/demo/format\"\n\t\"example.com/demo/format\"\n)\n\nimport \"example.com/demo/format\"\n\nimport \"C\"\n";
        let facts = scan_go_source(source).expect("parses");
        assert_eq!(
            facts.imports,
            ["C".to_string(), "example.com/demo/format".to_string()]
                .into_iter()
                .collect::<BTreeSet<String>>(),
            "dedup by path; the cgo `C` stays a fact — the shared stdlib rule, \
             not a second drop rule, keeps it out of every tier"
        );
    }

    #[test]
    fn damaged_tree_still_yields_located_imports() {
        // Unterminated func body — an ERROR region the grammar recovers from;
        // the located package clause and specs still contribute (no fallback).
        let source = "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n";
        let facts = scan_go_source(source).expect("error-tolerant tree");
        assert_eq!(
            facts.imports,
            ["example.com/demo/format".to_string()].into_iter().collect::<BTreeSet<String>>()
        );
        assert_eq!(facts.package.as_deref(), Some("store"));
    }

    #[test]
    fn qualified_types_and_struct_keys_are_not_selector_nodes() {
        // Grammar (verified node dump): a qualified type is a `qualified_type`
        // node (`package:` package_identifier, `name:` type_identifier) and a
        // struct literal key is a `keyed_element` `key:` literal_element —
        // neither is a `selector_expression`, so nothing is recorded even
        // though both spell `format.Options` / `Title`.
        let source = "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n\to := format.Options{Title: 1}\n\tvar s format.Options\n\t_, _ = o, s\n}\n";
        let facts = scan_go_source(source).expect("parses");
        assert!(
            facts.selectors.is_empty(),
            "selectors: {:?} — qualified type references and struct field \
             keys are grammar-disjoint from selectors",
            facts.selectors
        );
    }

    #[test]
    fn selector_casing_uses_the_first_character_and_honours_unicode() {
        // Go identifiers are unicode; export = first CHAR uppercase (not the
        // first byte / ASCII-only test).
        let source = "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n\tformat.Ähnlich()\n\tformat.ąd()\n\tformat._Padded()\n}\n";
        let facts = scan_go_source(source).expect("parses");
        assert_eq!(
            facts.selectors,
            selector_map(&[("format", &["Ähnlich"][..])]),
            "Ähnlich exported; ąd lowercase; `_Padded` starts with `_` (not \
             uppercase) — both unexported"
        );
    }

    #[test]
    fn blank_import_source_records_no_selectors_at_all() {
        // `_ "path"` binds no name: legal code around it cannot reference the
        // package, so no selector fact can ever exist for a blank import.
        let source = "package store\n\nimport _ \"example.com/demo/regist\"\n\nfunc Save() { _ = 1 }\n";
        let facts = scan_go_source(source).expect("parses");
        assert!(
            facts.selectors.is_empty(),
            "selectors: {:?} — a blank import is not referenceable",
            facts.selectors
        );
    }

    #[test]
    fn file_without_package_clause_still_yields_imports() {
        // No package clause at all (not compilable Go); the grammar reports
        // damage but the spec it locates still counts — the `import` keyword
        // is read regardless of the missing clause, so the tiers cannot drift.
        let source = "import \"example.com/demo/format\"\n";
        let facts = scan_go_source(source).expect("error-tolerant tree");
        assert_eq!(
            facts.imports,
            ["example.com/demo/format".to_string()].into_iter().collect::<BTreeSet<String>>()
        );
        assert_eq!(facts.package, None);
    }
}
