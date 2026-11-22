//! GitHub issue #59 — the silent-claim-deletion cure.
//!
//! Deleting the only `[[module]]` claim that named a C# project used to pass
//! `verify --strict` with exit 0: the project's unit fell into
//! `unexpected_components`, another claim's `matches.modules` territory
//! (the ancestor rule) covered its soft paths, and `augment_with_modules`
//! silently re-assigned it — no `unexpected component`, no `unassigned
//! unit`, no diagnostic at all. The fix extends the honest-reclaim law
//! (issue #50) to the `unexpected_components` vector: a unit reclaimed
//! through the soft-path sub-arm alone KEEPs its `unexpected component:`
//! finding beside the reclaim, at the SAME verdict tier the async-endpoint
//! arm speaks at (`Diff::error_fields` — non-zero exit without `--strict`).
//! The name-claim sub-arms stay silent byte-identically: the go D1 arm at
//! any depth, and on other languages a `modules` pattern addressing the
//! unit name itself (the rust shape fix50 cell 4 pins).
//!
//! Cells (per the fix plan):
//! 1. delete-the-claim fixture: `unexpected component:` speaks, the
//!    mixed-tier pair line is ABSENT, and the exit is non-zero WITHOUT
//!    `--strict` — the same verdict tier the `async endpoint unclaimed`
//!    arm fires at;
//! 2. claim-present tree: clean pass, no new line (byte-identity);
//! 3. go tree pin: a go unit claimed through the D1 sub-arm at any depth
//!    stays a full claim — clean, byte-identical;
//! 4. rust tree pin: a crate named by a `modules` pattern is a direct
//!    claim, not a territory absorb — clean, byte-identical;
//! 5. deletion + reclaim combo: a unit covered ONLY by a namespace-tier
//!    modules claim keeps the fix50 law — the `unassigned unit:` finding,
//!    never the `unexpected component:` one.

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

/// The issue's repro shape (the witness-corpus Webhooks deletion, minimized): the
/// project `Acme.Widget` is named by NO claim once its entry is deleted —
/// only the wiring claim's `matches.modules` territory `Acme` covers its
/// soft paths through the ancestor rule — and it uses `Acme.Core`, a
/// project claimed by `units` under its own boundary.
fn widget_fixture(spec: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Acme.Widget/Acme.Widget.csproj",
        &csproj_with_reference("net8.0", r"..\Acme.Core\Acme.Core.csproj"),
    );
    fixture.write(
        "Acme.Widget/Widget.cs",
        "using Acme.Core;\nnamespace Acme.Widget;\npublic class WidgetService { }\n",
    );
    fixture.write("Acme.Core/Acme.Core.csproj", &csproj("net8.0"));
    fixture.write(
        "Acme.Core/Core.cs",
        "namespace Acme.Core;\npublic class CoreService { }\n",
    );
    fixture.write("architecture.spec.toml", spec);
    fixture
}

/// The deleted-claim spec: `Acme.Widget` names no entry anymore; the wiring
/// claim's territory `Acme` still covers the project's paths (the witness-corpus
/// `wiring::root` `modules = [<namespace root>]` shape).
const DELETED: &str = "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"core\"\nmatches = { units = [\"Acme.Core\"], modules = [\"Acme::Core\"] }\n\n[[module]]\nname = \"wiring\"\nmatches = { modules = [\"Acme\"] }\nallowed = { depend_on = [\"core\"] }\n";

/// The claim-present spec: `Acme.Widget` is named by its own `units` claim;
/// the wiring claim keeps its territory (no unit depends on the absorb).
const CLAIMED: &str = "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"core\"\nmatches = { units = [\"Acme.Core\"], modules = [\"Acme::Core\"] }\n\n[[module]]\nname = \"widget\"\nmatches = { units = [\"Acme.Widget\"], modules = [\"Acme::Widget\"] }\nallowed = { depend_on = [\"core\"] }\n\n[[module]]\nname = \"wiring\"\nmatches = { modules = [\"Acme\"] }\n";

/// Cell 1 — the deletion speaks at the error tier: `unexpected component:`
/// present, exit non-zero WITHOUT `--strict` (the `unexpected component`
/// field is error-tier in `Diff::error_fields` — the same verdict tier the
/// `async endpoint unclaimed` arm fires at, so a claim deletion and an
/// async-endpoint deletion move the verdict identically), and the
/// mixed-tier pair line stays absent (the soft-reclaimed source is not
/// adjudicated at the unit tier, the fix50 law).
#[test]
fn issue59_deleted_claim_absorbed_by_territory_speaks_error_tier() {
    let fixture = widget_fixture(DELETED);
    let output = fixture.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "the unclaimed-component finding is error-tier, strict not required \
         (parity with the async-endpoint arm) (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("unexpected component: Acme.Widget"),
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
    // Strict parity: `--strict` keeps the same verdict and the same line.
    let strict = fixture.run(&["verify", "--strict"]);
    assert_ne!(
        strict.status.code(),
        Some(0),
        "the finding stays error-tier under --strict (stderr: {})",
        stderr(&strict)
    );
    assert!(
        stdout(&strict).contains("unexpected component: Acme.Widget"),
        "the same line ships under --strict:\n{}",
        stdout(&strict)
    );
}

/// Cell 2 — claim-present byte-identity: with `units` naming every project,
/// nothing reaches the reclaim arm — the run passes and no
/// `unexpected component` line appears.
#[test]
fn issue59_claim_present_tree_stays_clean() {
    let fixture = widget_fixture(CLAIMED);
    let output = fixture.run(&["verify", "--strict"]);
    let out = stdout(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the claimed tree verifies clean (stderr: {})",
        stderr(&output)
    );
    assert!(
        !out.contains("unexpected component") && !out.contains("unassigned unit"),
        "a unit claimed by `units` never acquires either finding:\n{out}"
    );
}

/// Cell 3 — the D1 go pin: a go unit claimed through the `matches.modules`
/// name-claim sub-arm at any depth is a full claim — the tree verifies
/// clean with no finding lines, byte-identically to before the fix.
#[test]
fn issue59_go_territory_claims_stay_clean_d1_pin() {
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
        !out.contains("unexpected component") && !out.contains("unassigned unit"),
        "a go unit claimed through the D1 sub-arm is claimed, never soft-reclaimed:\n{out}"
    );
}

/// Cell 4 — the rust pin: a crate named by a `modules` pattern addresses
/// the unit NAME (one `::`-free segment), so it is a direct claim, not a
/// territory absorb — the reclaim stays silent and the pair still
/// adjudicates, byte-identically to the shipped shape (fix50 cell 4).
#[test]
fn issue59_rust_name_claimed_module_path_stays_clean() {
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
        "the rust name claim must not change verdicts (stderr: {})",
        stderr(&output)
    );
    assert!(
        !out.contains("unexpected component") && !out.contains("unassigned unit"),
        "a rust crate named by a `modules` pattern is claimed, never absorbed:\n{out}"
    );
}

/// Cell 5 — deletion + reclaim combo, the fix50 law intact: a unit covered
/// ONLY by a namespace-tier modules claim through its NAMESPACE parent
/// keeps the `unassigned unit:` finding — never the `unexpected component:`
/// one (the two vectors stay distinct voices).
#[test]
fn issue59_namespace_reclaimed_unit_keeps_the_fix50_voice() {
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
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Acme::Shop\"\nmatches = { units = [\"Acme\"], modules = [\"Acme::Shop\"] }\n\n[[module]]\nname = \"Acme.Shop.Api\"\nmatches = { units = [\"Acme.Shop.Api\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "the unassigned-unit finding is error-tier (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("unassigned unit: Acme.Shop"),
        "fix50's voice still speaks for the namespace vector:\n{out}"
    );
    assert!(
        !out.contains("unexpected component"),
        "the namespace vector never speaks the component voice:\n{out}"
    );
}
