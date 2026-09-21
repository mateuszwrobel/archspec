mod common;

use common::{stderr, stdout};

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

fn edge(diagram: &str, from: &str, to: &str) -> String {
    let line = format!("{} --> {}", mermaid_id(from), mermaid_id(to));
    assert!(
        diagram.contains(&line),
        "missing edge {from} -> {to} ({line}):\n{diagram}"
    );
    line
}

/// Scenario: an import of an own-module package fans out to every file
/// declaring that package (the c# namespace-ownership shape).
#[test]
fn cross_package_import_fans_out_to_package_members() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write(
        "handler/handler.go",
        "package handler\n\nimport \"example.com/gotree/store\"\n\nfunc Handle() {}\n",
    );
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");
    fixture.write("store/helper.go", "package store\n\nfunc Help() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    assert!(diagram.contains("graph TD"), "missing graph TD:\n{diagram}");
    edge(&diagram, "handler/handler.go", "store/store.go");
    edge(&diagram, "handler/handler.go", "store/helper.go");
    assert!(
        diagram.contains("subgraph store"),
        "files must stay grouped by directory:\n{diagram}"
    );
    assert!(stderr(&output).is_empty());
}

/// Scenario: stdlib, third-party, cgo (`import \"C\"`) and `//go:embed`
/// produce no edges — external targets render nothing, the tokenizer skips
/// the embed comment, and the cgo marker is dropped by the scan tokenizer.
#[test]
fn foreign_embed_and_cgo_imports_render_no_edges() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write(
        "app/app.go",
        "package app\n\nimport (\n\t\"net/http\"\n\n\t\"github.com/prometheus/client_golang/prometheus\"\n\n\t\"C\"\n)\n\n//go:embed static\nvar dir string\n\nfunc App() { _ = http.DefaultServeMux; _ = prometheus.DefaultGauge }\n",
    );
    fixture.write("app/static.txt", "asset");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    assert!(diagram.contains("app/app.go"), "node missing:\n{diagram}");
    assert!(
        !diagram.contains("-->"),
        "stdlib/third-party/cgo/embed imports must render no edges:\n{diagram}"
    );
    assert!(!diagram.contains("github.com"), "external node leaked:\n{diagram}");
    assert!(stderr(&output).is_empty());
}

/// Scenario: `_test.go` files of both package forms are nodes with their
/// edges rendered — discovery includes tests, unlike the scan model.
#[test]
fn test_files_are_nodes_with_edges_in_both_package_forms() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write(
        "handler/handler_test.go",
        "package handler\n\nimport (\n\t\"testing\"\n\n\t\"example.com/gotree/upstream\"\n)\n\nfunc TestH(t *testing.T) { _ = upstream.Call }\n",
    );
    fixture.write(
        "handler/ext_test.go",
        "package handler_test\n\nimport (\n\t\"testing\"\n\n\t\"example.com/gotree/upstream\"\n)\n\nfunc TestX(t *testing.T) { _ = upstream.Call }\n",
    );
    fixture.write("upstream/upstream.go", "package upstream\n\nfunc Call() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    assert!(
        diagram.contains("handler/handler_test.go") && diagram.contains("handler/ext_test.go"),
        "test files must be nodes:\n{diagram}"
    );
    edge(&diagram, "handler/handler_test.go", "upstream/upstream.go");
    edge(&diagram, "handler/ext_test.go", "upstream/upstream.go");
    assert!(stderr(&output).is_empty());
}

/// Production imports target the directory's production files only: test
/// files of both package forms are never import targets (no production import
/// can compile against them), while staying nodes whose own imports render
/// as source-side edges.
#[test]
fn package_import_edges_never_target_the_directorys_test_files() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write(
        "cmd/cmd.go",
        "package cmd\n\nimport \"example.com/gotree/store\"\n\nfunc Run() {}\n",
    );
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");
    fixture.write(
        "store/store_test.go",
        "package store\n\nimport \"example.com/gotree/upstream\"\n\nfunc TestS(t *testing.T) { _ = upstream.Call }\n",
    );
    fixture.write(
        "store/external_test.go",
        "package store_test\n\nimport \"example.com/gotree/upstream\"\n\nfunc TestE(t *testing.T) { _ = upstream.Call }\n",
    );
    fixture.write("upstream/upstream.go", "package upstream\n\nfunc Call() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    edge(&diagram, "cmd/cmd.go", "store/store.go");
    for test_file in ["store/store_test.go", "store/external_test.go"] {
        assert!(
            diagram.contains(test_file),
            "test file must stay a node ({test_file}):\n{diagram}"
        );
        let targeted = format!("--> {}", mermaid_id(test_file));
        assert!(
            !diagram.contains(&targeted),
            "no production import may target a test file ({test_file}):\n{diagram}"
        );
    }
    edge(&diagram, "store/store_test.go", "upstream/upstream.go");
    edge(&diagram, "store/external_test.go", "upstream/upstream.go");
    assert!(stderr(&output).is_empty());
}

/// Scenario: a go.work tree resolves every member's module path — cross-
/// member and nested-member imports produce internal edges, and no member
/// root `go.mod` is required at the workspace root.
#[test]
fn workspace_cross_member_imports_are_internal() {
    let fixture = common::Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write(
        "api/api.go",
        "package api\n\nimport \"example.com/store\"\n\nfunc Api() {}\n",
    );
    fixture.write(
        "api/internal/handler/handler.go",
        "package handler\n\nimport \"example.com/store\"\n\nfunc Handle() {}\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write("store/store.go", "package store\n\nfunc Get() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    edge(&diagram, "api/api.go", "store/store.go");
    edge(&diagram, "api/internal/handler/handler.go", "store/store.go");
    assert!(stderr(&output).is_empty());
}

/// Scenario: exclusion parity — the node set obeys the go driver's predicate
/// through the shared function: `vendor/`, `testdata/` and dot-directories
/// invisible; `node_modules/` (which the go driver scans today) stays.
#[test]
fn exclusion_node_set_matches_the_go_driver_predicate() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write("app/app.go", "package app\n\nfunc App() {}\n");
    fixture.write("vendor/dep/dep.go", "package dep\n\nfunc Dep() {}\n");
    fixture.write("app/testdata/fixture.go", "package testdata\n\nfunc F() {}\n");
    fixture.write(".cache/x/x.go", "package x\n\nfunc X() {}\n");
    fixture.write("node_modules/pkg/pkg.go", "package pkg\n\nimport \"example.com/gotree/app\"\n\nfunc P() {}\n");
    fixture.write("vendorx/vendorx.go", "package vendorx\n\nimport \"example.com/gotree/app\"\n\nfunc V() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    assert!(diagram.contains("app/app.go"), "real file missing:\n{diagram}");
    for excluded in ["dep.go", "fixture.go", ".cache", "vendor/"] {
        assert!(
            !diagram.contains(excluded),
            "excluded content leaked ({excluded}):\n{diagram}"
        );
    }
    assert!(
        diagram.contains("node_modules/pkg/pkg.go"),
        "the go driver scans node_modules today, so inspect must too:\n{diagram}"
    );
    assert!(
        diagram.contains("vendorx/vendorx.go"),
        "vendorx is not a go exclusion member:\n{diagram}"
    );
    edge(&diagram, "node_modules/pkg/pkg.go", "app/app.go");
    edge(&diagram, "vendorx/vendorx.go", "app/app.go");
    assert!(stderr(&output).is_empty());
}

/// The root package of a module is importable under the bare module path:
/// its files own that import path.
#[test]
fn root_package_import_resolves_to_root_files() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write("gotree.go", "package gotree\n\nfunc Root() {}\n");
    fixture.write(
        "sub/sub.go",
        "package sub\n\nimport \"example.com/gotree\"\n\nfunc Sub() {}\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    edge(&diagram, "sub/sub.go", "gotree.go");
    assert!(stderr(&output).is_empty());
}

/// Importing one's own package is not expressible for non-test go files; a
/// test file must never gain a self-edge and a file with no internal imports
/// stays isolated.
#[test]
fn no_self_edges_and_in_package_files_need_no_edges() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write("solo/solo.go", "package solo\n\nfunc Solo() {}\n");
    fixture.write(
        "solo/solo_test.go",
        "package solo\n\nimport \"testing\"\n\nfunc TestSolo(t *testing.T) {}\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    assert!(
        !diagram.contains("-->"),
        "a package without internal imports must render no edges:\n{diagram}"
    );
    assert!(stderr(&output).is_empty());
}

/// Determinism: an unchanged go tree renders a byte-identical diagram.
#[test]
fn go_inspect_is_deterministic_across_runs() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write(
        "a/a.go",
        "package a\n\nimport \"example.com/gotree/b\"\n\nfunc A() {}\n",
    );
    fixture.write("b/b.go", "package b\n\nimport \"example.com/gotree/a\"\n\nfunc B() {}\n");
    let first = fixture.run(&["inspect"]);
    let second = fixture.run(&["inspect"]);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "mermaid output must be byte-identical across runs"
    );
}
