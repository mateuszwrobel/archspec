//! Golden derivation events (proposal hole (c)).
//!
//! Every file under `tests/goldens/` must appear in
//! `tests/goldens/derivation.json` with a content fingerprint and a named
//! derivation event, so a golden's freshness is DERIVED — a byte change
//! breaks the fingerprint and the guard names the file until the event is
//! updated. The fingerprint is FNV-1a 64 (a freshness weld, not a digest
//! claim), computed identically here and in regeneration mode.
//!
//! Regenerate after deliberately re-recording goldens
//! (`ARCHSPEC_GOLDEN=1 cargo test --test extraction_freeze`) with
//! `ARCHSPEC_GOLDEN=1 cargo test --test golden_derivation`: unchanged files
//! keep their events; changed or new files land with a PLACEHOLDER event and
//! the plain guard stays red until each placeholder names the event that
//! moved the bytes.

mod common;
#[allow(dead_code)]
mod shared;

use serde_json::{json, Value};
use shared::matrix::file_fingerprint;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const PLACEHOLDER: &str = "PLACEHOLDER — name the derivation event that moved these bytes";

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
        .join("derivation.json")
}

fn goldens_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read goldens dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.file_name().map(|n| n != "derivation.json").unwrap_or(false) {
            out.push(path);
        }
    }
}

fn golden_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(&goldens_dir(), &mut files);
    files.sort();
    files
}

fn relative(path: &Path) -> String {
    path.strip_prefix(goldens_dir())
        .expect("golden under tests/goldens")
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn every_golden_carries_a_current_derivation_event() {
    if std::env::var("ARCHSPEC_GOLDEN").is_ok() {
        regenerate();
        return;
    }
    let raw = std::fs::read_to_string(manifest_path())
        .expect("tests/goldens/derivation.json must exist — regenerate in golden mode");
    let manifest: Value = serde_json::from_str(&raw).expect("derivation manifest parses");
    let entries: Vec<&Value> = manifest["goldens"]
        .as_array()
        .expect("goldens array")
        .iter()
        .collect();
    let mut by_path: BTreeMap<&str, &Value> = BTreeMap::new();
    let mut problems: Vec<String> = Vec::new();
    for entry in &entries {
        let path = entry["path"].as_str().expect("entry path");
        if by_path.insert(path, entry).is_some() {
            problems.push(format!("{path}: duplicate manifest entry"));
        }
    }
    for file in golden_files() {
        let rel = relative(&file);
        match by_path.get(rel.as_str()) {
            None => problems.push(format!("{rel}: golden exists but has no derivation entry")),
            Some(entry) => {
                let want = file_fingerprint(&file);
                let got = entry["fingerprint"].as_str().unwrap_or_default();
                if got != want {
                    problems.push(format!(
                        "{rel}: bytes changed (manifest {got}, actual {want}) — update the fingerprint AND name the derivation event in tests/goldens/derivation.json"
                    ));
                }
                let event = entry["derivation_event"].as_str().unwrap_or_default();
                if event.is_empty() || event.starts_with("PLACEHOLDER") {
                    problems.push(format!("{rel}: derivation_event must name a real event, got {event:?}"));
                }
            }
        }
    }
    for entry in &entries {
        let path = entry["path"].as_str().unwrap_or_default();
        if !goldens_dir().join(path).exists() {
            problems.push(format!("{path}: manifest entry names a golden that no longer exists — remove the entry"));
        }
    }
    assert!(problems.is_empty(), "golden derivation drift:\n{}", problems.join("\n"));
}

fn regenerate() {
    let previous: Value = std::fs::read(manifest_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| json!({"goldens": []}));
    let mut prev_by_path: BTreeMap<String, (String, String)> = BTreeMap::new();
    if let Some(arr) = previous["goldens"].as_array() {
        for entry in arr {
            prev_by_path.insert(
                entry["path"].as_str().unwrap_or_default().to_string(),
                (
                    entry["fingerprint"].as_str().unwrap_or_default().to_string(),
                    entry["derivation_event"].as_str().unwrap_or_default().to_string(),
                ),
            );
        }
    }
    let mut goldens = Vec::new();
    let mut churned: Vec<String> = Vec::new();
    for file in golden_files() {
        let rel = relative(&file);
        let fingerprint = file_fingerprint(&file);
        let event = match prev_by_path.get(&rel) {
            Some((prev_fp, prev_event)) if *prev_fp == fingerprint && !prev_event.is_empty() => {
                prev_event.clone()
            }
            _ => {
                churned.push(rel.clone());
                PLACEHOLDER.to_string()
            }
        };
        goldens.push(json!({"path": rel, "fingerprint": fingerprint, "derivation_event": event}));
    }
    let manifest = json!({
        "schema": "archspec-golden-derivation-v0",
        "generated_by": "tests/golden_derivation.rs in ARCHSPEC_GOLDEN mode (never hand-edit fingerprints)",
        "goldens": goldens,
    });
    std::fs::write(manifest_path(), serde_json::to_vec_pretty(&manifest).expect("write manifest"))
        .expect("write derivation.json");
    if churned.is_empty() {
        println!("derivation manifest regenerated: no bytes moved, events preserved");
    } else {
        println!(
            "derivation manifest regenerated; {} golden(s) need their event named: {:?}",
            churned.len(),
            churned
        );
    }
}
