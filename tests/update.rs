mod common;

use common::{stderr, stdout};

/// A Rust workspace with two crates where billing depends on auth — produces
/// one hard edge billing -> auth. Shared by #3 and the determinism check.
fn workspace_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture
}

/// A Rust workspace with two crates and no inter-crate dependencies.
fn no_edge_workspace_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture
}

#[test]
fn update_scenario_one_creates_spec_at_project_root() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn demo() {}\n");

    let output = fixture.run(&["update"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        fixture.path("architecture.spec.toml").exists(),
        "spec must be created at the project root"
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");

    let out = stdout(&output);
    assert!(
        out.contains("architecture.spec.toml"),
        "stdout names the generated file:\n{out}"
    );
    assert!(out.contains("review"), "stdout suggests review:\n{out}");
    assert!(out.contains("verify"), "stdout suggests verify:\n{out}");
}

#[test]
fn update_scenario_two_creates_spec_in_given_subdirectory() {
    let fixture = workspace_fixture();
    let output = fixture.run(&["update", "crates/auth"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(
        fixture.path("crates/auth/architecture.spec.toml").exists(),
        "spec must be created inside ./crates/auth"
    );
    assert!(
        !fixture.path("architecture.spec.toml").exists(),
        "no spec may be written at the project root"
    );
    let out = stdout(&output);
    assert!(
        out.contains("architecture.spec.toml") && out.contains("verify"),
        "stdout names the generated file and verify:\n{out}"
    );
}

#[test]
fn update_scenario_three_captures_language_modules_and_edges() {
    let fixture = workspace_fixture();
    let output = fixture.run(&["update"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );

    let spec = fixture.read("architecture.spec.toml");
    let expected = "[project]\n\
        language = \"rust\"\n\
        \n\
        [[module]]\n\
        name = \"auth\"\n\
        matches = { units = [\"auth\"] }\n\
        \n\
        [[module]]\n\
        name = \"billing\"\n\
        matches = { units = [\"billing\"] }\n\
        allowed = { depend_on = [\"auth\"] }\n\
        \n\
        [[constraint]]\n\
        type = \"no_cycles\"\n\
        \n";
    assert_eq!(spec, expected, "seed spec must be byte-exact:\n{spec}");
}

#[test]
fn update_scenario_four_declares_units_without_invented_edges() {
    let fixture = no_edge_workspace_fixture();
    let output = fixture.run(&["update"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );

    let spec = fixture.read("architecture.spec.toml");
    let expected = "[project]\n\
        language = \"rust\"\n\
        \n\
        [[module]]\n\
        name = \"auth\"\n\
        matches = { units = [\"auth\"] }\n\
        \n\
        [[module]]\n\
        name = \"billing\"\n\
        matches = { units = [\"billing\"] }\n\
        \n\
        [[constraint]]\n\
        type = \"no_cycles\"\n\
        \n";
    assert_eq!(spec, expected, "seed spec must be byte-exact:\n{spec}");
    assert!(
        !spec.contains("allowed") && !spec.contains("depend_on"),
        "no invented edges:\n{spec}"
    );
}

#[test]
fn update_stdout_summary_names_file_review_and_verify() {
    let fixture = workspace_fixture();
    let output = fixture.run(&["update"]);

    assert_eq!(output.status.code(), Some(0), "update must exit 0");
    let out = stdout(&output);
    assert!(
        out.contains("generated ") && out.contains("architecture.spec.toml"),
        "summary must name the generated file:\n{out}"
    );
    assert!(
        out.contains("review") && out.contains("verify"),
        "summary must give review + verify guidance:\n{out}"
    );
}

#[test]
fn update_is_byte_identical_on_identical_fixtures() {
    let first = workspace_fixture();
    let second = workspace_fixture();
    let first_run = first.run(&["update"]);
    let second_run = second.run(&["update"]);

    assert_eq!(first_run.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second_run.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        first.read("architecture.spec.toml"),
        second.read("architecture.spec.toml"),
        "identical trees must produce byte-identical specs"
    );
}

/// A single crate named `app` with internal modules `billing` and `config`,
/// where `billing` imports `super::config` — producing one module edge
/// `app::billing -> app::config` and no hard unit edges.
fn single_crate_with_internal_modules_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod billing;\nmod config;\n");
    fixture.write("src/billing.rs", "use super::config;\n");
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture
}

// A single crate named `app` with nested internal modules: `billing` containing
// `billing::sub` and `billing::tests`, plus `config`. Both `billing::sub` and
// `billing::tests` reach `config` — producing module edges `app::billing::sub ->
// app::config` and `app::billing::tests -> app::config`.
fn nested_internal_modules_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod billing;\nmod config;\n");
    fixture.write("src/billing.rs", "mod sub;\nmod tests;\n");
    fixture.write("src/billing/sub.rs", "use crate::config;\n");
    fixture.write("src/billing/tests.rs", "use crate::config;\n");
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture
}

// A single crate named `app` with a nested `commands` module (`start`, `init`,
// `tests`) where `commands::start` reaches `config` — producing the module edge
// `app::commands::start -> app::config`.
fn commands_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod commands;\nmod config;\n");
    fixture.write("src/commands.rs", "mod start;\nmod init;\nmod tests;\n");
    fixture.write("src/commands/start.rs", "use crate::config;\n");
    fixture.write("src/commands/init.rs", "pub fn init() {}\n");
    fixture.write("src/commands/tests.rs", "pub fn tests() {}\n");
    fixture.write("src/config.rs", "pub fn config() {}\n");
    fixture
}

#[test]
fn update_seeds_module_boundaries_for_internal_modules() {
    let fixture = nested_internal_modules_fixture();
    let output = fixture.run(&["update"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");

    let spec = fixture.read("architecture.spec.toml");
    let expected = "[project]\n\
        language = \"rust\"\n\
        \n\
        [[module]]\n\
        name = \"app\"\n\
        matches = { units = [\"app\"] }\n\
        \n\
        [[module]]\n\
        name = \"app::billing\"\n\
        matches = { modules = [\"app::billing\"] }\n\
        allowed = { depend_on = [\"app::config\"] }\n\
        \n\
        [[module]]\n\
        name = \"app::config\"\n\
        matches = { modules = [\"app::config\"] }\n\
        \n\
        [[constraint]]\n\
        type = \"no_cycles\"\n\
        \n";
    assert_eq!(spec, expected, "seed spec must be byte-exact:\n{spec}");
    assert!(
        !spec.contains("app::billing::sub") && !spec.contains("app::billing::tests"),
        "nested and ::tests modules must fold into their top-level boundary:\n{spec}"
    );
}

// acceptance #22: a crate with BOTH lib.rs and main.rs seeds boundaries for the
// top-level modules of BOTH trees.
#[test]
fn update_seeds_boundaries_for_both_lib_and_main_trees() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod libmod;\n");
    fixture.write("src/libmod.rs", "pub fn lib_fn() {}\n");
    fixture.write("src/main.rs", "mod mainmod;\n");
    fixture.write("src/mainmod.rs", "pub fn main_fn() {}\n");
    let output = fixture.run(&["update"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");

    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"app::libmod\""),
        "lib tree boundary must be seeded:\n{spec}"
    );
    assert!(
        spec.contains("name = \"app-bin::mainmod\""),
        "main tree boundary must be seeded under the bin unit:\n{spec}"
    );
}

// acceptance #16/#17: one boundary per top-level module, none for nested or
// `::tests`; the `commands` boundary aggregates the config reach.
#[test]
fn update_seeds_one_coarse_boundary_per_top_level_module() {
    let fixture = commands_fixture();
    let output = fixture.run(&["update"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");

    let spec = fixture.read("architecture.spec.toml");
    let expected = "[project]\n\
        language = \"rust\"\n\
        \n\
        [[module]]\n\
        name = \"app\"\n\
        matches = { units = [\"app\"] }\n\
        \n\
        [[module]]\n\
        name = \"app::commands\"\n\
        matches = { modules = [\"app::commands\"] }\n\
        allowed = { depend_on = [\"app::config\"] }\n\
        \n\
        [[module]]\n\
        name = \"app::config\"\n\
        matches = { modules = [\"app::config\"] }\n\
        \n\
        [[constraint]]\n\
        type = \"no_cycles\"\n\
        \n";
    assert_eq!(spec, expected, "seed spec must be byte-exact:\n{spec}");
    assert!(
        !spec.contains("app::commands::start") && !spec.contains("app::commands::tests"),
        "no boundary for nested or ::tests modules:\n{spec}"
    );
}

// Prefix grouping is a hand-authored affordance: `update` must seed concrete
// module names only — a generated spec never contains a `*` claim.
#[test]
fn update_seed_emits_concrete_module_names_never_prefix_patterns() {
    let fixture = commands_fixture();
    let output = fixture.run(&["update"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );

    let spec = fixture.read("architecture.spec.toml");
    for line in spec.lines().filter(|line| line.contains("modules = [")) {
        assert!(
            !line.contains('*'),
            "seeded module claims must be literal names:\n{spec}"
        );
    }
    assert!(
        spec.contains("matches = { modules = [\"app::commands\"] }"),
        "the commands subtree seeds as one concrete top-level name:\n{spec}"
    );
}

// The seed must be self-consistent: `verify` against the just-generated seed
// reports no violations (exit 0). Module edges are seeded on the module
// boundaries, so nothing the tree does today is flagged.
#[test]
fn update_seed_verifies_clean_against_itself() {
    let fixture = single_crate_with_internal_modules_fixture();
    let run = fixture.run(&["update"]);
    assert_eq!(run.status.code(), Some(0), "update must exit 0");

    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed must verify clean (exit 0): {}",
        stdout(&verify)
    );
    let out = stdout(&verify);
    assert!(
        out.contains("matches source model") && !out.contains("violation"),
        "verify must report the seed matches, no violations:\n{out}"
    );
}

// acceptance #19: a crate whose lib.rs declares a module gated under
// `#[cfg(feature = "tauri")]` -> the seed emits a `feature_boundary` constraint
// naming `tauri` and the gated module, and `verify` against the seed exits 0.
// This tree has no edges between its boundaries, so the seeded global
// `no_cycles` guard is vacuous (documented warning, never a violation).
#[test]
fn update_seed_emits_feature_boundary_for_gated_module_and_verifies_clean() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "#[cfg(feature = \"tauri\")]\nmod tauri;\n");
    fixture.write("src/tauri.rs", "pub fn tauri() {}\n");

    let output = fixture.run(&["update"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );

    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("type = \"feature_boundary\""),
        "seed must emit a feature_boundary constraint:\n{spec}"
    );
    assert!(
        spec.contains("feature = \"tauri\""),
        "seed must name the gated feature:\n{spec}"
    );
    assert!(
        spec.contains("gated_modules = [\"app::tauri\"]"),
        "seed must name the gated module:\n{spec}"
    );
    assert!(
        spec.contains("name = \"app::tauri\""),
        "seed must still declare the gated module boundary:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed with feature_boundary must verify clean (exit 0): {}",
        stdout(&verify)
    );
    let out = stdout(&verify);
    assert!(
        !out.contains("violation")
            && !out.contains("disallowed")
            && !out.contains("missing edge")
            && !out.contains("cycle:"),
        "verify must report no violations:\n{out}"
    );
    assert!(
        out.contains("vacuous constraint") && out.contains("no_cycles"),
        "edge-free seed must report the vacuous global cycle guard:\n{out}"
    );
}

// acceptance #18 (regression for the coarse-seeding bug): a nested single-crate
// tree — `commands::start` reaching `config`, with a `::tests` module — must
// seed a seed that verifies clean against itself. A per-leaf seed would let the
// `commands` boundary shadow `commands::start` and fire false `missing edge`
// findings; the coarse seed must not.
#[test]
fn update_nested_seed_verifies_clean_against_itself() {
    let fixture = commands_fixture();
    let run = fixture.run(&["update"]);
    assert_eq!(run.status.code(), Some(0), "update must exit 0");

    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "nested seed must verify clean (exit 0): {}",
        stdout(&verify)
    );
    let out = stdout(&verify);
    assert!(
        out.contains("matches source model")
            && !out.contains("violation")
            && !out.contains("missing edge"),
        "nested seed must match with no missing edge findings:\n{out}"
    );
}

// acceptance #5: run once, then run again unchanged -> byte-identical spec.
// The overwrite guard makes a second bare `update` on the same dir impossible
// (that is #7), so the faithful realization is two identical fixtures (init #8
// precedent). A bare `update` from the fixture cwd prints `./architecture.spec.toml`,
// so the summaries are byte-identical too.
#[test]
fn update_behavior_5_identical_fixtures_byte_identical_spec_and_summary() {
    let first = workspace_fixture();
    let second = workspace_fixture();
    let first_run = first.run(&["update"]);
    let second_run = second.run(&["update"]);

    assert_eq!(first_run.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second_run.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        first.read("architecture.spec.toml"),
        second.read("architecture.spec.toml"),
        "identical trees must produce byte-identical specs"
    );
    assert_eq!(
        stdout(&first_run),
        stdout(&second_run),
        "summaries must be byte-identical"
    );
    assert!(
        stderr(&first_run).is_empty() && stderr(&second_run).is_empty(),
        "no stderr on success"
    );
}

// acceptance #5, alternate reading: the same unchanged dir re-run with --force
// (which bypasses the #7 guard) must also produce byte-identical output.
#[test]
fn update_behavior_5_reupdate_same_dir_with_force_is_byte_identical() {
    let fixture = workspace_fixture();
    let first = fixture.run(&["update"]);
    let before = fixture.read("architecture.spec.toml");
    let second = fixture.run(&["update", "--force"]);

    assert_eq!(first.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second.status.code(), Some(0), "force re-run must exit 0");
    assert_eq!(
        fixture.read("architecture.spec.toml"),
        before,
        "spec must be byte-identical across runs"
    );
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "summaries must be byte-identical"
    );
}

// acceptance #6: --force succeeds and regenerates a valid spec.
#[test]
fn update_behavior_6_force_creates_valid_spec() {
    let fixture = workspace_fixture();
    let output = fixture.run(&["update", "--force"]);

    assert_eq!(output.status.code(), Some(0), "update --force must exit 0");
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.starts_with("[project]\nlanguage = \"rust\""),
        "spec must be valid and declare the language:\n{spec}"
    );
    assert!(
        spec.contains("[[module]]"),
        "spec must declare modules:\n{spec}"
    );
    let out = stdout(&output);
    assert!(
        out.contains("generated") && out.contains("verify"),
        "summary must name the file and verify:\n{out}"
    );
}

// acceptance #7: an existing spec blocks a bare `update`; the file is untouched.
#[test]
fn update_behavior_7_existing_spec_blocks_bare_update() {
    let fixture = workspace_fixture();
    let first = fixture.run(&["update"]);
    assert_eq!(first.status.code(), Some(0), "seed run must exit 0");
    let before = fixture.read("architecture.spec.toml");

    let second = fixture.run(&["update"]);
    assert_ne!(second.status.code(), Some(0), "second run must fail");
    let err = stderr(&second);
    assert!(
        err.contains("spec already exists")
            && err.contains("architecture.spec.toml")
            && err.contains("--force"),
        "message must name the existing spec and --force:\n{err}"
    );
    assert!(stdout(&second).is_empty(), "no stdout on error");
    assert_eq!(
        fixture.read("architecture.spec.toml"),
        before,
        "existing spec must be byte-identical to before"
    );
}

// acceptance #8: --force regenerates the spec from the current tree.
#[test]
fn update_behavior_8_force_regenerates_after_tree_change() {
    let fixture = workspace_fixture();
    let first = fixture.run(&["update"]);
    assert_eq!(first.status.code(), Some(0), "seed run must exit 0");
    let before = fixture.read("architecture.spec.toml");

    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\", \"crates/audit\"]\n",
    );
    fixture.write(
        "crates/audit/Cargo.toml",
        "[package]\nname = \"audit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/audit/src/lib.rs", "pub fn audit() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\naudit = { path = \"../audit\" }\n",
    );

    let second = fixture.run(&["update", "--force"]);
    assert_eq!(
        second.status.code(),
        Some(0),
        "force re-run must exit 0 (stderr: {})",
        stderr(&second)
    );
    let after = fixture.read("architecture.spec.toml");
    assert_ne!(after, before, "spec must reflect the changed tree");
    assert!(
        after.contains("name = \"audit\""),
        "new module present:\n{after}"
    );
    assert!(
        after.contains("depend_on = [\"audit\", \"auth\"]"),
        "edge to the new crate present:\n{after}"
    );
}

// acceptance #9: missing path.
#[test]
fn update_behavior_9_missing_path_errors() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["update", "no/such/dir"]);

    assert_ne!(output.status.code(), Some(0), "must exit non-zero");
    assert!(
        stderr(&output).contains("path does not exist: no/such/dir"),
        "must identify the missing path:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no stdout on error");
}

// acceptance #10: path is a regular file.
#[test]
fn update_behavior_10_regular_file_path_errors() {
    let fixture = common::Fixture::new();
    fixture.write("plain.txt", "hello\n");
    let output = fixture.run(&["update", "plain.txt"]);

    assert_ne!(output.status.code(), Some(0), "must exit non-zero");
    assert!(
        stderr(&output).contains("path is not a directory: plain.txt"),
        "must identify the file and why it is rejected:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no stdout on error");
}

// acceptance #11: no supported-language sources.
#[test]
fn update_behavior_11_no_sources_errors() {
    let fixture = common::Fixture::new();
    fixture.write("README.md", "# nothing here\n");
    let output = fixture.run(&["update"]);

    assert_ne!(output.status.code(), Some(0), "must exit non-zero");
    assert!(
        stderr(&output).contains("no supported-language sources found under: ."),
        "must say no sources and where:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no stdout on error");
    assert!(
        !fixture.path("architecture.spec.toml").exists(),
        "no spec may be written"
    );
}

// acceptance #12: a source file with invalid syntax aborts the run.
#[test]
fn update_behavior_12_parse_failure_names_file_and_writes_no_spec() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn broken( {\n");
    let output = fixture.run(&["update"]);

    assert_ne!(output.status.code(), Some(0), "must exit non-zero");
    let err = stderr(&output);
    assert!(
        err.contains("failed to parse source:") && err.contains("lib.rs"),
        "must name the failing file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no stdout on error");
    assert!(
        !fixture.path("architecture.spec.toml").exists(),
        "no spec may be written for a partially parsed tree"
    );
}

// acceptance #13: missing driver/toolchain suggests archspec doctor.
#[test]
fn update_behavior_13_missing_driver_suggests_doctor() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_archspec"))
        .arg("update")
        .current_dir(&fixture.root)
        .env("ARCHSPEC_DISABLE_DRIVERS", "go")
        .output()
        .expect("run archspec");

    assert_ne!(output.status.code(), Some(0), "must exit non-zero");
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("go toolchain not found") && err.contains("archspec doctor"),
        "must name the driver and suggest doctor:\n{err}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).is_empty(),
        "no stdout on error"
    );
    assert!(
        !fixture.path("architecture.spec.toml").exists(),
        "no spec may be written"
    );
}

// acceptance #14: more than one positional path.
#[test]
fn update_behavior_14_two_paths_errors() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["update", "one", "two"]);

    assert_ne!(output.status.code(), Some(0), "must exit non-zero");
    assert!(
        stderr(&output).contains("expected at most one path argument"),
        "must say only one path is accepted:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no stdout on error");
}

// acceptance #18 (acme-lib scenario): a crate whose root declares a module
// gated under `#[cfg(feature = "tauri")]` AND the root re-exports from it
// (`pub use tauri::plugin::tauri_plugin`) produces a soft edge whose SOURCE is
// the crate root (`acme-lib -> acme-lib::tauri`). The seed must allow that
// edge on the crate-root unit boundary, so `verify` against the seed exits 0.
#[test]
fn update_seed_allows_root_to_gated_module_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"acme-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "#[cfg(feature = \"tauri\")]\npub mod tauri;\npub use tauri::plugin::tauri_plugin;\n",
    );
    fixture.write("src/tauri.rs", "pub mod plugin;\n");
    fixture.write("src/tauri/plugin.rs", "pub fn tauri_plugin() {}\n");

    let run = fixture.run(&["update"]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&run)
    );

    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"acme-lib\"") && spec.contains("depend_on = [\"acme-lib::tauri\"]"),
        "seed must allow the crate-root boundary to reach the gated module:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed must verify clean (exit 0): {}",
        stdout(&verify)
    );
    let out = stdout(&verify);
    assert!(
        out.contains("matches source model") && !out.contains("violation"),
        "seed must match source model, no violations:\n{out}"
    );
}

// acceptance #18 (symmetric case): a module edge whose TARGET is the crate root
// (`app::commands::index -> app`) must be allowed on the source boundary, so
// `verify` against the seed exits 0.
#[test]
fn update_seed_allows_submodule_to_root_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod commands;\npub fn run() {}\n");
    fixture.write("src/commands.rs", "mod index;\n");
    fixture.write("src/commands/index.rs", "use crate;\n");

    let run = fixture.run(&["update"]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&run)
    );

    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"app::commands\"") && spec.contains("depend_on = [\"app\"]"),
        "seed must allow the commands boundary to reach the crate root:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed must verify clean (exit 0): {}",
        stdout(&verify)
    );
    let out = stdout(&verify);
    assert!(
        out.contains("matches source model") && !out.contains("violation"),
        "seed must match source model, no violations:\n{out}"
    );
}

// The seed must carry the global cycle guard: one `no_cycles` constraint with
// no `modules` list (global = all declared modules), emitted exactly once and
// identical across repeated runs on the same tree.
#[test]
fn update_seed_includes_global_no_cycles_constraint_deterministically() {
    let fixture = workspace_fixture();
    let first = fixture.run(&["update"]);
    assert_eq!(first.status.code(), Some(0), "update must exit 0");

    let spec = fixture.read("architecture.spec.toml");
    assert_eq!(
        spec.matches("[[constraint]]\ntype = \"no_cycles\"\n").count(),
        1,
        "global no_cycles stanza must appear exactly once:\n{spec}"
    );
    assert!(
        !spec.contains("modules"),
        "global stanza must not pin a module list:\n{spec}"
    );

    let second = fixture.run(&["update", "--force"]);
    assert_eq!(second.status.code(), Some(0), "update --force must exit 0");
    assert_eq!(
        fixture.read("architecture.spec.toml"),
        spec,
        "seed with the global stanza must stay deterministic"
    );
}

// A cyclic tree must NOT be blessed by the seed: the seeded global no_cycles
// makes `verify` fail and name the cycle.
#[test]
fn update_seed_of_cyclic_workspace_fails_verify_with_cycle() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");

    let run = fixture.run(&["update"]);
    assert_eq!(run.status.code(), Some(0), "update must exit 0");
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("[[constraint]]\ntype = \"no_cycles\"\n"),
        "seed must carry the global cycle guard:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    assert_ne!(
        verify.status.code(),
        Some(0),
        "cyclic seed must fail verify: {}",
        stdout(&verify)
    );
    let out = stdout(&verify);
    assert!(
        out.contains("cycle: auth -> billing -> auth"),
        "verify must name the cycle:\n{out}"
    );
}

// F8: the seed must reflect the cfg-based rule, not a name-based one. A module
// literally named `tests` that is NOT `#[cfg(test)]` is production and seeds a
// boundary like any other top-level module; a `#[cfg(test)] mod tests` stays
// scaffolding and seeds none.
fn top_level_tests_fixture(gated: bool) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let lib = if gated {
        "mod prod;\n#[cfg(test)]\nmod tests;\n"
    } else {
        "mod prod;\nmod tests;\n"
    };
    fixture.write("src/lib.rs", lib);
    fixture.write("src/prod.rs", "pub fn prod() {}\n");
    fixture.write("src/tests.rs", "use crate::prod;\npub fn t() {}\n");
    fixture
}

#[test]
fn update_seed_includes_non_cfg_tests_boundary() {
    let fixture = top_level_tests_fixture(false);
    let output = fixture.run(&["update"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"app::tests\""),
        "a production (non-cfg) tests module must seed a boundary:\n{spec}"
    );
    assert!(
        spec.contains("allowed = { depend_on = [\"app::prod\"] }"),
        "the tests boundary must record its production dependency:\n{spec}"
    );
}

#[test]
fn update_seed_excludes_cfg_test_tests_boundary() {
    let fixture = top_level_tests_fixture(true);
    let output = fixture.run(&["update"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));

    let spec = fixture.read("architecture.spec.toml");
    assert!(
        !spec.contains("app::tests"),
        "a cfg(test) tests module must seed no boundary:\n{spec}"
    );
    assert!(
        spec.contains("name = \"app::prod\""),
        "the production boundary must still seed:\n{spec}"
    );
}

/// The canonical two-member go.work workspace (api with root + nested
/// internal/handler packages, store with one package), the same shape the
/// scan and depgraph workspace tests materialize.
fn go_workspace_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write(
        "api/api.go",
        "package api\n\nimport \"example.com/store\"\n\nfunc Api() {}\n",
    );
    fixture.write(
        "api/internal/handler/handler.go",
        "package handler\n\nimport (\n\t\"fmt\"\n\n\t\"example.com/store\"\n)\n\nfunc Handle() {}\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");
    fixture
}

/// A go.work tree carries the module tier natively, so the seed declares ONE
/// module per workspace member (the way csharp projects seed) with
/// depend_on from the real cross-member imports — not one module per package.
#[test]
fn update_seed_go_workspace_seeds_one_module_per_member() {
    let fixture = go_workspace_fixture();
    let output = fixture.run(&["update"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );

    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"example.com/api\"")
            && spec.contains("name = \"example.com/store\""),
        "seed must declare one module per workspace member:\n{spec}"
    );
    assert_eq!(
        spec.matches("[[module]]").count(),
        2,
        "members are the module tier — packages must not seed extra modules:\n{spec}"
    );
    assert!(
        spec.contains(
            "matches = { units = [\"example.com/api\", \"example.com/api/internal/handler\"] }"
        ),
        "the api member boundary must claim exactly its packages:\n{spec}"
    );
    assert!(
        spec.contains("allowed = { depend_on = [\"example.com/store\"] }"),
        "the api member must depend_on the store member it imports:\n{spec}"
    );
    assert!(
        !spec.contains("name = \"example.com/api/internal/handler\""),
        "packages inside a member must not become modules of their own:\n{spec}"
    );
}

/// Contract: verify on the seed of the go.work tree is clean — zero missing
/// edges, every finding category empty, under --strict too.
#[test]
fn update_seed_go_workspace_verifies_clean() {
    let fixture = go_workspace_fixture();
    let output = fixture.run(&["update"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    let verify = fixture.run(&["verify", "--strict"]);
    let out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed must verify clean under --strict:\n{out}"
    );
    assert!(
        out.contains("matches source model") && !out.contains("missing edge"),
        "seeded workspace spec must verify with zero missing edges:\n{out}"
    );
}

/// A single-go.mod tree offers no native module fact, so the seed keeps
/// today's per-package shape — and that shape still meets the seed contract:
/// verify on the seed reports zero missing edges.
#[test]
fn update_seed_go_single_module_keeps_shape_and_verifies_clean() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "a/a.go",
        "package a\n\nimport \"example.com/demo/shell\"\n\nfunc A() {}\n",
    );
    fixture.write(
        "shell/shell.go",
        "package shell\n\nimport \"example.com/demo/store\"\n\nfunc Call() {}\n",
    );
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");

    let output = fixture.run(&["update"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );

    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"example.com/demo/a\"")
            && spec.contains("matches = { units = [\"example.com/demo/a\"] }")
            && spec.contains("allowed = { depend_on = [\"example.com/demo/shell\"] }")
            && spec.contains("allowed = { depend_on = [\"example.com/demo/store\"] }"),
        "single-module go must keep today's per-package seed shape:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    let out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed must verify clean (stderr: {})",
        stderr(&verify)
    );
    assert!(
        out.contains("matches source model") && !out.contains("missing edge"),
        "seeded single-module spec must verify with zero missing edges:\n{out}"
    );
}

/// C# projects are units by dotted name (`HomeBudget.Api`) while namespaces
/// convert to `::` paths (`HomeBudget::Api::Controllers`), so the top-level
/// fold yields entries like `name = "HomeBudget::Api"`,
/// `matches = { modules = ["HomeBudget::Api"] }` — self-listing, no targets,
/// referenced by no kept entry, owning no module-edge endpoint, first segment
/// naming no unit: they declare no boundary pairs and float in diagrams. The
/// seed must not emit them. The folds here carry no `using`s, so no module
/// edge has an endpoint inside them. A single csproj-level `ProjectReference`
/// gives the global `no_cycles` guard a real edge, so dropping the folds is
/// literally warning-free — a clean `ok:` match, not a vacuous-guard warning.
#[test]
fn update_seed_drops_self_matching_namespace_modules_for_dotted_projects() {
    let fixture = common::Fixture::new();
    let csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n";
    let api_csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\HomeBudget.Core\\HomeBudget.Core.csproj\" />\n  </ItemGroup>\n</Project>\n";
    fixture.write("HomeBudget.Api/HomeBudget.Api.csproj", api_csproj);
    fixture.write(
        "HomeBudget.Api/C.cs",
        "namespace HomeBudget.Api.Controllers;\npublic class C { }\n",
    );
    fixture.write(
        "HomeBudget.Api/M.cs",
        "namespace HomeBudget.Api.Models;\npublic class M { }\n",
    );
    fixture.write("HomeBudget.Core/HomeBudget.Core.csproj", csproj);
    fixture.write(
        "HomeBudget.Core/D.cs",
        "namespace HomeBudget.Core.Domain;\npublic class D { }\n",
    );

    let output = fixture.run(&["update"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"HomeBudget.Api\"")
            && spec.contains("name = \"HomeBudget.Core\""),
        "project-level unit boundaries must still seed:\n{spec}"
    );
    assert!(
        !spec.contains("HomeBudget::Api") && !spec.contains("HomeBudget::Core"),
        "self-listing namespace-module entries must not seed:\n{spec}"
    );

    // Language-neutral rule: a module whose matches list only its own name and
    // which declares no allowed deps must still nest under a declared unit, be
    // referenced by a kept boundary's allowed list, or own a module-edge
    // endpoint — otherwise it matches nothing but itself, declares no boundary
    // pairs, and owns no endpoints. Here the folds are unreferenced and carry
    // no edges, so none may seed.
    let mut unit_names: Vec<String> = Vec::new();
    let mut self_listed: Vec<String> = Vec::new();
    for block in spec.split("[[module]]").skip(1) {
        let block = block.split("[[constraint]]").next().unwrap_or(block);
        let name = block
            .lines()
            .find_map(|line| line.strip_prefix("name = \""))
            .and_then(|rest| rest.split('"').next())
            .expect("module block carries a name");
        let Some(matches) = block
            .lines()
            .find_map(|line| line.strip_prefix("matches = { modules = ["))
        else {
            if block.contains("matches = { units = [") {
                unit_names.push(name.to_string());
            }
            continue;
        };
        let listed: Vec<&str> = matches
            .trim_end_matches("] }")
            .split('"')
            .filter(|s| !s.is_empty() && *s != ", ")
            .collect();
        if listed == [name] && !block.contains("allowed =") {
            self_listed.push(name.to_string());
        }
    }
    for entry in &self_listed {
        assert!(
            unit_names
                .iter()
                .any(|unit| entry.starts_with(&format!("{unit}::"))),
            "self-listing module `{entry}` matches nothing but itself and must not seed:\n{spec}"
        );
    }

    let verify = fixture.run(&["verify"]);
    let out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed must verify clean against itself (stderr: {})",
        stderr(&verify)
    );
    assert!(
        out.contains("ok: architecture.spec.toml matches source model")
            && !out.contains("does not match source model"),
        "drop-side seed must report a clean match, not a does-not-match or \
         vacuous-guard verdict:\n{out}"
    );
    assert!(
        !out.contains("warning:"),
        "drop-side seed must be literally warning-free (the ProjectReference \
         keeps the global no_cycles guard non-vacuous):\n{out}"
    );
}

/// Dotted C# tree where a KEPT fold's `allowed.depend_on` names a SINK fold:
/// the unit `HomeBudget.Api` declares `HomeBudget.Api.Controllers` (folds to
/// `HomeBudget::Api`) and `HomeBudget.Domain` (folds to `HomeBudget::Domain`,
/// first segment naming no unit), with an in-project using from the former to
/// the latter. The kept fold seeds a boundary naming the sink as a dependency;
/// the sink itself has no targets and no unit prefix. Dropping it would leave
/// the kept entry with a `dead reference` to an undeclared name and the module
/// edge with an `unowned module edge endpoint` — `verify` resolves
/// `allowed.depend_on` against declared boundary names and owns edge endpoints
/// through `matches.modules`. The seed must keep referenced sink folds.
#[test]
fn update_seed_keeps_sink_folds_referenced_by_kept_boundaries() {
    let fixture = common::Fixture::new();
    let csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n";
    fixture.write("HomeBudget.Api/HomeBudget.Api.csproj", csproj);
    fixture.write(
        "HomeBudget.Api/C.cs",
        "using HomeBudget.Domain;\nnamespace HomeBudget.Api.Controllers;\npublic class C { }\n",
    );
    fixture.write(
        "HomeBudget.Api/D.cs",
        "namespace HomeBudget.Domain;\npublic class D { }\n",
    );

    let output = fixture.run(&["update"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"HomeBudget::Api\"")
            && spec.contains("allowed = { depend_on = [\"HomeBudget::Domain\"] }"),
        "the referencing fold must seed a boundary with its sink dependency:\n{spec}"
    );
    assert!(
        spec.contains("name = \"HomeBudget::Domain\"")
            && spec.contains("matches = { modules = [\"HomeBudget::Domain\"] }"),
        "a fold referenced as a dependency target must seed a boundary:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    let out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed with sink fold must verify clean (exit 0): {out}"
    );
    assert!(
        !out.contains("dead reference") && !out.contains("unowned module edge endpoint"),
        "sink fold seeding must not leave dead-reference or unowned warnings:\n{out}"
    );
}

/// Dotted C# tree whose module edges live INSIDE a unit across namespaces
/// (`HomeBudget.Api` project, `HomeBudget::Api::Controllers` using
/// `HomeBudget::Api::Models`). The fold `HomeBudget::Api` has no cross-fold
/// targets (the intra-fold edge collapses), nests under no unit (dotted unit
/// name vs `::` path), and is named by no kept entry's `allowed.depend_on`.
/// Dropping it would leave every endpoint of the intra-unit edges unowned:
/// `verify` owns module-edge endpoints through `matches.modules`, so a fold
/// that is the top-level of any edge endpoint must seed. The cross-unit
/// `ProjectReference` gives the global `no_cycles` guard a real edge so its
/// vacuity never masks the finding. The seeded spec must verify with zero
/// warning lines, not merely exit 0.
#[test]
fn update_seed_keeps_folds_owning_intra_unit_module_edge_endpoints() {
    let fixture = common::Fixture::new();
    let csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n";
    let api_csproj = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"..\\HomeBudget.Domain\\HomeBudget.Domain.csproj\" />\n  </ItemGroup>\n</Project>\n";
    fixture.write("HomeBudget.Api/HomeBudget.Api.csproj", api_csproj);
    fixture.write(
        "HomeBudget.Api/C.cs",
        "using HomeBudget.Api.Models;\nnamespace HomeBudget.Api.Controllers;\npublic class C { }\n",
    );
    fixture.write(
        "HomeBudget.Api/M.cs",
        "namespace HomeBudget.Api.Models;\npublic class M { }\n",
    );
    fixture.write("HomeBudget.Domain/HomeBudget.Domain.csproj", csproj);
    fixture.write(
        "HomeBudget.Domain/E.cs",
        "namespace HomeBudget.Domain.Entities;\npublic class E { }\n",
    );

    let output = fixture.run(&["update"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "update must exit 0 (stderr: {})",
        stderr(&output)
    );
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("name = \"HomeBudget::Api\"")
            && spec.contains("matches = { modules = [\"HomeBudget::Api\"] }"),
        "the fold owning the intra-unit edge endpoints must seed a boundary:\n{spec}"
    );

    let verify = fixture.run(&["verify"]);
    let out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "seed with endpoint-owning fold must verify clean (exit 0): {out}"
    );
    assert!(
        out.contains("ok: architecture.spec.toml matches source model")
            && !out.contains("does not match source model"),
        "seeded dotted-csharp spec must report a clean match, not a \
         does-not-match or vacuous-guard verdict:\n{out}"
    );
    assert!(
        !out.contains("warning:"),
        "seeded dotted-csharp spec must carry zero warning lines:\n{out}"
    );
}

// Purity is an architectural claim, not a snapshot fact: a seed must never
// auto-generate `external_free`, not even from a genuinely pure leaf crate
// (that would fabricate intent the author never declared).
#[test]
fn update_seed_never_autogenerates_external_free() {
    let fixture = workspace_fixture();
    let output = fixture.run(&["update"]);
    assert_eq!(output.status.code(), Some(0), "update must exit 0");
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        !spec.contains("external_free"),
        "the seed must not fabricate purity guards from pure leaves:\n{spec}"
    );
}
