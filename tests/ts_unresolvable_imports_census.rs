//! Visibility phase 2, scenario 01 — ts unresolvable imports: the model
//! always lands, one census line names every drop, and mechanical
//! evidence (R7b-ext identity, NAPI-ext loader probes) routes misses to
//! the shipped report-only arms instead of the census. Cells C1–C7 of
//! `rust-arch-test-kit/worklog/2026-11-20-scan-robustness-decision-check.md`
//! §8 plus the python twin parity cell.
mod common;

use common::Fixture;
use serde_json::Value;

fn tree_with(files: &[(&str, &str)]) -> Fixture {
    let tree = Fixture::new();
    tree.write("package.json", "{\n  \"name\": \"demo-ui\"\n}\n");
    for (path, content) in files {
        tree.write(path, content);
    }
    tree
}

fn model(out: &std::process::Output) -> Value {
    serde_json::from_str(&common::stdout(out)).expect("scan stdout must be the model JSON")
}

fn both(out: &std::process::Output) -> String {
    format!("{}{}", common::stdout(out), common::stderr(out))
}

/// C1: a wildcard-alias dead-end no longer destroys the run. The model
/// lands (unit + module_edges survive), every anchored miss line prints
/// byte-identically, exactly one census line aggregates them, and the
/// sort is count-desc then specifier-asc (determinism, MINOR-1).
#[test]
fn c1_dead_alias_lands_model_and_one_census_line() {
    let tree = tree_with(&[
        ("tsconfig.json", "{\"compilerOptions\":{\"paths\":{\"@app/*\":[\"./src/app/*\"]}}}"),
        (
            "src/a.ts",
            "import { b } from \"./b\";\nimport { gone } from \"@app/miss\";\nexport const a = b;\n",
        ),
        (
            "src/b.ts",
            "import { gone } from \"@app/miss\";\nexport const b = 1;\n",
        ),
        (
            "src/c.ts",
            "import { gone } from \"@app/zother\";\nexport const c = 1;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a non-empty census means exit 1: {stderr}"
    );
    let model = model(&scan);
    assert!(
        model["units"].as_array().is_some_and(|u| !u.is_empty()),
        "the model lands with its units: {model}"
    );
    assert!(
        model["module_edges"]
            .as_array()
            .is_some_and(|e| !e.is_empty()),
        "the live edge (src/a.ts -> src/b.ts) survives a dead neighbour: {model}"
    );
    for expected in [
        "unresolvable import: src/a.ts -> \"@app/miss\" — alias target not found:",
        "unresolvable import: src/b.ts -> \"@app/miss\" — alias target not found:",
        "unresolvable import: src/c.ts -> \"@app/zother\" — alias target not found:",
    ] {
        assert!(stderr.contains(expected), "missing anchored line {expected}:\n{stderr}");
    }
    let census = "unresolvable imports census: 2 specifiers across 3 files — top: @app/miss ×2, @app/zother ×1";
    assert!(stderr.contains(census), "missing census line:\n{stderr}");
    assert_eq!(
        stderr.matches("unresolvable imports census:").count(),
        1,
        "exactly ONE census line:\n{stderr}"
    );

    let again = tree.run(&["scan", "."]);
    assert_eq!(
        common::stderr(&again),
        stderr,
        "the census and lines are deterministic across runs"
    );
}

/// C2: the zero case is byte-identical to the shipped law. A tree whose
/// imports all resolve prints an empty census — zero bytes on every
/// report surface, exit 0, and the machine body carries no census field
/// (the transient field is serde-skipped, so no serialized byte moves —
/// the shipped-bytes guard is structural; this cell pins the observable
/// half).
#[test]
fn c2_zero_census_is_byte_identical_and_green() {
    let tree = tree_with(&[
        ("src/a.ts", "import { b } from \"./b\";\nexport const a = b;\n"),
        ("src/b.ts", "export const b = 1;\n"),
    ]);
    let scan = tree.run(&["scan", "."]);
    assert_eq!(scan.status.code(), Some(0), "{}", common::stderr(&scan));
    let stderr = common::stderr(&scan);
    assert!(!stderr.contains("census"), "an empty census prints nothing:\n{stderr}");
    assert!(!stderr.contains("unresolvable import"), "{stderr}");
    let text = common::stdout(&scan);
    assert!(
        !text.contains("unresolvable"),
        "the machine body carries no census bytes: {text}"
    );
    let parsed = model(&scan);
    assert_eq!(parsed["schema_version"], 1);
}

/// C3: a non-empty census forces exit 1 even when `--check` compares
/// FRESH bytes — the green check must not hide dead edges.
#[test]
fn c3_check_fresh_but_census_nonempty_stays_red() {
    // The live edge keeps the silence tripwire quiet, so the model lands:
    // this cell pins the census exit arm, not the tripwire arm (C7's).
    let tree = tree_with(&[
        ("src/a.ts", "import { b } from \"./b\";\nimport { gone } from \"./missed\";\nexport const a = gone + b;\n"),
        ("src/b.ts", "export const b = 1;\n"),
    ]);
    let first = tree.run(&["scan", ".", "--output", "model.json"]);
    assert_eq!(first.status.code(), Some(1), "{}", common::stderr(&first));
    assert!(tree.path("model.json").is_file(), "the model still lands");

    let check = tree.run(&["scan", ".", "--output", "model.json", "--check"]);
    let stdout = common::stdout(&check);
    assert!(
        stdout.contains("up to date"),
        "the bytes ARE fresh — the check says so:\n{stdout}"
    );
    assert_eq!(
        check.status.code(),
        Some(1),
        "a non-empty census forces exit 1 even when fresh:\n{stdout}{}",
        common::stderr(&check)
    );
    assert!(
        common::stderr(&check).contains("unresolvable imports census:"),
        "{}",
        common::stderr(&check)
    );
}

/// C4: `verify` no longer aborts on a dead edge. The findings fold into
/// the warnings channel: green-with-warnings by default, exit 1 under
/// `--strict`, and the census names itself once. An empty census leaves
/// the verdict bytes unchanged.
#[test]
fn c4_verify_folds_census_into_warnings() {
    let spec = "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n";
    let tree = tree_with(&[
        ("architecture.spec.toml", spec),
        ("tsconfig.json", "{\"compilerOptions\":{\"paths\":{\"@app/*\":[\"./src/app/*\"]}}}"),
        (
            "src/a.ts",
            "import { b } from \"./b\";\nimport { gone } from \"@app/miss\";\nexport const a = b;\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
    ]);

    let verify = tree.run(&["verify"]);
    let out = both(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "census findings are warnings, not errors: {out}"
    );
    assert!(
        out.contains("warning: unresolvable import: src/a.ts -> \"@app/miss\""),
        "the anchored line rides the warnings channel:\n{out}"
    );
    assert!(
        out.contains("unresolvable imports census: 1 specifiers across 1 files"),
        "the census names itself once:\n{out}"
    );

    let strict = tree.run(&["verify", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(1),
        "`--strict` promotes the census: {}",
        both(&strict)
    );

    let clean = tree_with(&[
        ("architecture.spec.toml", spec),
        ("src/a.ts", "import { b } from \"./b\";\nexport const a = b;\n"),
        ("src/b.ts", "export const b = 1;\n"),
    ]);
    let clean_verify = clean.run(&["verify"]);
    let clean_out = both(&clean_verify);
    assert_eq!(clean_verify.status.code(), Some(0), "{clean_out}");
    assert!(
        !clean_out.contains("unresolvable imports census"),
        "an empty census leaves the verdict unchanged:\n{clean_out}"
    );
}

/// C5 (M2, NAPI-ext): a platform-addon probe a NAPI-RS loader requires
/// is the GENERATED class — one report-only line per probe, exit 0, no
/// census entry. Both spellings (bare and `/package.json` twin) ride the
/// arm; the evidence is the loader marker plus the package-name-plus-
/// triple shape, no vocabulary.
#[test]
fn c5_napi_loader_platform_probes_are_report_only() {
    let tree = tree_with(&[
        ("tsconfig.json", "{\"compilerOptions\":{\"paths\":{\"@acme/*\":[\"./ext/*\"]}}}"),
        ("pkg/package.json", "{\n  \"name\": \"@acme/native\"\n}\n"),
        (
            "pkg/index.js",
            "// auto-generated by NAPI-RS\n\
             const a = require(\"@acme/native-linux-x64-gnu\");\n\
             const b = require(\"@acme/native-darwin-arm64/package.json\");\n\
             module.exports = { a, b };\n",
        ),
    ]);
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(scan.status.code(), Some(0), "{stderr}\n{}", common::stdout(&scan));
    assert!(
        stderr.contains(
            "generated source not built: @acme/native-linux-x64-gnu — NAPI-RS loader pkg/index.js:1"
        ),
        "the bare probe rides the generated class:\n{stderr}"
    );
    assert!(
        stderr.contains(
            "generated source not built: @acme/native-darwin-arm64/package.json — NAPI-RS loader pkg/index.js:1"
        ),
        "the twin spelling rides the same arm:\n{stderr}"
    );
    // The ORPHAN census legitimately fires (the loader file states no
    // edge) — what must be absent is the UNRESOLVABLE census and lines.
    assert!(
        !stderr.contains("unresolvable imports census"),
        "no census entry for routed misses:\n{stderr}"
    );
    assert!(!stderr.contains("unresolvable import"), "{stderr}");
}

/// C6 (M1, R7b-ext): an alias dead-end whose identity the ROOT manifest
/// declares, or the ROOT lockfile names, lands the external fact with
/// the install line — past the first-manifest stop, nearest-manifest
/// priority intact. An identity nowhere declared keeps the census.
#[test]
fn c6_root_manifest_or_lockfile_identity_is_external() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"root\",\n  \"dependencies\": { \"@acme/lib\": \"1.0.0\" }\n}\n",
    );
    tree.write(
        "pnpm-lock.yaml",
        "lockfileVersion: '9.0'\npackages:\n  '@acme/tool@2.0.0':\n    resolution: {integrity: sha512-x}\n",
    );
    tree.write(
        "tsconfig.json",
        "{\"compilerOptions\":{\"paths\":{\"@acme/*\":[\"./ext/*\"]}}}",
    );
    tree.write("packages/widget/package.json", "{\n  \"name\": \"widget\"\n}\n");
    tree.write(
        "packages/widget/src/a.ts",
        "import { thing } from \"@acme/lib/deep\";\nexport const a = thing;\n",
    );
    tree.write(
        "packages/widget/src/b.ts",
        "import { tool } from \"@acme/tool\";\nexport const b = tool;\n",
    );
    tree.write(
        "packages/widget/src/c.ts",
        "import { ghost } from \"@acme/ghost\";\nexport const c = ghost;\n",
    );

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    let model = model(&scan);
    assert_eq!(scan.status.code(), Some(1), "the ghost keeps the census red: {stderr}");
    assert!(
        stderr.contains("external not installed: @acme/lib — declared in package.json, absent from disk"),
        "the root manifest answers the dead-end (walk past the widget manifest):\n{stderr}"
    );
    assert!(
        stderr.contains("external not installed: @acme/tool — named in pnpm-lock.yaml, absent from disk"),
        "the root lockfile NAMES the identity:\n{stderr}"
    );
    let external: Vec<&str> = model["external"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    assert!(external.contains(&"@acme/lib"), "external fact for {model}");
    assert!(external.contains(&"@acme/tool"), "external fact for {model}");
    let census = "unresolvable imports census: 1 specifiers across 1 files — top: @acme/ghost ×1";
    assert!(stderr.contains(census), "only the evidence-less miss counts:\n{stderr}");
    assert!(
        stderr.contains("unresolvable import: packages/widget/src/c.ts -> \"@acme/ghost\""),
        "{stderr}"
    );
}

/// C7 (MINOR-2, B1 ordering): the census launders nothing green. A tree
/// whose imports resolve to NOTHING states no fact, so the silence
/// tripwire still reddens the run — the census is computed strictly
/// after the tripwire and is never itself an arm.
#[test]
fn c7_findings_only_extractor_is_still_red() {
    let tree = tree_with(&[(
        "src/a.ts",
        "import { gone } from \"./missed\";\nexport const a = gone;\n",
    )]);
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(scan.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("tripwire"),
        "the silence tripwire is the red, not the census:\n{stderr}"
    );
    assert!(
        !stderr.contains("unresolvable imports census"),
        "a run with no model states no census:\n{stderr}"
    );
    assert!(common::stdout(&scan).is_empty(), "no model bytes on the tripwire red");
}

/// The python twin (parity cell): the relative miss class no longer
/// aborts there either — the model lands and the census names the drop.
#[test]
fn python_twin_lands_model_with_census() {
    let tree = Fixture::new();
    tree.write("pkg/__init__.py", "");
    tree.write(
        "pkg/mod.py",
        "import os\nfrom .... import beyond\nvalue = (os, beyond)\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a non-empty census means exit 1 on the twin too: {stderr}"
    );
    let model = model(&scan);
    assert!(
        model["units"].as_array().is_some_and(|u| !u.is_empty()),
        "the twin model lands: {model}"
    );
    assert!(
        stderr.contains("unresolvable import: pkg/mod.py"),
        "the anchored line prints on the twin too:\n{stderr}"
    );
    assert!(
        stderr.contains("relative depth beyond package root"),
        "{stderr}"
    );
    assert!(
        stderr.contains("unresolvable imports census: 1 specifiers across 1 files"),
        "{stderr}"
    );
    assert_eq!(
        stderr.matches("unresolvable imports census:").count(),
        1,
        "exactly ONE census line:\n{stderr}"
    );
}
