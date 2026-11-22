//! H2 resolve-replay memo law (decision-check 2026-11-22, scenario
//! `worklog/scenarios/h2_resolve_replay/01`): the relative arm of the fold's
//! `resolve()` replays its `(importing dir, spelled specifier)` verdict
//! in-scan, at the call site, in walk order — semantically INVISIBLE. The
//! hidden `ARCHSPEC_RESOLVE_MEMO_CAP` env is the test-control lever (shape
//! follows `ARCHSPEC_THREADS` in `src/scan/leg.rs`): `0` is the replay-OFF
//! arm (the pre-memo path byte-for-byte), a small value trips the stop-store
//! cap mid-scan. NOT public config.
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

mod common;
use common::Fixture;

fn run_with_cap(root: &Path, cap: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_archspec"));
    command.current_dir(root).arg("scan");
    match cap {
        Some(value) => {
            command.env("ARCHSPEC_RESOLVE_MEMO_CAP", value);
        }
        None => {
            command.env_remove("ARCHSPEC_RESOLVE_MEMO_CAP");
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

/// A replay-dense ts tree: same-dir relative imports repeated across sibling
/// files, one unresolvable spelling imported twice (the MISS verdict rides the
/// memo too, with its findings emitted at the call site), and one bare import
/// repeated (the existing `bare` memo under the same stop-store rule).
fn replay_dense_tree(fixture: &Fixture) {
    fixture.write("package.json", "{\n  \"name\": \"h2-memo\"\n}\n");
    fixture.write("src/shared.ts", "export const shared = 1;\n");
    for index in 0..8 {
        fixture.write(
            &format!("src/a/f{index}.ts"),
            &format!(
                "import {{ shared }} from \"../shared\";\nimport path from \"path\";\n\
                 export const v = shared + {index} + path.sep.length;\n"
            ),
        );
    }
    fixture.write("src/b/helper.ts", "export const helper = 2;\n");
    for index in 0..6 {
        fixture.write(
            &format!("src/b/g{index}.ts"),
            &format!(
                "import {{ helper }} from \"./helper\";\nexport const w = helper * {index};\n"
            ),
        );
    }
    for index in 0..2 {
        fixture.write(
            &format!("src/c/miss{index}.ts"),
            &format!(
                "import {{ gone }} from \"./gone\";\nexport const z = gone + {index};\n"
            ),
        );
    }
}

/// Done-when 3, byte-identity cell (B2): with the default cap the memo
/// replays; with cap 0 every relative resolution recomputes exactly as the
/// pre-memo code did. The replay must move NO byte — stdout, stderr and the
/// exit code are equal across the arms, and across the mid-scan cap values.
#[test]
fn relative_replay_moves_no_byte() {
    let fixture = Fixture::new();
    replay_dense_tree(&fixture);

    let memo_off = verdict(&run_with_cap(&fixture.root, Some("0")));
    assert_eq!(memo_off.0, Some(1), "the ./gone miss must keep the census exit");
    let memo_on = verdict(&run_with_cap(&fixture.root, None));
    assert_eq!(memo_on, memo_off, "the replay memo must not move a byte");
    for cap in ["1", "2", "3"] {
        let capped = verdict(&run_with_cap(&fixture.root, Some(cap)));
        assert_eq!(capped, memo_off, "a mid-scan cap trip must not move a byte (cap={cap})");
    }
    let stable = verdict(&run_with_cap(&fixture.root, None));
    assert_eq!(stable, memo_on, "the memo must be stable across runs");
    // Sanity that the tree really carries the replay shapes the memo answers:
    // the compact model row serializes `"from":…,"to":…` adjacently, so an
    // edge pair is a byte-checkable fact, not a substring coincidence.
    assert!(
        memo_on.1.contains("\"from\":\"src/a/f0.ts\",\"to\":\"src/shared.ts\""),
        "the cross-dir replay edge must be in the model"
    );
    assert!(
        memo_on.1.contains("\"from\":\"src/b/g0.ts\",\"to\":\"src/b/helper.ts\""),
        "the same-dir replay edge must be in the model"
    );
}

/// Done-when 3, conflation-refusal cell (B1): one specifier spelled in two
/// dirs resolves to EACH dir's own target — a spec-only key would merge the
/// contexts into one wrong hit with a green exit. Same dir, same specifier
/// resolves identically for every importer.
#[test]
fn same_specifier_from_different_dirs_resolves_own_targets() {
    let fixture = Fixture::new();
    fixture.write("package.json", "{\n  \"name\": \"h2-conflation\"\n}\n");
    for dir in ["one", "two"] {
        fixture.write(&format!("{dir}/mod.ts"), "export const value = 1;\n");
        for index in 0..3 {
            fixture.write(
                &format!("{dir}/main{index}.ts"),
                &format!(
                    "import {{ value }} from \"./mod\";\nexport const v = value + {index};\n"
                ),
            );
        }
    }
    let run = verdict(&run_with_cap(&fixture.root, None));
    assert_eq!(run.0, Some(0), "every edge resolves: {run:?}");
    // The conflation refusal in bytes: EACH importer points at its own dir's
    // target. A spec-only key would point both dirs at one target — one of
    // these two rows would then be absent and the negative rows present.
    for dir in ["one", "two"] {
        for index in 0..3 {
            let edge = format!("\"from\":\"{dir}/main{index}.ts\",\"to\":\"{dir}/mod.ts\"");
            assert!(run.1.contains(&edge), "missing own-dir edge {edge}");
        }
        let crossed = format!("\"from\":\"one/main0.ts\",\"to\":\"{dir}/mod.ts\"");
        if dir == "two" {
            assert!(!run.1.contains(&crossed), "cross-dir conflation row {crossed}");
        }
    }
    let off = verdict(&run_with_cap(&fixture.root, Some("0")));
    assert_eq!(run, off, "the conflation refusal must hold byte-for-byte across the lever");
}

/// Done-when 3, replay-exactly-once / F14 cell (M2): a parse-failing file at
/// walk index k aborts the fold; a REPLAY DONOR at file k+2 (a key the memo
/// already holds) and an unresolvable file past the abort point contribute
/// NOTHING — their rows, findings and report lines never exist. Pre-
/// population would surface exactly such donor bytes; this cell refuses it.
#[test]
fn f14_abort_holds_back_the_replay_donor() {
    let fixture = Fixture::new();
    fixture.write("package.json", "{\n  \"name\": \"h2-f14\"\n}\n");
    // f000 first computes the (src, "./f001") key; f070 is the abort site;
    // f071 is the same-key DONOR and f090 names a missing target — both sit
    // past the abort point and must stay silent in every arm.
    fixture.write("src/f000.ts", "import { leaf } from \"./f001\";\nexport const leaf = leaf;\n");
    for index in 1..99 {
        fixture.write(
            &format!("src/f{index:03}.ts"),
            "export const leaf = 1;\n",
        );
    }
    fs::write(fixture.path("src/f070.ts"), [0xff_u8, 0xfe, 0x00, 0x41])
        .expect("write undecodable ts file");
    fixture.write(
        "src/f071.ts",
        "import { leaf } from \"./f001\";\nexport const donor = leaf;\n",
    );
    fixture.write(
        "src/f090.ts",
        "import { nope } from \"./missing\";\nexport const leaf = nope;\n",
    );

    let memo_off = verdict(&run_with_cap(&fixture.root, Some("0")));
    assert_ne!(memo_off.0, Some(0), "the undecodable file must abort the fold");
    let memo_on = verdict(&run_with_cap(&fixture.root, None));
    assert_eq!(memo_on, memo_off, "the abort must hide donor bytes in both arms");
    let text = format!("{}{}", memo_on.1, memo_on.2);
    assert!(!text.contains("f090"), "a file past the abort point must contribute no finding");
    assert!(!text.contains("missing"), "the unresolvable donor line must never exist (F14)");
    assert!(!memo_on.1.contains("donor"), "the replay donor must contribute no row");
}

/// Done-when 3, cap-inertness cell (M3, behavior arm): with cap `1` the
/// second distinct key onward stores nothing — every later resolution takes
/// the miss path exactly as the pre-memo code — yet the scan stays
/// byte-correct against the replay-OFF and default arms.
#[test]
fn stop_store_cap_inertness_moves_no_byte() {
    let fixture = Fixture::new();
    replay_dense_tree(&fixture);
    let off = verdict(&run_with_cap(&fixture.root, Some("0")));
    let capped = verdict(&run_with_cap(&fixture.root, Some("1")));
    assert_eq!(capped, off, "above the cap the memo must be inert, never wrong");
    let default = verdict(&run_with_cap(&fixture.root, None));
    assert_eq!(default, off, "the default cap must be inert too, in the other direction");
}
