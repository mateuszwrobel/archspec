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

/// GitHub issue #2, enforcement half: the leaking declaration is a
/// SEMICOLON-BODIED positional record — the verbatim repro the reporter ran
/// against the released binary. The record emits its fact (grammar gives the
/// record family a `;` body), so strict verification names the leak and
/// fails; body shape never decides detection.
#[test]
fn issue_two_semicolon_bodied_record_leaks_under_strict_verify() {
    let fixture = common::Fixture::new();
    fixture.write("Sample/Sample.csproj", &csproj("net8.0"));
    fixture.write(
        "Sample/Contracts.cs",
        "namespace Sample.Contracts;\npublic sealed record BraceBodiedContract(string Id) { }\n",
    );
    fixture.write(
        "Sample/Implementation.cs",
        "namespace Sample.Implementation;\npublic sealed record SemicolonBodiedLeak(string Id);\n",
    );
    fixture.write("architecture.spec.toml", &repro_spec(&["Sample::Contracts"]));
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the semicolon-bodied record leak must fail --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains(
            "public api leak: Sample::Implementation exposes SemicolonBodiedLeak (not allowlisted)"
        ),
        "the report names the semicolon-bodied record leak verbatim:\n{out}"
    );
    assert!(
        !out.contains("Sample::Contracts exposes"),
        "the brace-bodied record in the allowlisted module stays silent:\n{out}"
    );
    assert!(
        !out.contains("vacuous"),
        "the constraint is engaged on a fact-carrying module — vacuity must \
         not stand in for the leak:\n{out}"
    );
}

/// The same tree with BOTH record modules allowlisted: the semicolon body
/// is nothing the allowlist treats differently — a clean pass, no findings.
#[test]
fn issue_two_records_allowlisted_pass_silently() {
    let fixture = common::Fixture::new();
    fixture.write("Sample/Sample.csproj", &csproj("net8.0"));
    fixture.write(
        "Sample/Contracts.cs",
        "namespace Sample.Contracts;\npublic sealed record BraceBodiedContract(string Id) { }\n",
    );
    fixture.write(
        "Sample/Implementation.cs",
        "namespace Sample.Implementation;\npublic sealed record SemicolonBodiedLeak(string Id);\n",
    );
    fixture.write(
        "architecture.spec.toml",
        &repro_spec(&["Sample::Contracts", "Sample::Implementation"]),
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "both record body shapes allowlisted must pass (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "confirmation on stdout:\n{out}");
    assert!(
        !out.contains("public api leak") && !out.contains("vacuous"),
        "an engaged satisfied allowlist over both shapes adds nothing:\n{out}"
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

// ---------------------------------------------------------------------------
// misuse diagnostics US 01: the did-you-mean nudge on the vacuous line
// ---------------------------------------------------------------------------

/// The GitHub issue #1 spec shape verbatim: modules declared under BARE
/// NAMES whose `matches.modules` address the real namespace paths — the
/// misread was putting those bare names into `allowed`.
fn bare_name_spec(allowed: &[&str]) -> String {
    let list: Vec<String> = allowed.iter().map(|p| format!("\"{p}\"")).collect();
    format!(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"contracts\"\nmatches = {{ units = [\"Sample\"], modules = [\"Sample::Contracts\"] }}\n\n[[module]]\nname = \"implementation\"\nmatches = {{ units = [\"Sample\"], modules = [\"Sample::Implementation\"] }}\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [{}]\n",
        list.join(", ")
    )
}

#[test]
fn bare_module_name_entry_nudges_the_module_path_it_meant() {
    // The verbatim issue #1 misread: `allowed = ["contracts"]` addresses no
    // module path while a module DECLARED under that name carries public
    // types under `Sample::Contracts` — the vacuity stops being a dead end.
    let fixture = issue_repro_tree();
    fixture.write("architecture.spec.toml", &bare_name_spec(&["contracts"]));
    let output = fixture.run(&["verify", "--strict"]);
    assert_eq!(output.status.code(), Some(1), "leaks still fail (stderr: {})", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("public api leak: Sample::Contracts exposes PublicContract (not allowlisted)")
            && out
                .contains("public api leak: Sample::Implementation exposes LeakedImplementation (not allowlisted)"),
        "the leak lines are byte-unchanged by the nudge:\n{out}"
    );
    assert!(
        out.contains("'allowed' pattern \"contracts\" matches no module with public types in the model; allowed entries are module paths — did you mean \"Sample::Contracts\"?"),
        "the vacuity names one concrete corrected module path:\n{out}"
    );
}

#[test]
fn bare_path_tail_entry_nudges_by_segment_equality() {
    // The case-exact dropped-namespace spelling: `Contracts` equals the last
    // `::`-separated segment of the fact-carrying path.
    let fixture = issue_repro_tree();
    fixture.write("architecture.spec.toml", &repro_spec(&["Contracts"]));
    let out = stdout(&fixture.run(&["verify", "--strict"]));
    assert!(
        out.contains("'allowed' pattern \"Contracts\" matches no module with public types in the model; allowed entries are module paths — did you mean \"Sample::Contracts\"?"),
        "segment equality names the path:\n{out}"
    );
}

#[test]
fn two_candidates_are_named_sorted_with_or() {
    let fixture = common::Fixture::new();
    fixture.write("Sample/Sample.csproj", &csproj("net8.0"));
    fixture.write(
        "Sample/East.cs",
        "namespace Sample.East.Core;\npublic sealed class EastWidget { }\n",
    );
    fixture.write(
        "Sample/North.cs",
        "namespace Sample.North.Core;\npublic sealed class NorthWidget { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Sample\"\nmatches = { units = [\"Sample\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"Core\"]\n",
    );
    let out = stdout(&fixture.run(&["verify", "--strict"]));
    assert!(
        out.contains("did you mean \"Sample::East::Core\" or \"Sample::North::Core\"?"),
        "two candidates, sorted, joined with or:\n{out}"
    );
}

#[test]
fn more_than_three_candidates_are_capped_with_a_count() {
    let fixture = common::Fixture::new();
    fixture.write("Sample/Sample.csproj", &csproj("net8.0"));
    for (dir, ns, name) in [
        ("West", "Sample.West.Core", "WestWidget"),
        ("East", "Sample.East.Core", "EastWidget"),
        ("South", "Sample.South.Core", "SouthWidget"),
        ("North", "Sample.North.Core", "NorthWidget"),
    ] {
        fixture.write(
            &format!("Sample/{dir}.cs"),
            &format!("namespace {ns};\npublic sealed class {name} {{ }}\n"),
        );
    }
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Sample\"\nmatches = { units = [\"Sample\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"Core\"]\n",
    );
    let out = stdout(&fixture.run(&["verify", "--strict"]));
    assert!(
        out.contains("did you mean \"Sample::East::Core\", \"Sample::North::Core\", \"Sample::South::Core\"...and 1 more module path?"),
        "the cap names the first three sorted, then the singular remainder:\n{out}"
    );
    assert!(
        !out.contains("\"Sample::West::Core\""),
        "the capped candidate is not named:\n{out}"
    );
}

#[test]
fn no_candidate_leaves_the_vacuity_line_byte_unchanged() {
    // No fuzzy guessing: without an exact structural candidate the sentence
    // is today's bytes and the nudge bytes never render.
    for entry in ["Sample::Nowhere", "Nowhere::At.All", "nowhere", "Implementation"] {
        let fixture = issue_repro_tree();
        fixture.write("architecture.spec.toml", &repro_spec(&[entry]));
        let out = stdout(&fixture.run(&["verify", "--strict"]));
        if entry == "Implementation" {
            // a bare path-tail candidate EXISTS (Sample::Implementation) —
            // this entry must nudge, proving the rule is not entry-blind.
            assert!(
                out.contains("; allowed entries are module paths — did you mean \"Sample::Implementation\"?"),
                "the existing candidate must fire:\n{out}"
            );
            continue;
        }
        assert!(
            out.contains(format!("'allowed' pattern \"{entry}\" matches no module with public types in the model").as_str()),
            "the sentence keeps its exact bytes for {entry}:\n{out}"
        );
        assert!(
            !out.contains("allowed entries are module paths"),
            "no suffix bytes for {entry}:\n{out}"
        );
    }
}

#[test]
fn rust_vacuity_sentence_is_never_nudged() {
    // Rust allowed entries are bare names legitimately (crate-root exports
    // carry no `::`) — the nudge consults only the module-tier fact map the
    // rust driver never fills, so the rust sentence stays byte-identical.
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("src/lib.rs", "mod internal;\n\nfn local() {}\n");
    fixture.write("src/internal.rs", "pub struct Hidden;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"internal\"]\n",
    );
    let out = stdout(&fixture.run(&["verify"]));
    assert!(
        out.contains("public_api_allowlist: no crate-root public exports to check"),
        "the rust sentence is byte-identical:\n{out}"
    );
    assert!(
        !out.contains("did you mean") && !out.contains("allowed entries are module paths"),
        "no nudge bytes on a rust run:\n{out}"
    );
}

#[test]
fn rust_leak_lines_carry_no_nudge_bytes() {
    // The rust leak shape (crate-root exports exist, entry matches none):
    // findings byte-unchanged, no advisory text anywhere in the report.
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("src/lib.rs", "pub mod api;\n");
    fixture.write("src/api.rs", "pub struct Surface;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"nothing\"]\n",
    );
    let out = stdout(&fixture.run(&["verify"]));
    assert!(
        out.contains("demo exposes api (not allowlisted)"),
        "the rust leak line is byte-unchanged:\n{out}"
    );
    assert!(
        !out.contains("did you mean"),
        "rust leaks are stated without advice:\n{out}"
    );
}
