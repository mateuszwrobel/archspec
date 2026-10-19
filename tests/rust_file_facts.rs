//! Rust file-grained FACTS substrate (design record D11 precondition, B5
//! law): the rust driver serializes file→file adjacency rows in the model,
//! the capability row reads granular because the weld guard READS those
//! facts from the binary (a granular row whose facts are absent reddens
//! here, never silently), and what rust does not carry is stated by
//! absence — macro-built and unresolved file adjacencies serialize no row,
//! never a fabricated one.

mod common;

use common::{Fixture, stderr, stdout};
use serde_json::Value;

/// The B5 weld: the granular row and the serialized facts must agree.
/// `Err` states the disagreement — a row claiming granular while the
/// model states no file-grained rows (the capability cell lies about the
/// substrate), or rows present while the row denies them.
fn facts_agree(row_granular: bool, model: &Value) -> Result<(), String> {
    let edges = model
        .get("file_edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    match (row_granular, edges.is_empty()) {
        (true, true) => Err(
            "capability row claims granular but the model serializes no file_edges rows".to_string(),
        ),
        (false, false) => Err(
            "capability row denies granular but the model serializes file_edges rows".to_string(),
        ),
        _ => Ok(()),
    }
}

fn scan_model(tree: &Fixture) -> Value {
    let out = tree.run(&["scan", ".", "--output", "-"]);
    assert_eq!(out.status.code(), Some(0), "scan failed: {}", stderr(&out));
    serde_json::from_str(&stdout(&out)).expect("scan body is JSON")
}

fn capability_granular(tree: &Fixture, language: &str, fact: &str) -> bool {
    tree.run(&["capability", "granular", language, fact])
        .status
        .code()
        == Some(0)
}

/// The weld guard green end to end: the binary's row for rust and the
/// binary's serialized facts agree on a tree whose adjacency cannot
/// honestly be empty (two declared modules, one importing the other).
#[test]
fn rust_facts_weld_guard_agrees_row_and_facts_from_the_binary() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write("src/lib.rs", "pub mod core;\npub mod ui;\n");
    tree.write("src/core.rs", "pub fn core() {}\n");
    tree.write("src/ui.rs", "use crate::core;\npub fn ui() {}\n");

    let granular = capability_granular(&tree, "rust", "file-grained-dependency-facts");
    assert!(granular, "the rust row must read granular");
    let model = scan_model(&tree);
    facts_agree(granular, &model).expect("row and facts agree");
}

/// The B5 red: strip the rows from the model and the guard fails while the
/// row claims granular — a capability cell lying about the substrate is a
/// red suite, never a green silence.
#[test]
fn facts_weld_guard_reds_when_facts_absent_while_row_granular() {
    let mut model = serde_json::json!({
        "schema_version": 1,
        "language": "rust",
        "units": [],
        "edges": [],
        "module_edges": [{"unit": "app", "from": "app::a", "to": "app::b", "symbols": []}]
    });
    assert_eq!(
        facts_agree(true, &model),
        Err(
            "capability row claims granular but the model serializes no file_edges rows".to_string()
        )
    );
    // And the rows-without-the-row direction is red too.
    model["file_edges"] = serde_json::json!([
        {"unit": "app", "from": "src/a.rs", "to": "src/b.rs", "kind": "mod"}
    ]);
    assert_eq!(
        facts_agree(false, &model),
        Err(
            "capability row denies granular but the model serializes file_edges rows".to_string()
        )
    );
}

/// The row shape the weld reads: `(from, to, kind)` rows spelled
/// repo-relative (the diagram's file identity), kinds from rust's truthful
/// lexical vocabulary. Module-granular `module_edges` keep their dotted
/// identity — the facts gain additively, never re-keying an existing row.
#[test]
fn rust_scan_serializes_mod_use_and_path_rows_over_repo_relative_files() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write("src/lib.rs", "pub mod core;\npub mod ui;\n");
    tree.write("src/core/mod.rs", "pub fn core() {}\n");
    tree.write("src/ui.rs", "use crate::core;\npub fn ui() { crate::core::core(); }\n");
    let model = scan_model(&tree);
    let rows: Vec<(String, String, String)> = model["file_edges"]
        .as_array()
        .expect("file_edges present on a multi-file crate")
        .iter()
        .map(|row| {
            (
                row["from"].as_str().unwrap_or_default().to_string(),
                row["to"].as_str().unwrap_or_default().to_string(),
                row["kind"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    for expected in [
        ("src/lib.rs", "src/core/mod.rs", "mod"),
        ("src/lib.rs", "src/ui.rs", "mod"),
        ("src/ui.rs", "src/core/mod.rs", "use"),
        ("src/ui.rs", "src/core/mod.rs", "path"),
    ] {
        assert!(
            rows.contains(&(expected.0.to_string(), expected.1.to_string(), expected.2.to_string())),
            "row {expected:?} missing from {rows:?}"
        );
    }
    // Module-granular identity untouched: the dotted rows the fan-out and
    // boundary rules read stay exactly as shipped.
    let module_edges = model["module_edges"].as_array().expect("module_edges");
    assert!(
        module_edges
            .iter()
            .all(|edge| !edge["from"].as_str().unwrap_or_default().contains('/')),
        "module_edges stay dotted module identities: {module_edges:?}"
    );
}

/// Cross-unit references stay unit-granular: a member importing another
/// member's item is a unit edge, never a file row pointing into the other
/// member's source tree. The rust driver states file identity only for
/// files of the unit it scanned.
#[test]
fn cross_unit_use_stays_a_unit_edge_never_a_file_row() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/app\", \"crates/core\"]\nresolver = \"2\"\n",
    );
    tree.write(
        "crates/core/Cargo.toml",
        "[package]\nname = \"core-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write("crates/core/src/lib.rs", "pub mod run;\n");
    tree.write("crates/core/src/run.rs", "pub fn run() {}\n");
    tree.write(
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore-lib = { path = \"../core\" }\n",
    );
    tree.write("crates/app/src/lib.rs", "pub mod wire;\n");
    tree.write("crates/app/src/wire.rs", "use core_lib::run;\npub fn wire() { run::run(); }\n");
    let model = scan_model(&tree);
    let rows = model["file_edges"].as_array().expect("file_edges");
    for row in rows {
        let (from, to) = (
            row["from"].as_str().unwrap_or_default(),
            row["to"].as_str().unwrap_or_default(),
        );
        let from_unit = from.split("/src/").next().unwrap_or_default();
        let to_unit = to.split("/src/").next().unwrap_or_default();
        assert_eq!(
            from_unit, to_unit,
            "a file row must stay inside one unit's tree: {row:?}"
        );
    }
    // Repo-relative spelling: member rows name the member directory.
    assert!(
        rows.iter().any(|row| {
            row["from"] == "crates/core/src/lib.rs" && row["to"] == "crates/core/src/run.rs"
        }),
        "member rows spell repo-relative files: {rows:?}"
    );
    // The cross-unit reference still states the unit edge.
    assert!(
        model["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["from"] == "app" && edge["to"] == "core-lib"),
        "the cross-unit use stays a unit edge: {}",
        model["edges"]
    );
}

/// An inline `mod x { ... }` has no file of its own: the facts sourced from
/// its items are facts of the declaring file, and no row names a file that
/// does not exist. Macro-built and unresolved adjacencies serialize no row
/// either — absence states "no such fact", never a fabricated edge.
#[test]
fn inline_macro_and_unresolved_constructs_fabricate_no_rows() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "src/lib.rs",
        "pub mod core;\npub mod ghost;\nmod helper {\n    use crate::core;\n    pub fn h() { core::core(); }\n}\n",
    );
    tree.write("src/core.rs", "pub fn core() {}\n");
    // `mod ghost;` declares a file-backed module that resolves to no file:
    // the unresolved-declaration warning names it; no file row does.
    let model = scan_model(&tree);
    let rows = model["file_edges"].as_array().expect("file_edges");
    let row_texts: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "{} -> {} [{}]",
                row["from"].as_str().unwrap_or_default(),
                row["to"].as_str().unwrap_or_default(),
                row["kind"].as_str().unwrap_or_default()
            )
        })
        .collect();
    assert!(
        row_texts.iter().any(|row| row
            == "src/lib.rs -> src/core.rs [use]"),
        "the inline module's use is a fact of the declaring file: {row_texts:?}"
    );
    for row in rows {
        for key in ["from", "to"] {
            let path = row[key].as_str().unwrap_or_default();
            assert!(
                !path.contains("ghost") && !path.contains("helper"),
                "no row may name an unresolved or inline pseudo-file: {row:?}"
            );
        }
    }

    // `include!` bodies are macro text the lexical scan does not see: no
    // row is invented for them either.
    let macro_tree = Fixture::new();
    macro_tree.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    macro_tree.write("src/lib.rs", "include!(\"generated.rs\");\npub mod core;\n");
    macro_tree.write("src/generated.rs", "pub fn generated() {}\n");
    macro_tree.write("src/core.rs", "pub fn core() {}\n");
    let model = scan_model(&macro_tree);
    let rows = model["file_edges"].as_array().expect("file_edges");
    assert!(
        rows.iter().all(|row| {
            !row["from"].as_str().unwrap_or_default().contains("generated")
                && !row["to"].as_str().unwrap_or_default().contains("generated")
        }),
        "an include! body serializes no fabricated row: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|row| row["from"] == "src/lib.rs" && row["to"] == "src/core.rs"),
        "the mod declaration beside the macro still states its fact: {rows:?}"
    );
}

/// go and csharp stay not-emitted: their model bytes carry no `file_edges`
/// key at all — the additive-field law keeps their artifacts byte-untouched.
#[test]
fn absent_file_edges_key_keeps_other_drivers_bytes_untouched() {
    let json = r#"{"schema_version":1,"language":"go","units":[],"edges":[]}"#;
    let model: Value = serde_json::from_str(json).expect("driver model loads");
    assert!(
        model.get("file_edges").is_none(),
        "an empty vector serializes no key: absence states no facts"
    );
    assert_eq!(
        facts_agree(false, &model),
        Ok(()),
        "not-emitted and no rows agree"
    );
}

/// The use-path resolution law (flip lane 2026-10-16): a bare first segment of
/// a `use` path is a CRATE name (Rust 2018+), never an in-scope module of this
/// crate. A module whose name collides with a dependency name (module `tauri`
/// while depending on crate `tauri`) must not gain a phantom back-edge from
/// `use tauri::{...}` inside it — that mis-attribution was the last file-tier
/// cycle of the fleet (voice-lib) and would flip a clean crate red.
#[test]
fn use_path_bare_first_segment_resolves_to_the_crate_not_an_own_module() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ntauri = \"1\"\n",
    );
    tree.write("src/lib.rs", "pub mod tauri;\n");
    tree.write("src/tauri/mod.rs", "mod state;\npub mod commands;\npub use state::VoiceState;\n");
    tree.write("src/tauri/state.rs", "pub struct VoiceState;\n");
    tree.write(
        "src/tauri/commands.rs",
        "use crate::tauri::state::VoiceState;\nuse tauri::{command, State};\n#[command]\npub fn f(s: &VoiceState) {}\n",
    );

    let model = scan_model(&tree);
    let rows = model["file_edges"].as_array().expect("file_edges");
    assert!(
        rows.iter().all(|row| !(row["from"] == "src/tauri/commands.rs"
            && row["to"] == "src/tauri/mod.rs")),
        "a dependency import inside module `tauri` states no file row to that module's own file: {rows:?}"
    );
    assert!(
        rows.iter().any(|row| {
            row["from"] == "src/tauri/commands.rs" && row["to"] == "src/tauri/state.rs"
        }),
        "the crate:: import still states its fact: {rows:?}"
    );
    let external = model["external"].as_array().expect("external");
    assert!(
        external.iter().any(|n| n.as_str() == Some("tauri")),
        "the dependency names an external crate: {external:?}"
    );
}

/// The containment-hop law: a file row whose target module CONTAINS the
/// using module states a fact the module tier already edges (a descendant
/// module reaching an ancestor's items), so the file tier must not row it —
/// whatever spelling the path uses (`super::`, `crate::`-absolute, or a
/// bare name). A clean containment tree may never ring at file grain: the
/// ring would re-report a module-tier fact and reddened the fleet's mod
/// trees once the rust posture row read `both` (ADR-023). Sibling hops
/// through a parent keep their row: the target is not an ancestor.
#[test]
fn ancestor_target_hops_contribute_no_file_row_whatever_their_spelling() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "src/lib.rs",
        "pub mod outer;\npub const ROOT_ITEM: u8 = 1;\n",
    );
    tree.write(
        "src/outer/mod.rs",
        "pub mod inner;\npub mod sibling;\npub const PARENT_ITEM: u8 = 2;\n",
    );
    tree.write(
        "src/outer/inner.rs",
        "use crate::ROOT_ITEM;\nuse super::PARENT_ITEM;\npub fn f() -> u8 { crate::ROOT_ITEM + PARENT_ITEM }\n",
    );
    tree.write("src/outer/sibling.rs", "pub const SIBLING_ITEM: u8 = 3;\n");
    let model = scan_model(&tree);
    let rows: Vec<(String, String)> = model["file_edges"]
        .as_array()
        .expect("file_edges present on a multi-file crate")
        .iter()
        .map(|row| {
            (
                row["from"].as_str().unwrap_or_default().to_string(),
                row["to"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    for banned in [
        ("src/outer/inner.rs", "src/outer/mod.rs"),
        ("src/outer/inner.rs", "src/lib.rs"),
        ("src/outer/mod.rs", "src/lib.rs"),
    ] {
        assert!(
            !rows.contains(&(banned.0.to_string(), banned.1.to_string())),
            "ancestor-target row {banned:?} must not be rowed at file grain: {rows:?}"
        );
    }
    // The module tier still edges the descendant->ancestor fact: the pair
    // stays visible there, so no cycle escapes the file tier into silence.
    let module_edges: Vec<(String, String)> = model["module_edges"]
        .as_array()
        .expect("module_edges")
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().unwrap_or_default().to_string(),
                edge["to"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    assert!(
        module_edges
            .iter()
            .any(|(from, to)| from.contains("outer::inner") && to == "app"),
        "the module tier still edges the ancestor hop: {module_edges:?}"
    );
}

/// The same law's keep-side: a `super::`-spelled hop whose target is a
/// SIBLING module (not an ancestor) is a real dependency and keeps its row.
#[test]
fn sibling_hop_through_a_parent_keeps_its_file_row() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write("src/lib.rs", "pub mod outer;\n");
    tree.write("src/outer/mod.rs", "pub mod inner;\npub mod sibling;\n");
    tree.write(
        "src/outer/inner.rs",
        "use super::sibling::SIBLING_ITEM;\npub fn f() -> u8 { SIBLING_ITEM }\n",
    );
    tree.write("src/outer/sibling.rs", "pub const SIBLING_ITEM: u8 = 3;\n");
    let model = scan_model(&tree);
    let rows: Vec<(String, String, String)> = model["file_edges"]
        .as_array()
        .expect("file_edges present on a multi-file crate")
        .iter()
        .map(|row| {
            (
                row["from"].as_str().unwrap_or_default().to_string(),
                row["to"].as_str().unwrap_or_default().to_string(),
                row["kind"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    assert!(
        rows.contains(&(
            "src/outer/inner.rs".to_string(),
            "src/outer/sibling.rs".to_string(),
            "use".to_string()
        )),
        "sibling hop keeps its row: {rows:?}"
    );
}
