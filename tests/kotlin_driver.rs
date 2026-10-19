//! Kotlin driver v1 — the fixture inventory of the kotlin driver design
//! record (`worklog/workplan_archspec_kotlin_driver.md` §Fixture inventory).
//! Edges come from import statements resolved through the package census
//! (K-P3); units are gradle module paths (K-P2); the settings file IS the
//! module list and the dual-settings class fails closed; gradle build
//! scripts carry the dependency facts, the version catalog the alias
//! evidence (K-P4); `.kt` is the only source predicate (K-P1).
//!
//! Every kotlin row ships its EMPTY-evidence variant in the same change
//! (vacuity-carrier law — the carrier keys on the FACTS, never on a row
//! label).

mod common;

use common::Fixture;
use serde_json::Value;

/// The canonical two-module tree: `app` depends on `core` through a
/// `project(":core")` gradle dependency and one import crossing; `core`
/// carries no dependencies of its own.
fn gradle_tree(files: &[(&str, &str)]) -> Fixture {
    let tree = Fixture::new();
    for (path, content) in files {
        tree.write(path, content);
    }
    tree
}

fn model(out: &std::process::Output) -> Value {
    serde_json::from_str(&common::stdout(out)).expect("scan stdout must be the model JSON")
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

fn edge_rows(model: &Value, field: &str) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = model[field]
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

fn external_facts(model: &Value) -> Vec<String> {
    let mut facts: Vec<String> = model["external"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|fact| fact.as_str().unwrap_or_default().to_string())
        .collect();
    facts.sort();
    facts
}

/// The indexed file set as the model shows it: every file that carries a
/// per-file external fact (each fixture source imports one external — the
/// witness channel). An excluded directory indexes NOTHING. Unit-keyed
/// manifest externals (the unit-rooted go-parity keys) are not per-file
/// facts and stay out of the witness.
fn indexed_files(model: &Value) -> Vec<String> {
    let units: Vec<String> = model["units"]
        .as_array()
        .map(|array| {
            array
                .iter()
                .filter_map(|unit| unit["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut files: Vec<String> = model["module_external"]
        .as_object()
        .map(|map| {
            map.keys()
                .filter(|key| !units.iter().any(|unit| unit == *key))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// KT-DETECT-FAMILY (K-P1): the four gradle markers co-occur at one root —
/// the family is ONE family; detection answers kotlin, never ambiguity.
#[test]
fn the_gradle_marker_family_detects_kotlin_without_ambiguity() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("settings.gradle", "// groovy sibling, benign at the family level\n"),
        ("build.gradle.kts", "// root build script\n"),
        ("build.gradle", "// groovy root script, benign at the family level\n"),
        ("app/src/main/kotlin/App.kt", "package app\nimport java.util.Locale\nval x = Locale.ENGLISH\n"),
    ]);

    // The dual-settings class below owns the two-settings-files collision;
    // this tree carries both so detection (not the parse) answers first —
    // detection still answers kotlin exactly like the single-marker tree.
    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "the dual-settings class is fail-closed even when detection answered kotlin:\n{}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan)
            .contains("ambiguous settings file: settings.gradle + settings.gradle.kts — keep one"),
        "the module-list parse must name the collision:\n{}",
        common::stderr(&scan)
    );

    // The one-family root without the collision answers kotlin.
    std::fs::remove_file(tree.path("settings.gradle")).expect("remove groovy settings");
    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        model(&scan)["language"].as_str(),
        Some("kotlin"),
        "the marker family must select the kotlin driver"
    );
}

/// KT-GRAMMAR (K-P8, the pinned community grammar): the `package_header`
/// and `import_header` nodes are the whole package/import surface; an
/// alias is a binding, never a path component.
#[test]
fn grammar_yields_package_and_import_facts_from_the_header_nodes() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        (
            "app/build.gradle.kts",
            "plugins { kotlin(\"jvm\") }\ndependencies {\n    implementation(\"com.example:util:1.0\")\n}\n",
        ),
        (
            "app/src/main/kotlin/p/A.kt",
            "package p.sub\n\nimport com.example.util.Helper as H\n\nval x = H.make()\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    // The alias import still states the fact `com` — the alias never
    // enters the path, the top segment law stays the attribution.
    assert!(
        external_facts(&model(&scan)).contains(&"com".to_string()),
        "the aliased import must reach the external tier:\n{}",
        common::stdout(&scan)
    );
    // The file is a model fact: it appears as a units' territory through
    // the module map, and it carries no first-party edge (no import of a
    // scanned package).
    assert!(
        edge_rows(&model(&scan), "file_edges").is_empty(),
        "an external-only import states no file edge:\n{}",
        common::stdout(&scan)
    );
}

/// KT-KTS-NOT-SOURCE (K-P1): `.kts` scripts are fact-legs, never vertices.
#[test]
fn gradle_scripts_are_fact_legs_never_source_vertices() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        (
            "build.gradle.kts",
            "plugins { kotlin(\"jvm\") }\ndependencies {\n    implementation(\"com.example:util:1.0\")\n}\n",
        ),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        ("app/src/main/kotlin/App.kt", "package app\nimport com.example.util.Helper\nval x = Helper\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let scan_model = model(&scan);
    for (from, to) in edge_rows(&scan_model, "file_edges") {
        for identity in [from, to] {
            assert!(
                identity.ends_with(".kt"),
                "a `.kts` script may never be a file-identity endpoint: {identity}"
            );
        }
    }
    // The root script is still a FACT-LEG: its coordinate dependency
    // reaches the external tier.
    assert!(
        external_facts(&scan_model)
            .contains(&"com.example:util".to_string()),
        "the root build script must still state its dependency fact:\n{}",
        common::stdout(&scan)
    );
}

/// KT-SETTINGS-INCLUDES (K-P2): literal include rows ARE the module list;
/// identity is the gradle path; the nested path joins dotted.
#[test]
fn literal_include_rows_are_the_module_list() {
    let tree = gradle_tree(&[
        (
            "settings.gradle.kts",
            "rootProject.name = \"demo\"\ninclude(\":app\", \":shared\")\ninclude(\":libs:core\")\ninclude(\":tools:cli\")\n",
        ),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies {\n    implementation(project(\":libs:core\"))\n}\n"),
        ("libs/core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        ("shared/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        ("tools/cli/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        (
            "app/src/main/kotlin/p/App.kt",
            "package p\nimport p.Core\nval x = Core\n",
        ),
        (
            "libs/core/src/main/kotlin/p/Core.kt",
            "package p\nobject Core\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let scan_model = model(&scan);
    assert_eq!(
        unit_names(&scan_model),
        ["app", "libs::core", "shared", "tools::cli"],
        "names minus the leading colon, nested paths joined dotted:\n{}",
        common::stdout(&scan)
    );
    // The module-tier dotted fact is MANDATORY for a gradle dependency
    // (K-P2): the project row reaches `depgraph` even before imports.
    assert!(
        edge_rows(&scan_model, "module_edges").contains(&("app".to_string(), "libs::core".to_string())),
        "the gradle-path dependency is itself a module-tier fact:\n{}",
        common::stdout(&scan)
    );
    assert_eq!(
        edge_rows(&scan_model, "edges"),
        vec![("app".to_string(), "libs::core".to_string())],
        "the project dependency is the hard unit edge:\n{}",
        common::stdout(&scan)
    );

    let depgraph = tree.run(&["depgraph", "modules"]);
    assert!(
        depgraph.status.success(),
        "a tree wired by project() rows must reach depgraph with the tier present: {}",
        common::stderr(&depgraph)
    );
}

/// KT-DUAL-SETTINGS (K-P2): both settings files at one root — detection
/// still answers kotlin, the PARSE refuses on the exact string.
#[test]
fn both_settings_files_fail_the_module_list_parse_closed() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("settings.gradle", "include ':app'\n"),
        ("app/src/main/kotlin/App.kt", "package app\nimport java.util.Locale\nval x = Locale.ENGLISH\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "the dual-settings class is a fail-closed run precondition:\n{}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan)
            .contains("ambiguous settings file: settings.gradle + settings.gradle.kts — keep one"),
        "the exact shipped string must name the collision:\n{}",
        common::stderr(&scan)
    );
    // The spec pins the language so VERIFY reaches the same module-list
    // parse: the refusal must fire there too, not only under scan.
    tree.write("architecture.spec.toml", "[project]\nlanguage = \"kotlin\"\n");
    let verify = tree.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "verify runs the same parse and refuses the same way:\n{}",
        common::stderr(&verify)
    );
}

/// KT-NO-SETTINGS (K-P2): a kotlin-pinned root without a settings file —
/// one root unit, the report-only line, exit 0.
#[test]
fn a_root_without_settings_files_answers_one_root_unit_and_a_line() {
    let tree = gradle_tree(&[
        ("architecture.spec.toml", "[project]\nlanguage = \"kotlin\"\n"),
        ("app/src/main/kotlin/App.kt", "package app\nimport java.util.Locale\nval x = Locale.ENGLISH\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let scan_model = model(&scan);
    assert_eq!(
        unit_names(&scan_model),
        ["."],
        "no module list means ONE unit at the scan root:\n{}",
        common::stdout(&scan)
    );
    assert!(
        common::stderr(&scan).contains(
            "no settings file: gradle module list unavailable — one unit at the scan root"
        ),
        "the missing module list must be stated, never silent:\n{}",
        common::stderr(&scan)
    );
}

/// KT-UNRESOLVED-INCLUDE (K-P2): a non-literal include argument names its
/// site and keeps the literals; the shipped vacuous-constraint warning is
/// the backstop for a claim over the unit the list skipped.
#[test]
fn a_logic_driven_include_names_its_site_and_the_claim_backstop_stays_honest() {
    let tree = gradle_tree(&[
        (
            "settings.gradle.kts",
            "include(\":app\")\nval extra = \":ghost\"\ninclude(extra)\n",
        ),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("app/src/main/kotlin/App.kt", "package app\nimport com.example.util.Helper\nval x = Helper\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert!(
        unit_names(&model(&scan)) == ["app"],
        "the literal row stays, the computed row states no unit:\n{}",
        common::stdout(&scan)
    );
    assert!(
        common::stderr(&scan)
            .contains("unresolved settings include: settings.gradle.kts:3 — module list may be incomplete"),
        "the named site must carry file and line:\n{}",
        common::stderr(&scan)
    );

    // The backstop: a claim over a unit the list skipped can never green a
    // file-level read silently — the shipped inspect-level vacuity carrier
    // owns the gap (the C5 line, shared with ts and python).
    let inspect = tree.run(&["inspect"]);
    assert!(
        inspect.status.success(),
        "inspect failed: {}",
        common::stderr(&inspect)
    );
    assert!(
        common::stderr(&inspect)
            .contains("vacuous constraint: inspect.file_level: no file-level edges resolved"),
        "a claim over a skipped unit must report through the vacuous-constraint category:\nstderr={}\nstdout={}",
        common::stderr(&inspect),
        common::stdout(&inspect)
    );
}

/// KT-PROJECTDIR-REMAP (K-P2): the remap moves LOCATION, never IDENTITY.
#[test]
fn a_projectdir_remap_moves_location_never_identity() {
    let tree = gradle_tree(&[
        (
            "settings.gradle.kts",
            "include(\":x\")\nproject(\":x\").projectDir = file(\"vendor/elsewhere\")\n",
        ),
        ("vendor/elsewhere/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        (
            "vendor/elsewhere/src/main/kotlin/p/X.kt",
            "package p\nimport com.example.util.Helper\nval x = Helper\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let scan_model = model(&scan);
    assert_eq!(
        unit_names(&scan_model),
        ["x"],
        "identity stays the gradle path:\n{}",
        common::stdout(&scan)
    );
    assert_eq!(
        scan_model["units"][0]["path"].as_str(),
        Some("vendor/elsewhere"),
        "location follows the remap:\n{}",
        common::stdout(&scan)
    );
}

/// KT-PACKAGE-UNIQUE (K-P3): a unique (package, basename) hit is ONE file
/// edge and the module row carries the symbol.
#[test]
fn a_unique_basename_hit_is_one_file_edge() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\ninclude(\":core\")\n"),
        (
            "app/build.gradle.kts",
            "plugins { kotlin(\"jvm\") }\ndependencies {\n    implementation(project(\":core\"))\n}\n",
        ),
        ("core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        (
            "app/src/main/kotlin/com/x/app/App.kt",
            "package com.x.app\n\nimport com.x.core.Core\n\nval use = Core.name\n",
        ),
        (
            "core/src/main/kotlin/com/x/core/Core.kt",
            "package com.x.core\n\nobject Core { val name = \"core\" }\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        edge_rows(&model(&scan), "file_edges"),
        vec![(
            "app/src/main/kotlin/com/x/app/App.kt".to_string(),
            "core/src/main/kotlin/com/x/core/Core.kt".to_string()
        )],
        "one import, one resolved target, one file edge:\n{}",
        common::stdout(&scan)
    );
    let symbol_rows: Vec<(String, Vec<String>)> = model(&scan)["module_edges"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            (
                format!(
                    "{}->{}",
                    row["from"].as_str().unwrap_or_default(),
                    row["to"].as_str().unwrap_or_default()
                ),
                row["symbols"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|symbol| symbol.as_str().unwrap_or_default().to_string())
                    .collect(),
            )
        })
        .collect();
    assert!(
        symbol_rows
            .iter()
            .any(|(pair, symbols)| pair == "app->core" && symbols.contains(&"Core".to_string())),
        "the module row carries the crossing symbols: {symbol_rows:?}"
    );
}

/// KT-PACKAGE-FILESET / KT-AMBIG (K-P3): the basename is not unique inside
/// the package — the edge is honest over the whole file set, no pick, no
/// invented directory vertex.
#[test]
fn an_ambiguous_basename_resolves_over_the_package_file_set() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":a\")\ninclude(\":b\")\n"),
        ("a/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("b/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        (
            "a/src/main/kotlin/p/Use.kt",
            "package p\nimport p.Base\nval x = Base\n",
        ),
        ("a/src/main/kotlin/p/Base.kt", "package p\nobject Base\n"),
        ("b/src/main/kotlin/p/Base.kt", "package p\nobject Base\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let edges = edge_rows(&model(&scan), "file_edges");
    assert!(
        edges.contains(&(
            "a/src/main/kotlin/p/Use.kt".to_string(),
            "a/src/main/kotlin/p/Base.kt".to_string()
        )),
        "file-set edges over the census — no pick: {edges:?}"
    );
    assert!(
        edges.contains(&(
            "a/src/main/kotlin/p/Use.kt".to_string(),
            "b/src/main/kotlin/p/Base.kt".to_string()
        )),
        "file-set edges over the census — no pick: {edges:?}"
    );
    // The census never invents a directory vertex where package↔dir
    // diverge: every module-edge endpoint names a gradle unit.
    let units = unit_names(&model(&scan));
    for (from, to) in edge_rows(&model(&scan), "module_edges") {
        for endpoint in [from, to] {
            let head = endpoint.split("::").next().unwrap_or_default();
            assert!(
                units.iter().any(|unit| unit.as_str() == head),
                "a module-edge endpoint must name a gradle unit, never a guessed directory: {endpoint}"
            );
        }
    }
}

/// KT-PACKAGE-ABSENT (K-P3): an import whose package no scanned file
/// declares lands in the external tier — the named miss class, never a
/// directory guess.
#[test]
fn an_import_of_an_unseen_package_lands_in_the_external_tier() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        (
            "app/src/main/kotlin/app/App.kt",
            "package app\nimport com.acme.third.Thing\nval x = Thing\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert!(
        edge_rows(&model(&scan), "file_edges").is_empty(),
        "an unseen package states no edge:\n{}",
        common::stdout(&scan)
    );
    assert!(
        external_facts(&model(&scan)).contains(&"com".to_string()),
        "the top-segment law attributes the miss: {:?}\n{}",
        external_facts(&model(&scan)),
        common::stdout(&scan)
    );
}

/// KT-SPLIT-PACKAGE (K-P3): one package declared by two modules is LEGAL
/// Kotlin — the census line states the shape, the edges still land.
#[test]
fn a_split_package_is_legal_and_stated_by_name() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":a\")\ninclude(\":b\")\n"),
        ("a/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("b/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        (
            "a/src/main/kotlin/s/A.kt",
            "package s\nimport s.B\nobject A\n",
        ),
        ("b/src/main/kotlin/s/B.kt", "package s\nobject B\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan)
            .contains("split package: s — declared by 2 modules: a, b"),
        "the census line names the sharing modules:\n{}",
        common::stderr(&scan)
    );
    assert_eq!(
        edge_rows(&model(&scan), "file_edges"),
        vec![(
            "a/src/main/kotlin/s/A.kt".to_string(),
            "b/src/main/kotlin/s/B.kt".to_string()
        )],
        "edges still land across the split:\n{}",
        common::stdout(&scan)
    );
}

/// KT-QUAL (K-P3, the resolution key): the import's basename decides the
/// target; a basename no file carries folds to the whole package file set.
#[test]
fn the_import_basename_selects_between_file_and_file_set() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":core\")\ninclude(\":app\")\n"),
        ("core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        (
            "core/src/main/kotlin/c/Core.kt",
            "package c\nobject Core\n",
        ),
        (
            "core/src/main/kotlin/c/Misc.kt",
            "package c\nobject Misc\n",
        ),
        (
            "app/src/main/kotlin/a/Qualified.kt",
            "package a\nimport c.Core\nval x = Core\n",
        ),
        (
            "app/src/main/kotlin/a/Unqualified.kt",
            "package a\nimport c.Whatever\nval y = Whatever\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let edges = edge_rows(&model(&scan), "file_edges");
    assert!(
        edges.contains(&(
            "app/src/main/kotlin/a/Qualified.kt".to_string(),
            "core/src/main/kotlin/c/Core.kt".to_string()
        )),
        "the unique basename hit is one edge: {edges:?}"
    );
    assert!(
        !edges.contains(&(
            "app/src/main/kotlin/a/Qualified.kt".to_string(),
            "core/src/main/kotlin/c/Misc.kt".to_string()
        )),
        "a unique hit never fans to the package: {edges:?}"
    );
    assert!(
        edges.contains(&(
            "app/src/main/kotlin/a/Unqualified.kt".to_string(),
            "core/src/main/kotlin/c/Core.kt".to_string()
        )) && edges.contains(&(
            "app/src/main/kotlin/a/Unqualified.kt".to_string(),
            "core/src/main/kotlin/c/Misc.kt".to_string()
        )),
        "a miss on every basename folds to the whole package file set: {edges:?}"
    );
}

/// KT-EXTERNAL-TIERS (K-P3, the operator ruling): android, androidx and
/// `uniffi.voice_lib`-family imports are plain EXTERNAL facts — exit 0,
/// no special vocabulary anywhere.
#[test]
fn android_androidx_and_bridge_imports_are_plain_external_facts() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        (
            "app/src/main/kotlin/app/App.kt",
            "package app\n\nimport android.content.Intent\nimport androidx.compose.material3.Text\nimport uniffi.voice_lib.VoiceEngine\n\nval a = Intent()\nval b = Text\nval c = VoiceEngine\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "external attribution is never a verdict: {}",
        common::stderr(&scan)
    );
    let facts = external_facts(&model(&scan));
    for fact in ["android", "androidx", "uniffi"] {
        assert!(
            facts.contains(&fact.to_string()),
            "`{fact}` must state its top-segment external fact: {facts:?}"
        );
    }
}

/// KT-TEST-TIER-PAIR (K-P4): the `*Test.kt` convention AND junit-family
/// evidence carry the tier; naming alone states nothing.
#[test]
fn the_test_tier_co_occurs_and_naming_alone_states_nothing() {
    // Half one: junit evidence — the edge TARGETING the test file never
    // renders as a production edge; the test file stays a node.
    let with_evidence = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\ninclude(\":core\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies {\n    testImplementation(\"org.junit.jupiter:junit-jupiter:5.10.2\")\n}\n"),
        (
            "app/src/main/kotlin/p/Use.kt",
            "package p\nimport p.HelperTest\nval x = HelperTest\n",
        ),
        (
            "core/src/test/kotlin/p/HelperTest.kt",
            "package p\nclass HelperTest\n",
        ),
    ]);
    let scan = with_evidence.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let inspect = with_evidence.run(&["inspect"]);
    assert!(
        inspect.status.success(),
        "inspect failed: {}",
        common::stderr(&inspect)
    );
    let rendered = common::stdout(&inspect);
    assert!(
        rendered.contains("HelperTest.kt"),
        "the test file stays a node: {rendered}"
    );
    assert!(
        !rendered.contains("--> core_src_test_kotlin_p_HelperTest"),
        "a production edge may never TARGET a test-tier file: {rendered}"
    );

    // Half two: the convention-only twin — no junit dependency anywhere,
    // the `*Test.kt` name carries no tier, the edge stays.
    let naming_only = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\ninclude(\":core\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        (
            "app/src/main/kotlin/p/Use.kt",
            "package p\nimport p.HelperTest\nval x = HelperTest\n",
        ),
        (
            "core/src/test/kotlin/p/HelperTest.kt",
            "package p\nclass HelperTest\n",
        ),
    ]);
    let inspect = naming_only.run(&["inspect"]);
    assert!(
        inspect.status.success(),
        "inspect failed: {}",
        common::stderr(&inspect)
    );
    assert!(
        common::stdout(&inspect).contains("--> core_src_test_kotlin_p_HelperTest"),
        "without framework evidence the file is production and the edge stays:\n{}",
        common::stdout(&inspect)
    );
}

/// KT-TEST-EVIDENCE-INLINE (K-P4): the inline GAV shape is evidence the
/// same as the catalog shape — the family predicate reads group or name.
#[test]
fn inline_junit_gav_is_test_tier_evidence() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\ninclude(\":core\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies {\n    testImplementation(\"org.junit:junit:4.13.2\")\n}\n"),
        (
            "app/src/main/kotlin/p/Use.kt",
            "package p\nimport p.HelperTest\nval x = HelperTest\n",
        ),
        ("core/src/test/kotlin/p/HelperTest.kt", "package p\nclass HelperTest\n"),
    ]);

    let inspect = tree.run(&["inspect"]);
    assert!(
        inspect.status.success(),
        "inspect failed: {}",
        common::stderr(&inspect)
    );
    assert!(
        !common::stdout(&inspect).contains("--> core_src_test_kotlin_p_HelperTest"),
        "the inline GAV is evidence (family predicate, inline shape):\n{}",
        common::stdout(&inspect)
    );
}

/// KT-CATALOG-UNREADABLE (K-P4, empty-evidence variant): a malformed
/// version catalog turns the catalog evidence leg OFF and says so — the
/// `*Test.kt` name alone states no tier without resolving to evidence.
#[test]
fn an_unreadable_catalog_degrades_the_test_tier_and_names_itself() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\ninclude(\":core\")\n"),
        ("gradle/libs.versions.toml", "THIS IS NOT TOML {{{\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies {\n    testImplementation(libs.junit)\n}\n"),
        (
            "app/src/main/kotlin/p/Use.kt",
            "package p\nimport p.HelperTest\nval x = HelperTest\n",
        ),
        ("core/src/test/kotlin/p/HelperTest.kt", "package p\nclass HelperTest\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan)
            .contains("gradle catalog unreadable: gradle/libs.versions.toml — test-tier evidence unavailable"),
        "the broken evidence leg must be named:\n{}",
        common::stderr(&scan)
    );
    let inspect = tree.run(&["inspect"]);
    assert!(
        common::stdout(&inspect).contains("--> core_src_test_kotlin_p_HelperTest"),
        "unresolved alias evidence states no tier — the file stays production:\n{}",
        common::stdout(&inspect)
    );
}

/// KT-BUILD-OWN-SET (K-P7): the driver excludes its OWN `<moduleDir>/build`
/// — and NOTHING else; a component's authored `build/` directory stays
/// indexed because the shared mechanical list was not touched.
#[test]
fn the_driver_excludes_own_build_dirs_and_only_own_build_dirs() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("app/src/main/kotlin/p/App.kt", "package p\nimport com.example.util.Helper\nval x = Helper\n"),
        (
            "app/build/generated/kotlin/Gen.kt",
            "package gen\nimport com.example.util.Helper\nval generated = Helper\n",
        ),
        (
            "docs-site/build/authored/Authored.kt",
            "package authored\nimport com.example.util.Helper\nval authored = Helper\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let files = indexed_files(&model(&scan));
    assert!(
        files.iter().any(|file| file == "app/src/main/kotlin/p/App.kt"),
        "the source set is indexed: {files:?}"
    );
    assert!(
        files
            .iter()
            .any(|file| file == "docs-site/build/authored/Authored.kt"),
        "an authored `build/` directory of another component stays indexed: {files:?}"
    );
    assert!(
        !files.iter().any(|file| file.starts_with("app/build/")),
        "the module's own build directory is the driver's own exclusion: {files:?}"
    );
}

/// KT-BUILDDIR-LITERAL (K-P7): a statically-visible relocation replaces the
/// `build` exclusion — the stated path is excluded, the literal `build`
/// name is not.
#[test]
fn a_literal_build_directory_relocation_replaces_the_exclusion() {
    let tree = gradle_tree(&[
        (
            "settings.gradle.kts",
            "include(\":app\")\nsubprojects { layout.buildDirectory.set(file(\"out\")) }\n",
        ),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        (
            "app/build/kept/K.kt",
            "package kept\nimport com.example.util.Helper\nval k = Helper\n",
        ),
        (
            "app/out/dropped/D.kt",
            "package dropped\nimport com.example.util.Helper\nval d = Helper\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let files = indexed_files(&model(&scan));
    assert!(
        files.iter().any(|file| file.starts_with("app/build/")),
        "the relocation moved the exclusion: `build/` is no longer excluded: {files:?}"
    );
    assert!(
        !files.iter().any(|file| file.starts_with("app/out/")),
        "the relocated directory the build states IS the build dir: {files:?}"
    );
}

/// KT-BUILDDIR-PROPERTY (K-P7, the config-duty half): a property-driven
/// relocation is statically invisible — the driver does NOT guess it; the
/// spec's `excluded_sources` entry is the shipped escape hatch.
#[test]
fn a_property_driven_relocation_is_the_excluded_sources_duty() {
    let files: [(&str, &str); 4] = [
        (
            "settings.gradle.kts",
            "include(\":app\")\nsubprojects { layout.buildDirectory.set(file(providedDir)) }\n",
        ),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        (
            "app/out/dropped/D.kt",
            "package dropped\nimport com.example.util.Helper\nval d = Helper\n",
        ),
        (
            "app/src/main/kotlin/p/App.kt",
            "package p\nimport com.example.util.Helper\nval x = Helper\n",
        ),
    ];

    let tree = gradle_tree(&files);
    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let found = indexed_files(&model(&scan));
    assert!(
        found.iter().any(|file| file.starts_with("app/out/")),
        "the driver never guesses a computed path: {found:?}"
    );

    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"kotlin\"\nexcluded_sources = [ { path = \"app/out\", reason = \"relocated build output (property-driven)\" } ]\n",
    );
    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let found = indexed_files(&model(&scan));
    assert!(
        !found.iter().any(|file| file.starts_with("app/out/")),
        "excluded_sources is the shipped config duty and it excludes: {found:?}"
    );
}

/// KT-JAVA-CENSUS (K-P6): `.java` carries REAL code, so its absence from
/// the model is COUNTED — a shape census, no java vertices, no java edges.
#[test]
fn java_sources_are_counted_never_indexed() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("app/src/main/kotlin/p/App.kt", "package p\nimport com.example.util.Helper\nval x = Helper\n"),
        (
            "app/src/main/java/p/Thing.java",
            "package p;\nimport java.util.Locale;\npublic class Thing { Locale l = Locale.ENGLISH; }\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan)
            .contains("java sources not indexed: 1 files — the kotlin driver indexes .kt only"),
        "the census line counts the java shape:\n{}",
        common::stderr(&scan)
    );
    for (from, to) in edge_rows(&model(&scan), "file_edges") {
        assert!(
            !from.ends_with(".java") && !to.ends_with(".java"),
        "the census never indexes a java file: {from} -> {to}"
        );
    }
    assert!(
        !external_facts(&model(&scan)).contains(&"java".to_string()),
        "a java file's imports state no facts: {:?}",
        external_facts(&model(&scan))
    );
    // The zero case: a pure-kotlin tree prints nothing (the census never
    // fires at zero).
    let pure = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("app/src/main/kotlin/p/App.kt", "package p\nimport com.example.util.Helper\nval x = Helper\n"),
    ]);
    let scan = pure.run(&["scan", "."]);
    assert!(scan.status.success(), "scan failed");
    assert!(
        !common::stderr(&scan).contains("java sources not indexed"),
        "the census never fires at zero:\n{}",
        common::stderr(&scan)
    );
}

/// KT-SAME-PKG-BLINDSPOT (K-P6): a same-package reference needs no import
/// and therefore states no edge — verify stays GREEN and the blind spot is
/// stated where the model is read.
#[test]
fn same_package_references_state_no_edge_and_the_blind_spot_is_stated() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("app/src/main/kotlin/p/A.kt", "package p\nobject A\n"),
        (
            "app/src/main/kotlin/p/B.kt",
            "package p\nval use = A\n",
        ),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert!(
        edge_rows(&model(&scan), "file_edges").is_empty(),
        "a same-package reference carries no import and therefore no edge:\n{}",
        common::stdout(&scan)
    );

    // The GREEN proof (the ts R8 precedent): a forbid_submodule_dependency
    // claim over the unit — a same-package reference models NO edge, so the
    // check adjudicates nothing and verify stays GREEN with the named
    // carrier, never a silent pass over invisible edges.
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"kotlin\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app/src/main/kotlin/p/A.kt\", \"app/src/main/kotlin/p/B.kt\"] }\n\n[[constraint]]\ntype = \"forbid_submodule_dependency\"\nparent = \"app\"\nfrom = [\"app::B\"]\nforbid = [\"app::A\"]\n",
    );
    let verify = tree.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "the blind spot keeps verify GREEN — the model states what imports state:\n{}\n{}",
        common::stdout(&verify),
        common::stderr(&verify)
    );

    let help = tree.run(&["help", "languages"]);
    let rendered = common::stdout(&help);
    let collapsed = rendered.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        collapsed.contains(
            "Same-package references need no import in Kotlin. The kotlin driver models edges only from import statements. A vertex can depend on same-package code with no edge in the model."
        ),
        "the model-honesty sentence must be stated where the model is read:\n{rendered}"
    );
}

/// KT-VACUITY (the carrier law): a kotlin-pinned root whose manifests are
/// MISSING (the ts/python C5 law — a script stating an empty dependency
/// set states a fact) must not scan to a silent green.
#[test]
fn a_tree_whose_scripts_state_no_dependencies_names_its_vacuity() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/src/main/kotlin/p/App.kt", "package p\nimport java.util.Locale\nval x = Locale.ENGLISH\n"),
    ]);

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

/// KT-BOUND (K-P5, amended): the scan-time boundary advisory — a declared
/// zone whose package's members sit in modules outside the zone states a
/// report-only line and never a finding.
#[test]
fn a_zone_that_splits_a_package_advises_without_a_verdict() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\ninclude(\":core\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("core/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { }\n"),
        ("app/src/main/kotlin/z/Home.kt", "package z\nobject Home\n"),
        ("app/src/main/kotlin/z/Also.kt", "package z\nobject Also\n"),
        ("core/src/main/kotlin/z/Stray.kt", "package z\nobject Stray\n"),
    ]);

    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"kotlin\"\n\n[[module]]\nname = \"z-zone\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"core\"\nmatches = { units = [\"core\"] }\n",
    );
    let scan = tree.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "the advisory never reddens the scan: {}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan)
            .contains("boundary advisory: zone z-zone splits the package z — 1 member files outside"),
        "the advisory states the split members:\n{}",
        common::stderr(&scan)
    );

    let verify = tree.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "the advisory is not a finding and --strict-gating is not its story:\n{}",
        common::stderr(&verify)
    );
}

/// The tripwire law (the ts/python precedent): a marker-only root states
/// an error, never an empty green.
#[test]
fn a_root_without_kotlin_sources_refuses_loud() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
    ]);

    let scan = tree.run(&["scan", "."]);
    assert_eq!(
        scan.status.code(),
        Some(1),
        "a kotlin root without sources is an error, not a silent empty model:\n{}",
        common::stderr(&scan)
    );
    assert!(
        common::stderr(&scan).contains("no kotlin sources found under:"),
        "the error names the missing sources:\n{}",
        common::stderr(&scan)
    );
}

/// The model-honesty surface weld (K-P6): the sentence reads the same at
/// `help languages` as the driver's const carries it (claim guard
/// `kotlin-model-honesty`).
#[test]
fn the_model_honesty_sentence_is_on_the_help_surface() {
    let tree = gradle_tree(&[
        ("settings.gradle.kts", "include(\":app\")\n"),
        ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\ndependencies { implementation(\"com.example:util:1.0\") }\n"),
        ("app/src/main/kotlin/p/App.kt", "package p\nimport com.example.util.Helper\nval x = Helper\n"),
    ]);
    let help = tree.run(&["help", "languages"]);
    let rendered = common::stdout(&help);
    let flattened = rendered.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flattened.contains("Same-package references need no import in Kotlin. The kotlin driver models edges only from import statements. A vertex can depend on same-package code with no edge in the model."),
        "the welded sentence must read off the surface:\n{rendered}"
    );
}
