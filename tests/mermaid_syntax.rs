mod common;

/// Scenario: mermaid ids quoted everywhere. Given units/modules/files whose
/// ids carry `::`, `.` or `/` (c# namespaces, rust module paths, file paths),
/// every mermaid line the renderers emit must parse: bare identifiers may only
/// contain `[A-Za-z0-9_]`, and any raw name must sit inside a quoted label.
fn assert_parseable_mermaid(label: &str, text: &str) {
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let bare = strip_quoted(&strip_pipe_labels(trimmed));
        for token in bare.split_whitespace() {
            let token = token.trim_matches(&['[', ']', '(', ')', '{', '}'][..]);
            if token.is_empty() || KEYWORDS.contains(&token) {
                continue;
            }
            if token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue;
            }
            assert!(
                OPERATORS.contains(&token),
                "{label} line {}: unquoted, unparseable token {token:?} in {line:?}\nfull output:\n{text}",
                index + 1
            );
        }
    }
}

const KEYWORDS: &[&str] = &["graph", "TD", "subgraph", "end"];
const OPERATORS: &[&str] = &["-->", "-.->", "~~~", ">", "<"];

/// Remove `|edge label|` spans (pipe-delimited, may contain anything).
fn strip_pipe_labels(line: &str) -> String {
    let mut out = String::new();
    let mut in_label = false;
    for c in line.chars() {
        if c == '|' {
            in_label = !in_label;
        } else if !in_label {
            out.push(c);
        }
    }
    out
}

/// Remove quoted strings, honoring backslash escapes.
fn strip_quoted(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            loop {
                match chars.next() {
                    None => break,
                    Some('\\') => {
                        chars.next();
                    }
                    Some('"') => break,
                    Some(_) => {}
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn rust_project() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("app/src/lib.rs", "mod core;\nmod orchestration;\n");
    fixture.write("app/src/core.rs", "pub fn core() {}\n");
    fixture.write(
        "app/src/orchestration/mod.rs",
        "mod control_loop;\npub fn orchestration() {}\n",
    );
    fixture.write(
        "app/src/orchestration/control_loop.rs",
        "pub fn control_loop() {}\n",
    );
    fixture
}

fn csharp_project() -> common::Fixture {
    let fixture = common::Fixture::new();
    for (unit, ns) in [
        ("Shop.Api", "Shop.Api.Models"),
        ("Shop.Domain", "Shop.Domain.Entities"),
    ] {
        fixture.write(
            &format!("{unit}/{unit}.csproj"),
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
        );
        let path = ns.replace('.', "/");
        let leaf = ns.rsplit('.').next().unwrap();
        let usings = if unit.ends_with("Api") {
            "using Shop.Domain.Entities;\n"
        } else {
            ""
        };
        fixture.write(
            &format!("{unit}/{path}.cs"),
            &format!("{usings}namespace {ns};\npublic class {leaf} {{ }}\n"),
        );
    }
    fixture
}

/// The canonical single-module go tree: package `app` imports sibling
/// package `shared`. The units address themselves with `/` and the module
/// edges with `::`, so no soft-tier declaration covers an edge endpoint —
/// the shape audit defect D3 was found in.
fn go_project() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "app/app.go",
        "package app\n\nimport \"example.com/demo/shared\"\n\nfunc Run() { shared.Help() }\n",
    );
    fixture.write("shared/shared.go", "package shared\n\nfunc Help() {}\n");
    fixture
}

// Given file-level inspect on a rust tree (ids with `/` and `.`): the render
// boundary quotes every hostile id and the output still parses.
#[test]
fn file_level_rust_ids_render_parseable_mermaid() {
    let fixture = rust_project();
    let output = fixture.run(&["inspect", "app"]);
    assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
    let out = common::stdout(&output);
    assert!(
        out.contains(
            "src_orchestration_control_loop_rs_77bae0a4[\"src/orchestration/control_loop.rs\"]"
        ),
        "file ids must appear quoted under sanitized ids:\n{out}"
    );
    assert_parseable_mermaid("inspect rust file-level", &out);
}

// Given structural inspect on the same tree (`::` ids and subgraph titles):
// module paths and unit headers are quoted, ids sanitized, still parseable.
#[test]
fn structural_rust_module_paths_render_parseable_mermaid() {
    let fixture = rust_project();
    for mode in ["tree", "scanner"] {
        let output = fixture.run(&["inspect", mode, "app"]);
        assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
        let out = common::stdout(&output);
        assert!(
            out.contains(
                "app__orchestration__control_loop_7eaf07ca[\"app::orchestration::control_loop\"]"
            ),
            "module path ids must appear quoted under sanitized ids:\n{out}"
        );
        assert_parseable_mermaid(&format!("inspect {mode} rust"), &out);
    }
}

// Given a c# tree (unit names with `.`, namespaces folding to `::` module
// paths): file-level and both structural views render parseable mermaid.
#[test]
fn csharp_dotted_ids_render_parseable_mermaid() {
    let fixture = csharp_project();
    for args in [
        vec!["inspect", "."],
        vec!["inspect", "tree", "."],
        vec!["inspect", "scanner", "."],
    ] {
        let output = fixture.run(&args);
        assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
        let out = common::stdout(&output);
        assert!(
            out.contains("Shop_Api_9243f63f[\"Shop.Api\"]"),
            "unit ids must appear quoted under sanitized ids: {args:?}\n{out}"
        );
        assert_parseable_mermaid(&format!("csharp {args:?}"), &out);
    }
}

// Given a spec diagram with dotted module names: nodes and cluster headers are
// quoted at the render boundary and parse.
#[test]
fn spec_diagram_dotted_ids_render_parseable_mermaid() {
    let fixture = common::Fixture::new();
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "Shop.Api"
matches = { units = ["Shop.Api*"] }

[module.allowed]
depend_on = ["Shop.Domain"]
forbidden = []

[[module.submodules]]
name = "Shop.Api.Models"
matches = { units = ["Shop.Api.Models*"] }

[[module]]
name = "Shop.Domain"
matches = { units = ["Shop.Domain*"] }
"#,
    );
    let output = fixture.run(&["diagram"]);
    assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
    let out = common::stdout(&output);
    assert!(
        out.contains("Shop_Api_9243f63f[\"Shop.Api\"]"),
        "dotted module nodes must be quoted:\n{out}"
    );
    assert!(
        out.contains("subgraph Shop_Api_9243f63f[\"Shop.Api\"]"),
        "dotted cluster headers must be quoted:\n{out}"
    );
    assert!(
        out.contains("Shop_Api_9243f63f --> Shop_Domain_c40afbde"),
        "edges must reference sanitized ids:\n{out}"
    );
    assert_parseable_mermaid("spec diagram", &out);
}

/// Ids of nodes/subgraphs whose quoted label (or title) is exactly `label`.
fn ids_for_label(text: &str, label: &str) -> Vec<String> {
    let needle = format!("[\"{label}\"]");
    text.lines()
        .map(str::trim)
        .filter(|line| line.ends_with(&needle))
        .map(|line| {
            line[..line.len() - needle.len()]
                .trim()
                .trim_start_matches("subgraph ")
                .to_string()
        })
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .collect()
}

fn edge_lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| line.split_once(" --> "))
        .map(|(from, to)| (from.trim().to_string(), to.trim().to_string()))
        .collect()
}

fn assert_no_self_loops(label: &str, text: &str) {
    for (from, to) in edge_lines(text) {
        assert_ne!(
            from, to,
            "{label}: declared dependency collapsed into a self-loop {from:?} (sanitized ids collided):\n{text}"
        );
    }
}

// Scenario (f70 review probe): a file graph holding BOTH `src/a/b.rs` and
// `src/a_b.rs`. Pure sanitization merges them into one node id, so the two
// raw files must get distinct ids and every edge must attach to the id whose
// node carries that raw file's label.
#[test]
fn file_graph_slash_and_underscore_siblings_keep_distinct_ids() {
    let fixture = common::Fixture::new();
    fixture.write(
        "app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "app/src/lib.rs",
        "pub mod a;\npub mod a_b;\npub mod util;\npub mod other;\n",
    );
    fixture.write("app/src/a/mod.rs", "pub mod b;\n");
    fixture.write(
        "app/src/a/b.rs",
        "use crate::util::u;\npub fn b() { u(); }\n",
    );
    fixture.write(
        "app/src/a_b.rs",
        "use crate::other::o;\npub fn a_b() { o(); }\n",
    );
    fixture.write("app/src/util.rs", "pub fn u() {}\n");
    fixture.write("app/src/other.rs", "pub fn o() {}\n");
    let output = fixture.run(&["inspect", "app"]);
    assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
    let out = common::stdout(&output);

    let slash_ids = ids_for_label(&out, "src/a/b.rs");
    let under_ids = ids_for_label(&out, "src/a_b.rs");
    assert_eq!(
        slash_ids.len(),
        1,
        "src/a/b.rs node missing/duplicated:\n{out}"
    );
    assert_eq!(
        under_ids.len(),
        1,
        "src/a_b.rs node missing/duplicated:\n{out}"
    );
    let slash = &slash_ids[0];
    let under = &under_ids[0];
    assert_ne!(
        slash, under,
        "src/a/b.rs and src/a_b.rs must not merge into one mermaid id:\n{out}"
    );

    let util = &ids_for_label(&out, "src/util.rs")[0];
    let other = &ids_for_label(&out, "src/other.rs")[0];
    let edges = edge_lines(&out);
    assert!(
        edges.contains(&((slash.clone()), util.clone())),
        "edge from src/a/b.rs to src/util.rs must use the a/b.rs id:\n{out}"
    );
    assert!(
        edges.contains(&((under.clone()), other.clone())),
        "edge from src/a_b.rs to src/other.rs must use the a_b.rs id:\n{out}"
    );
    assert_no_self_loops("file graph collision", &out);
    assert_parseable_mermaid("file graph collision", &out);
}

// Scenario (f70 review probe): a spec declaring `A.B -> A_B`. Sanitizing
// `A.B` to `A_B` made the declared dependency render as `A_B --> A_B`, a
// self-loop; the ids must stay distinct so the edge keeps its direction.
#[test]
fn spec_diagram_dotted_and_underscored_names_do_not_self_loop() {
    let fixture = common::Fixture::new();
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "A.B"
matches = { units = ["a.b"] }

[module.allowed]
depend_on = ["A_B"]
forbidden = []

[[module]]
name = "A_B"
matches = { units = ["a_b"] }
"#,
    );
    let output = fixture.run(&["diagram"]);
    assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
    let out = common::stdout(&output);

    let dotted = &ids_for_label(&out, "A.B")[0];
    assert_ne!(
        dotted.as_str(),
        "A_B",
        "A.B must not collapse onto the bare id of the pure name A_B:\n{out}"
    );
    assert!(
        out.contains("  A_B\n"),
        "pure name A_B must keep its bare readable id:\n{out}"
    );
    assert!(
        out.contains(&format!("{dotted} --> A_B")),
        "declared edge A.B -> A_B must attach to the distinct A.B id:\n{out}"
    );
    assert_no_self_loops("spec diagram collision", &out);
    assert_parseable_mermaid("spec diagram collision", &out);
}

// Scenario (f70 review probe): sibling directories `src/db/migrations/` and
// `src/db_migrations/` produced two subgraph headers with the same id, so
// mermaid merged (or rejected) the clusters. The headers must get distinct
// ids and keep their raw titles.
#[test]
fn subgraph_sibling_directories_keep_distinct_ids() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod db;\npub mod db_migrations;\n");
    fixture.write("src/db/mod.rs", "pub mod migrations;\n");
    fixture.write("src/db/migrations/mod.rs", "pub fn migrate() {}\n");
    fixture.write("src/db/migrations/init.rs", "pub fn init() {}\n");
    fixture.write("src/db_migrations/mod.rs", "pub fn apply() {}\n");
    fixture.write("src/db_migrations/init.rs", "pub fn seed() {}\n");
    let output = fixture.run(&["inspect"]);
    assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
    let out = common::stdout(&output);

    let slashed = ids_for_label(&out, "src/db/migrations");
    let under = ids_for_label(&out, "src/db_migrations");
    assert_eq!(
        slashed.len(),
        1,
        "src/db/migrations subgraph header missing/duplicated:\n{out}"
    );
    assert_eq!(
        under.len(),
        1,
        "src/db_migrations subgraph header missing/duplicated:\n{out}"
    );
    assert_ne!(
        slashed[0], under[0],
        "sibling directories must not share one subgraph id:\n{out}"
    );
    assert_parseable_mermaid("subgraph collision", &out);
}

// Scenario (audit defect D3, workplan archspec_audit_defects US 03): the
// structural renders referenced ids like `…_admin_5a0de90b` in arrows
// without ever declaring them, so mermaid drew detached, unlabelled nodes
// disconnected from their subgraphs. Every id left of `-->` must already
// have appeared in a node or subgraph-header declaration. The property runs
// over the live structural views (both formats of model view: `tree` and
// `scanner`) and over the four frozen `inspect-tree` goldens, so neither a
// behaviour change nor a golden re-record can smuggle an undeclared id back
// in.
#[test]
fn structural_views_declare_every_referenced_id() {
    for (fixture, path) in [
        (rust_project(), "app"),
        (csharp_project(), "."),
        (go_project(), "."),
    ] {
        for mode in ["tree", "scanner"] {
            let output = fixture.run(&["inspect", mode, path]);
            assert_eq!(
                output.status.code(),
                Some(0),
                "`inspect {mode} {path}` must exit 0: {}",
                common::stderr(&output)
            );
            let out = common::stdout(&output);
            common::assert_declared_before_reference(&format!("inspect {mode} {path}"), &out);
        }
    }
}

#[test]
fn inspect_tree_goldens_declare_every_referenced_id() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens/inspect-tree");
    let mut goldens: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()))
        .map(|entry| entry.expect("golden entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "mmd"))
        .collect();
    goldens.sort();
    assert_eq!(
        goldens.len(),
        4,
        "the four inspect-tree goldens must all be present:\n{goldens:?}"
    );
    for path in goldens {
        let text = std::fs::read_to_string(&path).expect("golden readable");
        common::assert_declared_before_reference(&path.display().to_string(), &text);
    }
}

// Scenario (roles-views US 01, acceptance #28): scan-mode node lines carrying
// role markers must still parse — the marker rides the quoted label, so the
// id of a marked node equals the id the same node carries unmarked, every edge
// line is unchanged, and declaration-before-reference holds.
#[test]
fn scan_diagram_role_marks_parse_and_leave_ids_and_edges_unchanged() {
    let units_edges = "\"schema_version\": 1,\n  \"language\": \"csharp\",\n  \"units\": [\n    { \"name\": \"Shop.Api\", \"kind\": \"project\", \"path\": \"Shop.Api\" },\n    { \"name\": \"Shop.Domain\", \"kind\": \"project\", \"path\": \"Shop.Domain\" }\n  ],\n  \"edges\": [\n    { \"from\": \"Shop.Api\", \"to\": \"Shop.Domain\" },\n    { \"from\": \"Shop.Api\", \"to\": \"Newtonsoft.Json\" }\n  ],\n  \"external\": [\"Newtonsoft.Json\"]";
    let fixture = common::Fixture::new();
    fixture.write(
        "model-roles.json",
        &format!("{{\n  {units_edges},\n  \"roles\": {{ \"Shop::Api\": \"composition\", \"Shop.Domain\": \"facade\" }}\n}}\n"),
    );
    fixture.write("model-plain.json", &format!("{{\n  {units_edges}\n}}\n"));

    let marked = fixture.run(&["diagram", "--source", "scan", "model-roles.json"]);
    assert_eq!(marked.status.code(), Some(0), "{}", common::stderr(&marked));
    let plain = fixture.run(&["diagram", "--source", "scan", "model-plain.json"]);
    assert_eq!(plain.status.code(), Some(0), "{}", common::stderr(&plain));

    let marked = common::stdout(&marked);
    let plain = common::stdout(&plain);
    assert_parseable_mermaid("scan diagram with role marks", &marked);
    common::assert_declared_before_reference("scan diagram with role marks", &marked);
    assert_eq!(
        edge_lines(&marked),
        edge_lines(&plain),
        "markers must not touch a single edge line"
    );
    // The id a marked node carries is the id the unmarked render gives it.
    assert_eq!(
        ids_for_label(&marked, "Shop.Api [composition]"),
        ids_for_label(&plain, "Shop.Api"),
        "the marked node must keep the unmarked node's id"
    );
    assert_eq!(
        ids_for_label(&marked, "Shop.Domain [facade]"),
        ids_for_label(&plain, "Shop.Domain"),
        "the marked node must keep the unmarked node's id"
    );
    assert_eq!(
        ids_for_label(&marked, "Newtonsoft.Json"),
        ids_for_label(&plain, "Newtonsoft.Json"),
        "the unaddressed external node must render the same line as unmarked"
    );
}
