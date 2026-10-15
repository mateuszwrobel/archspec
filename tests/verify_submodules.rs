//! Submodule dependency engagement (workplan
//! `workplan_archspec_submodule_dependency_engagement.md`): nested claims
//! carrying an `allowed` table gate edges with the same jurisdiction flat
//! top-level claims have, policy-free nested claims inherit the nearest
//! ancestor table, ancestor↔descendant pairs are composition, and the
//! ambiguity/engagement checks span every depth. Nested and flat equivalents
//! are byte-identical; the `nested_*`/`flat_*` pairs pin exactly that.

mod common;

use common::{stderr, stdout, Fixture};

/// Single-crate `app` tree: root module `app` plus the named module files.
/// `imports` maps a module file to the crate modules it uses.
fn app_tree(fixture: &Fixture, files: &[(&str, &[&str])]) {
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let mut mods: Vec<&str> = Vec::new();
    for (module, imports) in files {
        let name = module.rsplit("::").next().unwrap_or(module);
        mods.push(name);
        let mut body = String::new();
        for target in *imports {
            body.push_str(&format!("use crate::{target};\n"));
        }
        body.push_str(&format!("pub fn _{name}() {{ crate::{module}_uses(); }}\n"));
        // A use-only file would be pruned as dead code; keep one reference.
        let _ = body;
        fixture.write(
            &format!("src/{name}.rs"),
            &format!(
                "pub fn _{name}() {{\n{}\n}}\n",
                imports
                    .iter()
                    .map(|t| format!("    let _ = <dyn core::any::Any>::type_id;\n    let _ = crate::{t}::used;"))
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
        );
    }
    let mod_lines: Vec<String> = mods.iter().map(|m| format!("mod {m};")).collect();
    let pub_lines: Vec<String> = mods
        .iter()
        .map(|m| format!("pub fn {m}_uses() {{ /* {m} */ }}"))
        .collect();
    fixture.write(
        "src/lib.rs",
        &format!(
            "{}\n{}\n",
            mod_lines.join("\n"),
            pub_lines.join("\n")
        ),
    );
}

fn spec(header: &str, body: &str) -> String {
    format!("[project]\nlanguage = \"rust\"\n\n{header}{body}")
}

fn write_spec(fixture: &Fixture, spec: &str) {
    fixture.write("architecture.spec.toml", spec);
}

fn assert_pass(output: &std::process::Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0 (stderr: {})",
        stderr(output)
    );
}

fn assert_fail(output: &std::process::Output, expected_lines: &[&str]) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected non-zero exit (stderr: {})",
        stderr(output)
    );
    let out = stdout(output);
    assert!(
        out.contains("architecture.spec.toml does not match source model"),
        "report header on stdout:\n{out}"
    );
    for line in expected_lines {
        assert!(out.contains(line), "report must contain `{line}`:\n{out}");
    }
    assert!(
        stderr(output).is_empty(),
        "rule violations are report content, not stderr"
    );
}

// ---------------------------------------------------------------------------
// Scenario: nested `allowed` engages (workplan scenario 1)
// ---------------------------------------------------------------------------

fn engaging_nested_spec(submodule_table: &str) -> String {
    spec(
        "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n",
        &format!(
            "{submodule_table}\n[[module]]\nname = \"legacy\"\nmatches = {{ modules = [\"app::legacy\"] }}\n"
        ),
    )
}

#[test]
fn nested_allowed_engages_disallowed_edge() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["legacy"]), ("legacy", &[])]);
    write_spec(
        &fixture,
        &engaging_nested_spec(
            "[[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.submodules.allowed]\ndepend_on = []\n\n",
        ),
    );
    let output = fixture.run(&["verify"]);
    assert_fail(
        &output,
        &["disallowed cross-component dependency: billing -> legacy"],
    );
}

// ---------------------------------------------------------------------------
// Scenario: nested and flat equivalents are indistinguishable (B1)
// ---------------------------------------------------------------------------

fn equivalence_tree(fixture: &Fixture, billing_imports: &[&str]) {
    let mut files: Vec<(&str, Vec<&str>)> =
        vec![("billing", billing_imports.to_vec()), ("core", vec![]), ("legacy", vec![])];
    let _ = files.as_mut_slice();
    app_tree(
        fixture,
        &[
            ("billing", billing_imports),
            ("core", &[]),
            ("legacy", &[]),
        ],
    );
}

const FLAT_EQUIVALENT: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
    [[module]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
    [module.allowed]\ndepend_on = [\"core\"]\n\n\
    [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n\n\
    [[module]]\nname = \"legacy\"\nmatches = { modules = [\"app::legacy\"] }\n";

const NESTED_EQUIVALENT: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
    [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
    [module.submodules.allowed]\ndepend_on = [\"core\"]\n\n\
    [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n\n\
    [[module]]\nname = \"legacy\"\nmatches = { modules = [\"app::legacy\"] }\n";

#[test]
fn nested_and_flat_equivalents_are_byte_identical_passing() {
    let nested = Fixture::new();
    equivalence_tree(&nested, &["core"]);
    write_spec(&nested, NESTED_EQUIVALENT);
    let flat = Fixture::new();
    equivalence_tree(&flat, &["core"]);
    write_spec(&flat, FLAT_EQUIVALENT);

    let nested_run = nested.run(&["verify"]);
    let flat_run = flat.run(&["verify"]);
    assert_pass(&nested_run);
    assert_pass(&flat_run);
    assert_eq!(stdout(&nested_run), stdout(&flat_run));
    assert!(
        stdout(&nested_run).contains("(4 modules,"),
        "counts line enumerates every claim at every depth:\n{}",
        stdout(&nested_run)
    );
}

#[test]
fn nested_and_flat_equivalents_are_byte_identical_failing() {
    let nested = Fixture::new();
    equivalence_tree(&nested, &["legacy"]);
    write_spec(&nested, NESTED_EQUIVALENT);
    let flat = Fixture::new();
    equivalence_tree(&flat, &["legacy"]);
    write_spec(&flat, FLAT_EQUIVALENT);

    let nested_run = nested.run(&["verify"]);
    let flat_run = flat.run(&["verify"]);
    assert_fail(&nested_run, &["disallowed cross-component dependency: billing -> legacy"]);
    assert_eq!(nested_run.status.code(), flat_run.status.code());
    assert_eq!(stdout(&nested_run), stdout(&flat_run));
}

// ---------------------------------------------------------------------------
// Scenario: policy-free nested claim inherits the ancestor policy
// ---------------------------------------------------------------------------

const INHERIT_SPEC: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n\
    [module.allowed]\ndepend_on = [\"core\"]\n\n\
    [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
    [module.submodules.contract]\nforbid = []\n\n\
    [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n";

#[test]
fn policy_free_nested_claim_inherits_ancestor_allowance() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["core"]), ("core", &[])]);
    write_spec(&fixture, INHERIT_SPEC);
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let out = stdout(&output);
    assert!(out.contains("(3 modules,"), "claims at both depths counted:\n{out}");
    assert!(!out.contains("dead reference"), "no dead reference:\n{out}");
    assert!(!out.contains("unmatched claim"), "no unmatched claim:\n{out}");
}

#[test]
fn policy_free_nested_claim_cannot_smuggle_a_denied_edge() {
    let fixture = Fixture::new();
    app_tree(
        &fixture,
        &[("billing", &["legacy"]), ("core", &[]), ("legacy", &[])],
    );
    write_spec(
        &fixture,
        &format!(
            "{INHERIT_SPEC}\n[[module]]\nname = \"legacy\"\nmatches = {{ modules = [\"app::legacy\"] }}\n"
        ),
    );
    let output = fixture.run(&["verify"]);
    assert_fail(
        &output,
        &["disallowed cross-component dependency: billing -> legacy"],
    );
}

// ---------------------------------------------------------------------------
// Scenario: containment edges are composition, not dependencies (B2)
// ---------------------------------------------------------------------------

const CONTAINMENT_PARENT: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n\
    [module.allowed]\ndepend_on = []\n";

#[test]
fn containment_edges_are_composition_not_dependencies() {
    let fixture = Fixture::new();
    // Root module files import the submodule's files, and vice versa: both
    // directions of the parent<->child pair stay out of adjudication.
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "mod billing;\nmod root;\npub use root::wired;\npub fn billing_uses() {}\n",
    );
    fixture.write(
        "src/root.rs",
        "pub fn wired() {\n    let _ = <dyn core::any::Any>::type_id;\n    crate::billing::used();\n}\n",
    );
    fixture.write(
        "src/billing.rs",
        "pub fn used() {\n    let _ = crate::root::wired;\n}\n",
    );
    write_spec(
        &fixture,
        &format!(
            "{CONTAINMENT_PARENT}\n[[module.submodules]]\nname = \"billing\"\nmatches = {{ modules = [\"app::billing\"] }}\n"
        ),
    );
    assert_pass(&fixture.run(&["verify"]));
}

#[test]
fn cross_claim_edge_next_to_containment_still_adjudicates() {
    let fixture = Fixture::new();
    app_tree(
        &fixture,
        &[("billing", &["legacy"]), ("legacy", &[])],
    );
    write_spec(
        &fixture,
        &format!(
            "{CONTAINMENT_PARENT}\n[[module.submodules]]\nname = \"billing\"\nmatches = {{ modules = [\"app::billing\"] }}\n\n[[module]]\nname = \"legacy\"\nmatches = {{ modules = [\"app::legacy\"] }}\n"
        ),
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["disallowed cross-component dependency: billing -> legacy"],
    );
}

// ---------------------------------------------------------------------------
// Scenario: sibling submodule claims adjudicate normally (D4)
// ---------------------------------------------------------------------------

#[test]
fn sibling_submodule_forbidden_edge_adjudicates() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["legacy"]), ("legacy", &[])]);
    write_spec(
        &fixture,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
             [[module.submodules]]\nname = \"application-billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.submodules.allowed]\ndepend_on = []\nforbidden = [\"application-legacy\"]\n\n\
             [[module.submodules]]\nname = \"application-legacy\"\nmatches = { modules = [\"app::legacy\"] }\n\n\
             [module.submodules.allowed]\ndepend_on = []\n",
            "",
        ),
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden edge: application-billing -> application-legacy"],
    );
}

// ---------------------------------------------------------------------------
// Scenario: a descendant cannot cancel an ancestor prohibition (ratchet)
// ---------------------------------------------------------------------------

#[test]
fn descendant_cannot_cancel_ancestor_prohibition() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["legacy"]), ("legacy", &[])]);
    write_spec(
        &fixture,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n\
             [module.allowed]\ndepend_on = [\"core\"]\nforbidden = [\"legacy\"]\n\n\
             [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.submodules.allowed]\ndepend_on = []\nforbidden = []\n\n\
             [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n\n\
             [[module]]\nname = \"legacy\"\nmatches = { modules = [\"app::legacy\"] }\n",
            "",
        ),
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden edge: billing -> legacy"],
    );
}

// ---------------------------------------------------------------------------
// Scenario: a descendant may narrow an ancestor allowance
// ---------------------------------------------------------------------------

#[test]
fn descendant_may_narrow_ancestor_allowance() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["core"]), ("core", &[])]);
    write_spec(
        &fixture,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n\
             [module.allowed]\ndepend_on = [\"core\"]\n\n\
             [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.submodules.allowed]\ndepend_on = []\n\n\
             [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n",
            "",
        ),
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["disallowed cross-component dependency: billing -> core"],
    );
}

// ---------------------------------------------------------------------------
// Scenario: claim names engage as targets at every depth (D6)
// ---------------------------------------------------------------------------

const TARGETS_SPEC: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
    [module.allowed]\nforbidden = [\"billing\"]\n\n\
    [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
    [module.submodules.allowed]\ndepend_on = [\"core\"]\nforbidden = [\"legacy-sub\"]\n\n\
    [[module.submodules]]\nname = \"legacy-sub\"\nmatches = { modules = [\"app::legacy\"] }\n\n\
    [module.submodules.allowed]\ndepend_on = []\n\n\
    [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n";

#[test]
fn claim_names_targeted_at_every_depth_are_not_dead_references() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["core"]), ("core", &[]), ("legacy", &[])]);
    write_spec(&fixture, TARGETS_SPEC);
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let out = stdout(&output);
    assert!(!out.contains("dead reference"), "every target resolves:\n{out}");
}

#[test]
fn sibling_prohibition_named_across_depths_engages() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["legacy"]), ("core", &[]), ("legacy", &[])]);
    write_spec(&fixture, TARGETS_SPEC);
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden edge: billing -> legacy-sub"],
    );
}

// ---------------------------------------------------------------------------
// Scenario: one claim name at two depths is refused at parse (D6 weld)
// ---------------------------------------------------------------------------

const DUPLICATE_NAME_SPEC: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
    [[module.submodules]]\nname = \"legacy\"\nmatches = { modules = [\"app::legacy\"] }\n\n\
    [[module]]\nname = \"legacy\"\nmatches = { modules = [\"app::old\"] }\n";

fn duplicate_name_rejection_assertions(output: &std::process::Output, verb: &str) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "a claim name declared at two depths must fail {verb} (stdout: {})",
        stdout(output)
    );
    let err = stderr(output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("claim name \"legacy\" is declared twice")
            && err.contains("claim names are unique across all depths"),
        "{verb} must refuse the duplicate claim name, naming both depths:\n{err}"
    );
}

#[test]
fn duplicate_claim_name_across_depths_is_rejected_by_every_schema_validating_verb() {
    // Uniqueness lives in the schema validation backing verify's and
    // report's loaders (`spec::load_verify` / `spec::load_report`); the
    // diagram loader validates the submodule-unit rule only, so this weld
    // pins the two verbs that read the schema at load — like
    // `nested_matches_units_is_rejected_by_every_spec_loading_verb` pins
    // all three that share the unit-tier rule.
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &[])]);
    write_spec(&fixture, DUPLICATE_NAME_SPEC);

    duplicate_name_rejection_assertions(&fixture.run(&["verify"]), "verify");
    duplicate_name_rejection_assertions(&fixture.run(&["report"]), "report");
}

// ---------------------------------------------------------------------------
// Scenario: `matches.units` on a submodule is rejected at parse (M4/D7)
// ---------------------------------------------------------------------------

const NESTED_UNITS_SPEC: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n\
    [[module.submodules]]\nname = \"billing\"\nmatches = { units = [\"app\"], modules = [\"app::billing\"] }\n";

fn units_rejection_assertions(output: &std::process::Output, verb: &str) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "nested matches.units must fail {verb} (stdout: {})",
        stdout(output)
    );
    let err = stderr(output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("billing")
            && err.contains("matches.units")
            && err.contains("module tier only"),
        "{verb} must reject nested matches.units naming claim and field:\n{err}"
    );
}

#[test]
fn nested_matches_units_is_rejected_by_every_spec_loading_verb() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &[])]);
    write_spec(&fixture, NESTED_UNITS_SPEC);

    units_rejection_assertions(&fixture.run(&["verify"]), "verify");
    let diagram = fixture.run(&["diagram"]);
    units_rejection_assertions(&diagram, "diagram");
    assert!(
        !stdout(&diagram).contains("graph TD") && !stdout(&diagram).contains("@startuml"),
        "no diagram body on stdout:\n{}",
        stdout(&diagram)
    );
    let report = fixture.run(&["report"]);
    units_rejection_assertions(&report, "report");
}

// ---------------------------------------------------------------------------
// Scenario: ambiguity spans depths regardless of declaration order (B3/D8)
// ---------------------------------------------------------------------------

const AMBIGUITY_A: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"flat-b\"\nmatches = { modules = [\"app::billing\"] }\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
    [[module.submodules]]\nname = \"sub-b\"\nmatches = { modules = [\"app::billing\"] }\n";

const AMBIGUITY_B: &str = "[project]\nlanguage = \"rust\"\n\n\
    [[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
    [[module.submodules]]\nname = \"sub-b\"\nmatches = { modules = [\"app::billing\"] }\n\n\
    [[module]]\nname = \"flat-b\"\nmatches = { modules = [\"app::billing\"] }\n";

#[test]
fn cross_depth_ambiguity_never_depends_on_declaration_order() {
    let first = Fixture::new();
    app_tree(&first, &[("billing", &[])]);
    write_spec(&first, AMBIGUITY_A);
    let second = Fixture::new();
    app_tree(&second, &[("billing", &[])]);
    write_spec(&second, AMBIGUITY_B);

    let first_run = first.run(&["verify"]);
    let second_run = second.run(&["verify"]);
    assert_fail(
        &first_run,
        &["ambiguous module match: app::billing claimed by both flat-b entry 'app::billing' and sub-b entry 'app::billing' (equal specificity)"],
    );
    assert_eq!(first_run.status.code(), second_run.status.code());
    assert_eq!(stdout(&first_run), stdout(&second_run));
}

// ---------------------------------------------------------------------------
// Scenario: a claim matching nothing warns at any depth (M6/D9a)
// ---------------------------------------------------------------------------

#[test]
fn nested_claim_matching_nothing_warns() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["legacy"]), ("legacy", &[])]);
    write_spec(
        &fixture,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n\
             [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::ghost\"] }\n",
            "",
        ),
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let out = stdout(&output);
    assert!(
        out.contains("warning: unmatched claim: billing"),
        "warning-severity unmatched claim at depth:\n{out}"
    );
    assert!(!out.contains("matches source model"), "never a clean claim:\n{out}");
    let strict = fixture.run(&["verify", "--strict"]);
    assert_fail(&strict, &["unmatched claim: billing"]);
    assert!(
        stdout(&strict).contains("  unmatched claim: billing\n"),
        "no `warning: ` prefix under --strict:\n{}",
        stdout(&strict)
    );
}

#[test]
fn flat_claim_matching_nothing_warns() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &["legacy"]), ("legacy", &[])]);
    write_spec(
        &fixture,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n\
             [[module]]\nname = \"ghostly\"\nmatches = { modules = [\"app::ghost\"] }\n",
            "",
        ),
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let out = stdout(&output);
    assert!(
        out.contains("warning: unmatched claim: ghostly"),
        "same warning at top level:\n{out}"
    );
}

#[test]
fn unmatched_claim_warns_on_a_module_tier_less_go_tree() {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("demo.go", "package demo\n\nfunc Run() {}\n");
    write_spec(
        &fixture,
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"example.com/demo\"] }\n\n[[module]]\nname = \"ghostly\"\nmatches = { modules = [\"example.com/demo/internal/ghost\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        stdout(&output).contains("warning: unmatched claim: ghostly"),
        "model-side check covers tier-less legs:\n{}",
        stdout(&output)
    );
}

// ---------------------------------------------------------------------------
// Scenario: a shadowed claim warns (D9b)
// ---------------------------------------------------------------------------

#[test]
fn nested_claim_fully_shadowed_by_more_specific_claim_warns() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &[]), ("legacy", &[])]);
    write_spec(
        &fixture,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::billing\", \"app::legacy\"] }\n\n\
             [[module.submodules]]\nname = \"wide\"\nmatches = { modules = [\"app::*\"] }\n",
            "",
        ),
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let out = stdout(&output);
    assert!(
        out.contains("warning: shadowed claim: wide is fully claimed by app"),
        "shadow warning names both claims:\n{out}"
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_fail(&strict, &["shadowed claim: wide"]);
}

// ---------------------------------------------------------------------------
// Scenario: a dead target inside a submodule's allowed list warns like flat
// ---------------------------------------------------------------------------

#[test]
fn nested_dead_reference_matches_flat_warning() {
    let nested = Fixture::new();
    app_tree(&nested, &[("billing", &[])]);
    write_spec(
        &nested,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n\
             [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.submodules.allowed]\ndepend_on = [\"ghost\"]\n",
            "",
        ),
    );
    let flat = Fixture::new();
    app_tree(&flat, &[("billing", &[])]);
    write_spec(
        &flat,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\", \"app::*\"] }\n\n\
             [[module]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.allowed]\ndepend_on = [\"ghost\"]\n",
            "",
        ),
    );

    let nested_run = nested.run(&["verify"]);
    let flat_run = flat.run(&["verify"]);
    assert_pass(&nested_run);
    assert_pass(&flat_run);
    let pattern =
        "dead reference: module 'billing' allowed.depend_on target \"ghost\" does not exist";
    assert!(
        stdout(&nested_run).contains(pattern),
        "same dead reference at depth:\n{}",
        stdout(&nested_run)
    );
    assert!(stdout(&flat_run).contains(pattern));
    assert_fail(
        &nested.run(&["verify", "--strict"]),
        &["dead reference: module 'billing'"],
    );
}

// ---------------------------------------------------------------------------
// Scenario: a declared-but-unexercised `depend_on` at depth is a missing edge
// ---------------------------------------------------------------------------

#[test]
fn nested_missing_edge_matches_flat_verdict_and_text() {
    let nested = Fixture::new();
    app_tree(&nested, &[("billing", &[]), ("core", &[])]);
    write_spec(
        &nested,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
             [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.submodules.allowed]\ndepend_on = [\"core\"]\n\n\
             [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n",
            "",
        ),
    );
    let flat = Fixture::new();
    app_tree(&flat, &[("billing", &[]), ("core", &[])]);
    write_spec(
        &flat,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
             [[module]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.allowed]\ndepend_on = [\"core\"]\n\n\
             [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n",
            "",
        ),
    );
    assert_fail(
        &nested.run(&["verify"]),
        &["missing edge: billing -> core"],
    );
    let flat_run = flat.run(&["verify"]);
    assert_fail(&flat_run, &["missing edge: billing -> core"]);
}

// ---------------------------------------------------------------------------
// Scenario: the diagram renders every depth verify enforces (D11)
// ---------------------------------------------------------------------------

#[test]
fn diagram_spec_source_renders_submodule_claims_and_edges_at_full_depth() {
    let fixture = Fixture::new();
    app_tree(&fixture, &[("billing", &[])]);
    write_spec(
        &fixture,
        &spec(
            "[[module]]\nname = \"app\"\nmatches = { units = [\"app\"], modules = [\"app\"] }\n\n\
             [module.allowed]\ndepend_on = [\"billing\", \"core\"]\n\n\
             [[module.submodules]]\nname = \"billing\"\nmatches = { modules = [\"app::billing\"] }\n\n\
             [module.submodules.allowed]\ndepend_on = [\"core\"]\n\n\
             [[module.submodules]]\nname = \"deep\"\nmatches = { modules = [\"app::billing::deep\"] }\n\n\
             [[module]]\nname = \"core\"\nmatches = { modules = [\"app::core\"] }\n",
            "",
        ),
    );
    let output = fixture.run(&["diagram"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("billing"), "submodule node rendered:\n{out}");
    assert!(out.contains("deep"), "depth-2 submodule node rendered:\n{out}");
    assert!(
        out.contains("billing --> core"),
        "declared allowance arrow from a submodule:\n{out}"
    );
    assert!(
        !out.contains("app --> billing") && !out.contains("billing --> app"),
        "containment renders as nesting, never as an allowed-edge arrow:\n{out}"
    );
}
