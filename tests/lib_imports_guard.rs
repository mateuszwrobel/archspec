//! Source guard for cross-crate library imports (self-dogfood hardening,
//! deliverable C). A `use rust_arch_test_kit::…` line produces no module edge
//! in the extracted model — cross-crate imports surface only as the unit-tier
//! `archspec -> rust-arch-test-kit` edge — so no boundary rule can see WHICH
//! files import the library. The spec comment states that limitation; this
//! test owns the source-level fact: every file under `src/` that imports the
//! library must sit on the allowlist below, and every allowlist entry must
//! import something. `tests/` files may import the library freely — this
//! guard watches `src/` only.

use std::fs;
use std::path::{Path, PathBuf};

/// The files allowed to reference the library crate: the two graph engines,
/// the inspect renderers (one directory, all members) and the rust scan
/// driver. Keep this list and the engines-group comment in
/// `architecture.spec.toml` in step.
const LIB_IMPORT_ALLOWLIST: &[&str] = &[
    "src/archspec/depgraph.rs",
    "src/archspec/diagram.rs",
    "src/archspec/inspect/*.rs",
    "src/archspec/scan/rust.rs",
];

/// The import marker: a path into the library crate. Matching on the `::`
/// avoids flagging prose that merely names the crate.
const LIB_IMPORT_MARKER: &str = "rust_arch_test_kit::";

/// One allowlist entry against one crate-relative source path. Entries without
/// a `*` match exactly; the single `dir/*.rs` form matches direct members of
/// that directory only.
fn matches_allow_entry(entry: &str, rel: &str) -> bool {
    match entry.split_once('*') {
        None => entry == rel,
        Some((prefix, suffix)) => {
            rel.strip_prefix(prefix)
                .and_then(|rest| rest.strip_suffix(suffix))
                .is_some_and(|middle| !middle.contains('/') && !middle.is_empty())
        }
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn every_library_import_in_src_sits_on_the_allowlist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_rs_files(&root.join("src"), &mut files);
    assert!(files.len() > 30, "the source walk found too few files");

    let mut offenders: Vec<String> = Vec::new();
    let mut matched_entries: Vec<&str> = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .expect("file under the crate root")
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path).expect("read source file");
        if !text.contains(LIB_IMPORT_MARKER) {
            continue;
        }
        match LIB_IMPORT_ALLOWLIST
            .iter()
            .find(|entry| matches_allow_entry(entry, &rel))
        {
            Some(entry) => matched_entries.push(entry),
            None => offenders.push(rel),
        }
    }

    assert!(
        offenders.is_empty(),
        "files import the library outside the allowlist {LIB_IMPORT_ALLOWLIST:?}: {offenders:?}\n\
         a cross-crate import adds a unit-tier edge attributed to the engines group — \
         if this file is a new engine, update the allowlist and the engines-group comment \
         in architecture.spec.toml together"
    );
    let mut stale: Vec<&&str> = LIB_IMPORT_ALLOWLIST
        .iter()
        .filter(|entry| !matched_entries.contains(entry))
        .collect();
    stale.sort();
    stale.dedup();
    assert!(
        stale.is_empty(),
        "allowlist entries import nothing anymore (stale entries): {stale:?}"
    );
}
