//! The contract expose chain law (workplan
//! rust-arch-test-kit/worklog/workplan_archspec_module_public_api_diagrams.md):
//! `contract.expose` declares the exact public surface of the claiming module;
//! a crossing symbol must be declared by every ancestor claim that declares
//! an expose set (D2), a symbol fires once attributed to the deepest claiming
//! claim (D3), staleness is per entry territory-wide (D4), the classification
//! ladder is stereotype-first (D5), vacuity rides the shipped `vacuous
//! constraint` category with the named-subject carrier (D7), and the C# root
//! rows are the allowlist's sole territory (D10).

mod common;

use common::{stderr, stdout};

fn csproj() -> String {
    "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n"
        .to_string()
}

/// The App tree: project `App`; namespace `App.Billing` exports `Report` and
/// `Secret`; nested namespace `App.Billing.Payments` exports `PaymentLedger`
/// when `payments` is true.
fn app_tree(billing: &[&str], payments: bool) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    let types: Vec<String> = billing
        .iter()
        .map(|name| format!("public class {name} {{ }}\n"))
        .collect();
    fixture.write(
        "App/Billing.cs",
        &format!("namespace App.Billing;\n{}", types.concat()),
    );
    if payments {
        fixture.write(
            "App/Payments.cs",
            "namespace App.Billing.Payments;\npublic class PaymentLedger { }\n",
        );
    }
    fixture
}

fn run(fixture: &common::Fixture, spec: &str, args: &[&str]) -> std::process::Output {
    fixture.write("architecture.spec.toml", spec);
    let mut full: Vec<&str> = vec!["verify"];
    full.extend_from_slice(args);
    fixture.run(&full)
}

fn spec_of(body: &str) -> String {
    format!("[project]\nlanguage = \"csharp\"\n\n{body}")
}

/// One symbol, one line, attributed to the deepest claiming claim; the parent
/// depth never repeats it (D3).
#[test]
fn expose_leak_fires_once_at_the_deepest_claiming_claim() {
    let fixture = app_tree(&["Report", "Secret"], false);
    let output = run(
        &fixture,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[module.submodules]]\nname = \"App::Billing\"\nmatches = { modules = [\"App::Billing\"] }\ncontract = { expose = [\"Report\"] }\n",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(1), "undeclared crossing reds");
    let out = stdout(&output);
    assert!(
        out.contains("public api leak: App::Billing exposes Secret (not exposed)"),
        "one line at the deepest claim:\n{out}"
    );
    assert!(
        !out.contains("App exposes Secret"),
        "no parent-depth repeat line:\n{out}"
    );
    assert_eq!(
        out.matches("(not exposed)").count(),
        1,
        "exactly one expose leak line:\n{out}"
    );
}

/// The chain exploit: splitting a module and declaring the crossing at the
/// child grants nothing — the ancestor never permitted it (D2).
#[test]
fn split_to_weaken_grants_nothing() {
    let fixture_a = app_tree(&["Report"], true);
    let out_a = run(
        &fixture_a,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\"] }\n",
        ),
        &[],
    );
    assert_eq!(out_a.status.code(), Some(1), "ancestor-only spec reds");

    let fixture_b = app_tree(&["Report"], true);
    let out_b = run(
        &fixture_b,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\"] }\n\n[[module.submodules]]\nname = \"App::Billing::Payments\"\nmatches = { modules = [\"App::Billing::Payments\"] }\ncontract = { expose = [\"Report\", \"PaymentLedger\"] }\n",
        ),
        &[],
    );
    assert_eq!(
        out_b.status.code(),
        Some(1),
        "the split grants nothing (stderr: {})",
        stderr(&out_b)
    );
    let out = stdout(&out_b);
    assert!(
        out.contains("public api leak: App::Billing::Payments exposes PaymentLedger (not exposed)"),
        "the line names the deepest declaring claim:\n{out}"
    );
    assert_eq!(
        out.matches("exposes PaymentLedger").count(),
        1,
        "one line per symbol at any chain depth:\n{out}"
    );
}

/// An ancestor without a contract table claims nothing; a declared expose set
/// binds exactly; a spec free of expose is verdict-neutral.
#[test]
fn ancestor_silence_claims_nothing_declared_ancestor_binds() {
    let fixture = app_tree(&["Report", "Secret"], false);
    let output = run(
        &fixture,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[module.submodules]]\nname = \"App::Billing\"\nmatches = { modules = [\"App::Billing\"] }\ncontract = { expose = [\"Report\"] }\n",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(1));
    let out = stdout(&output);
    assert!(
        out.contains("public api leak: App::Billing exposes Secret (not exposed)"),
        "the submodule's own set governs Secret:\n{out}"
    );
    assert!(
        !out.contains("exposes Report"),
        "the submodule permits Report:\n{out}"
    );

    let quiet = app_tree(&["Report", "Secret"], false);
    let output = run(
        &quiet,
        &spec_of("[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n"),
        &[],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "a claim-making-free spec is verdict-neutral (stderr: {})",
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "pass runs keep stderr empty");
}

/// Nesting narrows, never widens: the child's narrower set governs the child's
/// surface, and an empty set is a declared deny-all ceiling.
#[test]
fn nesting_narrows_never_widens() {
    let fixture = app_tree(&["Audit", "Report"], false);
    let output = run(
        &fixture,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\", \"Audit\"] }\n\n[[module.submodules]]\nname = \"App::Billing\"\nmatches = { modules = [\"App::Billing\"] }\ncontract = { expose = [\"Report\"] }\n",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(1));
    let out = stdout(&output);
    assert!(
        out.contains("public api leak: App::Billing exposes Audit (not exposed)"),
        "the child's narrower set reds the child's crossing:\n{out}"
    );
    assert!(
        !out.contains("exposes Report"),
        "the child permits what its own set lists:\n{out}"
    );

    let denied = app_tree(&["Audit", "Report"], false);
    let output = run(
        &denied,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\", \"Audit\"] }\n\n[[module.submodules]]\nname = \"App::Billing\"\nmatches = { modules = [\"App::Billing\"] }\ncontract = { expose = [] }\n",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(1));
    let out = stdout(&output);
    assert!(
        out.contains("public api leak: App::Billing exposes Audit (not exposed)")
            && out.contains("public api leak: App::Billing exposes Report (not exposed)"),
        "an empty set is a deny-all ceiling for the declaring claim:\n{out}"
    );
}

/// Flat versus nested spellings of the same law are byte-identical (the pair
/// law): same lines, same counts, no two-depth multi-line firing.
#[test]
fn nested_and_flat_expose_specs_are_byte_identical() {
    let flat = app_tree(&["Report", "Secret"], false);
    let flat_out = run(
        &flat,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[module]]\nname = \"App::Billing\"\nmatches = { modules = [\"App::Billing\"] }\ncontract = { expose = [\"Report\"] }\n",
        ),
        &[],
    );
    let nested = app_tree(&["Report", "Secret"], false);
    let nested_out = run(
        &nested,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[module.submodules]]\nname = \"App::Billing\"\nmatches = { modules = [\"App::Billing\"] }\ncontract = { expose = [\"Report\"] }\n",
        ),
        &[],
    );
    assert_eq!(flat_out.status.code(), Some(1));
    assert_eq!(nested_out.status.code(), Some(1));
    assert_eq!(
        stdout(&flat_out),
        stdout(&nested_out),
        "flat and nested spellings must be byte-identical"
    );
}

/// An ancestor entry cleared by a descendant's export is engaged: staleness
/// reads own files UNION descendant exports (the fix-loop-poison closure).
#[test]
fn descendant_export_clears_the_ancestor_entry() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    fixture.write(
        "App/Billing.cs",
        "namespace App.Billing;\npublic class Ledger { }\n",
    );
    let output = run(
        &fixture,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Ledger\"] }\n\n[[module.submodules]]\nname = \"App::Billing\"\nmatches = { modules = [\"App::Billing\"] }\n",
        ),
        &[],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the descendant's export satisfies the ancestor entry (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        !out.contains("stale contract export"),
        "no staleness names the satisfied entry:\n{out}"
    );
}

/// Staleness is per entry: an entry globbing zero across the territory warns
/// while its engaged siblings stay quiet; `--strict` reds it.
#[test]
fn stale_expose_entry_warns_per_entry() {
    let fixture = app_tree(&["Report"], false);
    let output = run(
        &fixture,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\", \"Ghost*\"] }\n",
        ),
        &[],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "staleness is a warning (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("stale contract export: App declares Ghost* (never exported across its territory)"),
        "the stale entry is named per entry:\n{out}"
    );
    assert!(
        !out.contains("declares Report"),
        "the engaged entry stays quiet:\n{out}"
    );

    let strict = app_tree(&["Report"], false);
    let output = run(
        &strict,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\", \"Ghost*\"] }\n",
        ),
        &["--strict"],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "--strict promotes per-entry staleness"
    );
}

/// An entry that names no declared stereotype and matches no symbol anywhere
/// gets the shipped dead-reference classification — forbid parity.
#[test]
fn expose_entry_naming_nothing_is_a_dead_reference() {
    let fixture = app_tree(&["Report"], false);
    let output = run(
        &fixture,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\", \"NoSuchName\"] }\n",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "dead reference warns");
    let out = stdout(&output);
    assert!(
        out.contains("dead reference: module 'App' contract.expose target \"NoSuchName\" names no stereotype"),
        "the shipped classification covers expose entries:\n{out}"
    );
    assert!(
        !out.contains("stale contract export"),
        "dead reference replaces per-entry staleness for a name that exists nowhere:\n{out}"
    );

    let strict = app_tree(&["Report"], false);
    let output = run(
        &strict,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\", \"NoSuchName\"] }\n",
        ),
        &["--strict"],
    );
    assert_eq!(output.status.code(), Some(1), "--strict reds the dead reference");
}

/// An entry naming a declared stereotype AND matching symbols adjudicates the
/// stereotype arm and warns — never a silent symbol pass (D5).
#[test]
fn ambiguous_expose_entry_adjudicates_stereotype_arm_and_warns() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    fixture.write(
        "App/Api.cs",
        "namespace App.Api;\npublic class entity { }\n",
    );
    let output = run(
        &fixture,
        &spec_of(
            "[[stereotype]]\nname = \"entity\"\nmatch = { names = [\"entity\"] }\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"entity\"] }\n",
        ),
        &[],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the stereotype arm adjudicates the crossing (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("ambiguous expose entry: App entry entity names stereotype entity and matches symbols"),
        "the ambiguity is stated, the pass is never silent:\n{out}"
    );
    assert!(
        !out.contains("(not exposed)"),
        "the crossing is permitted through the stereotype arm:\n{out}"
    );

    let strict = common::Fixture::new();
    strict.write("App/App.csproj", &csproj());
    strict.write("App/Api.cs", "namespace App.Api;\npublic class entity { }\n");
    let output = run(
        &strict,
        &spec_of(
            "[[stereotype]]\nname = \"entity\"\nmatch = { names = [\"entity\"] }\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"entity\"] }\n",
        ),
        &["--strict"],
    );
    assert_eq!(output.status.code(), Some(1), "--strict reds the ambiguity");
}

/// The C# root rows are the allowlist's sole territory (D10): a namespace-less
/// public type fires the allowlist exactly once and the expose law never, even
/// under a claim whose patterns cover the root key; a non-root type under the
/// same claim still fires the expose law.
#[test]
fn csharp_root_rows_fire_the_allowlist_only() {
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj());
    fixture.write("App/Global.cs", "public class RootThing { }\n");
    fixture.write("App/Api.cs", "namespace App.Api;\npublic class Hidden { }\n");
    let output = run(
        &fixture,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"], modules = [\"App\", \"App::*\"] }\ncontract = { expose = [] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"App::Api\"]\n",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(1));
    let out = stdout(&output);
    assert_eq!(
        out.matches("RootThing").count(),
        1,
        "the root type fires exactly one line — the allowlist's:\n{out}"
    );
    assert!(
        out.contains("public api leak: App exposes RootThing (not allowlisted)"),
        "the allowlist owns the root row:\n{out}"
    );
    assert!(
        !out.contains("RootThing (not exposed)"),
        "the expose law never reaches the root key:\n{out}"
    );
    assert!(
        out.contains("public api leak: App exposes Hidden (not exposed)"),
        "the non-root crossing still fires the expose law, attributed to the declaring claim:\n{out}"
    );
}

/// Engaged and vacuous arms both print in one run (rust): the stereotype arm
/// adjudicates the crossing surface through the shipped classification
/// machinery while the symbol arm states its own vacuity — neither arm
/// silences the other.
#[test]
fn stereotype_arm_engages_while_symbol_arm_states_vacuity() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod b;\n");
    fixture.write("src/b.rs", "pub mod c;\n");
    fixture.write("src/b/c.rs", "pub fn c() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[stereotype]]\nname = \"c\"\nmatch = { names = [\"c\"] }\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[[module.submodules]]\nname = \"a::b\"\nmatches = { modules = [\"a::b\"] }\ncontract = { expose = [\"c\", \"Report\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "no error arms fire without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        !out.contains("(not exposed)"),
        "the stereotype arm adjudicates the crossing silently:\n{out}"
    );
    assert!(
        !out.contains("stale contract export"),
        "arm vacuity replaces per-entry staleness on a driver without symbol facts:\n{out}"
    );
    assert!(
        out.contains("vacuous constraint: [contract.expose of a::b]:"),
        "the symbol arm states its own vacuity line:\n{out}"
    );
}

/// The expose vacuity line names capability, tier, and the remedy pointer —
/// language-neutral, version-free on every driver (a go tree must not print
/// rust vocabulary).
#[test]
fn expose_vacuity_names_capability_tier_and_pointer_on_go() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("main.go", "package main\n\nfunc main() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"demo\"\nmatches = { modules = [\"example.com/demo\"] }\ncontract = { expose = [\"Report\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "vacuity is a warning (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("vacuous constraint: [contract.expose of demo]:"),
        "the named-subject carrier rides the shipped category:\n{out}"
    );
    assert!(
        out.contains("public-API surface") && out.contains("archspec help capabilities"),
        "the sentence names capability and the remedy pointer:\n{out}"
    );
    assert!(
        !out.contains("crate"),
        "no rust vocabulary on a go tree:\n{out}"
    );
    assert!(
        !out.contains(env!("CARGO_PKG_VERSION")),
        "no tool version on the line:\n{out}"
    );
}

/// The failing run's report renders the expose leak class; the run's stderr
/// footer carries the version instead of the report bytes (D8 split).
#[test]
fn expose_leak_report_keeps_bytes_stable_across_runs() {
    let fixture = app_tree(&["Report", "Secret"], false);
    let spec = spec_of(
        "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\"] }\n",
    );
    let first = run(&fixture, &spec, &[]);
    let second = run(&fixture, &spec, &[]);
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "expose findings keep the byte-stability promise"
    );
}

/// The version footer answers on every non-zero `verify` exit and is absent
/// from every stdout byte (the committed-artifact stability law, D8).
#[test]
fn failing_verify_ends_stderr_with_the_version_footer() {
    let fixture = app_tree(&["Report"], false);
    let output = run(
        &fixture,
        &spec_of("[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [] }\n"),
        &[],
    );
    assert_eq!(output.status.code(), Some(1));
    // Raw stderr on purpose: the shared `stderr()` helper strips the footer
    // for the diagnostic-byte tests; this test owns the footer itself.
    let err = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        err.ends_with(&format!("note: archspec {}\n", env!("CARGO_PKG_VERSION"))),
        "stderr ends with the single footer line:\n{err}"
    );
    assert!(
        !stdout(&output).contains(env!("CARGO_PKG_VERSION")),
        "stdout carries no version token"
    );

    let pass = app_tree(&["Report"], false);
    let output = run(
        &pass,
        &spec_of(
            "[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\ncontract = { expose = [\"Report\"] }\n",
        ),
        &[],
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stderr(&output).is_empty(),
        "a green run keeps stderr empty"
    );
}
