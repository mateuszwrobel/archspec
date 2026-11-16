//! Weld for the discovery-extraction rung one: the scan engine is reachable
//! from the LIBRARY target alone — no binary spawn, no `#[path]` include. A
//! consumer adds `rust-arch-test-kit` as a dependency, calls
//! [`rust_arch_test_kit::scan::extract`] with a `Language` and a root, and
//! receives the same `model::Model` vocabulary the `archspec scan` verb
//! prints. The extract is the stateless fresh walk: it never writes to the
//! scanned tree, so running it twice over one fixture must leave a
//! byte-identical filesystem snapshot (the zero-writes law).

use rust_arch_test_kit::language::Language;
use rust_arch_test_kit::scan;
use std::fs;
use std::path::{Path, PathBuf};

/// One small Rust crate: two top-level modules with one crossing import, so
/// a model with exactly one module edge must come back.
fn write_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(root.join("src/lib.rs"), "mod auth;\nmod billing;\n").expect("write lib.rs");
    fs::write(root.join("src/auth.rs"), "pub struct Token;\n").expect("write auth.rs");
    fs::write(
        root.join("src/billing.rs"),
        "use crate::auth::Token;\n",
    )
    .expect("write billing.rs");
}

/// The filesystem snapshot: every file path with its length and modification
/// time. Any write — new file, changed bytes, or a touched mtime — moves it.
fn snapshot(root: &Path) -> Vec<(PathBuf, u64, i64)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64, i64)>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .expect("read dir")
            .map(|entry| entry.expect("dir entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, out);
            } else {
                let meta = fs::metadata(&path).expect("metadata");
                let mtime = meta
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_nanos() as i64)
                            .unwrap_or(0)
                    })
                    .unwrap_or_default();
                out.push((path, meta.len(), mtime));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

#[test]
fn lib_scan_surface_extracts_facts_and_writes_nothing() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    write_fixture(dir.path());

    let before = snapshot(dir.path());
    let first = scan::extract(Language::Rust, dir.path()).expect("lib-only extract");
    let after_first = snapshot(dir.path());
    let second = scan::extract(Language::Rust, dir.path()).expect("second extract");
    let after_second = snapshot(dir.path());

    // Facts are reachable from the lib target alone: the crossing import is
    // a module edge and the crate is a unit.
    assert!(
        first
            .units
            .iter()
            .any(|unit| unit.name == "app" && unit.kind == "crate"),
        "the crate unit must be in the model"
    );
    assert!(
        first.module_edges.iter().any(|edge| {
            edge.from == "app::billing"
                && edge.to == "app::auth"
                && edge.symbols.iter().any(|s| s == "Token")
        }),
        "the billing -> auth import edge must be in the model: {first:?}"
    );

    // The two extracts answer the same facts — the walk is stateless.
    assert_eq!(
        first.to_json().expect("json"),
        second.to_json().expect("json"),
        "two extracts of one tree must answer the same model"
    );

    // Zero filesystem delta across both runs (the zero-writes law).
    assert_eq!(before, after_first, "the extract changed the tree");
    assert_eq!(before, after_second, "the second extract changed the tree");
}
