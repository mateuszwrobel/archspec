//! Golden freeze of the sole extraction path.
//!
//! For each supported language fixture these tests run every extraction
//! command with no flags and compare the emitted artefact
//! byte-for-byte against a committed golden under `tests/goldens/`. The
//! goldens were recorded from the grammar-based extraction (the removed
//! `--backend syntax` path became the only path), so they are the stability
//! contract: any unintended byte shift in `scan`, `update`,
//! `report --format json`, `depgraph modules` or `inspect tree` output fails
//! here.
//!
//! Refresh flow (mirrors the repo's `--check` freshness convention, inverted
//! for recording): run with `ARCHSPEC_GOLDEN=1 cargo test --test
//! extraction_freeze` to rewrite the goldens from the current binary; a plain
//! run only compares. Refreshing a golden is a deliberate, reviewed act —
//! never do it to make this suite pass after an unintended behaviour change.
//!
//! What is frozen per fixture:
//! - `scan` stdout (model JSON) -> `tests/goldens/scan/<fixture>.json`
//! - `update` written seed spec -> `tests/goldens/update/<fixture>.toml`
//!   (the update stdout is a constant two-line notice; the seed content is
//!   the observable artefact)
//! - `report --format json` stdout (metrics + findings; no volatile fields —
//!   verified there are no timestamps or absolute paths in the emission) ->
//!   `tests/goldens/report/<fixture>.json`, run against the seeded spec so
//!   the update -> report chain is frozen too
//! - `depgraph modules` stdout -> `tests/goldens/depgraph-modules/<fixture>.mmd`
//! - `inspect tree` stdout -> `tests/goldens/inspect-tree/<fixture>.mmd`
//!
//! Every probe fixture carries a module tier through its package references
//! (the go trees included: intra-module package imports project onto the
//! module tier), so every fixture renders graphs rather than refusing.

mod common;
#[allow(dead_code)]
mod shared;

use common::{stderr, stdout};
use shared::driver::{materialize_go_workspace, Driver, Language};
use std::path::PathBuf;

/// Golden refresh mode: `ARCHSPEC_GOLDEN` set to anything but `0` rewrites
/// the goldens from the running binary instead of comparing.
fn golden_mode() -> bool {
    std::env::var("ARCHSPEC_GOLDEN")
        .map(|value| !value.is_empty() && value != "0")
        .unwrap_or(false)
}

fn golden_path(command: &str, fixture: &str, extension: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(command)
        .join(format!("{fixture}.{extension}"))
}

/// Compare one emitted artefact against its golden, or record it in refresh
/// mode. A missing golden fails with the exact regeneration command.
fn freeze(label: &str, command: &str, fixture: &str, extension: &str, bytes: &[u8]) {
    let path = golden_path(command, fixture, extension);
    if golden_mode() {
        std::fs::create_dir_all(path.parent().expect("golden dir parent"))
            .expect("create golden dir");
        std::fs::write(&path, bytes).expect("write golden");
        eprintln!("ARCHSPEC_GOLDEN: recorded {}", path.display());
        return;
    }
    let golden = std::fs::read(&path).unwrap_or_else(|_| {
        panic!(
            "golden artefact missing: {}\n{label} emitted something the freeze suite has no \
             record of; re-record the baseline with:\n  \
             ARCHSPEC_GOLDEN=1 cargo test --test extraction_freeze",
            path.display()
        )
    });
    assert_eq!(
        golden.as_slice(),
        bytes,
        "{label} differs from golden {}",
        path.display()
    );
}

/// The full command matrix over one materialized fixture. Order matters:
/// `update` seeds the spec that `report` then evaluates, freezing the chain.
fn run_matrix(fixture: &common::Fixture, name: &str) {
    let scan = fixture.run(&["scan"]);
    assert_eq!(
        scan.status.code(),
        Some(0),
        "scan on {name} must exit 0 (stderr: {})",
        stderr(&scan)
    );
    freeze("scan stdout", "scan", name, "json", &scan.stdout);

    let update = fixture.run(&["update"]);
    assert_eq!(
        update.status.code(),
        Some(0),
        "update on {name} must exit 0 (stderr: {})",
        stderr(&update)
    );
    let seed = std::fs::read(fixture.root.join("architecture.spec.toml"))
        .expect("update must write architecture.spec.toml");
    freeze("update seed spec", "update", name, "toml", &seed);

    let report = fixture.run(&["report", "--format", "json"]);
    assert_eq!(
        report.status.code(),
        Some(0),
        "report on {name} must exit 0 against the seeded spec (stderr: {})",
        stderr(&report)
    );
    freeze("report json stdout", "report", name, "json", &report.stdout);

    let modules = fixture.run(&["depgraph", "modules"]);
    assert_eq!(
        modules.status.code(),
        Some(0),
        "depgraph modules on {name} must exit 0 (stderr: {})",
        stderr(&modules)
    );
    assert!(
        !stdout(&modules).is_empty(),
        "the module tier renders a non-empty graph on {name}"
    );
    freeze(
        "depgraph modules stdout",
        "depgraph-modules",
        name,
        "mmd",
        &modules.stdout,
    );

    let tree = fixture.run(&["inspect", "tree"]);
    assert_eq!(
        tree.status.code(),
        Some(0),
        "inspect tree on {name} must exit 0 (stderr: {})",
        stderr(&tree)
    );
    freeze(
        "inspect tree stdout",
        "inspect-tree",
        name,
        "mmd",
        &tree.stdout,
    );
}

/// The canonical probe tree materialized for one language: the same shape the
/// feature-matrix scenarios drive, exercising every model tier the extraction
/// populates (units, hard edges, soft modules, module edges, externals).
fn probe_fixture(language: Language) -> common::Fixture {
    let fixture = common::Fixture::new();
    let driver = Driver { language };
    driver.materialize(&fixture, &driver.probe_tree());
    fixture
}

#[test]
fn golden_freeze_rust() {
    run_matrix(&probe_fixture(Language::Rust), "rust");
}

#[test]
fn golden_freeze_csharp() {
    run_matrix(&probe_fixture(Language::Csharp), "csharp");
}

#[test]
fn golden_freeze_go_single_module() {
    run_matrix(&probe_fixture(Language::Go), "go");
}

#[test]
fn golden_freeze_go_workspace() {
    let fixture = common::Fixture::new();
    materialize_go_workspace(&fixture);
    run_matrix(&fixture, "go-workspace");
}

/// The removed back-end flag is an unknown flag everywhere: any value, any
/// extraction command, the standard usage error with exit 1.
#[test]
fn backend_flag_is_unknown_on_every_extraction_command() {
    let fixture = probe_fixture(Language::Rust);
    for command in [
        vec!["scan"],
        vec!["verify"],
        vec!["update"],
        vec!["report"],
        vec!["depgraph", "modules"],
        vec!["inspect", "tree"],
        vec!["inspect"],
    ] {
        for value in ["classic", "syntax"] {
            let mut args = command.clone();
            args.push("--backend");
            args.push(value);
            let run = fixture.run(&args);
            assert_eq!(
                run.status.code(),
                Some(1),
                "`{}` with a back-end flag must fail (stderr: {})",
                command.join(" "),
                stderr(&run)
            );
            let message = stderr(&run);
            assert!(
                message.contains("unknown flag: --backend"),
                "`{} --backend {value}` must state the standard unknown-flag error, got: {message}",
                command.join(" ")
            );
        }
        // The bare flag (no value) fails the same way.
        let mut args = command.clone();
        args.push("--backend");
        let run = fixture.run(&args);
        assert_eq!(run.status.code(), Some(1));
        assert!(stderr(&run).contains("unknown flag: --backend"));
    }
}

/// No command's help advertises a back-end flag.
#[test]
fn help_never_mentions_the_backend_flag() {
    let fixture = probe_fixture(Language::Rust);
    for topic in [
        "scan",
        "verify",
        "update",
        "report",
        "depgraph",
        "inspect",
        "capability",
        "",
    ] {
        let args: Vec<&str> = if topic.is_empty() {
            vec!["--help"]
        } else {
            vec!["help", topic]
        };
        let run = fixture.run(&args);
        let text = stdout(&run);
        assert!(
            !text.contains("--backend") && !text.contains("classic"),
            "`archspec {}` still advertises back-end vocabulary:\n{text}",
            args.join(" ")
        );
    }
}

/// The embedded skill is agent-facing procedure: every surface that emits it —
/// `skill` print, `skill print`, the installed file, `skill --help` — speaks
/// the flagless surface. A skill-following agent must never build a command
/// with the removed `--backend` flag (`unknown flag: --backend`), and no
/// back-end-era wording ("backend syntax", `classic`) survives in it.
#[test]
fn skill_never_mentions_the_backend_flag() {
    let fixture = common::Fixture::new();
    let mut surfaces = Vec::new();
    for args in [
        vec!["skill"],
        vec!["skill", "print"],
        vec!["skill", "--help"],
    ] {
        let run = fixture.run(&args);
        assert!(
            run.status.success(),
            "`archspec {}` failed: {}",
            args.join(" "),
            stderr(&run)
        );
        surfaces.push((format!("archspec {}", args.join(" ")), stdout(&run)));
    }
    let install = fixture.run(&["skill", "install"]);
    assert!(
        install.status.success(),
        "skill install failed: {}",
        stderr(&install)
    );
    surfaces.push((
        "installed .agent/skills/archspec.md".to_string(),
        fixture.read(".agent/skills/archspec.md"),
    ));
    for (label, text) in surfaces {
        let lowered = text.to_lowercase();
        assert!(
            !lowered.contains("--backend")
                && !lowered.contains("backend syntax")
                && !lowered.contains("syntax backend")
                && !lowered.contains("back-end")
                && !lowered.contains("classic"),
            "{label} still carries back-end vocabulary:\n{text}"
        );
    }
}
