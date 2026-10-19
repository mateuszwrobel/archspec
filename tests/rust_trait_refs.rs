//! Rust trait-reference extraction (adjudication
//! `rust-arch-test-kit/worklog/workplan_archspec_dyn_edge_adjudication.md` §7
//! item 2, accepted arms (a) impl-header trait positions, (b) trait-object
//! `dyn` bound paths, (c) TypePaths nested in generic arguments). Every new
//! reference is an ordinary static-reference edge: it rides the existing
//! `edges` array (unit tier, resolution law mirroring the cross-unit `use`
//! branch) and `module_edges`/`file_edges` (module tier, reserved-segment
//! mirror). The must-not-fire law (A2-A4): non-unit trait paths (external
//! and stdlib) contribute no unit edge and no external row, test-gated
//! trees and dev-dependency folds contribute nothing, self-impls and
//! self-unit references contribute nothing, blanket/generic impls are
//! covered by the same resolution law with no special case. The corpus
//! fixture shape mirrors impactspec's p2-4 trio (alpha trait / beta impl /
//! app dispatcher); the probe shapes mirror adjudication §3 v1-v6.

mod common;

use common::{Fixture, stderr, stdout};
use serde_json::Value;

fn scan_model(tree: &Fixture) -> Value {
    let out = tree.run(&["scan", ".", "--output", "-"]);
    assert_eq!(out.status.code(), Some(0), "scan failed: {}", stderr(&out));
    serde_json::from_str(&stdout(&out)).expect("scan body is JSON")
}

/// Sorted `(from, to)` pairs of the model's hard unit edges.
fn edge_pairs(model: &Value) -> Vec<(String, String)> {
    model
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|edge| {
            (
                edge["from"].as_str().unwrap_or_default().to_string(),
                edge["to"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

/// The p2-4 trio: trait in `alpha`, impl in `beta`, dispatcher in `app`.
/// Manifest deps are omitted wherever a probe isolates an extraction arm —
/// the same convention the adjudication scratch probes v1-v6 use.
fn trio(fixture: &Fixture, alpha: &str, beta: &str, app: &str) {
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/alpha\", \"crates/beta\", \"crates/app\"]\n",
    );
    for (unit, body) in [("alpha", alpha), ("beta", beta), ("app", app)] {
        fixture.write(
            &format!("crates/{unit}/Cargo.toml"),
            &format!("[package]\nname = \"{unit}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        );
        fixture.write(&format!("crates/{unit}/src/lib.rs"), body);
    }
}

const ALPHA: &str = "pub trait Reg {\n    fn make(&self);\n}\npub struct AlphaReg;\nimpl Reg for AlphaReg {\n    fn make(&self) {}\n}\n";

// ---------------------------------------------------------------- arms

/// Arm (a): the trait path of an `impl alpha::Reg for BetaReg` header is a
/// static reference from the impl's unit to the trait's unit. No `use`, no
/// manifest dependency — the impl header alone must carry the edge
/// (adjudication §3 v2 gap (a)).
#[test]
fn impl_header_trait_path_emits_unit_edge() {
    let fixture = Fixture::new();
    trio(
        &fixture,
        ALPHA,
        "pub struct BetaReg;\nimpl alpha::Reg for BetaReg {\n    fn make(&self) {}\n}\n",
        "pub fn noop() {}\n",
    );
    let pairs = edge_pairs(&scan_model(&fixture));
    let want: Vec<(String, String)> = vec![("beta".to_string(), "alpha".to_string())];
    assert_eq!(want, pairs, "impl-header trait path must emit exactly the beta->alpha unit edge");
}

/// Arm (a)/(b) on the p2-4 corpus shape verbatim (adjudication §3 v1): the
/// dispatching unit `app` references both crates in expressions, `beta`
/// imports the trait. The scan of these sources carries the full three-edge
/// set — this is the graph the impactspec p2-4a leg flips on (closure from
/// app reaches beta, so a beta edit selects t_app).
#[test]
fn p2_4_corpus_shape_scans_to_the_closure_complete_edge_set() {
    let fixture = Fixture::new();
    trio(
        &fixture,
        ALPHA,
        "use alpha::Reg;\npub struct BetaReg;\nimpl Reg for BetaReg {\n    fn make(&self) {}\n}\n",
        "pub fn registry() -> Vec<Box<dyn alpha::Reg>> {\n    let mut v: Vec<Box<dyn alpha::Reg>> = Vec::new();\n    v.push(Box::new(beta::BetaReg));\n    v.push(Box::new(alpha::AlphaReg));\n    v\n}\n",
    );
    let pairs = edge_pairs(&scan_model(&fixture));
    let want: Vec<(String, String)> = [
        ("app", "alpha"),
        ("app", "beta"),
        ("beta", "alpha"),
    ]
    .into_iter()
    .map(|(f, t)| (f.to_string(), t.to_string()))
    .collect();
    assert_eq!(want, pairs, "p2-4 corpus shape must scan to the full edge set");
}

/// Arm (b): a `dyn`-bound trait path inside type-position generic arguments
/// (`Vec<Box<dyn alpha::Reg>>`) is a static reference even when the unit
/// names the trait nowhere else — no expression path, no manifest
/// dependency (adjudication §3 v3 gap (b)).
#[test]
fn dyn_bound_trait_path_at_type_position_emits_unit_edge() {
    let fixture = Fixture::new();
    trio(
        &fixture,
        ALPHA,
        "pub fn own() {}\n",
        "pub struct Holder {\n    pub reg: Box<dyn alpha::Reg>,\n}\npub fn slice(rs: &[Box<dyn alpha::Reg>]) -> usize {\n    rs.len()\n}\n",
    );
    let pairs = edge_pairs(&scan_model(&fixture));
    let want: Vec<(String, String)> = vec![("app".to_string(), "alpha".to_string())];
    assert_eq!(
        want, pairs,
        "dyn-bound trait path must emit exactly the app->alpha unit edge (app names beta nowhere)"
    );
}

/// Arm (c): TypePaths nested in generic arguments are static references
/// too — `Vec<Box<beta::BetaReg>>` and `Vec<Option<beta::BetaReg>>` with no
/// expression path and no manifest dependency (adjudication §3 v4 gap (c)).
#[test]
fn nested_generic_argument_type_paths_emit_unit_edges() {
    let fixture = Fixture::new();
    trio(
        &fixture,
        ALPHA,
        "pub struct BetaReg;\n",
        "pub fn make() -> Vec<Box<beta::BetaReg>> {\n    Vec::new()\n}\npub fn opts() -> Vec<Option<beta::BetaReg>> {\n    Vec::new()\n}\n",
    );
    let pairs = edge_pairs(&scan_model(&fixture));
    let want: Vec<(String, String)> = vec![("app".to_string(), "beta".to_string())];
    assert_eq!(
        want, pairs,
        "type paths nested in generic arguments must emit exactly the app->beta unit edge"
    );
}

/// Module-tier truth (§3 closing paragraph): a same-unit
/// `impl crate::foo::Reg for Bar` inside module `bar` is a real bar->foo
/// module dependency — it lands in `module_edges` and contributes the
/// path-kind file row. Self-unit references still contribute no unit edge.
#[test]
fn impl_header_reserved_path_emits_module_edge_and_file_row() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod foo;\npub mod bar;\n");
    fixture.write("src/foo.rs", "pub trait Reg {}\n");
    fixture.write(
        "src/bar.rs",
        "pub struct Bar;\nimpl crate::foo::Reg for Bar {}\n",
    );
    let model = scan_model(&fixture);
    let found = model["module_edges"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .any(|edge| edge["from"] == "app::bar" && edge["to"] == "app::foo");
    assert!(
        found,
        "impl-header reference must emit the bar->foo module edge: {:?}",
        model["module_edges"]
    );
    let row = model["file_edges"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .any(|row| row["from"] == "src/bar.rs" && row["to"] == "src/foo.rs" && row["kind"] == "path");
    assert!(
        row,
        "the impl-header reference must contribute a path-kind file row: {:?}",
        model["file_edges"]
    );
    assert!(
        edge_pairs(&model).is_empty(),
        "a self-unit reference contributes no unit edge: {:?}",
        edge_pairs(&model)
    );
}

// ------------------------------------------------------- must-not-fire

/// A2 external traits: `impl serde::Serialize for BetaReg` is a reference
/// to a non-unit path — it contributes no unit edge and no external row
/// (trait positions never record externals; v5).
#[test]
fn external_trait_impl_header_fires_no_edge_and_no_external_row() {
    let fixture = Fixture::new();
    trio(
        &fixture,
        "pub fn nothing() {}\n",
        "pub struct BetaReg;\nimpl serde::Serialize for BetaReg {\n    fn serialize(&self) {}\n}\n",
        "pub fn nothing() {}\n",
    );
    let model = scan_model(&fixture);
    assert_eq!(Vec::<(String, String)>::new(), edge_pairs(&model));
    let external: Vec<String> = model["external"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(Vec::<String>::new(), external, "no external row from trait positions");
}

/// A2 stdlib: dyn bounds and nested generic arguments naming std/core are
/// dropped by the same stdlib law the qualified-path walk applies.
#[test]
fn stdlib_trait_and_type_paths_fire_nothing() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "use std::fmt::Debug;\npub fn make() -> Box<dyn std::fmt::Debug> {\n    unimplemented!()\n}\npub fn cols() -> Vec<Option<std::ffi::OsString>> {\n    Vec::new()\n}\npub fn boxed() -> Vec<Box<dyn Fn()>> {\n    Vec::new()\n}\n",
    );
    let model = scan_model(&fixture);
    assert_eq!(Vec::<(String, String)>::new(), edge_pairs(&model));
    let external: Vec<String> = model["external"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(Vec::<String>::new(), external);
}

/// A4 blanket and generic impls: the resolution law covers them with no
/// special case — a blanket impl of a crate-local item that resolves to no
/// module fires nothing, and a generic self type falls under the
/// single-segment law (v5 blanket form).
#[test]
fn blanket_and_generic_impls_fire_nothing() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub trait NoTrait {}\nimpl<T> crate::NoTrait for T {}\npub struct Wrapper<T>(T);\nimpl<T: Clone> Clone for Wrapper<T> {\n    fn clone(&self) -> Self {\n        Wrapper(self.0.clone())\n    }\n}\n",
    );
    let model = scan_model(&fixture);
    assert_eq!(Vec::<(String, String)>::new(), edge_pairs(&model));
    assert_eq!(
        Vec::<String>::new(),
        model["external"].as_array().cloned().unwrap_or_default(),
        "blanket impls contribute no external rows"
    );
}

/// A3 test-gated trees and the dev-dependency fold: a `#[cfg(test)] mod
/// tests` block holding `use` plus an impl header names the other unit, and
/// the other unit sits in `[dev-dependencies]` — both gates hold, so the
/// scan fires nothing (v6).
#[test]
fn test_gated_impls_and_dev_dependency_folds_fire_nothing() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/app\", \"crates/beta\"]\n",
    );
    fixture.write(
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/app/src/lib.rs", "pub trait Reg {}\n");
    fixture.write(
        "crates/beta/Cargo.toml",
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dev-dependencies]\napp = { path = \"../app\" }\n",
    );
    fixture.write(
        "crates/beta/src/lib.rs",
        "pub struct BetaT;\n#[cfg(test)]\nmod tests {\n    use app::Reg;\n    impl Reg for crate::BetaT {}\n}\n",
    );
    let model = scan_model(&fixture);
    assert_eq!(Vec::<(String, String)>::new(), edge_pairs(&model));
}

/// Self impls and self-unit references: `impl Foo for Bar` (single-segment
/// trait path) and `impl beta::Reg for BetaReg` (qualified with the OWN
/// crate name) stay internal — no unit edge, per the cross-unit law.
#[test]
fn self_impls_and_self_unit_trait_paths_fire_nothing() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub trait Foo {}\npub struct Bar;\nimpl Foo for Bar {}\npub trait Reg {}\npub struct BetaReg;\nimpl beta::Reg for BetaReg {}\n",
    );
    let model = scan_model(&fixture);
    assert_eq!(Vec::<(String, String)>::new(), edge_pairs(&model));
}
