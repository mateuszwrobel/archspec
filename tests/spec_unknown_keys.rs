//! Unknown-key refusal (GitHub issue 8 phase 1, decision-check BLOCKER-3).
//!
//! Before this law the spec model silently ignored every key it did not
//! define (no `deny_unknown_fields` anywhere), so a future semantic could be
//! typo'd into a no-op. The loaders now refuse an undefined key as the
//! shipped `invalid spec` class, naming the key and where it sits —
//! `validate_unknown_keys` in `src/archspec/spec.rs`, boundary
//! `SPEC_KEY_SHAPES`: the top level, `[project]` and its
//! `excluded_sources` entries, stereotype entries and their `match` tables,
//! module claims (submodules recursively) and their `matches`/`contract`/
//! `allowed` tables, and constraint entries. The `{ target, type_only }`
//! entry tables under `allowed` are refused by `parse_allowed_entry` (the
//! pre-existing entry-level class, pinned below too). Malformed TOML stays
//! with each loader's own parse-error class.
//!
//! Proving the VALUE-DOMAIN LAW of the design record (phase 2 extension of
//! the phase-1 sequencing law): `mode` on an `excluded_sources` entry is a
//! KNOWN key whose accepted values are exactly `abort` and `warn` — any
//! other value (the old `report-only` draft included) is invalid spec, so a
//! dial value can never be half-read by the validator.

mod common;

use common::{Fixture, stderr, stdout};

fn rust_tree(spec: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write("src/lib.rs", "pub fn f() {}\n");
    tree.write("architecture.spec.toml", spec);
    tree
}

fn assert_refuses(err: &str, key: &str, location: &str) {
    assert!(
        err.contains("invalid spec:")
            && err.contains(&format!("unknown key(s) \"{key}\""))
            && err.contains(location),
        "must name the file class, the key {key:?} and the location {location:?}:\n{err}"
    );
}

/// The value-domain law: `mode` is a known key (phase 2 landed the dial),
/// but a value outside {abort, warn} is refused fail-closed in the same
/// invalid-spec class, naming the entry, the value and the supported set —
/// the `report-only` draft of the design record can never ride the dial.
#[test]
fn mode_value_outside_the_domain_is_refused_at_excluded_sources_entries() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"web/vendor\"\nreason = \"vendored dependencies\"\nmode = \"report-only\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
    );
    let output = tree.run(&["verify", "."]);

    assert_ne!(output.status.code(), Some(0), "verify must refuse:\n{}", stdout(&output));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("excluded_sources entry #1")
            && err.contains("mode value \"report-only\" is not supported")
            && err.contains("\"abort\"")
            && err.contains("\"warn\""),
        "must name the invalid-spec class, the entry, the value and the \
         supported values:\n{err}"
    );
}

/// The typo class the BLOCKER names: `excluded_sourcez = [...]` in [project]
/// loaded with exit 0 before this law; now the key is named, not ignored.
#[test]
fn typo_excluded_sources_key_in_project_is_refused() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\nexcluded_sourcez = [ { path = \"web/vendor\", reason = \"vendored\" } ]\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
    );
    let output = tree.run(&["verify", "."]);

    assert_ne!(output.status.code(), Some(0));
    assert_refuses(&stderr(&output), "excluded_sourcez", "in [project]");
}

/// A stale label key on a constraint entry — the class the monorepo's own
/// specs carried — is refused naming the entry (inside the named boundary).
#[test]
fn unknown_key_on_a_constraint_entry_is_refused() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\nname = \"global guard\"\n",
    );
    let output = tree.run(&["verify", "."]);

    assert_ne!(output.status.code(), Some(0));
    assert_refuses(&stderr(&output), "name", "in [constraint] #1");
}

#[test]
fn unknown_top_level_table_is_refused() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[extras]\nnotes = \"stale\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
    );
    let output = tree.run(&["verify", "."]);

    assert_ne!(output.status.code(), Some(0));
    assert_refuses(&stderr(&output), "extras", "at the top level");
}

#[test]
fn unknown_keys_on_module_entries_and_tables_are_refused() {
    // One unknown per run: the refusal order is deterministic — a module
    // entry's own keys are checked before its subtables, then submodules.
    for (spec, key, location) in [
        (
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\nstale = true\n",
            "stale",
            "in [module] #1",
        ),
        (
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"], stale = [\"x\"] }\n",
            "stale",
            "in [module] #1 matches",
        ),
        (
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[module.allowed]\ndepend_on = [\"demo\"]\nforbidd = [\"x\"]\n",
            "forbidd",
            "in [module] #1 allowed",
        ),
        (
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n  [[module.submodules]]\n  name = \"demo::inner\"\n  matches = { modules = [\"demo::inner\"] }\n  stale = 1\n",
            "stale",
            "in [module] #1 submodule #1",
        ),
    ] {
        let tree = rust_tree(spec);
        let output = tree.run(&["verify", "."]);
        assert_ne!(output.status.code(), Some(0), "must refuse {location}:\n{}", stdout(&output));
        assert_refuses(&stderr(&output), key, location);
    }
}

#[test]
fn unknown_key_on_a_stereotype_match_table_is_refused() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[[stereotype]]\nname = \"domain\"\nmatch = { names = [\"demo\"], stale = [\"x\"] }\n",
    );
    let output = tree.run(&["verify", "."]);

    assert_ne!(output.status.code(), Some(0));
    assert_refuses(&stderr(&output), "stale", "in [stereotype] #1 match");
}

/// The entry-level class predates the table-level law and keeps its owner:
/// an unknown key inside a `{ target, type_only }` entry is refused by
/// `parse_allowed_entry`, from inside deserialization.
#[test]
fn unknown_key_in_an_allowed_entry_table_stays_refused() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[module.allowed]\ndepend_on = [ { target = \"demo\", mode = \"exempt\" } ]\n",
    );
    let output = tree.run(&["verify", "."]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("allowed.depend_on entry #1")
            && err.contains("\"mode\""),
        "the entry-level refusal must stay loud and name the key:\n{err}"
    );
}

/// The boundary is per-verb-independent: every verb that reads the spec
/// refuses the same file (`load` for diagram, `load_verify`, `load_report`,
/// `load_scan`), so no verb keeps silently accepting the same invalid spec
/// — the value-domain law reaches every loader through the shared walker.
#[test]
fn every_verb_that_reads_the_spec_refuses_an_invalid_spec() {
    for args in [
        &["verify", "."][..],
        &["scan", "."][..],
        &["report", "."][..],
        &["diagram", "."][..],
    ] {
        let tree = rust_tree(
            "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"web/vendor\"\nreason = \"vendored dependencies\"\nmode = \"report-only\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
        );
        let output = tree.run(args);
        assert_ne!(
            output.status.code(),
            Some(0),
            "{args:?} must refuse a stale-key spec:\n{}",
            stdout(&output)
        );
        let err = stderr(&output);
        assert!(
            err.contains("invalid spec:") && err.contains("mode value \"report-only\" is not supported"),
            "{args:?} must refuse the out-of-domain dial value in the same \
             invalid-spec class:\n{err}"
        );
    }
}

/// The accepted surface stays byte-identical: a valid minimal spec carrying
/// `excluded_sources` with both mandatory keys loads everywhere, and the
/// verdict bytes on stdout match the same tree without the key — the walker
/// changes nothing for specs that define only known keys.
#[test]
fn valid_spec_with_excluded_sources_keeps_verdict_bytes_identical() {
    let with = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"web/vendor\"\nreason = \"vendored dependencies\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
    );
    let without = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
    );

    let with_run = with.run(&["verify", "."]);
    let without_run = without.run(&["verify", "."]);
    assert!(with_run.status.success(), "valid spec must verify green: {}", stderr(&with_run));
    assert_eq!(
        stdout(&with_run),
        stdout(&without_run),
        "the verdict body must stay byte-identical — the entries ride the report surface, not the verdict"
    );
    assert!(
        stderr(&with_run)
            .lines()
            .any(|line| line == "excluded sources: web/vendor — vendored dependencies"),
        "the entry must print its report line:\n{}",
        stderr(&with_run)
    );
}

/// Boundary pinned explicitly: `names`/`paths` inside a module claim's
/// `matches` deserialize (the shared `MatchSet`) and are accepted, though
/// verify resolves module claims on `units`/`modules` only. They are model
/// keys, so the refusal law does not cover them — documented in
/// `docs/archspec/spec.md` and the emitted schema's descriptions.
#[test]
fn matches_names_and_paths_stay_accepted_on_module_claims() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"], names = [\"demo\"], paths = [\"demo\"] }\n",
    );
    let output = tree.run(&["verify", "."]);

    assert!(
        output.status.success(),
        "known keys of the shared match table must stay accepted:\n{}",
        stderr(&output)
    );
}
