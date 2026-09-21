#![cfg(unix)]

mod common;

use common::{stderr, stdout, Fixture};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

/// Version strings the stub toolchains print; the report echoes them verbatim
/// in parentheses on the `toolchain:` line, so these doubles are also the
/// expected detail text.
const RUSTC_VERSION: &str = "rustc 1.80.1 (cargo 1.80.1)";
const DOTNET_VERSION: &str = "8.0.201";
const GO_VERSION: &str = "go version go1.22.1 linux/amd64";

/// One language block: driver capability is stated FIRST and independently of
/// the toolchain probe (commands/doctor/output.md) — the in-binary parse-only
/// driver is present on every run of this suite (no driver is env-disabled),
/// regardless of which toolchains answer on PATH.
fn block(language: &str, toolchain: Option<&str>) -> String {
    match toolchain {
        Some(version) => format!(
            "{language}\n  driver: present (in-binary, parse-only)\n  toolchain: present ({version})"
        ),
        None => format!(
            "{language}\n  driver: present (in-binary, parse-only)\n  toolchain: absent\n  note: scanning {language} does not require the toolchain; the driver parses sources in-binary"
        ),
    }
}

fn report(blocks: &[&str], summary: &str) -> String {
    format!(
        "{}\n\nScannable languages: {summary}\n",
        blocks.join("\n\n")
    )
}

/// A temp `bin` dir containing only the given stub executables. Each stub is a
/// `#!/bin/sh` script that prints a fixed version string and exits 0, so a
/// "present" toolchain is a stub on PATH and an "absent" one simply has no
/// binary there.
fn stub_bin_dir(stubs: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("temp bin dir");
    for name in stubs {
        let body = match *name {
            "rustc" => format!("#!/bin/sh\necho \"{RUSTC_VERSION}\"\n"),
            "dotnet" => format!("#!/bin/sh\necho \"{DOTNET_VERSION}\"\n"),
            "go" => format!("#!/bin/sh\necho \"{GO_VERSION}\"\n"),
            other => panic!("unknown stub: {other}"),
        };
        let path = dir.path().join(name);
        fs::write(&path, body).expect("write stub");
        let mut perms = fs::metadata(&path).expect("stub metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("make stub executable");
    }
    dir
}

/// Run the real `archspec` binary with a controlled PATH (only `bin_dir`),
/// exactly as the acceptance scenarios drive a controlled machine.
fn run_with_path(cwd: &Path, bin_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_archspec"))
        .args(args)
        .current_dir(cwd)
        .env("PATH", bin_dir)
        .env_remove("ARCHSPEC_DOCTOR_FAIL")
        .output()
        .expect("run archspec")
}

/// Acceptance #1: rust, csharp, and go toolchains present; both facts of
/// every language read present.
#[test]
fn doctor_all_toolchains_present() {
    let bin = stub_bin_dir(&["rustc", "dotnet", "go"]);
    let fixture = Fixture::new();

    let output = run_with_path(&fixture.root, bin.path(), &["doctor"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        report(
            &[
                block("rust", Some(RUSTC_VERSION)).as_str(),
                block("csharp", Some(DOTNET_VERSION)).as_str(),
                block("go", Some(GO_VERSION)).as_str(),
            ],
            "rust, csharp, go",
        )
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

/// Acceptance #2: only the rust toolchain present. Toolchain availability is
/// absent for csharp/go, yet the DRIVER capability and the scannable summary
/// stay present — the two facts never lean on each other.
#[test]
fn doctor_only_rust_present() {
    let bin = stub_bin_dir(&["rustc"]);
    let fixture = Fixture::new();

    let output = run_with_path(&fixture.root, bin.path(), &["doctor"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        report(
            &[
                block("rust", Some(RUSTC_VERSION)).as_str(),
                block("csharp", None).as_str(),
                block("go", None).as_str(),
            ],
            "rust, csharp, go",
        )
    );
    assert!(stderr(&output).is_empty(), "no stderr on success");
}

/// Acceptance #3: no go toolchain; go reads "driver present / toolchain
/// absent" with the parse-only note, and stays in the scannable summary.
#[test]
fn doctor_without_go() {
    let bin = stub_bin_dir(&["rustc", "dotnet"]);
    let fixture = Fixture::new();

    let output = run_with_path(&fixture.root, bin.path(), &["doctor"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        report(
            &[
                block("rust", Some(RUSTC_VERSION)).as_str(),
                block("csharp", Some(DOTNET_VERSION)).as_str(),
                block("go", None).as_str(),
            ],
            "rust, csharp, go",
        )
    );
}

/// Acceptance #4: no supported toolchain at all; every block reads driver
/// present / toolchain absent — scanning works from the in-binary drivers —
/// and the summary lists all three languages.
#[test]
fn doctor_no_toolchain() {
    let bin = stub_bin_dir(&[]);
    let fixture = Fixture::new();

    let output = run_with_path(&fixture.root, bin.path(), &["doctor"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        report(
            &[
                block("rust", None).as_str(),
                block("csharp", None).as_str(),
                block("go", None).as_str(),
            ],
            "rust, csharp, go",
        )
    );
}

/// Acceptance #5: a fixed environment run twice produces byte-identical output.
#[test]
fn doctor_deterministic_across_runs() {
    let bin = stub_bin_dir(&["rustc", "dotnet"]);
    let fixture = Fixture::new();

    let first = run_with_path(&fixture.root, bin.path(), &["doctor"]);
    let second = run_with_path(&fixture.root, bin.path(), &["doctor"]);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "runs must be byte-identical"
    );
    assert_eq!(stderr(&first), stderr(&second));
}

/// Acceptance #6: a fixed environment probed from two different directories
/// produces byte-identical output.
#[test]
fn doctor_deterministic_across_directories() {
    let bin = stub_bin_dir(&["rustc", "go"]);
    let fixture_a = Fixture::new();
    let fixture_b = Fixture::new();

    let from_a = run_with_path(&fixture_a.root, bin.path(), &["doctor"]);
    let from_b = run_with_path(&fixture_b.root, bin.path(), &["doctor"]);

    assert_eq!(from_a.status.code(), Some(0));
    assert_eq!(from_b.status.code(), Some(0));
    assert_eq!(
        stdout(&from_a),
        stdout(&from_b),
        "output must not depend on the invocation directory"
    );
}

/// Acceptance #7: a positional path argument is rejected with a message saying
/// a path is not accepted.
#[test]
fn doctor_rejects_positional_path() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );

    let output = fixture.run(&["doctor", "./project"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output)
            .contains("doctor takes no path argument (diagnoses the environment, not a project)"),
        "message must say a path is not accepted:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no report on error");
}

/// Acceptance #8: an unknown flag is rejected with a message naming the flag.
#[test]
fn doctor_rejects_unknown_flag() {
    let fixture = Fixture::new();

    let output = fixture.run(&["doctor", "--bogus"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("error: unknown flag: --bogus"),
        "message must name the flag:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no report on error");
}

/// Acceptance #9: an environment that cannot be inspected fails with the cause
/// named and no report on stdout. The failure is hard to provoke genuinely
/// (probe failures degrade to "absent" findings), so doctor honors a documented
/// test seam: `ARCHSPEC_DOCTOR_FAIL` simulates an uninspectable environment.
#[test]
fn doctor_inspection_failure_names_cause() {
    let bin = stub_bin_dir(&["rustc", "dotnet", "go"]);
    let fixture = Fixture::new();

    let output = Command::new(env!("CARGO_BIN_EXE_archspec"))
        .arg("doctor")
        .current_dir(&fixture.root)
        .env("PATH", bin.path())
        .env("ARCHSPEC_DOCTOR_FAIL", "1")
        .output()
        .expect("run archspec");

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("error: failed to inspect environment: cannot probe toolchain availability"),
        "message must name the cause:\n{err}"
    );
    assert!(
        stdout(&output).is_empty(),
        "no report on inspection failure"
    );
}

/// A failing probe (stub exits non-zero) is a finding (absent), not an error.
#[test]
fn doctor_failing_probe_reports_absent() {
    let bin = tempfile::TempDir::new().expect("temp bin dir");
    let stub = bin.path().join("rustc");
    fs::write(&stub, "#!/bin/sh\nexit 1\n").expect("write failing stub");
    let mut perms = fs::metadata(&stub).expect("stub metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&stub, perms).expect("make stub executable");
    let fixture = Fixture::new();

    let output = run_with_path(&fixture.root, bin.path(), &["doctor"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "failing probe must not fail doctor"
    );
    let out = stdout(&output);
    assert!(
        out.contains("rust\n  driver: present (in-binary, parse-only)\n  toolchain: absent"),
        "failing probe must report the rust TOOLCHAIN absent while the driver stays present:\n{out}"
    );
    assert!(
        !out.contains("toolchain: present"),
        "no toolchain may be reported present:\n{out}"
    );
}
