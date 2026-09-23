//! The one-liner cluster (blind-4 follow-up US 04): nine facts the blind runs
//! needed lived in the wrong surface or no surface at all. Each scenario pins
//! the sentence at its single owner surface (parent D06/D07): the literal
//! `--output -` example on every render help, the literal artefact example on
//! `diagram --help`, the runnable positional form in `help roles`, the
//! `help roles` pointer on the api-usage note line, the csharp dual-spelling
//! sentence in the csharp section of `help languages`, and the depgraph
//! statements of `[output]` non-participation, the test-gated-edge exclusion,
//! the `mod` label, plus the init-vs-update split in the command summaries.
//! Docs and `skill` keep pointers only — a restatement anywhere fails here,
//! and the registry (US 06) extends the same rule with signatures.

mod common;

use common::{stderr, stdout, Fixture};
use std::fs;

/// Literal dash example form every render command's `--output` clause ships
/// (scenario: dash example is literal).
const DASH_EXAMPLE: &str = "`--output -` writes to stdout and creates no file — never a path positional";

/// Literal artefact-form example of `diagram --source scan` plus its meaning
/// (scenario: artefact example is literal).
const ARTEFACT_EXAMPLE: &str = "`archspec diagram --source scan docs/archspec/scan.json`";
const ARTEFACT_RULE: &str = "the argument is a scan-JSON artefact file written by `archspec scan`, not a project directory";

/// The api-usage note line: canonical sentence intact + pointer home
/// (scenario: role-less note points home).
const NOTE_SENTENCE: &str = "note: this view shows no roles by decision (a role is not a usage fact)";
const NOTE_LINE: &str =
    "note: this view shows no roles by decision (a role is not a usage fact) — explained in 'archspec help roles'";

/// The csharp dual-spelling sentence — migrated owner, not rewritten
/// (scenario: csharp naming sentence lives in help languages). Markdown bold
/// is the docs-era decoration the plain-text help surface drops; the
/// comparison normalizes `*` characters away.
const DUAL_SPELLING: &str = "Two names for one project. In a C# tree (and any seeded spec with both tiers), every project appears twice: its `.`-separated name is the build unit (named by `units` and unit edges), its `::`-separated name is the namespace identity (the key space of `module_edges` and `roles`); an `update`-seeded spec therefore carries both tiers and that is not duplication — ADR-018 assigns allowance ownership across them.";

/// The pointer docs and skill keep instead of the sentence itself.
const DUAL_SPELLING_POINTER: &str = "csharp section of `archspec help languages`";

/// depgraph owner sentences (scenarios: [output] non-participation,
/// test-gated edges, mod label).
const OUTPUT_NONPARTICIPATION: &str =
    "depgraph does not read archspec.toml [output] — an artefact path comes only from an explicit --output";
const TEST_GATED_EDGES: &str = "Test-gated/soft edges (test namespaces and their consumers) can appear in depgraph projections while being excluded from `depend_on` comparison";
const MOD_LABEL: &str = "The `mod` label is a unit's root-module bucket: the namespace-less root where the composition/wiring code lives";

/// The disambiguated summary lines (scenario: summary splits init from update).
const INIT_SUMMARY: &str = "scaffold config + starter spec (no real-tree capture)";
const UPDATE_SUMMARY: &str = "seed a spec by snapshotting the real tree";

fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whitespace-collapsed and `*`-stripped: markdown emphasis is decoration,
/// never a wording difference (the dual-spelling comparison must not hinge
/// on it).
fn flatten_plain(text: &str) -> String {
    flatten(&text.replace('*', ""))
}

/// stdout of `archspec <args>`, asserting the house contract (exit 0, empty
/// stderr) like the other help-surface guards.
fn help_surface(args: &[&str]) -> String {
    let fixture = Fixture::new();
    let output = fixture.run(args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`archspec {}` must exit 0: {}",
        args.join(" "),
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "help must not write stderr");
    stdout(&output)
}

/// Every text surface an agent can read from the binary (same list as
/// `tests/doc_surface_nits.rs`).
fn rendered_surfaces() -> Vec<(String, String)> {
    let mut surfaces = vec![("help".to_string(), help_surface(&["help"]))];
    for topic in ["commands", "glob", "spec", "constraints", "languages", "roles", "workflow", "diagnostics"] {
        surfaces.push((format!("help {topic}"), help_surface(&["help", topic])));
    }
    for command in ["init", "scan", "diagram", "verify", "update", "report", "doctor", "inspect", "depgraph", "skill"] {
        surfaces.push((format!("help {command}"), help_surface(&["help", command])));
        surfaces.push((
            format!("{command} --help"),
            help_surface(&[command, "--help"]),
        ));
    }
    surfaces.push(("skill".to_string(), help_surface(&["skill"])));
    surfaces
}

/// Every markdown surface shipped in the payload.
fn payload_docs() -> Vec<(String, String)> {
    let mut files = vec![concat!(env!("CARGO_MANIFEST_DIR"), "/README.md").to_string()];
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/docs");
    let mut stack = vec![root.to_string()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read docs dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path.to_string_lossy().into_owned());
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                files.push(path.to_string_lossy().into_owned());
            }
        }
    }
    files
        .into_iter()
        .map(|path| {
            (
                path.replace(env!("CARGO_MANIFEST_DIR"), ""),
                fs::read_to_string(&path).expect("read payload doc"),
            )
        })
        .collect()
}

/// A single-crate rust tree whose module tier lets `depgraph api-usage`
/// render (app lib with `billing` importing `auth`).
fn rust_module_tree() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");
    fixture
}

/// Scenario: dash example is literal.
#[test]
fn dash_example_is_literal() {
    for command in [
        vec!["scan"],
        vec!["report"],
        vec!["diagram"],
        vec!["inspect"],
        vec!["depgraph", "modules"],
    ] {
        let mut help_flag = command.clone();
        help_flag.push("--help");
        let mut topic = vec!["help"];
        topic.extend(command.iter().take(1).copied());
        for (label, args) in [("--help", help_flag), ("help topic", topic)] {
            let text = help_surface(&args);
            assert!(
                text.contains(DASH_EXAMPLE),
                "the {label} surface of {command:?} must carry the literal example form beside its meaning:\n{text}"
            );
            // No rendering may invite reading the dash as a positional path:
            // the bare token form is gone, the flag-bound form is the only
            // dash the clause ships.
            assert!(
                !text.contains("`-` writes"),
                "{command:?} {label} must not restate the dash as a bare token:\n{text}"
            );
        }
    }
    // The embedded-everywhere surface carries it as well.
    let flat = flatten(&help_surface(&["help", "commands"]));
    for command in ["archspec scan", "archspec report", "archspec diagram", "archspec inspect", "archspec depgraph"] {
        assert!(
            flat.contains(&format!("usage: {command}")),
            "help commands must embed {command} help:\n{flat}"
        );
    }
    assert!(
        !help_surface(&["help", "commands"]).contains("`-` writes"),
        "help commands must carry no bare-token dash form"
    );
}

/// Scenario: artefact example is literal.
#[test]
fn artefact_example_is_literal() {
    for args in [vec!["diagram", "--help"], vec!["help", "diagram"]] {
        let flat = flatten(&help_surface(&args));
        assert!(
            flat.contains(ARTEFACT_EXAMPLE),
            "`archspec {}` must show the artefact form as a literal example:\n{flat}",
            args.join(" ")
        );
        assert!(
            flat.contains(ARTEFACT_RULE),
            "`archspec {}` must state that with --source scan the argument is a scan-JSON artefact file, not a project directory:\n{flat}",
            args.join(" ")
        );
    }
}

/// Scenario: roles shows positional modes.
#[test]
fn roles_shows_positional_modes() {
    let text = help_surface(&["help", "roles"]);
    assert!(
        text.contains("`archspec inspect tree`"),
        "the roles topic must show the tree view in runnable positional form:\n{text}"
    );
    assert!(
        !text.contains("--tree"),
        "the roles topic must contain no form that reads as a flag:\n{text}"
    );
    assert!(
        text.contains("positional"),
        "the roles topic must say the mode word is positional:\n{text}"
    );
}

/// Scenario: role-less note points home.
#[test]
fn role_less_note_points_home() {
    let fixture = rust_module_tree();
    let output = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "api-usage must render: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert_eq!(
        out.matches(NOTE_SENTENCE).count(),
        1,
        "the existing sentence stays intact and appears exactly once:\n{out}"
    );
    assert!(
        out.lines().any(|line| line == NOTE_LINE),
        "the note line must carry the pointer suffix as its own line:\n{out}"
    );
    assert!(
        out.contains("help roles"),
        "the note must point at where the decision is explained:\n{out}"
    );
}

/// Scenario: csharp naming sentence lives in help languages.
#[test]
fn csharp_naming_sentence_lives_in_help_languages() {
    // The owner states it verbatim (modulo markdown emphasis) in its section.
    let out = help_surface(&["help", "languages"]);
    let csharp_block = flatten_plain(
        out.split("\ncsharp:")
            .nth(1)
            .and_then(|rest| rest.split("\ngo:").next())
            .expect("languages must have a csharp section"),
    );
    assert!(
        csharp_block.contains(DUAL_SPELLING),
        "the csharp section must state the dual-spelling rule verbatim:\n{csharp_block}"
    );

    // Single-owner legs: no other rendered surface restates the sentence.
    let owner_marker = "model tiers each scanner populates";
    for (name, text) in rendered_surfaces() {
        if text.contains(owner_marker) {
            continue; // embeds the languages block — the owner, rendered in place
        }
        assert!(
            !flatten_plain(&text).contains(DUAL_SPELLING),
            "{name} must not restate the csharp dual-spelling sentence:\n{text}"
        );
    }
    for (path, text) in payload_docs() {
        assert!(
            !flatten_plain(&text).contains(DUAL_SPELLING),
            "payload doc {path} must keep a pointer, not a restatement"
        );
    }

    // The skill output and the two migrated docs keep the pointer.
    let skill = help_surface(&["skill"]);
    assert!(
        skill.contains(DUAL_SPELLING_POINTER),
        "the skill output must point at the owner surface:\n{skill}"
    );
    for doc in ["/docs/archspec/skill.md", "/docs/archspec/spec.md"] {
        let text =
            fs::read_to_string(format!("{}{doc}", env!("CARGO_MANIFEST_DIR"))).expect("read doc");
        assert!(
            text.contains(DUAL_SPELLING_POINTER),
            "doc {doc} must keep an explicit pointer to the owner"
        );
    }
}

/// Scenario: depgraph states its [output] non-participation.
#[test]
fn depgraph_states_its_output_non_participation() {
    for args in [vec!["depgraph", "--help"], vec!["help", "depgraph"]] {
        let flat = flatten(&help_surface(&args));
        assert!(
            flat.contains(OUTPUT_NONPARTICIPATION),
            "`archspec {}` must state that depgraph reads no [output] destinations:\n{flat}",
            args.join(" ")
        );
    }
}

/// Scenario: test-gated edges explained once.
#[test]
fn test_gated_edges_explained_once() {
    for args in [vec!["depgraph", "--help"], vec!["help", "depgraph"]] {
        let flat = flatten(&help_surface(&args));
        assert!(
            flat.contains(TEST_GATED_EDGES),
            "`archspec {}` must explain the projection-vs-seed exclusion in one sentence:\n{flat}",
            args.join(" ")
        );
    }
    // One owner: no other surface (and no doc) carries a copy.
    for (name, text) in rendered_surfaces() {
        if text.contains("usage: archspec depgraph") {
            continue; // embeds the depgraph help — the owner, rendered in place
        }
        assert!(
            !flatten(&text).contains(TEST_GATED_EDGES),
            "{name} must not restate the test-gated-edge exclusion"
        );
    }
    for (path, text) in payload_docs() {
        assert!(
            !flatten(&text).contains(TEST_GATED_EDGES),
            "payload doc {path} must not restate the test-gated-edge sentence"
        );
    }
}

/// Scenario: mod label stated at the label surface.
#[test]
fn mod_label_stated_at_the_label_surface() {
    for args in [vec!["depgraph", "--help"], vec!["help", "depgraph"]] {
        let flat = flatten(&help_surface(&args));
        assert!(
            flat.contains(MOD_LABEL),
            "`archspec {}` renders the labels and must explain the `mod` label:\n{flat}",
            args.join(" ")
        );
    }
    // Single owner (registry enforces it from US 06; here the phrase pins it).
    for (name, text) in rendered_surfaces() {
        if text.contains("usage: archspec depgraph") {
            continue;
        }
        assert!(
            !flatten(&text).contains("root-module bucket"),
            "{name} must not restate the mod-label sentence"
        );
    }
    for (path, text) in payload_docs() {
        assert!(
            !flatten(&text).contains("root-module bucket"),
            "payload doc {path} must not restate the mod-label sentence"
        );
    }
}

/// Scenario: summary splits init from update.
#[test]
fn summary_splits_init_from_update() {
    for args in [
        vec!["--help"],
        vec!["help"],
        vec!["help", "commands"],
    ] {
        let flat = flatten(&help_surface(&args));
        assert!(
            flat.contains(INIT_SUMMARY),
            "`archspec {}` summary must say init scaffolds without a real-tree capture:\n{flat}",
            args.join(" ")
        );
        assert!(
            flat.contains(UPDATE_SUMMARY),
            "`archspec {}` summary must say update seeds from the real tree:\n{flat}",
            args.join(" ")
        );
    }
}
