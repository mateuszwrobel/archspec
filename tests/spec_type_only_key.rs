//! C6 registration of the entry-level `type_only` schema key (D3, R3-5):
//! an entry of `[module.allowed]` `depend_on`/`forbidden` may be a plain
//! string (default `count`) or a `{ target, type_only }` table with the
//! value domain `count | exempt`, validated at spec load with the shipped
//! invalid-spec class naming the entry and the supported values. The key is
//! LIVE: the dependency adjudication reads the entry's declaration (the
//! per-entry exemption law of D3, cell-by-cell in tests/type_only_exemption.rs
//! on ts trees). On rust models the `type_only` edge fact is absent, so every
//! crossing is a value crossing — an `exempt` entry governs it as the
//! value-import row of the scenario table states, and `count` stays
//! byte-identical to the plain-string shape.

mod common;

use common::{Fixture, stderr, stdout};

/// One unit `app`, modules `a` (the importer), `b` (the imported) and
/// `shell` (the declared-but-unused allowed target); `a` imports `b` and
/// the spec's `depend_on` entry for `a` names `shell`, so every shape of
/// the entry reddens the a→b crossing as disallowed — which is exactly
/// what the inertness cells compare.
fn tree_with_allowed(entry: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write("Cargo.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    tree.write("src/lib.rs", "pub mod a;\npub mod b;\npub mod shell;\n");
    tree.write("src/a.rs", "use crate::b::Target;\npub fn run() -> Target { Target }\n");
    tree.write("src/b.rs", "pub struct Target;\n");
    tree.write("src/shell.rs", "pub struct Shell;\n");
    tree.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = {{ modules = [\"app::a\"] }}\n\n[module.allowed]\ndepend_on = [{entry}]\n\n[[module]]\nname = \"b\"\nmatches = {{ modules = [\"app::b\"] }}\n\n[[module]]\nname = \"shell\"\nmatches = {{ modules = [\"app::shell\"] }}\n"
        ),
    );
    tree
}

fn verdict(tree: &Fixture) -> (Option<i32>, String) {
    let out = tree.run(&["verify", "."]);
    (out.status.code(), stdout(&out) + &stderr(&out))
}

#[test]
fn plain_string_entry_takes_the_count_default_and_verdicts() {
    // The shipped shape is unchanged: a plain string parses, the crossing
    // outside the allowed set reddens.
    let (code, out) = verdict(&tree_with_allowed("\"shell\""));
    assert_eq!(code, Some(1), "the disallowed crossing must redden:\n{out}");
    assert!(out.contains("disallowed cross-component dependency"), "out:\n{out}");
}

#[test]
fn table_entry_with_count_parses_and_verdicts_identically() {
    let (code_string, out_string) = verdict(&tree_with_allowed("\"shell\""));
    let (code_count, out_count) = verdict(&tree_with_allowed(
        "{ target = \"shell\", type_only = \"count\" }",
    ));
    assert_eq!(code_count, Some(1), "the table entry must parse:\n{out_count}");
    assert!(
        out_count.contains("disallowed cross-component dependency"),
        "the default law counts the entry's target:\n{out_count}"
    );
    assert_eq!(code_string, code_count);
    assert_eq!(
        out_string, out_count,
        "an explicit count entry verdicts byte-identically to the plain string"
    );
}

#[test]
fn an_exempt_entry_governs_only_the_pair_it_names() {
    // The exemption is per ENTRY: an entry naming an uncrossed pair changes
    // nothing for the crossed pair, so the verdict stays byte-identical to
    // the count default.
    let (code_count, out_count) =
        verdict(&tree_with_allowed("{ target = \"shell\", type_only = \"count\" }"));
    let (code_exempt, out_exempt) =
        verdict(&tree_with_allowed("{ target = \"shell\", type_only = \"exempt\" }"));
    assert_eq!(
        code_count, code_exempt,
        "an entry naming no crossed pair leaves the verdict's exit code"
    );
    assert_eq!(
        out_exempt, out_count,
        "an entry naming no crossed pair leaves the verdict's bytes"
    );
}

#[test]
fn an_exempt_entry_reddens_the_value_crossing_it_governs() {
    // On the rust model every crossing is a value crossing (the fact is
    // absent), so the exempt entry reddens the crossing it governs — the
    // value-import row of the scenario table — where the count default
    // keeps the allowed target green.
    let tree_count = Fixture::new();
    write_pair(&tree_count, "{ target = \"b\", type_only = \"count\" }");
    let (code_count, out_count) = verdict(&tree_count);
    assert_eq!(code_count, Some(0), "the count default counts the crossing:\n{out_count}");

    let tree_exempt = Fixture::new();
    write_pair(&tree_exempt, "{ target = \"b\", type_only = \"exempt\" }");
    let (code_exempt, out_exempt) = verdict(&tree_exempt);
    assert_eq!(code_exempt, Some(1), "the exempt entry reddens the value crossing:\n{out_exempt}");
    assert!(
        out_exempt.contains("disallowed cross-component dependency: a -> b"),
        "the finding names the value crossing:\n{out_exempt}"
    );
}

/// One unit `app` with modules `a` and `b`; `a` imports `b` and the spec's
/// `depend_on` entry for `a` names `b` with the given entry shape.
fn write_pair(tree: &Fixture, entry: &str) {
    tree.write("Cargo.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    tree.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    tree.write("src/a.rs", "use crate::b::Target;\npub fn run() -> Target { Target }\n");
    tree.write("src/b.rs", "pub struct Target;\n");
    tree.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = {{ modules = [\"app::a\"] }}\n\n[module.allowed]\ndepend_on = [{entry}]\n\n[[module]]\nname = \"b\"\nmatches = {{ modules = [\"app::b\"] }}\n"
        ),
    );
}

#[test]
fn unknown_value_is_an_invalid_spec_error_naming_the_entry_and_values() {
    let out = verdict(&tree_with_allowed("{ target = \"shell\", type_only = \"maybe\" }"));
    assert_eq!(out.0, Some(1), "the invalid-spec class is untagged — exit 1");
    assert!(out.1.contains("invalid spec:"), "stderr: {}", out.1);
    assert!(out.1.contains("#1"), "the message names the entry (1-indexed): {}", out.1);
    assert!(out.1.contains("type_only"), "the message names the key: {}", out.1);
    assert!(out.1.contains("count"), "the message names count: {}", out.1);
    assert!(out.1.contains("exempt"), "the message names exempt: {}", out.1);
}

#[test]
fn missing_target_is_an_invalid_spec_error_naming_the_entry_and_key() {
    let out = verdict(&tree_with_allowed("{ type_only = \"count\" }"));
    assert_eq!(out.0, Some(1));
    assert!(out.1.contains("invalid spec:"), "stderr: {}", out.1);
    assert!(out.1.contains("#1"), "the message names the entry: {}", out.1);
    assert!(out.1.contains("target"), "the message names the mandatory key: {}", out.1);
}

#[test]
fn forbidden_entries_take_the_same_entry_shapes() {
    // The key registers on `forbidden` entries with the same law.
    let tree = Fixture::new();
    tree.write("Cargo.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    tree.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    tree.write("src/a.rs", "use crate::b::Target;\npub fn run() -> Target { Target }\n");
    tree.write("src/b.rs", "pub struct Target;\n");
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { modules = [\"app::a\"] }\n\n[module.allowed]\nforbidden = [{ target = \"b\", type_only = \"count\" }]\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"app::b\"] }\n",
    );
    let (code, out) = verdict(&tree);
    assert_eq!(code, Some(1), "the forbidden crossing reddens:\n{out}");
    assert!(out.contains("forbidden edge"), "out:\n{out}");
}
