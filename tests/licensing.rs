//! Licensing guard (owner decision 2026-09-23: the tool is licensed
//! Apache-2.0). Four legs, each pinning one falsifiable surface of the
//! payload against its canonical source:
//!
//! 1. `LICENSE` exists at the payload root and opens with the canonical
//!    Apache-2.0 header lines, and carries the neutral copyright line.
//! 2. `Cargo.toml` states `license = "Apache-2.0"` (SPDX, parse-verified).
//! 3. `CREDITS.md` exists, is generated-shaped, lists at least as many
//!    dependency rows as the lockfile-resolved *normal* dependency closure
//!    of the root package (computed from `cargo metadata`, dev and
//!    build-host-only crates excluded), and every row carries a license.
//! 4. No remnant of the former permissive-only license token survives
//!    anywhere in the payload text — the one exception is `CREDITS.md`,
//!    whose rows state third-party licenses verbatim.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

/// Canonical first non-empty line of the Apache-2.0 license text.
const APACHE_TITLE: &str = "Apache License";
/// Canonical second line of the Apache-2.0 license text.
const APACHE_VERSION_LINE: &str = "Version 2.0, January 2004";
/// The license's own URL, part of the canonical header block.
const APACHE_URL: &str = "http://www.apache.org/licenses/";
/// The operative section heading of the Apache-2.0 text.
const APACHE_TERMS: &str = "TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION";
/// Neutral copyright line required in the license appendix.
const COPYRIGHT_LINE: &str = "Copyright 2026 archspec contributors";

fn payload_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The former license token, assembled from bytes so this scanner's own
/// source never contains it as a searchable word.
fn former_token() -> Vec<u8> {
    vec![b'M', b'I', b'T']
}

/// True when `haystack` contains `needle` at a word boundary on both sides
/// (boundaries are ASCII alphanumeric or `_`, mirroring `\b`).
fn contains_word(haystack: &str, needle: &[u8]) -> bool {
    let h = haystack.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    if needle.is_empty() || h.len() < needle.len() {
        return false;
    }
    h.windows(needle.len()).enumerate().any(|(i, w)| {
        w == needle
            && (i == 0 || !is_word(h[i - 1]))
            && (i + needle.len() == h.len() || !is_word(h[i + needle.len()]))
    })
}

#[test]
fn license_file_carries_canonical_apache_text() {
    let path = payload_root().join("LICENSE");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("payload root must carry a LICENSE file ({}): {e}", path.display()));
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.first().copied(),
        Some(APACHE_TITLE),
        "LICENSE must open with the canonical Apache title line"
    );
    assert_eq!(
        lines.get(1).copied(),
        Some(APACHE_VERSION_LINE),
        "LICENSE line 2 must be the canonical version/date line"
    );
    assert_eq!(
        lines.get(2).copied(),
        Some(APACHE_URL),
        "LICENSE line 3 must be the canonical license URL"
    );
    assert!(
        text.contains(APACHE_TERMS),
        "LICENSE must carry the full operative terms section, not a pointer"
    );
    assert!(
        text.contains(COPYRIGHT_LINE),
        "LICENSE appendix must state the neutral copyright line `{COPYRIGHT_LINE}`"
    );
}

#[test]
fn cargo_manifest_declares_apache_spdx_license() {
    let path = payload_root().join("Cargo.toml");
    let raw = std::fs::read_to_string(&path).expect("payload Cargo.toml must be readable");
    let value: toml::Value = raw
        .parse::<toml::Value>()
        .unwrap_or_else(|e| panic!("Cargo.toml must parse as TOML: {e}"));
    let license = value["package"]["license"]
        .as_str()
        .expect("[package] license must be a string");
    assert_eq!(
        license, "Apache-2.0",
        "Cargo.toml [package] license must be the Apache-2.0 SPDX identifier"
    );
}

/// The lockfile-resolved normal dependency closure of the root package:
/// every node reachable from the root through `normal` edges only (dev and
/// build-host-only crates are not part of the binary's linked set).
fn resolved_normal_deps() -> Vec<(String, String)> {
    let root = payload_root();
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--locked", "--offline"])
        .current_dir(&root)
        .output()
        .expect("`cargo metadata` must be runnable from the gate environment");
    assert!(
        output.status.success(),
        "cargo metadata must resolve the lockfile offline: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let meta: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits format-version 1 JSON");
    let root_id = meta["resolve"]["root"].as_str().expect("resolve.root");
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in meta["resolve"]["nodes"].as_array().expect("resolve.nodes") {
        let id = node["id"].as_str().expect("node.id");
        if let Some(deps) = node["deps"].as_array() {
            for dep in deps {
                let pkg = dep["pkg"].as_str().expect("dep.pkg");
                let is_normal = match dep["dep_kinds"].as_array() {
                    Some(kinds) => kinds.iter().any(|k| {
                        k.get("kind").map(|v| v.is_null()).unwrap_or(true)
                    }),
                    None => true,
                };
                if is_normal {
                    adj.entry(id).or_default().push(pkg);
                }
            }
        }
    }
    let mut seen: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    queue.push_back(root_id);
    seen.insert(root_id);
    let mut pkgs: HashMap<&str, &serde_json::Value> = HashMap::new();
    for pkg in meta["packages"].as_array().expect("packages") {
        pkgs.insert(pkg["id"].as_str().expect("package.id"), pkg);
    }
    let mut out = Vec::new();
    while let Some(id) = queue.pop_front() {
        if id != root_id {
            let pkg = pkgs[id];
            out.push((
                pkg["name"].as_str().expect("name").to_string(),
                pkg["version"].as_str().expect("version").to_string(),
            ));
        }
        for next in adj.get(id).into_iter().flatten() {
            if seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    out
}

/// `(crate, version, license)` triples of the generated markdown rows.
/// A row is `| \`name\` | version | license | url |`.
fn credit_rows(credits: &str) -> Vec<(String, String, String)> {
    let mut rows = Vec::new();
    for line in credits.lines() {
        if !line.starts_with("| `") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        assert_eq!(
            cells.len(),
            6,
            "credit row must have exactly four cells between the pipes: {line}"
        );
        let name = cells[1]
            .strip_prefix('`')
            .and_then(|n| n.strip_suffix('`'))
            .unwrap_or_else(|| panic!("crate name cell must be backticked: {line}"));
        rows.push((name.to_string(), cells[2].to_string(), cells[3].to_string()));
    }
    rows
}

#[test]
fn credits_listing_covers_the_resolved_dependency_closure() {
    let path = payload_root().join("CREDITS.md");
    let credits = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("payload root must carry CREDITS.md ({}): {e}", path.display()));
    assert!(
        credits.contains("Apache License 2.0"),
        "CREDITS.md header must state the top-level crate license"
    );
    assert!(
        credits.contains("scripts/gen-credits.sh"),
        "CREDITS.md header must name its regeneration script"
    );
    let rows = credit_rows(&credits);
    assert!(!rows.is_empty(), "CREDITS.md must carry dependency rows");
    let expected = resolved_normal_deps();
    assert!(
        rows.len() >= expected.len(),
        "CREDITS.md lists {} dependency rows but the lockfile resolves {} normal \
         dependencies — regenerate with ./scripts/gen-credits.sh",
        rows.len(),
        expected.len()
    );
    for (name, version, license) in &rows {
        assert!(!license.is_empty(), "row `{name} {version}` has no license field");
    }
    for (name, version) in &expected {
        assert!(
            rows.iter()
                .any(|(n, v, _)| n == name && v == version),
            "resolved dependency `{name} {version}` is missing from CREDITS.md — \
             regenerate with ./scripts/gen-credits.sh"
        );
    }
    // The table lists dependencies, never the tool itself.
    assert!(
        !rows.iter().any(|(n, _, _)| n == "rust-arch-test-kit"),
        "CREDITS.md rows must list dependencies, not the root crate"
    );
}

#[test]
fn no_former_license_token_remains_in_the_payload() {
    let root = payload_root();
    let token = former_token();
    let mut hits: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            !matches!(
                e.file_name().to_str(),
                Some("target") | Some(".git")
            )
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        // The credits listing states third-party licenses verbatim; license
        // expressions of dependencies are legitimate content there.
        if entry.path().file_name().and_then(|n| n.to_str()) == Some("CREDITS.md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue; // binary or unreadable content carries no prose claim
        };
        if contains_word(&text, &token) {
            hits.push(entry.path().strip_prefix(&root).unwrap_or(entry.path()).to_path_buf());
        }
    }
    assert!(
        hits.is_empty(),
        "former license token remains in the payload — licensing must be Apache-2.0 \
         everywhere (Cargo.toml, .PKGINFO, docs): {hits:?}"
    );
}
