mod common;

use common::{stderr, stdout};

fn spec_fixture(spec: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("architecture.spec.toml", spec);
    fixture
}

const BASIC_SPEC: &str = r#"[project]
language = "rust"

[[module]]
name = "Billing"
matches = { units = ["Billing*"] }

[module.allowed]
depend_on = ["Shared"]
forbidden = []

[[module]]
name = "Shared"
matches = { units = ["Shared*"] }

[module.allowed]
depend_on = []
forbidden = []
"#;

// acceptance #1: spec with modules Billing, Shared and edge Billing -> Shared.
#[test]
fn diagram_scenario_one_renders_node_per_module_and_declared_edge() {
    let fixture = spec_fixture(BASIC_SPEC);
    let output = fixture.run(&["diagram"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    assert_eq!(
        stdout(&output),
        "graph TD\n  Billing\n  Shared\n  Billing --> Shared\n",
        "mermaid must have a node per module and the declared edge"
    );
}

// acceptance #2: spec declares forbidden edge Billing -> Portal.
#[test]
fn diagram_scenario_two_marks_forbidden_edge_dashed() {
    let fixture = spec_fixture(
        r#"[project]
language = "rust"

[[module]]
name = "Billing"
matches = { units = ["Billing*"] }

[module.allowed]
depend_on = ["Shared"]
forbidden = ["Portal"]

[[module]]
name = "Portal"
matches = { units = ["Portal*"] }

[[module]]
name = "Shared"
matches = { units = ["Shared*"] }
"#,
    );
    let output = fixture.run(&["diagram"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let out = stdout(&output);
    assert!(
        out.contains("  Billing -.->|forbidden| Portal\n"),
        "forbidden edge must be present and dashed:\n{out}"
    );
    assert!(
        out.contains("  Billing --> Shared\n"),
        "allowed edge must stay solid:\n{out}"
    );
}

// acceptance #3: spec declares nested sub-components.
#[test]
fn diagram_scenario_three_renders_nested_subcomponents_in_cluster() {
    let fixture = spec_fixture(
        r#"[project]
language = "rust"

[[module]]
name = "Billing"
matches = { units = ["Billing*"] }

[module.allowed]
depend_on = ["Shared"]
forbidden = []

[[module.submodules]]
name = "Billing.Domain"
matches = { units = ["Billing.Domain*"] }

[[module.submodules]]
name = "Billing.Api"
matches = { units = ["Billing.Api*"] }

[[module]]
name = "Shared"
matches = { units = ["Shared*"] }

[module.allowed]
depend_on = []
forbidden = []
"#,
    );
    let output = fixture.run(&["diagram"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let out = stdout(&output);
    assert!(
        out.contains("  subgraph Billing\n    Billing_Api_4df77886[\"Billing.Api\"]\n    Billing_Domain_bf7d55ec[\"Billing.Domain\"]\n  end\n"),
        "nested sub-components must render inside the parent cluster, sorted:\n{out}"
    );
    assert!(
        out.contains("  Billing\n"),
        "parent module node must be present:\n{out}"
    );
    assert!(
        out.contains("  Billing --> Shared\n"),
        "parent edge must still be drawn:\n{out}"
    );
}

// acceptance #4: spec lives in a project directory other than the cwd.
#[test]
fn diagram_scenario_four_renders_spec_from_explicit_dir() {
    let fixture = common::Fixture::new();
    fixture.write(
        "crates/auth/architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "Auth"
matches = { units = ["Auth*"] }

[module.allowed]
depend_on = ["Core"]
forbidden = []

[[module]]
name = "Core"
matches = { units = ["Core*"] }

[module.allowed]
depend_on = []
forbidden = []
"#,
    );
    let output = fixture.run(&["diagram", "./crates/auth"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let out = stdout(&output);
    assert!(
        out.contains("  Auth\n") && out.contains("  Core\n"),
        "nodes must come from the explicit dir spec:\n{out}"
    );
    assert!(
        out.contains("  Auth --> Core\n"),
        "edge must come from the explicit dir spec:\n{out}"
    );
}

// acceptance #8: --format plantuml on a spec project.
#[test]
fn diagram_behavior_8_plantuml_contains_nodes_and_edges() {
    let fixture = spec_fixture(BASIC_SPEC);
    let output = fixture.run(&["diagram", "--format", "plantuml"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    assert_eq!(
        stdout(&output),
        "@startuml\ncomponent Billing\ncomponent Shared\nBilling --> Shared\n@enduml\n",
        "plantuml must have a component per module and the declared edge"
    );
}

// acceptance #9: --output writes the file and keeps stdout empty.
#[test]
fn diagram_behavior_9_output_flag_writes_file_and_keeps_stdout_empty() {
    let fixture = spec_fixture(BASIC_SPEC);
    let output = fixture.run(&["diagram", "--output", "out.mmd"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(
        stdout(&output).is_empty(),
        "stdout must be empty with --output"
    );
    let written = fixture.read("out.mmd");
    assert!(
        written.contains("graph TD\n"),
        "file must be mermaid:\n{written}"
    );
    assert!(
        written.contains("  Billing --> Shared\n"),
        "file must contain the edge:\n{written}"
    );
}

// acceptance #10: same project renders byte-identical output across runs.
#[test]
fn diagram_behavior_10_is_deterministic_across_runs() {
    let fixture = spec_fixture(BASIC_SPEC);
    let first = fixture.run(&["diagram"]);
    let second = fixture.run(&["diagram"]);

    assert_eq!(first.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "diagram output must be byte-identical across runs"
    );
}

// acceptance #12: path does not exist.
#[test]
fn diagram_behavior_12_rejects_missing_path() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["diagram", "/no/such/dir"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path does not exist: /no/such/dir"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #13: path is a regular file.
#[test]
fn diagram_behavior_13_rejects_regular_file_path() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\n");
    let output = fixture.run(&["diagram", "Cargo.toml"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path is not a directory: Cargo.toml"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #14: project directory without architecture.spec.toml.
#[test]
fn diagram_behavior_14_rejects_directory_without_spec() {
    let fixture = common::Fixture::new();
    fixture.write("docs/readme.md", "hello");
    let output = fixture.run(&["diagram", "docs"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("no spec found under: docs"),
        "must say no spec and where:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #15: spec file with invalid TOML names the file, no partial diagram.
#[test]
fn diagram_behavior_15_spec_parse_error_names_failing_file() {
    let fixture = common::Fixture::new();
    fixture.write("architecture.spec.toml", "[project\nlanguage = \"rust\"\n");
    let output = fixture.run(&["diagram"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("failed to parse spec:") && err.contains("architecture.spec.toml"),
        "stderr must name the failing spec file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #19: spec declaring no components.
#[test]
fn diagram_behavior_19_spec_without_modules_reports_no_content() {
    let fixture = spec_fixture(
        r#"[project]
language = "rust"
"#,
    );
    let output = fixture.run(&["diagram"]);

    assert_ne!(output.status.code(), Some(0), "empty spec must fail");
    assert!(
        stderr(&output).contains("no model content to render"),
        "must say there is nothing to render:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// a spec with modules but no dependencies still renders (nodes are content).
#[test]
fn diagram_spec_with_modules_but_no_edges_still_renders() {
    let fixture = spec_fixture(
        r#"[project]
language = "rust"

[[module]]
name = "Billing"
matches = { units = ["Billing*"] }

[module.allowed]
depend_on = []
forbidden = []
"#,
    );
    let output = fixture.run(&["diagram"]);

    assert_eq!(output.status.code(), Some(0), "module list is content");
    assert_eq!(
        stdout(&output),
        "graph TD\n  Billing\n",
        "a spec with one module and no edges must still render the node"
    );
}

// acceptance #20: unsupported --format value.
#[test]
fn diagram_behavior_20_rejects_unsupported_format() {
    let fixture = spec_fixture(BASIC_SPEC);
    let output = fixture.run(&["diagram", "--format", "bmp", "."]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("unsupported format: bmp (supported: mermaid, plantuml)"),
        "must name the value and the supported set:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #21: unsupported --source value.
#[test]
fn diagram_behavior_21_rejects_unsupported_source() {
    let fixture = spec_fixture(BASIC_SPEC);
    let output = fixture.run(&["diagram", "--source", "gif", "."]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("unsupported source: gif (supported: spec, scan)"),
        "must name the value and the supported set:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #22: --source scan without an artefact path.
#[test]
fn diagram_behavior_22_scan_requires_artefact_path() {
    let fixture = spec_fixture(BASIC_SPEC);
    let output = fixture.run(&["diagram", "--source", "scan"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("--source scan requires an artefact path"),
        "must say an artefact path is required:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #23: two positional paths given.
#[test]
fn diagram_behavior_23_rejects_two_positional_paths() {
    let fixture = spec_fixture(BASIC_SPEC);
    let output = fixture.run(&["diagram", "a", "b"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("expected at most one path argument"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// ---------- scan-snapshot mode (acceptance #5-#7, #11, #16-#18) ----------

// Rust workspace with crates auth, core, portal; auth depends on core and portal.
fn scan_workspace_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/core\", \"crates/portal\"]\n",
    );
    for name in ["auth", "core", "portal"] {
        let deps = if name == "auth" {
            "[dependencies]\ncore = { path = \"../core\" }\nportal = { path = \"../portal\" }\n"
        } else {
            ""
        };
        fixture.write(
            &format!("crates/{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n{deps}"
            ),
        );
        fixture.write(&format!("crates/{name}/src/lib.rs"), "pub fn f() {}\n");
    }
    fixture
}

fn scan_to_artefact(fixture: &common::Fixture) {
    let scan = fixture.run(&["scan", "--output", "model.json"]);
    assert_eq!(
        scan.status.code(),
        Some(0),
        "scan must produce the artefact"
    );
    assert!(stdout(&scan).is_empty(), "scan --output keeps stdout empty");
}

// acceptance #5: units auth, core, portal with edges auth -> core and auth -> portal;
// governing spec forbids auth -> portal.
#[test]
fn diagram_scan_marks_edge_that_violates_governing_spec() {
    let fixture = scan_workspace_fixture();
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "auth"
matches = { units = ["auth"] }

[module.allowed]
forbidden = ["portal"]

[[module]]
name = "core"
matches = { units = ["core"] }

[[module]]
name = "portal"
matches = { units = ["portal"] }
"#,
    );
    scan_to_artefact(&fixture);
    let output = fixture.run(&["diagram", "--source", "scan", "model.json"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    assert_eq!(
        stdout(&output),
        "graph TD\n  subgraph project\n    auth\n    core\n    portal\n    auth --> core\n    auth -.->|violates spec| portal\n  end\n",
        "scan diagram must have a node per unit and mark the violating edge dashed"
    );
}

// acceptance #6: scan artefact with an external dependency (serde).
#[test]
fn diagram_scan_renders_external_crates_in_separate_cluster() {
    let fixture = common::Fixture::new();
    fixture.write(
        "model.json",
        r#"{
  "schema_version": 1,
  "language": "rust",
  "units": [
    { "name": "auth", "kind": "crate", "path": "crates/auth" }
  ],
  "edges": [
    { "from": "auth", "to": "serde" }
  ],
  "external": ["serde"]
}
"#,
    );
    let output = fixture.run(&["diagram", "--source", "scan", "model.json"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    assert_eq!(
        stdout(&output),
        "graph TD\n  subgraph project\n    auth\n  end\n  subgraph external\n    serde\n  end\n  auth --> serde\n",
        "external crate must live in its own cluster and stay reachable from the edge"
    );
}

// acceptance #7: scan artefact, no governing spec present -> no edges marked.
#[test]
fn diagram_scan_without_governing_spec_marks_no_edges() {
    let fixture = scan_workspace_fixture();
    scan_to_artefact(&fixture);
    let output = fixture.run(&["diagram", "--source", "scan", "model.json"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
    let out = stdout(&output);
    assert!(
        !out.contains("-.->"),
        "no edges may be marked without a governing spec:\n{out}"
    );
    assert!(
        out.contains("  auth --> core\n"),
        "auth -> core must stay solid:\n{out}"
    );
    assert!(
        out.contains("  auth --> portal\n"),
        "auth -> portal must stay solid:\n{out}"
    );
}

// the positional path stays the project dir; --source scan binds its own artefact.
#[test]
fn diagram_scan_uses_project_dir_positional_for_governing_spec() {
    let fixture = scan_workspace_fixture();
    fixture.write(
        "crates/auth/architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "auth"
matches = { units = ["auth"] }

[module.allowed]
forbidden = ["portal"]
"#,
    );
    scan_to_artefact(&fixture);
    let output = fixture.run(&["diagram", "./crates/auth", "--source", "scan", "model.json"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);
    assert!(
        out.contains("  auth -.->|violates spec| portal\n"),
        "governing spec must be looked up in the project dir positional:\n{out}"
    );
}

// acceptance #11: same scan artefact, unchanged -> byte-identical output.
#[test]
fn diagram_behavior_11_scan_is_deterministic_across_runs() {
    let fixture = scan_workspace_fixture();
    scan_to_artefact(&fixture);
    let first = fixture.run(&["diagram", "--source", "scan", "model.json"]);
    let second = fixture.run(&["diagram", "--source", "scan", "model.json"]);

    assert_eq!(first.status.code(), Some(0), "first run must exit 0");
    assert_eq!(second.status.code(), Some(0), "second run must exit 0");
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "scan diagram must be byte-identical across runs"
    );
}

// acceptance #16: scan artefact that does not exist.
#[test]
fn diagram_behavior_16_scan_rejects_missing_artefact() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["diagram", "--source", "scan", "missing.json"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("scan artefact not found: missing.json"),
        "must identify the missing artefact:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #17: scan artefact that is not a valid model snapshot.
#[test]
fn diagram_behavior_17_scan_rejects_invalid_artefact() {
    let fixture = common::Fixture::new();
    fixture.write("bad.json", "this is not json");
    let output = fixture.run(&["diagram", "--source", "scan", "bad.json"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("invalid scan artefact: bad.json"),
        "must name the invalid artefact:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #17 edge case: valid JSON that is not a Model shape must map to
// `invalid scan artefact`, not a panic or a different message.
#[test]
fn diagram_behavior_17_scan_rejects_json_that_is_not_a_model() {
    let fixture = common::Fixture::new();
    fixture.write("not-model.json", r#"{"foo":"bar"}"#);
    let output = fixture.run(&["diagram", "--source", "scan", "not-model.json"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("invalid scan artefact: not-model.json"),
        "must name the invalid artefact, not panic:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// acceptance #18: scan artefact with an empty model.
#[test]
fn diagram_behavior_18_scan_rejects_empty_model() {
    let fixture = common::Fixture::new();
    fixture.write(
        "empty.json",
        r#"{"schema_version":1,"language":"rust","units":[],"edges":[]}"#,
    );
    let output = fixture.run(&["diagram", "--source", "scan", "empty.json"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("no model content to render"),
        "must say there is nothing to render:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// a flag following --source scan must not be swallowed as the artefact.
#[test]
fn diagram_scan_requires_artefact_when_flag_follows() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["diagram", "--source", "scan", "--format", "mermaid"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("--source scan requires an artefact path"),
        "a following flag must not be treated as the artefact:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

// workplan 05: all diagram entry points share one rendering owner, so hyphenated
// names get mermaid-safe ids with the raw name kept as the label.
#[test]
fn diagram_escapes_hyphenated_module_names_in_mermaid() {
    let fixture = spec_fixture(
        r#"[project]
language = "rust"

[[module]]
name = "my-mod"
matches = { units = ["my-mod"] }

[module.allowed]
depend_on = ["Shared"]

[[module]]
name = "Shared"
matches = { units = ["Shared"] }
"#,
    );
    let output = fixture.run(&["diagram"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);
    assert!(
        out.contains("my_mod_f59bd035[\"my-mod\"]"),
        "hyphenated module must render an escaped id with a label:\n{out}"
    );
    assert!(
        out.contains("my_mod_f59bd035 --> Shared"),
        "edges must reference the escaped id:\n{out}"
    );
    assert!(
        !out.contains("my-mod -->"),
        "raw hyphenated id must not be used in edges:\n{out}"
    );
}

// spec clusters must not interpolate raw names: the subgraph header uses the
// same escaped id the parent gets as a node, members render escaped.
#[test]
fn diagram_escapes_hyphenated_cluster_parent_and_members() {
    let fixture = spec_fixture(
        r#"[project]
language = "rust"

[[module]]
name = "my-parent"
matches = { units = ["my-parent*"] }

[module.allowed]
depend_on = ["Shared"]
forbidden = []

[[module.submodules]]
name = "my-child"
matches = { units = ["my-child*"] }

[[module]]
name = "Shared"
matches = { units = ["Shared*"] }

[module.allowed]
depend_on = []
forbidden = []
"#,
    );
    let output = fixture.run(&["diagram"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);
    assert_eq!(
        out,
        "graph TD\n  Shared\n  my_parent_a0fc5c53[\"my-parent\"]\n  subgraph my_parent_a0fc5c53[\"my-parent\"]\n    my_child_b2db3f7d[\"my-child\"]\n  end\n  my_parent_a0fc5c53 --> Shared\n",
        "cluster header id must equal the escaped declared node id and members must render escaped:\n{out}"
    );
    assert!(
        out.contains("  subgraph my_parent_a0fc5c53[\"my-parent\"]\n"),
        "subgraph header must reuse the escaped node id with a quoted title:\n{out}"
    );
    assert!(
        !out.contains("subgraph my-parent"),
        "raw hyphenated parent id must not appear in the header:\n{out}"
    );
    assert!(
        !out.contains("\n    my-child\n"),
        "raw hyphenated member id must not be interpolated bare:\n{out}"
    );
}

// the same escaping applies to scan-sourced diagrams.
#[test]
fn diagram_scan_escapes_hyphenated_unit_names() {
    let fixture = common::Fixture::new();
    fixture.write(
        "model.json",
        r#"{
  "schema_version": 1,
  "language": "rust",
  "units": [
    { "name": "my-unit", "kind": "crate", "path": "crates/my-unit" },
    { "name": "core", "kind": "crate", "path": "crates/core" }
  ],
  "edges": [
    { "from": "my-unit", "to": "core" }
  ]
}
"#,
    );
    let output = fixture.run(&["diagram", "--source", "scan", "model.json"]);

    assert_eq!(output.status.code(), Some(0), "exit must be 0");
    let out = stdout(&output);
    assert!(
        out.contains("my_unit_e447a9c8[\"my-unit\"]"),
        "scan diagram nodes must use the shared escaped-id rendering:\n{out}"
    );
    assert!(
        out.contains("my_unit_e447a9c8 --> core"),
        "scan diagram edges must reference escaped ids:\n{out}"
    );
    assert!(
        !out.contains("my-unit -->"),
        "raw hyphenated unit id must not be used:\n{out}"
    );
}
