//! The module `allowed` FORM law (GitHub issue #58).
//!
//! The `allowed` table has two renderings: the inline form
//! `allowed = { depend_on = [...] }` — what `update` emits and what the
//! reference teaches as canonical — and the `[module.allowed]` header
//! section. Each parses alone and adjudicates identically. Mixed on ONE
//! module they are malformed TOML (the parser refuses the second
//! declaration) or a swallowed inline key (TOML context puts a late inline
//! line INSIDE the open section); before this law both faces surfaced as a
//! context-bleeding error whose wording varied with the landing order. The
//! loaders now name the collision — `module #N declares "allowed" twice
//! (inline key + [module.allowed] section) — keep one` — with a
//! deterministic entry ordinal, while every unrelated unknown-key and
//! parse-error class keeps its shipped bytes (the issue-#8 strictness law
//! is unchanged).

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

const NAMED: &str = "declares \"allowed\" twice (inline key + [module.allowed] section) — keep one";

fn assert_named(err: &str, location: &str) {
    assert!(
        err.contains("invalid spec:") && err.contains(&format!("{location} {NAMED}")),
        "must name the file class, the entry {location:?} and the collision:\n{err}"
    );
}

/// Cell 1 (regression): the `[module.allowed]` header rendering parses and
/// verifies green on its own.
#[test]
fn header_form_only_stays_accepted() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[module.allowed]\ndepend_on = []\n",
    );
    let output = tree.run(&["verify", "."]);
    assert!(
        output.status.success(),
        "the header rendering must stay accepted:\n{}",
        stderr(&output)
    );
}

/// Cell 2 (regression): the inline rendering `update` emits parses and
/// verifies green on its own, and adjudicates identically to the header
/// rendering on one module (the same declared deny-all table).
#[test]
fn inline_form_only_stays_accepted() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\nallowed = { depend_on = [] }\n",
    );
    let output = tree.run(&["verify", "."]);
    assert!(
        output.status.success(),
        "the inline rendering must stay accepted:\n{}",
        stderr(&output)
    );

    let header = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[module.allowed]\ndepend_on = []\n",
    );
    assert_eq!(
        stdout(&output),
        stdout(&header.run(&["verify", "."])),
        "one module, either rendering: the verdict bytes must be identical"
    );
}

/// Cell 3: both renderings on ONE module are the named duplicate
/// declaration — both landing orders, the second module's ordinal, and the
/// submodule face — and the refusal is deterministic across runs and stable
/// across verbs (every loader shares the walker arm).
#[test]
fn both_forms_on_one_module_name_the_collision() {
    let orders = [
        // Inline first, header section second: the TOML duplicate-declaration
        // face (the file does not parse at all).
        (
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\nallowed = { depend_on = [] }\n\n[module.allowed]\ndepend_on = []\n",
            "module #1",
        ),
        // Header section first, inline key second: the parseable-nesting face
        // (the inline line lands inside the open section it re-declares).
        (
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[module.allowed]\ndepend_on = []\nallowed = { depend_on = [] }\n",
            "module #1",
        ),
        // Ordinal law: the collision names its OWN module, not the first.
        (
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\nallowed = { depend_on = [] }\n\n[module.allowed]\ndepend_on = []\n",
            "module #2",
        ),
        // Submodule face: module AND submodule ordinals.
        (
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[[module.submodules]]\nname = \"a::b\"\nmatches = { modules = [\"a::b\"] }\nallowed = { depend_on = [] }\n\n[module.submodules.allowed]\ndepend_on = []\n",
            "module #1 submodule #1",
        ),
    ];
    for (spec, location) in orders {
        let tree = rust_tree(spec);
        let output = tree.run(&["verify", "."]);
        assert_ne!(
            output.status.code(),
            Some(0),
            "mixing both forms on one module must refuse:\n{}",
            stdout(&output)
        );
        let first = stderr(&output);
        assert_named(&first, location);

        // Deterministic: a re-run speaks byte-identical bytes.
        let again = stderr(&tree.run(&["verify", "."]));
        assert_eq!(first, again, "the collision refusal must be deterministic");

        // Every verb that reads the spec names the same collision (shared
        // walker, the `every_verb…` law of the issue-8 suite).
        for args in [&["diagram", "."][..], &["report", "."][..], &["scan", "."][..]] {
            let run = tree.run(args);
            assert_ne!(
                run.status.code(),
                Some(0),
                "{args:?} must refuse the mixed-form spec:\n{}",
                stdout(&run)
            );
            assert_named(&stderr(&run), location);
        }
    }
}

/// Cell 5 (no collateral change): the strictness law keeps its shipped
/// bytes. A stray `matches` key inside a `[module.allowed]` section — a
/// context bleed WITHOUT the duplicate `allowed` key — stays the unknown-key
/// class, and a duplicate-key parse failure in another table stays the
/// loader's raw parse-error class.
#[test]
fn unrelated_errors_keep_their_existing_shape() {
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[module.allowed]\ndepend_on = []\nmatches = { units = [\"x\"] }\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("unknown key(s) \"matches\" in [module] #1 allowed")
            && !err.contains(NAMED),
        "a stray key without the collision must stay the unknown-key class:\n{err}"
    );

    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\nname = \"x\"\nname = \"y\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("duplicate key `name` in table `project`") && !err.contains(NAMED),
        "an unrelated duplicate key must stay the raw parse-error class:\n{err}"
    );
}

