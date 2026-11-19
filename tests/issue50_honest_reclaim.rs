//! GitHub issue #50 — the honest reclaim law.
//!
//! A C# project that no `matches.units` glob claims, covered only by a
//! namespace-tier `matches.modules` claim through the dotted-stem reclaim,
//! must keep its `unassigned unit:` finding. Before the fix,
//! `augment_with_modules` re-classified such a unit out of `unassigned_units`
//! "so their edges still resolve", and `resolve_module_edges` then projected
//! its unit-tier edge onto the boundary graph as a mixed-tier pair —
//! `check_allowed_edges` printed `disallowed cross-component dependency:
//! Acme::Shop -> Acme.Shop.Api` instead of the honest finding.
//!
//! Cells (per the fix plan, `worklog/2026-11-18-csharp-audit-fix-plan-adversarial.md` P10):
//! 1. the issue's repro shape: `unassigned unit:` present, the mixed-tier
//!    pair line ABSENT, exit stays red (the `unassigned unit` field is
//!    error-tier in `Diff::error_fields` — the finding never gets promoted);
//! 2. claimed C# projects: unit-tier adjudication byte-identical (clean and
//!    violation runs pin the shipped lines);
//! 3. go tree D1 pin: a unit claimed by name through the `go_unit_claims`
//!    sub-arm stays a full claim — the go tree verifies clean, byte-identical;
//! 4. rust tree pin: a crate reclaimed from `unexpected_components` through
//!    the module-path sub-arm keeps the shipped behavior — clean pass, the
//!    pair still adjudicates (the soft-reclaim mark never arms from
//!    `unexpected_components`).

mod common;

use common::{stderr, stdout};

fn csproj(tf: &str) -> String {
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>{tf}</TargetFramework>\n  </PropertyGroup>\n</Project>\n"
    )
}

fn csproj_with_reference(tf: &str, reference: &str) -> String {
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>{tf}</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"{reference}\" />\n  </ItemGroup>\n</Project>\n"
    )
}

/// The issue's repro tree: the project `Acme.Shop` is claimed by NO
/// `matches.units` glob — only the namespace-tier `matches.modules` pattern
/// `Acme::Shop` covers it (through its soft paths) — and it uses
/// `Acme.Shop.Api`, a project claimed by `units` under its own boundary.
fn unclaimed_project_fixture(spec: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Acme.Shop/Acme.Shop.csproj",
        &csproj_with_reference("net8.0", r"..\Acme.Shop.Api\Acme.Shop.Api.csproj"),
    );
    fixture.write(
        "Acme.Shop/Shop.cs",
        "using Acme.Shop.Api;\nnamespace Acme.Shop;\npublic class ShopService { }\n",
    );
    fixture.write("Acme.Shop.Api/Acme.Shop.Api.csproj", &csproj("net8.0"));
    fixture.write(
        "Acme.Shop.Api/Api.cs",
        "namespace Acme.Shop.Api;\npublic class OrderApi { }\n",
    );
    fixture.write("architecture.spec.toml", spec);
    fixture
}

/// The repro shape, pinned by the issue's own names. Before the fix this run
/// printed `disallowed cross-component dependency: Acme::Shop ->
/// Acme.Shop.Api` and NO `unassigned unit` line. After the fix the reclaim
/// still places the unit (its edges resolve on the module tier), but the
/// honest finding speaks and the mixed-tier pair is not adjudicated.
#[test]
fn issue50_reclaim_from_unassigned_keeps_the_unassigned_finding() {
    let fixture = unclaimed_project_fixture(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Acme::Shop\"\nmatches = { units = [\"Acme\"], modules = [\"Acme::Shop\"] }\n\n[[module]]\nname = \"Acme.Shop.Api\"\nmatches = { units = [\"Acme.Shop.Api\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    // The `unassigned unit` field is error-tier (`Diff::error_fields`): the
    // finding replaces the wrong pair line at the SAME verdict tier — never
    // promoted to a warning, the exit stays non-zero.
    assert_ne!(
        output.status.code(),
        Some(0),
        "the unassigned-unit finding is error-tier (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("unassigned unit: Acme.Shop"),
        "the honest finding must speak:\n{out}"
    );
    assert!(
        !out.contains("disallowed cross-component dependency"),
        "the mixed-tier pair (soft-reclaimed source, claimed target) is not adjudicated:\n{out}"
    );
    assert!(
        stderr(&output).is_empty(),
        "rule violations are report content, not stderr:\n{}",
        stderr(&output)
    );
}

/// Cell 2 — claimed projects byte-identical: with `units` globs claiming BOTH
/// projects, the unit-tier edge resolves and adjudicates exactly as shipped.
/// The clean run passes with its two endpoint warnings; the violation run
/// still prints the disallowed pair — the skip arms ONLY on soft-reclaimed
/// sources, never on a claimed one.
#[test]
fn issue50_claimed_projects_adjudicate_byte_identically() {
    let clean = unclaimed_project_fixture(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"shop\"\nmatches = { units = [\"Acme.Shop\"] }\n\n[module.allowed]\ndepend_on = [\"api\"]\n\n[[module]]\nname = \"api\"\nmatches = { units = [\"Acme.Shop.Api\"] }\n",
    );
    let output = clean.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a claimed pair with `depend_on` passes (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("ok: architecture.spec.toml passes with 2 warnings"),
        "pass confirmation with the shipped warnings headline (issue #54):\n{out}"
    );
    for line in [
        "warning: unowned module edge endpoint: Acme::Shop",
        "warning: unowned module edge endpoint: Acme::Shop::Api",
    ] {
        assert!(out.contains(line), "endpoint warnings ship unchanged:\n{out}");
    }

    let violating = unclaimed_project_fixture(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"shop\"\nmatches = { units = [\"Acme.Shop\"] }\n\n[module.allowed]\ndepend_on = []\n\n[[module]]\nname = \"api\"\nmatches = { units = [\"Acme.Shop.Api\"] }\n",
    );
    let output = violating.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "a claimed pair outside `depend_on` stays red (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("disallowed cross-component dependency: shop -> api"),
        "the claimed-source pair line ships byte-identically:\n{out}"
    );
    assert!(
        !out.contains("unassigned unit"),
        "a unit claimed by `units` never acquires the unassigned finding:\n{out}"
    );
}

/// Cell 3 — the D1 go pin (mapping.rs `go_unit_claims`, byte-identical): on
/// go a unit IS an addressable module path, so a `matches.modules` pattern
/// naming the import path CLAIMS the unit — the tree verifies clean exactly
/// as before, with no `unexpected component` and no `unassigned unit` line.
#[test]
fn issue50_go_name_claimed_units_stay_clean_d1_pin() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\n\ngo 1.22\n");
    fixture.write("auth/auth.go", "package auth\n\nfunc Check() {}\n");
    fixture.write(
        "billing/billing.go",
        "package billing\n\nimport \"example.com/demo/auth\"\n\nfunc Bill() { auth.Check() }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"auth\"\nmatches = { modules = [\"example.com/demo/auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { modules = [\"example.com/demo/billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n",
    );
    let output = fixture.run(&["verify"]);
    let out = stdout(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the go D1 name claim must land the units clean (stderr: {})",
        stderr(&output)
    );
    assert!(
        out.contains("ok: architecture.spec.toml matches source model (2 modules, 0 constraints checked)"),
        "the go tree verifies byte-identically:\n{out}"
    );
    assert!(
        !out.contains("unexpected component") && !out.contains("unassigned unit"),
        "a go unit claimed by name through the D1 sub-arm is claimed, never soft-reclaimed:\n{out}"
    );
}

/// Cell 4 — the rust pin: a crate reclaimed through the module-path sub-arm
/// arrives from `unexpected_components` (rust unit names carry no namespace
/// parent), so the soft-reclaim mark never arms there — the shipped behavior
/// stands: the reclaim assigns the crate, its pair still adjudicates against
/// the declared `depend_on`, and the run passes.
#[test]
fn issue50_rust_module_path_reclaim_stays_clean() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write(
        "crates/billing/src/lib.rs",
        "mod util;\npub fn bill() { auth::auth(); }\n",
    );
    fixture.write("crates/billing/src/util.rs", "pub fn helper() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { modules = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n",
    );
    let output = fixture.run(&["verify"]);
    let out = stdout(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the rust reclaim path must not change verdicts (stderr: {})",
        stderr(&output)
    );
    assert!(
        out.contains("ok: architecture.spec.toml matches source model (2 modules, 0 constraints checked)"),
        "the rust tree verifies byte-identically:\n{out}"
    );
    assert!(
        !out.contains("unexpected component") && !out.contains("unassigned unit"),
        "a rust crate reclaimed from `unexpected_components` keeps the shipped assignment:\n{out}"
    );
}
