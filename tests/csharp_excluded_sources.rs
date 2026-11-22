//! GitHub issue #60 cells — declared `excluded_sources` entries adjudicate
//! at the csharp `.cs` SOURCE FILE walk: a file at or under an entry inside a
//! KEPT project contributes no types, no namespaces and no unit source facts,
//! and states itself with one `excluded source: <path> — excluded_sources`
//! census line, sorted and never silent (the issue #39 law). The mechanical
//! `is_excluded_cs` gate stays the first door. Liveness is UNCHANGED and
//! driver-independent ("any file counts", `spec.rs`): an entry matching files
//! anywhere is ACTIVE and prints no `matched nothing` advisory. The no-double-
//! report law: a project dropped at discovery (issue #49) never reaches the
//! file gate, so its files state themselves only at the project line. The
//! P9 project-tier cells live in `tests/csharp_excluded_projects.rs` and stay
//! green unchanged.

mod common;

use common::Fixture;
use serde_json::Value;

/// The census forms pinned by the claim registry (`csharp-excluded-sources`,
/// owner surface: `help diagnostics`). The dedicated guard for those
/// placeholder bytes is this file: the exact-bytes cells below render them.
#[allow(dead_code)]
const CENSUS_FORMS: [&str; 1] = ["excluded source: <path> — excluded_sources"];

fn lib_csproj() -> &'static str {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n</Project>\n"
}

fn spec(entries: &[&str]) -> String {
    format!(
        "[project]\nlanguage = \"csharp\"\n\nexcluded_sources = [{}]\n",
        entries.join(", ")
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

fn file_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("excluded source: "))
        .map(str::to_string)
        .collect()
}

fn unit_lines(output: &std::process::Output) -> Vec<String> {
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

/// Mandatory cell 1: a dir-form entry naming a folder INSIDE a kept project
/// prunes the files under it — no namespace, no public type — and states one
/// exact-bytes census line per excluded source file. The projects stay; the
/// entry stays ACTIVE (its files count for liveness), so no `matched nothing`
/// advisory and no project-level line.
#[test]
fn dir_form_entry_prunes_files_under_a_kept_project() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write("App/App.cs", "namespace App { public class Greeter { } }\n");
    fx.write(
        "App/generated/Gen.cs",
        "namespace App.Generated { public class Gen { } }\n",
    );
    fx.write("Lib/Lib.csproj", lib_csproj());
    fx.write("Lib/Lib.cs", "namespace App.Lib { public class Core { } }\n");
    fx.write(
        "architecture.spec.toml",
        &spec(&["{ path = \"App/generated\", reason = \"generated sources\" }"]),
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
        vec!["App".to_string(), "Lib".to_string()],
        "a file-level entry drops no project"
    );
    let body = common::stdout(&scan);
    assert!(
        !body.contains("App::Generated"),
        "the excluded file contributes no namespace and no module:\n{body}"
    );
    assert!(
        !body.contains("\"Gen\""),
        "the excluded file contributes no public type:\n{body}"
    );
    assert_eq!(
        file_lines(&scan),
        vec!["excluded source: App/generated/Gen.cs — excluded_sources".to_string()],
        "exactly one census line per excluded file, exact bytes (never-silent law):\n{}",
        common::stderr(&scan)
    );
    assert!(
        unit_lines(&scan).is_empty(),
        "a kept project never prints the project-level line:\n{}",
        common::stderr(&scan)
    );
    assert!(
        liveness_lines(&scan).is_empty(),
        "an entry pruning files keeps its liveness through those files:\n{}",
        common::stderr(&scan)
    );
}

/// Mandatory cell 2 (the non-declaring law): a tree with no entries — no
/// spec at all, and a spec with an empty table — changes zero model bytes and
/// prints no census byte anywhere, even when a folder sits in the tree that
/// an entry WOULD name.
#[test]
fn no_entries_scans_byte_identical() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write("App/App.cs", "namespace App { public class Greeter { } }\n");
    fx.write(
        "App/generated/Gen.cs",
        "namespace App.Generated { public class Gen { } }\n",
    );
    let bare = fx.run(&["scan", "."]);
    assert!(
        bare.status.success(),
        "bare scan failed: {}",
        common::stderr(&bare)
    );
    fx.write("architecture.spec.toml", "[project]\nlanguage = \"csharp\"\n\nexcluded_sources = []\n");
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
        for marker in ["excluded source: ", "excluded unit:", "matched nothing"] {
            assert!(
                !all.contains(marker),
                "a non-declaring tree never renders `{marker}`:\n{all}"
            );
        }
    }
}

/// Mandatory cell 4 — the issue #60 body's own repro fixture: the declared
/// entry naming the `legacy` project dir flips the model. The `Demo.Legacy`
/// namespace and the legacy unit leave the machine body (the drop lands at
/// project discovery, the issue #49 arm — this cell pins the repro end to
/// end); the files under the dropped project print NO file-level lines
/// (no-double-report law), and the entry keeps its liveness.
#[test]
fn issue60_repro_fixture_exclusion_holds() {
    let fx = Fixture::new();
    fx.write("kept/kept.csproj", lib_csproj());
    fx.write("kept/Alpha.cs", "namespace Demo.Kept; public class Alpha {}\n");
    fx.write("legacy/legacy.csproj", lib_csproj());
    fx.write("legacy/Beta.cs", "namespace Demo.Legacy; public class Beta {}\n");
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n",
    );
    let before = fx.run(&["scan", "."]);
    assert!(
        before.status.success(),
        "before-scan failed: {}",
        common::stderr(&before)
    );
    assert!(
        common::stdout(&before).contains("Demo::Legacy"),
        "the unexcluded repro carries the legacy namespace:\n{}",
        common::stdout(&before)
    );
    fx.write(
        "architecture.spec.toml",
        &spec(&["{ path = \"legacy\", reason = \"legacy subtree\" }"]),
    );
    let after = fx.run(&["scan", "."]);
    assert!(
        after.status.success(),
        "after-scan failed: {}",
        common::stderr(&after)
    );
    let before_model: Value =
        serde_json::from_str(&common::stdout(&before)).expect("before stdout is JSON");
    let after_model: Value =
        serde_json::from_str(&common::stdout(&after)).expect("after stdout is JSON");
    assert_ne!(
        common::stdout(&before),
        common::stdout(&after),
        "the entry must move the machine body — the repro's cmp goes green"
    );
    assert!(
        !common::stdout(&after).contains("Demo::Legacy"),
        "the excluded namespace is absent from the model body:\n{}",
        common::stdout(&after)
    );
    assert_eq!(
        unit_names(&before_model).len(),
        2,
        "before: both projects are units"
    );
    assert_eq!(
        unit_names(&after_model),
        vec!["kept".to_string()],
        "after: only the kept project is a unit"
    );
    assert_eq!(
        unit_lines(&after),
        vec!["excluded unit: legacy — excluded_sources".to_string()],
        "the project states its drop exactly once:\n{}",
        common::stderr(&after)
    );
    assert!(
        file_lines(&after).is_empty(),
        "files under a project dropped at discovery never reach the file gate:\n{}",
        common::stderr(&after)
    );
    assert!(
        liveness_lines(&after).is_empty(),
        "the entry matches files: ACTIVE, no advisory:\n{}",
        common::stderr(&after)
    );
}

/// Mandatory cell 5: an entry naming a folder inside a kept project — and
/// nothing else anywhere — behaves exactly like cell 1: one sorted census
/// line per excluded file, and NO `matched nothing` row because the liveness
/// path counts those same files (driver-independent, untouched).
#[test]
fn file_only_entry_prunes_files_and_never_reads_dead() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write("App/App.cs", "namespace App { public class Greeter { } }\n");
    fx.write(
        "App/generated/First.cs",
        "namespace App.Generated.One { public class First { } }\n",
    );
    fx.write(
        "App/generated/Second.cs",
        "namespace App.Generated.Two { public class Second { } }\n",
    );
    fx.write(
        "architecture.spec.toml",
        &spec(&["{ path = \"App/generated\", reason = \"generated sources\" }"]),
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let body = common::stdout(&scan);
    for namespace in ["App::Generated::One", "App::Generated::Two"] {
        assert!(
            !body.contains(namespace),
            "the excluded files contribute no namespace: {namespace}\n{body}"
        );
    }
    assert_eq!(
        file_lines(&scan),
        vec![
            "excluded source: App/generated/First.cs — excluded_sources".to_string(),
            "excluded source: App/generated/Second.cs — excluded_sources".to_string(),
        ],
        "one sorted census line per excluded file:\n{}",
        common::stderr(&scan)
    );
    assert!(
        unit_lines(&scan).is_empty(),
        "the kept project stays: no project-level line:\n{}",
        common::stderr(&scan)
    );
    assert!(
        liveness_lines(&scan).is_empty(),
        "the liveness path is untouched — file matches keep the entry ACTIVE:\n{}",
        common::stderr(&scan)
    );
}

/// The no-double-report law, pinned: an entry dropping a project AND a
/// nested entry naming files under that same dropped project state exactly
/// the one project line — the dropped project's files never reach the file
/// gate — and both entries stay ACTIVE for liveness.
#[test]
fn project_level_entry_states_once_and_its_files_add_no_rows() {
    let fx = Fixture::new();
    fx.write("App/App.csproj", lib_csproj());
    fx.write("App/App.cs", "namespace App { public class Greeter { } }\n");
    fx.write("external/Ext.csproj", lib_csproj());
    fx.write(
        "external/Ext.cs",
        "namespace External.Lib { public class Legacy { } }\n",
    );
    fx.write(
        "external/gen/More.cs",
        "namespace External.Gen { public class More { } }\n",
    );
    fx.write(
        "architecture.spec.toml",
        &spec(&[
            "{ path = \"external\", reason = \"generated dependency\" }",
            "{ path = \"external/gen\", reason = \"generated sources\" }",
        ]),
    );
    let scan = fx.run(&["scan", "."]);
    assert!(
        scan.status.success(),
        "scan failed: {}",
        common::stderr(&scan)
    );
    let model: Value = serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is JSON");
    assert_eq!(unit_names(&model), vec!["App".to_string()]);
    assert_eq!(
        unit_lines(&scan),
        vec!["excluded unit: external — excluded_sources".to_string()],
        "the dropped project states itself exactly once:\n{}",
        common::stderr(&scan)
    );
    assert!(
        file_lines(&scan).is_empty(),
        "a dropped project's files never state themselves again:\n{}",
        common::stderr(&scan)
    );
    assert!(
        liveness_lines(&scan).is_empty(),
        "both entries match files: both ACTIVE, no advisory:\n{}",
        common::stderr(&scan)
    );
}

/// Mandatory cell 6 (the witness-corpus-shape entry): a wwwroot-style dir inside a
/// kept app with no `.cs` under it names NOTHING REAL — the machine body is
/// byte-identical to the same tree without the entry (zero model delta), no
/// file-level line fires, and the entry is ACTIVE (files under it count), so
/// the `matched nothing` advisory stays silent. Only the active-entry
/// advisory line moves.
#[test]
fn wwwroot_shape_entry_changes_no_model_byte() {
    let fx = Fixture::new();
    fx.write("src/WebApp/WebApp.csproj", lib_csproj());
    fx.write(
        "src/WebApp/Home.cs",
        "namespace Shop.Web { public class Home { } }\n",
    );
    fx.write("src/WebApp/wwwroot/index.html", "<html><body>shop</body></html>\n");
    fx.write("src/WebApp/wwwroot/css/site.css", "body { margin: 0; }\n");
    let bare = fx.run(&["scan", "."]);
    assert!(
        bare.status.success(),
        "bare scan failed: {}",
        common::stderr(&bare)
    );
    fx.write(
        "architecture.spec.toml",
        &spec(&[(
            "{ path = \"src/WebApp/wwwroot\", reason = \"static web assets\" }"
        )]),
    );
    let with = fx.run(&["scan", "."]);
    assert!(
        with.status.success(),
        "with-entry scan failed: {}",
        common::stderr(&with)
    );
    assert_eq!(
        common::stdout(&bare),
        common::stdout(&with),
        "an entry naming no `.cs` source changes zero machine bytes"
    );
    let stderr = common::stderr(&with);
    assert!(
        stderr.contains("excluded sources: src/WebApp/wwwroot — static web assets"),
        "the active-entry advisory line is present:\n{stderr}"
    );
    assert!(
        file_lines(&with).is_empty(),
        "no file was pruned, so no census line:\n{stderr}"
    );
    assert!(
        liveness_lines(&with).is_empty(),
        "files under the entry count for liveness: ACTIVE, no advisory:\n{stderr}"
    );
}
