//! Round-9 PARALLEL FACT LEG determinism law (decision-check round 9,
//! F13/F14): the per-file fact leg may run across threads, but the fold
//! replays in walk order and returns on FIRST Err, so every scan's
//! stdout, stderr and exit code stay byte-identical to the serial loop.
//! The `ARCHSPEC_THREADS` env is the hidden A/B lever (serial vs
//! parallel) for exactly these assertions — not public config.
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

mod common;
use common::Fixture;

fn run_with(root: &Path, threads: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_archspec"));
    command.current_dir(root).arg("scan");
    match threads {
        Some(value) => {
            command.env("ARCHSPEC_THREADS", value);
        }
        None => {
            command.env_remove("ARCHSPEC_THREADS");
        }
    }
    command.output().expect("run archspec")
}

fn verdict(output: &Output) -> (Option<i32>, String, String) {
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A ts tree of `count` files (above the 128-file parallel floor) with
/// a relative-import chain — edges for the tripwire, no findings.
fn ts_tree(fixture: &Fixture, count: usize) {
    fixture.write("package.json", "{\n  \"name\": \"parallel-leg\"\n}\n");
    for index in 0..count {
        let body = if index + 1 < count {
            format!(
                "import {{ leaf }} from \"./f{:03}\";\nexport const leaf = leaf;\n",
                index + 1
            )
        } else {
            "export const leaf = 1;\n".to_string()
        };
        fixture.write(&format!("src/f{index:03}.ts"), &body);
    }
}

#[test]
fn ts_fact_leg_bytes_match_serial_and_parallel() {
    let fixture = Fixture::new();
    ts_tree(&fixture, 140);

    let serial = verdict(&run_with(&fixture.root, Some("1")));
    assert_eq!(serial.0, Some(0), "serial leg must scan green");
    let parallel = verdict(&run_with(&fixture.root, Some("8")));
    assert_eq!(serial, parallel, "parallel leg must not move a byte");
    let again = verdict(&run_with(&fixture.root, Some("8")));
    assert_eq!(serial, again, "parallel leg must be stable across runs");
    let default = verdict(&run_with(&fixture.root, None));
    assert_eq!(serial, default, "default mode must match the serial leg");
}

#[test]
fn ts_error_at_file_k_hides_later_files_in_both_modes() {
    let fixture = Fixture::new();
    ts_tree(&fixture, 140);
    // Sorted walk order puts f070 before f139: the undecodable read at
    // k must abort before f139's unresolvable import can reach findings
    // (the `?` reproduction, F14).
    fs::write(fixture.path("src/f070.ts"), [0xff_u8, 0xfe, 0x00, 0x41])
        .expect("write undecodable ts file");
    fs::write(
        fixture.path("src/f139.ts"),
        "import { nope } from \"./missing\";\nexport const leaf = nope;\n",
    )
    .expect("write unresolvable ts file");

    let serial = verdict(&run_with(&fixture.root, Some("1")));
    let parallel = verdict(&run_with(&fixture.root, Some("8")));
    assert_eq!(serial, parallel, "first-Err fold must match the `?`");
    assert_ne!(serial.0, Some(0), "the undecodable file reddens the run");
    let stderr = &serial.2;
    assert!(stderr.contains("f070.ts"), "stderr names f070.ts: {stderr}");
    assert!(
        !stderr.contains("unresolvable import"),
        "a later file's findings must never exist when an earlier file errored: {stderr}"
    );
    assert!(serial.1.is_empty(), "no partial model on error");
}

/// A python tree of `count` modules (above the floor) with intra-package
/// imports — edges for the tripwire, no findings.
fn python_tree(fixture: &Fixture, count: usize) {
    fixture.write(
        "pyproject.toml",
        "[project]\nname = \"parallel-leg\"\nversion = \"0.1.0\"\n",
    );
    fixture.write("app/__init__.py", "");
    for index in 0..count {
        let body = if index + 1 < count {
            format!(
                "from app.mod{:03} import leaf\n\nleaf = leaf\n",
                index + 1
            )
        } else {
            "leaf = 1\n".to_string()
        };
        fixture.write(&format!("app/mod{index:03}.py"), &body);
    }
}

#[test]
fn python_fact_leg_bytes_match_serial_and_parallel() {
    let fixture = Fixture::new();
    python_tree(&fixture, 140);

    let serial = verdict(&run_with(&fixture.root, Some("1")));
    assert_eq!(serial.0, Some(0), "serial leg must scan green");
    let parallel = verdict(&run_with(&fixture.root, Some("8")));
    assert_eq!(serial, parallel, "parallel leg must not move a byte");
    let again = verdict(&run_with(&fixture.root, Some("8")));
    assert_eq!(serial, again, "parallel leg must be stable across runs");
    let default = verdict(&run_with(&fixture.root, None));
    assert_eq!(serial, default, "default mode must match the serial leg");
}

#[test]
fn python_error_at_file_k_hides_later_files_in_both_modes() {
    let fixture = Fixture::new();
    python_tree(&fixture, 140);
    fs::write(fixture.path("app/mod070.py"), [0xff_u8, 0xfe, 0x00, 0x41])
        .expect("write undecodable py file");

    let serial = verdict(&run_with(&fixture.root, Some("1")));
    let parallel = verdict(&run_with(&fixture.root, Some("8")));
    assert_eq!(serial, parallel, "first-Err fold must match the `?`");
    assert_ne!(serial.0, Some(0), "the undecodable file reddens the run");
    assert!(
        serial.2.contains("mod070.py"),
        "stderr names mod070.py: {}",
        serial.2
    );
    assert!(serial.1.is_empty(), "no partial model on error");
}
