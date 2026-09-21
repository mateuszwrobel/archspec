//! Parsing primitives shared by the `archspec` binary's engines: `syn`-based
//! helpers for paths, `use` trees, visibility, and cfg attributes.

use std::collections::BTreeSet;
use syn::{Meta, UseTree};

pub fn is_public(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

pub fn is_reserved_segment(segment: &str) -> bool {
    matches!(segment, "crate" | "self" | "super")
}

/// The `ident` segments of a `syn::Path`, in order. Shared by every walker
/// that needs a path's module/crate prefix.
pub fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect()
}

/// Leaf kind of a flattened `use` tree branch: a concrete named item (name or
/// rename — the ORIGINAL ident is kept) or a glob (the path is the prefix; the
/// imported set is unenumerable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseLeafKind {
    Name,
    Glob,
}

/// The single `use`-tree walker shared by the `scan` driver and the `inspect`
/// scanner: enumerate every leaf of a `use` tree as (dotted segment path, leaf
/// kind). Groups expand into their leaves; renames keep the original
/// (pre-rename) ident; globs yield the prefix.
pub fn use_tree_leaves(
    tree: &UseTree,
    prefix: Vec<String>,
    out: &mut Vec<(Vec<String>, UseLeafKind)>,
) {
    match tree {
        UseTree::Path(path) => {
            let mut next = prefix;
            next.push(path.ident.to_string());
            use_tree_leaves(&path.tree, next, out);
        }
        UseTree::Name(name) => {
            let mut next = prefix;
            next.push(name.ident.to_string());
            out.push((next, UseLeafKind::Name));
        }
        UseTree::Rename(rename) => {
            let mut next = prefix;
            next.push(rename.ident.to_string());
            out.push((next, UseLeafKind::Name));
        }
        UseTree::Glob(_) => out.push((prefix, UseLeafKind::Glob)),
        UseTree::Group(group) => {
            for item in &group.items {
                use_tree_leaves(item, prefix.clone(), out);
            }
        }
    }
}

/// Concrete paths of a `use` tree (groups expanded, globs as their prefix
/// path) — the path-only projection of `use_tree_leaves`.
pub fn flatten_use_tree(tree: &UseTree, prefix: Vec<String>, out: &mut Vec<Vec<String>>) {
    let mut leaves = Vec::new();
    use_tree_leaves(tree, prefix, &mut leaves);
    out.extend(leaves.into_iter().map(|(path, _)| path));
}

/// The concrete names a `pub use` re-exports. Only `Name` and `Rename` nodes
/// yield an exported name (renames export the NEW name); a glob is a wildcard
/// re-export and is not enumerable as a single public export.
pub fn pub_use_names(tree: &UseTree, out: &mut BTreeSet<String>) {
    match tree {
        UseTree::Path(path) => pub_use_names(&path.tree, out),
        UseTree::Name(name) => {
            out.insert(name.ident.to_string());
        }
        UseTree::Rename(rename) => {
            out.insert(rename.rename.to_string());
        }
        UseTree::Glob(_) => {}
        UseTree::Group(group) => {
            for item in &group.items {
                pub_use_names(item, out);
            }
        }
    }
}

/// The glob token a segment prefix renders to: `["a","b"]` -> `a::b::*`, an
/// empty prefix (a bare `*`) -> `*`. Shared by the token reporter below and by
/// the scan driver, which resolves a glob's segments to a module target and
/// must report the same token the parser saw.
pub fn glob_token(segments: &[String]) -> String {
    if segments.is_empty() {
        "*".to_string()
    } else {
        format!("{}::*", segments.join("::"))
    }
}

/// The glob prefixes a `pub use` re-exports, as segment paths (`a::b::*` ->
/// `["a","b"]`, a bare `*` -> `[]`). Named/renamed exports produce nothing
/// here — they belong to `pub_use_names`.
pub fn pub_use_glob_paths(tree: &UseTree, prefix: &[String], out: &mut Vec<Vec<String>>) {
    match tree {
        UseTree::Path(path) => {
            let mut next = prefix.to_vec();
            next.push(path.ident.to_string());
            pub_use_glob_paths(&path.tree, &next, out);
        }
        UseTree::Glob(_) => out.push(prefix.to_vec()),
        UseTree::Group(group) => {
            for item in &group.items {
                pub_use_glob_paths(item, prefix, out);
            }
        }
        UseTree::Name(_) | UseTree::Rename(_) => {}
    }
}

/// The glob tokens a `pub use` re-exports: `a::b::*` (or `*` with no prefix).
/// Named/renamed exports produce nothing here — they belong to
/// `pub_use_names`.
pub fn pub_use_globs(tree: &UseTree, prefix: &[String], out: &mut BTreeSet<String>) {
    let mut paths = Vec::new();
    pub_use_glob_paths(tree, prefix, &mut paths);
    for path in paths {
        out.insert(glob_token(&path));
    }
}

/// True when the item carries ANY `#[cfg(...)]` attribute, whatever its
/// predicate. Resolution of a glob re-export must not depend on whether a
/// predicate is parseable: a cfg-gated declaration is configuration-dependent
/// by definition, so every shape (`feature = "..."`, `unix`, `test`,
/// `all(not(feature = "..."))`, …) blocks it the same way.
pub fn has_cfg(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg"))
}

/// The `feature = "..."` gate name of a set of attributes, when an attribute
/// is exactly `#[cfg(feature = "name")]` or `#[cfg(feature("name"))]`.
/// Compound forms (`cfg(all(feature = "x", unix))`) carry no single gate name
/// and yield `None`.
pub fn cfg_feature_name(attributes: &[syn::Attribute]) -> Option<String> {
    attributes.iter().find_map(|attribute| {
        if !attribute.path().is_ident("cfg") {
            return None;
        }
        let meta = attribute.parse_args::<syn::Meta>().ok()?;
        match meta {
            Meta::NameValue(ref name_value) if name_value.path.is_ident("feature") => {
                expr_str(&name_value.value)
            }
            Meta::List(ref list) if list.path.is_ident("feature") => {
                Some(list.tokens.to_string().trim_matches('"').to_string())
            }
            _ => None,
        }
    })
}

/// True when the item's attributes gate it on the `test` cfg such that it is
/// compiled ONLY in a test build: a bare `#[cfg(test)]`, or a `#[cfg(all(...))]`
/// chain listing `test` among its required conjuncts (e.g.
/// `cfg(all(test, unix))`). A `cfg(any(test, ...))` is NOT test-gated — it can
/// compile in a production build — and neither is a predicate the parser cannot
/// classify: both report `false` so a module is treated as production unless it
/// is proven test-gated (fail closed).
pub fn is_cfg_test(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<syn::Meta>()
                .is_ok_and(|meta| cfg_requires_test(&meta))
    })
}

/// True when a parsed `cfg` predicate can hold only when `test` is enabled.
/// `test` => true; `all(...)` => any conjunct requires test; `any(...)` => only
/// when every disjunct requires test; `not(...)` and any other shape => false.
fn cfg_requires_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            let inner = cfg_list_inner(list);
            if list.path.is_ident("all") {
                inner.iter().any(cfg_requires_test)
            } else if list.path.is_ident("any") {
                !inner.is_empty() && inner.iter().all(cfg_requires_test)
            } else {
                false
            }
        }
        syn::Meta::NameValue(_) => false,
    }
}

/// The comma-separated predicates inside a `cfg` predicate list (`all(a, b)` =>
/// `[a, b]`). Unparseable token trees yield an empty vector (fail closed).
fn cfg_list_inner(list: &syn::MetaList) -> Vec<syn::Meta> {
    syn::parse::Parser::parse2(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        list.tokens.clone(),
    )
    .map(|parsed| parsed.into_iter().collect())
    .unwrap_or_default()
}

/// Extract the string value of a `syn::Expr` literal (e.g. the `"tauri"` in
/// `cfg(feature = "tauri")`), trimming surrounding quotes.
fn expr_str(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(value) => Some(value.value()),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn use_tree(source: &str) -> UseTree {
        syn::parse_str::<syn::ItemUse>(source)
            .expect("valid use item")
            .tree
    }

    fn leaves(source: &str) -> Vec<(Vec<String>, UseLeafKind)> {
        let mut out = Vec::new();
        use_tree_leaves(&use_tree(source), Vec::new(), &mut out);
        out
    }

    #[test]
    fn use_tree_leaves_expands_groups_keeps_rename_original_and_globs_as_prefix() {
        assert_eq!(
            leaves("use a::{b, c as d, e::*};"),
            vec![
                (vec!["a".to_string(), "b".to_string()], UseLeafKind::Name),
                (vec!["a".to_string(), "c".to_string()], UseLeafKind::Name),
                (vec!["a".to_string(), "e".to_string()], UseLeafKind::Glob),
            ],
            "rename leaf keeps the original ident; glob yields the prefix"
        );
        assert_eq!(
            leaves("use a::b::c;"),
            vec![(vec!["a".to_string(), "b".to_string(), "c".to_string()], UseLeafKind::Name)]
        );
        assert_eq!(
            leaves("use foo;"),
            vec![(vec!["foo".to_string()], UseLeafKind::Name)]
        );
    }

    #[test]
    fn flatten_use_tree_is_the_path_projection_of_leaves() {
        let mut flattened = Vec::new();
        flatten_use_tree(&use_tree("use a::{b, c as d, e::*};"), Vec::new(), &mut flattened);
        assert_eq!(
            flattened,
            leaves("use a::{b, c as d, e::*};")
                .into_iter()
                .map(|(path, _)| path)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn pub_use_names_exports_new_names_and_skips_globs() {
        let mut out = BTreeSet::new();
        pub_use_names(&use_tree("pub use a::{b, c as d};"), &mut out);
        pub_use_names(&use_tree("pub use core::types::*;"), &mut out);
        assert_eq!(out, BTreeSet::from(["b".to_string(), "d".to_string()]));
    }

    #[test]
    fn pub_use_globs_reports_prefixed_glob_tokens() {
        let mut out = BTreeSet::new();
        pub_use_globs(&use_tree("pub use a::b::*;"), &[], &mut out);
        pub_use_globs(&use_tree("pub use m::{n::*, q};"), &[], &mut out);
        assert_eq!(
            out,
            BTreeSet::from(["a::b::*".to_string(), "m::n::*".to_string()])
        );
    }

    #[test]
    fn pub_use_glob_paths_yields_segment_prefixes_matching_the_tokens() {
        let mut out = Vec::new();
        pub_use_glob_paths(&use_tree("pub use a::b::*;"), &[], &mut out);
        pub_use_glob_paths(&use_tree("pub use m::{n::*, q};"), &[], &mut out);
        pub_use_glob_paths(&use_tree("pub use *;"), &[], &mut out);
        assert_eq!(
            out,
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["m".to_string(), "n".to_string()],
                Vec::new(),
            ],
            "groups expand, named leaves produce nothing, a bare glob is empty"
        );
        let mut tokens = BTreeSet::new();
        for tree in ["pub use a::b::*;", "pub use m::{n::*, q};", "pub use *;"] {
            pub_use_globs(&use_tree(tree), &[], &mut tokens);
        }
        let from_paths: BTreeSet<String> =
            out.iter().map(|segments| glob_token(segments)).collect();
        assert_eq!(
            tokens, from_paths,
            "the token reporter and the segment walker agree"
        );
        assert_eq!(glob_token(&[]), "*");
    }

    #[test]
    fn has_cfg_recognizes_any_cfg_predicate() {
        for source in [
            "#[cfg(feature = \"png\")] mod png;",
            "#[cfg(all(feature = \"png\", unix))] mod png;",
            "#[cfg(unix)] mod png;",
            "#[cfg(test)] mod png;",
        ] {
            let gated: syn::ItemMod = syn::parse_str(source).unwrap();
            assert!(has_cfg(&gated.attrs), "{source} carries a cfg");
        }
        let plain: syn::ItemMod = syn::parse_str("mod plain;").unwrap();
        assert!(!has_cfg(&plain.attrs));
        let doc: syn::ItemMod = syn::parse_str("#[doc = \"x\"] mod documented;").unwrap();
        assert!(!has_cfg(&doc.attrs), "a doc attribute is not a cfg");
    }

    #[test]
    fn is_cfg_test_recognizes_test_gated_modules() {
        for source in [
            "#[cfg(test)] mod t;",
            "#[cfg(all(test, unix))] mod t;",
            "#[cfg(all(feature = \"x\", test))] mod t;",
            "#[cfg(all(test, all(unix, test)))] mod t;",
        ] {
            let gated: syn::ItemMod = syn::parse_str(source).unwrap();
            assert!(is_cfg_test(&gated.attrs), "{source} is test-gated");
        }
        for source in [
            "mod t;",
            "#[cfg(feature = \"png\")] mod t;",
            "#[cfg(unix)] mod t;",
            "#[cfg(any(test, unix))] mod t;",
            "#[cfg(not(test))] mod t;",
            "#[doc = \"x\"] mod t;",
        ] {
            let module: syn::ItemMod = syn::parse_str(source).unwrap();
            assert!(
                !is_cfg_test(&module.attrs),
                "{source} is NOT test-gated (compiles in production or unproven)"
            );
        }
    }

    #[test]
    fn path_segments_lists_idents_in_order() {
        let path = syn::parse_str::<syn::Path>("crate::render::mermaid_node").unwrap();
        assert_eq!(
            path_segments(&path),
            vec!["crate".to_string(), "render".to_string(), "mermaid_node".to_string()]
        );
    }

    #[test]
    fn cfg_feature_name_only_recognizes_single_feature_gates() {
        let gated: syn::ItemMod =
            syn::parse_str("#[cfg(feature = \"png\")] mod png;").unwrap();
        assert_eq!(cfg_feature_name(&gated.attrs), Some("png".to_string()));
        let compound: syn::ItemMod =
            syn::parse_str("#[cfg(all(feature = \"png\", unix))] mod png;").unwrap();
        assert_eq!(cfg_feature_name(&compound.attrs), None);
        let plain: syn::ItemMod = syn::parse_str("mod plain;").unwrap();
        assert_eq!(cfg_feature_name(&plain.attrs), None);
    }

    #[test]
    fn is_public_and_reserved_segment_primitives() {
        let public: syn::ItemFn = syn::parse_str("pub fn f() {}").unwrap();
        let private: syn::ItemFn = syn::parse_str("fn f() {}").unwrap();
        let crate_only: syn::ItemFn = syn::parse_str("pub(crate) fn f() {}").unwrap();
        assert!(is_public(&public.vis));
        assert!(!is_public(&private.vis));
        assert!(!is_public(&crate_only.vis));
        assert!(is_reserved_segment("crate"));
        assert!(is_reserved_segment("self"));
        assert!(is_reserved_segment("super"));
        assert!(!is_reserved_segment("graph"));
    }
}
