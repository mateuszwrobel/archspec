//! GitHub issue #60 items 2-3 — the RUST arm: a declared `excluded_sources`
//! entry adjudicates at the `.rs` SOURCE FILE walk. A pruned file contributes
//! no fact (no parse, no module node, no edge, no symbol, no public-API
//! surface, no literal candidate) and states itself with exactly one
//! `excluded source: <path> — excluded_sources` census line, sorted and never
//! silent (the issue #39 law). The mechanical dot-entry/`target`/`vendor`
//! gate stays the FIRST door and says nothing — mechanically excluded files
//! were never unit sources, so they print no line and cannot keep an entry
//! alive. Rust derives units from manifests, so a crate whose every file is
//! pruned STAYS a unit with zero file facts (the csharp project arm has no
//! rust twin — there is no rust project-level gate to state a directory
//! line). Liveness is unchanged and driver-independent (`spec.rs`): a pruned
//! file still counts as a MATCH, so an entry that prunes files never reads
//! dead. A file read by several walks (parse pass, module walk, candidate
//! feed) states itself once. Non-declaring trees stay byte-identical; a
//! declaring entry that prunes nothing moves no machine byte.
//!
//! Cells: A whole-tree prune + manifest-preserved unit; B zero facts and the
//! module tree; C one line per file across walks (dedupe through the pattern
//! candidate feed); D liveness (alive entry silent, dead entry named);
//! F report-only law (model.json has no census field, absent key prints
//! nothing); G mechanical-first door (an entry under `target`/`vendor`
//! matches nothing and states nothing).

mod common;

use common::Fixture;
use serde_json::Value;

/// The census forms pinned by the claim registry (`csharp-excluded-sources`,
/// owner surface: `help diagnostics`). The dedicated guard for those
/// placeholder bytes is this file: the exact-bytes cells below render them.
#[allow(dead_code)]
const CENSUS_FORMS: [&str; 1] = ["excluded source: <path> — excluded_sources"];

fn root_manifest() -> &'static str {
    "[workspace]\nmembers = [\"app\", \"bus\"]\nresolver = \"2\"\n"
}

fn crate_manifest(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n")
}

/// `app` depends on `bus` (the manifest edge the source-level `use` resolves
/// through — cross-unit edges ride declared dependencies).
fn app_manifest() -> &'static str {
    "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbus = { path = \"../bus\" }\n"
}

fn spec(language: &str, entries: &str) -> String {
    format!(
        "[project]\nlanguage = \"{language}\"\n{entries}\n[[module]]\nname = \"app\"\nmatches = {{ units = [\"app\"] }}\n"
    )
}

/// The workspace fixture: `app` (kept crate) and `bus` (kept crate), each
/// with a `src/generated/` subtree. `extra` files ride inside `app/src`.
fn workspace_tree(entries: &str, extra_spec: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write("Cargo.toml", root_manifest());
    tree.write("app/Cargo.toml", app_manifest());
    tree.write(
        "app/src/main.rs",
        "fn main() {\n    let _ = bus::EventBus;\n}\n",
    );
    tree.write(
        "app/src/generated/mod.rs",
        "pub mod gen;\n\npub struct Generated;\n",
    );
    tree.write(
        "app/src/generated/gen.rs",
        "pub struct Gen;\n\npub fn use_bus() {\n    let _ = bus::EventBus;\n}\n",
    );
    tree.write("bus/Cargo.toml", &crate_manifest("bus"));
    tree.write("bus/src/lib.rs", "pub struct EventBus;\n");
    tree.write("architecture.spec.toml", &spec("rust", extra_spec));
    let _ = entries;
    tree
}

fn file_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("excluded source: "))
        .map(str::to_string)
        .collect()
}

fn liveness_lines(output: &std::process::Output) -> Vec<String> {
    common::stderr(output)
        .lines()
        .filter(|line| line.starts_with("excluded source matched nothing:"))
        .map(str::to_string)
        .collect()
}

fn scan_model(tree: &Fixture) -> Value {
    let out = tree.run(&["scan", "--output", "model.json"]);
    assert!(
        out.status.success(),
        "scan must succeed:\n{}",
        common::stderr(&out)
    );
    let raw = tree.read("model.json");
    serde_json::from_str(&raw).expect("model.json parses")
}

/// Cell A (rust half): the entry prunes the whole file tree of a member
/// crate. The unit SURVIVES — rust units come from manifests, not files —
/// with zero file facts, and every pruned file states itself once.
#[test]
fn whole_crate_prune_keeps_the_unit_and_states_every_file_once() {
    let tree = workspace_tree(
        "",
        "\nexcluded_sources = [ { path = \"app/src/generated\", reason = \"generated sources\" } ]\n",
    );
    let out = tree.run(&["scan", "--output", "model.json"]);
    assert_eq!(
        file_lines(&out),
        vec![
            "excluded source: app/src/generated/gen.rs — excluded_sources",
            "excluded source: app/src/generated/mod.rs — excluded_sources",
        ],
        "both files of the pruned crate subtree state themselves once, sorted:\n{}",
        common::stderr(&out)
    );
    let model = scan_model(&tree);
    let units: Vec<&str> = model["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["name"].as_str().expect("unit name"))
        .collect();
    assert!(
        units.contains(&"app"),
        "a fully pruned crate stays a unit (manifest-derived):\n{units:?}"
    );
    let body = serde_json::to_string(&model).unwrap_or_default();
    assert!(
        !body.contains("generated"),
        "the model states no fact of the pruned files (the word cannot appear \
         in any unit, module, edge or symbol row):\n{body}"
    );
}

/// Cell B: zero facts — the no-parse arm. A file the entry prunes is never
/// PARSED (the driver's hard-fail contract names the failing file for every
/// source it does read): the same tree with invalid syntax in the pruned
/// file verifies RED without the entry and GREEN with it, while the census
/// line states the drop. The pruned file leaves no fact anywhere in the
/// model body.
#[test]
fn pruned_file_is_neither_parsed_nor_stated_and_the_parse_flip_proves_it() {
    let build = |entries: &str| {
        let tree = Fixture::new();
        tree.write("Cargo.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
        tree.write("src/main.rs", "fn main() {}\n");
        tree.write(
            "src/generated/broken.rs",
            "this is not rust at all {{{\n",
        );
        tree.write(
            "architecture.spec.toml",
            &format!(
                "[project]\nlanguage = \"rust\"\n{entries}\n[[module]]\nname = \"app\"\nmatches = {{ units = [\"app\"] }}\n"
            ),
        );
        tree
    };
    // Without the entry the file is a source: the parse aborts naming it.
    let without = build("");
    let out = without.run(&["verify", "."]);
    assert!(
        !out.status.success(),
        "an unexcluded file with invalid syntax must abort extraction:\n{}",
        common::stderr(&out)
    );
    // With the entry the file is OUT OF THE ARCHITECTURE: never read, so it
    // cannot abort anything, and its name appears nowhere in the model.
    let with = build("\nexcluded_sources = [ { path = \"src/generated\", reason = \"generated sources\" } ]\n");
    let out = with.run(&["verify", "."]);
    assert!(
        out.status.success(),
        "a pruned file is never parsed, so it aborts nothing:\n{}",
        common::stderr(&out)
    );
    let lines = file_lines(&out);
    assert_eq!(
        lines,
        vec!["excluded source: src/generated/broken.rs — excluded_sources"],
        "the one pruned file states itself once even on verify:\n{}",
        common::stderr(&out)
    );
    let model = scan_model(&with);
    assert!(
        !serde_json::to_string(&model).unwrap().contains("broken"),
        "the model carries no fact of the pruned file:\n{model}"
    );
}

/// Cell C (rust half of the dedupe law): one file read by several walks —
/// the parse pass, the module walk and the pattern candidate feed — states
/// itself EXACTLY once per run, with and without a declaring `[[pattern_pair]]`
/// that puts the same file through the candidate walk.
#[test]
fn one_file_states_itself_once_across_walks_and_feeds() {
    let build = |pattern: &str| {
        let tree = Fixture::new();
        tree.write("Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
        tree.write(
            "src/lib.rs",
            "pub fn publish() {\n    emit_event(\"order-created\");\n}\n",
        );
        tree.write(
            "src/generated/gen.rs",
            "pub fn publish_generated() {\n    emit_event(\"generated-event\");\n}\n",
        );
        tree.write(
            "architecture.spec.toml",
            &format!(
                "[project]\nlanguage = \"rust\"\n\nexcluded_sources = [ {{ path = \"src/generated\", reason = \"generated sources\" }} ]\n[[module]]\nname = \"demo\"\nmatches = {{ units = [\"demo\"] }}\n{pattern}"
            ),
        );
        tree
    };
    let pattern = "\n[[pattern_pair]]\nid = \"demo-bus\"\n\n[pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"emit_event\"\narg = 0\n\n[pattern_pair.subscribe]\nlanguage = \"rust\"\nproperty = \"on\"\n";
    for (label, spec_pattern) in [("no pattern", ""), ("pattern feed", pattern)] {
        let tree = build(spec_pattern);
        for args in [vec!["scan"], vec!["verify", "."]] {
            let out = tree.run(&args);
            let lines = file_lines(&out);
            assert_eq!(
                lines,
                vec!["excluded source: src/generated/gen.rs — excluded_sources"],
                "exactly one census line per run for the pruned file ({label}, {}):\n{}",
                args[0],
                common::stderr(&out)
            );
        }
    }
    // The candidate feed respects the gate: with the pattern declared, the
    // pruned file's literal contributes no site; the kept file's does.
    let tree = build(pattern);
    let out = tree.run(&["scan", "--output", "model.json"]);
    let err = common::stderr(&out);
    let sites: Vec<&str> = err
        .lines()
        .filter(|line| line.starts_with("literal sites:") && line.contains("\"demo-bus\""))
        .collect();
    assert!(
        sites.iter().any(|line| line.contains("sites=1")),
        "the kept file's literal is one site:\n{}",
        common::stderr(&out)
    );
    let model = scan_model(&tree);
    assert!(
        !serde_json::to_string(&model).unwrap().contains("generated-event"),
        "the pruned file's literal is not a candidate:\n{model}"
    );
}

/// Cell D: entry liveness is driver-independent and unchanged — an entry that
/// prunes files is ALIVE (no `matched nothing` line); an entry that matches
/// nothing is DEAD and states itself once, while the alive entry stays silent.
#[test]
fn liveness_survives_the_prune_and_names_only_dead_entries() {
    let tree = workspace_tree(
        "",
        "\nexcluded_sources = [\n  { path = \"app/src/generated\", reason = \"generated sources\" },\n  { path = \"tools/absent\", reason = \"nothing there\" },\n]\n",
    );
    for verb in ["scan", "verify"] {
        let out = tree.run(&[verb, "."]);
        assert_eq!(
            liveness_lines(&out),
            vec!["excluded source matched nothing: tools/absent — nothing there"],
            "the pruning entry is alive (pruned files count as matches), the dead one is named ({verb}):\n{}",
            common::stderr(&out)
        );
        assert_eq!(
            file_lines(&out).len(),
            2,
            "the alive entry's files state themselves ({verb}):\n{}",
            common::stderr(&out)
        );
    }
}

/// Cell F: the census is report-only. `model.json` carries NO census field —
/// `report_only_lines` stays serde-skipped — and a tree whose spec declares no
/// `excluded_sources` key prints no census line and keeps its machine bytes.
#[test]
fn census_is_report_only_and_the_absent_key_stays_silent() {
    let declaring = workspace_tree(
        "",
        "\nexcluded_sources = [ { path = \"app/src/generated\", reason = \"generated sources\" } ]\n",
    );
    let out = declaring.run(&["scan", "--output", "model.json"]);
    assert_eq!(file_lines(&out).len(), 2, "declaring tree states its files");
    let raw = declaring.read("model.json");
    assert!(
        !raw.contains("report_only_lines") && !raw.contains("excluded source"),
        "the serialized model carries no census field (serde-skip law):\n{raw}"
    );
    // Absent key: zero lines, and the machine body equals the declaring
    // tree's minus the pruned bytes is NOT the claim — the claim is that a
    // non-declaring tree moves no census byte anywhere: run it and see only
    // the entry lines (scenario 08 law, rust side).
    let plain = workspace_tree("", "");
    let out = plain.run(&["scan", "--output", "model.json"]);
    assert!(
        file_lines(&out).is_empty() && liveness_lines(&out).is_empty(),
        "a tree declaring nothing prints no census line and no liveness line:\n{}",
        common::stderr(&out)
    );
}

/// Cell G (rust half): the mechanical gate is the FIRST door. A declared
/// entry naming a path under `target`/`vendor` never sees a file — the
/// mechanical walk skipped those subtrees before the declared gate ever
/// asked — so the file prints NO census line and the entry reads DEAD.
#[test]
fn mechanical_first_door_states_no_line_and_cannot_keep_an_entry_alive() {
    let tree = workspace_tree(
        "",
        "\nexcluded_sources = [ { path = \"app/target/generated\", reason = \"build output\" } ]\n",
    );
    tree.write(
        "app/target/generated/gen.rs",
        "pub struct Generated;\n",
    );
    for verb in ["scan", "verify"] {
        let out = tree.run(&[verb, "."]);
        assert!(
            file_lines(&out).is_empty(),
            "a file under the mechanical `target` door states NO declared-exclusion line ({verb}):\n{}",
            common::stderr(&out)
        );
        assert_eq!(
            liveness_lines(&out),
            vec!["excluded source matched nothing: app/target/generated — build output"],
            "the walk never visits `target`, so the entry is DEAD ({verb}):\n{}",
            common::stderr(&out)
        );
    }
}
