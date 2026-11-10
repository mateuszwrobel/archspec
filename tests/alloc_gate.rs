//! Allocator gate probes (issue #33, decision-check M5 bounds).
//!
//! The `musl-alloc` feature may change the dependency graph of exactly one
//! build shape: a musl target with the feature explicitly enabled (the
//! release matrix's musl steps). These probes read the resolved graph from
//! `cargo metadata` per platform and prove the other shapes stay
//! byte-unchanged: default builds, non-musl builds with the feature, and
//! musl builds without it.

use std::path::PathBuf;
use std::process::Command;

fn payload_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Names in the resolved dependency graph for one platform, with extra
/// `--features` flags applied.
fn resolved_names(platform: &str, features: &[&str]) -> Vec<String> {
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
    meta["packages"]
        .as_array()
        .expect("packages array")
        .iter()
        .filter_map(|p| p["name"].as_str().map(str::to_string))
        .collect()
}

fn has_mimalloc(names: &[String]) -> bool {
    names.iter().any(|n| n == "mimalloc" || n == "libmimalloc-sys")
}

#[test]
fn default_resolution_carries_no_allocator_crate() {
    // Every build shape without the explicit feature — dev, test, and the
    // plain release builds — must resolve exactly the previous graph.
    for platform in [
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ] {
        let names = resolved_names(platform, &[]);
        assert!(
            !has_mimalloc(&names),
            "{platform} without the feature must not resolve mimalloc"
        );
    }
}

#[test]
fn feature_enables_mimalloc_only_on_musl_targets() {
    // The feature alone must not reach gnu/darwin/windows: the dependency is
    // target-gated, so their graphs stay byte-unchanged even with the flag.
    for platform in [
        "x86_64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "aarch64-unknown-linux-gnu",
    ] {
        let names = resolved_names(platform, &["musl-alloc"]);
        assert!(
            !has_mimalloc(&names),
            "{platform} with the feature must not resolve mimalloc (target gate)"
        );
    }
    // Only the musl shapes carry it once the feature is enabled — the exact
    // graph the release matrix's musl build steps get.
    for platform in ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"] {
        let names = resolved_names(platform, &["musl-alloc"]);
        assert!(
            has_mimalloc(&names),
            "{platform} with the feature must resolve mimalloc — the cure would \
             silently no-op the musl artifacts if this failed"
        );
    }
}

#[test]
fn release_profile_carries_the_lto_rider() {
    // Rider guard (decision-check M6, exposed test gap): the release profile
    // must set fat LTO and one codegen unit in the manifest itself, so a
    // silent profile deletion shows up as a red test, not a slow release.
    let text = std::fs::read_to_string(payload_root().join("Cargo.toml"))
        .expect("the crate manifest must be readable");
    let after = text
        .split("[profile.release]")
        .nth(1)
        .expect("Cargo.toml must carry a [profile.release] section (issue #33 rider)");
    let profile = after.split('[').next().unwrap_or(after);
    assert!(
        profile.contains("lto") && profile.contains("fat"),
        "[profile.release] must set lto = \"fat\""
    );
    assert!(
        profile.replace(' ', "").contains("codegen-units=1"),
        "[profile.release] must set codegen-units = 1"
    );
}
