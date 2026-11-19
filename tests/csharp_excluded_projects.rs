//! GitHub issue #49 cells — declared `excluded_sources` entries adjudicate
//! at csharp PROJECT discovery: a project whose csproj sits at or under an
//! entry contributes no unit, no unit edge and no module edge, keeps its
//! namespaces as DEAD prefixes (the test-gated law), and states itself with
//! one `excluded unit: <path> — excluded_sources` census line, sorted and
//! never silent (the issue #39 law). Entry liveness is unchanged and
//! driver-independent ("any file counts", `spec.rs`): an entry matching
//! only FILES inside kept projects (the reference-consumer shape) drops no
//! project, prints no census line, and does NOT start the `matched nothing`
//! advisory; a glob-form entry matches no csproj path and keeps today's
//! liveness advisory unchanged. A tree with no entries scans byte-identical
//! to the same tree without a spec. File-level pruning INSIDE a kept
//! project is issue #60 — deliberately not modeled here.

mod common;

use common::Fixture;
use serde_json::Value;

/// The census forms pinned by the claim registry (`csharp-excluded-units`,
/// owner surface: `help diagnostics`). The dedicated guard for those
/// placeholder bytes is this file: the exact-bytes cells below render them.
#[allow(dead_code)]
const CENSUS_FORMS: [&str; 1] = ["excluded unit: <path> — excluded_sources"];

fn lib_csproj() -> &'static str {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n</Project>\n"
}

/// The repro tree: a production `App` declaring a `using` over the
/// `external` project's namespace; `external/Ext.csproj` is the project a
/// declared entry drops. A kept project's folder-level files never enter
/// the project tier — cells pin that file-level entries drop nothing.
fn repro_tree() -> Fixture {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write(
        "App/App.cs",
        "using External.Lib;\nnamespace App { public class Greeter { } }\n",
    );
    fx.write("external/Ext.csproj", lib_csproj());
    fx.write(
        "external/Ext.cs",
        "namespace External.Lib { public class Legacy { } }\n",
    );
    fx
}

fn spec(entries: &[&str], units: &[&str]) -> String {
    let modules = if units.is_empty() {
        String::new()
    } else {
        format!("\n[[module]]\nname = \"app\"\nmatches = {{ units = {:?} }}\n", units)
    };
    format!(
        "[project]\nlanguage = \"csharp\"\n\nexcluded_sources = [{}]\n{}",
        entries.join(", "),
        modules
    )
}

fn unit_names(model: &Value) -> Vec<String> {
    model["units"]
        .as_array()
        .map(|units| {
            units
                .iter()
                .filter_map(|unit| unit["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn census_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("excluded unit: "))
        .map(str::to_string)
        .collect()
}

fn liveness_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("excluded source matched nothing: "))
        .map(str::to_string)
        .collect()
}

/// Mandatory cell (with follow-up cell b): a dir-form entry prunes the
/// whole project — no unit, no unit edge, no module edge, DEAD prefixes
/// keep the `using` from resurrecting it — and the drop prints exactly one
/// exact-bytes census line. The entry stays ACTIVE for liveness (the
/// project's files sit under it), so the `matched nothing` advisory must
/// NOT fire.
#[test]
fn dir_form_entry_prunes_the_whole_project() {
    let fx = repro_tree();
    fx.write(
        "architecture.spec.toml",
        &spec(
            &["{ path = \"external\", reason = \"generated dependency, class P\" }"],
            &["App"],
        ),
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(
        unit_names(&model),
        vec!["App".to_string()],
        "the excluded project names no unit (issue #49)"
    );
    let edges = model["edges"].as_array().expect("edges is a list");
    for edge in edges {
        assert_ne!(edge["from"].as_str(), Some("Ext"));
        assert_ne!(edge["to"].as_str(), Some("Ext"));
    }
    let module_edges = serde_json::to_string(&model["module_edges"]).expect("module_edges is JSON");
    assert!(
        !module_edges.contains("\"Ext\""),
        "a dropped project names no module edge: {module_edges}"
    );
    assert!(
        !common::stdout(&scan).contains("External::Lib"),
        "the DEAD-prefix law: App's `using External.Lib` projects no phantom module\n{}",
        common::stdout(&scan)
    );
    assert_eq!(
        census_lines(&scan),
        vec!["excluded unit: external — excluded_sources".to_string()],
        "exactly one census line, exact bytes (never-silent law):\n{}",
        common::stderr(&scan)
    );
    assert!(
        liveness_lines(&scan).is_empty(),
        "a project dir entry keeps its liveness through the project's files:\n{}",
        common::stderr(&scan)
    );
}

/// Mandatory cell: an excluded project no longer trips the verify
/// `unexpected component` finding — the repro's verify rc=1 arm flips to
/// green because the unit is gone from the model.
#[test]
fn excluded_project_no_longer_fails_verify_as_unexpected_component() {
    let fx = repro_tree();
    fx.write(
        "architecture.spec.toml",
        &spec(
            &["{ path = \"external\", reason = \"generated dependency, class P\" }"],
            &["App"],
        ),
    );
    let verify = fx.run(&["verify", "."]);
    let stderr = common::stderr(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "verify is green once the declared exclusion reaches project discovery:\n{stderr}"
    );
    assert!(
        !stderr.contains("unexpected component"),
        "the dropped project is no longer an unexpected component:\n{stderr}"
    );
    assert!(
        census_lines(&verify)
            == vec!["excluded unit: external — excluded_sources".to_string()],
        "the census line rides the verify report surface too:\n{stderr}"
    );
}

/// Mandatory cell: a tree with no entries — no spec at all, and a spec
/// with an empty table — changes zero model bytes and prints no census
/// byte anywhere.
#[test]
fn no_entries_scans_byte_identical() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write("App/App.cs", "namespace App { public class Greeter { } }\n");
    fx.write("Lib/Lib.csproj", lib_csproj());
    fx.write("Lib/Lib.cs", "namespace App.Lib { public class Core { } }\n");
    let bare = fx.run(&["scan", "."]);
    assert!(
        bare.status.success(),
        "bare scan failed: {}",
        common::stderr(&bare)
    );
    fx.write("architecture.spec.toml", &spec(&[], &["App", "Lib"]));
    let empty = fx.run(&["scan", "."]);
    assert!(
        empty.status.success(),
        "empty-entry scan failed: {}",
        common::stderr(&empty)
    );
    assert_eq!(
        common::stdout(&bare),
        common::stdout(&empty),
        "an empty `excluded_sources` table changes zero bytes of the machine body"
    );
    for output in [&bare, &empty] {
        let all = format!("{}{}", common::stdout(output), common::stderr(output));
        for marker in ["excluded unit:", "matched nothing"] {
            assert!(
                !all.contains(marker),
                "a non-declaring tree never renders `{marker}`:\n{all}"
            );
        }
    }
}

/// Mandatory cell (follow-up c): a truly-empty entry (no file, no csproj
/// anywhere under it) keeps today's zero-match liveness advisory unchanged
/// and drops nothing.
#[test]
fn truly_empty_entry_keeps_the_matched_nothing_advisory() {
    let fx = repro_tree();
    fx.write(
        "architecture.spec.toml",
        &spec(
            &["{ path = \"ghost\", reason = \"path no longer on disk\" }"],
            &["App", "Ext"],
        ),
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(
        unit_names(&model),
        vec!["App".to_string(), "Ext".to_string()],
        "an entry matching no csproj drops no project"
    );
    assert!(census_lines(&scan).is_empty());
    assert_eq!(
        liveness_lines(&scan),
        vec!["excluded source matched nothing: ghost — path no longer on disk".to_string()],
        "the liveness advisory is unchanged for an entry matching nothing:\n{}",
        common::stderr(&scan)
    );
}

/// Mandatory cell (follow-up a, the exact reference-consumer shape): an
/// entry that matches ONLY files inside kept projects — never a csproj
/// path — adds zero new stdout/stderr rows and keeps the language census
/// quieted through the driver-independent route. Deleting the entry flips
/// verify RED with the language mismatch: the entry is active, just not at
/// the project tier.
#[test]
fn file_only_entry_adds_no_rows_and_keeps_the_census_quieted() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write("App/App.cs", "namespace App { public class Greeter { } }\n");
    fx.write("e2e/smoke.spec.ts", "test('smoke', async () => {});\n");
    fx.write(
        "architecture.spec.toml",
        &spec(
            &["{ path = \"e2e\", reason = \"playwright harness, not csharp\" }"],
            &["App"],
        ),
    );
    let verify = fx.run(&["verify", "."]);
    let stderr = common::stderr(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "the file-level entry keeps quieting the census exactly as today:\n{stderr}"
    );
    for marker in ["excluded unit:", "matched nothing"] {
        assert!(
            !stderr.contains(marker),
            "a file-only entry adds no `{marker}` row:\n{stderr}"
        );
    }
    // The consumer fact, pinned from the other side: the entry is the
    // census's only quieting — without it the same tree is RED.
    fx.write("architecture.spec.toml", &spec(&[], &["App"]));
    let without = fx.run(&["verify", "."]);
    assert_ne!(
        without.status.code(),
        Some(0),
        "deleting the entry flips verify RED — the census consult is real"
    );
    assert!(
        common::stderr(&without).contains("language mismatch: csharp vs ts"),
        "the negative arm flips with the language-mismatch error:\n{}",
        common::stderr(&without)
    );
}

/// Mandatory cell (follow-up b is folded into cell 1): a project-dir entry
/// keeps liveness ACTIVE while it prunes — asserted there; this cell pins
/// the census determinism instead. Two excluded projects print exactly two
/// census lines, sorted by line, one per project.
#[test]
fn census_is_sorted_one_line_per_excluded_project() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write("App/App.cs", "namespace App { public class Greeter { } }\n");
    fx.write("vendor/Tools/Tools.csproj", lib_csproj());
    fx.write("vendor/Tools/Tools.cs", "namespace Vendor.Tools { public class Tool { } }\n");
    fx.write("external/Ext.csproj", lib_csproj());
    fx.write("external/Ext.cs", "namespace External.Lib { public class Legacy { } }\n");
    fx.write(
        "architecture.spec.toml",
        &spec(
            &[
                "{ path = \"vendor/Tools\", reason = \"vendored CLI\" }",
                "{ path = \"external\", reason = \"generated\" }",
            ],
            &["App"],
        ),
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    assert_eq!(
        census_lines(&scan),
        vec![
            "excluded unit: external — excluded_sources".to_string(),
            "excluded unit: vendor/Tools — excluded_sources".to_string(),
        ],
        "one census line per excluded project, sorted by line:\n{}",
        common::stderr(&scan)
    );
}

/// Mandatory cell (task d): a glob-form entry stays inert at the project
/// tier — the matcher is component-wise, the entry matches no csproj path,
/// no project drops, and today's liveness advisory is unchanged.
#[test]
fn glob_form_entry_keeps_the_matched_nothing_advisory() {
    let fx = repro_tree();
    fx.write(
        "architecture.spec.toml",
        &spec(
            &["{ path = \"**/Ext/**\", reason = \"glob form, inert for csharp\" }"],
            &["App", "Ext"],
        ),
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(
        unit_names(&model),
        vec!["App".to_string(), "Ext".to_string()],
        "a glob-form entry matches no csproj path, so no project drops"
    );
    assert!(
        census_lines(&scan).is_empty(),
        "a glob-form entry drops no project, so it prints no census line:\n{}",
        common::stderr(&scan)
    );
    assert_eq!(
        liveness_lines(&scan),
        vec!["excluded source matched nothing: **/Ext/** — glob form, inert for csharp".to_string()],
        "the glob-form advisory path is unchanged (issue #60's file class owns it):\n{}",
        common::stderr(&scan)
    );
}

/// The mechanical gate stays: `bin`/`obj`/`node_modules`/dot directories
/// never contribute projects with or without a declaring spec, and the
/// declared consult only ADDS drops on top of it.
#[test]
fn mechanical_gate_stays_and_declared_only_adds() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write("App/App.cs", "namespace App { public class Greeter { } }\n");
    fx.write("App/bin/Debug/Artifact.csproj", lib_csproj());
    fx.write("external/Ext.csproj", lib_csproj());
    fx.write("external/Ext.cs", "namespace External.Lib { public class Legacy { } }\n");
    fx.write(
        "architecture.spec.toml",
        &spec(
            &["{ path = \"external\", reason = \"generated dependency\" }"],
            &["App"],
        ),
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(
        unit_names(&model),
        vec!["App".to_string()],
        "the mechanical bin/obj gate stays armed and the declared consult only adds its drop"
    );
    assert_eq!(
        census_lines(&scan),
        vec!["excluded unit: external — excluded_sources".to_string()],
        "a mechanically dropped project never prints the census line:\n{}",
        common::stderr(&scan)
    );
}
