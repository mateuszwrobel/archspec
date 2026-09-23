//! The help-surface relocations bundle (blind5 follow-up US_02).
//!
//! Four sentences agents needed were real but unreachable — the destination
//! rule scattered across an acceptance row and --help hints, the first-spec
//! bootstrap hidden under the numbered list, the fold-ownership pointer
//! parked in `depgraph --help`'s last paragraph, and the capability forms
//! missing from the COMMANDS summary. This suite states each fact at the
//! surface an agent reads first and pins the bytes that moved (or must not):
//! the recipe's numbered steps and Supporting-commands rows stay byte-
//! identical, the fold sentence keeps its bytes and only its position moves
//! earlier, and bare `capability` keeps its usage error and exit 1 — the
//! blind5-go "exit 0" report is disproven by probe, the summary line was the
//! real gap. Rule: D03 and D08 of
//! `rust-arch-test-kit/worklog/workplan_archspec_blind5_followup/sources.md`.

mod common;

use common::{stderr, stdout, Fixture};

/// The destination-canonical rule (D03), stated where the command's flags
/// are read.
const DESTINATION_CANONICAL: &str = "A configured report destination is provisioned for the default text rendering: a run naming another `--format` renders to stdout and never writes the destination.";

/// The first-spec bootstrap sentence: one sentence in the recipe's intro
/// area, naming the seed command and pointing at the Supporting-commands row
/// that owns the seed's reconciliation.
const BOOTSTRAP: &str = "Steps 1–2 create no spec: on a first capture of an untracked repo, seed one with `archspec update` before step 3 runs and reconcile the seed per step 4 — the `update` row under Supporting commands owns that reconciliation.";

/// The registered fold-pointer sentence (blind4 US 05) — its bytes are
/// welded here; only its position moves (the `fold-pointer` claim owns the
/// text, this guard pins the order).
const FOLD_POINTER_SENTENCE: &str = "Node labels fold units — unit ownership lives in the `root_module_declarations` map that `archspec scan` emits in its scan JSON.";

fn surface(fixture: &Fixture, args: &[&str]) -> String {
    let output = fixture.run(args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`archspec {}` must exit 0: {}",
        args.join(" "),
        stderr(&output)
    );
    stdout(&output)
}

/// Scenario "destination-canonical sentence stated": `report --help` states
/// the canonical rule once, beside the flags it governs — the behaviour the
/// blind5 runs found only in config.md's acceptance row 10 and scattered
/// --help hints.
#[test]
fn report_help_states_the_destination_canonical_rule() {
    let fixture = Fixture::new();
    for args in [vec!["report", "--help"], vec!["help", "report"]] {
        let out = surface(&fixture, &args);
        assert!(
            out.contains(DESTINATION_CANONICAL),
            "`archspec {}` must state the destination-canonical rule:\n{out}",
            args.join(" ")
        );
    }
}

/// Scenario "workflow bootstrap sentence, recipe untouched": the bootstrap
/// is one sentence in the intro area, and the numbered recipe (steps and
/// their `[audit ...]` lines) and the Supporting commands rows keep their
/// bytes — the seed's owning row was never rewritten into a seventh step.
#[test]
fn workflow_bootstrap_sentence_with_the_recipe_byte_untouched() {
    let fixture = Fixture::new();
    let out = surface(&fixture, &["help", "workflow"]);
    // The topic renderer wraps paragraphs, so compare whitespace-flattened
    // text — the sentence's bytes are what the guard pins, not the fold.
    let flat = out.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains(BOOTSTRAP),
        "`help workflow` must state the bootstrap before the steps:\n{out}"
    );
    // The bootstrap sits before step 3 — that is what the go run asked for.
    let bootstrap_at = flat.find(BOOTSTRAP).expect("bootstrap sentence");
    let step3_at = flat
        .find("3. archspec verify")
        .expect("step 3 of the recipe");
    assert!(
        bootstrap_at < step3_at,
        "the bootstrap sentence must precede step 3"
    );
    for step in [
        "1. archspec scan",
        "2. archspec depgraph modules",
        "3. archspec verify",
        "4. tighten allowed.depend_on",
        "5. archspec verify --strict",
        "6. archspec inspect",
    ] {
        assert!(
            out.contains(step),
            "recipe step {step:?} must keep its bytes:\n{out}"
        );
    }
    // The Supporting commands rows are byte-identical — the update row owns
    // the seed's reconciliation and was not rewritten into a step.
    let flat = out.split_whitespace().collect::<Vec<_>>().join(" ");
    for row in [
        "init scaffold a minimal base spec to start from.",
        "update snapshot the current model as a seed spec once the architecture is healthy.",
        "doctor report driver capability and toolchain availability (rust, csharp, go) as separate facts",
        "skill install the agent-facing audit skill:",
    ] {
        assert!(
            flat.contains(row),
            "the Supporting-commands row {:?} must keep its bytes:\n{flat}",
            row
        );
    }
}

/// Scenario "fold sentence stated earlier": the welded sentence now precedes
/// the `mod`-label paragraph — a reader who hits the folded-label cloud hits
/// the explanation before the footnotes — and its bytes are unchanged.
#[test]
fn fold_sentence_is_stated_before_the_mod_label_paragraph() {
    let fixture = Fixture::new();
    for args in [vec!["depgraph", "--help"], vec!["help", "depgraph"]] {
        let out = surface(&fixture, &args);
        assert!(
            out.contains(FOLD_POINTER_SENTENCE),
            "`archspec {}` must keep the fold-pointer sentence byte-identical:\n{out}",
            args.join(" ")
        );
        let fold_at = out.find(FOLD_POINTER_SENTENCE).expect("fold sentence");
        let mod_at = out
            .find("The `mod` label is a unit's root-module bucket")
            .expect("mod-label paragraph");
        assert!(
            fold_at < mod_at,
            "the fold sentence must be stated before the `mod`-label paragraph in `archspec {}`:\n{out}",
            args.join(" ")
        );
    }
}

/// Scenario "COMMANDS summary names the capability forms": the summary line
/// mirrors the usage error's grammar, and bare `capability` still prints
/// that usage error and exits 1 (probe baseline — the blind5-go exit-0
/// report is disproven; the exit-code contract is untouched).
#[test]
fn commands_summary_names_the_capability_forms() {
    let fixture = Fixture::new();
    let out = surface(&fixture, &["--help"]);
    let line = out
        .lines()
        .find(|line| line.trim_start().starts_with("capability"))
        .unwrap_or_else(|| panic!("COMMANDS summary must list capability:\n{out}"));
    assert!(
        line.contains("capability table"),
        "the summary line keeps its subject:\n{line}"
    );
    assert!(
        line.contains("matrix | granular <language> <fact>"),
        "the summary line must name the forms the sub-command takes:\n{line}"
    );

    let refused = fixture.run(&["capability"]);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "bare capability keeps its exit-1 usage error"
    );
    assert!(
        stderr(&refused)
            .contains("usage: archspec capability matrix | granular <language> <fact>"),
        "{}",
        stderr(&refused)
    );
}
