//! C5 cells of the ts-driver design record — the extraction arm.
//!
//! Edges come from import, export-from, dynamic-literal and asset specifiers;
//! relative specifiers resolve through extension-and-index inference; bare
//! specifiers land in the external tier; an unresolvable specifier fails loud
//! (D2); a non-literal dynamic import is a named wildcard fact, report-only
//! (D2/R2-6); edges carry their `(from, to, kind)` row split with
//! `type_only` as the ts-only fact key whose absence keeps the rust, go and
//! csharp model bytes unchanged (D3/R3-9). Module identity is the file path
//! exactly as git spells it, extension included (D4); the package root is
//! the unit (D9). `.svelte` files are first-class units through their
//! `<script>` blocks (D1).

mod common;

use common::Fixture;
use serde_json::Value;

fn ts_tree(files: &[(&str, &str)]) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": { \"lodash\": \"^4.17.21\" }\n}\n",
    );
    for (path, content) in files {
        tree.write(path, content);
    }
    tree
}

fn model(out: &std::process::Output) -> Value {
    serde_json::from_str(&common::stdout(out)).expect("scan stdout must be the model JSON")
}

fn module_edge_rows(model: &Value) -> Vec<(String, String, bool)> {
    let mut rows: Vec<(String, String, bool)> = model["module_edges"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            (
                row["from"].as_str().unwrap_or_default().to_string(),
                row["to"].as_str().unwrap_or_default().to_string(),
                row["type_only"].as_bool().unwrap_or(false),
            )
        })
        .collect();
    rows.sort();
    rows
}

fn declarations(model: &Value) -> Vec<String> {
    let mut names: Vec<String> = model["root_module_declarations"]
        .as_object()
        .map(|units| {
            units
                .values()
                .flat_map(|rows| rows.as_array().cloned().unwrap_or_default())
                .map(|row| row["name"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Scenario R8 (C2 flip, vacuity variant): a scanned tree whose packages
/// state no manifest at all must not scan to a silent green — the named
/// carrier line states the empty evidence on the report surface, never a
/// finding, never red (the shipped carrier's scan-side instantiation).
#[test]
fn unit_manifests_carrier_names_trees_without_manifests() {
    let tree = Fixture::new();
    tree.write("a.ts", "export const a = 1;\n");

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan)
            .contains("vacuous constraint: scan.unit_manifests: no package manifest states dependencies"),
        "the empty-evidence scan must print the named carrier:\n{}",
        common::stderr(&scan)
    );
}

/// Scenario R8 (C5 flip, vacuity variant): a file-level render over a tree
/// with no resolved edges prints the named carrier instead of a mermaid
/// green that claims edges it does not have.
#[test]
fn file_level_inspect_carrier_names_edgeless_trees() {
    let tree = ts_tree(&[
        ("src/a.ts", "export const a = 1;\n"),
        ("src/b.ts", "export const b = 2;\n"),
    ]);

    let inspect = tree.run(&["inspect"]);
    assert!(
        inspect.status.success(),
        "inspect failed: {}",
        common::stderr(&inspect)
    );
    assert!(
        common::stderr(&inspect)
            .contains("vacuous constraint: inspect.file_level: no file-level edges resolved"),
        "the empty-evidence inspect must print the named carrier:\n{}",
        common::stderr(&inspect)
    );
}

/// Scenario R8 (C1 flip, vacuity variant): a declared `forbidden` ban over a
/// tree that resolves no module edges guards nothing — the named vacuity
/// carrier states it instead of silence, and the verdict stays green
/// (vacuous never gates without `--strict`).
#[test]
fn forbidden_laundering_carrier_names_edgeless_bans() {
    let tree = ts_tree(&[
        ("src/a.ts", "export const a = 1;\n"),
        ("src/b.ts", "export const b = 2;\n"),
    ]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { modules = [\"src/a.ts\"] }\n\n[module.allowed]\nforbidden = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"src/b.ts\"] }\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "a vacuous run stays green: {}\n{}",
        common::stdout(&verify),
        common::stderr(&verify)
    );
    let stdout = common::stdout(&verify);
    let stderr = common::stderr(&verify);
    assert!(
        stdout.contains("verify.forbidden_laundering: no module edges resolved")
            || stderr.contains("verify.forbidden_laundering: no module edges resolved"),
        "the empty-evidence ban must print the named carrier:\n{stdout}\n{stderr}"
    );
}

/// Scenario R8 (C4 flip, vacuity variant): the roles map derives a facade
/// root, yet no edge enters it — the named carrier states the rule had no
/// domain to adjudicate, instead of a green that hides the declared root.
#[test]
fn root_facade_carrier_names_root_without_consumers() {
    let tree = ts_tree(&[
        ("index.ts", "export { secret } from \"./src/internal\";\n"),
        ("src/internal.ts", "export const secret = 1;\n"),
    ]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"index.ts\", \"src/internal.ts\"] }\n",
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "a vacuous run stays green: {}\n{}",
        common::stdout(&verify),
        common::stderr(&verify)
    );
    let stdout = common::stdout(&verify);
    let stderr = common::stderr(&verify);
    assert!(
        stdout.contains("verify.root_facade: no edge enters a facade root")
            || stderr.contains("verify.root_facade: no edge enters a facade root"),
        "the consumerless root must print the named carrier:\n{stdout}\n{stderr}"
    );
}

/// Scenario R8 (C3 flip, vacuity variant): a `forbid_submodule_dependency`
/// constraint over a tree with no intra-parent submodule edges surfaces on
/// the shipped constraint-channel vacuity carrier, never silence.
#[test]
fn submodule_dependency_carrier_names_edgeless_parents() {
    let tree = ts_tree(&[("src/a.ts", "export const a = 1;\n")]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/a.ts\"] }\n\n[[constraint]]\ntype = \"forbid_submodule_dependency\"\nparent = \"app\"\nfrom = [\"app::a\"]\nforbid = [\"app::b\"]\n",
    );

    let verify = tree.run(&["verify", "."]);
    let stdout = common::stdout(&verify);
    let stderr = common::stderr(&verify);
    assert!(
        stdout.contains("forbid_submodule_dependency: no intra-parent submodule edges to check")
            || stderr.contains("forbid_submodule_dependency: no intra-parent submodule edges to check"),
        "the edgeless parent must print the named carrier:\n{stdout}\n{stderr}"
    );
}

/// Scenario "a ts tree gets a driver": file-grained facts for every
/// resolvable specifier, the package root as the unit, and the mechanical
/// exclusion set contributing no files and no edges.
#[test]
fn ts_tree_scans_with_file_grained_facts() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { b } from \"./b.js\";\nimport { c } from \"./c\";\nimport { d } from \"./d.jsx\";\nexport const a = b + c + d;\n",
        ),
        ("src/b.js", "export const b = 1;\n"),
        ("src/c.tsx", "export const c = 2;\n"),
        ("src/d.jsx", "export const d = 3;\n"),
        // Excluded trees contribute no files and no edges (D1/D2).
        (
            "node_modules/pkg/index.ts",
            "import { a } from \"../../src/a\";\nexport const n = a;\n",
        ),
        (
            "dist/built.js",
            "import { a } from \"../src/a\";\nexport const n = a;\n",
        ),
        (
            ".svelte-kit/generated.js",
            "import { a } from \"../src/a\";\nexport const n = a;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert_eq!(model["language"], "ts");
    assert_eq!(model["units"][0]["kind"], "package");
    assert_eq!(model["units"][0]["name"], "demo-ui");
    assert_eq!(model["units"][0]["path"], ".");
    assert_eq!(
        module_edge_rows(&model),
        vec![
            ("src/a.ts".to_string(), "src/b.js".to_string(), false),
            ("src/a.ts".to_string(), "src/c.tsx".to_string(), false),
            ("src/a.ts".to_string(), "src/d.jsx".to_string(), false),
        ],
        "the edge set is exactly the resolvable first-party specifiers"
    );
    let files = declarations(&model);
    assert_eq!(
        files,
        vec!["src/a.ts", "src/b.js", "src/c.tsx", "src/d.jsx"],
        "every scanned file is a vertex; no excluded tree contributes one"
    );
}

/// Scenario "Svelte components are first-class units": every `<script>`
/// block — plain and `context="module"` — is parsed with the TypeScript
/// grammar; a script-less component is a vertex with no imports.
#[test]
fn svelte_script_blocks_are_first_class_units() {
    let tree = ts_tree(&[
        (
            "src/App.svelte",
            "<script context=\"module\">\nimport { boot } from \"./boot\";\n</script>\n<script>\nimport { store } from \"./store\";\n</script>\n<button onclick={() => boot(store)}>{ $value }</button>\n",
        ),
        ("src/boot.ts", "export const boot = (s: unknown) => s;\n"),
        ("src/store.ts", "export const store = { value: 1 };\n"),
        ("src/Static.svelte", "<h1>markup only</h1>\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert_eq!(
        module_edge_rows(&model),
        vec![
            ("src/App.svelte".to_string(), "src/boot.ts".to_string(), false),
            (
                "src/App.svelte".to_string(),
                "src/store.ts".to_string(),
                false
            ),
        ],
        "both script blocks of the component carry their imports"
    );
    let files = declarations(&model);
    assert!(
        files.contains(&"src/App.svelte".to_string()),
        "the component is a vertex"
    );
    assert!(
        files.contains(&"src/Static.svelte".to_string()),
        "a component with no script block contributes a vertex with no imports"
    );
}

/// Scenario "`import type` carries its kind" (the fact-site half of D3):
/// a pair carrying both a value import and a type import serializes TWO
/// `(from, to, kind)` rows in deterministic key order.
#[test]
fn mixed_pair_serializes_two_rows_in_key_order() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { value } from \"./b\";\nimport type { Kind } from \"./b\";\nexport const a = value as Kind;\n",
        ),
        ("src/b.ts", "export const value = 1;\nexport type Kind = number;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![
            ("src/a.ts".to_string(), "src/b.ts".to_string(), false),
            ("src/a.ts".to_string(), "src/b.ts".to_string(), true),
        ],
        "one value row and one type_only row, value row first (deterministic key order)"
    );
}

/// The `type_only` fact key is ts-only: rust and go models carry no
/// `type_only` key and no `dynamic_import_wildcards` key at all — their
/// serialized bytes are unchanged by the ts fact sites (D3/R3-9).
#[test]
fn rust_and_go_models_carry_no_ts_fact_keys() {
    let rust = Fixture::new();
    rust.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    rust.write("src/lib.rs", "pub fn f() {}\n");
    let rust_scan = rust.run(&["scan", "."]);
    assert!(
        rust_scan.status.success(),
        "rust scan failed: {}",
        common::stderr(&rust_scan)
    );
    let rust_json = common::stdout(&rust_scan);
    assert!(
        !rust_json.contains("type_only") && !rust_json.contains("dynamic_import_wildcards"),
        "the rust model bytes carry no ts fact keys"
    );

    let go = Fixture::new();
    go.write("go.mod", "module example.com/tool\n\ngo 1.22\n");
    go.write(
        "cmd/tool/main.go",
        "package main\n\nimport \"example.com/tool/internal/core\"\n\nfunc main() { core.Run() }\n",
    );
    go.write(
        "internal/core/core.go",
        "package core\n\nfunc Run() {}\n",
    );
    let go_scan = go.run(&["scan", "."]);
    assert!(
        go_scan.status.success(),
        "go scan failed: {}",
        common::stderr(&go_scan)
    );
    let go_json = common::stdout(&go_scan);
    assert!(
        !go_json.contains("type_only") && !go_json.contains("dynamic_import_wildcards"),
        "the go model bytes carry no ts fact keys"
    );
}

/// Scenario "a dynamic import with a non-literal specifier is stated, not
/// dropped": the literal specifier is an ordinary edge, the computed
/// specifier is a named wildcard fact with its report line on both verbs.
#[test]
fn dynamic_import_literal_edges_and_computed_wildcards() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "export async function load(name: string) {\n  const mod = await import(`./dyn/${name}`);\n  const lazy = await import(\"./lazy\");\n  return { mod, lazy };\n}\n",
        ),
        (
            "src/lazy.ts",
            "export const lazy = true;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert_eq!(
        module_edge_rows(&model),
        vec![("src/a.ts".to_string(), "src/lazy.ts".to_string(), false)],
        "the literal dynamic import resolves by the ordinary resolution law"
    );
    assert_eq!(
        model["dynamic_import_wildcards"]["src/a.ts"],
        serde_json::json!(["`./dyn/${name}`"]),
        "the computed specifier is a named wildcard fact attributed to the file"
    );
    let line = "dynamic import wildcard: src/a.ts -> `./dyn/${name}`";
    let stderr = common::stderr(&scan);
    assert!(
        stderr.lines().any(|l| l == line),
        "the scan report prints the wildcard line, exact bytes:\n{stderr}"
    );

    // verify prints the same report-only line (D13: the scan report owner
    // covers both verbs).
    let verify = tree.run(&["verify", "."]);
    // No spec at the root: verify refuses operationally — write one first.
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
    );
    let _ = verify;
    let verify = tree.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "verify failed: {}",
        common::stderr(&verify)
    );
    let stderr = common::stderr(&verify);
    assert!(
        stderr.lines().any(|l| l == line),
        "the verify run prints the wildcard line, exact bytes:\n{stderr}"
    );
}

/// Scenario "a tree whose import statements are ALL non-literal dynamic
/// imports stays green" (R2-6): the tripwire counts wildcard facts beside
/// edges, so an honest wildcard-only parser never dies on its own tripwire.
#[test]
fn wildcard_only_tree_stays_green() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "export async function load(name: string) {\n  return await import(`./dyn/${name}`);\n}\n",
    )]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
    );

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "the wildcard-only scan must stay green: {}",
        common::stderr(&scan)
    );
    let verify = tree.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "the wildcard-only verify must stay green: {}",
        common::stderr(&verify)
    );
    let model = model(&scan);
    assert!(model["module_edges"].as_array().unwrap().is_empty());
    assert!(model["dynamic_import_wildcards"]["src/a.ts"].is_array());
}

/// Scenario "an asset import is a counted edge" (the fact half — the
/// cycle-graph half rides the file tier of a later phase): `.css` and
/// friends ride as ordinary edges to the asset file.
#[test]
fn asset_import_rides_as_a_counted_edge() {
    let tree = ts_tree(&[
        (
            "src/main.ts",
            "import \"./styles/site.css\";\nimport { mount } from \"./mount\";\nmount();\n",
        ),
        ("src/mount.ts", "export const mount = () => {};\n"),
        (
            "src/styles/site.css",
            "button { color: red; }\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![
            (
                "src/main.ts".to_string(),
                "src/mount.ts".to_string(),
                false
            ),
            (
                "src/main.ts".to_string(),
                "src/styles/site.css".to_string(),
                false
            ),
        ],
        "the asset import is an ordinary edge to the asset file"
    );
}

/// Scenario "bare specifiers land in the external tier": npm package
/// identities — scoped names keep their scope, subpaths fold to the package,
/// `node:` builtins are ordinary externals — one external vocabulary.
#[test]
fn bare_specifiers_land_in_the_external_tier() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "import { cloneDeep } from \"lodash\";\nimport { helper } from \"@org/pkg/sub/module\";\nimport * as fs from \"node:fs\";\nexport const run = () => [cloneDeep, helper, fs];\n",
    )]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    for package in ["lodash", "@org/pkg", "node:fs"] {
        assert!(
            model["external"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(package)),
            "external tier must name {package}: {model}"
        );
    }
    assert!(
        module_edge_rows(&model).is_empty(),
        "a bare specifier resolves to the external tier, never an edge"
    );
}

/// Scenario "bare specifiers land in the external tier", the adjudication
/// half: the shipped `forbid_external_crates` arm reddens npm packages.
#[test]
fn forbid_external_crates_adjudicates_npm_packages() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "import { cloneDeep } from \"lodash\";\nexport const run = () => cloneDeep({});\n",
    )]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n\n[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"demo-ui\"]\nforbid = [\"lodash\"]\n",
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "verify must redden the forbidden npm package: {}",
        common::stderr(&verify)
    );
    let stderr = common::stderr(&verify);
    let stdout = common::stdout(&verify);
    assert!(
        (stderr.contains("forbidden external crate") && stderr.contains("lodash"))
            || (stdout.contains("forbidden external crate") && stdout.contains("lodash")),
        "the shipped finding names the package (verdict body is stdout):\n{stdout}\n{stderr}"
    );
}

/// Scenario "an unresolvable specifier fails loud": a typo or a deleted
/// file names the importing file and the specifier verbatim — on both
/// verbs, because the model can never state a dropped edge.
#[test]
fn unresolvable_specifier_fails_loud_on_both_verbs() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "import { gone } from \"./missed\";\nexport const a = gone;\n",
    )]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
    );
    let expected = "unresolvable import: src/a.ts -> \"./missed\"";

    for verb in ["scan", "verify"] {
        let run = tree.run(&[verb, "."]);
        assert_eq!(
            run.status.code(),
            Some(1),
            "`{verb}` on an unresolvable specifier must exit 1"
        );
        assert!(
            common::stderr(&run).contains(expected),
            "`{verb}` must name the file and the specifier verbatim:\n{}",
            common::stderr(&run)
        );
    }
}

/// Scenario "an unresolvable specifier fails loud", the excluded-path half
/// (R2-4, R3-1): a specifier resolving only into an excluded path is an
/// `unresolvable import` naming the exclusion it hit — never a wildcard,
/// never a silent drop.
#[test]
fn unresolvable_into_excluded_path_names_the_exclusion() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { legacy } from \"./vendor/legacy\";\nexport const a = legacy;\n",
        ),
        ("src/vendor/legacy.ts", "export const legacy = 1;\n"),
    ]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\nexcluded_sources = [\n  { path = \"src/vendor\", reason = \"vendored legacy code\" },\n]\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
    );

    for verb in ["scan", "verify"] {
        let run = tree.run(&[verb, "."]);
        assert_eq!(
            run.status.code(),
            Some(1),
            "`{verb}` must exit 1 on a specifier resolving only into excluded paths"
        );
        let stderr = common::stderr(&run);
        assert!(
            stderr.contains("unresolvable import: src/a.ts -> \"./vendor/legacy\""),
            "`{verb}` must name the file and specifier:\n{stderr}"
        );
        assert!(
            stderr.contains("excluded path src/vendor"),
            "`{verb}` must name the exclusion it hit:\n{stderr}"
        );
    }
}

/// Scenario "an unresolvable specifier fails loud", the falsifier half
/// (R3-1): a bare specifier into a present `node_modules` stays an ordinary
/// external fact — the excluded-path class never fires on bare specifiers.
#[test]
fn bare_specifier_with_node_modules_present_stays_external() {
    let tree = ts_tree(&[
        ("src/a.ts", "import _ from \"lodash\";\nexport const a = _;\n"),
        (
            "node_modules/lodash/index.js",
            "module.exports = { cloneDeep: x => x };\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a bare specifier must stay green (external), never unresolvable: {}",
        common::stderr(&scan)
    );
    assert!(model(&scan)["external"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("lodash")));
}

/// Scenario "a ts tree gets a driver", the resolution-law half: extension
/// inference (`./lib` → `lib.ts`) and index inference (`./comp` →
/// `comp/index.*`).
#[test]
fn extension_and_index_inference_resolve_relative_specifiers() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { lib } from \"./lib\";\nimport { comp } from \"./comp\";\nexport const a = [lib, comp];\n",
        ),
        ("src/lib.ts", "export const lib = 1;\n"),
        (
            "src/comp/index.ts",
            "export const comp = 2;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![
            ("src/a.ts".to_string(), "src/comp/index.ts".to_string(), false),
            ("src/a.ts".to_string(), "src/lib.ts".to_string(), false),
        ]
    );
}

/// Scenario "a ts tree gets a driver", the unit half (D9): with no
/// `package.json` the package root still names the unit — byte-stably, so
/// the D4 byte test survives every clone.
#[test]
fn tree_without_package_json_names_the_unit_by_its_root_path() {
    let tree = Fixture::new();
    tree.write("a.ts", "export const a = 1;\n");

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert_eq!(model["language"], "ts");
    assert_eq!(model["units"][0]["name"], ".");
    assert_eq!(model["units"][0]["kind"], "package");
    assert_eq!(model["units"][0]["path"], ".");
}

/// Scenario "a ts tree gets a driver" + D9: the export facts — named and
/// default exports become the file's public-API rows so `contract.expose`
/// engages honestly on ts (the D9 public-API law, the factual family of the
/// csharp module rows).
#[test]
fn exports_are_public_api_facts_attributed_to_the_file() {
    let tree = ts_tree(&[
        (
            "src/api.ts",
            "export function fetcher() {}\nexport const LIMIT = 10;\nclass Hidden {}\nexport default Hidden;\nexport { Hidden as visible };\n",
        ),
        (
            "src/reexport.ts",
            "export { fetcher } from \"./api\";\nexport * from \"./api\";\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    let rows = |file: &str| -> Vec<String> {
        model["module_public_types"][file]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    let api = rows("src/api.ts");
    for exported in ["fetcher", "LIMIT", "default", "visible"] {
        assert!(
            api.iter().any(|name| name == exported),
            "src/api.ts must state its public export {exported:?}: {api:?}"
        );
    }
    assert!(
        !api.iter().any(|name| name == "Hidden"),
        "a non-exported class is not public API"
    );
    let reexport = rows("src/reexport.ts");
    assert!(
        reexport.iter().any(|name| name == "fetcher"),
        "the export-from clause states the re-exported name: {reexport:?}"
    );
    assert_eq!(
        module_edge_rows(&model),
        vec![(
            "src/reexport.ts".to_string(),
            "src/api.ts".to_string(),
            false
        )],
        "export-from specifiers are edges (D2)"
    );
}

/// D4 byte test: the scan JSON unit/module identity spellings for a
/// three-file ts tree, pinned as exact bytes. The spelling is the record's:
/// the repo-relative path with extension, exactly as git spells it.
#[test]
fn scan_json_identity_spellings_are_pinned_for_a_three_file_tree() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { b } from \"./b.js\";\nexport const a = b;\n",
        ),
        ("src/b.js", "export const b = 1;\n"),
        ("src/C.svelte", "<script>\nimport { a } from \"./a\";\n</script>\n<p>{a}</p>\n"),
    ]);

    let scan = tree.run(&["scan", "--pretty", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let body = common::stdout(&scan);
    for pinned in [
        "\"language\": \"ts\",",
        "\"name\": \"demo-ui\",",
        "\"kind\": \"package\",",
        "\"path\": \".\"",
        "\"name\": \"src/a.ts\",",
        "\"name\": \"src/b.js\",",
        "\"name\": \"src/C.svelte\",",
        "\"from\": \"src/C.svelte\",",
        "\"to\": \"src/a.ts\",",
    ] {
        assert!(
            body.contains(pinned),
            "the scan JSON must carry the pinned identity spelling {pinned:?}:\n{body}"
        );
    }
}

/// The seam tests (ADR-024 resolution record R1 part B) below assert
/// absolute-path finding strings; the verbs canonicalize the scan root, so
/// the expected spellings ride the canonical fixture root.
fn canonical_root(tree: &Fixture) -> std::path::PathBuf {
    tree.root.canonicalize().expect("canonical fixture root")
}

/// True when the finding line rides a stderr line byte-verbatim: bare on the
/// report surface, or behind the `error: ` framing as the first finding of a
/// failing extract (the joined findings land verbatim on stderr).
fn stderr_carries_line(stderr: &str, line: &str) -> bool {
    stderr.lines().any(|l| l == line || l == format!("error: {line}"))
}

/// Scenario "resolution is per importing file, not per run" (R1 per-file
/// falsifier): the same alias key in the root config and in the package's
/// own config resolves each importing file through the config that OWNS it.
/// The two edge targets prove the owning config — an engine delegating every
/// file through the root config would emit the wrong target for the package
/// file (and vice versa), reddening the exact edge set asserted here.
#[test]
fn per_file_tsconfig_aliases_resolve_first_party() {
    let tree = ts_tree(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@alias/*":["./src/root-target/*"]}}}"#,
        ),
        (
            "packages/app/tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@alias/*":["./app-target/*"]}}}"#,
        ),
        (
            "src/a.ts",
            "import { r } from \"@alias/widget\";\nexport const a = r;\n",
        ),
        (
            "packages/app/a.ts",
            "import { l } from \"@alias/widget\";\nexport const m = l;\n",
        ),
        ("src/root-target/widget.ts", "export const r = 1;\n"),
        (
            "packages/app/app-target/widget.ts",
            "export const l = 2;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![
            (
                "packages/app/a.ts".to_string(),
                "packages/app/app-target/widget.ts".to_string(),
                false
            ),
            (
                "src/a.ts".to_string(),
                "src/root-target/widget.ts".to_string(),
                false
            ),
        ],
        "each file's alias resolves through its OWNING config, never the root's"
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("unresolvable import"),
        "both alias hits are first-party edges:\n{stderr}"
    );
}

/// Fixture BARE-MEMO-KEY (perf lane 2026-10-17, fix 4 cache trap): the
/// bare-resolution memo keys on (importing dir, spelled specifier). Two
/// files in ONE dir repeat the pair — the second call is a memo replay
/// and must still land its own edge; a file in ANOTHER dir spells the
/// same specifier — a key collapsed to the specifier alone would replay
/// the wrong config's verdict and redden the exact edge set asserted
/// here (the per-file law, same falsifier shape as R1).
#[test]
fn bare_resolution_memo_keys_on_dir_and_replays_repeats() {
    let tree = ts_tree(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@alias/*":["./src/root-target/*"]}}}"#,
        ),
        (
            "packages/app/tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@alias/*":["./app-target/*"]}}}"#,
        ),
        (
            "src/a.ts",
            "import { r } from \"@alias/widget\";\nexport const a = r;\n",
        ),
        (
            "src/b.ts",
            "import { r } from \"@alias/widget\";\nexport const b = r;\n",
        ),
        (
            "packages/app/a.ts",
            "import { l } from \"@alias/widget\";\nexport const m = l;\n",
        ),
        ("src/root-target/widget.ts", "export const r = 1;\n"),
        (
            "packages/app/app-target/widget.ts",
            "export const l = 2;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![
            (
                "packages/app/a.ts".to_string(),
                "packages/app/app-target/widget.ts".to_string(),
                false,
            ),
            (
                "src/a.ts".to_string(),
                "src/root-target/widget.ts".to_string(),
                false,
            ),
            (
                "src/b.ts".to_string(),
                "src/root-target/widget.ts".to_string(),
                false,
            ),
        ],
        "the repeat replays the verdict for the second edge and the other \
         dir keeps its own config's verdict"
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("unresolvable import"),
        "every alias hit is a first-party edge:\n{stderr}"
    );
}

/// Scenario "a dead alias reddens on both sides" (R1 class (a)): a wildcard
/// alias and an exact-key alias whose targets are gone each fail loud with
/// the anchor naming the importing file and specifier plus the
/// `alias target not found` wording — exit 1, never an external fact.
#[test]
fn dead_alias_reddens_both_sides() {
    let tree = ts_tree(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@app/*":["./src/app/*"],"@exact":["./src/exactthing"]}}}"#,
        ),
        (
            "src/a.ts",
            "import { gone } from \"@app/nope\";\nexport const a = gone;\n",
        ),
        (
            "src/b.ts",
            "import { kind } from \"@exact\";\nexport const b = kind;\n",
        ),
    ]);
    let root = canonical_root(&tree);

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a dead alias must exit 1: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains(&format!(
            "unresolvable import: src/a.ts -> \"@app/nope\" — alias target not found: {}",
            root.join("src/app/nope").display()
        )),
        "the wildcard miss must name the file, specifier and first alias candidate:\n{stderr}"
    );
    assert!(
        stderr.contains("unresolvable import: src/b.ts -> \"@exact\"")
            && stderr.contains("alias target not found"),
        "the exact-key miss must ride the same class in either spelling:\n{stderr}"
    );
    assert!(
        !stderr.contains("external not installed"),
        "a dead alias states no install fact:\n{stderr}"
    );
}

/// Scenario "a require-only export map still resolves" (R3 conditions):
/// `require` rides the active condition set, so a cjs-only package resolves
/// through its exports map to the external tier — green, never an
/// export-map miss.
#[test]
fn cjs_only_package_resolves() {
    let tree = ts_tree(&[
        ("src/a.ts", "import dep from \"cjsdep\";\nexport const a = dep;\n"),
        (
            "node_modules/cjsdep/package.json",
            "{\n  \"name\": \"cjsdep\",\n  \"exports\": { \"require\": \"./index.js\" }\n}\n",
        ),
        (
            "node_modules/cjsdep/index.js",
            "module.exports = { helper: 1 };\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a require-condition export map must resolve green: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert!(
        model["external"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("cjsdep")),
        "the resolved package rides the external tier: {model}"
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("unresolvable import"),
        "the require condition is in the active set:\n{stderr}"
    );
}

/// Scenario "builtins land external in both spellings" (N3 builtins law):
/// the bare `path` and the prefixed `node:fs` are external facts pre-empted
/// before any delegation — never unresolvable, never an install notice.
#[test]
fn builtins_land_external_in_both_forms() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "import path from \"path\";\nimport * as fs from \"node:fs\";\nexport const run = () => [path, fs];\n",
    )]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "builtins must exit 0: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    for package in ["path", "node:fs"] {
        assert!(
            model["external"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(package)),
            "the builtins law keeps {package} external: {model}"
        );
    }
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("unresolvable import"),
        "builtins are never delegated, never unresolvable:\n{stderr}"
    );
    assert!(
        !stderr.contains("external not installed"),
        "a builtin names no install gap:\n{stderr}"
    );
}

/// Scenario "a broken extends target reddens through the pre-read" (R7c):
/// the fault names the extended FILE by absolute path — an any-file-name
/// closure pin, never the entry config — byte-identical on both verbs. The
/// brokenness is unparseable content, never a permission trick (the gate
/// container may run as root).
#[test]
fn broken_extends_target_reddens_through_the_preread() {
    let tree = ts_tree(&[
        (
            "architecture.spec.toml",
            "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
        ),
        (
            "packages/app/tsconfig.json",
            "{\n  \"extends\": \"./tsconfig.base.json\"\n}\n",
        ),
        ("packages/app/tsconfig.base.json", "{ oops\n"),
        (
            "packages/app/a.ts",
            "import _ from \"lodash\";\nexport const a = _;\n",
        ),
    ]);
    let root = canonical_root(&tree);
    let expected = format!(
        "tsconfig load failed: {}",
        root.join("packages/app/tsconfig.base.json").display()
    );

    for verb in ["scan", "verify"] {
        let run = tree.run(&[verb, "."]);
        assert_eq!(
            run.status.code(),
            Some(1),
            "`{verb}` on a broken extends target must exit 1:\n{}",
            common::stderr(&run)
        );
        let stderr = common::stderr(&run);
        assert!(
            stderr.contains(&expected),
            "`{verb}` must name the extended file verbatim:\n{stderr}"
        );
        assert!(
            stderr_carries_line(&stderr, &expected),
            "`{verb}` must carry the pre-read line on its own stderr line:\n{stderr}"
        );
        assert!(
            !stderr.contains(root.join("packages/app/tsconfig.json").to_str().expect("utf8 path")),
            "the fault names the extended file, not the entry config:\n{stderr}"
        );
    }
}

/// Scenario "a missing extends target names the attempted path" (R7c F6):
/// the not-found line spells the joined absolute path the loader attempted,
/// exit 1.
#[test]
fn missing_extends_target_names_the_attempted_path() {
    let tree = ts_tree(&[
        (
            "packages/app/tsconfig.json",
            "{\n  \"extends\": \"./gone.json\"\n}\n",
        ),
        (
            "packages/app/a.ts",
            "import _ from \"lodash\";\nexport const a = _;\n",
        ),
    ]);
    let root = canonical_root(&tree);
    let expected = format!(
        "tsconfig not found: {}",
        root.join("packages/app/gone.json").display()
    );

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a missing extends target must exit 1: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains(&expected),
        "the line must name the attempted joined path exactly:\n{stderr}"
    );
    assert!(
        stderr_carries_line(&stderr, &expected),
        "the not-found line rides its own stderr line verbatim:\n{stderr}"
    );
}

/// Scenario "an extends cycle reddens once, named" (R7e): the cycle line
/// spells the walked chain with " -> " separators and appears once — exit 1.
#[test]
fn extends_cycle_reddens_once_named() {
    let tree = ts_tree(&[
        (
            "packages/app/tsconfig.json",
            "{\n  \"extends\": \"../shared/base.json\"\n}\n",
        ),
        (
            "packages/shared/base.json",
            "{\n  \"extends\": \"../app/tsconfig.json\"\n}\n",
        ),
        (
            "packages/app/a.ts",
            "import _ from \"lodash\";\nexport const a = _;\n",
        ),
    ]);
    let root = canonical_root(&tree);
    let app = root.join("packages/app/tsconfig.json");
    let base = root.join("packages/shared/base.json");
    let anchor = "tsconfig cycle (extends or reference):";
    let expected = format!(
        "{anchor} {} -> {} -> {}",
        app.display(),
        base.display(),
        app.display()
    );

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "an extends cycle must exit 1: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains(&expected),
        "the cycle line must spell the walked chain with \" -> \":\n{stderr}"
    );
    assert_eq!(
        stderr.matches(anchor).count(),
        1,
        "the entry fault names itself once, not once per import:\n{stderr}"
    );
}

/// Scenario "a config nothing references never reddens" (R2 discovery
/// bound): an unparseable tsconfig.json in a directory with no scanned file
/// and no chain naming it is never read — exit 0.
#[test]
fn broken_tsconfig_in_never_referenced_dir_stays_green() {
    let tree = ts_tree(&[
        ("tools/unrelated/tsconfig.json", "{ oops\n"),
        (
            "src/a.ts",
            "import _ from \"lodash\";\nexport const a = _;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "an unreferenced broken config must stay green: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("tsconfig load failed"),
        "the pre-read walks only OWNED extends closures:\n{stderr}"
    );
    assert!(
        !stderr.contains("unresolvable import"),
        "the scan stays honest green:\n{stderr}"
    );
}

/// Scenario "a config above the scan root never poisons the subtree"
/// (R2 bound, the M2 pin): the owning walk stops AT the scan root, so the
/// broken parent config is never read and the subtree resolves alias-free —
/// bare misses stay ordinary external facts, exit 0.
#[test]
fn broken_tsconfig_above_scan_root_stays_green() {
    let tree = Fixture::new();
    tree.write("tsconfig.json", "{ oops\n");
    tree.write(
        "sub/src/a.ts",
        "import { thing } from \"@x/thing\";\nexport const a = thing;\n",
    );

    let scan = tree.run_in("sub", &["scan", "."]);
    assert!(
        scan.status.success(),
        "a broken config above the scan root must never redden the subtree: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert!(
        model["external"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("@x/thing")),
        "the inferred project is alias-free: the miss stays an external fact: {model}"
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("tsconfig"),
        "the out-of-bound config is never read, so it never speaks:\n{stderr}"
    );
    assert!(
        !stderr.contains("unresolvable import"),
        "the alias-free bare miss fails nothing:\n{stderr}"
    );
}

/// Scenario "an invalid manifest behind the legacy fallback reddens" (M1
/// honesty law): the package IS installed and its entry file exists, yet a
/// first existing package.json on the resolution path is unparseable — the
/// finding names that manifest and the run reddens beside the resolvable
/// file; it never masquerades as not-installed or unresolvable.
#[test]
fn invalid_manifest_behind_legacy_fallback_reddens() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": { \"broken-dep\": \"^1.0.0\" }\n}\n",
    );
    tree.write(
        "src/a.ts",
        "import dep from \"broken-dep\";\nexport const a = dep;\n",
    );
    tree.write(
        "node_modules/broken-dep/index.js",
        "module.exports = { helper: 1 };\n",
    );
    tree.write("node_modules/broken-dep/package.json", "{ oops\n");
    let root = canonical_root(&tree);
    let expected = format!(
        "package manifest invalid: {}",
        root.join("node_modules/broken-dep/package.json").display()
    );

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "the invalid manifest on the resolution path must redden the run: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains(&expected),
        "the finding must name the manifest's absolute path:\n{stderr}"
    );
    assert!(
        stderr_carries_line(&stderr, &expected),
        "the manifest finding rides its own stderr line verbatim:\n{stderr}"
    );
    assert!(
        !stderr.contains("unresolvable import"),
        "the resolvable package never masquerades as an unresolvable import:\n{stderr}"
    );
    assert!(
        !stderr.contains("external not installed"),
        "installed-but-broken never masquerades as not-installed:\n{stderr}"
    );
}

/// Scenario "an undeclared bare miss stays external" (class (d) waits for
/// R2): nothing declared, nothing installed — the legacy external fact
/// stands on exit 0 with zero new stderr shapes.
#[test]
fn undeclared_bare_absent_stays_external() {
    let tree = Fixture::new();
    tree.write("package.json", "{\n  \"name\": \"demo-ui\"\n}\n");
    tree.write(
        "src/a.ts",
        "import { helper } from \"@org/pkg/sub/module\";\nexport const a = helper;\n",
    );

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "an undeclared bare miss must stay green: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert!(
        model["external"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("@org/pkg")),
        "the undeclared miss keeps the legacy external fact: {model}"
    );
    let stderr = common::stderr(&scan);
    for shape in [
        "unresolvable import",
        "external not installed",
        "package manifest invalid",
        "unclassified resolver error",
        "tsconfig",
    ] {
        assert!(
            !stderr.contains(shape),
            "an undeclared bare miss must print no `{shape}` shape:\n{stderr}"
        );
    }
}

/// Scenario "a declared-absent package prints the report line" (R7b law):
/// report-only — exit 0, the exact stderr line naming package and declaring
/// manifest, and the machine body stays pure model JSON (no
/// `report_only_lines` key).
#[test]
fn declared_external_absent_prints_the_report_line() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": { \"react\": \"^18.2.0\" }\n}\n",
    );
    tree.write(
        "src/a.ts",
        "import react from \"react\";\nexport const a = react;\n",
    );

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "the report-only line must never redden the run: {}",
        common::stderr(&scan)
    );
    let line = "external not installed: react — declared in package.json, absent from disk";
    let stderr = common::stderr(&scan);
    assert!(
        stderr.lines().any(|l| l == line),
        "the scan report prints the report-only line, exact bytes:\n{stderr}"
    );
    let model = model(&scan);
    assert!(
        model["external"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("react")),
        "the declared-absent package still states its external fact: {model}"
    );
    assert!(
        model.get("report_only_lines").is_none(),
        "the report-only line never rides the machine body: {model}"
    );
}

/// Scenario NODEXT-JS-PREFERS-EXISTING-JS (R10 literal-first, round 4 B):
/// a `.js`/`.mjs`/`.cjs` import with the literal AND the ts twin on disk
/// lands the edge on the LITERAL — the runtime loads what it loads, so a
/// shadowing twin must never flip the mixed-tree verdict.
#[test]
fn nodext_literal_js_wins_over_the_ts_twin() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { x } from \"./x.js\";\nimport { w } from \"./w.mjs\";\nimport { v } from \"./v.cjs\";\nexport const a = [x, w, v];\n",
        ),
        ("src/x.js", "export const x = 1;\n"),
        ("src/x.ts", "export const x = 2;\n"),
        ("src/w.mjs", "export const w = 1;\n"),
        ("src/w.mts", "export const w = 2;\n"),
        ("src/v.cjs", "export const v = 1;\n"),
        ("src/v.cts", "export const v = 2;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![
            ("src/a.ts".to_string(), "src/v.cjs".to_string(), false),
            ("src/a.ts".to_string(), "src/w.mjs".to_string(), false),
            ("src/a.ts".to_string(), "src/x.js".to_string(), false),
        ],
        "the literal spelling wins over every ts twin"
    );
}

/// Scenario NODEXT-JS-MAPS-TO-TS (R10 substitution, round 4 B): with the
/// literal ABSENT, the `.js` import maps to the `.ts` twin; `.mjs` maps to
/// `.mts`; `.cjs` maps to `.cts`. No `.svelte` row exists in any mapping.
#[test]
fn nodext_js_import_maps_to_the_ts_twin() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { x } from \"./x.js\";\nimport { y } from \"./y.mjs\";\nimport { z } from \"./z.cjs\";\nexport const a = [x, y, z];\n",
        ),
        ("src/x.ts", "export const x = 1;\n"),
        ("src/y.mts", "export const y = 1;\n"),
        ("src/z.cts", "export const z = 1;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![
            ("src/a.ts".to_string(), "src/x.ts".to_string(), false),
            ("src/a.ts".to_string(), "src/y.mts".to_string(), false),
            ("src/a.ts".to_string(), "src/z.cts".to_string(), false),
        ],
        "the literal miss maps to the twin: .js→.ts, .mjs→.mts, .cjs→.cts"
    );
}

/// Scenario NODEXT-PURE-JS-PACKAGE (R10 Rejected paragraph, round 4 B): a
/// package whose `main` is `lib.js` with NO twin resolves to the external
/// fact — the oxc-side `extension_alias` table stays disabled, so nothing
/// false-reddens as `unclassified resolver error: ExtensionAlias`.
#[test]
fn nodext_pure_js_package_stays_external() {
    let tree = ts_tree(&[
        (
            "node_modules/purejs/package.json",
            "{\n  \"name\": \"purejs\",\n  \"main\": \"lib.js\"\n}\n",
        ),
        ("node_modules/purejs/lib.js", "module.exports = { p: 1 };\n"),
        (
            "src/a.ts",
            "import { p } from \"purejs\";\nexport const a = p;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a twin-less main must never redden: {}",
        common::stderr(&scan)
    );
    assert!(
        model(&scan)["external"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("purejs")),
        "the twin-less package states its external fact"
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("unclassified resolver error"),
        "the disabled alias table emits no ExtensionAlias red:\n{stderr}"
    );
}

/// Scenario VIRTUAL-SPECIFIER (R7 virtual-specifier law, round 4 C): a
/// scheme-shaped specifier is NEVER delegated and NEVER stripped — one
/// report-only line names it verbatim, exit 0, NO edge and NO external-tier
/// entry (the `https:` identity pollution is the named fix).
#[test]
fn virtual_scheme_specifier_is_reported_not_modelled() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import \"https://esm.sh/lodash\";\nimport { b } from \"./b\";\nexport const a = b;\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a virtual specifier names no verdict: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr_carries_line(&stderr, "virtual specifier not modelled: https://esm.sh/lodash"),
        "the report-only line names the specifier verbatim:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("virtual specifier not modelled").count(),
        1,
        "the line is deduped to one occurrence:\n{stderr}"
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![("src/a.ts".to_string(), "src/b.ts".to_string(), false)],
        "the virtual import contributes NO edge"
    );
    let model_json = model(&scan);
    let external: Vec<&str> = model_json["external"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap_or_default())
        .collect();
    assert!(
        !external.iter().any(|name| name.contains(':') || name.contains("//")),
        "the virtual import contributes NO external-tier entry: {external:?}"
    );
}

/// Scenario VIRTUAL-SPECIFIER, NUL-leading arm (R7, round 4 C): a specifier
/// starting with a NUL byte is a bundler virtual-module id — the run states
/// the line verbatim, records no edge and no external, exit 0. The source
/// spells the id with the `\0` escape (ECMA-262 StringLiteral), which the
/// scanner decodes to the runtime NUL the law tests by name.
#[test]
fn virtual_nul_specifier_is_reported_not_modelled() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import x from \"\\0virtual:module\";\nimport { b } from \"./b\";\nexport const a = [x, b];\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a NUL-leading specifier names no verdict: {}",
        common::stderr(&scan)
    );
    assert!(
        stderr_carries_line(&common::stderr(&scan), "virtual specifier not modelled: \u{0}virtual:module"),
        "the report-only line names the NUL id verbatim:\n{}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![("src/a.ts".to_string(), "src/b.ts".to_string(), false)],
        "the virtual import contributes NO edge"
    );
    let model_json = model(&scan);
    let external: Vec<&str> = model_json["external"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap_or_default())
        .collect();
    assert!(
        !external.iter().any(|name| name.contains("virtual")),
        "the virtual import contributes NO external-tier entry: {external:?}"
    );
}

/// Scenario R7 query-suffix law, found arms (round 4 C): a `?`-query on a
/// relative, aliased or bare specifier resolves on the BASE — the edge is
/// ordinary and the external identity LOSES the query
/// (`canvas-confetti?optional` names `canvas-confetti`).
#[test]
fn query_suffix_resolves_on_the_base() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { b } from \"./b?worker\";\nimport confetti from \"canvas-confetti?optional\";\nexport const a = [b, confetti];\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
        (
            "node_modules/canvas-confetti/package.json",
            "{\n  \"name\": \"canvas-confetti\",\n  \"main\": \"index.js\"\n}\n",
        ),
        ("node_modules/canvas-confetti/index.js", "module.exports = function () {};\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "query suffixes resolve on the base: {}",
        common::stderr(&scan)
    );
    assert!(
        module_edge_rows(&model(&scan))
            .contains(&("src/a.ts".to_string(), "src/b.ts".to_string(), false)),
        "the queried relative import lands the ordinary edge: {:?}",
        module_edge_rows(&model(&scan))
    );
    assert!(
        model(&scan)["external"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("canvas-confetti")),
        "the external identity drops the query: {}",
        common::stdout(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("canvas-confetti?optional") && !stderr.contains("unresolvable import"),
        "the query never reaches the miss classes:\n{stderr}"
    );
}

/// Scenario R7 query-suffix law, absent-base arm (round 4 C): the base
/// miss rides the ORDINARY class NAMING THE FULL ORIGINAL SPECIFIER
/// verbatim — the query stays in the reported line.
#[test]
fn query_suffix_on_absent_base_names_the_full_specifier() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { g } from \"./gone?worker\";\nexport const a = g;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "an absent base is the ordinary miss: {}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan).contains("unresolvable import: src/a.ts -> \"./gone?worker\""),
        "the miss names the FULL original specifier:\n{}",
        common::stderr(&scan)
    );
}

/// Scenario R9/R7 query-suffix law on the ALIASED arm (round 4 C): the
/// queried alias specifier resolves its base through the owning config's
/// paths map — the ordinary first-party edge, no query in any line.
#[test]
fn query_suffix_resolves_on_an_aliased_base() {
    let tree = ts_tree(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"paths":{"@alias/*":["./src/lib/*"]}}}"#,
        ),
        (
            "src/a.ts",
            "import { w } from \"@alias/widget?raw\";\nexport const a = w;\n",
        ),
        ("src/lib/widget.ts", "export const w = 1;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "the queried alias base resolves: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![(
            "src/a.ts".to_string(),
            "src/lib/widget.ts".to_string(),
            false
        )],
        "the aliased query lands the ordinary first-party edge"
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("unresolvable import"),
        "the aliased base is found — no miss line:\n{stderr}"
    );
}

/// Scenario VIRTUAL-SPECIFIER, silent-parser tripwire arm (R7 + D2, round 4
/// C): a file whose ONLY import is a virtual specifier states a report-only
/// outcome, not silence — the tripwire never reddens the law (exit 0).
#[test]
fn virtual_only_import_does_not_trip_the_silence_tripwire() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { v } from \"virtual:config\";\nexport const a = v;\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "the virtual-only import is a stated outcome, not parser silence: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("tripwire"),
        "the tripwire stays silent on the report-only arm:\n{stderr}"
    );
    assert!(
        stderr_carries_line(&stderr, "virtual specifier not modelled: virtual:config"),
        "the report-only line names the scheme-shaped id verbatim:\n{stderr}"
    );
    assert!(
        module_edge_rows(&model(&scan)).is_empty(),
        "the virtual import contributes NO edge"
    );
}

/// Fixture GEN-NOTBUILT-IMPORTS-MAP (R12, round 7 P-GENSRC): an imports-map
/// target that is ABSENT and gitignored is the generated class — ONE
/// report-only line, exit 0, NO edge, NO external-tier entry. No git is
/// spawned; the verdict is the static evaluation of the committed
/// `.gitignore`.
#[test]
fn gen_not_built_imports_map_target_rides_the_report_line() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"imports\": { \"#map\": \"./generated/map.js\" }\n}\n",
    );
    tree.write(
        ".gitignore",
        "# generated artifacts\nnode_modules/\ngenerated/\n",
    );
    tree.write(
        "src/main.ts",
        "import { map } from \"#map\";\nexport const m = map;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "a gitignored imports target is not-built, not a defect: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            "generated source not built: #map — gitignored .gitignore:3"
        ),
        "the line names the verbatim specifier and the evidence file:line:\n{stderr}"
    );
    let model = model(&scan);
    assert!(
        model["external"].as_array().unwrap().is_empty(),
        "the generated class adds NO external-tier entry: {model}"
    );
}

/// The vacuity carrier of GEN-NOTBUILT-IMPORTS-MAP: with NO `.gitignore`
/// evidence the class never fires and the red stands (fail closed, R12).
#[test]
fn gen_imports_map_target_without_gitignore_evidence_stays_red() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"imports\": { \"#map\": \"./generated/map.js\" }\n}\n",
    );
    tree.write(
        "src/main.ts",
        "import { map } from \"#map\";\nexport const m = map;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "no committed ignore evidence keeps the honest red: {stderr}"
    );
    assert!(
        stderr.contains("unresolvable import"),
        "the miss stays an unresolvable import:\n{stderr}"
    );
    assert!(
        !stderr.contains("generated source not built"),
        "the class never fires without evidence:\n{stderr}"
    );
}

/// Fixture GEN-NOTBUILT-ALIAS (R12): the alias-target-not-found arm with a
/// gitignored computed target (the open-mercato `.mercato` family — 107
/// pilot reds) rides the report line. The unanchored `.mercato` pattern
/// carries at ANY depth (fidelity law).
#[test]
fn gen_not_built_alias_target_rides_the_report_line() {
    let tree = Fixture::new();
    tree.write("package.json", "{\n  \"name\": \"demo-ui\"\n}\n");
    tree.write(
        "tsconfig.json",
        "{\n  \"compilerOptions\": { \"baseUrl\": \".\", \"paths\": { \"@mercato/*\": [\"./.mercato/*\"] } }\n}\n",
    );
    tree.write(
        ".gitignore",
        "node_modules/\n.mercato\n",
    );
    tree.write(
        "src/main.ts",
        "import { map } from \"@mercato/map\";\nexport const m = map;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "a gitignored alias target is not-built, not a defect: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            "generated source not built: @mercato/map — gitignored .gitignore:2"
        ),
        "the line names the verbatim specifier and the evidence file:line:\n{stderr}"
    );
    let model = model(&scan);
    assert!(
        module_edge_rows(&model).is_empty(),
        "the generated class adds NO edge: {model}"
    );
    assert!(
        model["external"].as_array().unwrap().is_empty(),
        "the generated class adds NO external-tier entry: {model}"
    );
}

/// The vacuity carrier of GEN-NOTBUILT-ALIAS: without the ignore evidence
/// the alias-target red stands.
#[test]
fn gen_alias_target_without_gitignore_evidence_stays_red() {
    let tree = Fixture::new();
    tree.write("package.json", "{\n  \"name\": \"demo-ui\"\n}\n");
    tree.write(
        "tsconfig.json",
        "{\n  \"compilerOptions\": { \"baseUrl\": \".\", \"paths\": { \"@mercato/*\": [\"./.mercato/*\"] } }\n}\n",
    );
    tree.write(
        "src/main.ts",
        "import { map } from \"@mercato/map\";\nexport const m = map;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "no committed ignore evidence keeps the alias-target red: {stderr}"
    );
    assert!(
        stderr.contains("alias target not found"),
        "the ordinary alias-target red stands:\n{stderr}"
    );
}

/// Fixture GEN-PARENT-DIR-IGNORE (R12 fidelity law): `build/` excludes the
/// directory and the later `!build/keep.ts` can NEVER re-include — git
/// does not descend into excluded directories — so the absent target under
/// the excluded parent is the generated class (the parent's line rides).
#[test]
fn gen_parent_dir_ignore_blocks_reentry_and_rides_the_parent_line() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"imports\": { \"#keep\": \"./build/keep.ts\" }\n}\n",
    );
    tree.write("src/main.ts", "export const m = 1;\n");
    tree.write(
        ".gitignore",
        "build/\n!build/keep.ts\n",
    );
    let probe = tree.run(&["scan", "."]);
    assert!(
        probe.status.success(),
        "the control scan (no importer of #keep) is green: {}",
        common::stderr(&probe)
    );
    // The verdict itself is pinned by the evaluator unit tests against the
    // git oracle; the scan-level carrier here is the imports-map shape.
    let tree2 = Fixture::new();
    tree2.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"imports\": { \"#keep\": \"./build/keep.ts\" }\n}\n",
    );
    tree2.write(
        ".gitignore",
        "build/\n!build/keep.ts\n",
    );
    tree2.write(
        "src/main.ts",
        "import { keep } from \"#keep\";\nexport const m = keep;\n",
    );
    let scan = tree2.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "an excluded parent is not-built regardless of the deeper negation: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            "generated source not built: #keep — gitignored .gitignore:1"
        ),
        "the PARENT's exclusion line decides, not the negation:\n{stderr}"
    );
}

/// Fixture GEN-NEGATION (R12 fidelity law): a file-shaped negation that
/// matches the target is the LAST match — not ignored — so the class does
/// NOT fire and the red stands (fail closed: tracked intent means a real
/// missing file).
#[test]
fn gen_negated_pattern_keeps_the_red() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"imports\": { \"#x\": \"./src/x.gen.ts\" }\n}\n",
    );
    tree.write(
        ".gitignore",
        "*.gen.ts\n!src/x.gen.ts\n",
    );
    tree.write(
        "src/main.ts",
        "import { x } from \"#x\";\nexport const m = x;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a negated target is tracked intent — the red stands: {stderr}"
    );
    assert!(
        !stderr.contains("generated source not built"),
        "the class never fires on a negated target:\n{stderr}"
    );
}

/// Fixture GEN-NESTED-PRECEDENCE (R12 fidelity law): the DEEPER
/// `.gitignore` states the same exclusion — the deeper file's line is the
/// evidence named on the report line (deeper wins, not shallower).
#[test]
fn gen_nested_gitignore_precedence_names_the_deeper_file() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        ".gitignore",
        "generated/\n",
    );
    tree.write(
        "packages/app/package.json",
        "{\n  \"name\": \"@org/app\",\n  \"imports\": { \"#map\": \"./generated/map.js\" }\n}\n",
    );
    tree.write("packages/app/.gitignore", "generated/\n");
    tree.write(
        "packages/app/main.ts",
        "import { map } from \"#map\";\nexport const m = map;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "a gitignored imports target is not-built: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            "generated source not built: #map — gitignored packages/app/.gitignore:1"
        ),
        "the deeper file's line is the named evidence:\n{stderr}"
    );
}

/// Fixture GEN-ANCHORED (R12 fidelity law): a leading-slash pattern is
/// anchored to its file's directory — it carries the root shape (green)
/// and must NOT depth-carry a same-named dir elsewhere (the red twin).
#[test]
fn gen_anchored_pattern_carries_only_its_directory() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"imports\": { \"#k\": \"./generated/map.js\" }\n}\n",
    );
    tree.write(".gitignore", "/generated\n");
    tree.write(
        "src/main.ts",
        "import { map } from \"#k\";\nexport const m = map;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "the anchored pattern carries its own directory: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            "generated source not built: #k — gitignored .gitignore:1"
        ),
        "the anchored line decides at the root:\n{stderr}"
    );
    // The twin: the same spelling in a DEEPER package dir is not covered
    // by the anchored root pattern — the red stands there.
    let twin = Fixture::new();
    twin.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    twin.write(".gitignore", "/generated\n");
    twin.write(
        "packages/app/package.json",
        "{\n  \"name\": \"@org/app\",\n  \"imports\": { \"#k\": \"./generated/map.js\" }\n}\n",
    );
    twin.write(
        "packages/app/main.ts",
        "import { map } from \"#k\";\nexport const m = map;\n",
    );
    let scan = twin.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "an anchored pattern never depths — that miss stays honest: {stderr}"
    );
    assert!(
        !stderr.contains("generated source not built"),
        "no generated-class line without coverage:\n{stderr}"
    );
}

/// Fixture GEN-RELATIVE (R12 generality law): the generated class covers
/// RELATIVE specifiers too — an absent `./generated/...` target ignored by
/// a committed `.gitignore` is report-only (the open-mercato storybook
/// shape), and without the ignore evidence the red stands (vacuity twin).
#[test]
fn gen_relative_import_gitignored_is_report_only() {
    let tree = Fixture::new();
    tree.write("package.json", "{\n  \"name\": \"demo-ui\"\n}\n");
    tree.write(".gitignore", "generated/\n");
    tree.write(
        "src/main.ts",
        "import { data } from \"./generated/data\";\nexport const d = data;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "the relative generated target is report-only, not a verdict: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            "generated source not built: ./generated/data — gitignored .gitignore:1"
        ),
        "the verbatim relative specifier rides the line:\n{stderr}"
    );
    // The vacuity twin: the identical tree without the committed evidence
    // keeps the red — the class never silences a genuine miss.
    let twin = Fixture::new();
    twin.write("package.json", "{\n  \"name\": \"demo-ui\"\n}\n");
    twin.write(
        "src/main.ts",
        "import { data } from \"./generated/data\";\nexport const d = data;\n",
    );
    let scan = twin.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "no ignore evidence, no exemption: {stderr}"
    );
    assert!(
        stderr.contains("unresolvable import"),
        "the red stands without the evidence:\n{stderr}"
    );
    assert!(
        !stderr.contains("generated source not built"),
        "no generated-class line without coverage:\n{stderr}"
    );
}

/// Fixture GEN-GI-CACHE (perf lane 2026-10-17, fix 1 cache trap): the
/// per-scan `.gitignore` parse cache must never confuse ONE directory's
/// file with another's. One scan walks TWO sibling trees through the SAME
/// cached root file; only ONE of them carries a deeper `.gitignore` whose
/// negation re-includes the name — deeper-precedence must survive the
/// cache in both directions.
#[test]
fn gen_gitignore_cache_keeps_deeper_precedence() {
    // The report-only twin: the b-side has no deeper file, so the root
    // rule decides through the cache — one line, the root file's line.
    let tree = Fixture::new();
    tree.write("package.json", "{\n  \"name\": \"demo\"\n}\n");
    tree.write(".gitignore", "gen\n");
    tree.write("packages/a/.gitignore", "!gen\n");
    tree.write(
        "packages/a/main.ts",
        "import { h } from \"../b/gen/index.js\";\nexport const v = h;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "the root rule decides the b-side through the cache: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            "generated source not built: ../b/gen/index.js — gitignored .gitignore:1"
        ),
        "the cached root file alone decides the b-side:\n{stderr}"
    );
    // The red twin: the SAME run also walks a/ through the root file, but
    // a/ carries the deeper negation — deeper-precedence must survive the
    // shared cache and keep the miss honest.
    let twin = Fixture::new();
    twin.write("package.json", "{\n  \"name\": \"demo\"\n}\n");
    twin.write(".gitignore", "gen\n");
    twin.write("packages/a/.gitignore", "!gen\n");
    twin.write(
        "packages/a/main.ts",
        "import { g } from \"./gen/index.js\";\nimport { h } from \"../b/gen/index.js\";\nexport const v = [g, h];\n",
    );
    let scan = twin.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "the deeper negation keeps the a-side miss honest: {stderr}"
    );
    assert!(
        stderr.contains("unresolvable import: packages/a/main.ts -> \"./gen/index.js\""),
        "the deeper file's `!gen` re-includes under a — the red stands:\n{stderr}"
    );
}

/// Fixture TSCGEN-SVELTEKIT-ABSENT (round-6 P-GEN-TSCONFIG, the PAIRING
/// law; round-8 P-FW1 RESIGNS the signal: THE KEY, NOT THE FILE): a
/// sveltekit tree whose generated tsconfig was never built — the extends
/// target under `.svelte-kit` is ABSENT. The run prints ONE report-only
/// line, exits 0, and the static implicit `$lib` alias lands the EDGES:
/// line and edge are one paired law — unpaired, the quiet line deletes the
/// only witness of the external-tier mis-tiers. The pairing half now
/// engages on `framework = "sveltekit"` at `[project]`; the
/// `svelte.config.js` file is written here to prove it is NEVER consulted
/// (the probe is deleted — the file's presence decides nothing).
#[test]
fn tscgen_sveltekit_absent_pairs_line_with_lib_edges() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\nframework = \"sveltekit\"\n",
    );
    tree.write(
        "svelte.config.js",
        "export default { kit: { files: { lib: 'src/lib' } } };\n",
    );
    tree.write(
        "tsconfig.json",
        "{\n  \"extends\": \"./.svelte-kit/tsconfig.json\"\n}\n",
    );
    tree.write(
        "src/routes/+page.svelte",
        "<script>\nimport { util } from \"$lib/util\";\nexport const page = util;\n</script>\n",
    );
    tree.write("src/lib/util.ts", "export const util = 1;\n");

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "a sveltekit fresh clone is the normal state, not a defect: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            &format!(
                "generated tsconfig absent: {}",
                tree.path(".svelte-kit/tsconfig.json").display()
            )
        ),
        "the line names the absent generated target verbatim:\n{stderr}"
    );
    let rows = module_edge_rows(&model(&scan));
    assert!(
        rows.iter()
            .any(|(from, to, _)| from == "src/routes/+page.svelte" && to == "src/lib/util.ts"),
        "the paired alias half must land the $lib edge, never a mistier: {rows:?}"
    );
}

/// The vacuity twin of TSCGEN-SVELTEKIT-ABSENT: an ABSENT extends-target
/// OUTSIDE the generated class keeps the `tsconfig not found` red
/// byte-identical — only the two named generated/installed absences leave
/// the red (genuinely broken authoring stays honest).
#[test]
fn tscgen_absent_extends_outside_generated_class_stays_red() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": { \"lodash\": \"^4.17.21\" }\n}\n",
    );
    tree.write(
        "tsconfig.json",
        "{\n  \"extends\": \"./base.json\"\n}\n",
    );
    tree.write(
        "src/main.ts",
        "import _ from \"lodash\";\nexport const x = _;\n",
    );

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a non-generated absent extends is broken authoring: {stderr}"
    );
    assert!(
        stderr.contains("tsconfig not found:"),
        "the red names the missing target:\n{stderr}"
    );
    assert!(
        !stderr.contains("generated tsconfig absent"),
        "no report-only line outside the generated class:\n{stderr}"
    );
}

/// Fixture TSCGEN-EXTENDS-NOT-INSTALLED (round-6 P-GEN-TSCONFIG iii): a
/// node_modules-style extends target with no installed copy — ONE
/// report-only line naming the SPECIFIER verbatim (the `external not
/// installed` precedent; `@tsconfig/*` bases carry no paths, the measured
/// nil delta), exit 0.
#[test]
fn tscgen_extends_not_installed_names_the_specifier() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"srv\",\n  \"dependencies\": { \"lodash\": \"^4.17.21\" }\n}\n",
    );
    tree.write(
        "tsconfig.json",
        "{\n  \"extends\": \"@tsconfig/node20/tsconfig.json\"\n}\n",
    );
    tree.write(
        "src/main.ts",
        "import _ from \"lodash\";\nexport const x = _;\n",
    );

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "an uninstalled extends base is the clone state, not a defect: {stderr}"
    );
    assert!(
        stderr_carries_line(
            &stderr,
            "tsconfig extends not installed: @tsconfig/node20/tsconfig.json"
        ),
        "the line names the as-declared specifier verbatim:\n{stderr}"
    );
    assert!(
        !stderr.contains("tsconfig not found"),
        "the red leaves the not-installed class:\n{stderr}"
    );
}

/// Fixture SVELTE-ALIAS-VACUITY (round-8 P-FW1/P-FW2 rewrite: the signal
/// is the KEY, the probe is DELETED): `svelte.config.js` sits at the scan
/// root while the spec declares NO `[project] framework` key — the file
/// decides nothing. `$lib` keeps its honest external fact (behavior
/// unchanged) and no edge is invented; the ONE new witness is the
/// report-only shape line asking the framework question, once per
/// (importing dir, spelled specifier).
#[test]
fn svelte_alias_vacuity_without_framework_key_keeps_the_fact() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "svelte.config.js",
        "export default { kit: { files: { lib: 'src/lib' } } };\n",
    );
    tree.write(
        "src/routes/+page.svelte",
        "<script>\nimport { util } from \"$lib/util\";\nexport const page = util;\n</script>\n",
    );
    tree.write("src/lib/util.ts", "export const util = 1;\n");

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(scan.status.success(), "unchanged verdict law: {stderr}");
    let model = model(&scan);
    assert!(
        module_edge_rows(&model).is_empty(),
        "a framework config file without the key never gains the implicit alias: {model}"
    );
    assert_eq!(
        model["external"].as_array().unwrap().len(),
        1,
        "the $lib miss stays the plain external fact it was before: {model}"
    );
    assert_eq!(model["external"][0].as_str(), Some("$lib"));
    assert!(
        stderr_carries_line(
            &stderr,
            "specifier not a package name: $lib/util — framework conventions not configured?"
        ),
        "the $ shape without the key gains the report-only question line:\n{stderr}"
    );
}

/// The key-gated twin of SVELTE-ALIAS-VACUITY: WITH `framework =
/// "sveltekit"` the `$lib` alias engages (edge lands) and the shape line
/// is silent for `$lib` specifiers.
#[test]
fn svelte_alias_engages_on_framework_key_silences_shape_line() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\nframework = \"sveltekit\"\n",
    );
    tree.write(
        "src/routes/+page.svelte",
        "<script>\nimport { util } from \"$lib/util\";\nexport const page = util;\n</script>\n",
    );
    tree.write("src/lib/util.ts", "export const util = 1;\n");

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(scan.status.success(), "declared tree stays green: {stderr}");
    let rows = module_edge_rows(&model(&scan));
    assert!(
        rows.iter()
            .any(|(from, to, _)| from == "src/routes/+page.svelte" && to == "src/lib/util.ts"),
        "the key registers the $lib alias: {rows:?}"
    );
    assert!(
        !stderr.contains("framework conventions not configured"),
        "the configured framework silences the question line:\n{stderr}"
    );
}

/// Fixture SHAPE-LINE-DEDUP-COEXISTS (round-8 P-FW2/P-SEQ, F9): without
/// the framework key every `$`-shaped bare specifier gains the report-only
/// line EXACTLY ONCE per (importing dir, spelled specifier) — repeated
/// imports memo-dedup — while each specifier KEEPS its generic
/// external-tier attribution: the line never launders the miss into
/// first-party and never replaces the external fact.
#[test]
fn shape_line_dedups_and_coexists_with_external_tier() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "src/main.ts",
        "import { a } from \"$lib/a\";\nimport { b } from \"$lib/a\";\nimport { c } from \"$app/build\";\nexport const x = [a, b, c];\n",
    );

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(scan.status.success(), "report-only never reddens: {stderr}");
    let lib_line = "specifier not a package name: $lib/a — framework conventions not configured?";
    let app_line = "specifier not a package name: $app/build — framework conventions not configured?";
    assert!(stderr_carries_line(&stderr, lib_line), "$lib line missing:\n{stderr}");
    assert!(stderr_carries_line(&stderr, app_line), "$app line missing:\n{stderr}");
    assert_eq!(
        stderr.matches(lib_line).count(),
        1,
        "the memo replays the pair exactly once:\n{stderr}"
    );
    let model = model(&scan);
    let external: Vec<&str> = model["external"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row.as_str().unwrap())
        .collect();
    assert!(
        external.contains(&"$lib") && external.contains(&"$app"),
        "the shape line coexists with — never replaces — attribution: {external:?}"
    );
    assert!(
        module_edge_rows(&model).is_empty(),
        "no framework means no invented edges: {model}"
    );
}

/// Round-8 P-FW1 fail-closed law at spec load: a framework name outside
/// the declared driver's registry is an `invalid spec` error on every
/// spec-loading verb, naming the driver's registered alternatives.
#[test]
fn framework_unknown_name_fails_spec_load_fail_closed() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\nframework = \"nextjs\"\n",
    );
    tree.write("src/main.ts", "export const x = 1;\n");

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "an unregistered framework fails closed: {stderr}"
    );
    assert!(
        stderr.contains("unknown framework \"nextjs\" — registered: sveltekit"),
        "the red names the registered alternatives:\n{stderr}"
    );
}

/// Fixture VENDOR-SOURCE-EXT-UNDER-ROOT (round-6 P-VENDOR, R11): a
/// SOURCE-extension file under a `vendor` segment under the scan root
/// stays IN SCOPE — the same predicate law answers collect_files and
/// is_excluded (the pilot's swallowed first-party glue).
#[test]
fn vendor_source_ext_under_root_stays_in_scope() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write("src/main.ts", "import { glue } from \"../vendor/glue/index.ts\";\nexport const w = glue;\n");
    tree.write("vendor/glue/index.ts", "export const glue = 1;\n");

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert!(
        scan.status.success(),
        "vendor source glue under the scan root is first-party scope: {stderr}"
    );
    let rows = module_edge_rows(&model(&scan));
    assert!(
        rows.iter()
            .any(|(from, to, _)| from == "src/main.ts" && to == "vendor/glue/index.ts"),
        "the vendor source-ext file is scanned and linked: {rows:?}"
    );
}

/// Fixture VENDOR-ABSOLUTE-CLASSES: the absolute mechanical classes stay
/// absolute — a file under `dist` remains excluded and the shipped red
/// text stays byte-identical (declared `excluded_sources` still wins
/// everywhere; this is the mechanical half).
#[test]
fn vendor_absolute_classes_stay_excluded() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write("src/main.ts", "import { g } from \"../dist/generated.ts\";\nexport const v = g;\n");
    tree.write("dist/generated.ts", "export const g = 2;\n");

    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "the absolute classes never narrow: {stderr}"
    );
    assert!(
        stderr.contains("resolves only into excluded path dist"),
        "the exclusion red names the mechanical dir byte-identically:\n{stderr}"
    );
}

/// Fixture SRC-MJS-CJS-INDEX + SRC-MTS-CTS-INDEX (detection record): the
/// Node resolution flavours join the source family on the EXISTING
/// grammars — `.mjs`/`.cjs` parse with the JavaScript grammar, `.mts`/`.cts`
/// with the TypeScript grammar. Counted files, extracted edges; the flavour
/// states nothing about the grammar or the driver.
#[test]
fn mjs_cjs_mts_cts_are_indexed_with_the_existing_grammars() {
    let tree = ts_tree(&[
        ("src/app.mjs", "import { x } from \"./x.cjs\";\nexport const app = x;\n"),
        ("src/x.cjs", "module.exports.x = 1;\n"),
        (
            "src/mod.mts",
            "import { y } from \"./y.cts\";\nexport const m: number = y;\n",
        ),
        ("src/y.cts", "export const y: number = 2;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert_eq!(model["language"], "ts");
    let mut files = declarations(&model);
    files.sort();
    assert_eq!(
        files,
        vec![
            "src/app.mjs",
            "src/mod.mts",
            "src/x.cjs",
            "src/y.cts",
        ],
        "every flavour file is a counted vertex"
    );
    let rows = module_edge_rows(&model);
    assert!(
        rows.iter()
            .any(|(from, to, _)| from == "src/app.mjs" && to == "src/x.cjs"),
        "the mjs -> cjs edge lands: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(from, to, _)| from == "src/mod.mts" && to == "src/y.cts"),
        "the mts -> cts edge lands: {rows:?}"
    );
}

/// Fixture SRC-MJS-NEXTMAP (detection record): a `.mjs` importer reaches a
/// literal `.mjs` target and a NodeNext-style `./y.js` specifier mapped to
/// the TypeScript source — the same extension-mapping law the `.ts`
/// importers already ride, now proven from the `.mjs` side.
#[test]
fn mjs_importer_resolves_literal_and_mapped_targets() {
    let tree = ts_tree(&[
        (
            "src/entry.mjs",
            "import { x } from \"./x.mjs\";\nimport { y } from \"./y.js\";\nexport const e = [x, y];\n",
        ),
        ("src/x.mjs", "export const x = 1;\n"),
        ("src/y.ts", "export const y = 2;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let rows = module_edge_rows(&model(&scan));
    assert!(
        rows.iter()
            .any(|(from, to, _)| from == "src/entry.mjs" && to == "src/x.mjs"),
        "the literal .mjs target hits: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(from, to, _)| from == "src/entry.mjs" && to == "src/y.ts"),
        "the ./y.js specifier maps to y.ts: {rows:?}"
    );
}

// ── require() tracing (workplan workplan_archspec_require_tracing) ──────────
//
// Law: a literal `require("lit")` is an ordinary value edge through the same
// resolution seam (D1); a computed `require(expr)` rides the existing
// `dynamic_import_wildcards` census and the existing report line (D2); the
// extraction is grammar-driven and extension-blind (D3); no symbols, never
// republication (D4); `require.resolve`, `require.cache`, `jest.requireActual`
// and `jest.requireMock` are NOT sites (D6); unresolvable literal requires
// fail loud through the existing law (Scenario 6).

/// Scenario "a literal require is an ordinary edge" (pilot shape
/// `apps/mercato/jest.config.cjs:2`): the `.cjs` config require states one
/// `(from, to, type_only=false)` row with an EMPTY symbols list (D1/D4).
#[test]
fn literal_require_in_cjs_is_an_ordinary_value_edge() {
    let tree = ts_tree(&[
        (
            "jest.config.cjs",
            "const base = require('./jest.config.base.cjs');\nmodule.exports = { ...base, name: 'demo' };\n",
        ),
        ("jest.config.base.cjs", "module.exports = { testEnvironment: 'node' };\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    let rows = module_edge_rows(&model);
    assert_eq!(
        rows,
        vec![(
            "jest.config.cjs".to_string(),
            "jest.config.base.cjs".to_string(),
            false
        )],
        "the literal require resolves by the ordinary resolution law"
    );
    let symbols = model["module_edges"][0]["symbols"].as_array().unwrap();
    assert!(
        symbols.is_empty(),
        "a require edge states no symbols (D4): {symbols:?}"
    );
}

/// Scenario "a computed require is a census fact, never an edge and never
/// red" (pilot shapes `docxRenderer.ts:451`, `module-registry.ts:3686`), in
/// its M3 hardening shape: a tree whose ONLY facts are computed requires
/// stays green with exactly the existing wildcard lines.
#[test]
fn computed_require_states_the_wildcard_line_and_stays_green() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "export function load(workerData: { modulePath: string }, name: string) {\n  return [require(workerData.modulePath), require(`./dyn/${name}`)];\n}\n",
    )]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a computed require is report-only, never red (M3): {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert_eq!(
        model["dynamic_import_wildcards"]["src/a.ts"],
        serde_json::json!([
            "`./dyn/${name}`",
            "workerData.modulePath"
        ]),
        "the computed require rides the existing wildcard census, verbatim (D2)"
    );
    assert!(
        model["module_edges"].as_array().unwrap().is_empty(),
        "a computed require never mints an edge: {model}"
    );
    let stderr = common::stderr(&scan);
    for line in [
        "dynamic import wildcard: src/a.ts -> `./dyn/${name}`",
        "dynamic import wildcard: src/a.ts -> workerData.modulePath",
    ] {
        assert!(
            stderr.lines().any(|l| l == line),
            "the existing report line renders the require spelling verbatim:\n{stderr}"
        );
    }
}

/// Scenario "jest API helpers and require.resolve state nothing" (D6, the
/// pilot's 522-site non-class): none of the non-site forms produces an edge,
/// a wildcard or a counter increment — and a tree of NOTHING but non-sites
/// stays green because it states no import-like statement at all.
#[test]
fn require_sites_that_are_not_sites_state_nothing() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "export const p = require.resolve('x');\nexport const hit = require.cache;\nexport const q = jest.requireActual('./x');\nexport const m = jest.requireMock('./x');\n",
    )]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a non-site is not an import-like statement (D7): {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    assert!(
        model["module_edges"].as_array().unwrap().is_empty(),
        "no non-site form may mint an edge: {model}"
    );
    assert!(
        model.get("dynamic_import_wildcards").is_none(),
        "no non-site form may mint a wildcard: {model}"
    );
    assert!(
        model["module_external"].get("src/a.ts").is_none(),
        "require.resolve names no external: {model}"
    );
}

/// Scenario "a bare require lands on the external tier" (pilot: 46
/// `require('react')`-class sites): builtins and npm names enter `external`
/// and the file-keyed `module_external` through the same seam an import uses.
#[test]
fn bare_requires_land_on_the_external_tier() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "const React = require('react');\nconst { Worker } = require('node:worker_threads');\nconst guard = require('server-only');\nexport const a = [React, Worker, guard];\n",
    )]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "bare requires are externals, not findings: {}",
        common::stderr(&scan)
    );
    let model = model(&scan);
    for name in ["react", "server-only", "node:worker_threads"] {
        assert!(
            model["external"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(name)),
            "the bare require `{name}` enters the external tier: {model}"
        );
    }
    let per_file: Vec<String> = model["module_external"]["src/a.ts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();
    for name in ["react", "server-only", "node:worker_threads"] {
        assert!(
            per_file.iter().any(|n| n == name),
            "file identity (D10): src/a.ts witnesses `{name}`: {per_file:?}"
        );
    }
}

/// Scenario "an unresolvable literal require fails loud like any literal
/// specifier" (Scenario 6, the B1 disclosure): exit 1, no model, the
/// existing `unresolvable import:` line naming file, specifier and reason —
/// no require-specific exemption.
#[test]
fn unresolvable_literal_require_fails_loud() {
    let tree = ts_tree(&[("src/a.ts", "const gone = require('./gone');\nexport const a = gone;\n")]);

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "an unresolvable literal require fails loud: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains("unresolvable import: src/a.ts -> \"./gone\""),
        "the finding names file, specifier and the existing fail-loud law:\n{stderr}"
    );
}

/// Scenario "a require of an asset is an edge to the exact file" (3 pilot
/// `→ *.json` edges): the D2 asset law applies to requires verbatim.
#[test]
fn require_of_json_asset_targets_the_exact_file() {
    let tree = ts_tree(&[
        (
            "src/__tests__/i18n.test.ts",
            "const en = require('../i18n/en.json');\nexport const t = en;\n",
        ),
        ("src/i18n/en.json", "{ \"hello\": \"Hello\" }\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        module_edge_rows(&model(&scan)),
        vec![(
            "src/__tests__/i18n.test.ts".to_string(),
            "src/i18n/en.json".to_string(),
            false
        )],
        "the json target is exact, never extension-inferred"
    );
}

/// Scenarios "a require inside a test file is stated" (pilot shape
/// `tenantContext.test.ts:18`) and "the ts driver owns require in every file
/// it parses" (D3, `.cjs`/`.mjs`/`.svelte` placement): the edges exist for
/// every extension the driver parses; the test file's edges ride the
/// existing C2 gate (its package declares the jest runner), so verify stays
/// green with the edges stated.
#[test]
fn literal_requires_in_mjs_svelte_and_test_files_are_edges() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": { \"lodash\": \"^4.17.21\" },\n  \"devDependencies\": { \"jest\": \"^29.0.0\" }\n}\n",
    );
    tree.write(
        "src/loader.mjs",
        "import { createRequire } from 'node:module';\nconst require = createRequire(import.meta.url);\nexport const target = require('./target-mjs');\n",
    );
    tree.write("src/target-mjs.ts", "export const m = 1;\n");
    tree.write(
        "Component.svelte",
        "<script context=\"module\">\nconst { s } = require('./src/target-svelte');\nexport { s };\n</script>\n<p>text {s}</p>\n",
    );
    tree.write("src/target-svelte.ts", "export const s = 2;\n");
    tree.write(
        "src/__tests__/x.test.ts",
        "const x = require('../x');\nexport const t = x;\n",
    );
    tree.write("src/x.ts", "export const x = 3;\n");
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
    );

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let rows = module_edge_rows(&model(&scan));
    for (from, to) in [
        ("src/loader.mjs", "src/target-mjs.ts"),
        ("Component.svelte", "src/target-svelte.ts"),
        ("src/__tests__/x.test.ts", "src/x.ts"),
    ] {
        assert!(
            rows.iter()
                .any(|(f, t, type_only)| f == from && t == to && !type_only),
            "the require edge {from} -> {to} is stated by the ordinary law (D3): {rows:?}"
        );
    }

    let verify = tree.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "the test file's edges ride the existing C2 gate: {}",
        common::stderr(&verify)
    );
}

/// Scenario "an unresolvable require whose target is GENERATED is absorbed
/// by the existing report law" (B1 mitigation): a gitignored absent target
/// is the generated class — one report-only line, exit 0, no edge (the R12
/// generated-source law replayed through the require arm).
#[test]
fn generated_require_target_rides_the_report_law() {
    let tree = ts_tree(&[("src/a.ts", "const g = require('./generated/thing');\nexport const a = g;\n")]);
    tree.write(".gitignore", "node_modules/\ngenerated/\n");

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a gitignored require target is not-built, not a defect: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr_carries_line(
            &stderr,
            "generated source not built: ./generated/thing — gitignored .gitignore:2"
        ),
        "the generated class names the verbatim specifier:\n{stderr}"
    );
    let model = model(&scan);
    assert!(
        model["module_edges"].as_array().unwrap().is_empty(),
        "the generated class adds NO edge: {model}"
    );
}

/// Scenario "config requires enter the production graph" (D5) in its M1
/// proof shape: the config→base edge crosses a forbidden boundary and turns
/// verify RED; the documented spec fix — the allow row — greens it. This is
/// the red-risk the CHANGELOG states first.
#[test]
fn config_require_edge_reddens_a_forbidden_boundary_and_greens_with_the_allow_row() {
    let tree = ts_tree(&[
        (
            "jest.config.cjs",
            "const base = require('./jest.config.base.cjs');\nmodule.exports = { ...base };\n",
        ),
        ("jest.config.base.cjs", "module.exports = { testEnvironment: 'node' };\n"),
    ]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"jest.config.cjs\"] }\n\n[module.allowed]\nforbidden = [\"base\"]\n\n[[module]]\nname = \"base\"\nmatches = { modules = [\"jest.config.base.cjs\"] }\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "the config require edge enters the production graph and crosses the forbidden boundary (D5): {}\n{}",
        common::stdout(&verify),
        common::stderr(&verify)
    );

    // The documented absorption: the allow row (depend_on) names the edge.
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"config\"\nmatches = { modules = [\"jest.config.cjs\"] }\n\n[module.allowed]\ndepend_on = [\"base\"]\n\n[[module]]\nname = \"base\"\nmatches = { modules = [\"jest.config.base.cjs\"] }\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
    );
    let verify = tree.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "one allow row absorbs the new edge — the spec-fix path (D5/D8): {}\n{}",
        common::stdout(&verify),
        common::stderr(&verify)
    );
}

// ── orphan file census (workplan workplan_archspec_unreferenced_census) ─────
//
// Law: two report-only stderr line classes on the ts scan report surface —
// `orphan file: <path>` per walk-set file the model lists NO fact for, and
// one `orphan file census:` summary (never at zero). A fact is membership
// in exactly three sets: an endpoint (EITHER end) of `module_edges`, a key
// of `module_external`, or a key of `dynamic_import_wildcards` (PL2). The
// export-symbol tier (`module_public_types`) is NOT a fact set. The lines
// ride the transient, serde-skipped `report_only_lines` channel: never the
// model JSON, never stdout or any artefact, never other drivers (D5/D8).
// The census computes STRICTLY AFTER the named tripwire and never feeds
// its stated-fact arms (D7/B1).

/// Scenario "an unwired ts file states itself orphan": the connected pair
/// carries facts, `src/dead.ts` carries none — one orphan line, one census
/// line whose count equals the orphan-line count, byte-identical model.
#[test]
fn unwired_ts_file_states_itself_orphan() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { b } from \"./b\";\nexport const a = b + 1;\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
        ("src/dead.ts", "export const dead = 42;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "the census is report-only, never red: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains("orphan file: src/dead.ts\n"),
        "the unwired file states itself orphan:\n{stderr}"
    );
    let orphan_lines = stderr
        .lines()
        .filter(|line| line.starts_with("orphan file: "))
        .count();
    assert_eq!(orphan_lines, 1, "exactly one orphan line:\n{stderr}");
    let census_lines: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("orphan file census: "))
        .collect();
    assert_eq!(
        census_lines,
        vec!["orphan file census: 1 of 3 scanned files — the model lists no fact for any of them"],
        "one census line, exact bytes, count equals the orphan-line count:\n{stderr}"
    );
    let stdout = common::stdout(&scan);
    assert!(
        !stdout.contains("orphan"),
        "the machine body carries no census bytes (D5): stdout leaked"
    );
}

/// Scenario "an orphan line needs BOTH zeros": referencing, referenced and
/// external-importing files all carry a fact and stay quiet. `c.ts` —
/// unreferenced but referencing — is honestly NOT flagged (zero-fact, not
/// zero-in-degree, D2); `e.ts` — its fact is the external tier — is not
/// flagged (the model lists it).
#[test]
fn orphan_line_needs_both_zeros() {
    let tree = ts_tree(&[
        ("src/a.ts", "export const a = 1;\n"),
        ("src/b.ts", "import { a } from \"./a\";\nexport const b = a + 1;\n"),
        (
            "src/c.ts",
            "import { d } from \"./d\";\nexport const c = d + 1;\n",
        ),
        ("src/d.ts", "export const d = 1;\n"),
        ("src/e.ts", "import _ from \"lodash\";\nexport const e = _.VERSION;\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    for file in ["a", "b", "c", "d", "e"] {
        assert!(
            !stderr.contains(&format!("orphan file: src/{file}.ts")),
            "src/{file}.ts carries a model fact and must stay unflagged:\n{stderr}"
        );
    }
    assert!(
        !stderr.contains("orphan file census:"),
        "no orphan lines means no census line:\n{stderr}"
    );
}

/// Scenario "census never fires at zero, per-file lines ride sorted order":
/// two orphans print sorted before exactly one census line; a clean tree
/// prints neither class (the java-census never-fires-at-zero law).
#[test]
fn census_never_fires_at_zero_and_orphan_lines_sort() {
    let tree = ts_tree(&[
        (
            "src/a.ts",
            "import { b } from \"./b\";\nexport const a = b + 1;\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
        ("src/zed.ts", "export const zed = 1;\n"),
        ("src/alpha.ts", "export const alpha = 1;\n"),
    ]);
    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "{}", common::stderr(&scan));
    let stderr = common::stderr(&scan);
    let lines: Vec<&str> = stderr.lines().collect();
    let orphan_lines: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| line.starts_with("orphan file: "))
        .collect();
    assert_eq!(
        orphan_lines,
        vec!["orphan file: src/alpha.ts", "orphan file: src/zed.ts"],
        "orphan lines are sorted by path:\n{stderr}"
    );
    let census_at = lines
        .iter()
        .position(|line| line.starts_with("orphan file census: "));
    let last_orphan_at = lines
        .iter()
        .rposition(|line| line.starts_with("orphan file: "));
    assert!(
        census_at == Some(lines.len() - 1),
        "exactly one census line closes the run:\n{stderr}"
    );
    assert!(
        last_orphan_at.is_some_and(|last| last < lines.len() - 1),
        "the orphan lines precede the closing census line:\n{stderr}"
    );

    let clean = ts_tree(&[
        (
            "src/a.ts",
            "import { b } from \"./b\";\nexport const a = b + 1;\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
    ]);
    let scan = clean.run(&["scan", "."]);
    assert!(scan.status.success(), "{}", common::stderr(&scan));
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("orphan file"),
        "a tree with no orphans prints neither class (never fires at zero):\n{stderr}"
    );
}

/// Scenario "verify states orphans on the report surface under both
/// verdicts" (D6, the shipped report-surface law): the orphan lines print
/// before adjudication on the green run and stay on a red run's stderr.
#[test]
fn verify_states_orphans_under_both_verdicts() {
    let spec_green = "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/a.ts\", \"src/b.ts\", \"src/dead.ts\"] }\n";
    let spec_red = "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { modules = [\"src/a.ts\"] }\n\n[module.allowed]\nforbidden = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"src/b.ts\"] }\n\n[[module]]\nname = \"dead\"\nmatches = { modules = [\"src/dead.ts\"] }\n";

    let green = ts_tree(&[
        (
            "src/a.ts",
            "import { b } from \"./b\";\nexport const a = b + 1;\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
        ("src/dead.ts", "export const dead = 42;\n"),
    ]);
    green.write("architecture.spec.toml", spec_green);
    let verify = green.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "green verify failed: {}\n{}",
        common::stdout(&verify),
        common::stderr(&verify)
    );
    let stderr = common::stderr(&verify);
    assert!(
        stderr.contains("orphan file: src/dead.ts"),
        "the green run carries the orphan line on stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("orphan file census:"),
        "the green run carries the census line:\n{stderr}"
    );

    let red = ts_tree(&[
        (
            "src/a.ts",
            "import { b } from \"./b\";\nexport const a = b + 1;\n",
        ),
        ("src/b.ts", "export const b = 1;\n"),
        ("src/dead.ts", "export const dead = 42;\n"),
    ]);
    red.write("architecture.spec.toml", spec_red);
    let verify = red.run(&["verify", "."]);
    assert!(
        verify.status.code() != Some(0),
        "the forbidden a→b ban over the a-imports-b tree must verify red: {}",
        common::stdout(&verify)
    );
    let stderr = common::stderr(&verify);
    assert!(
        stderr.contains("orphan file: src/dead.ts"),
        "a red verdict never suppresses the orphan lines (D6):\n{stderr}"
    );
}

/// Scenario "an aborted scan states no census": the report surface runs
/// only on a produced model, so a loud extraction failure prints no orphan
/// or census line even though every file is orphan-shaped.
#[test]
fn aborted_scan_states_no_census() {
    let tree = ts_tree(&[(
        "src/a.ts",
        "import { x } from \"./missing\";\nexport const a = x;\n",
    )]);
    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "the unresolvable import fails loud: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains("unresolvable import"),
        "the resolution error stays the message:\n{stderr}"
    );
    assert!(
        !stderr.contains("orphan file"),
        "an aborted scan states no census:\n{stderr}"
    );
}

/// PL2 symbol-tier precision: a file that exports public types yet carries
/// no fact-set membership IS listed — the export-symbol tier is NOT one of
/// the three fact sets, so presence there neither suppresses an orphan
/// line nor is denied by one.
#[test]
fn public_types_membership_is_not_a_fact_set() {
    let tree = ts_tree(&[(
        "src/types.ts",
        "export interface Orphaned { id: number }\n",
    )]);
    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "{}", common::stderr(&scan));
    let model = model(&scan);
    assert!(
        model["module_public_types"]["src/types.ts"].is_array(),
        "the fixture must carry the export-symbol tier: {model}"
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains("orphan file: src/types.ts"),
        "the symbol tier never suppresses the census line (PL2):\n{stderr}"
    );
}

/// The wildcard arm of the fact sets: a file whose only fact is a computed
/// require (a `dynamic_import_wildcards` key) is listed by the model and
/// stays unflagged.
#[test]
fn wildcard_attributed_file_carries_a_fact_not_an_orphan() {
    let tree = ts_tree(&[(
        "src/loader.ts",
        "export function load(workerData: { modulePath: string }) {\n  return require(workerData.modulePath);\n}\n",
    )]);
    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a computed require stays green (require-tracing M3): {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains("dynamic import wildcard: src/loader.ts ->"),
        "the wildcard fact stays on its shipped line:\n{stderr}"
    );
    assert!(
        !stderr.contains("orphan file"),
        "the wildcard key carries a fact, not an orphan line:\n{stderr}"
    );
}

/// Scenario "exclusions own the floor": gitignored build output and a spec
/// `excluded_sources` path are outside the walk set, so neither carries an
/// orphan line (D9 — exclusions are the only noise machinery).
#[test]
fn exclusions_own_the_orphan_floor() {
    let tree = ts_tree(&[
        ("src/a.ts", "export const a = 1;\n"),
        ("dist/built.ts", "export const built = 1;\n"),
    ]);
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\nexcluded_sources = [\n  { path = \"scripts\", reason = \"build tooling\" },\n]\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/a.ts\"] }\n",
    );
    tree.write("scripts/tool.ts", "export const tool = 1;\n");

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("orphan file: dist/built.ts")
            && !stderr.contains("orphan file: scripts/tool.ts"),
        "excluded trees are outside the walk set (D9):\n{stderr}"
    );
    assert!(
        stderr.contains("orphan file: src/a.ts"),
        "the non-excluded orphan still states itself (the census is not an exclusion sink):\n{stderr}"
    );
}

/// Scenario "other drivers stay byte-identical": the rust driver states no
/// orphan class at all — the census is ts-only until each driver's own
/// pilot measurement flips its matrix row (D8).
#[test]
fn orphan_lines_never_fire_outside_the_ts_driver() {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "src/lib.rs",
        "pub fn helper() -> u32 {\n    crate::util::base()\n}\n",
    );
    tree.write("src/util.rs", "pub fn base() -> u32 {\n    1\n}\n");
    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "rust scan failed: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        !stderr.contains("orphan file"),
        "the census is emitted by: ts only (D8):\n{stderr}"
    );
}

/// Round-1 test gap 3 (M7, the doc weld): a self-contained framework entry
/// point that imports nothing IS listed by the census, and the
/// `help diagnostics` meaning field carries the entry-point sentence.
#[test]
fn self_contained_entry_point_is_listed_and_the_catalog_names_the_class() {
    let tree = ts_tree(&[(
        "src/routes/home/+page.svelte",
        "<script>let x = 1;</script>\n<h1>{x}</h1>\n",
    )]);
    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "a route page scans green: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr.contains("orphan file: src/routes/home/+page.svelte"),
        "an entry point importing nothing is listed (the residual is disclosed, not excluded, D3):\n{stderr}"
    );

    let help = tree.run(&["help", "diagnostics"]);
    let out = common::stdout(&help);
    assert!(
        out.contains("orphan file"),
        "the catalog carries the orphan-file block:\n{out}"
    );
    assert!(
        out.contains("import NOTHING are listed"),
        "the meaning field carries the M7 entry-point sentence (doc weld):\n{out}"
    );
}
