//! Self-dogfood verify, reachable from plain `cargo test` (self-dogfood
//! hardening, deliverable I): the runner script's `archspec verify --strict`
//! leg used to live only behind the git hook, so a bare `cargo test` never
//! checked this crate against its own `architecture.spec.toml`. This test
//! runs the same check over the real crate root — zero violations, zero
//! warnings (a vacuous finding is a warning in its own right, and `--strict`
//! fails on it) — and welds the checked-constraint count to the number of
//! `[[constraint]]` declarations in the spec file.
//!
//! Unlike the `feature_matrix` suite this test WRITES NOTHING: `verify` is a
//! read-only command, the crate root comes from `CARGO_MANIFEST_DIR`, and no
//! fixture is materialized anywhere.

use std::path::Path;
use std::process::Command;

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_crate_verifies_strict_clean_against_its_own_spec() {
    let root = crate_root();
    let output = Command::new(env!("CARGO_BIN_EXE_archspec"))
        .args(["verify", ".", "--strict"])
        .current_dir(root)
        .output()
        .expect("spawn archspec verify");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    assert!(
        output.status.success(),
        "strict self-verify must pass, exit {:?}\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}",
        output.status.code()
    );
    assert!(
        stdout.contains("matches source model"),
        "the clean run states the match:\n{stdout}"
    );
    assert!(
        !stdout.contains("vacuous constraint"),
        "every declared constraint engages on the real tree:\n{stdout}"
    );

    // The count in the ok line equals the declarations in the spec file —
    // no constraint silently dropped from the checked set.
    let spec = std::fs::read_to_string(root.join("architecture.spec.toml"))
        .expect("read architecture.spec.toml");
    let declared = spec
        .lines()
        .filter(|line| line.trim() == "[[constraint]]")
        .count();
    assert!(declared > 0, "the spec declares constraints");
    let checked: usize = stdout
        .split("constraints checked")
        .next()
        .map(str::trim)
        .and_then(|head| head.rsplit(['(', ' ']).next())
        .and_then(|number| number.parse().ok())
        .unwrap_or_else(|| panic!("no `N constraints checked` count in:\n{stdout}"));
    assert_eq!(
        checked, declared,
        "verify checked a different number of constraints than the spec declares"
    );
}
