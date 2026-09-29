//! C# public API enforcement (csharp public-api plan, US 02): on a csharp
//! tree, `public_api_allowlist.allowed` names the modules allowed to expose;
//! every public type of a module outside the list is a `public api leak`
//! naming module and type. Rust trees keep the crate-root semantics
//! byte-unchanged (their tests live in `verify_constraints.rs`).

mod common;

use common::{stderr, stdout};

fn csproj(tf: &str) -> String {
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>{tf}</TargetFramework>\n  </PropertyGroup>\n</Project>\n"
    )
}

/// The issue repro tree (see `csharp_public_api.rs` for the weld rationale).
fn issue_repro_tree() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("Sample/Sample.csproj", &csproj("net8.0"));
    fixture.write(
        "Sample/Contracts.cs",
        "namespace Sample.Contracts;\npublic sealed class PublicContract { }\n",
    );
    fixture.write(
        "Sample/Implementation.cs",
        "namespace Sample.Implementation;\npublic sealed class LeakedImplementation { }\n",
    );
    fixture
}

/// Spec over the repro tree: the unit is accounted for, the two namespaces
/// are declared modules, and the allowlist names the modules allowed to
/// expose (the pattern grammar is the model-path glob of spec.md).
fn repro_spec(allowed: &[&str]) -> String {
    let list: Vec<String> = allowed.iter().map(|p| format!("\"{p}\"")).collect();
    format!(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Sample\"\nmatches = {{ units = [\"Sample\"] }}\n\n[[module]]\nname = \"Sample::Contracts\"\nmatches = {{ modules = [\"Sample::Contracts\"] }}\n\n[[module]]\nname = \"Sample::Implementation\"\nmatches = {{ modules = [\"Sample::Implementation\"] }}\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [{}]\n",
        list.join(", ")
    )
}

#[test]
fn issue_repro_verify_names_the_leak_and_fails() {
    let fixture = issue_repro_tree();
    fixture.write("architecture.spec.toml", &repro_spec(&["Sample::Contracts"]));
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the leak must fail --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("public api leak"),
        "report names the rule:\n{out}"
    );
    assert!(
        out.contains("Sample::Implementation exposes LeakedImplementation (not allowlisted)"),
        "report names module and type of the leak:\n{out}"
    );
    assert!(
        !out.contains("Sample::Contracts exposes"),
        "the allowlisted module stays silent:\n{out}"
    );
}

#[test]
fn allowed_modules_expose_in_silence() {
    let fixture = issue_repro_tree();
    fixture.write(
        "architecture.spec.toml",
        &repro_spec(&["Sample::Contracts", "Sample::Implementation"]),
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "every public type's module allowlisted must pass (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "confirmation on stdout:\n{out}");
    assert!(
        !out.contains("public api leak") && !out.contains("vacuous"),
        "an engaged satisfied allowlist adds nothing:\n{out}"
    );
}

#[test]
fn allowlist_pattern_addressing_no_fact_module_is_stated_vacuous() {
    let fixture = issue_repro_tree();
    fixture.write("architecture.spec.toml", &repro_spec(&["Nowhere::At.All"]));
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an allowlist gating nothing leaks everything and says so (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("public api leak: Sample::Contracts exposes PublicContract (not allowlisted)")
            && out
                .contains("public api leak: Sample::Implementation exposes LeakedImplementation (not allowlisted)"),
        "with no pattern matching, every public type leaks:\n{out}"
    );
    assert!(
        out.contains("'allowed' pattern \"Nowhere::At.All\" matches no module with public types in the model"),
        "the vacuity is stated with the pattern that gates nothing:\n{out}"
    );
}

/// The 3-segment corporate tree (`Acmecorp.Inventory.*`): the shape the
/// roles cells once lied about, probed here so the public-api rows cannot.
fn corporate_tree(leaky: bool) -> common::Fixture {
    let fixture = common::Fixture::new();
    for unit in ["Acmecorp.Inventory.Api", "Acmecorp.Inventory.Core"] {
        fixture.write(&format!("{unit}/{unit}.csproj"), &csproj("net8.0"));
    }
    fixture.write(
        "Acmecorp.Inventory.Api/Endpoints.cs",
        "namespace Acmecorp.Inventory.Api;\npublic sealed class Endpoints { }\n",
    );
    fixture.write(
        "Acmecorp.Inventory.Core/Stock.cs",
        &format!(
            "namespace Acmecorp.Inventory.Core;\npublic sealed class Stock {{ }}{}\n",
            if leaky {
                "\ninternal class Cache { }\nnamespace Acmecorp.Inventory.Core.Internal;\npublic sealed class BackDoor { }"
            } else {
                ""
            }
        ),
    );
    fixture
}

fn corporate_spec(allowed: &[&str]) -> String {
    let list: Vec<String> = allowed.iter().map(|p| format!("\"{p}\"")).collect();
    format!(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Acmecorp.Inventory.Api\"\nmatches = {{ units = [\"Acmecorp.Inventory.Api\"] }}\n\n[[module]]\nname = \"Acmecorp.Inventory.Core\"\nmatches = {{ units = [\"Acmecorp.Inventory.Core\"] }}\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [{}]\n",
        list.join(", ")
    )
}

#[test]
fn glob_allowed_covers_the_module_subtree() {
    let fixture = corporate_tree(false);
    fixture.write(
        "architecture.spec.toml",
        &corporate_spec(&["Acmecorp::Inventory::*"]),
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the prefix glob covers both modules (stderr: {})",
        stderr(&output)
    );
}

#[test]
fn unallowlisted_module_at_corporate_naming_leaks() {
    let fixture = corporate_tree(true);
    fixture.write(
        "architecture.spec.toml",
        &corporate_spec(&["Acmecorp::Inventory::Api", "Acmecorp::Inventory::Core"]),
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a module the tree adds under the declared pattern set must leak"
    );
    let out = stdout(&output);
    assert!(
        out.contains("Acmecorp::Inventory::Core::Internal exposes BackDoor (not allowlisted)"),
        "the leak at three-segment corporate naming is named:\n{out}"
    );
}
#[test]
fn csharp_tree_without_public_types_announces_no_public_api_facts() {
    // Engagement honesty (US 03): a C# tree whose sources declare no
    // explicitly public type cannot be gated by the allowlist — the csharp
    // driver says so in its own vocabulary (the rust sentence stays pinned
    // by the rust vacuity guards).
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj("net8.0"));
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\ninternal class Engine { }\nclass Quiet { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"App\"\nmatches = { units = [\"App\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"App::Core\"]\n",
    );
    let output = fixture.run(&["verify", "--strict"]);
    let out = stdout(&output);
    assert!(
        out.contains("vacuous constraint: [constraint #1] public_api_allowlist: no public API facts to check"),
        "the csharp emptiness is stated with the driver's own vocabulary:\n{out}"
    );
    assert!(
        !out.contains("no crate-root public exports to check"),
        "the rust sentence does not leak into csharp output:\n{out}"
    );
}
