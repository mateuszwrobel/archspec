//! Phase-profile gate probes (GitHub issue #34, operator ruling).
//!
//! The `profile-phases` feature is a DEV-only harness: `hotpath` rides as a
//! dev-dependency and must NEVER join the normal dependency graph of any
//! build shape — with or without the feature, on every published platform.
//! The shipped binary keeps zero profiling cost: the binary target refuses
//! the feature at compile time, and the feature-gated phase instrumentation
//! in the sources compiles to nothing when the feature is off. These probes
//! read the resolved graph from `cargo metadata` (same shape as
//! `alloc_gate.rs`) plus the manifest/source bytes the guards pin.

use std::path::PathBuf;
use std::process::Command;

fn payload_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Names reachable from the root package through NON-dev resolve edges for
/// one platform, with extra `--features` flags applied. Dev edges (tests,
/// examples, benches) are never linked into the shipped binaries, so the
/// closure they exclude is exactly the shipped graph.
fn normal_closure(platform: &str, features: &[&str]) -> Vec<String> {
    let mut cmd = Command::new("cargo");
    cmd.arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--locked")
        .arg("--offline")
        .arg("--filter-platform")
        .arg(platform)
        .current_dir(payload_root());
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    let output = cmd
        .output()
        .expect("`cargo metadata` must be runnable from the gate environment");
    assert!(
        output.status.success(),
        "cargo metadata must resolve the lockfile offline: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let meta: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("metadata emits format-version 1 JSON");
    let root_id = meta["resolve"]["root"].as_str().expect("root node id");
    let nodes: Vec<&serde_json::Value> = meta["resolve"]["nodes"]
        .as_array()
        .expect("resolve.nodes array")
        .iter()
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut queue = vec![root_id];
    let mut out = Vec::new();
    while let Some(id) = queue.pop() {
        let Some(node) = nodes.iter().find(|n| n["id"].as_str() == Some(id)) else {
            continue;
        };
        if id != root_id {
            let pkg = &meta["metadata"]["packages"][id];
            // The metadata map keys packages by the same node id; fall back
            // to parsing the name out of the id when the map omits it.
            let name = pkg["name"].as_str().map_or_else(
                || {
                    id.rsplit('#')
                        .next()
                        .unwrap_or(id)
                        .split('@')
                        .next()
                        .unwrap_or(id)
                        .to_string()
                },
                str::to_string,
            );
            out.push(name);
        }
        for dep in node["deps"].as_array().into_iter().flatten() {
            // Pure dev/build edges are never linked into the shipped binary
            // of this shape; everything else rides the shipped graph.
            let shipped = dep["dep_kinds"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|kind| !matches!(kind["kind"].as_str(), Some("dev") | Some("build")));
            if !shipped {
                continue;
            }
            if let Some(next) = dep["pkg"].as_str()
                && seen.insert(next)
            {
                queue.push(next);
            }
        }
    }
    out
}

fn carries_hotpath(names: &[String]) -> bool {
    names.iter().any(|n| n == "hotpath" || n == "hotpath-macros" || n == "hotpath-drain")
}

#[test]
fn hotpath_never_enters_the_normal_graph_in_any_shape() {
    // The ruling in test form: no build shape that links the shipped binary
    // may resolve hotpath — not even a release build that passes the feature
    // explicitly (dev edges stay dev edges).
    for platform in [
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ] {
        for features in [Vec::new(), vec!["profile-phases"], vec!["musl-alloc", "profile-phases"]] {
            let names = normal_closure(platform, &features);
            assert!(
                !carries_hotpath(&names),
                "{platform} with features {features:?} must not reach hotpath through \
                 non-dev edges — the shipped binary keeps zero profiling cost"
            );
        }
    }
}

#[test]
fn hotpath_is_declared_a_dev_dependency() {
    // The manifest edge kind is the declaration the graph proof rides on.
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--locked")
        .arg("--offline")
        .current_dir(payload_root())
        .output()
        .expect("`cargo metadata` must be runnable from the gate environment");
    assert!(output.status.success());
    let meta: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("metadata emits format-version 1 JSON");
    let root_id = meta["resolve"]["root"].as_str().expect("root node id");
    let root = meta["resolve"]["nodes"]
        .as_array()
        .expect("resolve.nodes array")
        .iter()
        .find(|n| n["id"].as_str() == Some(root_id))
        .expect("root node present");
    let hotpath = root["deps"]
        .as_array()
        .expect("root deps array")
        .iter()
        .find(|d| d["name"].as_str() == Some("hotpath"))
        .expect("hotpath must be declared — the issue #34 harness rides on it");
    for kind in hotpath["dep_kinds"]
        .as_array()
        .expect("dep_kinds array")
        .iter()
    {
        assert_eq!(
            kind["kind"].as_str(),
            Some("dev"),
            "hotpath must join the graph as a dev-dependency only"
        );
    }
}

#[test]
fn the_shipped_binary_refuses_the_profile_feature_at_compile_time() {
    // The dev-only guard in the binary target: enabling the feature in any
    // binary build fails loud with a named error, so a release pipeline can
    // never bake profiling into the shipped artifact.
    let text = std::fs::read_to_string(payload_root().join("src/main.rs"))
        .expect("the binary source must be readable");
    assert!(
        text.contains("all(feature = \"profile-phases\", not(test))") && text.contains("compile_error!"),
        "src/main.rs must carry a compile_error! guard binding profile-phases \
         to the dev harness only"
    );
}

#[test]
fn the_manifest_declares_the_feature_and_the_dev_only_dependency() {
    let text = std::fs::read_to_string(payload_root().join("Cargo.toml"))
        .expect("the crate manifest must be readable");
    let features = text
        .split("[features]")
        .nth(1)
        .expect("Cargo.toml must carry a [features] section")
        .split("\n[")
        .next()
        .unwrap_or("");
    assert!(
        features.contains("profile-phases") && features.contains("hotpath"),
        "[features] must define profile-phases riding on hotpath"
    );
    let dev = text
        .split("[dev-dependencies]")
        .nth(1)
        .expect("Cargo.toml must carry a [dev-dependencies] section")
        .split('[')
        .next()
        .unwrap_or("");
    assert!(
        dev.contains("hotpath"),
        "hotpath must be declared under [dev-dependencies] — never under [dependencies]"
    );
    assert!(
        !text
            .split("[dependencies]")
            .nth(1)
            .unwrap_or("")
            .split("\n[")
            .next()
            .unwrap_or("")
            .contains("hotpath"),
        "hotpath must not appear under [dependencies]"
    );
}

#[test]
fn the_phase_budget_table_ships_and_the_help_surface_cites_it() {
    let doc = payload_root().join("docs/archspec/performance.md");
    let text = std::fs::read_to_string(&doc)
        .unwrap_or_else(|e| panic!("the measured phase budgets must ship in {doc:?}: {e}"));
    for marker in [
        "Measured phase budgets",
        "profile-phases",
        "dev-dependency",
        "machine",
        "2026-11-09",
    ] {
        assert!(
            text.contains(marker),
            "performance.md must state `{marker}` — every measured number carries \
             its command, machine class and date"
        );
    }
    // The binary-owned citation: the capabilities topic points at the table.
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "capabilities"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("docs/archspec/performance.md"),
        "`archspec help capabilities` must cite the phase-budget table by path"
    );
}

mod common;

use common::Fixture;
