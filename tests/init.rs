mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use common::{stderr, stdout};

fn running_as_root() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
        .unwrap_or(false)
}

fn rust_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    fixture
}

fn go_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture
}

fn csproj_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "demo/Orders.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture
}

fn sln_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "demo/Orders.sln",
        "Microsoft Visual Studio Solution File, Format Version 12.00\n# Visual Studio Version 17\n",
    );
    fixture
}

/// The exact `[output]` defaults scaffolded by `init` into `archspec.toml`.
const OUTPUT_CONFIG_CONTENT: &str = "[output]\ninspect = \"docs/archspec/inspect.mmd\"\ndiagram = \"docs/archspec/diagram.mmd\"\nreport  = \"docs/archspec/report.md\"\nscan    = \"docs/archspec/scan.json\"\n";

#[test]
fn init_scenario_one_creates_spec_in_explicit_rust_dir() {
    let fixture = common::Fixture::new();
    fixture.write(
        "demo/Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    let output = fixture.run(&["init", "demo"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        fixture.path("demo/architecture.spec.toml").exists(),
        "spec must be created in the target dir"
    );
    let spec = fixture.read("demo/architecture.spec.toml");
    assert!(spec.contains("[project]"), "missing [project]:\n{spec}");
    assert!(
        spec.contains("language = \"rust\""),
        "missing language = rust:\n{spec}"
    );
    assert_eq!(
        stdout(&output),
        "created architecture.spec.toml\ncreated archspec.toml\nlanguage: rust\nnext step: run archspec update to seed boundaries from the tree, then edit, then verify\n",
        "summary must match the output contract exactly"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn init_scenario_two_creates_go_spec_in_explicit_go_dir() {
    let fixture = common::Fixture::new();
    fixture.write("demo/go.mod", "module example.com/demo\ngo 1.21\n");
    let output = fixture.run(&["init", "demo"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        fixture.path("demo/architecture.spec.toml").exists(),
        "spec must be created in the target dir"
    );
    let spec = fixture.read("demo/architecture.spec.toml");
    assert!(spec.contains("[project]"), "missing [project]:\n{spec}");
    assert!(
        spec.contains("language = \"go\""),
        "missing language = go:\n{spec}"
    );
    assert_eq!(
        stdout(&output),
        "created architecture.spec.toml\ncreated archspec.toml\nlanguage: go\nnext step: run archspec update to seed boundaries from the tree, then edit, then verify\n",
        "summary must match the output contract exactly"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn init_scenario_three_creates_csharp_spec_from_csproj() {
    let fixture = csproj_fixture();
    let output = fixture.run(&["init", "demo"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        fixture.path("demo/architecture.spec.toml").exists(),
        "spec must be created in the target dir"
    );
    let spec = fixture.read("demo/architecture.spec.toml");
    assert!(spec.contains("[project]"), "missing [project]:\n{spec}");
    assert!(
        spec.contains("language = \"csharp\""),
        "missing language = csharp:\n{spec}"
    );
    assert_eq!(
        stdout(&output),
        "created architecture.spec.toml\ncreated archspec.toml\nlanguage: csharp\nnext step: run archspec update to seed boundaries from the tree, then edit, then verify\n",
        "summary must match the output contract exactly"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn init_scenario_three_detects_csharp_from_sln() {
    let fixture = sln_fixture();
    let output = fixture.run(&["init", "demo"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        fixture.path("demo/architecture.spec.toml").exists(),
        "spec must be created in the target dir"
    );
    let spec = fixture.read("demo/architecture.spec.toml");
    assert!(spec.contains("[project]"), "missing [project]:\n{spec}");
    assert!(
        spec.contains("language = \"csharp\""),
        "missing language = csharp:\n{spec}"
    );
    assert_eq!(
        stdout(&output),
        "created architecture.spec.toml\ncreated archspec.toml\nlanguage: csharp\nnext step: run archspec update to seed boundaries from the tree, then edit, then verify\n",
        "summary must match the output contract exactly"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn init_two_identical_go_projects_are_byte_identical() {
    let first = go_fixture();
    let second = go_fixture();
    let o1 = first.run(&["init"]);
    let o2 = second.run(&["init"]);

    assert_eq!(o1.status.code(), Some(0));
    assert_eq!(o2.status.code(), Some(0));
    assert_eq!(
        first.read("architecture.spec.toml"),
        second.read("architecture.spec.toml"),
        "identical go projects must produce byte-identical specs"
    );
    assert_eq!(stdout(&o1), stdout(&o2), "summaries must be byte-identical");
}

#[test]
fn init_refuses_to_overwrite_existing_go_spec() {
    let fixture = go_fixture();
    fixture.write("architecture.spec.toml", "existing content\n");
    let output = fixture.run(&["init"]);

    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(
        err.contains("architecture.spec.toml already exists: ./architecture.spec.toml"),
        "must identify the existing file path:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
    assert_eq!(
        fixture.read("architecture.spec.toml"),
        "existing content\n",
        "existing spec must not be touched"
    );
}

#[test]
fn init_dot_targets_cwd() {
    let fixture = rust_fixture();
    let output = fixture.run(&["init", "."]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        "created architecture.spec.toml\ncreated archspec.toml\nlanguage: rust\nnext step: run archspec update to seed boundaries from the tree, then edit, then verify\n",
        "summary must match the output contract exactly"
    );
    assert!(fixture.path("architecture.spec.toml").exists());
    let spec = fixture.read("architecture.spec.toml");
    assert!(spec.contains("language = \"rust\""));
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn init_stdout_summary_is_exact() {
    let fixture = rust_fixture();
    let output = fixture.run(&["init"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        "created architecture.spec.toml\ncreated archspec.toml\nlanguage: rust\nnext step: run archspec update to seed boundaries from the tree, then edit, then verify\n",
        "summary must match the output contract exactly"
    );
}

#[test]
fn init_creates_spec_and_output_config_in_rust_dir() {
    let fixture = rust_fixture();
    let output = fixture.run(&["init"]);

    assert_eq!(output.status.code(), Some(0));
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("language = \"rust\""),
        "missing language = rust:\n{spec}"
    );
    assert_eq!(
        fixture.read("archspec.toml"),
        OUTPUT_CONFIG_CONTENT,
        "output config must match the standard defaults byte-for-byte"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn init_go_fixture_creates_identical_output_config() {
    let fixture = go_fixture();
    let output = fixture.run(&["init"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        fixture.path("architecture.spec.toml").exists(),
        "spec must be created in the target dir"
    );
    assert_eq!(
        fixture.read("archspec.toml"),
        OUTPUT_CONFIG_CONTENT,
        "config defaults must be language-independent"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn init_refuses_to_overwrite_existing_output_config() {
    let fixture = rust_fixture();
    fixture.write("archspec.toml", "[output]\ninspect = \"custom.mmd\"\n");
    let output = fixture.run(&["init"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("archspec.toml already exists"),
        "must name the guarded file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
    assert_eq!(
        fixture.read("archspec.toml"),
        "[output]\ninspect = \"custom.mmd\"\n",
        "existing config must not be touched"
    );
    assert!(
        !fixture.path("architecture.spec.toml").exists(),
        "no spec must be created when the config is refused"
    );
}

#[test]
fn init_materializes_cargo_workspace_profile_content() {
    let fixture = rust_fixture();
    let output = fixture.run(&["init"]);

    assert_eq!(output.status.code(), Some(0));
    let spec = fixture.read("architecture.spec.toml");
    assert_eq!(
        spec,
        "[project]\nlanguage = \"rust\"\n\n[[constraint]]\ntype = \"no_cycles\"\n",
        "spec must be the base spec plus the global no_cycles constraint:\n{spec}"
    );
    for forbidden in [
        "[[stereotype]]",
        "[[module]]",
        "[module.allowed]",
        "external_free",
    ] {
        assert!(
            !spec.contains(forbidden),
            "minimal spec must not contain {forbidden}:\n{spec}"
        );
    }
}

/// The scaffolded constraint must be the GLOBAL form: an empty (absent)
/// `modules` list already means "all declared modules" in the engine, so the
/// stanza must carry no `modules` key and no new schema fields.
#[test]
fn init_scaffolds_global_no_cycles_constraint() {
    for (fixture, language) in [(rust_fixture(), "rust"), (go_fixture(), "go")] {
        let output = fixture.run(&["init"]);
        assert_eq!(output.status.code(), Some(0), "init must exit 0");
        let spec = fixture.read("architecture.spec.toml");
        assert!(
            spec.contains("language = \"") && spec.contains(language),
            "language section missing:\n{spec}"
        );
        assert!(
            spec.contains("[[constraint]]\ntype = \"no_cycles\"\n"),
            "global no_cycles stanza missing:\n{spec}"
        );
        assert!(
            !spec.contains("modules"),
            "global stanza must not pin a module list:\n{spec}"
        );
    }
}

/// A project whose crates form a mutual dependency cycle: the scaffolded
/// global `no_cycles` must make `verify` fail once module declarations exist.
/// The module mapping is added after `init` (init scaffolds no modules).
#[test]
fn init_scaffolded_cycle_fails_verify_with_module_mapping() {
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

    let init = fixture.run(&["init"]);
    assert_eq!(
        init.status.code(),
        Some(0),
        "init must exit 0 (stderr: {})",
        stderr(&init)
    );

    let modules = "[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[module.allowed]\ndepend_on = [\"billing\"]\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n";
    let spec = fixture.read("architecture.spec.toml");
    fixture.write("architecture.spec.toml", &format!("{spec}\n{modules}"));

    let verify = fixture.run(&["verify"]);
    assert_ne!(
        verify.status.code(),
        Some(0),
        "scaffolded global no_cycles must fail a cycle: {}",
        stdout(&verify)
    );
    let out = stdout(&verify);
    assert!(
        out.contains("cycle: auth -> billing -> auth"),
        "verify must name the cycle:\n{out}"
    );
}

#[test]
fn init_rejects_missing_path() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["init", "/no/such/dir"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path does not exist: /no/such/dir"));
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
}

#[test]
fn init_rejects_regular_file_path() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\n");
    let output = fixture.run(&["init", "Cargo.toml"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path is not a directory: Cargo.toml"));
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
}

#[test]
fn init_rejects_two_positional_paths() {
    let fixture = rust_fixture();
    let output = fixture.run(&["init", "a", "b"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("expected at most one path argument"));
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
}

#[test]
fn init_rejects_unknown_flag() {
    let fixture = rust_fixture();
    let output = fixture.run(&["init", "--force"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("unknown flag: --force"));
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
    assert!(
        !fixture.path("architecture.spec.toml").exists(),
        "no spec on unknown flag"
    );
}

#[test]
fn init_rejects_undetectable_language_without_profile() {
    let fixture = common::Fixture::new();
    fixture.write("empty/README.md", "no manifests here\n");
    let output = fixture.run(&["init", "empty"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("could not detect project language: empty"),
        "must say no language detected and where:\n{err}"
    );
    assert!(
        err.contains("Cargo.toml, go.mod, or *.csproj/*.sln"),
        "must list expected signals:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
    assert!(
        !fixture.path("empty/architecture.spec.toml").exists(),
        "no spec when detection fails"
    );
}

#[test]
fn init_refuses_to_overwrite_existing_spec() {
    let fixture = rust_fixture();
    fixture.write("architecture.spec.toml", "existing content\n");
    let output = fixture.run(&["init"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("architecture.spec.toml already exists"),
        "must name the guarded file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
    assert_eq!(
        fixture.read("architecture.spec.toml"),
        "existing content\n",
        "existing spec must not be touched"
    );
}

#[test]
fn init_scenario_four_creates_spec_in_cwd() {
    let fixture = rust_fixture();
    let output = fixture.run(&["init"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(
        fixture.path("architecture.spec.toml").exists(),
        "spec must be created in the current directory"
    );
    let spec = fixture.read("architecture.spec.toml");
    assert!(
        spec.contains("language = \"rust\""),
        "missing language = rust:\n{spec}"
    );
    assert_eq!(
        stdout(&output),
        "created architecture.spec.toml\ncreated archspec.toml\nlanguage: rust\nnext step: run archspec update to seed boundaries from the tree, then edit, then verify\n",
        "summary must match the output contract exactly"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn init_scenario_eight_identical_rust_specs_no_args() {
    let first = rust_fixture();
    let second = rust_fixture();
    let o1 = first.run(&["init"]);
    let o2 = second.run(&["init"]);

    assert_eq!(o1.status.code(), Some(0), "first run must exit 0");
    assert_eq!(o2.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        first.read("architecture.spec.toml"),
        second.read("architecture.spec.toml"),
        "identical rust projects must produce byte-identical specs"
    );
    assert_eq!(stdout(&o1), stdout(&o2), "summaries must be byte-identical");
    assert!(
        stderr(&o1).is_empty() && stderr(&o2).is_empty(),
        "stderr should be empty"
    );
}

#[test]
fn init_scenario_nine_identical_rust_specs_explicit_path() {
    let first = common::Fixture::new();
    let second = common::Fixture::new();
    for fixture in [&first, &second] {
        fixture.write(
            "demo/Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        );
    }
    let o1 = first.run(&["init", "demo"]);
    let o2 = second.run(&["init", "demo"]);

    assert_eq!(o1.status.code(), Some(0), "first run must exit 0");
    assert_eq!(o2.status.code(), Some(0), "second run must exit 0");
    assert!(
        first.path("demo/architecture.spec.toml").exists()
            && second.path("demo/architecture.spec.toml").exists(),
        "spec must be created in each target dir"
    );
    assert_eq!(
        first.read("demo/architecture.spec.toml"),
        second.read("demo/architecture.spec.toml"),
        "identical projects must produce byte-identical specs"
    );
    assert_eq!(stdout(&o1), stdout(&o2), "summaries must be byte-identical");
    assert!(
        stderr(&o1).is_empty() && stderr(&o2).is_empty(),
        "stderr should be empty"
    );
}

#[test]
fn init_scenario_sixteen_reports_unwritable_target_dir() {
    // chmod does not restrict root; under root the write succeeds and the
    // "cannot write spec" path is never reached. Skip to avoid CI flakiness.
    if running_as_root() {
        return;
    }

    let fixture = common::Fixture::new();
    fixture.write(
        "guarded/Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    let target = fixture.path("guarded");
    struct RestoreOnDrop {
        path: std::path::PathBuf,
    }
    impl Drop for RestoreOnDrop {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755));
        }
    }
    fs::set_permissions(&target, fs::Permissions::from_mode(0o555))
        .expect("make target dir non-writable");
    let _guard = RestoreOnDrop {
        path: target.clone(),
    };
    let output = fixture.run(&["init", "guarded"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("cannot write spec"),
        "must say the spec could not be written:\n{err}"
    );
    assert!(
        err.contains("guarded/architecture.spec.toml"),
        "must identify the spec path:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "stdout must be empty on error");
    assert!(
        !fixture.path("guarded/architecture.spec.toml").exists(),
        "no spec file must be created on write failure"
    );
}
