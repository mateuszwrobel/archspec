mod common;

use common::{stderr, stdout};
use serde_json::Value;

/// Single crate named `app` with top-level modules `auth` and `billing`.
/// `billing.rs` imports `crate::auth::Token` (acceptance #13, #14, #18, #19).
fn app_module_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");
    fixture
}

fn model_of(fixture: &common::Fixture) -> Value {
    let output = fixture.run(&["scan"]);
    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    serde_json::from_str(&stdout(&output)).expect("stdout must be JSON")
}

fn module_edge_strings(model: &Value) -> Vec<String> {
    model["module_edges"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|edge| {
                    format!(
                        "{}:{}->{}:{}",
                        edge["unit"].as_str().unwrap_or_default(),
                        edge["from"].as_str().unwrap_or_default(),
                        edge["to"].as_str().unwrap_or_default(),
                        edge["symbols"]
                            .as_array()
                            .map(|s| s
                                .iter()
                                .map(|v| v.as_str().unwrap_or_default().to_string())
                                .collect::<Vec<_>>()
                                .join(","))
                            .unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn scan_scenario_13_lists_top_level_modules_as_soft_structure() {
    let fixture = app_module_fixture();
    let model = model_of(&fixture);

    let modules = model["soft_structure"]["app"]
        .as_array()
        .expect("app soft_structure must be an array");
    let paths: Vec<&str> = modules
        .iter()
        .map(|m| m.as_str().expect("module path"))
        .collect();
    assert_eq!(
        paths,
        ["app::auth", "app::billing"],
        "auth and billing listed as module nodes"
    );
}

#[test]
fn scan_scenario_14_records_module_edge_billing_to_auth() {
    let fixture = app_module_fixture();
    let model = model_of(&fixture);

    let edges = model["module_edges"].as_array().expect("module_edges");
    assert_eq!(edges.len(), 1, "exactly one module-level edge");
    assert_eq!(edges[0]["unit"].as_str(), Some("app"));
    assert_eq!(edges[0]["from"].as_str(), Some("app::billing"));
    assert_eq!(edges[0]["to"].as_str(), Some("app::auth"));
    assert_eq!(
        model["edges"].as_array().expect("edges").len(),
        0,
        "no hard unit edges in single crate"
    );
}

#[test]
fn scan_scenario_15_nests_nested_module_in_soft_structure() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\n");
    fixture.write("src/auth.rs", "mod storage;\n");
    fixture.write("src/auth/storage.rs", "pub fn load() {}\n");
    let model = model_of(&fixture);

    let modules = model["soft_structure"]["app"]
        .as_array()
        .expect("app soft_structure");
    let paths: Vec<&str> = modules
        .iter()
        .map(|m| m.as_str().expect("module path"))
        .collect();
    assert_eq!(
        paths,
        ["app::auth", "app::auth::storage"],
        "auth::storage nested under auth"
    );
}

#[test]
fn scan_scenario_16_records_module_edge_to_sibling_via_super() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod a;\nmod b;\n");
    fixture.write("src/a.rs", "use super::b;\n");
    fixture.write("src/b.rs", "pub fn b() {}\n");
    let model = model_of(&fixture);

    let edges = model["module_edges"].as_array().expect("module_edges");
    let strings = module_edge_strings(&model);
    assert!(
        strings.contains(&"app:app::a->app::b:".to_string()),
        "super sibling edge missing: {strings:?}"
    );
    assert_eq!(edges.len(), 1, "only the super sibling edge");
}

#[test]
fn scan_scenario_17_lists_external_crates_from_use() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "use serde;\nuse serde_json::Value;\n");
    let model = model_of(&fixture);

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert_eq!(
        names,
        ["serde", "serde_json"],
        "external lists imported crates, sorted"
    );
}

#[test]
fn scan_scenario_18_records_used_symbol_on_module_edge() {
    let fixture = app_module_fixture();
    let model = model_of(&fixture);

    let edges = model["module_edges"].as_array().expect("module_edges");
    assert_eq!(edges.len(), 1, "one module edge");
    let symbols: Vec<&str> = edges[0]["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .map(|s| s.as_str().expect("symbol"))
        .collect();
    assert_eq!(
        symbols,
        ["Token"],
        "symbol Token recorded on billing -> auth"
    );
}

#[test]
fn scan_scenario_19_module_output_is_byte_identical_across_runs() {
    let fixture = app_module_fixture();
    let first = fixture.run(&["scan"]);
    let second = fixture.run(&["scan"]);

    assert_eq!(first.status.code(), Some(0), "first run exit 0");
    assert_eq!(second.status.code(), Some(0), "second run exit 0");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "module output must be byte-identical across runs"
    );
    let model: Value = serde_json::from_str(&stdout(&first)).expect("JSON");
    assert!(model.get("module_edges").is_some(), "module_edges present");
    assert!(model.get("external").is_some(), "external present");
}

// acceptance #22: a crate with BOTH lib.rs and main.rs captures module structure
// from both root trees, not just lib. Each target is its own unit (B25): the lib
// tree lives under `app`, the main tree under `app-bin`.
#[test]
fn scan_scenario_22_captures_both_lib_and_main_trees() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod libmod;\n");
    fixture.write("src/libmod.rs", "pub fn lib_fn() {}\n");
    fixture.write("src/main.rs", "mod mainmod;\n");
    fixture.write("src/mainmod.rs", "pub fn main_fn() {}\n");
    let model = model_of(&fixture);

    let lib_modules = model["soft_structure"]["app"]
        .as_array()
        .expect("app soft_structure must be an array");
    let lib_paths: Vec<&str> = lib_modules
        .iter()
        .map(|m| m.as_str().expect("module path"))
        .collect();
    assert!(
        lib_paths.contains(&"app::libmod"),
        "lib tree module must be captured: {lib_paths:?}"
    );

    let bin_modules = model["soft_structure"]["app-bin"]
        .as_array()
        .expect("app-bin soft_structure must be an array");
    let bin_paths: Vec<&str> = bin_modules
        .iter()
        .map(|m| m.as_str().expect("module path"))
        .collect();
    assert!(
        bin_paths.contains(&"app-bin::mainmod"),
        "main tree module must be captured under the bin unit: {bin_paths:?}"
    );
}

// regression guard for #22: a crate with only lib.rs still captures only the lib
// tree (behavior unchanged).
#[test]
fn scan_scenario_22_lib_only_captures_lib_tree_unchanged() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod libmod;\n");
    fixture.write("src/libmod.rs", "pub fn lib_fn() {}\n");
    let model = model_of(&fixture);

    let modules = model["soft_structure"]["app"]
        .as_array()
        .expect("app soft_structure must be an array");
    let paths: Vec<&str> = modules
        .iter()
        .map(|m| m.as_str().expect("module path"))
        .collect();
    assert_eq!(paths, ["app::libmod"], "only the lib tree is captured");
}

#[test]
fn scan_scenario_20_lists_third_party_manifest_dependencies() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\ntokio = { version = \"1\", features = [\"full\"] }\n",
    );
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    let model = model_of(&fixture);

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert_eq!(
        names,
        ["serde", "tokio"],
        "third-party manifest deps listed as external"
    );
}

// acceptance #23: a module calling a sibling via a qualified path (no `use`)
// produces a module edge. `main.rs` calls `audio::list_devices()`,
// `asr::transcribe()`, `tmux::send_keys()` with no `use crate::...`.
#[test]
fn scan_scenario_23_qualified_path_calls_produce_module_edges() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-cli\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/main.rs",
        "fn main() {\n    audio::list_devices();\n    asr::transcribe();\n    tmux::send_keys();\n}\nmod audio;\nmod asr;\nmod tmux;\n",
    );
    fixture.write("src/audio.rs", "pub fn list_devices() {}\n");
    fixture.write("src/asr.rs", "pub fn transcribe() {}\n");
    fixture.write("src/tmux.rs", "pub fn send_keys() {}\n");
    let model = model_of(&fixture);

    let strings = module_edge_strings(&model);
    assert!(
        strings.contains(&"voice-cli:voice-cli::main->voice-cli::audio:".to_string()),
        "qualified edge main->audio missing: {strings:?}"
    );
    assert!(
        strings.contains(&"voice-cli:voice-cli::main->voice-cli::asr:".to_string()),
        "qualified edge main->asr missing: {strings:?}"
    );
    assert!(
        strings.contains(&"voice-cli:voice-cli::main->voice-cli::tmux:".to_string()),
        "qualified edge main->tmux missing: {strings:?}"
    );
}

// acceptance #33: BOTH a qualified-path call and a `use` import produce edges.
#[test]
fn scan_scenario_33_both_qualified_and_use_edges() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-cli\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/main.rs",
        "fn main() { audio::list_devices(); }\nmod audio;\nmod tmux;\nuse crate::tmux::send_keys;\n",
    );
    fixture.write("src/audio.rs", "pub fn list_devices() {}\n");
    fixture.write("src/tmux.rs", "pub fn send_keys() {}\n");
    let model = model_of(&fixture);

    let strings = module_edge_strings(&model);
    assert!(
        strings.contains(&"voice-cli:voice-cli::main->voice-cli::audio:".to_string()),
        "qualified edge main->audio missing: {strings:?}"
    );
    assert!(
        strings.contains(&"voice-cli:voice-cli::main->voice-cli::tmux:send_keys".to_string()),
        "use edge main->tmux missing: {strings:?}"
    );
}

// acceptance #24: a bin's main.rs referencing its own lib via a qualified path
// (`voice_app::run()`) yields a main->lib unit edge and never lands in external.
// The lib and bin are separate units (B25); hyphen/underscore is normalized.
#[test]
fn scan_scenario_24_own_crate_qualified_ref_is_unit_edge_not_external() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn run() {}\n");
    fixture.write("src/main.rs", "fn main() { voice_app::run(); }\n");
    let model = model_of(&fixture);

    let edges = model["edges"].as_array().expect("edges");
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        pairs.contains(&("voice-app-bin", "voice-app")),
        "main->lib unit edge missing: {pairs:?}"
    );

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert!(
        !names.contains(&"voice_app") && !names.contains(&"voice-app"),
        "own crate must not be external: {names:?}"
    );

    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert!(
        units.contains(&"voice-app") && units.contains(&"voice-app-bin"),
        "lib and bin must be separate units: {units:?}"
    );
}

// acceptance #25: lib.rs + main.rs are two separate compilation targets.
#[test]
fn scan_scenario_25_lib_and_main_are_separate_units() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn lib_fn() {}\n");
    fixture.write("src/main.rs", "fn main() {}\n");
    let model = model_of(&fixture);

    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        units,
        ["app", "app-bin"],
        "lib and bin must be distinct units (naming: lib=<pkg>, main bin=<pkg>-bin)"
    );
}

// acceptance #25: lib.rs + main.rs + src/bin/tool.rs -> three units.
#[test]
fn scan_scenario_25_three_targets_are_three_units() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn lib_fn() {}\n");
    fixture.write("src/main.rs", "fn main() {}\n");
    fixture.write("src/bin/tool.rs", "fn main() {}\n");
    let model = model_of(&fixture);

    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(units, ["app", "app-bin", "tool"], "three distinct units");
}

// acceptance #26: src/bin/uniffi-bindgen.rs is its own unit.
#[test]
fn scan_scenario_26_src_bin_file_is_own_unit() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"acme-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn lib_fn() {}\n");
    fixture.write("src/bin/uniffi-bindgen.rs", "fn main() {}\n");
    let model = model_of(&fixture);

    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert!(
        units.contains(&"uniffi-bindgen"),
        "src/bin tool must be its own unit: {units:?}"
    );
}

// acceptance #27: stdlib crates (std/core/alloc) are not external.
#[test]
fn scan_scenario_27_stdlib_not_external() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "use std::path::Path;\nuse std::collections::HashMap;\npub fn f(p: &Path) {}\n",
    );
    let model = model_of(&fixture);

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert!(
        !names.contains(&"std"),
        "std must not be external: {names:?}"
    );
    let module_external = model["module_external"]
        .as_object()
        .expect("module_external");
    assert!(
        module_external.values().all(|crates| {
            crates
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .all(|c| c.as_str() != Some("std"))
                })
                .unwrap_or(true)
        }),
        "std must not appear in any module_external list"
    );
}

// acceptance #28: a bare re-export whose first segment is an OWN module is not
// external (`use core::types::*` where `core` is a module of the crate).
#[test]
fn scan_scenario_28_own_modules_not_external() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"acme-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod core;\nmod asr_client;\npub use core::types::*;\n");
    fixture.write("src/core.rs", "pub mod types { pub struct T; }\n");
    fixture.write("src/asr_client.rs", "pub struct AsrTranscriber;\n");
    let model = model_of(&fixture);

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert!(
        !names.contains(&"core") && !names.contains(&"asr_client"),
        "own modules must not be external: {names:?}"
    );
}

// acceptance #29: dash/underscore normalization dedups external crates.
#[test]
fn scan_scenario_29_dash_underscore_normalization() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ntower-http = \"0.1\"\n",
    );
    fixture.write("src/lib.rs", "use tower_http::RequestExt;\npub fn f() {}\n");
    let model = model_of(&fixture);

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert_eq!(
        names,
        ["tower_http"],
        "dash/underscore normalized to a single external entry"
    );
}

// acceptance #30: dev-dependencies are not mixed into dependencies/external.
#[test]
fn scan_scenario_30_dev_deps_separated() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n\n[dev-dependencies]\ntempfile = \"3\"\n",
    );
    fixture.write("src/lib.rs", "pub fn f() {}\n");
    let model = model_of(&fixture);

    let deps = model["manifest"]["dependencies"]
        .as_array()
        .expect("manifest.dependencies");
    let dep_names: Vec<&str> = deps
        .iter()
        .map(|d| d.as_str().expect("dep"))
        .collect();
    assert_eq!(dep_names, ["serde"], "only runtime deps in manifest.dependencies");

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert!(
        !names.contains(&"tempfile"),
        "dev-dep must not be external: {names:?}"
    );
}

// acceptance #31: root_module_declarations capture which feature gates a module.
#[test]
fn scan_scenario_31_feature_name_captured() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "#[cfg(feature = \"tauri\")]\nmod tauri;\nmod plain;\n",
    );
    fixture.write("src/tauri.rs", "pub fn tauri() {}\n");
    fixture.write("src/plain.rs", "pub fn plain() {}\n");
    let model = model_of(&fixture);

    let decls = model["root_module_declarations"]["app"]
        .as_array()
        .expect("declarations");
    let tauri = decls
        .iter()
        .find(|d| d["name"].as_str() == Some("tauri"))
        .expect("tauri declaration");
    assert_eq!(
        tauri["feature"].as_str(),
        Some("tauri"),
        "feature name captured for gated module"
    );
    assert_eq!(tauri["gated"].as_bool(), Some(true), "still gated");

    let plain = decls
        .iter()
        .find(|d| d["name"].as_str() == Some("plain"))
        .expect("plain declaration");
    assert_eq!(plain["feature"].as_str(), None, "ungated module has no feature");
}

// acceptance #32: `publish = false` is distinct from an absent publish field.
#[test]
fn scan_scenario_32_publish_false_distinct_from_absent() {
    let no_publish = common::Fixture::new();
    no_publish.write(
        "Cargo.toml",
        "[package]\nname = \"nopub\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    no_publish.write("src/lib.rs", "pub fn f() {}\n");
    let absent_model = model_of(&no_publish);
    assert_eq!(
        absent_model["manifest"]["publish"],
        serde_json::Value::Null,
        "absent publish field is null (None)"
    );

    let disabled = common::Fixture::new();
    disabled.write(
        "Cargo.toml",
        "[package]\nname = \"disabled\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n",
    );
    disabled.write("src/lib.rs", "pub fn f() {}\n");
    let false_model = model_of(&disabled);
    assert_eq!(
        false_model["manifest"]["publish"],
        serde_json::Value::Bool(false),
        "publish=false is Some(false), distinct from absent"
    );

    let gitea = common::Fixture::new();
    gitea.write(
        "Cargo.toml",
        "[package]\nname = \"pubcrate\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = [\"gitea\"]\n",
    );
    gitea.write("src/lib.rs", "pub fn f() {}\n");
    let gitea_model = model_of(&gitea);
    assert_eq!(
        gitea_model["manifest"]["publish"],
        serde_json::Value::Bool(true),
        "publish=[\"gitea\"] reflects Some(true)"
    );
}

// acceptance #21: at a workspace root (no `[package]`), per-member manifest facts
// are surfaced in `unit_manifests` so `manifest_integrity` is meaningful, not
// vacuous.
#[test]
fn scan_exposes_per_unit_manifest_facts() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\nserde = \"1\"\n\n[features]\ntelemetry = []\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nclap = \"4\"\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    let model = model_of(&fixture);

    let um = model["unit_manifests"].as_object().expect("unit_manifests");
    let auth = um.get("auth").expect("auth unit facts");
    assert_eq!(auth["publish"].as_bool(), Some(false), "auth publish=false");
    let auth_deps: Vec<&str> = auth["dependencies"]
        .as_array()
        .expect("auth deps")
        .iter()
        .map(|d| d.as_str().expect("dep"))
        .collect();
    assert_eq!(auth_deps, ["serde"], "auth deps surface");
    let auth_features: Vec<&str> = auth["features"]
        .as_array()
        .expect("auth features")
        .iter()
        .map(|f| f.as_str().expect("feature"))
        .collect();
    assert_eq!(auth_features, ["telemetry"], "auth features surface");

    let billing = um.get("billing").expect("billing unit facts");
    let billing_deps: Vec<&str> = billing["dependencies"]
        .as_array()
        .expect("billing deps")
        .iter()
        .map(|d| d.as_str().expect("dep"))
        .collect();
    assert_eq!(billing_deps, ["clap"], "billing deps surface");
}

// acceptance #21: workspace member is hard edge not external (regression guard
// for the units-per-target change in B25).
#[test]
fn scan_scenario_21_workspace_member_is_hard_edge_not_external() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub struct Token;\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write(
        "crates/billing/src/lib.rs",
        "use auth::Token;\npub fn bill() {}\n",
    );
    let model = model_of(&fixture);

    let edges = model["edges"].as_array().expect("edges");
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        pairs.contains(&("billing", "auth")),
        "workspace member must be a hard edge: {pairs:?}"
    );
    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert!(
        !names.contains(&"auth"),
        "workspace member must not be external: {names:?}"
    );
}

// acceptance: a pure-bin crate's root module (`main.rs`) IS a soft module node.
// The bin root's edges source `<unit>::main`, so that node must also be present
// in soft_structure — otherwise update seeds a boundary that verify reports as
// a missing component.
#[test]
fn scan_bin_main_is_a_soft_module_node() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-cli\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/main.rs",
        "fn main() {\n    audio::list_devices();\n    asr::transcribe();\n}\nmod audio;\nmod asr;\n",
    );
    fixture.write("src/audio.rs", "pub fn list_devices() {}\n");
    fixture.write("src/asr.rs", "pub fn transcribe() {}\n");
    let model = model_of(&fixture);

    let soft: Vec<&str> = model["soft_structure"]["voice-cli"]
        .as_array()
        .expect("bin unit soft_structure must be an array")
        .iter()
        .map(|p| p.as_str().expect("module path"))
        .collect();
    assert!(
        soft.contains(&"voice-cli::main"),
        "bin root module must be a soft node: {soft:?}"
    );
    assert!(
        soft.contains(&"voice-cli::audio") && soft.contains(&"voice-cli::asr"),
        "declared sub-modules must remain soft nodes: {soft:?}"
    );

    let strings = module_edge_strings(&model);
    assert!(
        strings.contains(&"voice-cli:voice-cli::main->voice-cli::audio:".to_string()),
        "qualified edge main->audio missing: {strings:?}"
    );
    assert!(
        strings.contains(&"voice-cli:voice-cli::main->voice-cli::asr:".to_string()),
        "qualified edge main->asr missing: {strings:?}"
    );
}

// acceptance: for a pure-bin crate, `update --force` seeds a `<unit>::main`
// boundary that matches a real soft node, so a subsequent `verify` exits 0
// (no missing component).
#[test]
fn update_bin_seed_verifies_clean() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-cli\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/main.rs",
        "fn main() {\n    audio::list_devices();\n    asr::transcribe();\n}\nmod audio;\nmod asr;\n",
    );
    fixture.write("src/audio.rs", "pub fn list_devices() {}\n");
    fixture.write("src/asr.rs", "pub fn transcribe() {}\n");

    let update = fixture.run(&["update", "--force"]);
    assert_eq!(
        update.status.code(),
        Some(0),
        "update --force must exit 0 (stderr: {})",
        stderr(&update)
    );
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("matches = { modules = [\"voice-cli::main\"] }"),
        "seed must declare the main boundary:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "verify must exit 0 after seeding (stdout: {}, stderr: {})",
        stdout(&verify),
        stderr(&verify)
    );
}

// acceptance #34: a crate referenced only via a fully-qualified path
// (`toml::from_str(...)` with no `use toml`) is indexed as external when it is a
// declared runtime dependency. Feeds `forbid_external_crates`.
#[test]
fn scan_scenario_34_fully_qualified_external_crate_detected() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"docs-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ntoml = \"0.8\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub fn parse_config() -> toml::Value { toml::from_str(\"\").unwrap() }\n",
    );
    let model = model_of(&fixture);

    let external: Vec<&str> = model["external"]
        .as_array()
        .expect("external")
        .iter()
        .map(|n| n.as_str().unwrap_or_default())
        .collect();
    assert!(
        external.contains(&"toml"),
        "fully-qualified declared dep must be external: {external:?}"
    );

    let module_external = model["module_external"]["docs-lib"]
        .as_array()
        .expect("module_external for docs-lib")
        .iter()
        .map(|n| n.as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert!(
        module_external.contains(&"toml"),
        "module_external must record the fully-qualified dep: {module_external:?}"
    );
}

// acceptance #35: a bin referencing its lib via the lib's crate-id
// (`voice_app_lib::run()`) where `[lib] name` differs from the package name
// still yields a bin->lib edge, and the lib crate-id is not external.
#[test]
fn scan_scenario_35_lib_crate_name_resolves_bin_to_lib_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\nname = \"voice_app_lib\"\n",
    );
    fixture.write("src/lib.rs", "pub fn run() {}\n");
    fixture.write("src/main.rs", "fn main() { voice_app_lib::run(); }\n");
    let model = model_of(&fixture);

    let edges = model["edges"].as_array().expect("edges");
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        pairs.contains(&("voice-app-bin", "voice-app")),
        "bin->lib edge must resolve via lib crate-id: {pairs:?}"
    );

    let external: Vec<&str> = model["external"]
        .as_array()
        .expect("external")
        .iter()
        .map(|n| n.as_str().unwrap_or_default())
        .collect();
    assert!(
        !external.contains(&"voice_app_lib") && !external.contains(&"voice_app"),
        "lib crate-id must not be external: {external:?}"
    );
}

// acceptance: a bin importing its own lib via a `use` statement (`use voice_app::run;`)
// must yield a bin->lib unit edge, not be silently dropped as a project-unit reference.
#[test]
fn scan_use_based_cross_unit_edge_detected() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn run() {}\n");
    fixture.write("src/main.rs", "use voice_app::run;\nfn main() { run(); }\n");
    let model = model_of(&fixture);

    let edges = model["edges"].as_array().expect("edges");
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        pairs.contains(&("voice-app-bin", "voice-app")),
        "use-based bin->lib unit edge missing: {pairs:?}"
    );

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert!(
        !names.contains(&"voice_app") && !names.contains(&"voice-app"),
        "own lib must not be external: {names:?}"
    );
}

// acceptance: a lib referencing itself by its own crate name in a `use` is internal,
// never a spurious self unit edge and never external.
#[test]
fn scan_use_based_self_crate_reference_is_not_self_unit_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod inner;\nuse voice_app::inner;\n");
    fixture.write("src/inner.rs", "pub fn helper() {}\n");
    let model = model_of(&fixture);

    let edges = model["edges"].as_array().expect("edges");
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        !pairs.contains(&("voice-app", "voice-app")),
        "self unit edge must not exist: {pairs:?}"
    );

    let external = model["external"].as_array().expect("external");
    let names: Vec<&str> = external
        .iter()
        .map(|n| n.as_str().expect("crate name"))
        .collect();
    assert!(
        !names.contains(&"voice_app") && !names.contains(&"voice-app"),
        "own crate must not be external: {names:?}"
    );
}

// workplan_03 scenario "declared bin targeting main.rs yields one unit": a
// `[[bin]]` entry pointing at `src/main.rs` replaces the implicit `<pkg>-bin`
// unit for that file — exactly one bin unit, named from the declaration.
#[test]
fn scan_declared_bin_at_main_yields_single_unit() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"toolname\"\npath = \"src/main.rs\"\n",
    );
    fixture.write("src/lib.rs", "pub fn lib_fn() {}\n");
    fixture.write("src/main.rs", "mod mainmod;\n\nfn main() {}\n");
    fixture.write("src/mainmod.rs", "pub fn main_fn() {}\n");
    let model = model_of(&fixture);

    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        units,
        ["app", "toolname"],
        "declared bin at main.rs must replace the implicit <pkg>-bin unit"
    );

    let bin_modules: Vec<&str> = model["soft_structure"]["toolname"]
        .as_array()
        .expect("declared bin soft_structure must be an array")
        .iter()
        .map(|m| m.as_str().expect("module path"))
        .collect();
    assert!(
        bin_modules.contains(&"toolname::main") && bin_modules.contains(&"toolname::mainmod"),
        "modules must be attributed once to the declared unit: {bin_modules:?}"
    );
    assert!(
        model["soft_structure"].get("app-bin").is_none(),
        "phantom <pkg>-bin soft_structure must be absent"
    );
}

// workplan_03 scenario "implicit main.rs naming unchanged": without any
// `[[bin]]` declarations, `main.rs` alongside a lib keeps the `<pkg>-bin` name.
#[test]
fn scan_implicit_main_bin_naming_unchanged_without_bin_section() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn lib_fn() {}\n");
    fixture.write("src/main.rs", "fn main() {}\n");
    let model = model_of(&fixture);

    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(units, ["app", "app-bin"], "implicit naming must stay <pkg>-bin");
}

// workplan_03 scenario "src/bin targets unaffected": a declared bin at
// `main.rs` must not disturb auto-discovered `src/bin/*.rs` targets.
#[test]
fn scan_declared_bin_at_main_leaves_src_bin_targets_unaffected() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"toolname\"\npath = \"src/main.rs\"\n",
    );
    fixture.write("src/lib.rs", "pub fn lib_fn() {}\n");
    fixture.write("src/main.rs", "fn main() {}\n");
    fixture.write("src/bin/extra.rs", "fn main() {}\n");
    let model = model_of(&fixture);

    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        units,
        ["app", "extra", "toolname"],
        "src/bin target must appear once under its own name"
    );
}

// workplan_03 scenario "edges attribute to the declared unit": the lib
// dependency edge is attributed once to the declared bin; no edge references
// the phantom unit and no duplicate edge pair exists.
#[test]
fn scan_declared_bin_at_main_attributes_edges_once() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"toolname\"\npath = \"src/main.rs\"\n",
    );
    fixture.write("src/lib.rs", "pub fn run() {}\n");
    fixture.write("src/main.rs", "fn main() { voice_app::run(); }\n");
    let model = model_of(&fixture);

    let edges: Vec<(&str, &str)> = model["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap_or_default(),
                e["to"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    let declared_edges: Vec<&(&str, &str)> = edges
        .iter()
        .filter(|(from, to)| *from == "toolname" && *to == "voice-app")
        .collect();
    assert_eq!(
        declared_edges.len(),
        1,
        "dependency edge must be attributed once to the declared unit: {edges:?}"
    );
    assert!(
        !edges.iter().any(|(from, to)| from.contains("-bin") || to.contains("-bin")),
        "no edge may reference the phantom unit: {edges:?}"
    );
}
