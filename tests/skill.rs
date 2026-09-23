//! The `archspec skill` command (workplan archspec_agent_skill).
//!
//! The skill is a deliverable class of its own: agent procedure shipped
//! in-binary, printed on stdout and installed into a project's skill-loading
//! path. What is asserted here, per the plan's Gherkin scenarios:
//! - `skill` prints the embedded document byte-for-byte and writes nothing;
//! - the emitted bytes equal the payload document `docs/archspec/skill.md`
//!   (drift guard — a change to either alone fails);
//! - `install` creates `.agent/skills/archspec.md` under the positional path
//!   (default cwd), reports state, and is idempotent;
//! - a locally modified install is user work: refused without `--force`
//!   (content kept), replaced with it;
//! - usage errors (unknown subcommand, extra positionals) exit 1 with the
//!   house texts;
//! - discovery: the top-level command list names `skill`, `skill --help`
//!   prints the usage contract, and the workflow manual topic points at the
//!   install command outside the numbered recipe sequence.

mod common;

use common::{run_in, stderr, stdout, Fixture};
use std::fs;

fn skill_document() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/archspec/skill.md");
    fs::read_to_string(path).expect("read payload skill document")
}

#[test]
fn print_emits_the_shipped_document_and_writes_nothing() {
    let fixture = Fixture::new();
    let output = fixture.run(&["skill"]);
    assert!(output.status.success(), "skill print must exit 0");
    assert_eq!(stdout(&output), skill_document());
    assert_eq!(stderr(&output), "");
    assert!(
        fs::read_dir(fixture.root.join(".agent")).is_err(),
        "printing must not create files"
    );
}

#[test]
fn install_writes_the_document_into_the_skill_loading_path() {
    let fixture = Fixture::new();
    let output = fixture.run(&["skill", "install"]);
    assert!(output.status.success(), "install failed: {}", stderr(&output));
    let installed = fixture.read(".agent/skills/archspec.md");
    assert_eq!(installed, skill_document());
    assert!(
        stdout(&output).contains("installed ./.agent/skills/archspec.md"),
        "install must report the path: {}",
        stdout(&output)
    );
}

#[test]
fn install_honours_the_positional_path_regardless_of_cwd() {
    let fixture = Fixture::new();
    fixture.write("sub/keep.txt", "");
    let output = fixture.run_in("sub", &["skill", "install", ".."]);
    assert!(output.status.success(), "install .. failed: {}", stderr(&output));
    assert!(fixture.path(".agent/skills/archspec.md").exists());
}

#[test]
fn install_is_idempotent_and_never_rewrites_identical_content() {
    let fixture = Fixture::new();
    assert!(fixture.run(&["skill", "install"]).status.success());
    let first = fs::metadata(fixture.path(".agent/skills/archspec.md"))
        .expect("metadata")
        .modified()
        .expect("mtime");
    let output = fixture.run(&["skill", "install"]);
    assert!(output.status.success(), "second install failed");
    let second = fs::metadata(fixture.path(".agent/skills/archspec.md"))
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(first, second, "an unchanged install must not rewrite the file");
    assert!(
        stdout(&output).starts_with("already installed: "),
        "state not reported: {}",
        stdout(&output)
    );
}

#[test]
fn local_edits_are_kept_and_force_is_required_to_replace_them() {
    let fixture = Fixture::new();
    assert!(fixture.run(&["skill", "install"]).status.success());
    fixture.write(".agent/skills/archspec.md", "---\nname: archspec\nlocal edit\n");

    let refused = fixture.run(&["skill", "install"]);
    assert_eq!(refused.status.code(), Some(1), "must refuse without --force");
    assert!(
        stderr(&refused).contains("--force to overwrite"),
        "the refusal must name --force: {}",
        stderr(&refused)
    );
    assert_eq!(
        fixture.read(".agent/skills/archspec.md"),
        "---\nname: archspec\nlocal edit\n",
        "the local edit must survive a refused install"
    );

    let forced = fixture.run(&["skill", "install", "--force"]);
    assert!(forced.status.success(), "force failed: {}", stderr(&forced));
    assert_eq!(fixture.read(".agent/skills/archspec.md"), skill_document());
    assert!(stdout(&forced).starts_with("overwrote "), "state not reported");
}

#[test]
fn bad_invocations_are_usage_errors() {
    let fixture = Fixture::new();
    let unknown = fixture.run(&["skill", "remove"]);
    assert_eq!(unknown.status.code(), Some(1));
    assert_eq!(
        stderr(&unknown),
        "error: unknown skill subcommand: remove (expected: print, install)\n"
    );

    let too_many = fixture.run(&["skill", "install", "a", "b"]);
    assert_eq!(too_many.status.code(), Some(1));
    assert_eq!(
        stderr(&too_many),
        "error: expected at most one path argument\n"
    );

    let print_path = fixture.run(&["skill", "print", "."]);
    assert_eq!(print_path.status.code(), Some(1));
    assert_eq!(
        stderr(&print_path),
        "error: skill print takes no path arguments\n"
    );

    let unknown_flag = fixture.run(&["skill", "install", "--yes"]);
    assert_eq!(unknown_flag.status.code(), Some(1));
    assert!(stderr(&unknown_flag).contains("unknown flag: --yes"));
}

#[test]
fn the_skill_is_self_discoverable() {
    let fixture = Fixture::new();

    let help = fixture.run(&["--help"]);
    assert!(help.status.success());
    assert!(
        stdout(&help).contains("skill"),
        "top-level command list must name skill:\n{}",
        stdout(&help)
    );

    let per_command = fixture.run(&["skill", "--help"]);
    assert!(per_command.status.success());
    assert!(stdout(&per_command).starts_with("usage: archspec skill"));

    let workflow = run_in(&fixture.root, &["help", "workflow"]);
    assert!(workflow.status.success());
    let text = stdout(&workflow);
    assert!(
        text.contains("archspec skill install"),
        "the workflow topic must point at the install command:\n{text}"
    );
    // the pointer lives in the supporting-commands area, outside the numbered
    // recipe sequence — the recipe numbering must stay untouched
    let numbered_zone = text
        .find("Supporting commands:")
        .expect("supporting commands section");
    let pointer_at = text.find("archspec skill install").expect("pointer");
    assert!(
        pointer_at > numbered_zone,
        "the skill pointer must not enter the numbered recipe sequence"
    );
}

#[test]
fn workflow_table_seed_row_names_target_and_review_step() {
    // Blind-run friction: the table said only "Seed a spec from the tree" —
    // where the seed lands and that it must be reviewed/trimmed lived only in
    // a stdout line. The row must carry both, plus the identity row pairs
    // skill.md's discoverability with `--version`.
    let doc = skill_document();
    assert!(
        doc.contains("`architecture.spec.toml` in the scanned directory"),
        "the update row must name the seed target:\n{doc}"
    );
    assert!(
        doc.contains("review and trim"),
        "the update row must require review-and-trim:\n{doc}"
    );
    assert!(
        doc.contains("mirrors the graph, not the intent"),
        "the update row must say why review is required:\n{doc}"
    );
    assert!(
        doc.contains("`archspec --version`"),
        "the workflow table must offer the identity command:\n{doc}"
    );
}
