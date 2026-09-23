//! blind4_followup US 05 — the fold-ownership pointer and the absence of a
//! grouping flag.
//!
//! (a) A reader who only ever runs `archspec depgraph --help` learns where
//! unit ownership lives: node labels fold units, and the ownership map is the
//! scan JSON `root_module_declarations` field that `archspec scan` emits
//! (parent D05: a pointer on the depgraph surface, not a second rendering of
//! a fact the scan JSON owns).
//!
//! (b) The parent rejected a `--show-units` grouping flag (D08): grouping is
//! fixed at the view level (`modules` folds units into labels), the flag set
//! stays exactly today's, so the rejection error and the unchanged surface
//! are pinned — the rejected flag cannot creep in later.

mod common;

use common::{stderr, stdout, Fixture};

/// The ownership-source key the pointer must name, and the folding verb that
/// ties it to the labels (scenario: fold pointer stated).
const FOLD_POINTER: &str = "root_module_declarations";

#[test]
fn depgraph_help_points_at_the_fold_ownership_source() {
    let fixture = Fixture::new();
    let output = fixture.run(&["depgraph", "--help"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains(FOLD_POINTER),
        "`depgraph --help` must name the ownership source, got:\n{out}"
    );
    assert!(
        out.contains("fold"),
        "`depgraph --help` must tie the pointer to the label folding, \
         got:\n{out}"
    );
    // The pointer also names the command that emits the map, so the
    // round-trip is one pointer, not one discovery cycle.
    assert!(
        out.contains("`archspec scan`"),
        "`depgraph --help` must name the emitting command, got:\n{out}"
    );
}

#[test]
fn depgraph_has_no_unit_grouping_flag() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");

    // Rejected exactly: unknown-flag error, exit 1, no diagram.
    let output = fixture.run(&["depgraph", "modules", "--show-units"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "error: unknown flag: --show-units\n",
        "the rejected grouping flag must fail with the unknown-flag error"
    );
    assert!(
        output.stdout.is_empty(),
        "a rejected flag must print no diagram, got: {}",
        stdout(&output)
    );

    // The accepted flag set is exactly today's: the rendering flags are still
    // advertised and no grouping flag joined them.
    let help = fixture.run(&["depgraph", "--help"]);
    assert_eq!(help.status.code(), Some(0), "{}", stderr(&help));
    let help = stdout(&help);
    assert!(
        !help.contains("--show-units"),
        "depgraph --help must not advertise a grouping flag, got:\n{help}"
    );
    for flag in ["--format", "--parent", "--output", "--check"] {
        assert!(
            help.contains(flag),
            "depgraph --help must still list `{flag}` (flag set unchanged), \
             got:\n{help}"
        );
    }

    // The documented grouping choice still works unchanged.
    let output = fixture.run(&["depgraph", "modules", "--output", "-"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}
