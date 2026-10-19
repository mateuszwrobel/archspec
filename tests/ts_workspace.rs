//! R2 cells of the ts-resolution completeness record (ADR-024 R5/R6): the
//! workspace registry classifies linked members first-party by REALPATH
//! (precedence case (i), symlink resolution ON per R4), reports the
//! `workspace link missing` line for declared-but-not-linked crossings
//! (cases (ii)/(iii)), decides bare undeclared member-name misses by the
//! case-(iv) linker-family gate (as narrowed round 4: membership edge with
//! the `undeclared member import` line under a hoisting lockfile, honest
//! red under `pnpm-lock.yaml` or no lockfile; member subpaths resolve
//! through the registry MEMBERSHIP context-free (law A, round 6) — a
//! subpath with an existing member-relative target is a first-party edge
//! under every family, a subpath without one stays honest red), feeds the
//! package tier with declared package edges, and
//! reddens package cycles the file tier cannot see (R6's union projection).

mod common;

use common::Fixture;
use serde_json::Value;
use std::path::Path;

fn json_model(out: &std::process::Output) -> Value {
    serde_json::from_str(&common::stdout(out)).expect("scan stdout must be the model JSON")
}

fn link(from: &Path, to_dir: &Path) {
    std::fs::create_dir_all(from.parent().expect("link parent")).expect("link parent dir");
    std::os::unix::fs::symlink(to_dir, from).expect("symlink");
}

fn module_edges(model: &Value) -> Vec<(String, String, String)> {
    let mut rows: Vec<(String, String, String)> = model["module_edges"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            (
                row["unit"].as_str().unwrap_or_default().to_string(),
                row["from"].as_str().unwrap_or_default().to_string(),
                row["to"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    rows.sort();
    rows
}

fn package_edges(model: &Value) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = model["edges"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            (
                row["from"].as_str().unwrap_or_default().to_string(),
                row["to"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    rows.sort();
    rows
}

fn unit_names(model: &Value) -> Vec<String> {
    let mut names: Vec<String> = model["units"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|unit| unit["name"].as_str().unwrap_or_default().to_string())
        .collect();
    names.sort();
    names
}

fn linked_workspace() -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/app/package.json",
        "{\n  \"name\": \"@org/app\",\n  \"dependencies\": { \"@org/ui\": \"workspace:*\" }\n}\n",
    );
    tree.write(
        "packages/app/src/main.ts",
        "import { ui } from \"@org/ui\";\nexport const main = ui;\n",
    );
    tree.write(
        "packages/app/src/utils.ts",
        "export const utils = 1;\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\",\n  \"dependencies\": { \"@org/app\": \"workspace:*\" }\n}\n",
    );
    tree.write(
        "packages/ui/index.ts",
        "import { utils } from \"@org/app/src/utils\";\nexport const ui = utils;\n",
    );
    link(
        &tree.path("node_modules/@org/ui"),
        &tree.path("packages/ui"),
    );
    link(
        &tree.path("node_modules/@org/app"),
        &tree.path("packages/app"),
    );
    tree
}

/// Scenario "workspace links classify first-party" (case (i)): the member
/// import lands on the symlink's realpath inside the member dir, deep and
/// entry imports alike; no workspace name rides the external tier and no
/// report-only line fires.
#[test]
fn linked_workspace_members_classify_first_party_by_realpath() {
    let tree = linked_workspace();
    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(0),
        "linked workspace members resolve first-party: {}",
        common::stderr(&scan)
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/app".to_string(),
            "packages/app/src/main.ts".to_string(),
            "packages/ui/index.ts".to_string()
        )),
        "the linked import is a first-party edge onto the member file: {:?}",
        model["module_edges"]
    );
    assert!(
        module_edges(&model).contains(&(
            "@org/ui".to_string(),
            "packages/ui/index.ts".to_string(),
            "packages/app/src/utils.ts".to_string()
        )),
        "the deep import through the member link is first-party too (case (i))"
    );
    let external: Vec<&str> = model["external"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    assert!(
        !external.contains(&"@org/ui") && !external.contains(&"@org/app"),
        "a workspace name is never an external fact while the link resolves inside the member dir (R5): {external:?}"
    );
    assert!(
        !common::stderr(&scan).contains("workspace link missing"),
        "case (i) owes no report-only line"
    );
}

/// Multi-unit assembly (R5): the root keeps its unit, every member dir is
/// a unit of its own, and a workspace-protocol dependency is a package-level
/// edge beside the file edges.
#[test]
fn a_workspace_tree_is_multi_unit_with_package_level_edges() {
    let tree = linked_workspace();
    let scan = tree.run(&["scan", "."]);
    assert_eq!(scan.status.code(), Some(0));
    let model = json_model(&scan);
    assert_eq!(
        unit_names(&model),
        vec!["@org/app".to_string(), "@org/ui".to_string(), "mono".to_string()],
        "a workspace tree scans MULTI-UNIT: root plus one unit per member"
    );
    assert_eq!(
        package_edges(&model),
        vec![
            ("@org/app".to_string(), "@org/ui".to_string()),
            ("@org/ui".to_string(), "@org/app".to_string()),
        ],
        "the workspace:* declarations are package-level dependency edges (R5/R6)"
    );
    let paths: Vec<&str> = model["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| unit["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"packages/app") && paths.contains(&"packages/ui"));
    for unit in ["@org/app", "@org/ui"] {
        assert!(
            model["unit_manifests"].get(unit).is_some(),
            "each member manifest rides the manifest-fact tier: {unit}"
        );
    }
}

/// Scenario "the collision variant" (case (ii)): a declared member whose
/// import resolves to a registry copy instead of the member dir rides the
/// EXTERNAL classification plus one `workspace link missing` line, exit 0.
#[test]
fn a_declared_member_resolved_to_a_registry_copy_fires_the_link_missing_line() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\"\n}\n",
    );
    tree.write("packages/ui/index.ts", "export const ui = 1;\n");
    tree.write(
        "packages/collide/package.json",
        "{\n  \"name\": \"@org/collide\",\n  \"dependencies\": { \"@org/ui\": \"^1.0.0\" }\n}\n",
    );
    tree.write(
        "packages/collide/main.ts",
        "import { ui } from \"@org/ui\";\nexport const c = ui;\n",
    );
    tree.write(
        "packages/collide/node_modules/@org/ui/package.json",
        "{\n  \"name\": \"other-ui\",\n  \"main\": \"index.js\"\n}\n",
    );
    tree.write(
        "packages/collide/node_modules/@org/ui/index.js",
        "module.exports = {};\n",
    );

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(0),
        "case (ii) is external, not red: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    let line = stderr
        .lines()
        .find(|line| line.starts_with("workspace link missing: @org/ui"))
        .unwrap_or_else(|| panic!("the case-(ii) line is owed:\n{stderr}"));
    assert!(
        line.contains("resolved to") && line.contains("instead of the member dir"),
        "the line spells the resolved path beside the member-dir clause: {line}"
    );
    let model = json_model(&scan);
    assert!(
        model["module_external"]
            .get("packages/collide/main.ts")
            .and_then(|names| names.as_array())
            .is_some_and(|names| names
                .iter()
                .any(|name| name.as_str() == Some("@org/ui"))),
        "the realpath wins: the import rides the external fact (R5 case (ii))"
    );
    assert_eq!(
        package_edges(&model),
        vec![("@org/collide".to_string(), "@org/ui".to_string())],
        "the declared plain range naming a member is a package edge (R6 union source (ii))"
    );
}

/// Scenario N5 variant: an UNDECLARED member-NAME import resolving through
/// a hoisted registry copy rides the plain external fact with NO line.
#[test]
fn an_undeclared_member_name_through_a_hoisted_copy_is_plain_external() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\"\n}\n",
    );
    tree.write("packages/ui/index.ts", "export const ui = 1;\n");
    tree.write(
        "packages/loose/package.json",
        "{\n  \"name\": \"@org/loose\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "packages/loose/main.ts",
        "import { ui } from \"@org/ui\";\nexport const l = ui;\n",
    );
    tree.write(
        "node_modules/@org/ui/package.json",
        "{\n  \"name\": \"other-ui\",\n  \"main\": \"index.js\"\n}\n",
    );
    tree.write("node_modules/@org/ui/index.js", "module.exports = {};\n");

    let scan = tree.run(&["scan", "."]);
    assert_eq!(scan.status.code(), Some(0));
    assert!(
        !common::stderr(&scan).contains("workspace link missing"),
        "no declaration, no owed link (round-2 N5): {}",
        common::stderr(&scan)
    );
    let model = json_model(&scan);
    assert!(
        model["external"]
            .as_array()
            .unwrap()
            .iter()
            .any(|name| name.as_str() == Some("@org/ui")),
        "the hoisted copy is a plain external fact"
    );
}

/// Scenario "a declared member with no link on disk is first-party with
/// the report-only line" (case (iii), round-2 F1(b)): plain range, absent
/// from disk — the declaration adjudicates the first-party edge.
#[test]
fn a_declared_member_absent_from_disk_is_first_party_with_the_line() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\"\n}\n",
    );
    tree.write("packages/ui/index.ts", "export const ui = 1;\n");
    tree.write(
        "packages/edge/package.json",
        "{\n  \"name\": \"@org/edge\",\n  \"dependencies\": { \"@org/ui\": \"^1.0.0\" }\n}\n",
    );
    tree.write(
        "packages/edge/main.ts",
        "import { ui } from \"@org/ui\";\nexport const e = ui;\n",
    );

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(0),
        "the declared member adjudicates first-party, exit 0: {}",
        common::stderr(&scan)
    );
    let stderr = common::stderr(&scan);
    assert!(
        stderr
            .lines()
            .any(|line| line.starts_with("workspace link missing: @org/ui")),
        "case (iii) owes one report-only line: {stderr}"
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/edge".to_string(),
            "packages/edge/main.ts".to_string(),
            "packages/ui/index.ts".to_string()
        )),
        "the first-party edge lands on the member's entry (case (iii)): {:?}",
        model["module_edges"]
    );
}

/// Scenario "an undeclared cross-workspace deep import stays honest"
/// (case (iv), R7d as narrowed round 4): the tree carries NO lockfile —
/// the refusing family — so the linker's omission reddens as
/// `unresolvable import`; the registry conjures no edge. Under a hoisting
/// lockfile the same bare miss rides the case-(iv) membership edge with
/// the report-only line (fixture HOISTED-UNDECLARED below).
#[test]
fn an_undeclared_member_import_is_an_unresolvable_import() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\"\n}\n",
    );
    tree.write("packages/ui/index.ts", "export const ui = 1;\n");
    tree.write(
        "packages/loose/package.json",
        "{\n  \"name\": \"@org/loose\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "packages/loose/main.ts",
        "import { ui } from \"@org/ui\";\nexport const l = ui;\n",
    );

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "case (iv) fails loud like the linker's owner: {}",
        common::stdout(&scan)
    );
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert!(
        text.contains("unresolvable import") && text.contains("\"@org/ui\""),
        "the finding names the specifier verbatim: {text}"
    );
}

/// Fixture HOISTED-UNDECLARED / HOISTED-UNDECLARED-PNPM-TWIN (record R8
/// inventory, round-4 A): one tree — a workspace whose `packages/loose`
/// imports the bare member name `@org/ui` WITHOUT declaring it and with
/// NO on-disk link — differing only in the lockfile NAME at the registry
/// root. The lockfile is the linker-family signal the case-(iv) gate
/// reads; the tree, the registry and the import are byte-identical.
fn undeclared_member_tree(lockfile: Option<&str>) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    if let Some(name) = lockfile {
        tree.write(name, "");
    }
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\"\n}\n",
    );
    tree.write("packages/ui/index.ts", "export const ui = 1;\n");
    tree.write(
        "packages/loose/package.json",
        "{\n  \"name\": \"@org/loose\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "packages/loose/main.ts",
        "import { ui } from \"@org/ui\";\nexport const l = ui;\n",
    );
    tree
}

/// Scenario "a bare undeclared member import under a hoisting lockfile is
/// a first-party edge with the report-only line" (fixture HOISTED-UNDECLARED,
/// R5 case (iv) as narrowed round 4): `yarn.lock` at the registry root
/// names the hoisting family, so the import is a REAL runtime edge — a
/// first-party membership edge onto the first registered member for the
/// name, one `undeclared member import` line, exit 0. The edge and the
/// line are asserted POSITIVELY: if either vanishes from the run this
/// guard fails red (vacuity carrier, R8 law).
#[test]
fn a_bare_undeclared_member_import_under_yarn_lock_is_first_party_with_the_line() {
    let tree = undeclared_member_tree(Some("yarn.lock"));
    let scan = tree.run(&["scan", "."]);
    let stderr = common::stderr(&scan);
    assert_eq!(
        scan.status.code(),
        Some(0),
        "the hoisting-family membership edge passes with a report-only line: {stderr}"
    );
    let line = stderr
        .lines()
        .find(|line| line.starts_with("undeclared member import: "))
        .unwrap_or_else(|| panic!("the case-(iv) report-only line is owed:\n{stderr}"));
    assert_eq!(
        line,
        "undeclared member import: packages/loose/main.ts -> @org/ui",
        "the line names the importing file and the member for which the edge rides"
    );
    assert_eq!(
        stderr
            .lines()
            .filter(|line| line.starts_with("undeclared member import: "))
            .count(),
        1,
        "exactly one report-only line: {stderr}"
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/loose".to_string(),
            "packages/loose/main.ts".to_string(),
            "packages/ui/index.ts".to_string()
        )),
        "the membership edge lands first-party on the member's entry: {:?}",
        model["module_edges"]
    );
    let external: Vec<&str> = model["external"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    assert!(
        !external.contains(&"@org/ui"),
        "the membership edge keeps the name off the external tier: {external:?}"
    );
    let text = format!("{}{stderr}", common::stdout(&scan));
    assert!(
        !text.contains("unresolvable import"),
        "the membership edge leaves no red behind: {text}"
    );
    assert!(
        !stderr.contains("workspace link missing"),
        "an UNDECLARED member import rides the case-(iv) line, never the case-(iii) one"
    );
}

/// Scenario twin (fixture HOISTED-UNDECLARED-PNPM-TWIN): the byte-identical
/// tree with `pnpm-lock.yaml` at the registry root — the refusing family.
/// pnpm-strict denies the undeclared link, so the import keeps the honest
/// `NotFound` red, exit 1: no membership edge, no report-only line.
#[test]
fn the_same_tree_under_pnpm_lock_stays_an_unresolvable_import() {
    let tree = undeclared_member_tree(Some("pnpm-lock.yaml"));
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(1),
        "pnpm-strict refuses the link — red is the runtime truth: {text}"
    );
    assert!(
        text.contains("unresolvable import") && text.contains("\"@org/ui\""),
        "the finding names the specifier verbatim: {text}"
    );
    assert!(
        !text.contains("undeclared member import"),
        "the refusing family emits no case-(iv) line: {text}"
    );
    // The red scan prints no model JSON to stdout (pre-existing red
    // behavior, family-independent), so the no-edge carrier is the
    // combined text: the membership edge would name the member's entry,
    // and the green twin pins the edge's JSON shape positively.
    assert!(
        !text.contains("packages/ui/index.ts"),
        "no membership edge is conjured against the refusing linker: {text}"
    );
    assert!(
        text.contains("unresolvable import") && text.contains("@org/ui"),
        "the honest NotFound red itself is the positive carrier here: {text}"
    );
}

/// Law item "subpath misses (member name + path): unchanged NotFound red,
/// Law A (P-CTX, round 6): a member SUBPATH resolves through the registry
/// MEMBERSHIP, context-free — the importing member's tsconfig and the
/// linker family open no gate here (the family gate stays on the BARE
/// branch only). Fixture CTX-MEMBER-SUBPATH-CARRY: the member-relative
/// target exists, so a first-party membership edge lands and exit is green
/// with NO report-only line (the `undeclared member import` line is the
/// bare family).
#[test]
fn a_member_subpath_resolves_through_the_registry_membership_context_free() {
    let tree = undeclared_member_tree(Some("yarn.lock"));
    tree.write(
        "packages/ui/src/utils.ts",
        "export const utils = 1;\n",
    );
    tree.write(
        "packages/loose/main.ts",
        "import { utils } from \"@org/ui/src/utils\";\nexport const l = utils;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(0),
        "a member subpath with an existing target resolves context-free: {text}"
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/loose".to_string(),
            "packages/loose/main.ts".to_string(),
            "packages/ui/src/utils.ts".to_string()
        )),
        "the membership edge lands on the member-relative file: {:?}",
        model["module_edges"]
    );
    assert!(
        !text.contains("undeclared member import"),
        "the subpath arm emits no report-only line — it is the bare family's: {text}"
    );
}

/// Fixture WS-MEMBER-MANIFEST-DIFFERS (perf lane 2026-10-17, fix 3 memo
/// trap): two members carry DIFFERENT manifests and both are resolved on
/// one scan — alpha declares an existing source `main` (plain edge), beta
/// declares a build-output `main` whose source stands elsewhere (edge plus
/// the `workspace target not built` line). The per-dir manifest memo must
/// never answer one member from the other's parse: either confusion flips
/// one of the two verdicts asserted here.
#[test]
fn two_members_with_differing_manifests_keep_their_own_verdicts() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write("yarn.lock", "");
    tree.write(
        "packages/alpha/package.json",
        "{\n  \"name\": \"@org/alpha\",\n  \"main\": \"src/a.ts\"\n}\n",
    );
    tree.write("packages/alpha/src/a.ts", "export const a = 1;\n");
    tree.write(
        "packages/beta/package.json",
        "{\n  \"name\": \"@org/beta\",\n  \"main\": \"dist/b\"\n}\n",
    );
    tree.write("packages/beta/b.ts", "export const b = 1;\n");
    tree.write(
        "packages/loose/package.json",
        "{\n  \"name\": \"@org/loose\",\n  \"dependencies\": { \"@org/alpha\": \"*\", \"@org/beta\": \"*\" }\n}\n",
    );
    tree.write(
        "packages/loose/main.ts",
        "import { a } from \"@org/alpha\";\nimport { b } from \"@org/beta\";\nexport const v = [a, b];\n",
    );
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(0),
        "both member manifests resolve their own way: {text}"
    );
    let model = json_model(&scan);
    let edges = module_edges(&model);
    assert!(
        edges.contains(&(
            "@org/loose".to_string(),
            "packages/loose/main.ts".to_string(),
            "packages/alpha/src/a.ts".to_string()
        )),
        "alpha's own manifest lands the edge on its declared source: {edges:?}"
    );
    assert!(
        edges.contains(&(
            "@org/loose".to_string(),
            "packages/loose/main.ts".to_string(),
            "packages/beta/b.ts".to_string()
        )),
        "beta's own manifest lands the SOURCE answer under the build-output law: {edges:?}"
    );
    assert!(
        text.contains("workspace target not built: @org/beta -> packages/beta/dist/b"),
        "the not-built line rides beta's manifest only:\n{text}"
    );
}

/// The vacuity twin of the CTX-MEMBER-SUBPATH-CARRY pair: when the
/// member-relative target does NOT exist, law A conjures nothing — the
/// honest `unresolvable import` red stands (a green twin above proves the
/// tree itself is resolvable when the file is there).
#[test]
fn a_member_subpath_without_a_target_stays_an_unresolvable_import() {
    let tree = undeclared_member_tree(Some("yarn.lock"));
    tree.write(
        "packages/loose/main.ts",
        "import { utils } from \"@org/ui/src/utils\";\nexport const l = utils;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a member subpath with no target stays honest: {text}"
    );
    assert!(
        text.contains("unresolvable import") && text.contains("\"@org/ui/src/utils\""),
        "the finding names the specifier verbatim: {text}"
    );
    assert!(
        !text.contains("undeclared member import"),
        "the bare-only line never rides a subpath: {text}"
    );
}

/// Fixture CTX-MEMBER-BARE-FROM-UNDECLARED (law A, round 6 P-CTX): a BARE
/// member import from an UNDECLARED importer under a REFUSING family (no
/// lockfile) — the target member's own declaration (its `main` field)
/// resolves the name context-free, so the round-4 NotFound red leaves and
/// a first-party edge lands with NO report-only line (the attribution
/// line is the hoisting family's). Before round 6 this exact tree reddened
/// (the declaration gate demoted it).
#[test]
fn ctx_member_bare_from_undeclared_importer_resolves_through_the_targets_declaration() {
    let tree = undeclared_member_tree(None);
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\",\n  \"main\": \"./index.ts\"\n}\n",
    );
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(0),
        "the target member's declaration carries the name context-free: {text}"
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/loose".to_string(),
            "packages/loose/main.ts".to_string(),
            "packages/ui/index.ts".to_string()
        )),
        "the membership edge lands on the member's source entry: {:?}",
        model["module_edges"]
    );
    assert!(
        !text.contains("unresolvable import") && !text.contains("undeclared member import"),
        "a resolved import prints neither an unresolvable finding nor the hoisting-only line: {text}"
    );
}

/// Fixture CTX-EXPORTS-WILDCARD-SUBPATH (law A, round 6 P-CTX): a
/// member-named subpath whose spelled remainder the TARGET member maps
/// through its `exports` wildcard (`./*` → `./src/*`) under the R3
/// condition set. The registry membership decides under every family —
/// this tree is UNLINKED (no node_modules, no lockfile), so ONLY the
/// archspec-side exports derivation can carry it.
#[test]
fn ctx_exports_wildcard_subpath_resolves_context_free_in_an_unlinked_tree() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\",\n  \"exports\": { \".\": \"./index.ts\", \"./*\": \"./src/*\" }\n}\n",
    );
    tree.write("packages/ui/index.ts", "export const ui = 1;\n");
    tree.write("packages/ui/src/utils.ts", "export const utils = 1;\n");
    tree.write(
        "packages/loose/package.json",
        "{\n  \"name\": \"@org/loose\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "packages/loose/main.ts",
        "import { utils } from \"@org/ui/utils\";\nexport const l = utils;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(0),
        "the target member's exports wildcard maps the subpath context-free: {text}"
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/loose".to_string(),
            "packages/loose/main.ts".to_string(),
            "packages/ui/src/utils.ts".to_string()
        )),
        "the wildcard-mapped subpath edge lands on the member source: {:?}",
        model["module_edges"]
    );
    assert!(
        !text.contains("unresolvable import"),
        "no unresolvable finding survives the exports derivation: {text}"
    );
}

/// Fixture DIST-TYPES-CONDITION-WINS (law B, round 6 P-DIST): the member
/// exports a `types` condition naming a TS SOURCE next to a BUILT default
/// under `dist` — the member edge NEVER targets build output, so the edge
/// rides the types-condition source and the built default raises no line
/// (the declared target is present; the line is the ABSENT-target carrier).
#[test]
fn dist_types_condition_wins_over_a_built_default() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/app/package.json",
        "{\n  \"name\": \"@org/app\",\n  \"dependencies\": { \"@org/ui\": \"workspace:*\" }\n}\n",
    );
    tree.write(
        "packages/app/src/main.ts",
        "import { ui } from \"@org/ui\";\nexport const main = ui;\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\",\n  \"exports\": { \".\": { \"types\": \"./src/index.ts\", \"default\": \"./dist/index.js\" } }\n}\n",
    );
    tree.write("packages/ui/src/index.ts", "export const ui = 1;\n");
    tree.write("packages/ui/dist/index.js", "export const ui = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(0),
        "the types-condition source carries the member edge: {text}"
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/app".to_string(),
            "packages/app/src/main.ts".to_string(),
            "packages/ui/src/index.ts".to_string()
        )),
        "the edge lands on the types-condition SOURCE, never the built default: {:?}",
        model["module_edges"]
    );
    assert!(
        !text.contains("workspace target not built"),
        "a PRESENT declared target raises no not-built line: {text}"
    );
}

/// Fixture DIST-STRIP-MAIN (law B, round 6 P-DIST): `main` declares
/// `./dist/index.js` and dist is NOT built — the prefix-stripped candidate
/// (the member-root source) carries the edge, and the run prints the
/// report-only `workspace target not built` line naming the ABSENT
/// declared target, exit 0.
#[test]
fn dist_strip_main_lands_on_source_with_the_not_built_line() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/app/package.json",
        "{\n  \"name\": \"@org/app\",\n  \"dependencies\": { \"@org/ui\": \"workspace:*\" }\n}\n",
    );
    tree.write(
        "packages/app/src/main.ts",
        "import { ui } from \"@org/ui\";\nexport const main = ui;\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\",\n  \"main\": \"./dist/index.js\"\n}\n",
    );
    tree.write("packages/ui/index.js", "export const ui = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(0),
        "the stripped source carries the edge while dist stays unbuilt: {text}"
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/app".to_string(),
            "packages/app/src/main.ts".to_string(),
            "packages/ui/index.js".to_string()
        )),
        "the edge lands on the prefix-stripped member source: {:?}",
        model["module_edges"]
    );
    let line = text
        .lines()
        .find(|line| line.starts_with("workspace target not built: "))
        .expect("the absent declared target raises the report-only line");
    assert!(
        line.contains("@org/ui") && line.contains("packages/ui/dist/index.js"),
        "the line names the member and the declared target: {line}"
    );
}

/// Scenario "a package cycle reddens with no file-level SCC": the linked
/// fixture's file graph has no file SCC — both return edges land on leaf
/// files — yet the claims tier adjudicates the package SCC over the
/// resolved-ownership projection, exit 1.
#[test]
fn a_linked_package_cycle_reds_although_no_file_scc_exists() {
    let tree = linked_workspace();
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"@org/app\"], modules = [\"packages/app/**\"] }\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"@org/ui\"], modules = [\"packages/ui/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\nscope = \"claims\"\n",
    );
    let verify = tree.run(&["verify", "."]);
    let text = format!(
        "{}{}",
        common::stdout(&verify),
        common::stderr(&verify)
    );
    assert_eq!(
        verify.status.code(),
        Some(1),
        "the package SCC is red although the file tier sees no SCC: {text}"
    );
    assert!(
        text.contains("cycle") && text.contains("app") && text.contains("ui"),
        "one cycle finding naming both packages: {text}"
    );
}

/// Scenario "a declared unlinked package cycle reddens through the union
/// projection" (R6, round-2 N2): mutual plain-range declarations with
/// registry copies installed and NO links — the declared package edges
/// close the cycle the resolved edges alone cannot form.
#[test]
fn a_declared_unlinked_package_cycle_reds_through_the_union_projection() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    for (name, other) in [("a", "@org/b"), ("b", "@org/a")] {
        tree.write(
            &format!("packages/{name}/package.json"),
            &format!(
                "{{\n  \"name\": \"@org/{name}\",\n  \"dependencies\": {{ \"{other}\": \"^1.0.0\" }}\n}}\n"
            ),
        );
        tree.write(
            &format!("packages/{name}/index.ts"),
            &format!("import x from \"{other}\";\nexport const v = x;\n"),
        );
        tree.write(
            &format!("packages/{name}/node_modules/{other}/package.json"),
            "{\n  \"name\": \"registry-copy\",\n  \"main\": \"index.js\"\n}\n",
        );
        tree.write(
            &format!("packages/{name}/node_modules/{other}/index.js"),
            "module.exports = {};\n",
        );
    }
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"@org/a\"], modules = [\"packages/a/**\"] }\n\n[[module]]\nname = \"b\"\nmatches = { units = [\"@org/b\"], modules = [\"packages/b/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\nscope = \"claims\"\n",
    );
    let verify = tree.run(&["verify", "."]);
    let text = format!(
        "{}{}",
        common::stdout(&verify),
        common::stderr(&verify)
    );
    assert_eq!(
        verify.status.code(),
        Some(1),
        "the declared cycle reddens over registry copies (R6 union): {text}"
    );
    assert!(
        text.contains("workspace link missing: @org/a")
            && text.contains("workspace link missing: @org/b"),
        "the report-only line rides each involved import: {text}"
    );
}

/// Scenario "workspace protocol dependencies become package-level edges"
/// for the pnpm spelling: membership from `pnpm-workspace.yaml` drives the
/// same registry, no npm manifest field needed.
#[test]
fn a_pnpm_workspace_file_registers_members() {
    let tree = Fixture::new();
    tree.write("package.json", "{\n  \"name\": \"mono\"\n}\n");
    tree.write(
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\n",
    );
    tree.write(
        "packages/app/package.json",
        "{\n  \"name\": \"@org/app\",\n  \"dependencies\": { \"@org/ui\": \"link:../ui\" }\n}\n",
    );
    tree.write(
        "packages/app/main.ts",
        "import { ui } from \"@org/ui\";\nexport const m = ui;\n",
    );
    tree.write(
        "packages/ui/package.json",
        "{\n  \"name\": \"@org/ui\"\n}\n",
    );
    tree.write("packages/ui/index.ts", "export const ui = 1;\n");

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(0),
        "the pnpm registry declares @org/ui as a member — case (iii) adjudicates: {}",
        common::stderr(&scan)
    );
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "@org/app".to_string(),
            "packages/app/main.ts".to_string(),
            "packages/ui/index.ts".to_string()
        )),
        "the link: declaration names the member and the edge lands first-party: {:?}",
        model["module_edges"]
    );
    assert_eq!(
        package_edges(&model),
        vec![("@org/app".to_string(), "@org/ui".to_string())]
    );
}

/// Root files stay attributed to the outer root unit (the root's own
/// sources are no member's territory), and the single-package dialect
/// stays byte-identical when no registry declares members.
#[test]
fn root_files_attribute_to_the_root_unit_and_single_package_trees_unchanged() {
    let tree = linked_workspace();
    tree.write(
        "tools/root-tool.ts",
        "import { ui } from \"@org/ui\";\nexport const t = ui;\n",
    );
    let scan = tree.run(&["scan", "."]);
    assert_eq!(scan.status.code(), Some(0));
    let model = json_model(&scan);
    assert!(
        module_edges(&model).contains(&(
            "mono".to_string(),
            "tools/root-tool.ts".to_string(),
            "packages/ui/index.ts".to_string()
        )),
        "a root file attributes to the root unit: {:?}",
        model["module_edges"]
    );

    let plain = Fixture::new();
    plain.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": { \"lodash\": \"^4\" }\n}\n",
    );
    plain.write("src/a.ts", "import _ from \"lodash\";\nexport const a = _;\n");
    let scan = plain.run(&["scan", "."]);
    assert_eq!(scan.status.code(), Some(0));
    let model = json_model(&scan);
    assert_eq!(
        unit_names(&model),
        vec!["demo-ui".to_string()],
        "no registry — the single-package unit stays alone (R5 compat clause)"
    );
    assert!(model["edges"].as_array().is_some_and(|rows| rows.is_empty()));
}

// ---------------------------------------------------------------------------
// Generated-attribution law (workplan_archspec_generated_attribution.md D1-D3,
// decision-check ledger B2/M1/M2/M5/M6): a member-named SUBPATH import whose
// runtime resolution lands on an ABSENT, gitignored build-output target is
// attributed to the UNIQUE source the winning exports entry declares under
// its `types` arm — one entry, two reads. Exactly one existing candidate
// states an edge plus the `workspace target not built` disclosure (the first
// slot spells the member-qualified specifier) and the
// `generated source not built` line is REPLACED for that import; zero or
// multiple existing candidates keep the gensrc line VERBATIM and state no
// edge. `.d.ts` spellings are never sources (the `types_target` exclusion).
// The probe fires ONLY inside the gensrc branch: built targets, missing
// ignore evidence and non-member paths are byte-identical.
// ---------------------------------------------------------------------------

/// Shared shape: a two-member workspace, the app importing
/// `@org/ui/<subpath>`, ui's exports entry spelled `exports`, `dist/`
/// gitignored at line 2 (the R12 evidence carrier).
fn attribution_tree(exports: &str, subpath: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    tree.write(
        "packages/app/package.json",
        "{\n  \"name\": \"@org/app\",\n  \"dependencies\": { \"@org/ui\": \"workspace:*\" }\n}\n",
    );
    tree.write(
        "packages/app/src/main.ts",
        &format!("import {{ x }} from \"@org/ui/{subpath}\";\nexport const v = x;\n"),
    );
    tree.write(
        "packages/ui/package.json",
        &format!("{{\n  \"name\": \"@org/ui\",\n  \"exports\": {exports}\n}}\n"),
    );
    tree.write(".gitignore", "node_modules/\ndist/\n");
    tree
}

fn attributed_edge(model: &Value, from: &str, to: &str) -> Option<bool> {
    model["module_edges"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .find(|row| row["from"].as_str() == Some(from) && row["to"].as_str() == Some(to))
        .map(|row| row["type_only"].as_bool().unwrap_or(false))
}

/// Scenario "the types-arm subpath import becomes an edge (the miniws flip)"
/// plus D8 (type_only rides the site's own spelling): the unique `types`
/// source carries an ordinary edge, the disclosure line spells the
/// member-qualified specifier, the gensrc line is ABSENT for that import
/// (the M6 double-claim probe), exit 0.
#[test]
fn member_subpath_types_unique_attributes_the_edge_and_replaces_the_line() {
    let tree = attribution_tree(
        "{ \"./gen/map\": { \"types\": \"./src/gen/map.ts\", \"default\": \"./dist/gen/map.js\" } }",
        "gen/map",
    );
    tree.write(
        "packages/ui/src/gen/map.ts",
        "export const map = 1;\n",
    );
    tree.write(
        "packages/app/src/kind.ts",
        "import type { map } from \"@org/ui/gen/map\";\nexport type K = typeof map;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "attribution is report-disclosed, exit stays 0: {text}");
    let model = json_model(&scan);
    assert_eq!(
        attributed_edge(&model, "packages/app/src/main.ts", "packages/ui/src/gen/map.ts"),
        Some(false),
        "the value-import site lands a value edge on the declared SOURCE: {:?}",
        model["module_edges"]
    );
    assert_eq!(
        attributed_edge(&model, "packages/app/src/kind.ts", "packages/ui/src/gen/map.ts"),
        Some(true),
        "the `import type` site lands a type-only edge — D8 inherits the site spelling: {:?}",
        model["module_edges"]
    );
    assert!(
        text
            .lines()
            .any(|line| line
                == "workspace target not built: @org/ui/gen/map -> packages/ui/dist/gen/map.js"),
        "the disclosure line spells the member-qualified specifier and the absent declared target:\n{text}"
    );
    assert!(
        !text.contains("generated source not built: @org/ui/gen/map"),
        "the attributed import states its fact ONCE — the gensrc line is replaced, not doubled (M6):\n{text}"
    );
}

/// Decision-check B2 law (the guard the fleet never exercises — mercato
/// measures ZERO ambiguities, so the FIXTURE is the weld): an array `types`
/// arm whose BOTH spellings exist is MULTIPLE candidates — the probe refuses,
/// the gensrc line states verbatim, the model gains nothing.
#[test]
fn member_subpath_types_array_both_existing_refuses_attribution() {
    let tree = attribution_tree(
        "{ \"./x\": { \"types\": [\"./src/x.ts\", \"./src/x.tsx\"], \"default\": \"./dist/x.js\" } }",
        "x",
    );
    tree.write("packages/ui/src/x.ts", "export const x = 1;\n");
    tree.write("packages/ui/src/x.tsx", "export const x = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "the refusal is the honest report-only line: {text}");
    let model = json_model(&scan);
    assert!(
        module_edges(&model).is_empty(),
        "multiple existing candidates state NO edge — archspec never position-picks (B2): {:?}",
        model["module_edges"]
    );
    assert!(
        text.lines().any(|line| line
            == "generated source not built: @org/ui/x — gitignored .gitignore:2"),
        "the gensrc line states VERBATIM, gitignore-evidence tail included:\n{text}"
    );
    assert!(
        !text.contains("workspace target not built"),
        "a refused probe adds no disclosure line:\n{text}"
    );
}

/// D1 declaration-order expansion without ambiguity: an array arm whose
/// SECOND spelling hits an absent file and whose FIRST exists is exactly
/// ONE existing candidate — the edge attributes to the existing one.
#[test]
fn member_subpath_types_array_one_existing_attributes_in_declaration_order() {
    let tree = attribution_tree(
        "{ \"./x\": { \"types\": [\"./src/x.ts\", \"./src/x.tsx\"], \"default\": \"./dist/x.js\" } }",
        "x",
    );
    tree.write("packages/ui/src/x.ts", "export const x = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "one existing candidate attributes: {text}");
    let model = json_model(&scan);
    assert_eq!(
        attributed_edge(&model, "packages/app/src/main.ts", "packages/ui/src/x.ts"),
        Some(false),
        "the single existing candidate carries the edge: {:?}",
        model["module_edges"]
    );
    assert!(
        text.lines().any(|line| line
            == "workspace target not built: @org/ui/x -> packages/ui/dist/x.js"),
        "the disclosure replaces the gensrc line:\n{text}"
    );
    assert!(
        !text.contains("generated source not built"),
        "no gensrc line survives for an attributed import:\n{text}"
    );
}

/// Zero-candidate class (the disclosed ai-assistant defect — an entry with
/// no `types` condition at all): the gensrc line states verbatim and the
/// probe adds nothing.
#[test]
fn member_subpath_without_types_arm_keeps_the_generated_line() {
    let tree = attribution_tree(
        "{ \"./x\": { \"default\": \"./dist/x.js\" } }",
        "x",
    );
    tree.write("packages/ui/src/x.ts", "export const x = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "the honest line keeps the run green: {text}");
    let model = json_model(&scan);
    assert!(
        module_edges(&model).is_empty(),
        "no types arm — NO candidate exists, no edge is minted: {:?}",
        model["module_edges"]
    );
    assert!(
        text.lines().any(|line| line
            == "generated source not built: @org/ui/x — gitignored .gitignore:2"),
        "the gensrc line states verbatim:\n{text}"
    );
}

/// Zero-candidate class, spelled-arm variant: the `types` arm names an
/// ABSENT source file — zero existing candidates keep the line verbatim.
#[test]
fn member_subpath_types_to_absent_file_keeps_the_generated_line() {
    let tree = attribution_tree(
        "{ \"./x\": { \"types\": \"./src/absent.ts\", \"default\": \"./dist/x.js\" } }",
        "x",
    );
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "an absent source is still report-only: {text}");
    let model = json_model(&scan);
    assert!(
        module_edges(&model).is_empty(),
        "the probe only reads EXISTING files: {:?}",
        model["module_edges"]
    );
    assert!(
        text.lines().any(|line| line
            == "generated source not built: @org/ui/x — gitignored .gitignore:2"),
        "the gensrc line states verbatim:\n{text}"
    );
}

/// Decision-check M5 (the `.d.ts` decoy): a `types` arm naming a committed
/// declaration file — PRESENT on disk — is never a source (the
/// `types_target` exclusion carried into the probe verbatim); the line
/// stays and no edge launders the blind spot into a declaration.
#[test]
fn member_subpath_types_to_dts_keeps_the_generated_line() {
    let tree = attribution_tree(
        "{ \"./x\": { \"types\": \"./dist/x.d.ts\", \"default\": \"./dist/x.js\" } }",
        "x",
    );
    tree.write("packages/ui/dist/x.d.ts", "export declare const x: number;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "the d.ts decoy stays report-only: {text}");
    let model = json_model(&scan);
    assert!(
        module_edges(&model).is_empty(),
        "a declaration file is never an attribution target (M5): {:?}",
        model["module_edges"]
    );
    assert!(
        text.lines().any(|line| line
            == "generated source not built: @org/ui/x — gitignored .gitignore:2"),
        "the gensrc line states verbatim:\n{text}"
    );
}

/// Decision-check M2 (two key-matchings, one specifier): the exact key has
/// NO runtime target and a pattern key carries it — the runtime law skips
/// the types-only exact entry, so the SINGLE match is the pattern key. A
/// naive second lookup ("exact key wins") would point the edge at
/// `src/wrong.ts`; the edge must name the pattern entry's source.
#[test]
fn member_subpath_attribution_reads_the_entry_the_runtime_law_selected() {
    let tree = attribution_tree(
        "{ \"./a/b\": { \"types\": \"./src/wrong.ts\" }, \"./a/*\": { \"types\": \"./src/right/*.ts\", \"default\": \"./dist/a/*.js\" } }",
        "a/b",
    );
    tree.write("packages/ui/src/wrong.ts", "export const wrong = 1;\n");
    tree.write("packages/ui/src/right/b.ts", "export const x = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "the pattern entry attributes cleanly: {text}");
    let model = json_model(&scan);
    assert_eq!(
        attributed_edge(&model, "packages/app/src/main.ts", "packages/ui/src/right/b.ts"),
        Some(false),
        "the edge names the entry the RUNTIME law selected: {:?}",
        model["module_edges"]
    );
    assert!(
        attributed_edge(&model, "packages/app/src/main.ts", "packages/ui/src/wrong.ts").is_none(),
        "a divergent second key-match would land HERE — forbidden (M2): {:?}",
        model["module_edges"]
    );
    assert!(
        text.lines().any(|line| line
            == "workspace target not built: @org/ui/a/b -> packages/ui/dist/a/b.js"),
        "the disclosure carries the same single-match target:\n{text}"
    );
    assert!(
        !text.contains("generated source not built"),
        "the attributed import states its fact once:\n{text}"
    );
}

/// Plan scenario "pattern-key entries resolve their source through the SAME
/// entry": the `*` capture substituted into the `types` spelling is the same
/// capture the runtime target got — a mis-derived capture (whole subpath,
/// empty, head-overlapped) would name an absent file and the attribution
/// would collapse into the line the assertions below forbid.
#[test]
fn member_subpath_pattern_capture_flows_into_the_types_arm() {
    let tree = attribution_tree(
        "{ \"./*\": { \"types\": \"./src/*\", \"default\": \"./dist/*.js\" } }",
        "gen/map",
    );
    tree.write(
        "packages/ui/src/gen/map.ts",
        "export const x = 1;\n",
    );
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "the capture-expanded source carries the edge: {text}");
    let model = json_model(&scan);
    assert_eq!(
        attributed_edge(&model, "packages/app/src/main.ts", "packages/ui/src/gen/map.ts"),
        Some(false),
        "one match, two reads — the capture flows into BOTH spellings: {:?}",
        model["module_edges"]
    );
    assert!(
        text.lines().any(|line| line
            == "workspace target not built: @org/ui/gen/map -> packages/ui/dist/gen/map.js"),
        "the runtime slot carries the SAME capture:\n{text}"
    );
}

/// Placement guard M1 (the probe never re-routes a resolvable specifier):
/// the declared default target EXISTS and is edgeable — the ordinary
/// mapped-target edge lands and NO line of either class rides. This is the
/// byte-identity half: the probe is unreachable where `found` exists.
#[test]
fn member_subpath_with_existing_declared_target_is_untouched() {
    let tree = attribution_tree(
        "{ \"./x\": { \"types\": \"./src/x.ts\", \"default\": \"./lib/x.js\" } }",
        "x",
    );
    tree.write("packages/ui/src/x.ts", "export const x = 1;\n");
    tree.write("packages/ui/lib/x.js", "export const x = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "the built-target edge stays as today: {text}");
    let model = json_model(&scan);
    assert_eq!(
        attributed_edge(&model, "packages/app/src/main.ts", "packages/ui/lib/x.js"),
        Some(false),
        "the EXISTING declared target keeps its edge: {:?}",
        model["module_edges"]
    );
    assert!(
        !text.contains("workspace target not built") && !text.contains("generated source not built"),
        "a resolvable specifier raises no report line of either class:\n{text}"
    );
}

/// Placement guard M1 (the probe never softens a red): an absent target
/// WITHOUT gitignore evidence keeps the `unresolvable import` fail-loud —
/// the probe lives inside the gensrc branch and cannot reach this state.
#[test]
fn member_subpath_absent_without_gitignore_evidence_stays_red() {
    let tree = attribution_tree(
        "{ \"./x\": { \"types\": \"./src/x.ts\", \"default\": \"./dist/x.js\" } }",
        "x",
    );
    tree.write("packages/ui/src/x.ts", "export const x = 1;\n");
    tree.write(".gitignore", "node_modules/\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(1),
        "no ignore evidence keeps the fail-loud red — attribution never softens it: {text}"
    );
    assert!(
        text.contains("unresolvable import"),
        "the ordinary red stands:\n{text}"
    );
}

/// Placement guard N2 (stale-built trees are not the probe's state): the
/// declared default target EXISTS under the mechanically-excluded dist
/// class — the edge law refuses the target and the miss is NOT the
/// absent-generated state, so no attribution can mint a source edge and
/// the verdict stays today's red.
#[test]
fn member_subpath_existing_excluded_target_keeps_todays_verdict() {
    let tree = attribution_tree(
        "{ \"./x\": { \"types\": \"./src/x.ts\", \"default\": \"./dist/x.js\" } }",
        "x",
    );
    tree.write("packages/ui/src/x.ts", "export const x = 1;\n");
    tree.write("packages/ui/dist/x.js", "export const x = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a PRESENT build-output target is today's red, never a widened attribution (N2): {text}"
    );
    assert!(
        !text.contains("workspace target not built") && !text.contains("generated source not built"),
        "the built-state raises neither report line:\n{text}"
    );
}

/// Zero-candidate class, no-entry variant: NO exports entry matched the
/// subpath and the member-dir join with the spelled remainder lands on the
/// gitignored path — the R12 line rides on base evidence with no entry to
/// read, so the probe adds nothing (the census's `generated/*` shape).
#[test]
fn member_subpath_without_any_matching_entry_keeps_the_generated_line() {
    let tree = attribution_tree(
        "{ \".\": { \"types\": \"./src/index.ts\", \"default\": \"./dist/index.js\" } }",
        "generated/report",
    );
    tree.write(".gitignore", "node_modules/\ngenerated/\n");
    tree.write("packages/ui/src/index.ts", "export const x = 1;\n");
    let scan = tree.run(&["scan", "."]);
    let text = format!("{}{}", common::stdout(&scan), common::stderr(&scan));
    assert_eq!(scan.status.code(), Some(0), "the no-entry line keeps the run green: {text}");
    let model = json_model(&scan);
    assert!(
        module_edges(&model).is_empty(),
        "no entry read means NO candidate — path inference is forbidden: {:?}",
        model["module_edges"]
    );
    assert!(
        text.lines().any(|line| line
            == "generated source not built: @org/ui/generated/report — gitignored .gitignore:2"),
        "the base-join gensrc line states verbatim:\n{text}"
    );
}
