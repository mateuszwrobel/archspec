//! Grammar source facts for C# extraction: the source-fact half of the csharp
//! driver (usings US 06, type positions US 08).
//!
//! Parses each `.cs` file through the in-binary tree-sitter grammar
//! ([`crate::parse::grammar`]) and fills the per-unit collections
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
//!   resolution). BODY-SHAPE table (verified against parse dumps AND against
//!   the released 0.5.2 binary during the GitHub issue #2 investigation):
//!   the grammar gives the `;` body alternative to `record_declaration`
//!   ONLY — positional `record X(...);`, braced, bodiless `record X;`,
//!   `record struct`, `record class`, partial/generic/`where`/base/attribute
//!   spellings all dump to `record_declaration` and are collected in every
//!   body shape. The other four kinds REQUIRE a braced body
//!   (`declaration_list`, `enum_member_declaration_list`), so the legal
//!   current-C# spellings `class X;`, `class X(int A);`, `struct X(int A);`,
//!   `interface I;`, `enum E;` dump to NO declaration node: an `ERROR`
//!   fragment (`modifier`* + bare `identifier`, optionally `parameter_list`;
//!   the `;` reparses as a sibling `empty_statement`). Facts are never
//!   invented from ERROR fragments — a stray `public Foo;` garbage dumps to
//!   the same shape — so those spellings contribute nothing, the bounded gap
//!   welded in the public-api suite (workplan csharp_declaration_bodies).
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
//! when [`crate::parse::grammar::first_error_range`] reports damage.
//! Only a file tree-sitter cannot parse at all yields an error, propagated to
//! the scan failure.

use tree_sitter::Node;

use crate::language::Language;
use crate::model::LiteralCandidateAnchor;
use crate::parse::grammar::{node_text, parse_tree};

use std::collections::{BTreeMap, BTreeSet};

/// Collects declared namespaces, `using` targets and type-position references
/// from one `.cs` source into the shared per-unit maps, with the attribution
/// semantics documented in this module. `type_facts` collects the type-position
/// references (US 08).
pub fn scan_cs_source(
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
pub struct TypeFacts {
    pub(crate) qualified: BTreeMap<String, BTreeSet<String>>,
    pub(crate) candidates: BTreeMap<String, BTreeSet<(String, String)>>,
    pub(crate) declared_types: BTreeMap<String, BTreeSet<String>>,
    /// Public-API facts (US 01): namespace -> type names whose declaration
    /// carries an explicit `public` modifier (partials deduped, nested types
    /// named `Outer.Nested`). The public surface of the file set scanned so
    /// far; the scan layer attributes it to modules and drops test-tier
    /// projects before serializing.
    pub(crate) public_types: BTreeMap<String, BTreeSet<String>>,
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
        for (ns, types) in other.public_types {
            self.public_types.entry(ns).or_default().extend(types);
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
    public_types: BTreeMap<String, BTreeSet<String>>,
    ns_stack: Vec<String>,
    /// Enclosing public-type name chain (`Outer` while walking Outer's body):
    /// names nested type declarations through their parents. Type
    /// declarations push/pop around their own descent.
    type_stack: Vec<String>,
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
                    let name = node_text(&name_node, source).to_string();
                    self.declared_types
                        .entry(ns.clone())
                        .or_default()
                        .insert(name.clone());
                    // Public-API fact (US 01 of the csharp public-api plan):
                    // an explicit `public` modifier on ANY declaration of the
                    // type (partial parts merge modifiers in C#) makes it
                    // surface API. Nested types are named through their
                    // parent chain (`Outer.Nested`) under the same namespace
                    // context as their top-level owner.
                    if declares_public(node, source) {
                        let qualified = if self.type_stack.is_empty() {
                            name.clone()
                        } else {
                            format!("{}.{}", self.type_stack.join("."), name)
                        };
                        self.public_types.entry(ns).or_default().insert(qualified);
                    }
                    // Descend with the type context pushed so nested type
                    // declarations name their parent; the namespace stack
                    // belongs to the enclosing namespace context already.
                    self.type_stack.push(name);
                    self.walk_children(node, source, &[]);
                    self.type_stack.pop();
                    return;
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
            if let Some(field) = node.field_name_for_child(index as u32)
                && skip.contains(&field) {
                    continue;
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
    /// shared descent in `walk` still walks their subexpressions (this
    /// function does NOT descend: the arm is in the record-and-fall-through
    /// group, and a self-descent here made nested chains scan in Θ(2^depth)
    /// by visiting each subtree twice per level, GitHub issue #53).
    fn record_chain(&mut self, node: &Node<'_>, source: &str) {
        if let Some(reference) = dotted_chain(node, source) {
            self.record_qualified(reference);
        }
        if let Some(expression) = node.child_by_field_name("expression")
            && matches!(
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
        // Public-API facts pass through un-folded: a declaration lives in the
        // namespace it is written in (the `""` key is the global namespace,
        // which the scan layer attributes to the unit root) — unlike pending
        // references, a type is never "visible from" later namespaces.
        type_facts.public_types = self.public_types;
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
    
    directive
        .children(&mut cursor)
        .any(|child| child.kind() == "global")
}

/// True when a type declaration node states the `public` modifier explicitly:
/// as a direct keyword child or inside a `modifiers` node (the grammar shapes
/// differ across versions; both spellings count). Every other spelling — no
/// modifier, `internal`, `private`, `protected`, `file` — exposes nothing:
/// C# defaults a type to `internal` at namespace level and to `private`
/// nested, so visibility is ONLY what the declaration states (no inherited
/// visibility is invented — a member's modifier never promotes its type).
fn declares_public(node: &Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    let stated = node
        .children(&mut cursor)
        .any(|child| node_text(&child, source) == "public");
    if stated {
        return true;
    }
    let mut cursor = node.walk();
    
    node.children(&mut cursor).any(|child| {
        child.kind() == "modifiers"
            && node_text(&child, source)
                .split_whitespace()
                .any(|word| word == "public")
    })
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

// ---------------------------------------------------------------------------
// Event payload-shape derivations (workplan archspec_async_depends, D2/M5,
// scenario 04). TWO facts, one per side of a declared `[[async_depends]]`
// edge, and shape is never declared: the PUBLISHER side is the parsed
// record-declaration shape (positional parameters and field/property
// declarations of records; the enum census names the enum-typed members the
// projection class reads), the CONSUMER side is the bound-field shape (which
// fields the unit's code references on variables of the record type). A
// dynamic, `JsonDocument` or map binding binds no variable of the event type
// and derives NOTHING — the drift census stays silent on such a side, never
// a guess (M5). The walk is deliberately a separate pass over a freshly
// parsed tree: it reads member structure, cannot perturb the reference
// facts of `FileFacts::walk` above, and keeps the byte law of every other
// surface trivially intact.
// ---------------------------------------------------------------------------

/// Per-file (merged to per-unit by the scan layer) shape facts. BTree
/// ordering everywhere; a file that carries no record declaration, no enum
/// and no typed binding contributes an empty value.
#[derive(Clone, Debug, Default)]
pub struct ShapeFacts {
    /// namespace -> record name -> members sorted by member name.
    pub(crate) records: BTreeMap<String, BTreeMap<String, Vec<MemberShape>>>,
    /// namespace -> record name -> the simple names written on the
    /// declaration's base list, sorted and deduped (scenario 07: the base
    /// chain the drift census follows on the publisher side). A record with
    /// no base list carries no entry.
    pub(crate) bases: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// namespace -> declared enum type names.
    pub(crate) enums: BTreeMap<String, BTreeSet<String>>,
    /// event type name (simple name, as declared or written) -> the fields
    /// the file's code binds on variables of that type, attributed through
    /// the scope chain of each access site (issue #46: a name in one method
    /// or one class never borrows another's binding), and which of them are
    /// bound as a string projection.
    pub(crate) bounds: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)>,
}

/// One member of a derived record shape: `(name, declared type as written)`.
type MemberShape = (String, String);

impl ShapeFacts {
    pub(crate) fn merge(&mut self, other: ShapeFacts) {
        for (ns, records) in other.records {
            let entry = self.records.entry(ns).or_default();
            for (name, members) in records {
                entry.entry(name).or_default().extend(members);
            }
        }
        for (ns, bases) in other.bases {
            let entry = self.bases.entry(ns).or_default();
            for (name, list) in bases {
                let entry = entry.entry(name).or_default();
                entry.extend(list);
                entry.sort();
                entry.dedup();
            }
        }
        for (ns, names) in other.enums {
            self.enums.entry(ns).or_default().extend(names);
        }
        for (name, (fields, projected)) in other.bounds {
            let entry = self.bounds.entry(name).or_default();
            entry.0.extend(fields);
            entry.1.extend(projected);
        }
    }
}

/// Shape facts of one `.cs` source (see the module section above): parses the
/// file a second time on purpose — the shape walk is independent of the
/// reference walk, and an independence a reader can verify beats a shared
/// walker with two jobs.
pub fn scan_cs_shapes(source: &str) -> Result<ShapeFacts, String> {
    let tree = parse_tree(Language::Csharp, source).map_err(|err| err.to_string())?;
    // The file itself is the outermost scope frame (id 0): every frame the
    // walk pushes rides it, and every chain's recorded path is non-empty.
    let mut walk = ShapeWalk {
        frame_seq: 1,
        scope_stack: vec![0],
        ..ShapeWalk::default()
    };
    walk.walk(&tree.root_node(), source);
    Ok(walk.finish())
}

/// Chain segment of a flattened member access: the scope path at the access
/// site (frame ids, outermost first — resolution walks it innermost-first),
/// the receiver root, the flattened segments and the flag saying the
/// outermost member-access node is the callee of an invocation (issue #46:
/// the tail of `evt.ToString()` / `evt.GetType()` is a method name, never a
/// bound field).
type Chain = (Vec<usize>, String, Vec<String>, bool);

/// Collection state: the document-order namespace stack with the same
/// attribution semantics as `FileFacts` above (block declarations push,
/// file-scoped declarations reset), the pending facts recorded while the
/// stack was empty (folded like `FileFacts::finish` folds), the variable
/// bindings keyed `(scope frame, variable)`, the member-access chains with
/// the scope path of their access site, and the string-literal comparisons
/// awaiting the end-of-file join (issue #46: the join resolves a chain root
/// through the scope chain of that site, never against the whole file).
#[derive(Default)]
struct ShapeWalk {
    ns_stack: Vec<String>,
    declared: BTreeSet<String>,
    saw_namespace: bool,
    pending_records: Vec<(String, String, Vec<MemberShape>)>,
    pending_bases: Vec<(String, String, Vec<String>)>,
    records: BTreeMap<String, BTreeMap<String, Vec<MemberShape>>>,
    bases: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pending_enums: Vec<(String, String)>,
    enums: BTreeMap<String, BTreeSet<String>>,
    /// Fresh id for the next pushed scope frame — every frame is unique, so
    /// two sibling blocks never share a binding key.
    frame_seq: usize,
    /// The scope frames active at the walk position, outermost first.
    scope_stack: Vec<usize>,
    /// `(frame, variable) -> SET of type tails` bound at that frame: the
    /// conservative union of the frame's declarations, never last-wins; an
    /// EMPTY set is a shadow without a type (an implicit lambda parameter —
    /// it binds nothing, but hides every outer declaration of the name).
    binds: BTreeMap<(usize, String), BTreeSet<String>>,
    /// member-access chains rooted at a bare identifier, each carrying the
    /// scope path of its access site and the invocation-tail flag.
    chains: BTreeSet<Chain>,
    /// `(scope path, root, field, direct)` pairs compared against a
    /// string literal with `==`/`!=`; `direct` says the compared field hangs
    /// off the bound variable itself (scenario 07: a property-hop comparison
    /// binds the projected field, not a direct reference of the event
    /// record).
    string_comparisons: BTreeSet<(Vec<usize>, String, String, bool)>,
}

impl ShapeWalk {
    fn walk(&mut self, node: &Node<'_>, source: &str) {
        match node.kind() {
            "namespace_declaration" => {
                self.declare_ns(node, source, false);
                self.walk_children(node, source, &["name"]);
                return;
            }
            "file_scoped_namespace_declaration" => {
                self.declare_ns(node, source, true);
                self.walk_children(node, source, &["name"]);
                return;
            }
            // A using directive contains no declarations and no bindings.
            "using_directive" => return,
            "class_declaration" | "interface_declaration" | "struct_declaration"
            | "record_declaration" => {
                // ONE scope frame per type declaration (issue #46, M4): the
                // type's fields and properties bind into THIS frame, so two
                // classes in one file never share a field binding.
                // declare_type descends the declaration itself (skipping its
                // name, which is a declaration, not a binding); no second
                // descent below — the subtree is walked exactly once.
                self.push_frame();
                self.declare_type(node, source);
                self.pop_frame();
                return;
            }
            "enum_declaration" => {
                self.declare_type(node, source);
                return;
            }
            // Every other binding-bearing scope: methods and constructors
            // carry their parameters, blocks carry their locals, a lambda
            // carries its parameters (typed ones bind, implicit ones shadow),
            // a `using` statement and the loop heads carry their variables,
            // and the switch sections carry their pattern variables.
            "method_declaration" | "constructor_declaration" | "local_function_statement"
            | "lambda_expression" | "block" | "using_statement" | "for_statement"
            | "foreach_statement" | "switch_section" | "switch_expression_arm" => {
                self.push_frame();
                self.walk_children(node, source, &[]);
                self.pop_frame();
                return;
            }
            "parameter" => {
                if let (Some(name), Some(type_node)) = (
                    node.child_by_field_name("name"),
                    node.child_by_field_name("type"),
                ) {
                    self.bind(&type_node, source, node_text(&name, source));
                } else if let Some(name) = node.child_by_field_name("name") {
                    // A parameter written WITHOUT a type is a lambda
                    // parameter: it shadows its name and binds nothing
                    // (the implicit-lambda silence law, unchanged).
                    self.shadow(node_text(&name, source));
                }
            }
            // A pattern variable — `case OrderPlaced p:` or
            // `message is OrderPlaced p` (`declaration_pattern`) — binds in
            // the frame the walk stands in: the section frame for a switch
            // arm, the enclosing block for an `is` test (issue #46: these
            // arms are NEW — chains that were silent here now speak, scoped
            // to the section that declares the variable).
            "declaration_pattern" => {
                if let (Some(name), Some(type_node)) = (
                    node.child_by_field_name("name"),
                    node.child_by_field_name("type"),
                ) {
                    self.bind(&type_node, source, node_text(&name, source));
                }
            }
            // An implicit lambda parameter (`e => e.Field()`) shadows its
            // name and binds NOTHING — it hides every outer declaration of
            // the name, and the silence law for this shape is unchanged.
            "implicit_parameter" => self.shadow(node_text(node, source)),
            "variable_declaration" => {
                if let Some(type_node) = node.child_by_field_name("type") {
                    let mut cursor = node.walk();
                    for declarator in node.named_children(&mut cursor) {
                        if declarator.is_named() && declarator.kind() == "variable_declarator"
                            && let Some(name) = declarator.child_by_field_name("name")
                        {
                            self.bind(&type_node, source, node_text(&name, source));
                        }
                    }
                }
                if let (Some(type_node), Some(name)) = (
                    node.child_by_field_name("type"),
                    node.child_by_field_name("name"),
                ) {
                    self.bind(&type_node, source, node_text(&name, source));
                }
            }
            "field_declaration" | "property_declaration" => {
                if let Some(type_node) = node.child_by_field_name("type") {
                    if let Some(name) = node.child_by_field_name("name") {
                        self.bind(&type_node, source, node_text(&name, source));
                    }
                    let mut cursor = node.walk();
                    for declarator in node.named_children(&mut cursor) {
                        if declarator.is_named() && declarator.kind() == "variable_declarator"
                            && let Some(name) = declarator.child_by_field_name("name")
                        {
                            self.bind(&type_node, source, node_text(&name, source));
                        }
                    }
                }
            }
            "member_access_expression" => {
                let mut segments = Vec::new();
                flatten_chain(node, source, &mut segments);
                if segments.len() >= 2 {
                    // The invocation flag: a member access that is the
                    // CALLEE of an invocation (`evt.ToString()`,
                    // `evt.GetType()`) ends in a method name, and that tail
                    // joins no bound field (issue #46 — the false
                    // `drift field absent: ToString` class).
                    let invoked = node.parent().is_some_and(|parent| {
                        parent.kind() == "invocation_expression"
                            && parent
                                .child_by_field_name("function")
                                .is_some_and(|function| function.id() == node.id())
                    });
                    let path = self.scope_stack.clone();
                    self.chains
                        .insert((path, segments[0].clone(), segments, invoked));
                }
            }
            "binary_expression" => {
                let operator = node
                    .child_by_field_name("operator")
                    .map(|token| node_text(&token, source).to_string())
                    .unwrap_or_default();
                if (operator == "==" || operator == "!=")
                    && let Some((root, field, direct)) = string_compared_field(node, source) {
                    let path = self.scope_stack.clone();
                    self.string_comparisons
                        .insert((path, root, field, direct));
                }
            }
            _ => {}
        }
        self.walk_children(node, source, &[]);
    }

    fn walk_children(&mut self, node: &Node<'_>, source: &str, skip: &[&str]) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if skip.contains(&child.kind()) {
                continue;
            }
            self.walk(&child, source);
        }
    }

    /// Enter a new scope frame: every push takes a fresh id so sibling
    /// frames never collide, and every chain recorded inside carries it.
    fn push_frame(&mut self) {
        let frame = self.frame_seq;
        self.frame_seq += 1;
        self.scope_stack.push(frame);
    }

    fn pop_frame(&mut self) {
        self.scope_stack.pop();
    }

    fn declare_ns(&mut self, node: &Node<'_>, source: &str, file_scoped: bool) {
        if let Some(name) = node.child_by_field_name("name") {
            let ns = dotted_name(node_text(&name, source));
            if !ns.is_empty() {
                if file_scoped {
                    self.ns_stack.clear();
                }
                self.ns_stack.push(ns.clone());
                self.declared.insert(ns);
                self.saw_namespace = true;
            }
        }
    }

    /// Classify one type declaration: enums join the enum census, records
    /// carry their member shape (positional parameters AND field/property
    /// declarations of the body — the two spellings the grammar gives a
    /// record), and every declaration still descends so nested types,
    /// parameters and member chains inside any body join the walk.
    fn declare_type(&mut self, node: &Node<'_>, source: &str) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = node_text(&name_node, source).to_string();
        let is_record = node.kind() == "record_declaration" || has_modifier(node, source, "record");
        let is_enum = node.kind() == "enum_declaration" || has_modifier(node, source, "enum");
        let ns = self.ns_stack.last().cloned().unwrap_or_default();
        if is_enum {
            if ns.is_empty() {
                self.pending_enums.push((String::new(), name.clone()));
            } else {
                self.enums.entry(ns.clone()).or_default().insert(name.clone());
            }
        }
        if is_record {
            let mut members: Vec<(String, String)> = Vec::new();
            let mut bases: Vec<String> = Vec::new();
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                match child.kind() {
                    "parameter_list" => {
                        let mut inner = child.walk();
                        for parameter in child.named_children(&mut inner) {
                            if parameter.kind() != "parameter" {
                                continue;
                            }
                            if let (Some(field), Some(type_node)) = (
                                parameter.child_by_field_name("name"),
                                parameter.child_by_field_name("type"),
                            ) {
                                members.push((
                                    node_text(&field, source).to_string(),
                                    node_text(&type_node, source).trim().to_string(),
                                ));
                            }
                        }
                    }
                    // The base list (scenario 07): every name the declaration
                    // writes after `:` is a base the drift census follows.
                    // Generics and dotted spellings cut to the simple name
                    // (the same type-tail law as bindings); a base_list node
                    // carries both base classes and interfaces, and the
                    // census follows only names a record census declares.
                    "base_list" => {
                        let mut inner = child.walk();
                        for base in child.named_children(&mut inner) {
                            let name = type_tail(node_text(&base, source));
                            if !name.is_empty() {
                                bases.push(name);
                            }
                        }
                    }
                    "declaration_list" => {
                        let mut inner = child.walk();
                        for member in child.named_children(&mut inner) {
                            if matches!(
                                member.kind(),
                                "field_declaration" | "property_declaration"
                            ) {
                                member_shape(&member, source, &mut members);
                            }
                        }
                    }
                    _ => {}
                }
            }
            members.sort();
            members.dedup();
            bases.sort();
            bases.dedup();
            if ns.is_empty() {
                self.pending_records
                    .push((String::new(), name.clone(), members));
                if !bases.is_empty() {
                    self.pending_bases.push((String::new(), name, bases));
                }
            } else {
                self.records
                    .entry(ns.clone())
                    .or_default()
                    .insert(name.clone(), members);
                if !bases.is_empty() {
                    self.bases.entry(ns).or_default().insert(name, bases);
                }
            }
        }
        // Descend: nested declarations, the body's fields and properties,
        // parameters and expression chains all belong to the other arms.
        self.walk_children(node, source, &["name"]);
    }

    /// Bind a variable name to the simple name of its declared type IN THE
    /// INNERMOST SCOPE FRAME. Only identifier and qualified-name type
    /// spellings bind (generics, arrays and pointer types bind NOTHING — a
    /// binding the census would have to guess through is a binding it does
    /// not have, M5 silence). The frame key is the fix for issue #46: the
    /// same name in two methods or two classes never shares a binding.
    fn bind(&mut self, type_node: &Node<'_>, source: &str, variable: &str) {
        if !matches!(type_node.kind(), "identifier" | "qualified_name" | "alias_qualified_name") {
            return;
        }
        let tail = type_tail(node_text(type_node, source));
        if !tail.is_empty()
            && !variable.is_empty()
            && let Some(frame) = self.scope_stack.last().copied()
        {
            self.binds
                .entry((frame, variable.to_string()))
                .or_default()
                .insert(tail);
        }
    }

    /// Record a name as SHADOWED in the innermost frame with no type at all
    /// (an implicit lambda parameter): resolution stops at this frame and
    /// answers nothing, so no outer same-named declaration is ever
    /// attributed to a chain that the shadow covers.
    fn shadow(&mut self, variable: &str) {
        if variable.is_empty() {
            return;
        }
        if let Some(frame) = self.scope_stack.last().copied() {
            self.binds
                .entry((frame, variable.to_string()))
                .or_default();
        }
    }

    /// Resolve a chain root recorded at `path`: walk the scope chain OUTWARD
    /// from the access site, innermost first — the first frame declaring the
    /// name answers with its whole union tail set (a frame that declares the
    /// name twice answers with BOTH tails), an earlier shadow answers with
    /// NOTHING and hides the frames beyond it, and a root declared only in
    /// a sibling scope answers nothing at all (issue #46).
    fn resolve_root(&self, path: &[usize], root: &str) -> Option<&BTreeSet<String>> {
        path.iter()
            .rev()
            .find_map(|frame| self.binds.get(&(*frame, root.to_string())))
    }

    /// End-of-file join, with the same attribution fold `FileFacts::finish`
    /// applies (single namespace -> that one; none -> the unit-root sentinel
    /// `""`; several -> every declared namespace owns the fact): pending
    /// root-scope declarations fold, then every chain contributes its second
    /// segment to the bound fields of the type ITS root resolves to AT THE
    /// ACCESS SITE — never to every type that ever spelled that name in the
    /// file (issue #46) — projected at the segment before a `ToString` call,
    /// at any hop (scenario 07). An invocation-tailed two-segment chain
    /// (`evt.ToString()`) names a method, not a field, and contributes no
    /// field. A string comparison joins the compared field the same way:
    /// direct accesses name a bound field, property-hop accesses project
    /// their last segment alone.
    fn finish(mut self) -> ShapeFacts {
        let declared: Vec<String> = self.declared.iter().cloned().collect();
        let fold: Vec<String> = if self.ns_stack.len() == 1 {
            vec![self.ns_stack[0].clone()]
        } else if !self.saw_namespace || declared.is_empty() {
            vec![String::new()]
        } else {
            declared.clone()
        };
        for (ns, name, members) in self.pending_records.drain(..) {
            let _ = ns;
            for target in &fold {
                self.records
                    .entry(target.clone())
                    .or_default()
                    .entry(name.clone())
                    .or_default()
                    .extend(members.iter().cloned());
            }
        }
        for (ns, name, bases) in self.pending_bases.drain(..) {
            let _ = ns;
            for target in &fold {
                let entry = self
                    .bases
                    .entry(target.clone())
                    .or_default()
                    .entry(name.clone())
                    .or_default();
                entry.extend(bases.iter().cloned());
                entry.sort();
                entry.dedup();
            }
        }
        for (_, name) in self.pending_enums.drain(..) {
            for target in &fold {
                self.enums.entry(target.clone()).or_default().insert(name.clone());
            }
        }
        let mut out = ShapeFacts {
            records: self.records.clone(),
            bases: self.bases.clone(),
            enums: self.enums.clone(),
            bounds: BTreeMap::new(),
        };
        for (path, root, segments, invoked) in &self.chains {
            if segments.len() < 2 {
                continue;
            }
            let Some(tails) = self.resolve_root(path, root) else {
                continue;
            };
            for typ in tails {
                let entry = out.bounds.entry(typ.clone()).or_default();
                // The field candidate is the second segment — EXCEPT when
                // the whole chain is an invocation of a direct member
                // (`evt.ToString()`, `evt.GetType()`): that tail is a method
                // name and joins no field (issue #46).
                if !(*invoked && segments.len() == 2) {
                    entry.0.insert(segments[1].clone());
                }
                // The projected field is the segment BEFORE the first
                // `ToString` call, at any hop (scenario 07): hop 1 keeps
                // the shipped shape (the field is also a bound field);
                // a deeper hop joins `projected` alone.
                if let Some(pos) = (2..segments.len()).find(|&i| segments[i] == "ToString") {
                    entry.1.insert(segments[pos - 1].clone());
                }
            }
        }
        for (path, root, field, direct) in &self.string_comparisons {
            let Some(tails) = self.resolve_root(path, root) else {
                continue;
            };
            for typ in tails {
                let entry = out.bounds.entry(typ.clone()).or_default();
                // `direct` (scenario 07): a two-segment access names a
                // bound field AND projects it; a property-hop access
                // projects its last segment only — never a direct
                // reference the record must declare.
                if *direct {
                    entry.0.insert(field.clone());
                }
                entry.1.insert(field.clone());
            }
        }
        out.bounds.retain(|_, (fields, _)| !fields.is_empty());
        out
    }
}

/// True when the declaration's modifiers carry the given keyword (`record`
/// rides the class rule in this grammar — see the module header).
fn has_modifier(node: &Node<'_>, source: &str, keyword: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| {
        child.kind() == "modifier" && node_text(&child, source).trim() == keyword
    })
}

/// Members carried by a record body's field or property declaration.
fn member_shape(node: &Node<'_>, source: &str, members: &mut Vec<(String, String)>) {
    let Some(type_node) = node.child_by_field_name("type") else {
        return;
    };
    let field_type = node_text(&type_node, source).trim().to_string();
    if let Some(name) = node.child_by_field_name("name") {
        members.push((node_text(&name, source).to_string(), field_type.clone()));
        return;
    }
    let mut cursor = node.walk();
    for declarator in node.named_children(&mut cursor) {
        if declarator.is_named() && declarator.kind() == "variable_declarator"
            && let Some(name) = declarator.child_by_field_name("name")
        {
            members.push((node_text(&name, source).to_string(), field_type.clone()));
        }
    }
}

/// Flatten a member-access chain through its receiver side: identifier and
/// member-access nodes contribute their names; anything deeper (an
/// invocation receiver, `this`, a parenthesized expression) contributes
/// nothing, so a chain rooted anywhere but a bare identifier never joins.
fn flatten_chain(node: &Node<'_>, source: &str, segments: &mut Vec<String>) {
    match node.kind() {
        "identifier" => segments.push(node_text(node, source).to_string()),
        "member_access_expression" => {
            if let Some(receiver) = node.child_by_field_name("expression") {
                flatten_chain(&receiver, source, segments);
            }
            if let Some(name) = node.child_by_field_name("name") {
                segments.push(node_text(&name, source).to_string());
            }
        }
        _ => {}
    }
}

/// One operand of an equality is a member access on a bare identifier and
/// the other is a string literal: the field is bound AS A STRING on this
/// side (`order.Status == "Placed"`). A multi-segment access names the
/// string-bound field at its LAST segment (`order.Data.Status == "…"`
/// projects `Status`, scenario 07) with `direct = false`, so the census
/// reads it as a projection of a nested field and never as a direct
/// reference the record must declare; an access whose receiver chain is
/// not rooted at a bare identifier derives nothing, M5 silence.
fn string_compared_field(node: &Node<'_>, source: &str) -> Option<(String, String, bool)> {
    let left = node.child_by_field_name("left")?;
    let right = node.child_by_field_name("right")?;
    for (access, other) in [(&left, &right), (&right, &left)] {
        if other.kind() != "string_literal" {
            continue;
        }
        if access.kind() != "member_access_expression" {
            continue;
        }
        let mut segments = Vec::new();
        flatten_chain(access, source, &mut segments);
        let (first, field) = (segments.first()?, segments.last()?.clone());
        if segments.len() < 2 {
            continue;
        }
        let direct = segments.len() == 2;
        return Some((first.clone(), field, direct));
    }
    None
}

/// The simple name of a written type: generics cut at `<`, a dotted name at
/// its last segment (`List<Item>` and `App.Contracts.OrderPlaced` yield
/// `Item` and `OrderPlaced`).
fn type_tail(text: &str) -> String {
    let plain = text.split('<').next().unwrap_or(text).trim();
    plain.rsplit('.').next().unwrap_or(plain).trim().to_string()
}

/// The literal-candidate feed (workplan visibility_phase2 scenario 03, the
/// sixth driver): one candidate per plain string-literal argument position of
/// every `invocation_expression` in the tree, carrying the callee's SIMPLE
/// name (the method name as the source spells it) and the 0-based argument
/// position — the shape the ts/rust/go/python/kotlin feeds ship. The walk is
/// vocabulary-free (A6): names ride as spelled, the engine matcher verbs
/// decide equality at the resolution site, no name is read here. A named
/// argument (`name:` form) states no positional candidate; non-plain strings
/// (interpolated, verbatim, raw, escaped, char literals) state none — the
/// feed is literal-only, the same floor the other drivers state.
pub(crate) fn literal_candidates(source: &str) -> Vec<(LiteralCandidateAnchor, String)> {
    let Ok(tree) = parse_tree(Language::Csharp, source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_call_candidates(&tree.root_node(), source, &mut out);
    out
}

fn collect_call_candidates(node: &Node<'_>, source: &str, out: &mut Vec<(LiteralCandidateAnchor, String)>) {
    if node.kind() == "invocation_expression" {
        let callee = node
            .child_by_field_name("function")
            .and_then(|target| simple_callee_name(target, source));
        if let Some(callee) = callee
            && let Some(arguments) = node.child_by_field_name("arguments")
        {
            let mut cursor = arguments.walk();
            for (position, argument) in arguments.named_children(&mut cursor).enumerate() {
                // The named-argument form spells an argument name; the
                // position is not what the call means — no candidate stated.
                if argument.child_by_field_name("name").is_some() {
                    continue;
                }
                let mut argument_cursor = argument.walk();
                let value = argument
                    .named_children(&mut argument_cursor)
                    .find_map(|child| plain_string_value(&child, source));
                if let Some(value) = value {
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
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_candidates(&child, source, out);
    }
}

/// The callee's simple name: an `identifier` callee as spelled, the method
/// segment of a member-access or qualified chain (the chain's dot parts are
/// receiver noise for the matcher); anything else (a lambda target, a
/// generic instantiation, an index) states no name and no candidate.
fn simple_callee_name(node: Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(&node, source).to_string()),
        "member_access_expression" | "qualified_name" => node
            .child_by_field_name("name")
            .filter(|name| name.kind() == "identifier")
            .map(|name| node_text(&name, source).to_string()),
        _ => None,
    }
}

/// The value of a PLAIN string literal: the exact concatenation of the
/// `string_literal_content` pieces of a `string_literal` node. Any escape
/// sequence makes the value not exact, so nothing is stated (the kotlin
/// interpolation law reused); every other literal form
/// (`verbatim_string_literal`, `raw_string_literal`, char, interpolated)
/// states no plain value here.
fn plain_string_value(node: &Node<'_>, source: &str) -> Option<String> {
    if node.kind() != "string_literal" {
        return None;
    }
    let mut value = String::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "string_literal_content" => value.push_str(node_text(&child, source)),
            _ => return None,
        }
    }
    Some(value)
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
