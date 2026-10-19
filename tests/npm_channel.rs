//! npm-channel static guards (ADR-051 implementation lane).
//!
//! The executable npm behavior lives in `npm/test/` (node's own runner);
//! this suite pins what must be TRUE WITHOUT RUNNING NODE — the pairings
//! between the five-key matrix and the release machinery's emitted names,
//! the publish workflow's gate/freeze structure, the windows sidecar format
//! fix, and the payload-neutrality of the npm tree (the exact grep against
//! the publish guard's identifier list runs outside the payload, in the
//! project's internal test suite, because the list belongs to the publish
//! workflow).

use std::fs;
use std::path::Path;

fn manifest() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn load(rel: &str) -> String {
    fs::read_to_string(format!("{}/{rel}", manifest())).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// The five matrix asset names as `npm/lib/matrix.mjs` spells them, paired
/// with the token form the payload's `release.yml` uses to emit them (the
/// same pairing law `tests/blind5_release_prep.rs` pins for the doc surface).
const MATRIX_PAIRS: &[(&str, &str)] = &[
    (
        "archspec-${version}-x86_64-unknown-linux-musl.tar.gz",
        "x86_64-unknown-linux-musl.tar.gz",
    ),
    (
        "archspec-${version}-aarch64-unknown-linux-gnu.tar.gz",
        "aarch64-unknown-linux-gnu.tar.gz",
    ),
    (
        "archspec-${version}-x86_64-apple-darwin.tar.gz",
        "x86_64-apple-darwin.tar.gz",
    ),
    (
        "archspec-${version}-aarch64-apple-darwin.tar.gz",
        "aarch64-apple-darwin.tar.gz",
    ),
    (
        "archspec-${version}-x86_64-pc-windows-msvc.zip",
        "x86_64-pc-windows-msvc.zip",
    ),
];

#[test]
fn matrix_names_pair_with_the_release_machinery() {
    let matrix_src = load("npm/lib/matrix.mjs");
    let release = load(".github/workflows/release.yml");
    for (matrix_form, release_suffix) in MATRIX_PAIRS {
        let ver_underbrace = matrix_form.replace("${version}", "${ver}");
        let ver_dollar = matrix_form.replace("${version}", "$ver");
        assert!(
            release.contains(ver_underbrace.as_str()) || release.contains(ver_dollar.as_str()),
            "release.yml must emit {matrix_form} — doc, matrix and workflow cite the same bytes"
        );
        assert!(
            matrix_src.contains(matrix_form),
            "matrix.mjs must name {matrix_form}"
        );
        assert!(
            matrix_src.contains(release_suffix),
            "the matrix carries the target triple {release_suffix}"
        );
    }
    // The matrix law excludes the non-installables by construction.
    assert!(!matrix_src.contains(".deb"), "no deb name in the matrix");
    assert!(!matrix_src.contains(".pkg.tar.zst"), "no zst name in the matrix");
}

#[test]
fn workflow_carries_the_freeze_gate_and_the_publish_job() {
    let release = load(".github/workflows/release.yml");
    // Per-tag serialization without cancellation (republish-freeze law).
    assert!(release.contains("group: archspec-release-${{ github.ref_name }}"));
    assert!(release.contains("cancel-in-progress: false"));
    // The gate job and every gated job.
    for job in [
        "build-linux-x86_64",
        "build-linux-aarch64",
        "build-macos-x86_64",
        "build-macos-aarch64",
        "build-windows-x86_64",
    ] {
        assert!(
            release.contains(&format!("{job}:\n    needs: publish-gate")),
            "job {job} must gate on publish-gate"
        );
    }
    assert!(release.contains("      - publish-gate"), "the release job gates too");
    assert!(release.contains("sh npm/scripts/gate-tag.sh"));
    assert!(release.contains("sh npm/scripts/gate-registry.sh"));
    // Trusted publishing: OIDC, no token, npm floor, provenance assert.
    assert!(release.contains("id-token: write"));
    // A YAML-wired token is forbidden; the in-job guard reads the variable
    // with the `:-` default form, so only the colon+space key form is a wire.
    assert!(
        !release.contains("NODE_AUTH_TOKEN: "),
        "the workflow must never wire a token"
    );
    assert!(release.contains("NODE_AUTH_TOKEN:-"));
    assert!(release.contains("11.5.1"), "the npm floor is asserted in-job");
    assert!(release.contains("npm publish --provenance"));
    assert!(release.contains("assert-attestations.sh"));
    // Fail-closed uploads (finding M2): five build jobs, five error settings.
    assert_eq!(
        release.matches("if-no-files-found: error").count(),
        5,
        "every upload-artifact step fails closed on an empty asset dir"
    );
    // The scope arrives only from the repo variable (identity injection law).
    assert!(release.contains("vars.NPM_SCOPE"));
    assert!(release.contains("--repository \"${{ github.repository }}\""));
}

#[test]
fn tag_gate_and_sidecar_formats_are_the_portable_shapes() {
    let gate = load("npm/scripts/gate-tag.sh");
    assert!(
        gate.contains("^v[0-9]+\\.[0-9]+\\.[0-9]+$"),
        "the publish-gate refuses suffix tags with the exact shape regex"
    );
    let release = load(".github/workflows/release.yml");
    assert!(
        release.contains("\"$hash  $zip`n\""),
        "the windows sidecar is written as hash␣␣filename LF — the portable `sha256sum -c` shape (finding M1)"
    );
    assert!(
        !release.contains("| Set-Content"),
        "the legacy windows sidecar writer must not survive"
    );
}

#[test]
fn npm_tree_hosts_stay_on_the_public_allow_list() {
    // Payload-neutrality of the npm tree, structural form: every URL host the
    // tree names must be public infrastructure (github.com — the release the
    // package installs from) or a reserved/synthetic test sentinel. The exact
    // grep against the publish guard's identifier list runs outside the
    // payload, in the project's internal test suite — a
    // payload that names the list leaks it.
    let allowed = ["github.com", "example.invalid", "release.example"];
    let root = format!("{}/npm", manifest());
    let mut files = Vec::new();
    collect(&root, &mut files);
    assert!(files.len() >= 10, "the npm tree must carry the whole package: {files:#?}");
    for file in files {
        let bytes = fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {file}: {e}"));
        let mut rest = bytes.as_str();
        while let Some(at) = rest.find("://") {
            let tail = &rest[at + 3..];
            let host: String = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                .collect();
            if host.is_empty() {
                rest = tail;
                continue; // a bare "://" literal (a scanner's own needle), not a URL
            }
            assert!(
                allowed.contains(&host.as_str()),
                "host {host:?} in {file} is outside the public allow-list {allowed:?} — move such content out of the payload"
            );
            rest = tail;
        }
    }
}

fn collect(dir: &str, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read dir {dir}: {e}")) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            if path.file_name().map(|n| n == "node_modules").unwrap_or(false) {
                continue;
            }
            collect(path.to_str().expect("utf-8 path"), out);
        } else if path.file_name().map(|n| n == "LICENSE").unwrap_or(false) {
            continue; // the license text is the crate's, not npm-tree prose
        } else {
            out.push(path.to_str().expect("utf-8 path").to_string());
        }
    }
}

#[test]
fn package_manifest_neutrality_and_files_list() {
    let pkg = load("npm/package.json");
    let parsed: serde_json::Value = serde_json::from_str(&pkg).expect("npm/package.json is valid JSON");
    // Neutral sentinels: valid enough to pack, free of identity (finding B2).
    assert_eq!(parsed["name"], "@scope/archspec");
    assert_eq!(parsed["version"], "0.0.0");
    assert_eq!(parsed["publishConfig"]["access"], "public"); // finding m5
    assert!(parsed["files"]
        .as_array()
        .expect("files list")
        .iter()
        .any(|f| f == "assets.json"));
    assert!(parsed["files"]
        .as_array()
        .expect("files list")
        .iter()
        .all(|f| f != "test/" && !f.as_str().unwrap_or("").starts_with("test/")), "tests never pack");
    assert_eq!(parsed["bin"]["archspec"], "bin/archspec.mjs");
    let url = parsed["repository"]["url"].as_str().expect("repository.url");
    assert!(url.starts_with("https://") && url.contains("invalid"), "the committed manifest carries only a neutral placeholder URL: {url}");
    assert!(!Path::new(&format!("{}/npm/assets.json", manifest())).exists(), "assets.json is generated at publish time, never committed");
}
