// US 01 (workplan_archspec_blind4_followup): the universal `wrote` echo +
// exploratory renders that never clobber. Every command mode that writes a
// body to a path — an `[output]` destination of any command and any format,
// or an explicit `--output` on any command — echoes exactly one line
// `wrote <path>` (D01); an exploratory `diagram --source scan` render without
// `--output` goes to stdout and leaves the configured destination untouched
// (D02). Exit codes are unchanged (D08); a run without a destination keeps
// today's bytes (body on stdout, no echo).

mod common;

use common::{stderr, stdout, Fixture};

/// A single crate whose modules import each other, with a spec that matches:
/// every render command runs on this tree with exit 0 and the module tier the
/// depgraph views project.
fn model_tree_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
    fixture
}

/// Scenario "depgraph's silent mode echoes": the mode the go run named —
/// `depgraph --output` wrote silently while report echoed — now echoes the
/// same line, and only stdout moves.
#[test]
fn depgraph_output_mode_echoes_wrote_line() {
    let fixture = model_tree_fixture();
    let output = fixture.run(&["depgraph", "modules", "--output", "graph.mmd"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "wrote graph.mmd\n",
        "depgraph --output echoes exactly one wrote line (stderr: {})",
        stderr(&output)
    );
    // the bytes at P are identical to the pre-plan bytes: they equal the body
    // the same tree prints to stdout, where nothing moved (scenario leg 3).
    let body = fixture.run(&["depgraph", "modules", "--output", "-"]);
    assert_eq!(
        fixture.read("graph.mmd"),
        stdout(&body),
        "only stdout moved: the written body equals the stdout body"
    );
}

/// Scenario "the echo form is universal": one line `wrote <path>` on every
/// path-writing mode — configured destinations of human AND machine formats,
/// explicit `--output` on every command, every format — the path spelled as
/// configured/resolved; no machine body anywhere carries status text inside it.
#[test]
fn echo_form_is_universal_across_commands_formats_and_origins() {
    let fixture = model_tree_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\nreport = \"docs/archspec/report.md\"\nscan = \"docs/archspec/scan.json\"\n",
    );

    // configured destinations: the human report (echo already existed, wave 3)
    // and the machine model JSON (silent today — D01 retires the format split).
    let runs: Vec<Vec<&str>> = vec![
        vec!["report"],
        vec!["scan"],
        vec!["report", "--output", "flag/report.txt"],
        vec!["report", "--format", "json", "--output", "flag/report.json"],
        vec!["diagram", "--output", "flag/diagram.mmd"],
        vec!["diagram", "--format", "plantuml", "--output", "flag/diagram.puml"],
        vec!["inspect", "--output", "flag/inspect.mmd"],
        vec!["depgraph", "modules", "--output", "flag/graph.mmd"],
        vec!["depgraph", "api-usage", "--output", "flag/api.md"],
        vec!["scan", "--output", "flag/model.json"],
    ];
    let echoes = [
        "wrote docs/archspec/report.md",
        "wrote docs/archspec/scan.json",
        "wrote flag/report.txt",
        "wrote flag/report.json",
        "wrote flag/diagram.mmd",
        "wrote flag/diagram.puml",
        "wrote flag/inspect.mmd",
        "wrote flag/graph.mmd",
        "wrote flag/api.md",
        "wrote flag/model.json",
    ];
    for (args, line) in runs.iter().zip(echoes) {
        let output = fixture.run(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?} must exit 0 (stderr: {})",
            stderr(&output)
        );
        assert_eq!(
            stdout(&output),
            format!("{line}\n"),
            "{args:?} stdout carries exactly the one wrote line"
        );
    }

    // no machine body carries status text inside it: the written JSON parses
    // and holds no echo bytes; a machine body on stdout stays byte-clean.
    for artefact in ["docs/archspec/scan.json", "flag/model.json", "flag/report.json"] {
        let body = fixture.read(artefact);
        let value: serde_json::Value =
            serde_json::from_str(&body).expect("machine artefact stays pure JSON");
        assert!(value.is_object(), "{artefact} is a JSON body");
        assert!(
            !body.contains("wrote ") && !body.contains("ok:") && !body.contains("note:"),
            "{artefact} must carry no status text inside the body"
        );
    }
    let stdout_run = fixture.run(&["scan", "--output", "-"]);
    let out = stdout(&stdout_run);
    let value: serde_json::Value = serde_json::from_str(&out).expect("stdout body stays pure");
    assert!(value.is_object(), "scan stdout body is JSON:\n{out}");
}

/// Scenario "exploratory scan render goes to stdout": with the destination
/// configured and holding the committed spec-mode artefact, an exploratory
/// `diagram --source scan` render without `--output` prints the body and
/// leaves the destination byte-identical — it can never clobber the artefact.
#[test]
fn exploratory_scan_render_prints_to_stdout_and_never_touches_destination() {
    let fixture = model_tree_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\ndiagram = \"docs/archspec/diagram.mmd\"\n",
    );
    fixture.write("docs/archspec/diagram.mmd", "COMMITTED SPEC-MODE DIAGRAM\n");
    let scan = fixture.run(&["scan", "--output", "model.json"]);
    assert_eq!(scan.status.code(), Some(0), "{}", stderr(&scan));

    let output = fixture.run(&["diagram", "--source", "scan", "model.json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("graph"), "stdout carries the mermaid body:\n{out}");
    assert_eq!(
        fixture.read("docs/archspec/diagram.mmd"),
        "COMMITTED SPEC-MODE DIAGRAM\n",
        "the committed spec-mode artefact survives the exploratory render"
    );
}

/// Scenario "explicit --output still writes": the exploratory axis with an
/// explicit `--output` keeps the old capability (body to Q + `wrote Q`, D
/// untouched), while the canonical spec mode keeps writing D and echoing.
#[test]
fn explicit_output_still_writes_and_canonical_mode_keeps_its_contract() {
    let fixture = model_tree_fixture();
    fixture.write(
        "archspec.toml",
        "[output]\ndiagram = \"docs/archspec/diagram.mmd\"\n",
    );
    fixture.write("docs/archspec/diagram.mmd", "COMMITTED SPEC-MODE DIAGRAM\n");
    let scan = fixture.run(&["scan", "--output", "model.json"]);
    assert_eq!(scan.status.code(), Some(0), "{}", stderr(&scan));

    let exploratory =
        fixture.run(&["diagram", "--source", "scan", "model.json", "--output", "Q.mmd"]);
    assert_eq!(
        exploratory.status.code(),
        Some(0),
        "{}",
        stderr(&exploratory)
    );
    assert_eq!(
        stdout(&exploratory),
        "wrote Q.mmd\n",
        "the explicit write is announced and nothing else moves"
    );
    assert!(fixture.read("Q.mmd").contains("graph"), "body at Q");
    assert_eq!(
        fixture.read("docs/archspec/diagram.mmd"),
        "COMMITTED SPEC-MODE DIAGRAM\n",
        "the configured destination stays untouched"
    );

    // canonical mode: unchanged contract — writes D, echoes `wrote D`.
    let canonical = fixture.run(&["diagram"]);
    assert_eq!(canonical.status.code(), Some(0), "{}", stderr(&canonical));
    assert_eq!(
        stdout(&canonical),
        "wrote docs/archspec/diagram.mmd\n",
        "spec mode keeps its contract"
    );
    assert!(
        fixture.read("docs/archspec/diagram.mmd").contains("graph"),
        "spec mode writes the destination"
    );
}

/// Scenario "nothing configured, nothing echoed": with no destination for the
/// command, every form keeps today's bytes on stdout — the body, no `wrote`
/// line, no destination statement.
#[test]
fn nothing_configured_nothing_echoed() {
    let fixture = model_tree_fixture();
    for args in [
        vec!["report"],
        vec!["diagram"],
        vec!["inspect"],
        vec!["depgraph", "modules"],
    ] {
        let output = fixture.run(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?} must exit 0 (stderr: {})",
            stderr(&output)
        );
        let out = stdout(&output);
        assert!(!out.is_empty(), "{args:?} keeps its body on stdout");
        assert!(
            !out.contains("wrote ") && !out.contains("not written"),
            "{args:?} must echo nothing without a destination:\n{out}"
        );
    }
    let scan = fixture.run(&["scan"]);
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&scan)).expect("scan stdout stays pure JSON");
    assert!(value.is_object());
    assert!(
        !fixture.path("docs/archspec").exists(),
        "nothing configured writes no files"
    );
}
