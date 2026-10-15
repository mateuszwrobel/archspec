//! The per-module diagram views (workplan
//! rust-arch-test-kit/worklog/workplan_archspec_module_public_api_diagrams.md,
//! D11/D12): `--module <claim>` renders one claim's territory with the public
//! API marks read through the shared adjudication seam — a dashed crossing is
//! exactly a red crossing in `verify` — and `--modules-dir <dir>` batch-writes
//! one stable-named file per claim with an injective filename escape,
//! orphan-detecting `--check`, and per-file atomic writes.

mod common;

use common::{stderr, stdout, Fixture};

fn csproj() -> String {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n"
        .to_string()
}

/// App project; `App.Billing` (uses `App.Shared`) exports `Report` + `Secret`;
/// `App.Shared` exports `Helper` — one real module edge for the marks to ride.
fn app_tree() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    fixture.write(
        "App/Billing.cs",
        "using App.Shared;\nnamespace App.Billing;\npublic class Report { }\npublic class Secret { public Helper H = new(); }\n",
    );
    fixture.write(
        "App/Shared.cs",
        "namespace App.Shared;\npublic class Helper { }\n",
    );
    fixture
}

fn spec(expose_billing: &str) -> String {
    format!(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = {{ units = [\"App\"], modules = [\"App\", \"App::*\"] }}\n\n[[module.submodules]]\nname = \"App::Billing\"\nmatches = {{ modules = [\"App::Billing\"] }}\nallowed = {{ depend_on = [\"App::Shared\"] }}\ncontract = {{ expose = {expose_billing} }}\n\n[[module.submodules]]\nname = \"App::Shared\"\nmatches = {{ modules = [\"App::Shared\"] }}\ncontract = {{ expose = [\"Helper\"] }}\n"
    )
}

/// `--module` renders the claim's territory (nodes for its module paths) and
/// exits 0; the render is a graph, not a refusal.
#[test]
fn module_view_renders_one_claims_territory() {
    let fixture = app_tree();
    fixture.write("architecture.spec.toml", &spec(r#"["Report", "Secret"]"#));
    let output = fixture.run(&["diagram", "--module", "App"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "territory render exits 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("graph TD\n"), "mermaid body:\n{out}");
    assert!(
        out.contains("[\"App::Billing\"]"),
        "the submodule's module path renders as a node inside App's territory:\n{out}"
    );
    assert!(
        out.contains("[\"App::Shared\"]"),
        "the neighbour across the edge renders as a node:\n{out}"
    );
}

/// The parity law (D11): a mark in the render is exactly a red crossing in
/// `verify` — the leaking spec dashes the arrow and reds verify; the complete
/// spec carries no dash and verifies green. One adjudication, two surfaces.
#[test]
fn module_view_marks_dashed_exactly_where_verify_reds() {
    let leaking = spec(r#"["Report"]"#);
    let fixture = app_tree();
    fixture.write("architecture.spec.toml", &leaking);
    let verify = fixture.run(&["verify"]);
    assert_eq!(verify.status.code(), Some(1), "undeclared crossing reds");
    let diagram = fixture.run(&["diagram", "--module", "App"]);
    assert_eq!(diagram.status.code(), Some(0));
    assert!(
        stdout(&diagram).contains("violates spec"),
        "the leaking crossing renders dashed:\n{}",
        stdout(&diagram)
    );

    let complete = spec(r#"["Report", "Secret"]"#);
    fixture.write("architecture.spec.toml", &complete);
    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "a complete expose set verifies green (stdout: {})",
        stdout(&verify)
    );
    let diagram = fixture.run(&["diagram", "--module", "App"]);
    assert_eq!(diagram.status.code(), Some(0));
    assert!(
        !stdout(&diagram).contains("violates spec"),
        "a green territory carries no marks:\n{}",
        stdout(&diagram)
    );
}

/// The mark vocabulary is format-neutral: the same dashed arrow renders in
/// plantuml.
#[test]
fn module_view_marks_parity_across_formats() {
    let fixture = app_tree();
    fixture.write("architecture.spec.toml", &spec(r#"["Report"]"#));
    let output = fixture.run(&[
        "diagram",
        "--module",
        "App",
        "--format",
        "plantuml",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).contains("violates spec"),
        "plantuml carries the same mark vocabulary:\n{}",
        stdout(&output)
    );
}

/// An unknown claim is a usage error naming the claim (exit 64).
#[test]
fn module_view_unknown_claim_exits_64_naming_the_claim() {
    let fixture = app_tree();
    fixture.write("architecture.spec.toml", &spec(r#"["Report", "Secret"]"#));
    let output = fixture.run(&["diagram", "--module", "App::Nope"]);
    assert_eq!(output.status.code(), Some(64), "unknown claim is a usage error");
    let err = stderr(&output);
    assert!(
        err.contains("unknown module: no claim of the spec carries that name: App::Nope"),
        "the refusal names the claim:\n{err}"
    );
}

/// Both flags are spec-source-only and mutually exclusive; every rejected
/// combination is a usage error naming the rule (exit 64).
#[test]
fn module_view_usage_conflicts_exit_64_naming_the_rule() {
    let fixture = app_tree();
    fixture.write("architecture.spec.toml", &spec(r#"["Report", "Secret"]"#));
    let scan = fixture.run(&["scan", "--output", "model.json"]);
    assert_eq!(scan.status.code(), Some(0));

    let both = fixture.run(&["diagram", "--module", "App", "--modules-dir", "out"]);
    assert_eq!(both.status.code(), Some(64));
    assert!(
        stderr(&both).contains("mutually exclusive"),
        "the refusal names the rule:\n{}",
        stderr(&both)
    );

    let scan_leg = fixture.run(&["diagram", "--source", "scan", "model.json", "--module", "App"]);
    assert_eq!(scan_leg.status.code(), Some(64));
    assert!(
        stderr(&scan_leg).contains("require the spec source"),
        "the refusal names the rule:\n{}",
        stderr(&scan_leg)
    );

    let batch_scan = fixture.run(&[
        "diagram",
        "--source",
        "scan",
        "model.json",
        "--modules-dir",
        "out",
    ]);
    assert_eq!(batch_scan.status.code(), Some(64));
    assert!(stderr(&batch_scan).contains("require the spec source"));
}

/// The batch writes one file per claim under the escape; a rerun reproduces
/// every file byte for byte (stable names AND stable bytes).
#[test]
fn modules_dir_writes_one_stable_file_per_claim() {
    let fixture = app_tree();
    fixture.write("architecture.spec.toml", &spec(r#"["Report", "Secret"]"#));
    let first = fixture.run(&["diagram", "--modules-dir", "out"]);
    assert_eq!(
        first.status.code(),
        Some(0),
        "batch write exits 0 (stderr: {})",
        stderr(&first)
    );
    assert!(fixture.path("out/App.mmd").exists(), "the top claim file exists");
    assert!(
        fixture.path("out/App%3a%3aBilling.mmd").exists(),
        "the nested claim's file names the claim through the escape"
    );
    assert!(fixture.path("out/App%3a%3aShared.mmd").exists());
    let before = fixture.read("out/App.mmd");
    let second = fixture.run(&["diagram", "--modules-dir", "out"]);
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(
        fixture.read("out/App.mmd"),
        before,
        "a rerun is byte-identical — stable names carry stable bytes"
    );
    let check = fixture.run(&["diagram", "--modules-dir", "out", "--check"]);
    assert_eq!(check.status.code(), Some(0), "a fresh batch checks fresh");
    assert!(
        stdout(&check).contains("up to date"),
        "the fresh check states the summary:\n{}",
        stdout(&check)
    );
}

/// `--check` flags stale files, flags orphan render-extension files, never
/// touches or reports foreign files, and names the regeneration command.
#[test]
fn modules_dir_check_flags_stale_and_orphan_files() {
    let fixture = app_tree();
    fixture.write("architecture.spec.toml", &spec(r#"["Report", "Secret"]"#));
    assert_eq!(
        fixture
            .run(&["diagram", "--modules-dir", "out"])
            .status
            .code(),
        Some(0)
    );

    fixture.write("out/App.mmd", "stale bytes\n");
    let stale = fixture.run(&["diagram", "--modules-dir", "out", "--check"]);
    assert_ne!(stale.status.code(), Some(0), "a stale file is a red check");
    let err = stderr(&stale);
    assert!(
        err.contains("out of date diagram") && err.contains("out/App.mmd"),
        "the finding names the file and its state:\n{err}"
    );

    assert_eq!(
        fixture.run(&["diagram", "--modules-dir", "out"]).status.code(),
        Some(0),
        "regenerate the stale file"
    );
    fixture.write("out/junk.mmd", "orphan\n");
    fixture.write("out/notes.txt", "foreign file, never the writer's territory\n");
    let orphan = fixture.run(&["diagram", "--modules-dir", "out", "--check"]);
    assert_ne!(orphan.status.code(), Some(0), "an orphan is a red check");
    let err = stderr(&orphan);
    assert!(
        err.contains("orphan diagram") && err.contains("out/junk.mmd"),
        "the finding names the orphan:\n{err}"
    );
    assert!(
        !err.contains("notes.txt"),
        "a foreign-extension file is never an orphan:\n{err}"
    );

    std::fs::remove_file(fixture.path("out/junk.mmd")).expect("remove orphan");
    let fresh = fixture.run(&["diagram", "--modules-dir", "out", "--check"]);
    assert_eq!(
        fresh.status.code(),
        Some(0),
        "with the orphan gone and foreign files untouched, the batch checks fresh (stderr: {})",
        stderr(&fresh)
    );
}

/// Per-file atomicity: one blocked target fails only its own file — every
/// other file lands at complete bytes, the blocked path stays untouched, and
/// the run exits operational (70) naming the failure. No temp residue.
#[test]
fn modules_dir_partial_failure_keeps_previous_or_complete_bytes() {
    let fixture = app_tree();
    fixture.write("architecture.spec.toml", &spec(r#"["Report", "Secret"]"#));
    // The first claim's target is a directory: its write can never succeed.
    std::fs::create_dir_all(fixture.path("out")).expect("create out");
    std::fs::create_dir(fixture.path("out/App.mmd")).expect("block one target");

    let output = fixture.run(&["diagram", "--modules-dir", "out"]);
    assert_eq!(
        output.status.code(),
        Some(70),
        "a failed batch write is operational (stderr: {})",
        stderr(&output)
    );
    assert!(
        fixture.path("out/App%3a%3aBilling.mmd").exists(),
        "the unblocked claim's file lands complete"
    );
    assert!(
        fixture.path("out/App.mmd").is_dir(),
        "the blocked path is untouched"
    );
    let residue: Vec<String> = std::fs::read_dir(fixture.path("out"))
        .expect("read out")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".tmp"))
        .collect();
    assert!(residue.is_empty(), "no temp residue after a partial failure");
}

/// The escape confines every claim name: no `/`, no bare `.`/`..`, and the
/// dotted round trip recovers the name — verified through the batch surface
/// with an escaped claim name (`a::b`).
#[test]
fn module_batch_filenames_never_escape_the_directory() {
    let fixture = Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    fixture.write(
        "App/Billing.cs",
        "namespace App.Billing;\npublic class Report { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"], modules = [\"App\", \"App::*\"] }\n\n[[module.submodules]]\nname = \"App::Billing\"\nmatches = { modules = [\"App::Billing\"] }\ncontract = { expose = [\"Report\"] }\n",
    );
    let output = fixture.run(&["diagram", "--modules-dir", "out"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "batch write exits 0 (stderr: {})",
        stderr(&output)
    );
    for entry in std::fs::read_dir(fixture.path("out")).expect("read out") {
        let name = entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        assert!(
            !name.contains('/') && name != ".".to_string() + ".mmd" && !name.starts_with(".."),
            "escaped filename stays inside the directory: {name}"
        );
    }
    assert!(
        fixture.path("out/App%3a%3aBilling.mmd").exists(),
        "the `::` claim name escapes to a single confined file"
    );
}
