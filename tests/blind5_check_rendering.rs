//! The freshness check names the rendering it compared (blind5 follow-up
//! US_01).
//!
//! A stale `report --check` used to hint `regenerate with: archspec report`
//! no matter which rendering it compared; following that hint verbatim on a
//! markdown artefact rewrites the artefact as text — the trap two blind5 runs
//! named (their "CI footgun that punishes the honest reader of its error
//! message"). The contract now: the report family's stale finding names the
//! rendering it compared (`compared the text|markdown rendering`), the
//! regenerate hint carries that rendering's exact command form (`--format`
//! suffix when non-default), and the canonical-rendering advice clause is
//! stated exactly when the canonical rendering was compared. The ok-line and
//! every sibling command's check bytes are unchanged — report renders one
//! artefact in several renderings; scan, diagram, inspect and depgraph render
//! one each and their hints were already exact form. Rule: D01 of
//! `rust-arch-test-kit/worklog/workplan_archspec_blind5_followup/sources.md`.

mod common;

use common::{stderr, stdout, Fixture};

/// One crate whose `billing` module imports `auth`: a model with a module
/// tree, valid against its spec (same tree shape the blind4 ok-line suite
/// checks with).
fn model_tree_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write(
        "src/auth/mod.rs",
        "pub mod tokens;\npub use tokens::Token;\n",
    );
    fixture.write("src/auth/tokens.rs", "pub struct Token;\n");
    fixture.write("src/billing/mod.rs", "use crate::auth::Token;\npub fn bill(_: Token) {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
    fixture
}

const CANONICAL_BRANCH: &str = "compared the text rendering; regenerate with: archspec report; pass the same --format to --check to verify a --format-generated artefact";
const ADVICE_CLAUSE: &str =
    "pass the same --format to --check to verify a --format-generated artefact";

/// Scenario "markdown checked bare names the text comparison": the configured
/// destination holds a markdown body (generated with `--format markdown
/// --output <path>`); a bare `report --check` compares the TEXT rendering, so
/// it must name that comparison, hint the canonical exact form, and state the
/// advice clause the reader needs in the moment the mistake happens. Exit
/// stays 1 — unchanged.
#[test]
fn markdown_checked_bare_names_the_text_comparison() {
    let fixture = model_tree_fixture();
    fixture.write("archspec.toml", "[output]\nreport = \"check-report.md\"\n");
    let generate = fixture.run(&["report", "--format", "markdown", "--output", "check-report.md"]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr(&generate));

    let check = fixture.run(&["report", "--check"]);
    assert_eq!(check.status.code(), Some(1), "{}", stderr(&check));
    let err = stderr(&check);
    assert!(err.contains("differs from generated output"), "{err}");
    assert!(err.contains(CANONICAL_BRANCH), "the error must name the comparison it made and carry the advice in full:\n{err}");
    assert!(err.contains("regenerate with: archspec report"), "{err}");
}

/// Scenario "format-named check on fresh markdown speaks ok": a format-named
/// check of a byte-fresh markdown artefact is already true behaviour — pinned
/// so it cannot regress while the error bytes move.
#[test]
fn format_named_check_on_fresh_markdown_speaks_ok() {
    let fixture = model_tree_fixture();
    let generate = fixture.run(&["report", "--format", "markdown", "--output", "m.md"]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr(&generate));

    let check = fixture.run(&["report", "--check", "--output", "m.md", "--format", "markdown"]);
    assert_eq!(check.status.code(), Some(0), "{}", stderr(&check));
    assert_eq!(
        stdout(&check),
        "ok: m.md up to date\n",
        "the ok-line family extended to a format-named check"
    );
}

/// Scenario "format-named check carries the format": the stale finding names
/// the markdown rendering and its hint is the exact form that regenerates the
/// compared body byte-exactly; the default-rendering advice clause is absent
/// — the exact-form hint already is the advice.
#[test]
fn format_named_check_carries_the_format() {
    let fixture = model_tree_fixture();
    let generate = fixture.run(&["report", "--format", "markdown", "--output", "m.md"]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr(&generate));
    fixture.write("m.md", &format!("{}\ntampered\n", fixture.read("m.md")));

    let check = fixture.run(&["report", "--check", "--output", "m.md", "--format", "markdown"]);
    assert_eq!(check.status.code(), Some(1), "{}", stderr(&check));
    let err = stderr(&check);
    assert!(
        err.contains(
            "compared the markdown rendering; regenerate with: archspec report --format markdown)"
        ),
        "the hint must carry the compared rendering's exact form:\n{err}"
    );
    assert!(
        !err.contains(ADVICE_CLAUSE),
        "the default-rendering advice must be absent on a format-named check:\n{err}"
    );
}

/// Naming applies to the report family only: the other checking commands
/// emit one rendering each and their hints are already exact form — their
/// stale bytes stay byte-unchanged (blind4 red-check stability).
#[test]
fn sibling_command_check_bytes_are_unchanged() {
    let fixture = model_tree_fixture();
    let generate = fixture.run(&["scan", "--output", "model.json"]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr(&generate));
    fixture.write("model.json", "STALE ON PURPOSE\n");

    let check = fixture.run(&["scan", "--check", "--output", "model.json"]);
    assert_eq!(check.status.code(), Some(1), "{}", stderr(&check));
    assert_eq!(
        stderr(&check),
        "error: out of date model: model.json differs from generated output (regenerate with: archspec scan)\n",
        "the scan family's stale finding must not move a byte"
    );
}

/// The guidance sentence (D02) is stated at exactly two surfaces — the
/// `report --help` `--check` line (owner) and the `[output]` section of
/// `docs/archspec/config.md` (the config guidance surface both blind5 agents
/// actually read; there is no `help config` topic) — byte-identically, here
/// pinned through one constant.
#[test]
fn guidance_sentence_is_stated_at_both_surfaces_byte_identically() {
    const GUIDANCE: &str = "A `--check` compares the canonical rendering unless `--format` names another; to verify a `--format`-generated artefact, pass the same `--format` to `--check`.";

    let fixture = Fixture::new();
    let help = fixture.run(&["report", "--help"]);
    assert_eq!(help.status.code(), Some(0), "{}", stderr(&help));
    let out = stdout(&help);
    assert!(
        out.contains(GUIDANCE),
        "`report --help` must carry the guidance sentence on its `--check` line:\n{out}"
    );

    let manifest = env!("CARGO_MANIFEST_DIR");
    let doc = std::fs::read_to_string(format!("{manifest}/docs/archspec/config.md"))
        .expect("read config.md");
    assert!(
        doc.contains(GUIDANCE),
        "config.md must carry the guidance sentence verbatim"
    );
}
