//! GitHub issue #61: dotted module names must render whole in the depgraph
//! modules projection.
//!
//! On the reference consumer tree a dotted-name module (`Webhooks.API`) lost
//! its name entirely: the fold collapsed the name to its last segment (`API`),
//! the node carried the identity's solid and async edges under a generic
//! label, and a same-suffix sibling could merge into the collision node. The
//! render layer now folds dotted module names identity-preservingly: the node
//! carries the declared name verbatim (quoted by the shared render
//! primitives where the target grammar needs it), and where two names'
//! truncated forms collide, one stderr census row names both full names
//! (extending the issue #48 row shape to module names).
//!
//! Cells:
//!   1. the #61 repro: the full dotted name renders, no truncation, and the
//!      async endpoint places on the named node (issue Expected arm (a));
//!   2. two modules with the same last component render distinctly and the
//!      collision census row names both;
//!   3. single-word modules render byte-identical to the committed golden
//!      (the non-declaring/unchanged law);
//!   4. repeated runs are cmp-clean.

mod common;
#[allow(dead_code)]
mod shared;

use common::{stderr, stdout, Fixture};
use shared::driver::{Driver, Language};

fn csproj(references: &[&str]) -> String {
    let mut text =
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0\
         </TargetFramework>\n  </PropertyGroup>\n"
            .to_string();
    if !references.is_empty() {
        text.push_str("  <ItemGroup>\n");
        for reference in references {
            text.push_str(&format!(
                "    <ProjectReference Include=\"..\\{reference}\\{reference}.csproj\" />\n"
            ));
        }
        text.push_str("  </ItemGroup>\n");
    }
    text.push_str("</Project>\n");
    text
}

fn event_bus(fixture: &Fixture) {
    fixture.write("EventBus/EventBus.csproj", &csproj(&[]));
    fixture.write(
        "EventBus/Bus.cs",
        "namespace EventBus;\npublic class Bus\n{\n    public void Send() {}\n}\n",
    );
}

/// The #61 repro shape: the dotted-name project `Webhooks.API` referencing
/// the single-word projects `EventBus` and `Catalog` — the reference tree's
/// `Webhooks.API` whose every edge landed on a generic `API` node. A
/// declared `[[async_depends]]` entry rides the same endpoint (issue
/// Expected arm (a): a node whose id or label carries the declared name).
fn repro_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[async_depends]]\nfrom = \"Webhooks.API\"\nto = \
         \"EventBus\"\nevent = \"Webhooks.API::Events::WebhookReceived\"\n",
    );
    fixture.write("Webhooks.API/Webhooks.API.csproj", &csproj(&["EventBus"]));
    fixture.write(
        "Webhooks.API/Controllers.cs",
        "using EventBus;\nnamespace Webhooks.API;\npublic class WebhookReceiver\n{\n    \
         public void Receive() {}\n}\n",
    );
    event_bus(&fixture);
    fixture
}

#[test]
fn dotted_module_name_renders_full_not_truncated() {
    let fixture = repro_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("[\"Webhooks.API"),
        "the declared dotted name must render verbatim in the node label:\n{out}"
    );
    assert!(
        !out.lines().any(|line| line.trim() == "API" || line.trim().starts_with("API[")),
        "the truncated collision node must not exist:\n{out}"
    );
    assert!(
        out.contains("Webhooks.API::WebhookReceived"),
        "the async arrow label carries the publishing name:\n{out}"
    );
    let arrow = out
        .lines()
        .find(|line| line.contains(" -.->|Webhooks.API::WebhookReceived| "))
        .unwrap_or_default();
    assert!(
        arrow.contains("Webhooks_API"),
        "the async from-endpoint places on the renamed node (issue Expected \
         arm (a)):\n{arrow}"
    );
    // The solid edge lands on the named node too — no `API` node anywhere.
    assert!(
        !out.contains("--> API\n") && !out.contains(" API --> "),
        "no arrow may reference a truncated `API` node:\n{out}"
    );
}

#[test]
fn same_suffix_collision_renders_both_and_names_both() {
    let fixture = Fixture::new();
    for name in ["Webhooks.API", "Internal.API"] {
        fixture.write(&format!("{name}/{name}.csproj"), &csproj(&["EventBus"]));
        fixture.write(
            &format!("{name}/Wiring.cs"),
            &format!("using EventBus;\nnamespace {name};\npublic class Sink {{ }}\n"),
        );
    }
    event_bus(&fixture);
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("[\"Internal.API\"]") && out.contains("[\"Webhooks.API\"]"),
        "both modules must render distinctly by full name:\n{out}"
    );
    assert!(
        !out.lines().any(|line| line.trim() == "API"),
        "the collision node must not exist:\n{out}"
    );
    let err = stderr(&output);
    assert!(
        err.contains(
            "note: module name collision: 2 module names truncate to \"API\" — rendered \
             by full name: \"Internal.API\", \"Webhooks.API\""
        ),
        "the collision census row must name both full names:\n{err}"
    );
}

/// The non-declaring/unchanged law: a single-word module tree renders its
/// committed golden byte-for-byte and prints no census row.
#[test]
fn single_word_modules_keep_golden_bytes() {
    let fixture = Fixture::new();
    let driver = Driver {
        language: Language::Csharp,
    };
    driver.materialize(&fixture, &driver.probe_tree());
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let golden = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens/depgraph-modules/csharp.mmd"))
    .expect("csharp depgraph golden");
    assert_eq!(
        stdout(&output).as_bytes(),
        golden.as_slice(),
        "a single-word tree must keep its exact golden bytes"
    );
    assert!(
        !stderr(&output).contains("module name collision"),
        "a single-word tier earns no census row: {}",
        stderr(&output)
    );
}

/// Determinism: repeated runs on the colliding tree are cmp-clean on stdout
/// and on the census rows.
#[test]
fn collision_render_is_deterministic_across_runs() {
    let fixture = same_suffix_tree_fixture();
    let first = fixture.run(&["depgraph", "modules"]);
    let second = fixture.run(&["depgraph", "modules"]);
    assert_eq!(stdout(&first), stdout(&second), "stdout bytes must repeat");
    assert_eq!(
        stderr(&first).lines().filter(|line| line.starts_with("note:"))
            .collect::<Vec<_>>(),
        stderr(&second).lines().filter(|line| line.starts_with("note:")).collect::<Vec<_>>(),
        "the census rows must repeat"
    );
}

fn same_suffix_tree_fixture() -> Fixture {
    let fixture = Fixture::new();
    for name in ["Webhooks.API", "Internal.API"] {
        fixture.write(&format!("{name}/{name}.csproj"), &csproj(&["EventBus"]));
        fixture.write(
            &format!("{name}/Wiring.cs"),
            &format!("using EventBus;\nnamespace {name};\npublic class Sink {{ }}\n"),
        );
    }
    event_bus(&fixture);
    fixture
}
