// Shared runner helpers for the e2e matrix harness (tests/e2e_matrix.rs).
//
// Fail-closed binary resolution (law): the `ARCHSPEC` env var wins when set;
// otherwise the subject-under-test built for this run (`CARGO_BIN_EXE_archspec`)
// is used. An explicitly empty `ARCHSPEC` fails by name — never a silent skip.
//
// Generated, never hand-edited surfaces (e2e-matrix.json) are rendered by the
// harness and freshness-checked against the committed copy, mirroring the
// `feature_matrix.rs` write-and-compare law.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Output};

/// Resolve the archspec binary: ENV override -> build product -> fail by name.
pub fn resolve_binary() -> PathBuf {
    match std::env::var("ARCHSPEC") {
        Ok(v) if !v.trim().is_empty() => PathBuf::from(v),
        Ok(_) => panic!("ARCHSPEC is set but empty — fail closed, name the binary or unset it"),
        Err(_) => PathBuf::from(env!("CARGO_BIN_EXE_archspec")),
    }
}

/// Run the binary in `dir` with `args`; returns the raw std::process::Output.
pub fn run_lossy(dir: &std::path::Path, args: &[&str]) -> Output {
    Command::new(resolve_binary())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("archspec binary must be resolvable and runnable")
}

/// (exit code, lossy stdout, lossy stderr) for `run_lossy`.
pub fn run3(dir: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let out = run_lossy(dir, args);
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Assert the run exited with `code` and its stdout contains `needle`.
pub fn expect_exit_stdout(dir: &std::path::Path, args: &[&str], code: i32, needle: &str) {
    let (c, out, err) = run3(dir, args);
    assert_eq!(c, code, "exit class for {args:?}\nstdout:\n{out}\nstderr:\n{err}");
    if !needle.is_empty() {
        assert!(out.contains(needle), "stdout must contain {needle:?} for {args:?}\ngot:\n{out}");
    }
}

/// Assert the run exited with 64 (usage) and stderr names the usage problem.
pub fn expect_usage(dir: &std::path::Path, args: &[&str], needle: &str) {
    let (c, out, err) = run3(dir, args);
    assert_eq!(c, 64, "usage class for {args:?}\nstdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains(needle), "stderr must contain {needle:?} for {args:?}\ngot:\n{err}");
}

/// Assert the run exited with 70 (missing precondition) naming `needle`.
pub fn expect_precondition(dir: &std::path::Path, args: &[&str], needle: &str) {
    let (c, out, err) = run3(dir, args);
    assert_eq!(c, 70, "precondition class for {args:?}\nstdout:\n{out}\nstderr:\n{err}");
    let both = format!("{out}{err}");
    assert!(both.contains(needle), "output must name {needle:?} for {args:?}\ngot:\n{both}");
}

/// Content fingerprint of a file: FNV-1a 64 — a freshness weld, not a
/// cryptographic digest claim. Pure Rust so the writer (golden mode) and the
/// checker (plain run) can never disagree across hosts.
pub fn file_fingerprint(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64-{h:016x}")
}

/// FNV-1a 64 over bytes (same weld as `file_fingerprint`).
pub fn bytes_fingerprint(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64-{h:016x}")
}
