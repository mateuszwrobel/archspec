//! ts/JS per-file fact extraction (tree-sitter). One walk per parsed text
//! (a `.ts`/`.tsx`/`.js`/`.jsx` file, or one `<script>` block of a `.svelte`
//! component) yields the specifier facts the scan assembly consumes:
//! import specifiers with their value/type split, export-from specifiers,
//! dynamic import specifiers (literal or computed), and the file's public
//! export names.
//!
//! The fact vocabulary maps the grammar's node kinds (ADR-022): a statement
//! `type` child marks the whole import/export type-only; an `import_specifier`
//! carrying its own `type` modifier splits the statement across the two
//! edge kinds. A `call_expression` whose function node is the grammar's
//! `import` is a dynamic import — a single `string` argument is literal,
//! anything else is the non-literal wildcard case named in D2.

use crate::archspec::parse::grammar;
use std::path::Path;
use tree_sitter::Node;

/// One specifier's imported names, split by kind.
#[derive(Debug, Default)]
struct SpecifierFacts {
    /// Value-side imported names (original/exported names; `default` for a
    /// default import, `*` for a namespace import).
    value_symbols: Vec<String>,
    /// Type-side imported names (an `import type` clause or `type`-modified
    /// specifiers).
    type_symbols: Vec<String>,
}

/// One resolvable specifier seen by the walk (import, side-effect import,
/// export-from or literal dynamic import) and the symbol rows it carries.
#[derive(Debug)]
pub(crate) struct EdgeSpec {
    pub specifier: String,
    pub value_symbols: Vec<String>,
    pub type_symbols: Vec<String>,
    /// Statement-level `type` (or an `export type ... from` row): the edge
    /// is type-only even where no per-specifier modifier marks it.
    pub type_only: bool,
    /// True for `export ... from` statements (the export-from family, D2):
    /// the row republishes the target's items, it does not consume them.
    /// The root-barrel predicate (C4) reads this split; every other row
    /// (import, side-effect, dynamic) is consumption, never republication.
    pub export_from: bool,
}

/// The facts of one parsed text.
#[derive(Debug, Default)]
pub(crate) struct FileFacts {
    pub edges: Vec<EdgeSpec>,
    /// Verbatim spellings of non-literal dynamic-import and computed
    /// require specifiers.
    pub wildcards: Vec<String>,
    /// Names the file exports (D9 public-API facts).
    pub exports: Vec<String>,
    /// Import-like statements seen (import, export-from, dynamic import) —
    /// the tripwire's counter (D2).
    pub import_statements: usize,
}

/// Parses one source text with the grammar for `path` and collects the
/// specifier facts. Error-tolerant like the go/csharp syntax scanners: a
/// damaged tree still yields the specifiers it can locate. `.svelte` files
/// arrive block-split through [`svelte_script_blocks`].
pub(crate) fn scan_file(path: &Path, source: &str) -> Result<FileFacts, String> {
    let grammar = grammar::ts_grammar_for(path)
        .ok_or_else(|| format!("no ts grammar for file: {}", path.display()))?;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar)
        .map_err(|err| format!("tree-sitter failed to parse: {err}"))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "parser returned no tree".to_string())?;
    let mut facts = FileFacts::default();
    collect(&tree.root_node(), source.as_bytes(), &mut facts);
    Ok(facts)
}

fn node_text<'a>(node: &Node, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or_default()
}

fn children<'a>(node: &Node<'a>) -> Vec<Node<'a>> {
    node.children(&mut node.walk()).collect()
}

fn child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    children(&node).into_iter().find(|c| c.kind() == kind)
}

fn has_direct_child(node: Node, kind: &str) -> bool {
    children(&node).iter().any(|c| c.kind() == kind)
}

/// The unescaped text of a specifier's string node: fragments and escape
/// sequences join IN ORDER, with escapes decoded to their runtime value —
/// a bundler virtual id spelled `"\0virtual:module"` decodes to a NUL-LEADING
/// specifier, which the virtual-specifier law (round 4 C) tests by name.
/// A damaged tree without a single content child falls back to the whole
/// node text minus its enclosing quotes, never the quoted raw.
fn string_text(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "string" => {
            let mut text = String::new();
            let mut saw_content = false;
            for child in children(&node) {
                match child.kind() {
                    "string_fragment" => {
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
            // Damaged tree without any content child (a raw NUL byte inside
            // the literal starves the fragment lexer): state the string's
            // text minus its enclosing quotes, never the quoted raw.
            let raw = node_text(&node, source);
            let inner = raw.strip_prefix(['"', '\'', '`']).unwrap_or(raw);
            Some(
                inner
                    .strip_suffix(['"', '\'', '`'])
                    .unwrap_or(inner)
                    .to_string(),
            )
        }
        _ => None,
    }
}

/// One JS escape sequence decoded (ECMA-262 StringLiteralsNoLineTerminators
/// subset): the single-char forms, NUL (`\0` NOT followed by a digit),
/// `\xHH`, `\uHHHH`, `\u{...}`, and identity escapes for anything else.
fn decode_escape(escape: &str) -> String {
    let Some(rest) = escape.strip_prefix('\\') else {
        return escape.to_string();
    };
    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    match first {
        'n' => "\n".to_string(),
        'r' => "\r".to_string(),
        't' => "\t".to_string(),
        'b' => "\u{8}".to_string(),
        'f' => "\u{c}".to_string(),
        'v' => "\u{b}".to_string(),
        '0' => {
            // \0 alone is NUL; \0 followed by a digit is an octal form the
            // drivers never model — kept verbatim.
            if rest.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) {
                escape.to_string()
            } else {
                "\u{0}".to_string()
            }
        }
        'x' => {
            let hex: String = rest.chars().skip(1).take(2).collect();
            u32::from_str_radix(&hex, 16)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| escape.to_string(), |c| c.to_string())
        }
        'u' => {
            let digits: String = rest.chars().skip(1).collect();
            let hex = digits.strip_prefix('{').unwrap_or(&digits);
            let hex = hex.strip_suffix('}').unwrap_or(hex);
            u32::from_str_radix(hex, 16)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| escape.to_string(), |c| c.to_string())
        }
        '\n' | '\r' => String::new(), // line continuation contributes nothing
        other => other.to_string(),   // identity escape: \/ \" \' \\ etc.
    }
}

/// The `from` clause source node of an import/export statement, or the
/// direct string of a side-effect import.
fn source_node<'a>(statement: Node<'a>) -> Option<Node<'a>> {
    statement
        .child_by_field_name("source")
        .or_else(|| child_of_kind(statement, "string"))
}

/// Names imported by one `import_clause`: original names on the value side,
/// `type`-modified specifiers on the type side; `default` and `*` land on
/// the value side (an `import type` statement marks the whole clause at the
/// caller).
fn collect_import_clause(clause: Node, source: &[u8], facts: &mut SpecifierFacts) {
    for child in children(&clause) {
        match child.kind() {
            "identifier" => facts.value_symbols.push("default".to_string()),
            "namespace_import" => facts.value_symbols.push("*".to_string()),
            "named_imports" => {
                for spec in children(&child) {
                    if spec.kind() != "import_specifier" {
                        continue;
                    }
                    let name = spec
                        .child_by_field_name("name")
                        .or_else(|| {
                            children(&spec)
                                .into_iter()
                                .find(|c| matches!(c.kind(), "identifier" | "string"))
                        })
                        .map(|n| node_text(&n, source).to_string())
                        .unwrap_or_default();
                    let side = if has_direct_child(spec, "type") {
                        &mut facts.type_symbols
                    } else {
                        &mut facts.value_symbols
                    };
                    side.push(name);
                }
            }
            _ => {}
        }
    }
}

fn collect_declaration_exports(node: Node, source: &[u8], facts: &mut FileFacts) {
    let named = matches!(
        node.kind(),
        "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "lexical_declaration"
    );
    if !named {
        return;
    }
    if let Some(name) = node.child_by_field_name("name") {
        facts.exports.push(node_text(&name, source).to_string());
        return;
    }
    for child in children(&node) {
        if child.kind() == "variable_declarator"
            && let Some(name) = child.child_by_field_name("name")
        {
            facts.exports.push(node_text(&name, source).to_string());
        }
    }
}

fn collect_export_clause(clause: Node, source: &[u8], facts: &mut FileFacts) -> Vec<String> {
    let mut names = Vec::new();
    for spec in children(&clause) {
        if spec.kind() != "export_specifier" {
            continue;
        }
        // The public name of a re-export is the alias, else the local name;
        // the local name is what the target file states, never invented.
        let public = spec
            .child_by_field_name("alias")
            .or_else(|| spec.child_by_field_name("name"))
            .map(|n| node_text(&n, source).to_string())
            .unwrap_or_default();
        if !public.is_empty() {
            names.push(public);
        }
    }
    facts.exports.extend(names.iter().cloned());
    names
}

fn collect_import_statement(node: Node, source: &[u8], facts: &mut FileFacts) {
    let Some(spec) = source_node(node).and_then(|s| string_text(s, source)) else {
        return;
    };
    let type_only = has_direct_child(node, "type");
    let mut spec_facts = SpecifierFacts::default();
    if let Some(clause) = child_of_kind(node, "import_clause") {
        collect_import_clause(clause, source, &mut spec_facts);
    }
    if type_only {
        spec_facts.type_symbols.append(&mut spec_facts.value_symbols);
    }
    facts.import_statements += 1;
    facts.edges.push(EdgeSpec {
        specifier: spec,
        value_symbols: spec_facts.value_symbols,
        type_symbols: spec_facts.type_symbols,
        type_only,
        export_from: false,
    });
}

fn collect_export_statement(node: Node, source: &[u8], facts: &mut FileFacts) {
    let type_only = has_direct_child(node, "type");
    let mut public_names = Vec::new();
    if let Some(clause) = child_of_kind(node, "export_clause") {
        public_names = collect_export_clause(clause, source, facts);
    }
    if let Some(spec) = source_node(node).and_then(|s| string_text(s, source)) {
        // export-from: an edge (D2). `export *` carries the wildcard symbol;
        // an export clause carries its public names.
        let mut symbols = public_names;
        if has_direct_child(node, "*") {
            symbols.push("*".to_string());
        }
        facts.import_statements += 1;
        facts.edges.push(EdgeSpec {
            specifier: spec,
            value_symbols: if type_only { Vec::new() } else { symbols.clone() },
            type_symbols: if type_only { symbols } else { Vec::new() },
            type_only,
            export_from: true,
        });
        return;
    }
    // Export without a source is a public-API fact, never an edge.
    if has_direct_child(node, "default") {
        facts.exports.push("default".to_string());
        return;
    }
    if !public_names.is_empty() {
        return; // a plain `export { ... }` clause was already recorded.
    }
    for child in children(&node) {
        collect_declaration_exports(child, source, facts);
    }
}

fn collect_dynamic_import(node: Node, source: &[u8], facts: &mut FileFacts) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    if function.kind() != "import" {
        return;
    }
    let Some(arguments) = node.child_by_field_name("arguments") else {
        return;
    };
    let args: Vec<Node> = arguments
        .named_children(&mut arguments.walk())
        .collect();
    facts.import_statements += 1;
    if args.len() == 1
        && let Some(literal) = string_text(args[0], source)
    {
        // A literal dynamic specifier is an ordinary edge (D2).
        facts.edges.push(EdgeSpec {
            specifier: literal,
            value_symbols: Vec::new(),
            type_symbols: Vec::new(),
            type_only: false,
            export_from: false,
        });
        return;
    }
    // The non-literal case (D2): a named wildcard fact, verbatim spelling —
    // the argument's source text, parentheses excluded.
    let expression = args
        .first()
        .or(args.first().or(Some(&arguments)))
        .map(|n| node_text(n, source))
        .unwrap_or_default();
    facts.wildcards.push(expression.trim().to_string());
}

/// The require arm (workplan workplan_archspec_require_tracing D1-D4/D6/D7):
/// the CommonJS sibling of the dynamic-import arm above. A site is the
/// `require` identifier CALLED — a property access (`require.resolve`,
/// `require.cache`, `require.main`) or a member call (`jest.requireActual`,
/// `jest.requireMock`) is a different function and states nothing (D6). One
/// string-literal argument is an ordinary value edge with NO symbols and
/// never republication (D1/D4); any other argument is the same wildcard
/// census fact the dynamic arm states (D2). Every site increments the
/// tripwire counter before the split, exactly like the dynamic arm (D7).
fn collect_require_call(node: Node, source: &[u8], facts: &mut FileFacts) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    if function.kind() != "identifier" || node_text(&function, source) != "require" {
        return;
    }
    let Some(arguments) = node.child_by_field_name("arguments") else {
        return;
    };
    let args: Vec<Node> = arguments
        .named_children(&mut arguments.walk())
        .collect();
    facts.import_statements += 1;
    if args.len() == 1
        && let Some(literal) = string_text(args[0], source)
    {
        // A literal require is an ordinary value edge (D1) — the resolution
        // seam below decides edge/external/fail-loud exactly as for imports.
        facts.edges.push(EdgeSpec {
            specifier: literal,
            value_symbols: Vec::new(),
            type_symbols: Vec::new(),
            type_only: false,
            export_from: false,
        });
        return;
    }
    // The computed case (D2): the existing wildcard census, verbatim
    // spelling — the argument's source text, parentheses excluded.
    let expression = args
        .first()
        .or(Some(&arguments))
        .map(|n| node_text(n, source))
        .unwrap_or_default();
    facts.wildcards.push(expression.trim().to_string());
}

fn collect(node: &Node, source: &[u8], facts: &mut FileFacts) {
    match node.kind() {
        "import_statement" => collect_import_statement(*node, source, facts),
        "export_statement" => collect_export_statement(*node, source, facts),
        "call_expression" => {
            collect_dynamic_import(*node, source, facts);
            collect_require_call(*node, source, facts);
        }
        _ => {}
    }
    for child in children(node) {
        collect(&child, source, facts);
    }
}

/// The `<script>` blocks of a Svelte component: every block, plain and
/// `context="module"` alike, parsed by the caller with the TypeScript
/// grammar (ADR-022). Markup between blocks renders nothing and states
/// nothing — the driver claims no template knowledge.
pub(crate) fn svelte_script_blocks(source: &str) -> Vec<String> {
    const CLOSE: &str = "</script>";
    let mut blocks = Vec::new();
    let mut offset = 0;
    while let Some(at) = find_open_tag(source, offset) {
        let Some(tag_close) = source[at..].find('>').map(|close| at + close + 1) else {
            break; // unterminated tag: the file is damaged, state nothing.
        };
        let block_end = source[tag_close..]
            .find(CLOSE)
            .map(|close| tag_close + close)
            .unwrap_or(source.len());
        blocks.push(source[tag_close..block_end].to_string());
        offset = block_end + CLOSE.len();
    }
    blocks
}

fn find_open_tag(source: &str, from: usize) -> Option<usize> {
    let mut at = source[from..].find("<script")? + from;
    loop {
        let next = at + "<script".len();
        match source.get(next..next + 1).map(|s| s.as_bytes()) {
            // `<script>` and `<script attr…` both open a block; `<scriptfoo`
            // is not the tag.
            Some(b">") | Some(b" ") | Some(b"\n") | Some(b"\t") | Some(b"\r") => return Some(at),
            _ => {
                at = source[next..].find("<script")? + next;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_escape, scan_file, FileFacts};
    use std::path::Path;

    fn facts_for(name: &str, source: &str) -> FileFacts {
        scan_file(Path::new(name), source).expect("the grammar parses the fixture")
    }

    /// Phase 1 (D1/D2/D4/D7): the require arm's FileFacts split — a literal
    /// `require("lit")` is an ordinary value edge with NO symbols and
    /// export_from=false; a computed or template `require(expr)` is the
    /// existing wildcard census fact with its verbatim spelling; every site
    /// increments the tripwire counter.
    #[test]
    fn literal_requires_edge_computed_requires_wildcard() {
        let facts = facts_for(
            "a.cjs",
            "const base = require('./jest.config.base.cjs');\n\
             const w = require(workerData.modulePath);\n\
             const t = require(`./${name}`);\n\
             const { a, b } = require('./destructured');\n",
        );
        let mut specifiers: Vec<String> =
            facts.edges.iter().map(|edge| edge.specifier.clone()).collect();
        specifiers.sort();
        assert_eq!(
            specifiers,
            vec!["./destructured".to_string(), "./jest.config.base.cjs".to_string()],
            "the two literal requires are ordinary value edges"
        );
        for edge in &facts.edges {
            assert!(edge.value_symbols.is_empty(), "a require edge states no symbols (D4): {edge:?}");
            assert!(edge.type_symbols.is_empty(), "a require is never type-only: {edge:?}");
            assert!(!edge.type_only, "a require is a value load (D1): {edge:?}");
            assert!(!edge.export_from, "a require never republishes (D4): {edge:?}");
        }
        assert_eq!(
            facts.wildcards,
            vec!["workerData.modulePath".to_string(), "`./${name}`".to_string()],
            "the computed and template arguments ride the wildcard census verbatim (D2)"
        );
        assert_eq!(facts.import_statements, 4, "every require site counts as an import-like statement (D7)");
    }

    /// Phase 1 (D4, stated law with a synthetic fixture): `module.exports =
    /// require('./x')` is a consumption edge, never an export-from row.
    #[test]
    fn republication_require_is_a_value_edge_not_an_export_from() {
        let facts = facts_for("i.cjs", "module.exports = require('./reexported');\n");
        assert_eq!(facts.edges.len(), 1);
        assert_eq!(facts.edges[0].specifier, "./reexported");
        assert!(!facts.edges[0].export_from, "the republication shape stays a consumption edge (D4)");
        assert!(facts.edges[0].value_symbols.is_empty());
    }

    /// Phase 1 (D6): the NON-site forms — `require.resolve`, `require.cache`,
    /// `jest.requireActual`, `jest.requireMock` — state no edge, no wildcard,
    /// no counter increment: the site is the `require` identifier CALLED.
    #[test]
    fn require_like_sites_that_are_not_loads_state_nothing() {
        let facts = facts_for(
            "a.ts",
            "const p = require.resolve('x');\n\
             if (require.cache[k]) {}\n\
             jest.requireActual('./x');\n\
             jest.requireMock('./y');\n\
             const main = require.main;\n",
        );
        assert!(facts.edges.is_empty(), "a property access or jest helper call is not a require site (D6)");
        assert!(facts.wildcards.is_empty(), "a non-site states no census fact");
        assert_eq!(facts.import_statements, 0, "a non-site is not an import-like statement (D7)");
    }

    /// Phase 1 (D3, the N2 rebuttal shipped as a test): `require('...')` text
    /// inside string content never parses as a call_expression, so it states
    /// no fact at all.
    #[test]
    fn string_content_requires_state_nothing() {
        let facts = facts_for(
            "a.mjs",
            "const out = execSync(`node -p \"require('./package.json').version\"`);\n\
             const doc = 'run require(\"./x\") to load it';\n\
             export { out, doc };\n",
        );
        assert!(facts.edges.is_empty(), "string bodies are string content, never call_expression nodes");
        assert!(facts.wildcards.is_empty());
        assert_eq!(facts.import_statements, 0);
    }

    #[test]
    fn escape_sequences_decode_to_runtime_values() {
        // The virtual-module spelling: \0 decodes to a leading NUL.
        assert_eq!(decode_escape("\\0"), "\u{0}");
        assert_eq!(decode_escape("\\x00"), "\u{0}");
        assert_eq!(decode_escape("\\u0000"), "\u{0}");
        assert_eq!(decode_escape("\\u{1f600}"), "\u{1f600}");
        // The ordinary forms.
        assert_eq!(decode_escape("\\n"), "\n");
        assert_eq!(decode_escape("\\\\"), "\\");
        assert_eq!(decode_escape("\\/"), "/");
        // Octal-shaped \0d stays verbatim (never modelled).
        assert_eq!(decode_escape("\\012"), "\\012");
    }
}
