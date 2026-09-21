mod common;

use common::{stderr, stdout};

fn csharp_scenario_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("src/A.cs", "namespace Demo.App;\npublic class A { }\n");
    fixture.write(
        "src/B.cs",
        "using Demo.App;\nnamespace Demo.Other;\npublic class B { }\n",
    );
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
fn inspect_csharp_renders_node_per_file_and_resolved_edge() {
    let fixture = csharp_scenario_fixture();
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(stdout.contains("graph TD"), "missing graph TD:\n{stdout}");
    assert!(
        stdout.contains("src/A.cs"),
        "missing node src/A.cs:\n{stdout}"
    );
    assert!(
        stdout.contains("src/B.cs"),
        "missing node src/B.cs:\n{stdout}"
    );
    assert!(
        stdout.contains("src_B_cs_7cf27ff1 --> src_A_cs_ff3d220b"),
        "missing edge src_B_cs_7cf27ff1 --> src_A_cs_\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_folders_into_subgraphs() {
    let fixture = common::Fixture::new();
    fixture.write("src/Web/Controller.cs", "namespace Demo.Web;\npublic class Controller { }\n");
    fixture.write(
        "src/Web/Models/Item.cs",
        "namespace Demo.Web.Models;\npublic class Item { }\n",
    );
    fixture.write(
        "src/Web/Services/Handler.cs",
        "using Demo.Web.Models;\nnamespace Demo.Web.Services;\npublic class Handler { }\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    let web_block = subgraph_block(&stdout, "src/Web");
    assert!(
        web_block.contains("src/Web/Controller.cs"),
        "Controller.cs missing from Web subgraph:\n{web_block}"
    );
    assert!(
        web_block.contains("subgraph src_Web_Models_dd9ff296[\"src/Web/Models\"]"),
        "Models subgraph missing from Web subgraph:\n{web_block}"
    );
    assert!(
        web_block.contains("subgraph src_Web_Services_1fa52ab1[\"src/Web/Services\"]"),
        "Services subgraph missing from Web subgraph:\n{web_block}"
    );
    let models = subgraph_block(&stdout, "src/Web/Models");
    assert!(
        models.contains("src/Web/Models/Item.cs"),
        "Item.cs missing from Models subgraph:\n{models}"
    );
    let services = subgraph_block(&stdout, "src/Web/Services");
    assert!(
        services.contains("src/Web/Services/Handler.cs"),
        "Handler.cs missing from Services subgraph:\n{services}"
    );
    assert!(
        stdout.contains("src_Web_Services_Handler_cs_90ca20ea --> src_Web_Models_Item_cs_c726a37f"),
        "missing resolved edge:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_plantuml_renders_entities_and_packages() {
    let fixture = csharp_scenario_fixture();
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
        stdout.contains("entity \"src/A.cs\""),
        "missing entity src/A.cs:\n{stdout}"
    );
    assert!(
        stdout.contains("entity \"src/B.cs\""),
        "missing entity src/B.cs:\n{stdout}"
    );
    assert!(
        stdout.contains("\"src/B.cs\" --> \"src/A.cs\""),
        "missing edge src_B_cs_7cf27ff1 --> src_A_cs_\n{stdout}"
    );
    let src_block = package_block(&stdout, "src");
    assert!(
        src_block.contains("entity \"src/A.cs\"") && src_block.contains("entity \"src/B.cs\""),
        "entities missing from src package:\n{src_block}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_output_flag_writes_file_and_keeps_stdout_empty() {
    let fixture = csharp_scenario_fixture();
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
        written.contains("src_B_cs_7cf27ff1 --> src_A_cs_ff3d220b"),
        "missing edge in file:\n{written}"
    );
}

#[test]
fn inspect_csharp_is_deterministic_across_runs() {
    let fixture = csharp_scenario_fixture();
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
        "graph TD\n  subgraph src\n    src_A_cs_ff3d220b[\"src/A.cs\"]\n    src_B_cs_7cf27ff1[\"src/B.cs\"]\n    src_A_cs_ff3d220b ~~~ src_B_cs_7cf27ff1\n  end\n  src_B_cs_7cf27ff1 --> src_A_cs_ff3d220b\n",
        "mermaid output must match canonical ordering exactly"
    );
}

#[test]
fn inspect_csharp_plantuml_is_deterministic_across_runs() {
    let fixture = csharp_scenario_fixture();
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
        "@startuml\n  package \"src\" {\n    entity \"src/A.cs\"\n    entity \"src/B.cs\"\n  }\n\"src/B.cs\" --> \"src/A.cs\"\n@enduml\n",
        "plantuml output must match canonical ordering exactly"
    );
}

#[test]
fn inspect_csharp_multi_file_namespace_fans_out_to_all_files() {
    let fixture = common::Fixture::new();
    fixture.write("src/Models/M1.cs", "namespace Demo.Models;\npublic class M1 { }\n");
    fixture.write("src/Models/M2.cs", "namespace Demo.Models;\npublic class M2 { }\n");
    fixture.write(
        "src/Use.cs",
        "using Demo.Models;\nnamespace Demo.Use;\npublic class Use { }\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_Use_cs_6d2bbbab --> src_Models_M1_cs_c6500c54"),
        "using Demo.Models must edge to M1.cs:\n{stdout}"
    );
    assert!(
        stdout.contains("src_Use_cs_6d2bbbab --> src_Models_M2_cs_3642425d"),
        "using Demo.Models must edge to M2.cs:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_self_namespace_using_no_self_edge() {
    let fixture = common::Fixture::new();
    fixture.write(
        "src/A.cs",
        "using Demo.App;\nnamespace Demo.App;\npublic class A { }\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/A.cs"),
        "node src/A.cs missing:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_A_cs_ff3d220b --> src_A_cs_ff3d220b"),
        "spurious self-edge for using own namespace:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_external_using_produces_no_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/A.cs", "namespace Demo.App;\npublic class A { }\n");
    fixture.write(
        "src/B.cs",
        "using System.Text;\nnamespace Demo.Other;\npublic class B { }\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/A.cs") && stdout.contains("src/B.cs"),
        "both files must be nodes:\n{stdout}"
    );
    assert!(
        !stdout.contains("src/B.cs -->"),
        "external using System.Text must not produce an edge:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_comment_and_string_using_no_edge() {
    let fixture = common::Fixture::new();
    fixture.write("src/A.cs", "namespace Demo.App;\npublic class A { }\n");
    fixture.write(
        "src/B.cs",
        "namespace Demo.Other;\n// using Demo.App;\n/* using Demo.App; */\nstring s = \"using Demo.App;\";\npublic class B { }\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/A.cs") && stdout.contains("src/B.cs"),
        "both files must be nodes:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_B_cs_7cf27ff1 --> src_A_cs_ff3d220b"),
        "using inside comment/string must not create an edge:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_using_before_file_scoped_namespace_resolves() {
    let fixture = common::Fixture::new();
    fixture.write("src/A.cs", "namespace Demo.App;\npublic class A { }\n");
    fixture.write(
        "src/B.cs",
        "using System;\nusing Demo.App;\nnamespace Demo.Other;\npublic class B { }\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_B_cs_7cf27ff1 --> src_A_cs_ff3d220b"),
        "using before the file-scoped namespace must still resolve:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_block_namespace_resolves() {
    let fixture = common::Fixture::new();
    fixture.write(
        "src/A.cs",
        "namespace Demo.App\n{\n    public class A { }\n}\n",
    );
    fixture.write(
        "src/B.cs",
        "namespace Demo.Other\n{\n    using Demo.App;\n    public class B { }\n}\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_B_cs_7cf27ff1 --> src_A_cs_ff3d220b"),
        "block-style namespace using must resolve:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_cross_folder_using_resolves() {
    let fixture = common::Fixture::new();
    fixture.write(
        "src/Shared/Models.cs",
        "namespace Demo.Shared;\npublic class Item { }\n",
    );
    fixture.write(
        "src/Web/Controller.cs",
        "using Demo.Shared;\nnamespace Demo.Web;\npublic class Controller { }\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src_Web_Controller_cs_2ebe0b7b --> src_Shared_Models_cs_f4789836"),
        "cross-folder using must resolve to the declaring file:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_excludes_bin_obj_and_hidden_dirs() {
    let fixture = csharp_scenario_fixture();
    fixture.write("bin/gen.cs", "namespace Demo.Gen;\npublic class Gen { }\n");
    fixture.write("obj/temp.cs", "namespace Demo.Temp;\npublic class Temp { }\n");
    fixture.write(".hidden/h.cs", "namespace Demo.Hidden;\npublic class H { }\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/A.cs") && stdout.contains("src/B.cs"),
        "real src files must remain nodes:\n{stdout}"
    );
    assert!(
        stdout.contains("src_B_cs_7cf27ff1 --> src_A_cs_ff3d220b"),
        "real src edge must remain:\n{stdout}"
    );
    assert!(
        !stdout.contains("bin/") && !stdout.contains("obj/") && !stdout.contains(".hidden"),
        "excluded paths must not appear as nodes or edges:\n{stdout}"
    );
    assert!(
        !stdout.contains("gen.cs") && !stdout.contains("temp.cs") && !stdout.contains("h.cs"),
        "no node from excluded dirs may appear:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

#[test]
fn inspect_csharp_rejects_missing_path() {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["inspect", "/no/such/dir"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path does not exist: /no/such/dir"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

#[test]
fn inspect_csharp_rejects_regular_file_path() {
    let fixture = common::Fixture::new();
    fixture.write("A.cs", "namespace Demo;\n");
    let output = fixture.run(&["inspect", "A.cs"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("path is not a directory: A.cs"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

#[test]
fn inspect_csharp_rejects_directory_without_supported_sources() {
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

#[test]
fn inspect_csharp_rejects_unsupported_format() {
    let fixture = csharp_scenario_fixture();
    let output = fixture.run(&["inspect", "--format", "bmp"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("unsupported format: bmp (supported: mermaid, plantuml)"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

#[test]
fn inspect_csharp_rejects_two_positional_paths() {
    let fixture = csharp_scenario_fixture();
    let output = fixture.run(&["inspect", "src", "tests"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("expected at most one path argument"));
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

#[test]
fn inspect_csharp_rejects_tree_with_only_excluded_sources() {
    let fixture = common::Fixture::new();
    fixture.write("bin/gen.cs", "namespace Demo.Gen;\npublic class Gen { }\n");
    fixture.write("obj/temp.cs", "namespace Demo.Temp;\npublic class Temp { }\n");
    let output = fixture.run(&["inspect"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("no C# sources found under:"),
        "stderr must say no C# sources found:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}