mod common;

use common::{stderr, stdout};

fn scenario_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "use crate::b::Thing;\n");
    fixture.write("src/b.rs", "pub struct Thing;\n");
    fixture
}

/// A crate with a manifest (so `scan::extract` can build a model) and a module
/// edge. The lib is the only unit, named `app`; `billing` imports `auth`.
fn structural_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");
    fixture
}

fn block(diagram: &str, marker: &str, name: &str) -> String {
    let lines: Vec<&str> = diagram.lines().collect();
    let quoted = format!("{marker} \"{name}\"");
    let bare = format!("{marker} {name}");
    let start = lines.iter().position(|line| {
        let trimmed = line.trim();
        trimmed == quoted || trimmed == bare || trimmed == format!("{quoted} {{")
    });
    let start = start.unwrap_or_else(|| panic!("{marker} {name} not found in:\n{diagram}"));
    let mut depth = 0usize;
    let mut block = Vec::new();
    for line in &lines[start..] {
        block.push(*line);
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            depth += 1;
        } else if trimmed == "end" || trimmed == "}" {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
    }
    block.join("\n")
}

fn package_block(diagram: &str, name: &str) -> String {
    block(diagram, "package", name)
}

fn mermaid_id(name: &str) -> String {
    let mut sanitized = String::new();
    let mut changed = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            sanitized.push(c);
        } else {
            sanitized.push('_');
            changed = true;
        }
    }
    if !changed {
        return sanitized;
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let hex = format!("{hash:016x}");
    format!("{sanitized}_{}", &hex[..8])
}

fn subgraph_block(diagram: &str, name: &str) -> String {
    let lines: Vec<&str> = diagram.lines().collect();
    let id = mermaid_id(name);
    let start = lines
        .iter()
        .position(|line| {
            let trimmed = line.trim();
            trimmed == format!("subgraph {id}") || trimmed.starts_with(&format!("subgraph {id}["))
        })
        .unwrap_or_else(|| panic!("subgraph {name} not found in:\n{diagram}"));
    let mut depth = 0usize;
    let mut block = Vec::new();
    for line in &lines[start..] {
        block.push(*line);
        let trimmed = line.trim();
        if trimmed.starts_with("subgraph ") {
            depth += 1;
        } else if trimmed == "end" {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
    }
    block.join("\n")
}

#[test]
fn inspect_renders_node_per_file_and_resolved_edge() {
    let fixture = scenario_fixture();
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(stdout.contains("graph TD"), "missing graph TD:\n{stdout}");
    assert!(
        stdout.contains("src/lib.rs"),
        "missing node src/lib.rs:\n{stdout}"
    );
    assert!(
        stdout.contains("src/a.rs"),
        "missing node src/a.rs:\n{stdout}"
    );
    assert!(
        stdout.contains("src/b.rs"),
        "missing node src/b.rs:\n{stdout}"
    );
    assert!(
        stdout.contains("src_a_rs_d1e1ab14 --> src_b_rs_a74680fd"),
        "missing edge src_a_rs_d1e1ab14 --> src_b_rs_\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_folders_into_subgraphs_scenario_two() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod orchestration;\n");
    fixture.write("src/orchestration/common.rs", "pub struct Util;\n");
    fixture.write(
        "src/orchestration/state.rs",
        "use crate::orchestration::common::Util;\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    let src_block = subgraph_block(&stdout, "src");
    assert!(
        src_block.contains("src/lib.rs"),
        "lib.rs missing from src subgraph:\n{src_block}"
    );
    assert!(
        src_block.contains("subgraph src_orchestration_a72cbfd5[\"src/orchestration\"]"),
        "orchestration subgraph missing from src subgraph:\n{src_block}"
    );
    assert!(
        stdout.contains("src_orchestration_state_rs_824af904 --> src_orchestration_common_rs_6fb2aeed"),
        "missing resolved edge:\n{stdout}"
    );
    let orchestration = subgraph_block(&stdout, "src/orchestration");
    assert!(
        orchestration.contains("src/orchestration/common.rs"),
        "common.rs missing from orchestration subgraph:\n{orchestration}"
    );
    assert!(
        orchestration.contains("src/orchestration/state.rs"),
        "state.rs missing from orchestration subgraph:\n{orchestration}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_chains_sibling_top_level_subgraphs_for_vertical_stacking() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("tests/integration.rs", "fn smoke() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("~~~") && !stdout.contains("linkStyle"),
        "sibling subgraphs must be chained with invisible links so the diagram stacks vertically:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// Edge lines of a mermaid diagram: any line carrying a link operator.
fn edge_lines(diagram: &str) -> Vec<&str> {
    diagram
        .lines()
        .map(str::trim)
        .filter(|line| line.contains("-->") || line.contains("~~~") || line.contains("---"))
        .collect()
}

// Scenario: hosted renderer shows only import edges
#[test]
fn inspect_hosted_renderers_show_only_import_edges() {
    let fixture = scenario_fixture();
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    for line in edge_lines(&stdout) {
        assert!(
            !line.contains("---"),
            "visible layout edge leaked into diagram: {line}"
        );
    }
    assert!(
        stdout.contains("~~~"),
        "layout links must use the invisible link operator ~~~:\n{stdout}"
    );
    assert!(
        stdout.contains("subgraph src"),
        "files must stay grouped in directory subgraphs:\n{stdout}"
    );
    assert!(
        stdout.contains("src_a_rs_d1e1ab14 --> src_b_rs_a74680fd"),
        "import edge must remain visible:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: no linkStyle directives emitted
#[test]
fn inspect_emits_no_linkstyle_and_only_known_edge_kinds() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\npub mod util;\n");
    fixture.write("src/a.rs", "use crate::b::Thing;\npub struct A;\n");
    fixture.write("src/b.rs", "pub struct Thing;\n");
    fixture.write("src/util/help.rs", "pub struct Help;\n");
    fixture.write("src/util/kind.rs", "use crate::util::help::Help;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        !stdout.contains("linkStyle"),
        "no linkStyle directive may be emitted:\n{stdout}"
    );
    assert!(
        stdout.contains("~~~"),
        "layout spine must still be emitted as invisible links:\n{stdout}"
    );
    for line in edge_lines(&stdout) {
        let has_import = line.contains("-->");
        let has_layout = line.contains("~~~");
        let has_visible = line.contains("---");
        assert!(
            (has_import || has_layout) && !has_visible && !(has_import && has_layout),
            "edge must be either an import edge or an invisible layout link: {line}"
        );
    }
    assert!(
        stdout.contains("src_a_rs_d1e1ab14 --> src_b_rs_a74680fd"),
        "import edges must be unchanged:\n{stdout}"
    );
    assert!(
        stdout.contains("src_util_kind_rs_51dcd8bc --> src_util_help_rs_0b9de17f"),
        "import edges must be unchanged:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: layout links carry no semantics
#[test]
fn inspect_zero_import_project_has_only_invisible_layout_links() {
    let fixture = common::Fixture::new();
    // Wired neither by imports nor by declarations: the lib stays empty so the
    // graph has no visible connectors at all.
    fixture.write("src/lib.rs", "");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/b.rs", "pub struct B;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        !stdout.contains("-->"),
        "a project without imports must show no visible connectors:\n{stdout}"
    );
    assert!(
        stdout.contains("~~~"),
        "files must still be chained with invisible links:\n{stdout}"
    );
    let src_block = subgraph_block(&stdout, "src");
    assert!(
        src_block.contains("src/lib.rs")
            && src_block.contains("src/a.rs")
            && src_block.contains("src/b.rs"),
        "files must be grouped by directory:\n{src_block}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: single file or single directory still renders
#[test]
fn inspect_single_file_per_directory_emits_no_dangling_endpoints() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod group;\n");
    fixture.write("src/group/item.rs", "pub struct Item;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    let nodes: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("graph ")
                && !line.starts_with("subgraph ")
                && *line != "end"
                && !line.contains("-->")
                && !line.contains("~~~")
                && !line.contains("---")
        })
        .collect();
    assert!(
        nodes.contains(&"src_lib_rs_2fba4152[\"src/lib.rs\"]")
            && nodes.contains(&"src_group_item_rs_17e4a9b2[\"src/group/item.rs\"]"),
        "single-file directories must still declare their nodes:\n{stdout}"
    );
    for line in edge_lines(&stdout) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(
            parts.len(),
            3,
            "edge line must have exactly two non-empty endpoints: {line}"
        );
        assert!(
            nodes.iter().any(|node| node.split('[').next() == Some(parts[0])),
            "edge endpoint is not a declared node: {line}"
        );
        assert!(
            nodes.iter().any(|node| node.split('[').next() == Some(parts[2])),
            "edge endpoint is not a declared node: {line}"
        );
    }
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_output_flag_writes_file_and_keeps_stdout_empty() {
    let fixture = scenario_fixture();
    let output = fixture.run(&["inspect", "--output", "out.mmd"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).is_empty(),
        "stdout should be empty when --output is given"
    );
    let written = fixture.read("out.mmd");
    assert!(
        written.contains("graph TD"),
        "missing graph TD in file:\n{written}"
    );
    assert!(
        written.contains("src_a_rs_d1e1ab14 --> src_b_rs_a74680fd"),
        "missing edge in file:\n{written}"
    );
}

#[test]
fn inspect_rejects_missing_path() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["inspect", "/no/such/dir"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path does not exist: /no/such/dir"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

#[test]
fn inspect_rejects_regular_file_path() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\n");
    let output = fixture.run(&["inspect", "Cargo.toml"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path is not a directory: Cargo.toml"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

/// Scenario: inspect on a file points the way — the error names the modes
/// that exist (file-level import maps for rust/csharp/go and structural
/// tree/scanner model views) and shows the directory form, not just the
/// bare "not a directory". The prose must stay honest: no mode-existence
/// claim may single Go out as having no inspect mode.
#[test]
fn inspect_file_error_names_modes_and_directory_form() {
    let fixture = common::Fixture::new();
    fixture.write("src/a.rs", "fn a() {}\n");
    let output = fixture.run(&["inspect", "src/a.rs"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(err.contains("path is not a directory: src/a.rs"), "{err}");
    assert!(err.contains("file-level import map modes (rust, csharp"), "{err}");
    assert!(err.contains("inspect tree|scanner"), "{err}");
    assert!(
        !err.contains("go has no inspect mode"),
        "go has a file-level import map now; the message must not claim otherwise:\n{err}"
    );
    assert!(err.contains("archspec inspect src"), "directory form must be shown:\n{err}");
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

/// The structural modes refuse a file with the same directional error.
#[test]
fn inspect_structural_file_error_names_modes_and_directory_form() {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", "[package]\n");
    let output = fixture.run(&["inspect", "tree", "Cargo.toml"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(err.contains("path is not a directory: Cargo.toml"), "{err}");
    assert!(err.contains("file-level import map modes (rust, csharp"), "{err}");
    assert!(err.contains("archspec inspect ."), "directory form must be shown:\n{err}");
}

#[test]
fn inspect_rejects_directory_without_supported_sources() {
    let fixture = common::Fixture::new();
    fixture.write("docs/readme.md", "hello");
    let output = fixture.run(&["inspect", "docs"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("no supported-language sources found under: docs"),
        "expected language-agnostic no-sources message:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

fn go_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("main.go", "package main\n\nfunc main() {}\n");
    fixture
}

/// The default mode renders the go file-level import map: nodes per file,
/// resolved import edges, grouped by directory like the other languages.
#[test]
fn inspect_renders_go_file_import_map_in_default_mode() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "handler/handler.go",
        "package handler\n\nimport \"example.com/demo/store\"\n\nfunc Handle() {}\n",
    );
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("graph TD"), "missing graph TD:\n{stdout}");
    assert!(stdout.contains("handler/handler.go"), "node missing:\n{stdout}");
    assert!(stdout.contains("store/store.go"), "node missing:\n{stdout}");
    let edge = format!(
        "{} --> {}",
        mermaid_id("handler/handler.go"),
        mermaid_id("store/store.go")
    );
    assert!(stdout.contains(&edge), "missing edge {edge}:\n{stdout}");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// The tree view projects only the module tier; a single-module go model
/// carries none, so the refusal names the missing model fact (and how a Go
/// tree acquires one), not a language verdict.
#[test]
fn inspect_tree_refuses_single_module_go_with_module_tier_message() {
    let output = go_fixture().run(&["inspect", "tree"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("inspect tree needs the module tier, which this model has none of"),
        "stderr must name the missing module tier:\n{err}"
    );
    assert!(
        err.contains("go.work members"),
        "stderr must say how a Go tree acquires the tier:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

/// The scanner view renders whatever units and edges the model carries: a
/// single-module go tree has units, so it renders, not refuses.
#[test]
fn inspect_scanner_renders_go_unit_model() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "app/app.go",
        "package app\n\nimport \"example.com/demo/shared\"\n\nfunc App() {}\n",
    );
    fixture.write("shared/shared.go", "package shared\n\nfunc Shared() {}\n");
    let output = fixture.run(&["inspect", "scanner"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("graph TD"), "missing graph TD:\n{stdout}");
    assert!(
        stdout.contains("example.com/demo/app") && stdout.contains("example.com/demo/shared"),
        "units missing from scanner view:\n{stdout}"
    );
    let edge = format!(
        "{} --> {}",
        mermaid_id("example.com/demo/app"),
        mermaid_id("example.com/demo/shared")
    );
    assert!(stdout.contains(&edge), "missing unit edge {edge}:\n{stdout}");
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// A `go.work` tree carries the module tier (the members), so the tree view
/// renders containment instead of refusing.
#[test]
fn inspect_tree_renders_go_workspace_module_tier() {
    let fixture = common::Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write(
        "api/api.go",
        "package api\n\nimport \"example.com/store\"\n\nfunc Api() {}\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");
    let output = fixture.run(&["inspect", "tree"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("example.com/api") && stdout.contains("example.com/store"),
        "member modules missing:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_plantuml_renders_nodes_and_resolved_edges() {
    let fixture = scenario_fixture();
    let output = fixture.run(&["inspect", "--format", "plantuml"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("@startuml"),
        "missing @startuml header:\n{stdout}"
    );
    assert!(
        stdout.contains("@enduml"),
        "missing @enduml footer:\n{stdout}"
    );
    assert!(
        stdout.contains("entity \"src/lib.rs\""),
        "missing entity src/lib.rs:\n{stdout}"
    );
    assert!(
        stdout.contains("entity \"src/a.rs\""),
        "missing entity src/a.rs:\n{stdout}"
    );
    assert!(
        stdout.contains("entity \"src/b.rs\""),
        "missing entity src/b.rs:\n{stdout}"
    );
    assert!(
        stdout.contains("\"src/a.rs\" --> \"src/b.rs\""),
        "missing edge src_a_rs_d1e1ab14 --> src_b_rs_\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_plantuml_packages_group_folder_files() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod orchestration;\n");
    fixture.write("src/orchestration/common.rs", "pub struct Util;\n");
    fixture.write(
        "src/orchestration/state.rs",
        "use crate::orchestration::common::Util;\n",
    );
    let output = fixture.run(&["inspect", "--format", "plantuml"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    let src_block = package_block(&stdout, "src");
    assert!(
        src_block.contains("entity \"src/lib.rs\""),
        "lib.rs missing from src package:\n{src_block}"
    );
    assert!(
        src_block.contains("package \"src/orchestration\""),
        "orchestration package missing from src package:\n{src_block}"
    );
    let orchestration = package_block(&stdout, "src/orchestration");
    assert!(
        orchestration.contains("entity \"src/orchestration/common.rs\""),
        "common.rs missing from orchestration package:\n{orchestration}"
    );
    assert!(
        orchestration.contains("entity \"src/orchestration/state.rs\""),
        "state.rs missing from orchestration package:\n{orchestration}"
    );
    assert!(
        stdout.contains("\"src/orchestration/state.rs\" --> \"src/orchestration/common.rs\""),
        "missing resolved edge:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_output_flag_writes_plantuml_file_and_keeps_stdout_empty() {
    let fixture = scenario_fixture();
    let output = fixture.run(&["inspect", "--format", "plantuml", "--output", "out.puml"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).is_empty(),
        "stdout should be empty when --output is given"
    );
    let written = fixture.read("out.puml");
    assert!(
        written.contains("@startuml"),
        "missing @startuml in file:\n{written}"
    );
    assert!(
        written.contains("\"src/a.rs\" --> \"src/b.rs\""),
        "missing edge in file:\n{written}"
    );
}

#[test]
fn inspect_is_deterministic_across_runs() {
    let fixture = scenario_fixture();
    let first = fixture.run(&["inspect"]);
    let second = fixture.run(&["inspect"]);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "mermaid output must be byte-identical across runs"
    );
    assert_eq!(
        stdout(&first),
        "graph TD\n  subgraph src\n    src_a_rs_d1e1ab14[\"src/a.rs\"]\n    src_b_rs_a74680fd[\"src/b.rs\"]\n    src_lib_rs_2fba4152[\"src/lib.rs\"]\n    src_a_rs_d1e1ab14 ~~~ src_b_rs_a74680fd\n    src_b_rs_a74680fd ~~~ src_lib_rs_2fba4152\n  end\n  src_a_rs_d1e1ab14 --> src_b_rs_a74680fd\n  src_lib_rs_2fba4152 --> src_a_rs_d1e1ab14\n  src_lib_rs_2fba4152 --> src_b_rs_a74680fd\n",
        "mermaid output must match canonical ordering exactly"
    );
}

#[test]
fn inspect_plantuml_is_deterministic_across_runs() {
    let fixture = scenario_fixture();
    let first = fixture.run(&["inspect", "--format", "plantuml"]);
    let second = fixture.run(&["inspect", "--format", "plantuml"]);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "plantuml output must be byte-identical across runs"
    );
    assert_eq!(
        stdout(&first),
        "@startuml\n  package \"src\" {\n    entity \"src/a.rs\"\n    entity \"src/b.rs\"\n    entity \"src/lib.rs\"\n  }\n\"src/a.rs\" --> \"src/b.rs\"\n\"src/lib.rs\" --> \"src/a.rs\"\n\"src/lib.rs\" --> \"src/b.rs\"\n@enduml\n",
        "plantuml output must match canonical ordering exactly"
    );
}

#[test]
fn inspect_rejects_unsupported_format() {
    let fixture = scenario_fixture();
    let output = fixture.run(&["inspect", "--format", "bmp"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("unsupported format: bmp (supported: mermaid, plantuml)"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

#[test]
fn inspect_rejects_two_positional_paths() {
    let fixture = scenario_fixture();
    let output = fixture.run(&["inspect", "src", "tests"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("expected at most one path argument"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

#[test]
fn inspect_test_dir_does_not_shadow_src_module() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/b.rs", "use crate::a::A;\n");
    fixture.write("tests/a.rs", "pub struct Shadow;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_b_rs_a74680fd --> src_a_rs_d1e1ab14"),
        "crate::a must resolve to src/a.rs, not tests/a.rs:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_b_rs_a74680fd --> tests_a_rs_8393175e"),
        "tests/a.rs must not shadow src/a.rs:\n{stdout}"
    );
}

#[test]
fn inspect_resolves_super_import_to_sibling_module() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod parent;\n");
    fixture.write("src/parent/mod.rs", "pub mod common;\npub mod pipeline;\n");
    fixture.write("src/parent/common.rs", "pub struct Thing;\n");
    fixture.write("src/parent/pipeline.rs", "use super::common::Thing;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_parent_pipeline_rs_99408973 --> src_parent_common_rs_1a307840"),
        "missing edge via super:::\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_resolves_repeated_super_relative_to_file_module() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod outer;\n");
    fixture.write("src/outer/mod.rs", "pub mod inner;\npub mod sibling;\n");
    fixture.write("src/outer/sibling.rs", "pub struct S;\n");
    fixture.write("src/outer/inner/mod.rs", "pub mod deep;\n");
    fixture.write("src/outer/inner/deep.rs", "use super::super::sibling::S;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_outer_inner_deep_rs_5683b6e8 --> src_outer_sibling_rs_9f1c778e"),
        "missing edge via repeated super:::\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_resolves_super_from_mod_rs_to_parent_level_module() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod parent;\npub mod top;\n");
    fixture.write(
        "src/parent/mod.rs",
        "pub mod common;\nuse super::top::Top;\n",
    );
    fixture.write("src/parent/common.rs", "pub struct C;\n");
    fixture.write("src/top.rs", "pub struct Top;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_parent_mod_rs_ca54a90c --> src_top_rs_65aa3712"),
        "super from mod.rs must resolve above the parent module:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_resolves_super_from_root_level_file() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/b.rs", "use super::a::A;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_b_rs_a74680fd --> src_a_rs_d1e1ab14"),
        "super from a root-level file must resolve to the sibling module file:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_resolves_inline_module_to_containing_file() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod c;\n");
    fixture.write("src/a.rs", "mod inner {\n    pub struct Type;\n}\n");
    fixture.write("src/c.rs", "use crate::a::inner::Type;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_c_rs_25753b05 --> src_a_rs_d1e1ab14"),
        "inline module a::inner must resolve to src/a.rs:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_real_module_file_wins_over_inline_declaration() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod c;\n");
    fixture.write(
        "src/a.rs",
        "mod inner {\n    pub struct Type;\n}\npub struct A;\n",
    );
    fixture.write("src/a/inner.rs", "pub struct Type;\n");
    fixture.write("src/c.rs", "use crate::a::inner::Type;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_c_rs_25753b05 --> src_a_inner_rs_eac5870d"),
        "real module file must own a::inner over the inline declaration:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_c_rs_25753b05 --> src_a_rs_d1e1ab14"),
        "fallback to src/a.rs must not happen when src/a/inner.rs exists:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_resolves_nested_inline_modules_to_containing_file() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod c;\n");
    fixture.write(
        "src/a.rs",
        "mod outer {\n    mod inner {\n        pub struct Type;\n    }\n}\n",
    );
    fixture.write("src/c.rs", "use crate::a::outer::inner::Type;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_c_rs_25753b05 --> src_a_rs_d1e1ab14"),
        "nested inline modules must resolve to src/a.rs:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_inline_module_in_lib_rs_resolves_to_root() {
    let fixture = common::Fixture::new();
    fixture.write(
        "src/lib.rs",
        "mod inner {\n    pub struct Type;\n}\npub mod c;\n",
    );
    fixture.write("src/c.rs", "use crate::inner::Type;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_c_rs_25753b05 --> src_lib_rs_2fba4152"),
        "inline module at crate root must resolve to src/lib.rs:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_super_inside_inline_module_resolves_relative_to_it() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\n");
    fixture.write(
        "src/a.rs",
        "mod x {\n    use super::y::Y;\n}\nmod y {\n    pub struct Y;\n}\n",
    );
    fixture.write("src/y.rs", "pub struct RealY;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        !stdout.contains("src_a_rs_d1e1ab14 --> src_y_rs_898edad3"),
        "super:: inside inline mod x must resolve to inline a::y, not src/y.rs:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_self_import_same_file_no_self_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\n");
    fixture.write("src/a.rs", "pub struct Thing;\nuse self::Thing as T;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/a.rs"),
        "node src/a.rs missing:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_a_rs_d1e1ab14 --> src_a_rs_d1e1ab14"),
        "spurious self-edge for self::Thing:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_crate_reference_to_own_inline_module_no_self_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\n");
    fixture.write(
        "src/a.rs",
        "mod self_path {\n    pub fn func() {}\n}\nfn run() {\n    crate::a::self_path::func();\n}\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/a.rs"),
        "node src/a.rs missing:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_a_rs_d1e1ab14 --> src_a_rs_d1e1ab14"),
        "spurious self-edge for crate::a::self_path::func():\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_crate_reference_to_own_module_keeps_cross_file_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write(
        "src/a.rs",
        "pub struct Thing;\nfn run() {\n    let _ = crate::a::Thing;\n}\n",
    );
    fixture.write("src/b.rs", "use crate::a::Thing;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_b_rs_a74680fd --> src_a_rs_d1e1ab14"),
        "cross-file edge crate::a::Thing must resolve to src/a.rs:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_a_rs_d1e1ab14 --> src_a_rs_d1e1ab14"),
        "spurious self-edge for crate::a::Thing in own file:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_self_inline_import_keeps_cross_file_edge_drops_self_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write(
        "src/a.rs",
        "mod inner {\n    pub struct X;\n}\nuse self::inner::X;\npub struct Thing;\n",
    );
    fixture.write("src/b.rs", "use crate::a::Thing;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_b_rs_a74680fd --> src_a_rs_d1e1ab14"),
        "cross-file edge crate::a::Thing must resolve to src/a.rs:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_a_rs_d1e1ab14 --> src_a_rs_d1e1ab14"),
        "spurious self-edge for self::inner::X:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_self_inline_submodule_same_file_no_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\n");
    fixture.write(
        "src/a.rs",
        "mod x {\n    pub struct Y;\n}\nuse self::x::Y;\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/a.rs"),
        "node src/a.rs missing:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_a_rs_d1e1ab14 --> src_a_rs_d1e1ab14"),
        "self::x to inline mod x in same file must not draw a self-edge:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_self_from_mod_rs_resolves_to_submodule_file() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod orchestration;\n");
    fixture.write(
        "src/orchestration/mod.rs",
        "pub mod common;\npub mod util;\nuse self::common::Thing;\n",
    );
    fixture.write("src/orchestration/common.rs", "pub struct Thing;\n");
    fixture.write("src/orchestration/util.rs", "");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_orchestration_mod_rs_aaa1e54e --> src_orchestration_common_rs_6fb2aeed"),
        "self::common from mod.rs must resolve to the submodule file:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_orchestration_mod_rs_aaa1e54e --> src_orchestration_mod_rs_aaa1e54e"),
        "no spurious self-edge:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_self_submodule_real_file_draws_cross_file_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\n");
    fixture.write("src/a.rs", "pub mod x;\nuse self::x::Y;\n");
    fixture.write("src/a/x.rs", "pub struct Y;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_a_rs_d1e1ab14 --> src_a_x_rs_b6f19178"),
        "self::x to real submodule file src/a/x.rs must draw an edge:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

fn broken_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/broken.rs", "fn broken( {\n");
    fixture
}

#[test]
fn inspect_parse_error_names_failing_file_and_writes_no_partial_diagram() {
    let fixture = broken_fixture();
    let output = fixture.run(&["inspect"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("failed to parse source:") && err.contains("src/broken.rs"),
        "stderr must name the failing file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

#[test]
fn inspect_parse_error_with_output_flag_creates_no_file() {
    let fixture = broken_fixture();
    let output = fixture.run(&["inspect", "--output", "out.mmd"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("failed to parse source:") && err.contains("src/broken.rs"),
        "stderr must name the failing file:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "stdout should be empty");
    assert!(
        !fixture.path("out.mmd").exists(),
        "no partial diagram file on parse error"
    );
}

#[test]
fn inspect_tree_renders_module_structure_from_scan_model() {
    let fixture = structural_fixture();
    let output = fixture.run(&["inspect", "tree"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(stdout.contains("graph TD"), "missing graph TD:\n{stdout}");
    assert!(stdout.contains("app"), "unit name missing:\n{stdout}");
    assert!(
        stdout.contains("app::auth"),
        "module app::auth missing:\n{stdout}"
    );
    assert!(
        stdout.contains("app::billing"),
        "module app::billing missing:\n{stdout}"
    );
    assert!(
        stdout.contains("app__billing_2a6e299f --> app__auth_b11d7a62"),
        "module edge missing:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_scanner_reflects_scan_model() {
    let fixture = structural_fixture();
    let output = fixture.run(&["inspect", "scanner"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(stdout.contains("graph TD"), "missing graph TD:\n{stdout}");
    assert!(stdout.contains("app"), "unit name missing:\n{stdout}");
    assert!(
        stdout.contains("app::auth"),
        "module boundary app::auth missing:\n{stdout}"
    );
    assert!(
        stdout.contains("app__billing_2a6e299f --> app__auth_b11d7a62"),
        "module edge missing:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// A two-unit package whose lib name is hyphenated (`weird-kit`) so its mermaid
/// id is FNV-suffixed, and whose bin (`weirdkit`) references the lib through the
/// underscore crate name. The bin->lib reference is a cross-unit (unit-tier)
/// edge: it belongs to `inspect scanner` and must never appear in `inspect tree`.
fn cross_unit_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"weird-kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"weirdkit\"\npath = \"src/main.rs\"\n",
    );
    fixture.write("src/lib.rs", "mod feature;\n");
    fixture.write("src/feature.rs", "pub fn run() {}\n");
    fixture.write("src/main.rs", "fn main() {\n    let _ = weird_kit::feature::run;\n}\n");
    fixture
}

#[test]
fn inspect_tree_is_deterministic_across_runs_and_excludes_unit_edge() {
    let fixture = cross_unit_fixture();
    let first = fixture.run(&["inspect", "tree"]);
    assert_eq!(first.status.code(), Some(0));
    let canonical = stdout(&first);
    for _ in 0..5 {
        let again = fixture.run(&["inspect", "tree"]);
        assert_eq!(again.status.code(), Some(0));
        assert_eq!(
            canonical,
            stdout(&again),
            "inspect tree output must be byte-identical across runs"
        );
    }
    // The cross-unit edge is a unit-tier edge: absent from tree, present in
    // scanner. This pins the reported phantom edge to its real (mode) source.
    assert!(
        !canonical
            .lines()
            .any(|line| line.starts_with("  weirdkit --> weird_kit_")),
        "tree view must not emit the cross-unit edge:\n{canonical}"
    );
    let scanner = fixture.run(&["inspect", "scanner"]);
    assert_eq!(scanner.status.code(), Some(0));
    let scanner_out = stdout(&scanner);
    assert!(
        scanner_out
            .lines()
            .any(|line| line.starts_with("  weirdkit --> weird_kit_")),
        "scanner view must draw the cross-unit edge:\n{scanner_out}"
    );
}

#[test]
fn inspect_default_file_view_unchanged() {
    let fixture = structural_fixture();
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(stdout.contains("graph TD"), "missing graph TD:\n{stdout}");
    assert!(
        stdout.contains("src/lib.rs"),
        "file-level view must render src/lib.rs:\n{stdout}"
    );
    assert!(
        stdout.contains("src_billing_rs_7822d1ec --> src_auth_rs_fd5c4eea"),
        "file-level edge must be preserved:\n{stdout}"
    );
}

// Scenario: bin import of lib resolves to lib file (workplan 06)
#[test]
fn inspect_own_crate_name_import_in_bin_resolves_to_lib_file() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod render;\n");
    fixture.write("src/render.rs", "pub fn draw() {}\n");
    fixture.write("src/main.rs", "use kit::render::draw;\nfn main() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_main_rs_df9f8d2d --> src_render_rs_8e751ed1"),
        "own-crate-name import from bin must resolve to the lib file:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: nested lib module path resolves to owning file (workplan 06)
#[test]
fn inspect_own_crate_name_nested_path_resolves_to_owning_file() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod render;\n");
    fixture.write("src/render.rs", "pub mod svg {\n    pub struct Svg;\n}\n");
    fixture.write("src/main.rs", "use kit::render::svg::Svg;\nfn main() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_main_rs_df9f8d2d --> src_render_rs_8e751ed1"),
        "nested own-crate-name path must point to the single file owning svg:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: lib-only project: own-name import from a lib file resolves (workplan 06)
#[test]
fn inspect_own_crate_name_import_in_lib_file_resolves() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "use kit::b::Thing;\npub struct A;\n");
    fixture.write("src/b.rs", "pub struct Thing;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_a_rs_d1e1ab14 --> src_b_rs_a74680fd"),
        "own-crate-name import in a lib file must resolve to src/b.rs:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: genuine externals unchanged (workplan 06)
#[test]
fn inspect_third_party_import_remains_edgeless_despite_matching_directory() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    fixture.write("src/lib.rs", "pub mod a;\nmod serde;\n");
    fixture.write("src/a.rs", "use serde::Serialize;\npub struct A;\n");
    fixture.write("src/serde.rs", "pub struct Serialize;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        !stdout.contains("src_a_rs_d1e1ab14 --> src_serde_rs_8eca0750"),
        "external crate name must not resolve via directory names:\n{stdout}"
    );
    assert!(
        stdout.contains("src/serde.rs"),
        "module file must still be a node:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: name collision between lib and bin roots, lib wins deterministically (workplan 06)
#[test]
fn inspect_own_name_collision_prefers_lib_deterministically() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod run;\npub mod support;\n");
    fixture.write("src/support.rs", "pub struct Helper;\n");
    fixture.write("src/run.rs", "use kit::support::Helper;\n");
    fixture.write(
        "src/main.rs",
        "mod support {\n    pub struct Local;\n}\nfn main() {}\n",
    );
    let first = fixture.run(&["inspect"]);
    let second = fixture.run(&["inspect"]);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    let first_out = stdout(&first);
    assert!(
        first_out.contains("src_run_rs_6db088db --> src_support_rs_362777e5"),
        "lib file must win the lib/bin root-name collision:\n{first_out}"
    );
    assert!(
        !first_out.contains("src_run_rs_6db088db --> src_main_rs_df9f8d2d"),
        "bin root module must not win the collision:\n{first_out}"
    );
    assert_eq!(
        first_out,
        stdout(&second),
        "collision resolution must be byte-identical across runs"
    );
    assert!(stderr(&first).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_from_src_dir_resolves_edges() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/b.rs", "use crate::a::A;\n");
    let output = fixture.run(&["inspect", "src"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("b_rs_ab76ce9a --> a_rs_6ad38482"),
        "edges must resolve when scanning src/ directly:\n{stdout}"
    );
}

/// Occurrences of an exact edge line in a mermaid diagram.
fn edge_count(diagram: &str, edge: &str) -> usize {
    diagram.lines().filter(|line| line.trim() == edge).count()
}

// Scenario: declaration produces an edge (workplan 08)
#[test]
fn inspect_declaration_edge_resolves_sibling_file_and_mod_rs_forms() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod group;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/group/mod.rs", "pub struct G;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_lib_rs_2fba4152 --> src_a_rs_d1e1ab14"),
        "declaration mod a must draw lib_rs_7f53aba3 --> src_a_rs_\n{stdout}"
    );
    assert!(
        stdout.contains("src_lib_rs_2fba4152 --> src_group_mod_rs_94d5afd9"),
        "declaration mod group must resolve the dir/mod.rs form:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: hub is no longer isolated (workplan 08)
#[test]
fn inspect_declaration_only_entrypoint_hub_not_isolated() {
    let fixture = common::Fixture::new();
    fixture.write("src/main.rs", "mod commands;\nmod config;\nfn main() {}\n");
    fixture.write("src/commands/mod.rs", "pub mod build;\npub mod run;\n");
    fixture.write("src/commands/build.rs", "pub fn build() {}\n");
    fixture.write("src/commands/run.rs", "pub fn run() {}\n");
    fixture.write("src/config.rs", "pub struct Config;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_main_rs_df9f8d2d --> src_commands_mod_rs_98633f2a")
            && stdout.contains("src_main_rs_df9f8d2d --> src_config_rs_13ec58ab"),
        "entrypoint wiring only through declarations must still show outgoing edges:\n{stdout}"
    );
    assert!(
        stdout.contains("src_commands_mod_rs_98633f2a --> src_commands_build_rs_55382373")
            && stdout.contains("src_commands_mod_rs_98633f2a --> src_commands_run_rs_61189c11"),
        "directory mod.rs hub must connect to its children:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: declarations and imports both visible, collapsed (workplan 08)
#[test]
fn inspect_declaration_and_import_pair_collapses_to_single_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\nuse crate::a::Thing;\n");
    fixture.write("src/a.rs", "pub struct Thing;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert_eq!(
        edge_count(&stdout, "src_lib_rs_2fba4152 --> src_a_rs_d1e1ab14"),
        1,
        "declaration and import edges between the same pair must collapse to one:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

// Scenario: unresolved or inline declarations change nothing (workplan 08)
#[test]
fn inspect_unresolved_and_inline_declarations_emit_no_edge_or_node() {
    let fixture = common::Fixture::new();
    fixture.write(
        "src/lib.rs",
        "pub mod a;\npub mod ghost;\nmod inlined {\n    pub struct I;\n}\n",
    );
    fixture.write("src/a.rs", "mod nested_inline {\n    pub struct N;\n}\nuse crate::b::B;\n");
    fixture.write("src/b.rs", "pub struct B;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        !stdout.contains("ghost"),
        "unresolved declaration must emit neither edge nor phantom node:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_lib_rs_2fba4152 --> src_lib_rs_2fba4152") && !stdout.contains("src_a_rs_d1e1ab14 --> src_a_rs_d1e1ab14"),
        "inline mod blocks must not draw declaration edges:\n{stdout}"
    );
    assert!(
        stdout.contains("src_a_rs_d1e1ab14 --> src_b_rs_a74680fd") && stdout.contains("src_lib_rs_2fba4152 --> src_a_rs_d1e1ab14"),
        "the rest of the graph must be unchanged:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}
