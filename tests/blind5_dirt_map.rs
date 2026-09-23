//! The artefact dirt map (blind5 follow-up US_04): which source change
//! dirties which artefact. The rust blind run proved a comment append left
//! every `--check` green, then a zero-edge module dirtied `scan` while
//! `report`, the spec-mode `diagram` and `verify` stayed green — and found
//! this stated nowhere. The owner sentence lives in `archspec help
//! diagnostics`; the scenarios below are the probe rows of
//! `workplan_archspec_blind5_followup/sources.md` replayed as fixture edits,
//! so every clause of the sentence is welded to behavior, not prose.
//! The tree mirrors the probed lib-tree: a crate-root module owning the soft
//! (undeclared) modules, two declared components — `surface` with one
//! declared edge to `facts` — and `contract`, a component with no declared
//! edges at baseline. All four destinations are wired and a depgraph
//! projection is committed (checked via its explicit
//! `--output … --check` pairing — depgraph reads no `[output]`).
mod common;

use common::{stderr, stdout, Fixture};

/// The owner sentence byte-for-byte as `archspec help diagnostics` states
/// it (welded constant; US_05 registers it and adds the surface weld).
const DIRT_MAP_SENTENCE: &str = "Artefact freshness keys on structure, not content: a comment-only edit dirties no artefact; edits that change structure (files, modules, or edges) dirty the `scan` model JSON, the `inspect` file map, and `depgraph` projections; `report` follows only structure that reaches the component layer (component edges, metrics, violations), the same layer at which `verify` engages; the spec-mode `diagram` renders declared components and never dirties on source edits, only on spec changes.";

const DEPROJECTION: &str = "docs/archspec/depgraph.mmd";

/// A tree with configured destinations for scan, diagram, report and
/// inspect plus a committed depgraph projection, all artefacts fresh.
fn dirt_tree() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub mod contract;\npub mod facts;\npub mod model;\npub mod surface;\n",
    );
    fixture.write("src/facts.rs", "pub struct Facts;\n");
    fixture.write("src/model.rs", "pub struct Data;\n");
    fixture.write(
        "src/surface.rs",
        "use crate::facts::Facts;\npub fn handle(_: Facts) {}\n",
    );
    fixture.write("src/contract.rs", "pub struct Guard;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"app::surface\"\nmatches = { modules = [\"app::surface\"] }\nallowed = { depend_on = [\"app::facts\"] }\n\n[[module]]\nname = \"app::facts\"\nmatches = { modules = [\"app::facts\"] }\n\n[[module]]\nname = \"app::contract\"\nmatches = { modules = [\"app::contract\"] }\n",
    );
    fixture.write(
        "archspec.toml",
        "[output]\nscan = \"docs/archspec/scan.json\"\ndiagram = \"docs/archspec/diagram.mmd\"\ninspect = \"docs/archspec/inspect.mmd\"\nreport = \"docs/archspec/report.md\"\n",
    );
    for command in ["scan", "diagram", "inspect", "report"] {
        let generated = fixture.run(&[command]);
        assert_eq!(
            generated.status.code(),
            Some(0),
            "generate {command}: {}",
            stderr(&generated)
        );
    }
    let projection = fixture.run(&[
        "depgraph",
        "modules",
        "--format",
        "mermaid",
        "--output",
        DEPROJECTION,
    ]);
    assert_eq!(
        projection.status.code(),
        Some(0),
        "commit the depgraph projection: {}",
        stderr(&projection)
    );
    fixture
}

/// Run every freshness surface: the four configured `--check`s and the
/// projection's explicit pairing. Returns (name, exit code, combined text).
fn run_checks(fixture: &Fixture) -> Vec<(String, i32, String)> {
    let mut runs = Vec::new();
    for command in ["scan", "diagram", "report", "inspect"] {
        let check = fixture.run(&[command, "--check"]);
        runs.push((
            command.to_string(),
            check.status.code().unwrap_or(-1),
            format!("{}{}", stdout(&check), stderr(&check)),
        ));
    }
    let projection = fixture.run(&[
        "depgraph",
        "modules",
        "--format",
        "mermaid",
        "--output",
        DEPROJECTION,
        "--check",
    ]);
    runs.push((
        "depgraph".to_string(),
        projection.status.code().unwrap_or(-1),
        format!("{}{}", stdout(&projection), stderr(&projection)),
    ));
    let verify = fixture.run(&["verify"]);
    runs.push((
        "verify".to_string(),
        verify.status.code().unwrap_or(-1),
        format!("{}{}", stdout(&verify), stderr(&verify)),
    ));
    runs
}

fn require_clean(fixture: &Fixture, context: &str) {
    for (name, code, text) in run_checks(fixture) {
        assert_eq!(code, 0, "{context}: {name} check failed:\n{text}");
        if name != "verify" {
            assert!(
                text.starts_with("ok: ") && text.contains("up to date"),
                "{context}: green {name} check must echo the ok-line, got:\n{text}"
            );
        }
    }
}

fn require_red(fixture: &Fixture, name: &str, context: &str, hint: Option<&str>) {
    let runs = run_checks(fixture);
    let (_, code, text) = runs
        .iter()
        .find(|(surface, _, _)| surface == name)
        .expect("surface ran");
    assert_ne!(*code, 0, "{context}: {name} must be red:\n{text}");
    if let Some(hint) = hint {
        assert!(
            text.contains(hint),
            "{context}: red {name} must carry the exact-form hint {hint}:\n{text}"
        );
    }
}

/// Scenario: the dirt-map sentence is stated at the diagnostics surface.
#[test]
fn dirt_map_sentence_is_stated_at_help_diagnostics() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "diagnostics"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains(DIRT_MAP_SENTENCE),
        "help diagnostics must state the dirt-map sentence verbatim:\n{}",
        stdout(&output)
    );
}

/// Scenario: a comment-only edit dirties nothing — every check exits 0 with
/// its ok-line and verify passes (probe row: all six green).
#[test]
fn comment_only_edit_dirties_no_artefact() {
    let fixture = dirt_tree();
    require_clean(&fixture, "baseline");

    let mut model = fixture.read("src/model.rs");
    model.push_str("// dirt-map probe comment\n");
    fixture.write("src/model.rs", &model);

    require_clean(&fixture, "after a comment-only edit");
}

/// Scenario: a structural edit whose module never reaches the component
/// layer dirties the model-coupled renders only — scan JSON, inspect file
/// map and depgraph projection go red while the spec-mode diagram, report
/// and verify stay green (probe row: zero-edge file + `mod` line).
#[test]
fn structural_edit_dirties_the_model_coupled_renders() {
    let fixture = dirt_tree();
    require_clean(&fixture, "baseline");

    fixture.write("src/probe_extra.rs", "pub struct ProbeExtra;\n");
    let mut lib = fixture.read("src/lib.rs");
    lib.insert_str(0, "pub mod probe_extra;\n");
    fixture.write("src/lib.rs", &lib);

    require_red(&fixture, "scan", "after a zero-edge module", Some("regenerate with: archspec scan"));
    require_red(&fixture, "inspect", "after a zero-edge module", Some("regenerate with: archspec inspect"));
    require_red(&fixture, "depgraph", "after a zero-edge module", Some("regenerate with: archspec depgraph"));
    for surface in ["diagram", "report"] {
        let (_, code, text) = run_checks(&fixture)
            .into_iter()
            .find(|(name, _, _)| name == surface)
            .expect("surface ran");
        assert_eq!(code, 0, "{surface} must stay green after a soft structural edit:\n{text}");
        assert!(
            text.starts_with("ok: ") && text.contains("up to date"),
            "{surface} must echo its ok-line:\n{text}"
        );
    }
    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "a zero-edge module engages no component boundary, verify must pass:\n{}",
        stdout(&verify)
    );
}

/// Scenario: an undeclared edge from a component reaches report and engages
/// verify (scan, inspect and the projection are red anyway), while the
/// spec-mode diagram still tracks only the spec (probe row: component gains
/// an edge).
#[test]
fn component_layer_edge_reaches_report_and_verify() {
    let fixture = dirt_tree();
    require_clean(&fixture, "baseline");

    fixture.write(
        "src/contract.rs",
        "use crate::facts::Facts;\npub struct Guard;\n#[allow(dead_code)]\nfn require(_: Facts) {}\n",
    );

    require_red(&fixture, "scan", "after a component edge", None);
    require_red(&fixture, "inspect", "after a component edge", None);
    require_red(&fixture, "depgraph", "after a component edge", None);
    require_red(&fixture, "report", "after a component edge", Some("regenerate with: archspec report"));
    let diagram = fixture.run(&["diagram", "--check"]);
    assert_eq!(
        diagram.status.code(),
        Some(0),
        "the spec-mode diagram tracks the spec only, it must stay green:\n{}",
        stdout(&diagram)
    );
    let verify = fixture.run(&["verify"]);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "a component-level undeclared edge must fail verify: {}",
        stderr(&verify)
    );
    assert!(
        stdout(&verify).contains(
            "disallowed cross-component dependency: app::contract -> app::facts"
        ),
        "verify must name the cross-component dependency:\n{}",
        stdout(&verify)
    );
}
