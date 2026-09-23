//! The Go module tier (workplan archspec_syntax_backends,
//! US 11 "Go intra-module package edges").
//!
//! CLI-level behaviour of `scan` on single-module Go trees:
//! every package of a module is a module (workplan decision "Go visibility"),
//! so an import of a sibling package of the same module records a MODULE edge
//! addressed at the package's full import path with `/` -> `::` (the same
//! vocabulary the go.work edges use), owned by the importing module.
//!
//! Shared prep is one code path: go.mod module
//! path, package-directory discovery, the `*_test.go` exclusion and the
//! external attribution rule (stdlib list + non-module-prefix imports) are
//! fixed; the per-file import facts come from the
//! grammar (`import_spec` path strings) and — since US 12 —
//! the exported selector symbols riding those edges: a `selector_expression`
//! whose `operand:` is an `identifier` matching a file qualifier (import
//! alias or package tail) contributes its capitalized `field:` to the edge
//! that import created; unexported selectors keep the edge symbolless (casing
//! is the visibility rule), dot-imported calls are indistinguishable bare
//! identifiers and contribute nothing, and external/stdlib selectors stay
//! out of the module tier exactly like their import-driven edges.
//!
//! Since US 13 the SAME rules extend to `go.work` trees with 2+ members
//! (workplan scenario "Go workspace edges keep working"): cross-member edges
//! keep their import-path endpoints and unit and additionally carry the exported
//! selector symbols, and within-member package imports project module edges
//! exactly like the single-module path.
//!
//! The US 17 audit finding D-1 adds the `update` seed-shape round-trip on
//! single-module trees: the model carries module facts without
//! go.work, so the seed lists the tree's packages (not the workspace-member
//! shape) and `report` exits 0 with zero findings.
//!
//! Canonical acceptance rows: `docs/archspec/commands/scan/acceptance.md`
//! row 56 (Go intra-module package edges: edge with importing module as
//! unit), row 57 (Go selector symbols
//! honour casing, including the dot-import and chained-call exclusions) and
//! row 58 (Go workspace edges: cross-member endpoints plus symbols,
//! within-member package edges).

mod common;
#[allow(dead_code)]
mod shared;

use common::{stderr, stdout, Fixture};
use serde_json::Value;
use shared::driver::materialize_go_workspace;

/// The canonical Gherkin tree: one module `example.com/demo` whose package
/// `store` imports sibling package `format`.
fn gherkin_fixture() -> Fixture {
    go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {}\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ])
}

/// One single-module Go tree (`example.com/demo`) with the given .go files.
fn go_fixture(files: &[(&str, &str)]) -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    for (path, content) in files {
        fixture.write(path, content);
    }
    fixture
}

/// The module edges of a model as comparable tuples.
fn edges(model: &Value) -> Vec<(String, String, String, Vec<String>)> {
    model["module_edges"]
        .as_array()
        .expect("module_edges must be an array")
        .iter()
        .map(|edge| {
            (
                edge["unit"].as_str().expect("unit").to_string(),
                edge["from"].as_str().expect("from").to_string(),
                edge["to"].as_str().expect("to").to_string(),
                edge["symbols"]
                    .as_array()
                    .expect("symbols")
                    .iter()
                    .map(|s| s.as_str().expect("symbol").to_string())
                    .collect(),
            )
        })
        .collect()
}

fn scan(fixture: &Fixture) -> Value {
    let output = fixture.run(&["scan"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`scan` must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "stderr must be empty");
    serde_json::from_str(&stdout(&output)).expect("stdout must be JSON")
}

fn scan_bytes(fixture: &Fixture) -> Vec<u8> {
    let output = fixture.run(&["scan"]);
    assert_eq!(output.status.code(), Some(0), "scan must exit 0");
    output.stdout
}

/// Gherkin clause 1 (row 56): the sibling import records a module edge from
/// the `store` module path to the `format` module path (`/` -> `::`, the
/// go.work-edge vocabulary), `unit` = the importing module; the fixture has
/// no selector call sites, so symbols stay empty.
#[test]
fn edge_addresses_sibling_package_with_importing_module_as_unit() {
    let fixture = gherkin_fixture();
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            Vec::new()
        )],
        "one module edge: unit = importing module, endpoints = package import paths with / -> ::"
    );
}

/// Nested packages address at their full import-path depth: `x/y` importing
/// `z` AND the module-root package produces `::`-paths for both, the root
/// import included as a module edge (the root dir is a discovered package).
#[test]
fn nested_subdir_packages_address_at_full_depth() {
    let fixture = go_fixture(&[
        (
            "main.go",
            "package demo\n\nfunc Root() {}\n",
        ),
        (
            "x/y/y.go",
            "package y\n\nimport (\n\t\"example.com/demo\"\n\t\"example.com/demo/z\"\n)\n\nfunc Y() {}\n",
        ),
        ("z/z.go", "package z\n\nfunc Z() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![
            (
                "example.com/demo".to_string(),
                "example.com::demo::x::y".to_string(),
                "example.com::demo".to_string(),
                Vec::new()
            ),
            (
                "example.com/demo".to_string(),
                "example.com::demo::x::y".to_string(),
                "example.com::demo::z".to_string(),
                Vec::new()
            ),
        ],
        "one edge per imported sibling, addressed at the full path depth, sorted by (from, to)"
    );
}

/// An aliased import (`f "path"`) records the same path-driven edge; the
/// alias only feeds the qualifier map the selector attribution keys on.
#[test]
fn aliased_import_records_the_same_path_driven_edge() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport f \"example.com/demo/format\"\n\nfunc Save() {}\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            Vec::new()
        )],
        "the import path decides the edge; the alias is invisible at the edge level"
    );
}

/// Blank (`_ "path"`) and dot (`. "path"`) imports are real dependencies:
/// they record module edges exactly like plain imports.
#[test]
fn blank_and_dot_imports_record_edges() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport (\n\t_ \"example.com/demo/regist\"\n\t. \"example.com/demo/helper\"\n)\n\nfunc Save() {}\n",
        ),
        ("regist/regist.go", "package regist\n\nfunc Init() {}\n"),
        ("helper/helper.go", "package helper\n\nfunc Help() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![
            (
                "example.com/demo".to_string(),
                "example.com::demo::store".to_string(),
                "example.com::demo::helper".to_string(),
                Vec::new()
            ),
            (
                "example.com/demo".to_string(),
                "example.com::demo::store".to_string(),
                "example.com::demo::regist".to_string(),
                Vec::new()
            ),
        ],
        "blank and dot specs are dependency facts, sorted by to"
    );
}

/// Stdlib and external (non-module-prefix) imports produce NO module edges,
/// and the external tier (`external` + `module_external`) comes from
/// the shared attribution rule, not from the import-path edges.
#[test]
fn stdlib_and_external_imports_edge_nothing_and_attribute_externals() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport (\n\t\"fmt\"\n\t\"os\"\n\t\"github.com/shop/sdk\"\n\t\"example.com/demo/format\"\n)\n\nfunc Save() { fmt.Sprint(os.Args) }\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    let model = scan(&fixture);
    assert_eq!(
        edges(&model),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            Vec::new()
        )],
        "exactly ONE module edge — the sibling import; stdlib and externals contribute none"
    );
    let external: Vec<&str> = model["external"]
        .as_array()
        .expect("external")
        .iter()
        .map(|n| n.as_str().expect("import path"))
        .collect();
    assert_eq!(
        external,
        ["github.com/shop/sdk"],
        "the non-module-prefix, non-stdlib import is the external fact"
    );
}

/// Duplicate specs (two blocks in one file, the same import in another file)
/// record the edge exactly ONCE — the edge set is keyed, like the csharp side.
#[test]
fn duplicate_specs_across_blocks_and_files_record_the_edge_once() {
    let fixture = go_fixture(&[
        (
            "store/a.go",
            "package store\n\nimport (\n\t\"example.com/demo/format\"\n)\n\nimport (\n\t\"example.com/demo/format\"\n)\n\nfunc Save() {}\n",
        ),
        (
            "store/b.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Load() {}\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)).len(),
        1,
        "the repeated sibling import collapses to one edge"
    );
}

/// Whole-model determinism: two runs on a multi-package tree emit
/// byte-identical JSON (deterministic sort by (unit, from, to)).
#[test]
fn model_is_byte_identical_across_runs() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {}\n",
        ),
        (
            "format/format.go",
            "package format\n\nimport \"example.com/demo/util\"\n\nfunc Title() {}\n",
        ),
        ("util/util.go", "package util\n\nfunc Pad() {}\n"),
    ]);
    assert_eq!(
        scan_bytes(&fixture),
        scan_bytes(&fixture),
        "the scan is byte-identical across runs"
    );
}

/// Error tolerance mirrors the csharp policy: a file whose tail the grammar
/// cannot parse (ERROR region) still contributes its located imports — no
/// error exit.
#[test]
fn error_tolerant_tree_still_yields_edges() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            Vec::new()
        )],
        "the damaged file's surviving import still produces the edge"
    );
}

/// The from==to guard: a spec naming the importing package's own path (broken
/// Go, but parseable) contributes no module edge — no self-edge is invented.
#[test]
fn self_import_path_contributes_no_module_edge() {
    let fixture = go_fixture(&[(
        "store/store.go",
        "package store\n\nimport \"example.com/demo/store\"\n\nfunc Save() {}\n",
    )]);
    assert!(
        edges(&scan(&fixture)).is_empty(),
        "a package never becomes a module edge to itself"
    );
}

/// The `_test.go` exclusion rule: the scan EXCLUDES the test tier
/// (is_test_file), so test-file imports are no production fact — the package
/// walker never sees them: no edges, no externals.
#[test]
fn test_files_contribute_nothing() {
    let fixture = go_fixture(&[
        ("store/store.go", "package store\n\nfunc Save() {}\n"),
        (
            "store/store_test.go",
            "package store\n\nimport (\n\t\"example.com/demo/format\"\n\t\"github.com/test/only\"\n)\n\nfunc TestSave() {}\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    let model = scan(&fixture);
    assert!(
        edges(&model).is_empty(),
        "test-tier imports are no production fact"
    );
    assert_eq!(
        model["external"].as_array().expect("external").len(),
        0,
        "the test-only external contributes nothing"
    );
}

// ---------------------------------------------------------------------------
// US 12 — selector symbols honour casing (acceptance row 57)
// ---------------------------------------------------------------------------

/// The canonical Gherkin tree of "Go selector symbols honour casing": package
/// `store` imports sibling `format` and calls `format.Title()` (exported) and
/// `format.helper()` (unexported).
fn selector_fixture() -> Fixture {
    go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n\tformat.Title()\n\tformat.helper()\n}\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n\nfunc helper() {}\n"),
    ])
}

/// Gherkin (row 57): the capitalized selector becomes the edge's only symbol,
/// the lowercased one stays invisible, and the edge exists exactly once.
#[test]
fn selector_casing_puts_only_exported_names_on_the_edge() {
    let fixture = selector_fixture();
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            vec!["Title".to_string()]
        )],
        "symbols == [Title], helper absent, one edge"
    );
}

/// An aliased qualifier (`f "…/format"`) attributes exported selectors to the
/// same edge the import created — the alias is the file's name for the path.
#[test]
fn aliased_qualifier_selectors_join_the_import_driven_edge() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport f \"example.com/demo/format\"\n\nfunc Save() {\n\tf.Rename()\n\tf.skip()\n}\n",
        ),
        ("format/format.go", "package format\n\nfunc Rename() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            vec!["Rename".to_string()]
        )],
        "the alias resolves through the qualifier map to the import's edge"
    );
}

/// A blank import (`_ "…/regist"`) records the edge but is not referenceable
/// in Go source — no selector can ever name it, so its edge carries no
/// symbols (the grammar keeps `_` out of selector operands by the language
/// rule, and the attribution skips the `_` qualifier anyway).
#[test]
fn blank_import_edge_carries_no_selector_symbols() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport (\n\t_ \"example.com/demo/regist\"\n\t\"example.com/demo/format\"\n)\n\nfunc Save() { format.Title() }\n",
        ),
        ("regist/regist.go", "package regist\n\nfunc Init() {}\n"),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![
            (
                "example.com/demo".to_string(),
                "example.com::demo::store".to_string(),
                "example.com::demo::format".to_string(),
                vec!["Title".to_string()]
            ),
            (
                "example.com/demo".to_string(),
                "example.com::demo::store".to_string(),
                "example.com::demo::regist".to_string(),
                Vec::new()
            ),
        ],
        "the blank-import edge exists (import-driven) with empty symbols"
    );
}

/// A dot import (`. "…/helper"`) puts the package's names into the file scope
/// directly: `Help()` is a bare identifier call, indistinguishable from a
/// local function — selector attribution is anchored on qualifiers, so a
/// dot-import edge carries NO symbols (same anchored-prefix philosophy as the
/// csharp bare-name rule). The edge itself stays (import-driven).
#[test]
fn dot_import_bare_calls_are_indistinguishable_and_contribute_no_symbols() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport . \"example.com/demo/helper\"\n\nfunc Save() { Help() }\n",
        ),
        ("helper/helper.go", "package helper\n\nfunc Help() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::helper".to_string(),
            Vec::new()
        )],
        "the dot-import edge records, the bare exported call attributes nothing"
    );
}

/// Selectors qualified by external (non-module) or stdlib imports add nothing
/// module-side: no new edges, no symbols — the external tier stays purely
/// import-driven.
#[test]
fn external_and_stdlib_selectors_add_nothing_module_side() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport (\n\t\"strings\"\n\t\"github.com/shop/sdk\"\n\t\"example.com/demo/format\"\n)\n\nfunc Save() {\n\tstrings.Join(nil, \"\")\n\tsdk.Call()\n\tformat.Title()\n}\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    let model = scan(&fixture);
    assert_eq!(
        edges(&model),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            vec!["Title".to_string()]
        )],
        "exactly the sibling edge gets symbols; stdlib/external selectors are \
         invisible module-side and invent no external edges"
    );
    let external: Vec<&str> = model["external"]
        .as_array()
        .expect("external")
        .iter()
        .map(|n| n.as_str().expect("import path"))
        .collect();
    assert_eq!(
        external,
        ["github.com/shop/sdk"],
        "selectors against externals or stdlib add nothing to the \
         import-driven external tier"
    );
}

/// Noise guard: a method call on a LOCAL value (`v.Method()` — the operand is
/// a variable, not a qualifier) attributes nothing, and a chained call
/// (`format.Title().String()` — the trailing operand is a call_expression,
/// not an identifier) attributes only the FIRST matched selector's field.
#[test]
fn local_values_and_chained_call_tails_attribute_nothing_extra() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n\tv := format.New()\n\tv.Method()\n\tformat.Title().String()\n}\n",
        ),
        ("format/format.go", "package format\n\nfunc New() *T { return nil }\nfunc Title() string { return \"\" }\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            vec!["New".to_string(), "Title".to_string()]
        )],
        "New and Title from identifier-qualified selectors only — neither \
         Method (local variable) nor String (call-expression operand)"
    );
}

/// Struct literal field KEYS are not selectors: grammar-wise `Title:` inside
/// `format.Options{Title: 1}` is a `keyed_element` key (plain identifier), and
/// the qualified type `format.Options` parses as a `qualified_type` node
/// (package_identifier + type_identifier), not a selector_expression — so the
/// literal contributes no symbols at all.
#[test]
fn struct_literal_field_keys_and_qualified_types_are_not_selectors() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {\n\to := format.Options{Title: 1}\n\tvar s format.Options\n\t_ = o\n\t_ = s\n}\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    assert_eq!(
        edges(&scan(&fixture)),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            Vec::new()
        )],
        "the edge is import-driven with no symbols: neither the struct key \
         `Title` nor the qualified type name is a selector field"
    );
}

/// Many call sites — repeated in several functions and several files — merge
/// into ONE edge whose symbol set is deduplicated and sorted.
#[test]
fn many_call_sites_merge_into_one_sorted_deduped_symbol_set() {
    let fixture = go_fixture(&[
        (
            "store/a.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() { format.Zeta(); format.Alpha() }\nfunc Load() { format.Zeta() }\n",
        ),
        (
            "store/b.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Peek() { format.Zeta() }\n",
        ),
        ("format/format.go", "package format\n\nfunc Alpha() {}\nfunc Zeta() {}\n"),
    ]);
    let model = scan(&fixture);
    assert_eq!(
        edges(&model),
        vec![(
            "example.com/demo".to_string(),
            "example.com::demo::store".to_string(),
            "example.com::demo::format".to_string(),
            vec!["Alpha".to_string(), "Zeta".to_string()]
        )],
        "one edge, symbols deduped across files and sorted"
    );
    assert_eq!(
        scan_bytes(&fixture),
        scan_bytes(&fixture),
        "symbol sets keep the whole-model byte determinism"
    );
}

/// The go.work tree of `materialize_go_workspace` carries no selector call
/// sites and no within-member imports, so cross-member imports are its only
/// module facts — and those keep their import-path endpoints and unit.
#[test]
fn go_work_tree_without_selector_or_intra_member_facts_keeps_cross_member_edges() {
    let fixture = Fixture::new();
    materialize_go_workspace(&fixture);
    let cross: Vec<(String, String, String)> = edges(&scan(&fixture))
        .into_iter()
        .map(|(unit, from, to, _symbols)| (unit, from, to))
        .collect();
    assert_eq!(
        cross,
        vec![
            (
                "example.com/api".to_string(),
                "example.com::api".to_string(),
                "example.com::store".to_string()
            ),
            (
                "example.com/api".to_string(),
                "example.com::api::internal::handler".to_string(),
                "example.com::store".to_string()
            ),
        ],
        "the cross-member edges keep their import-path endpoints and unit"
    );
}

// ---------------------------------------------------------------------------
// US 13 — workspace edges extended (acceptance row 58)
// ---------------------------------------------------------------------------

/// The canonical Gherkin tree of "Go workspace edges keep working": a `go.work`
/// naming members `example.com/api` and `example.com/store`. Member package
/// `api` imports `example.com/store` across members and calls `store.Get()`
/// (exported) and `store.skip()` (unexported — the casing rule applies to
/// cross-member edges too); member package `api/internal/handler` imports the
/// member root `example.com/api` (WITHIN member) and `example.com/store` (cross
/// member, no selector call sites).
fn workspace_selector_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write(
        "api/api.go",
        "package api\n\nimport \"example.com/store\"\n\nfunc Api() {\n\tstore.Get()\n\tstore.skip()\n}\n",
    );
    fixture.write(
        "api/internal/handler/handler.go",
        "package handler\n\nimport (\n\t\"example.com/api\"\n\t\"example.com/store\"\n)\n\nfunc Handle() { api.Render() }\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write(
        "store/store.go",
        "package store\n\nfunc Get() {}\n\nfunc skip() {}\n",
    );
    fixture
}

/// The (unit, from, to) endpoints of a model's module edges, symbols stripped.
fn endpoints(model: &Value) -> Vec<(String, String, String)> {
    edges(model)
        .into_iter()
        .map(|(unit, from, to, _symbols)| (unit, from, to))
        .collect()
}

/// The canonical cross-member pair shared by the Gherkin tests.
fn cross_edge() -> (String, String, String) {
    (
        "example.com/api".to_string(),
        "example.com::api".to_string(),
        "example.com::store".to_string(),
    )
}

/// Gherkin clause 1 (row 58): every cross-member module edge addresses (unit,
/// from, to) through import-path vocabulary — unit = the importing member
/// module — and the within-member import projects exactly one additional
/// module edge.
#[test]
fn cross_member_edges_keep_import_path_endpoints() {
    let fixture = workspace_selector_fixture();
    assert_eq!(
        endpoints(&scan(&fixture)),
        vec![
            cross_edge(),
            (
                "example.com/api".to_string(),
                "example.com::api::internal::handler".to_string(),
                "example.com::api".to_string()
            ),
            (
                "example.com/api".to_string(),
                "example.com::api::internal::handler".to_string(),
                "example.com::store".to_string()
            ),
        ],
        "the cross-member pair plus the within-member edge, sorted by \
         (unit, from, to) — no endpoint invented beyond the imports"
    );
}

/// Gherkin clause 2 (row 58): the cross-member edge additionally carries the
/// exported selector symbols — the casing rule from row 57 applies unchanged
/// (`store.Get()` joins, `store.skip()` does not), and selectors against
/// members add nothing to OTHER edges they are not qualified by.
#[test]
fn workspace_edges_carry_exported_selector_symbols() {
    let fixture = workspace_selector_fixture();
    assert_eq!(
        edges(&scan(&fixture)),
        vec![
            (
                "example.com/api".to_string(),
                "example.com::api".to_string(),
                "example.com::store".to_string(),
                vec!["Get".to_string()]
            ),
            (
                "example.com/api".to_string(),
                "example.com::api::internal::handler".to_string(),
                "example.com::api".to_string(),
                vec!["Render".to_string()]
            ),
            (
                "example.com/api".to_string(),
                "example.com::api::internal::handler".to_string(),
                "example.com::store".to_string(),
                Vec::new()
            ),
        ],
        "symbols keyed on (from, to): Get on the api->store edge (skip \
         excluded), Render on the within-member edge, the call-site-free \
         handler->store edge symbolless"
    );
}

/// Gherkin clause 3 (row 58): the within-member import projects a module edge
/// addressed exactly like the single-module path (member import paths with
/// `/` -> `::`, unit = the member module) alongside the unit-tier edges and
/// units of the workspace.
#[test]
fn within_member_imports_project_module_edges() {
    let fixture = workspace_selector_fixture();
    let model = scan(&fixture);
    assert!(
        endpoints(&model).contains(&(
            "example.com/api".to_string(),
            "example.com::api::internal::handler".to_string(),
            "example.com::api".to_string()
        )),
        "the within-member import is on the module tier"
    );
    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|unit| unit["name"].as_str().expect("unit name"))
        .collect();
    assert_eq!(
        units,
        [
            "example.com/api",
            "example.com/api/internal/handler",
            "example.com/store"
        ],
        "units stay the member packages named through their module paths"
    );
    assert_eq!(
        model["edges"],
        serde_json::json!([
            { "from": "example.com/api", "to": "example.com/store" },
            { "from": "example.com/api/internal/handler", "to": "example.com/api" },
            { "from": "example.com/api/internal/handler", "to": "example.com/store" }
        ]),
        "the unit-tier edges are the member-level import facts"
    );
}

/// Module-edge assembly determinism on a workspace tree: BTreeSet iteration
/// sorted by (unit, from, to) with sorted symbol sets gives byte-identical
/// repeats.
#[test]
fn go_work_model_is_byte_identical_across_runs() {
    let fixture = workspace_selector_fixture();
    assert_eq!(
        scan_bytes(&fixture),
        scan_bytes(&fixture),
        "the workspace scan is byte-identical across runs"
    );
}

/// The grammar error policy extends to the workspace path: a member file
/// whose tail the grammar cannot parse still contributes its located imports
/// — no error exit (same policy as the single-module path).
#[test]
fn go_work_error_tolerant_member_file_still_yields_edges() {
    let fixture = Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write(
        "api/api.go",
        "package api\n\nimport \"example.com/store\"\n\nfunc Api() {\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");
    assert_eq!(
        endpoints(&scan(&fixture)),
        vec![cross_edge()],
        "the damaged member file's surviving import still produces the \
         cross-member edge"
    );
}

// D-1 (US 17 own-application audit): the `update` seed shape on a single
// go.mod tree. The model carries the
// module tier natively (every package reference projects a module edge) even
// though the tree has no go.work members, so the seed must NOT route through
// the workspace-member shape — that emits a spec with zero `[[module]]`
// blocks, and the following `report` flags every component
// unexpected and exits 1. Routing keys on the model's actual module facts
// plus tree shape: members seed members, package-level facts seed packages.

/// A single-module Go tree with a three-package dependency chain
/// (store -> format -> text, store -> text) — the layered back-end shape in
/// miniature.
fn go_seed_fixture() -> Fixture {
    go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport (\n\t\"example.com/demo/format\"\n\t\"example.com/demo/text\"\n)\n\nfunc Save() {\n\tformat.Title()\n\ttext.Render()\n}\n",
        ),
        (
            "format/format.go",
            "package format\n\nimport \"example.com/demo/text\"\n\nfunc Title() string {\n\treturn text.Render()\n}\n",
        ),
        ("text/text.go", "package text\n\nfunc Render() string {\n\treturn \"\"\n}\n"),
    ])
}

/// The update -> report round-trip: the seed lists the tree's
/// PACKAGES as modules (mirroring the intra-module module_edges), every
/// component is owned, every module edge endpoint is claimed, and `report`
/// therefore exits 0 with zero findings — round-trip clean
/// by construction, the way the per-package seed is.
#[test]
fn update_single_module_seeds_packages_and_reports_clean() {
    let fixture = go_seed_fixture();
    let update = fixture.run(&["update"]);
    assert_eq!(
        update.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&update)
    );

    let seed = fixture.read("architecture.spec.toml");
    assert!(
        seed.contains("name = \"example.com/demo/store\"")
            && seed.contains("name = \"example.com/demo/format\"")
            && seed.contains("name = \"example.com/demo/text\""),
        "the seed must declare one module per package, not an empty \
         workspace-shaped spec:\n{seed}"
    );
    assert!(
        seed.contains("matches = { units = [\"example.com/demo/store\"] }"),
        "each package boundary must claim exactly its own import path:\n{seed}"
    );
    assert!(
        seed.contains(
            "allowed = { depend_on = [\"example.com/demo/format\", \"example.com/demo/text\"] }"
        ),
        "the store package must be allowed to depend on both packages its \
         module edges cross:\n{seed}"
    );

    let report = fixture.run(&["report", "--format", "json"]);
    assert_eq!(
        report.status.code(),
        Some(0),
        "report on the seed must exit 0:\n{}",
        stdout(&report)
    );
    let value: Value = serde_json::from_str(&stdout(&report)).expect("report JSON");
    let findings = value["findings"]
        .as_array()
        .expect("findings array must be present");
    assert!(
        findings.is_empty(),
        "the seed must round-trip with zero findings:\n{}",
        stdout(&report)
    );
}

// ---------------------------------------------------------------------------
// roles US 03 (workplan archspec_roles): go composition — package main,
// facade honestly absent. The go driver derives `composition` from its own
// source facts — the `package_clause` of a unit's files — and keys the entry
// at the unit's model path (the directory-derived import path, the same
// identity every go consumer and the update seed use). No go fixture anywhere
// gains a `facade` entry: the non-derivability from go shapes is a recorded
// decision (ADR-017 §Go facade investigation, go-facade US 02b), and the
// guards below pin that absence as the honest truth plus the decision
// surface that states it.
// ---------------------------------------------------------------------------

/// A single-package main tree wired at the module root: `main.go` glues the
/// internal `core` package — the composition shape of a go binary.
fn go_main_fixture() -> Fixture {
    go_fixture(&[
        (
            "main.go",
            "package main\n\nimport \"example.com/demo/internal/core\"\n\nfunc main() { core.Run() }\n",
        ),
        (
            "internal/core/core.go",
            "package core\n\nimport \"example.com/demo/internal/store\"\n\nfunc Run() { store.Get() }\n",
        ),
        ("internal/store/store.go", "package store\n\nfunc Get() {}\n"),
    ])
}

/// The roles map of a model as a plain list of (model path, role) pairs.
fn roles(model: &Value) -> Vec<(String, String)> {
    model["roles"]
        .as_object()
        .expect("roles must be an object")
        .iter()
        .map(|(path, role)| (path.clone(), role.as_str().expect("role").to_string()))
        .collect()
}

/// A go unit whose `package_clause` is `main` gains `composition` at its unit
/// path; the packages it wires gain nothing beyond that — the binary linking
/// other modules states no further roles (the composition entry is the whole
/// fact).
#[test]
fn scan_scenario_go_main_package_unit_gains_composition_role() {
    let model = scan(&go_main_fixture());
    assert_eq!(
        model["roles"]["example.com/demo"].as_str(),
        Some("composition"),
        "the main-package root is the composition root:\n{}",
        model
    );
    assert!(
        model["roles"].get("example.com/demo/internal/core").is_none()
            && model["roles"].get("example.com/demo/internal/store").is_none(),
        "packages wired BY the main package carry no role:\n{}",
        model
    );
    assert_eq!(
        roles(&model),
        vec![("example.com/demo".to_string(), "composition".to_string())],
        "the main package is the only derivable role in the tree"
    );
}

/// A `go.work` tree marks each member's main packages by their own member
/// module paths: a member without any main package states no role, and a
/// member's non-main packages stay out of the map.
#[test]
fn scan_scenario_go_workspace_marks_each_members_main_packages() {
    let fixture = Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./web\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write("api/api.go", "package api\n\nfunc Render() {}\n");
    fixture.write("web/go.mod", "module example.com/web\ngo 1.21\n");
    fixture.write(
        "web/cmd/web/main.go",
        "package main\n\nimport \"example.com/api\"\n\nfunc main() { api.Render() }\n",
    );
    fixture.write("web/lib/lib.go", "package lib\n\nfunc Helper() {}\n");
    let model = scan(&fixture);
    assert_eq!(
        roles(&model),
        vec![(
            "example.com/web/cmd/web".to_string(),
            "composition".to_string()
        )],
        "each member's main package gains composition at its own module path \
         and nothing else does"
    );
}

/// A library-only tree derives nothing: the roles map is empty and the
/// serialized model carries no `roles` key at all — absence states "no role
/// stated", never "role denied".
#[test]
fn scan_scenario_go_library_only_tree_states_no_roles() {
    let fixture = gherkin_fixture();
    let model = scan(&fixture);
    assert!(
        model.get("roles").is_none(),
        "a library-only tree emits no roles key:\n{}",
        model
    );
    let raw = String::from_utf8(scan_bytes(&fixture)).expect("utf-8 JSON");
    assert!(
        !raw.contains("\"roles\""),
        "the empty roles map must not serialize a key:\n{raw}"
    );
}

/// The facade-absence guard: no go tree — single-module main, cmd-rooted
/// main, go.work workspace, canonical workspace or library-only shape —
/// carries a `facade` entry, and no scan output names the facade value at
/// all. The four trees the go-facade US 01 probe planted (alias umbrella,
/// its wrapper decoy, delegating root, defining decoy) ride the same
/// absence loop: a future derivation cannot sneak a facade entry through
/// those shapes silently. The non-derivability behind this absence is a
/// recorded decision, not an assertion — ADR-017 §Go facade investigation
/// — and the decision-surface pin below keeps this guard's surface and
/// that record in agreement.
#[test]
fn guard_go_scans_never_state_the_facade_role() {
    let workspace = Fixture::new();
    materialize_go_workspace(&workspace);
    let trees: Vec<(&str, Fixture)> = vec![
        ("single-module main", go_main_fixture()),
        (
            "cmd-rooted main",
            go_fixture(&[(
                "cmd/server/main.go",
                "package main\n\nimport \"example.com/demo/lib\"\n\nfunc main() { lib.Run() }\n",
            ), ("lib/lib.go", "package lib\n\nfunc Run() {}\n")]),
        ),
        ("go.work workspace", workspace),
        ("library-only", gherkin_fixture()),
        // The US 01 probe corpus (worklog/workplan_archspec_go_facade/
        // findings.md): the alias-shaped trees come first because they are
        // the shapes a future derivation would most plausibly key on —
        // `fx-alias` is the intended publication umbrella, and
        // `fx-alias-ordinary` is the thin wrapper it scans byte-identical
        // to, so any predicate that marks one must mark the other and this
        // loop fails on the decoy long before a facade rule could fire on
        // an ordinary package in production.
        (
            "alias umbrella (probe fx-alias)",
            go_fixture(&[
                (
                    "core/core.go",
                    "package core\n\ntype Widget struct{}\n\nfunc New() *Widget { return &Widget{} }\n\nfunc Version() string { return \"v1\" }\n",
                ),
                (
                    "umbrella/umbrella.go",
                    "package umbrella\n\nimport \"example.com/demo/core\"\n\ntype Widget = core.Widget\n\nvar New = core.New\n\nvar Version = core.Version\n",
                ),
                (
                    "app/app.go",
                    "package app\n\nimport \"example.com/demo/umbrella\"\n\nfunc Make() *umbrella.Widget { return umbrella.New() }\n",
                ),
            ]),
        ),
        (
            "wrapper decoy (probe fx-alias-ordinary)",
            go_fixture(&[
                (
                    "core/core.go",
                    "package core\n\ntype Widget struct{}\n\nfunc New() *Widget { return &Widget{} }\n\nfunc Version() string { return \"v1\" }\n",
                ),
                (
                    "umbrella/umbrella.go",
                    "package umbrella\n\nimport \"example.com/demo/core\"\n\ntype Widget = core.Widget\n\nvar Version = core.Version\n\nfunc New() *core.Widget { return core.New() }\n",
                ),
                (
                    "app/app.go",
                    "package app\n\nimport \"example.com/demo/umbrella\"\n\nfunc Make() *umbrella.Widget { return umbrella.New() }\n",
                ),
            ]),
        ),
        (
            "delegating root (probe fx-root)",
            go_fixture(&[
                (
                    "root.go",
                    "package rootdel\n\nimport \"example.com/demo/sub\"\n\ntype Thing = sub.Thing\n\nvar Get = sub.Get\n",
                ),
                (
                    "sub/sub.go",
                    "package sub\n\ntype Thing struct{}\n\nfunc Get() *Thing { return &Thing{} }\n",
                ),
            ]),
        ),
        (
            "defining root decoy (probe fx-root-noalias)",
            go_fixture(&[
                (
                    "root.go",
                    "package rootdel\n\nimport \"example.com/demo/sub\"\n\nfunc Get() *sub.Thing { return sub.Get() }\n",
                ),
                (
                    "sub/sub.go",
                    "package sub\n\ntype Thing struct{}\n\nfunc Get() *Thing { return &Thing{} }\n",
                ),
            ]),
        ),
    ];
    for (label, fixture) in trees {
        let raw = String::from_utf8(scan_bytes(&fixture)).expect("utf-8 JSON");
        assert!(
            !raw.contains("facade"),
            "the go scan for `{label}` states the facade role:\n{raw}"
        );
    }
}

/// The decision-surface pin (go-facade US 03, branch b): the single inert
/// note a go `verify` run prints and the ADR-017 amendment must tell the
/// same story — the note names ADR-017 and enumerates the investigated
/// patterns under the labels the amendment's §Go facade investigation
/// assigns them. A wording-only drift on EITHER side reddens this pin, not
/// a production tree; the capability row and the absence loop above stay
/// untouched here by design (the guard is kept and extended, never
/// contradicted).
#[test]
fn guard_go_facade_note_and_adr_amendment_agree() {
    let fixture = go_fixture(&[
        (
            "store/store.go",
            "package store\n\nimport \"example.com/demo/format\"\n\nfunc Save() {}\n",
        ),
        ("format/format.go", "package format\n\nfunc Title() {}\n"),
    ]);
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n\n[module.allowed]\ndepend_on = [\"format\"]\n\n[[module]]\nname = \"format\"\nmatches = { units = [\"example.com/demo/format\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the note must not change the verdict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    let notes: Vec<&str> = out
        .lines()
        .filter(|line| line.starts_with("note:"))
        .collect();
    assert_eq!(notes.len(), 1, "exactly one inert note per go run:\n{out}");
    let note = notes[0];
    assert!(
        note.starts_with("note: facade dependency rule inert for go:")
            && note.contains("ADR-017"),
        "the note must state the facade decision and cite the record:\n{note}"
    );
    let adr = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/adr/017-roles-in-model-closed-vocabulary.md"
    ))
    .expect("ADR-017 ships in the payload");
    let (_, section) = adr.split_once("## Go facade investigation").unwrap_or_else(|| {
        panic!("the ADR must carry the §Go facade investigation amendment")
    });
    for pattern in ["alias umbrella", "public-package", "root delegation"] {
        assert!(
            note.contains(pattern),
            "the note must enumerate the `{pattern}` pattern it cites:\n{note}"
        );
        assert!(
            section.contains(pattern),
            "the ADR amendment must name the `{pattern}` pattern the note enumerates"
        );
    }
    assert!(
        adr.contains("§Go facade investigation"),
        "the ADR body must point at its own amendment section"
    );
}
