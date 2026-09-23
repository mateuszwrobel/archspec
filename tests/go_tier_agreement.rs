//! go module-tier prose coherence (fanout-output US 03): one binary surface
//! owns the sentence that go derives the module tier from the tree's own
//! packages, every other surface points at it or at the capability row, and
//! the `depgraph --help` refusal paragraph reads as quoted error copy, not a
//! prophecy about a healthy tree. Registry claims: `go-module-tier` (owner:
//! the `help languages` go section) and `depgraph-refusal` (the refusal bytes
//! as registered here must equal the live bytes of a tier-less tree and the
//! quoted copy inside `depgraph --help`).

mod common;

use common::{stderr, stdout, Fixture};
use std::fs;

/// Registry claim `go-module-tier`: the canonical derivation phrase. It may
/// appear in exactly one rendered surface — `help languages` — and nowhere
/// else (pointer surfaces name `archspec capability matrix` instead).
const CANONICAL_GO_TIER_PHRASE: &str = "derived from the tree's own packages";

/// Registry claim `depgraph-refusal`: the refusal bytes the command emits on a
/// tier-less tree, unchanged, quoted as error copy in `depgraph --help`.
const DEPGRAPH_REFUSAL: &str = "depgraph needs the module tier, which this model has none of; \
the module tier is derived from package references, which this tree records none of";

fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn help_surface(args: &[&str]) -> String {
    let fixture = Fixture::new();
    let output = fixture.run(args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`archspec {}` must exit 0: {}",
        args.join(" "),
        stderr(&output)
    );
    stdout(&output)
}

/// Every rendered surface except `help languages` (the registered owner).
fn other_surfaces() -> Vec<(String, String)> {
    let mut surfaces = vec![("help".to_string(), help_surface(&["help"]))];
    for topic in ["commands", "glob", "spec", "constraints", "languages", "roles", "workflow", "diagnostics"] {
        surfaces.push((format!("help {topic}"), help_surface(&["help", topic])));
    }
    for command in ["init", "scan", "diagram", "verify", "update", "report", "doctor", "inspect", "depgraph", "skill"] {
        surfaces.push((format!("help {command}"), help_surface(&["help", command])));
    }
    surfaces.push(("skill".to_string(), help_surface(&["skill"])));
    surfaces.retain(|(name, _)| name != "help languages");
    surfaces.push(("depgraph --help".to_string(), help_surface(&["depgraph", "--help"])));
    surfaces
}

/// Scenario: the canonical sentence is owned by `help languages` and no
/// rendered surface re-derives or rephrases it (and none resurrects the stale
/// "single-module go" example).
#[test]
fn go_tier_sentence_is_owned_by_the_languages_topic() {
    let languages = help_surface(&["help", "languages"]);
    let go_block = languages
        .split("\ngo:")
        .nth(1)
        .and_then(|after_go| after_go.split("\ncsharp:").next().map(str::to_owned))
        .expect("languages must have a go section boundary");
    let flat = flatten(&go_block);
    assert!(
        flat.contains(CANONICAL_GO_TIER_PHRASE),
        "the go section must own the canonical derivation sentence:\n{flat}"
    );
    assert!(
        flat.contains("archspec capability matrix"),
        "the go section must point at the capability row that reports the fact:\n{flat}"
    );

    for (name, surface) in other_surfaces() {
        let flat = flatten(&surface);
        assert!(
            !flat.contains("the tree's own packages") && !flat.contains("from its own packages"),
            "`{name}` re-states the go-tier derivation sentence owned by `help languages`"
        );
        assert!(
            !flat.contains("single-module go"),
            "`{name}` cites the stale single-module go example of a tier-less driver"
        );
    }
}

/// Scenario: constraints stops naming a stale example — the spot where the
/// example lived now points at the capability matrix.
#[test]
fn constraints_points_at_the_capability_matrix() {
    let surface = flatten(&help_surface(&["help", "constraints"]));
    assert!(
        surface.contains("archspec capability matrix"),
        "help constraints must point at `archspec capability matrix` for per-language tier facts:\n{surface}"
    );
}

/// Payload markdown surfaces, as doc_surface_nits collects them.
fn payload_docs() -> Vec<(String, String)> {
    let mut files = vec![concat!(env!("CARGO_MANIFEST_DIR"), "/README.md").to_string()];
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/docs");
    let mut stack = vec![root.to_string()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read docs dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path.to_string_lossy().into_owned());
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                files.push(path.to_string_lossy().into_owned());
            }
        }
    }
    files
        .into_iter()
        .map(|path| {
            (
                path.replace(env!("CARGO_MANIFEST_DIR"), ""),
                fs::read_to_string(&path).expect("read payload doc"),
            )
        })
        .collect()
}

/// Sentences that falsify the go-tier agreement on a flat multi-package tree:
/// any claim that such a tree's module tier is absent or "tier-free" — the
/// live model derives the tier from package references.
const FORBIDDEN_DOC_CLAIMS: &[&str] = &[
    "stays tier-free",
    "model carries none",
    "carries no tier",
    "no native module tier this run",
    "plain go tree",
];

/// Scenario: no payload doc states that a flat go tree's module tier or
/// module_edges are empty — such a sentence anywhere fails here, named.
#[test]
fn docs_never_claim_an_empty_go_tier() {
    for (path, text) in payload_docs() {
        for (lineno, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            // Emptiness claims are falsifiable only when tied to go; and the
            // refusal bytes quoted in docs legitimately say "records none".
            let go_tied = lower.contains("go ");
            if lower.contains("module_edges") && lower.contains("empty") && go_tied {
                panic!("{path}:{} claims empty module_edges for a go tree: {line}", lineno + 1);
            }
            for needle in FORBIDDEN_DOC_CLAIMS {
                if lower.contains(needle) && (*needle == "no native module tier this run" || go_tied) {
                    panic!("{path}:{} states the forbidden claim {needle:?}: {line}", lineno + 1);
                }
            }
        }
    }
}

/// Scenario: capability's argument shape is stated where capability is
/// mentioned — every command-shaped mention names its subcommand.
#[test]
fn capability_mentions_name_their_subcommand() {
    let mut surfaces = other_surfaces();
    surfaces.retain(|(name, _)| !name.starts_with("help capability") && name != "help commands");
    let mut checked = surfaces;
    for doc in payload_docs() {
        // capability's own docs define both subcommand forms verbatim.
        if !doc.0.contains("/commands/capability/") {
            checked.push(doc);
        }
    }
    for (name, text) in &checked {
        let flat = flatten(text);
        let mut cursor = 0;
        while let Some(at) = flat[cursor..].find("archspec capability") {
            let after = &flat[cursor + at + "archspec capability".len()..];
            assert!(
                after.starts_with(" matrix") || after.starts_with(" granular"),
                "`{name}` mentions the capability command without its subcommand form: …{}",
                &after[..after.len().min(48)]
            );
            cursor += at + "archspec capability".len();
        }
    }
    // At least one payload doc must demonstrate the matrix form (this guard
    // must not pass by nothing mentioning capability at all).
    assert!(
        checked.iter().any(|(_, t)| t.contains("archspec capability matrix")),
        "no payload doc or surface states the `archspec capability matrix` form"
    );
}

/// Scenario: a flat go tree's module_edges agree with the prose — live.
#[test]
fn flat_go_tree_module_edges_are_populated() {
    let fx = Fixture::new();
    fx.write("go.mod", "module example.com/flat\n\ngo 1.21\n");
    fx.write("app/app.go", "package app\n\nimport \"example.com/flat/util\"\n\nfunc Use() { util.X() }\n");
    fx.write("util/util.go", "package util\n\nconst X = 1\n");

    let output = fx.run(&["scan"]);
    assert_eq!(output.status.code(), Some(0), "scan failed: {}", stderr(&output));
    let model: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("scan JSON");
    let edges = model["module_edges"].as_array().expect("module_edges array");
    assert!(
        !edges.is_empty(),
        "a flat single-`go.mod` tree with cross-package imports must carry module_edges derived \
         from the tree's own packages — the prose says so; the model says otherwise:\n{model:#}\n"
    );

    let view = fx.run(&["depgraph", "modules"]);
    assert_eq!(
        view.status.code(),
        Some(0),
        "depgraph renders the tier the scan derived: {}",
        stderr(&view)
    );
}

/// Scenario: depgraph help refuses, not prophesies — the paragraph frames the
/// refusal as quoted error copy, and the quoted bytes equal the live bytes of
/// a tier-less tree (registry claim `depgraph-refusal`).
#[test]
fn depgraph_help_quotes_the_refusal_instead_of_prophesying() {
    let surface = flatten(&help_surface(&["depgraph", "--help"]));
    assert!(
        surface.contains(&format!("no module tier the command refuses with: {DEPGRAPH_REFUSAL}")),
        "the refusal sentence may appear only as quoted error copy introduced as the error the \
         command emits: …{surface}"
    );
    assert!(
        !surface.contains("is refused:"),
        "the paragraph must not read as a standing conditional status:\n{surface}"
    );

    // Live leg: a tree with no module tier emits exactly the registered bytes.
    let fx = Fixture::new();
    fx.write("go.mod", "module example.com/solo\n\ngo 1.21\n");
    fx.write("pkg/solo.go", "package pkg\n\nconst X = 1\n");
    let scan = fx.run(&["scan"]);
    assert_eq!(scan.status.code(), Some(0), "scan failed: {}", stderr(&scan));
    let model: serde_json::Value = serde_json::from_str(&stdout(&scan)).expect("scan JSON");
    assert!(
        model["module_edges"].as_array().map(|a| a.is_empty()).unwrap_or(false),
        "the fixture must be tier-less for the refusal leg:\n{model:#}\n"
    );
    let refused = fx.run(&["depgraph", "modules"]);
    assert_eq!(refused.status.code(), Some(1), "a tier-less tree must refuse");
    assert!(
        stderr(&refused).contains(DEPGRAPH_REFUSAL),
        "live refusal bytes drifted from the registered claim: {}",
        stderr(&refused)
    );
}

/// The stale-example framing must be gone from doc sentences too: the phrase
/// "single-module go" may not appear tied to a tier-less claim on the same
/// line (the bare phrase is fine when it names the tree, not its tier).
#[test]
fn docs_do_not_name_single_module_go_as_tier_less() {
    for (path, text) in payload_docs() {
        for line in text.lines() {
            let lower = line.to_lowercase();
            let tierless = lower.contains("no module tier")
                || lower.contains("without a module tier")
                || lower.contains("tier-free")
                || lower.contains("carries no")
                || lower.contains("empty");
            // A line that states the derivation keeps its balance: naming the
            // tree while pointing at the package-reference rule is coherent.
            let derives = lower.contains("derived from") || lower.contains("derives");
            assert!(
                !(lower.contains("single-module go") && tierless && !derives),
                "{path} names single-module go as a tier-less driver: {line}"
            );
        }
    }
}
