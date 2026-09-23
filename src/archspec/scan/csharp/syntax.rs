//! Grammar source facts for C# extraction: the source-fact half of the csharp
//! driver (usings US 06, type positions US 08).
//!
//! Parses each `.cs` file through the in-binary tree-sitter grammar
//! ([`crate::archspec::parse::grammar`]) and fills the per-unit collections
//! the graph layer consumes — declared namespaces and `using`
//! targets attributed to the namespace context they appear in — so every
//! downstream rule (prefix ownership, ambiguity and reachability guards,
//! external attribution, edge assembly) reads one source of facts and the
//! addressing lives in [`super::edge_target`]. The walk also
//! collects TYPE-POSITION references (US 08): every place the source names a
//! type — base lists, object creations, casts, attributes, signatures,
//! generics, field/property/event/indexer types — plus anchored dotted
//! member-access chains, into a per-file [`TypeFacts`] the scan layer
//! resolves through the declared-namespace machinery (see [`super`]'s
//! type-position pass).
//!
//! Grammar node kinds consumed (tree-sitter-c-sharp 0.23.1, pinned, verified
//! against a node dump of representative constructs):
//! - `namespace_declaration` — block-scoped `namespace A.B { ... }`; field
//!   `name:` (`identifier` / `qualified_name`), field `body:` (usings nested
//!   under it belong to that namespace).
//! - `file_scoped_namespace_declaration` — `namespace A.B;`. NOTE: this
//!   grammar node covers only the declaration line; the members that follow
//!   are `compilation_unit` siblings, so "the file-scoped namespace owns the
//!   whole file" is implemented here by attribution order, not by
//!   node containment.
//! - `using_directive` — every import form: plain (`using A.B;`),
//!   `using static` (target follows the `static` token), alias
//!   (`using X = A.B;` — the alias carries field `name:` and is dropped,
//!   the right-hand target is what counts), and `global using` (a `global`
//!   token child). `using (expr)` resource statements parse as
//!   `using_statement` and `using var x = ...;` as
//!   `local_declaration_statement`, so both fall out of scope for free — they
//!   cannot reach the using facts.
//!
//! Type-position carriers (each dumps to the shown field of the shown kind;
//! the child is then interpreted by [`record_type_name`]):
//! - `base_list` — base types as DIRECT children (no field);
//! - `object_creation_expression`, `cast_expression`, `typeof_expression`,
//!   `default_expression`, `parameter`, `variable_declaration` (fields and
//!   locals), `property_declaration`, `indexer_declaration` — field `type:`;
//! - `method_declaration` — field `returns:` (NOT `type:` — verified);
//! - `attribute` — field `name:`;
//! - `is_pattern_expression` — field `type:`; `as_expression` — field
//!   `right:` (the only type carried in a positional field);
//! - `generic_name` / `type_argument_list` — the generic tail inside a
//!   `qualified_name` (`App.Core.Wrapper<Inner>` dumps as `qualified_name`
//!   whose `name:` is a `generic_name`), the arguments as children of
//!   `type_argument_list`;
//! - `qualified_name` — fields `qualifier:`/`name:`, text reconstructed
//!   segment-wise so line breaks inside chains cannot poison the name;
//!   `alias_qualified_name` — `global::X` / `alias::X` prefixes (the
//!   `global` anchor is dropped by [`dotted_name`], an alias root keeps its
//!   lowercase name and resolves as a partial dotted name);
//! - `member_access_expression` — dotted chains in EXPRESSION context
//!   (`App.Core.Store.Create()` dumps as nested member accesses, not a
//!   `qualified_name`); reconstructed chains count only when their leading
//!   segments anchor into a declared namespace (see the resolution table
//!   below), so instance/chain noise rooted at a local or `this` or a BCL
//!   name contributes nothing;
//! - `class_declaration` / `interface_declaration` / `struct_declaration` /
//!   `enum_declaration` / `record_declaration` — field `name:` collected as
//!   the namespace's DECLARED TYPE symbols (the precision anchor for bare
//!   resolution).
//!
//! Exclusions: `type_parameter_constraints_clause` (`where T : X`) subtrees
//! contribute nothing (by decision, explicit where-constraints are not type
//! positions for edges); `using_directive` bodies are never descended; the
//! `name:` of a namespace declaration is a declaration, not a reference;
//! `implicit_type` (`var`) and `predefined_type` (`int`, `object`, …) carry no
//! name. XML-doc comments parse as `comment` leaves — text never reaches a
//! name node, so comments cannot reference anything.
//!
//! Bare-name resolution table (decided in the workplan, implemented in the
//! scan layer's type-position pass; case-SENSITIVE everywhere):
//!
//! | written as                    | resolves when                                                   | reference recorded        |
//! |-------------------------------|-----------------------------------------------------------------|---------------------------|
//! | `A.B.Type` (qualified)        | longest declared-namespace prefix of `A.B.Type` exists           | the dotted name as written |
//! | `global::A.B.Type`            | same, after the `global.` anchor is dropped                      | `A.B.Type`                |
//! | `Type` (bare) via `using U;`  | `U` is a declared namespace AND `Type` is declared under `U`     | `U.Type`                  |
//! | `Type` (bare) via `using U.Type;` | `U` is a declared namespace (type-targeted using)            | `U.Type`                  |
//! | `root.Member…` chain          | longest declared-namespace prefix of the reconstructed chain     | the reconstructed chain   |
//! | `engine.Run()`, `this.X`, `var x`, string content, `Console.WriteLine()` | never (no declared-namespace anchor, or no declared type under a using'd namespace) | nothing |
//!
//! A bare identifier resolving through TWO different visible usings to two
//! different references is ambiguous and contributes nothing (the ambiguity
//! stance of the using path, per identifier). Type-position references
//! contribute module edges and symbols ONLY: a reference no declared
//! namespace owns (a BCL or NuGet type used without any using) attributes
//! NOTHING into the external tier — that tier stays driven by `using`
//! directives, so type-position references never add `module_external` entries
//! (decision recorded in the US 08 __log__).
//!
//! Attribution follows document order with a last-seen namespace stack never
//! popped, deciding WHICH module an edge hangs from: a block
//! `namespace_declaration` pushes its name
//! (nested declarations win while they are the last seen), a file-scoped
//! declaration clears the stack and owns the whole file, and usings seen while
//! the stack is empty stay pending: at end of file a compilation-unit using is
//! visible to every namespace the file declares, so it folds into the single
//! declared namespace when there is exactly one, into the unit-root sentinel
//! (`""`) when the file declares none, and joins EVERY declared namespace of a
//! multi-namespace file (US 10): each such namespace legitimately owns the
//! fact.
//!
//! Error tolerance: the tree-sitter tree survives broken constructs, so a
//! file with `ERROR`/`MISSING` nodes still contributes every using and
//! namespace the parser located; there is deliberately NO alternate parse path
//! when [`crate::archspec::parse::grammar::first_error_range`] reports damage.
//! Only a file tree-sitter cannot parse at all yields an error, propagated to
//! the scan failure.

use tree_sitter::Node;

use crate::archspec::language::Language;
use crate::archspec::parse::grammar::{node_text, parse_tree};

use std::collections::{BTreeMap, BTreeSet};

/// Collects declared namespaces, `using` targets and type-position references
/// from one `.cs` source into the shared per-unit maps, with the attribution
/// semantics documented in this module. `type_facts` collects the type-position
/// references (US 08).
pub(crate) fn scan_cs_source(
    source: &str,
    namespaces: &mut BTreeSet<String>,
    usings: &mut BTreeMap<String, BTreeSet<String>>,
    type_facts: &mut TypeFacts,
) -> Result<(), String> {
    let tree = parse_tree(Language::Csharp, source).map_err(|err| err.to_string())?;
    let mut file = FileFacts::default();
    file.walk(&tree.root_node(), source);
    file.finish(namespaces, usings, type_facts);
    Ok(())
}

/// Per-file type-position facts, keyed by the namespace context they were
/// collected in (the same attribution buckets as `usings`):
/// - `qualified`: anchored dotted references as written (`global::` anchors
///   already dropped) — from qualified type names and reconstructed
///   member-access chains;
/// - `candidates`: for every BARE identifier appearing in a type position or
///   as the root of a member-access chain, the DOTTED CANDIDATE references it
///   could resolve through (one per using target visible in the file at that
///   position, plus the enclosing namespace itself) — visibility is computed
///   per file (file-level, namespace-scoped and `global` usings of THIS file
///   only, no cross-file leakage) while whether a candidate is REAL is
///   decided by the scan layer against the tree-wide declared namespaces and
///   declared types;
/// - `declared_types`: the type names each namespace declares (class /
///   interface / struct / enum / record), the precision anchor that keeps a
///   local `engine` from matching the type `Engine` (case-sensitive).
#[derive(Clone, Debug, Default)]
pub(crate) struct TypeFacts {
    pub(crate) qualified: BTreeMap<String, BTreeSet<String>>,
    pub(crate) candidates: BTreeMap<String, BTreeSet<(String, String)>>,
    pub(crate) declared_types: BTreeMap<String, BTreeSet<String>>,
}

impl TypeFacts {
    pub(crate) fn merge(&mut self, other: TypeFacts) {
        for (ns, refs) in other.qualified {
            self.qualified.entry(ns).or_default().extend(refs);
        }
        for (ns, cands) in other.candidates {
            self.candidates.entry(ns).or_default().extend(cands);
        }
        for (ns, types) in other.declared_types {
            self.declared_types.entry(ns).or_default().extend(types);
        }
    }
}

/// Per-file collection state: the document-order namespace stack (block
/// declarations push and are never popped, file-scoped
/// declarations reset it), the pending usings AND pending type references
/// recorded while the stack was empty, the file's `global using` targets
/// (visible at every position of the file regardless of namespace context),
/// and the results keyed by attributed namespace.
#[derive(Default)]
struct FileFacts {
    namespaces: BTreeSet<String>,
    usings: BTreeMap<String, BTreeSet<String>>,
    global_usings: BTreeSet<String>,
    qualified: BTreeMap<String, BTreeSet<String>>,
    bare: BTreeMap<String, BTreeSet<String>>,
    declared_types: BTreeMap<String, BTreeSet<String>>,
    ns_stack: Vec<String>,
    pending_root_usings: Vec<String>,
    pending_qualified: Vec<String>,
    pending_bare: Vec<String>,
    saw_namespace: bool,
}

impl FileFacts {
    fn walk(&mut self, node: &Node<'_>, source: &str) {
        match node.kind() {
            "namespace_declaration" => {
                self.declare(node, source, false);
                self.walk_children(node, source, &["name"]);
                return;
            }
            "file_scoped_namespace_declaration" => {
                self.declare(node, source, true);
                self.walk_children(node, source, &["name"]);
                return;
            }
            "using_directive" => {
                if let Some(target) = using_target(node, source) {
                    if has_global_token(node) {
                        self.global_usings.insert(target.clone());
                    }
                    match self.ns_stack.last() {
                        Some(ns) => {
                            self.usings.entry(ns.clone()).or_default().insert(target);
                        }
                        None => self.pending_root_usings.push(target),
                    }
                }
                // A using directive contains no declarations worth walking.
                return;
            }
            // Explicit generic constraints are not type positions for edges
            // (workplan decision): the whole clause contributes nothing.
            "type_parameter_constraints_clause" => return,
            "attribute" => {
                if let Some(name) = node.child_by_field_name("name") {
                    self.record_type(&name, source);
                }
            }
            "base_list" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.record_type(&child, source);
                }
            }
            "member_access_expression" => self.record_chain(node, source),
            "as_expression" => {
                // The only type carried in a POSITIONAL field (verified dump).
                if let Some(type_node) = node.child_by_field_name("right") {
                    self.record_type(&type_node, source);
                }
            }
            "type_argument_list" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.record_type(&child, source);
                }
            }
            "class_declaration" | "interface_declaration" | "struct_declaration"
            | "enum_declaration" | "record_declaration" => {
                // Record declarations parse as `class_declaration` in this
                // grammar version (the `record` keyword rides the class
                // rule), so `record_declaration` is listed defensively.
                if let Some(name_node) = node.child_by_field_name("name") {
                    let ns = self.ns_stack.last().cloned().unwrap_or_default();
                    self.declared_types
                        .entry(ns)
                        .or_default()
                        .insert(node_text(&name_node, source).to_string());
                }
            }
            _ => {
                // Field-driven type positions: every grammar node that carries
                // a type under `type:` or `returns:` (verified against the
                // dump: object creation, casts, typeof/default, parameters,
                // variable/property/indexer declarations, method returns).
                for field in ["type", "returns"] {
                    if let Some(type_node) = node.child_by_field_name(field) {
                        self.record_type(&type_node, source);
                    }
                }
            }
        }
        self.walk_children(node, source, &[]);
    }

    /// Descends all children except the fields named in `skip` (and except
    /// nested `qualified_name` / `alias_qualified_name` children of a name
    /// chain — the outer chain was already recorded as a whole, and recording
    /// the nested qualifier `App.Core` of `App.Core.Widget` on its own would
    /// restate a prefix of an already-recorded reference).
    fn walk_children(&mut self, node: &Node<'_>, source: &str, skip: &[&str]) {
        for index in 0..node.child_count() {
            let child = match node.child(index) {
                Some(child) => child,
                None => continue,
            };
            if let Some(field) = node.field_name_for_child(index as u32) {
                if skip.contains(&field) {
                    continue;
                }
            }
            if matches!(child.kind(), "qualified_name" | "alias_qualified_name")
                && matches!(
                    child.parent().map(|parent| parent.kind()),
                    Some("qualified_name") | Some("alias_qualified_name")
                ) {
                continue;
            }
            self.walk(&child, source);
        }
    }

    /// Interprets a node appearing in a type position: a bare identifier
    /// joins the bare set, a name chain joins the qualified set with generic
    /// arguments recorded as references of their own, wrapper kinds (arrays,
    /// nullables, tuples, refs) recurse, and predefined/`var` nodes
    /// contribute nothing.
    fn record_type(&mut self, node: &Node<'_>, source: &str) {
        match node.kind() {
            "identifier" => self.record_bare(node_text(node, source)),
            "generic_name" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.record_type(&child, source);
                }
            }
            "array_type" | "nullable_type" | "pointer_type" | "tuple_type" | "ref_type" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.record_type(&child, source);
                }
            }
            "predefined_type" | "implicit_type" => {}
            _ => match dotted_chain(node, source) {
                Some(reference) => self.record_qualified(reference),
                None => {
                    let mut cursor = node.walk();
                    for child in node.named_children(&mut cursor) {
                        self.record_type(&child, source);
                    }
                }
            },
        }
    }

    /// Records one dotted reference (qualified type name or reconstructed
    /// chain), attributed like a using target: to the current namespace
    /// context, or pending until the file's namespace picture is known. The
    /// shared [`dotted_name`] normalization runs here, so `@verbatim`
    /// segments and `global::` anchors read identically to using targets.
    fn record_qualified(&mut self, reference: String) {
        let reference = dotted_name(&reference);
        if reference.is_empty() {
            return;
        }
        match self.ns_stack.last() {
            Some(ns) => {
                self.qualified.entry(ns.clone()).or_default().insert(reference);
            }
            None => self.pending_qualified.push(reference),
        }
    }

    fn record_bare(&mut self, ident: &str) {
        if ident.is_empty() {
            return;
        }
        match self.ns_stack.last() {
            Some(ns) => {
                self.bare
                    .entry(ns.clone())
                    .or_default()
                    .insert(ident.to_string());
            }
            None => self.pending_bare.push(ident.to_string()),
        }
    }

    /// Reconstructs the dotted reference of a member-access chain
    /// (`App.Core.Store.Create()` — expressions nest member accesses, they do
    /// NOT parse as `qualified_name`s; verified dump) and records it; the
    /// chain's root identifier additionally joins the bare set so a STATIC
    /// call through a bare type name (`Store.Create()` with `using App.Core;`)
    /// resolves through the using path. Chains through a call, an index or a
    /// `this` are not reconstructible and contribute nothing here — the
    /// recursion below still walks their subexpressions.
    fn record_chain(&mut self, node: &Node<'_>, source: &str) {
        if let Some(reference) = dotted_chain(node, source) {
            self.record_qualified(reference);
        }
        if let Some(expression) = node.child_by_field_name("expression") {
            if matches!(
                expression.kind(),
                "identifier" | "member_access_expression"
            ) {
                let mut root = expression;
                while let Some(inner) = root.child_by_field_name("expression") {
                    root = inner;
                }
                if root.kind() == "identifier" {
                    self.record_bare(node_text(&root, source));
                }
            }
        }
        self.walk_children(node, source, &[]);
    }

    /// Registers one namespace declaration; `file_scoped` implements the rule
    /// that a file-scoped declaration owns the whole file
    /// (the stack resets to just it); a block declaration joins the
    /// last-seen context of whatever encloses it.
    fn declare(&mut self, node: &Node<'_>, source: &str, file_scoped: bool) {
        if let Some(name) = node.child_by_field_name("name") {
            let ns = dotted_name(node_text(&name, source));
            if !ns.is_empty() {
                if file_scoped {
                    self.ns_stack.clear();
                }
                self.ns_stack.push(ns.clone());
                self.namespaces.insert(ns);
                self.saw_namespace = true;
            }
        }
    }

    /// End-of-file attribution of the facts recorded before any namespace:
    /// with exactly one namespace context they fold into it; in a file with no
    /// namespace they fold into the unit-root
    /// sentinel (`""`); in a multi-namespace file they join EVERY namespace
    /// the file declares — a compilation-unit `using` (or root-scope type
    /// reference) is visible to all of them, so each namespace owns the
    /// fact (US 10). Then the per-file
    /// expansion of bare identifiers into their candidate dotted references
    /// (visibility is exactly THIS file's usings: the context's own — now
    /// including the distributed root-scope ones —, the file's `global
    /// using`s, and the enclosing namespace itself — never another file's
    /// directives).
    fn finish(
        mut self,
        namespaces: &mut BTreeSet<String>,
        usings: &mut BTreeMap<String, BTreeSet<String>>,
        type_facts: &mut TypeFacts,
    ) {
        let declared = std::mem::take(&mut self.namespaces);
        namespaces.extend(declared.iter().cloned());
        let fold_targets: Vec<String> = if self.pending_root_usings.is_empty()
            && self.pending_qualified.is_empty()
            && self.pending_bare.is_empty()
        {
            Vec::new()
        } else if self.ns_stack.len() == 1 {
            vec![self.ns_stack[0].clone()]
        } else if !self.saw_namespace {
            vec![String::new()]
        } else {
            // Multi-namespace file: the root-scope facts attribute to each
            // declared namespace (visible to all of them), NOT to a synthetic
            // unit-root module — every edge's `from` stays a real module of
            // the soft tier (decision recorded in the US 10 __log__).
            declared.iter().cloned().collect()
        };
        let pending_usings = std::mem::take(&mut self.pending_root_usings);
        let pending_qualified = std::mem::take(&mut self.pending_qualified);
        let pending_bare = std::mem::take(&mut self.pending_bare);
        for ns in &fold_targets {
            if !pending_usings.is_empty() {
                self.usings
                    .entry(ns.clone())
                    .or_default()
                    .extend(pending_usings.iter().cloned());
            }
            if !pending_qualified.is_empty() {
                self.qualified
                    .entry(ns.clone())
                    .or_default()
                    .extend(pending_qualified.iter().cloned());
            }
            if !pending_bare.is_empty() {
                self.bare
                    .entry(ns.clone())
                    .or_default()
                    .extend(pending_bare.iter().cloned());
            }
        }
        for (ns, targets) in &self.usings {
            usings.entry(ns.clone()).or_default().extend(targets.iter().cloned());
        }
        let mut candidates: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
        for (ns, idents) in &self.bare {
            let bucket = candidates.entry(ns.clone()).or_default();
            for ident in idents {
                for visible in self.visible_targets(ns) {
                    bucket.insert((ident.clone(), format!("{visible}.{ident}")));
                    // A type-targeted using (`using App.Core.Engine;`) names
                    // the bare identifier itself: the target IS a candidate.
                    if visible.rsplit('.').next() == Some(ident.as_str()) {
                        bucket.insert((ident.clone(), visible.clone()));
                    }
                }
            }
        }
        type_facts.qualified = self.qualified;
        type_facts.candidates = candidates;
        type_facts.declared_types = self.declared_types;
    }

    /// The dotted namespaces a bare identifier at position `ns` can resolve
    /// through inside this file: the using directives attributed to that
    /// context here, the file's `global using` targets, and the enclosing
    /// namespace itself.
    fn visible_targets(&self, ns: &str) -> BTreeSet<String> {
        let mut visible = self.global_usings.clone();
        if let Some(targets) = self.usings.get(ns) {
            visible.extend(targets.iter().cloned());
        }
        if !ns.is_empty() {
            visible.insert(ns.to_string());
        }
        visible
    }
}

/// The imported target of a `using_directive`: the last identifier-shaped
/// child that is not the alias (field `name:` of `using Alias = A.B;`).
/// `static`, `global` and the keyword/punctuation children are anonymous
/// tokens or foreign kinds and never match.
fn using_target(directive: &Node<'_>, source: &str) -> Option<String> {
    let alias_id = directive.child_by_field_name("name").map(|node| node.id());
    let mut cursor = directive.walk();
    let mut target = None;
    for child in directive.named_children(&mut cursor) {
        if Some(child.id()) == alias_id {
            continue;
        }
        if matches!(
            child.kind(),
            "identifier" | "qualified_name" | "alias_qualified_name" | "generic_name"
        ) {
            target = Some(child);
        }
    }
    target.map(|node| dotted_name(node_text(&node, source)))
}

/// True when the directive carries the `global` token (`global using X;`) —
/// such targets are visible at every position of the file for bare-name
/// candidate generation (US 08), not just in the context they were declared
/// in (across-FILE global usings remain invisible here: candidates never
/// consult another file's directives, documented approximation).
fn has_global_token(directive: &Node<'_>) -> bool {
    let mut cursor = directive.walk();
    let global = directive
        .children(&mut cursor)
        .any(|child| child.kind() == "global");
    global
}

/// Node text to a dotted name: `@verbatim` markers stripped and `::`
/// separators normalized to dots — the
/// grammar renders `global::System.Text` targets and `alias::Namespace`
/// qualifiers with colon pairs the dotted prefix vocabulary of the shared
/// resolution rules cannot see otherwise. The anchoring `global::` prefix
/// carries no name and is dropped; an alias qualifier keeps its lowercase
/// alias root and resolves to no declared namespace — no false owner is
/// invented.
fn dotted_name(text: &str) -> String {
    let normalized = text.replace('@', "").replace("::", ".");
    normalized
        .strip_prefix("global.")
        .map(str::to_string)
        .unwrap_or(normalized)
}

/// Reconstruct the dotted reference a name-shaped node stands for, built
/// SEGMENT-WISE (not from raw node text, so line breaks and generic argument
/// lists inside a chain cannot poison the name):
/// - `identifier` — itself;
/// - `qualified_name` — qualifier chain + `.` + name tail;
/// - `alias_qualified_name` — alias root + `.` + name tail through the shared
///   `global::` normalization of [`dotted_name`] (a non-global alias keeps
///   its lowercase root and resolves into no declared namespace — the same
///   stance as alias-qualified using targets);
/// - `member_access_expression` — expression chain + `.` + name tail (stops
///   at the first non-name-shaped link: chains through a call, an index or
///   `this` answer `None`);
/// - `generic_name` — only the base identifier (generic ARGUMENTS are
///   recorded as references of their own by the `type_argument_list` walk
///   arm);
/// - predefined types, `var` and anything else — `None`.
fn dotted_chain(node: &Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source).to_string()),
        "qualified_name" => {
            let mut chain = dotted_chain(&node.child_by_field_name("qualifier")?, source)?;
            chain.push('.');
            chain.push_str(&chain_tail(&node.child_by_field_name("name")?, source)?);
            Some(chain)
        }
        "alias_qualified_name" => {
            let mut chain =
                node_text(&node.child_by_field_name("alias")?, source).to_string();
            chain.push('.');
            chain.push_str(&chain_tail(&node.child_by_field_name("name")?, source)?);
            Some(dotted_name(&chain))
        }
        "member_access_expression" => {
            let mut chain = dotted_chain(&node.child_by_field_name("expression")?, source)?;
            chain.push('.');
            chain.push_str(&chain_tail(&node.child_by_field_name("name")?, source)?);
            Some(chain)
        }
        "generic_name" => {
            let mut cursor = node.walk();
            let ident = node
                .named_children(&mut cursor)
                .find(|child| child.kind() == "identifier")?;
            Some(node_text(&ident, source).to_string())
        }
        _ => None,
    }
}

/// The name part of one chain position: a plain identifier, or a generic
/// tail contributing only its base identifier (the arguments are recorded
/// separately). Anything else (a keyword, a call) breaks the chain.
fn chain_tail(node: &Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source).to_string()),
        "generic_name" => dotted_chain(node, source),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{scan_cs_source, TypeFacts};
    use std::collections::{BTreeMap, BTreeSet};

    fn facts(source: &str) -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
        let mut namespaces = BTreeSet::new();
        let mut usings = BTreeMap::new();
        scan_cs_source(source, &mut namespaces, &mut usings, &mut TypeFacts::default())
            .expect("parses");
        (namespaces, usings)
    }

    fn type_facts(source: &str) -> TypeFacts {
        let mut facts = TypeFacts::default();
        scan_cs_source(
            source,
            &mut BTreeSet::new(),
            &mut BTreeMap::new(),
            &mut facts,
        )
        .expect("parses");
        facts
    }

    #[test]
    fn file_scoped_namespace_owns_usings_written_above_it() {
        let source = "using App.Core.Engine;\nnamespace App.Api;\npublic class Api { }\n";
        let (namespaces, usings) = facts(source);
        assert_eq!(
            namespaces,
            ["App.Api".to_string()].into_iter().collect(),
            "the file declares only App.Api; the using target is no declaration"
        );
        assert_eq!(
            usings.get("App.Api").cloned().unwrap_or_default(),
            ["App.Core.Engine".to_string()].into_iter().collect()
        );
    }

    #[test]
    fn block_namespace_bodies_attribute_their_own_usings() {
        let source = "namespace App.Api\n{\n    using App.Core.Engine;\n    class Api { }\n}\n";
        let (_namespaces, usings) = facts(source);
        assert_eq!(
            usings.get("App.Api").cloned().unwrap_or_default(),
            ["App.Core.Engine".to_string()].into_iter().collect()
        );
    }

    #[test]
    fn every_using_form_shares_the_target() {
        // alias, static and global forms all resolve to the right-hand target;
        // the `global::` qualifier prefix is dropped; resource `using (...)`
        // and `using var` forms are not import facts.
        let source = "namespace App.Api\n{\n    using Engine = App.Core.Engine;\n    using static App.Core.Store;\n    global using global::App.Core.Vehicle;\n    class Api\n    {\n        void Run()\n        {\n            using (var d = new D()) { }\n            using var e = new D();\n        }\n    }\n}\n";
        let (_namespaces, usings) = facts(source);
        assert_eq!(
            usings.get("App.Api").cloned().unwrap_or_default(),
            [
                "App.Core.Engine".to_string(),
                "App.Core.Store".to_string(),
                "App.Core.Vehicle".to_string()
            ]
            .into_iter()
            .collect(),
            "alias/static/global targets collected; resource and `using var` \
             statements excluded; the `global::` anchor dropped"
        );
    }

    #[test]
    fn namespace_less_file_attributes_usings_to_the_unit_root() {
        let source = "using App.Core.Engine;\npublic class Program { }\n";
        let (namespaces, usings) = facts(source);
        assert!(namespaces.is_empty());
        assert_eq!(
            usings.get("").cloned().unwrap_or_default(),
            ["App.Core.Engine".to_string()].into_iter().collect(),
            "the empty-string sentinel marks the unit root"
        );
    }

    #[test]
    fn error_tolerant_tree_still_contributes_usings() {
        // Unterminated class body — the grammar reports an ERROR region but
        // the directives above it are located and collected (no fallback).
        let source = "using App.Core.Engine;\nnamespace App.Api;\npublic class Api\n{\n";
        let (namespaces, usings) = facts(source);
        assert!(namespaces.contains("App.Api"));
        assert_eq!(
            usings.get("App.Api").cloned().unwrap_or_default(),
            ["App.Core.Engine".to_string()].into_iter().collect()
        );
    }

    // ---- US 08: type-position collection ---------------------------------

    /// Every covered grammar position dumps its reference: base list, object
    /// creation, cast, attribute, signature parameter and return, generic
    /// arguments, field type (qualified tail of a generic name included).
    #[test]
    fn type_positions_dump_their_dotted_references() {
        let source = "namespace App.Api\n{\n    [App.Core.Attr]\n    public class Api : App.Core.IBase\n    {\n        private App.Core.IRepository _repo;\n        public App.Core.Widget Make(App.Core.Parameter p)\n        {\n            var engine = new App.Core.Engine();\n            object o = null;\n            var w = (App.Core.Widget)o;\n            var list = new App.Core.Wrapper<App.Core.Inner>();\n            return null;\n        }\n    }\n}\n";
        let facts = type_facts(source);
        assert_eq!(
            facts.qualified.get("App.Api").cloned().unwrap_or_default(),
            [
                "App.Core.Attr".to_string(),
                "App.Core.Engine".to_string(),
                "App.Core.IBase".to_string(),
                "App.Core.IRepository".to_string(),
                "App.Core.Inner".to_string(),
                "App.Core.Parameter".to_string(),
                "App.Core.Widget".to_string(),
                "App.Core.Wrapper".to_string(),
            ]
            .into_iter()
            .collect(),
            "one entry per referenced type, generic base WITHOUT its argument list (args recorded separately)"
        );
    }

    /// Chains in EXPRESSION context nest member accesses; every
    /// RECONSTRUCTIBLE chain joins the qualified set (whether it anchors into
    /// a declared namespace is the scan layer's decision) — locals, BCL roots
    /// and all. Chains through a keyword (`this`) or a call are not
    /// reconstructible and never appear.
    #[test]
    fn member_access_chains_are_reconstructed_and_unreconstructible_ones_are_not() {
        let source = "namespace App.Api;\nclass Api\n{\n    void M()\n    {\n        var x = App.Core.Store.Create();\n        engine.Run();\n        foo.Bar.Baz();\n        var y = this.Value;\n        var z = (new object()).ToString();\n        Console.WriteLine();\n    }\n}\n";
        let facts = type_facts(source);
        let qualified = facts.qualified.get("App.Api").cloned().unwrap_or_default();
        for chain in ["App.Core.Store.Create", "engine.Run", "foo.Bar.Baz", "Console.WriteLine"] {
            assert!(qualified.contains(chain), "chain {chain} reconstructed: {qualified:?}");
        }
        assert!(
            qualified.iter().all(|r| !r.contains("this") && !r.contains("ToString")),
            "chains through a keyword or a call break reconstruction: {qualified:?}"
        );
    }

    /// A bare identifier at a chain root gets one candidate per using visible
    /// in the file (plus the enclosing namespace); a type-targeted using
    /// contributes its own target as the candidate naming that identifier.
    /// Another file's directives would NOT be visible (per-file here).
    #[test]
    fn bare_identifiers_become_candidates_from_visible_usings_only() {
        let source = "using App.Core;\nglobal using App.Shared;\nnamespace App.Api;\nclass Api\n{\n    private Engine _e;\n    void M()\n    {\n        Store.Create();\n        engine.Run();\n    }\n}\n";
        let facts = type_facts(source);
        let cands = facts.candidates.get("App.Api").cloned().unwrap_or_default();
        assert!(
            cands.contains(&("Engine".to_string(), "App.Core.Engine".to_string())),
            "context usings give the ns+ident candidate: {cands:?}"
        );
        assert!(
            cands.contains(&("Engine".to_string(), "App.Api.Engine".to_string())),
            "the enclosing namespace is itself a lookup scope: {cands:?}"
        );
        assert!(
            cands.contains(&("Store".to_string(), "App.Shared.Store".to_string())),
            "the file's global using is visible at every position: {cands:?}"
        );
        assert!(
            cands.contains(&("engine".to_string(), "App.Core.engine".to_string())),
            "candidates are generated case-agnostic — whether `App.Core.engine` is a REAL reference (a declared type named exactly `engine`) is the scan layer's anchored filter: {cands:?}"
        );
        // A type-targeted using names the bare identifier through the target
        // itself: `using App.Core.Engine;` + bare `Engine` -> candidate
        // "App.Core.Engine" (as well as the ns-style "App.Core.Engine.Engine"
        // from the using TARGET treated as a namespace, filtered later).
        let source = "using App.Core.Engine;\nnamespace App.Api;\nclass Api { void M() { var e = new Engine(); } }\n";
        let facts = type_facts(source);
        let cands = facts.candidates.get("App.Api").cloned().unwrap_or_default();
        assert!(
            cands.contains(&("Engine".to_string(), "App.Core.Engine".to_string())),
            "type-targeted using contributes its own target: {cands:?}"
        );
    }

    /// Explicit generic constraints are not type positions (workplan
    /// decision): a `where T : App.Core.Constraint` contributes nothing.
    #[test]
    fn where_clauses_contribute_nothing() {
        let source = "namespace App.Api\nclass Api<T> where T : App.Core.Constraint\n{\n}\n";
        let facts = type_facts(source);
        assert!(
            facts.qualified.values().all(|refs| refs.iter().all(|r| !r.contains("Constraint"))),
            "the where constraint contributes no reference: {:?}",
            facts.qualified
        );
    }

    /// `global::` in a TYPE position normalizes through the same anchor-drop
    /// rule the using path applies (US 07 handoff): the reference is recorded
    /// without the anchor, never as a `global`-rooted name.
    #[test]
    fn global_anchor_is_dropped_in_type_positions() {
        let source = "class Api\n{\n    global::App.Core.Anchored _a;\n    void M()\n    {\n        var t = global::App.Core.Engine.Create();\n    }\n}\n";
        let facts = type_facts(source);
        let qualified = facts.qualified.get("").cloned().unwrap_or_default();
        assert!(
            qualified.contains("App.Core.Anchored"),
            "field type chain normalized: {qualified:?}"
        );
        assert!(
            qualified.contains("App.Core.Engine.Create"),
            "expression chain normalized: {qualified:?}"
        );
        assert!(
            qualified.iter().all(|r| !r.starts_with("global")),
            "no reference keeps the anchor: {qualified:?}"
        );
    }

    /// US 09 (injected fields): the FIELD TYPE is the only real reference;
    /// the member-access chain through the field reconstructs as `_engine.Run`
    /// — noise the anchored-prefix rule kills at the scan layer — and the
    /// chain ROOT identifier `_engine` joins the bare set with candidates that
    /// the declared-type filter rejects (no namespace declares `_engine` or
    /// `Run`). No identifier-to-field link step is needed: the field
    /// declaration is file-scoped, so its type position already carries the
    /// edge and the calls add nothing structurally.
    #[test]
    fn injected_field_and_its_call_chains_record_the_field_type_only() {
        let source = "namespace App.Api\nclass Api\n{\n    private readonly App.Core.IEngine _engine;\n    void M()\n    {\n        _engine.Run();\n        _engine.Run();\n    }\n}\n";
        let facts = type_facts(source);
        let qualified = facts.qualified.get("App.Api").cloned().unwrap_or_default();
        assert!(
            qualified.contains("App.Core.IEngine"),
            "the field type position is the real reference: {qualified:?}"
        );
        assert!(
            qualified.contains("_engine.Run"),
            "the chain IS recorded (ownership decides later): {qualified:?}"
        );
        let cands = facts.candidates.get("App.Api").cloned().unwrap_or_default();
        assert!(
            cands
                .iter()
                .any(|(ident, _)| ident == "_engine")
                && cands
                    .iter()
                    .all(|(_, reference)| !reference.ends_with(".Run")),
            "the chain root becomes a bare candidate; method tails never do: {cands:?}"
        );
    }

    /// US 10: a `using` at compilation-unit scope (above the first block
    /// namespace) is visible to EVERY namespace of the file, so it attributes
    /// to EACH declared namespace rather than being dropped or folded into
    /// one. With exactly one namespace it folds into that one.
    #[test]
    fn root_usings_in_multi_namespace_files_attribute_to_each_namespace() {
        let source = "using App.Core.Engine;\nnamespace App.Api\n{\n    class Api { }\n}\nnamespace App.Model\n{\n    class Model { }\n}\n";
        let (_namespaces, usings) = facts(source);
        assert_eq!(
            usings.get("App.Api").cloned().unwrap_or_default(),
            ["App.Core.Engine".to_string()].into_iter().collect(),
            "the compilation-unit using reaches the first namespace"
        );
        assert_eq!(
            usings.get("App.Model").cloned().unwrap_or_default(),
            ["App.Core.Engine".to_string()].into_iter().collect(),
            "and the second one too — it is visible there as well"
        );
    }

    /// Bare identifiers in type positions become candidates; declarations
    /// feed the declared-type map (the case-sensitive anchor the scan layer
    /// matches bare names against); `var` and predefined types contribute
    /// nothing.
    #[test]
    fn bare_type_positions_and_declared_types_are_collected() {
        let source = "using System;\nnamespace App.Core;\npublic class Engine { }\npublic interface IRepository { }\npublic enum Kind { A }\npublic struct Point { }\nnamespace App.Api\n{\n    class Api\n    {\n        private Engine _e;\n        public Kind Make(Parameter p)\n        {\n            var local = new Engine();\n            int n = 1;\n            return default;\n        }\n    }\n}\n";
        let facts = type_facts(source);
        assert_eq!(
            facts.declared_types.get("App.Core").cloned().unwrap_or_default(),
            [
                "Engine".to_string(),
                "IRepository".to_string(),
                "Kind".to_string(),
                "Point".to_string(),
            ]
            .into_iter()
            .collect(),
            "class, interface, enum and struct names all declare types"
        );
        let cands = facts.candidates.get("App.Api").cloned().unwrap_or_default();
        for ident in ["Engine", "Kind", "Parameter"] {
            assert!(
                cands.contains(&(ident.to_string(), format!("App.Api.{ident}"))),
                "bare {ident} candidate through the enclosing namespace: {cands:?}"
            );
        }
        for noise in ["local", "n"] {
            assert!(
                !cands.iter().any(|(ident, _)| ident == noise),
                "locals are not type positions: {cands:?}"
            );
        }
    }
}
