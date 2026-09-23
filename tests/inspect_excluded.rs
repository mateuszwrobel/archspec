mod common;

use common::{stderr, stdout};

/// Real `src/` sources plus a cargo `target/` OUT_DIR build artifact. The
/// artifact must never appear as a node and no edge may reference it.
#[test]
fn inspect_excludes_target_out_dir_build_artifacts() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/b.rs", "use crate::a::A;\n");
    fixture.write(
        "target/debug/build/some/out/generated.rs",
        "pub struct Generated;\n",
    );
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/lib.rs") && stdout.contains("src/a.rs") && stdout.contains("src/b.rs"),
        "real src files must remain nodes:\n{stdout}"
    );
    assert!(
        !stdout.contains("generated.rs"),
        "target OUT_DIR artifact must not appear:\n{stdout}"
    );
    assert!(
        !stdout.contains("target/"),
        "no node or edge may reference target/:\n{stdout}"
    );
    assert!(
        !stdout.contains("src_b_rs_a74680fd --> generated_rs_3a358bc5"),
        "no edge may resolve into an excluded target file:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// A `vendor/` directory containing vendored `.rs` files must not produce
/// nodes.
#[test]
fn inspect_excludes_vendor_directory() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("vendor/dep.rs", "pub struct Vendored;\n");
    fixture.write("vendor/dep/lib.rs", "pub struct Nested;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/lib.rs") && stdout.contains("src/a.rs"),
        "real src files must remain nodes:\n{stdout}"
    );
    assert!(
        !stdout.contains("vendor"),
        "vendored files must not appear as nodes:\n{stdout}"
    );
    assert!(
        !stdout.contains("dep.rs"),
        "no node from vendor/ may appear:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// Hidden (dot) directories such as `.git/` and `.cargo/` must not produce
/// nodes, even when they contain `.rs` files.
#[test]
fn inspect_excludes_hidden_dot_directories() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write(".git/objects/x.rs", "pub struct Git;\n");
    fixture.write(".cargo/config.rs", "pub struct Cargo;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/lib.rs") && stdout.contains("src/a.rs"),
        "real src files must remain nodes:\n{stdout}"
    );
    assert!(
        !stdout.contains(".git") && !stdout.contains(".cargo"),
        "hidden-dir files must not appear:\n{stdout}"
    );
    assert!(
        !stdout.contains("x.rs") && !stdout.contains("config.rs"),
        "no hidden-dir node may appear:\n{stdout}"
    );
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// A directory whose only `.rs` files live under excluded paths (here only a
/// `target/` dir remains) is rejected with the "no Rust sources found"
/// message, mirroring the #11 wording.
#[test]
fn inspect_rejects_crate_with_only_excluded_sources() {
    let fixture = common::Fixture::new();
    fixture.write("target/x.rs", "pub struct X;\n");
    fixture.write("target/debug/build/a/out/generated.rs", "pub struct G;\n");
    let output = fixture.run(&["inspect"]);

    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("no Rust sources found under:"),
        "stderr must say no Rust sources found:\n{}",
        stderr(&output)
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}

/// A crate with both a `target/` dir and real `src/` sources renders exactly
/// the real sources; the node set excludes `target/`.
#[test]
fn inspect_node_set_excludes_target_with_real_sources_present() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/b.rs", "use crate::a::A;\n");
    fixture.write("target/debug/build/a/out/generated.rs", "pub struct G;\n");
    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("src/lib.rs") && stdout.contains("src/a.rs") && stdout.contains("src/b.rs"),
        "real src nodes must be present:\n{stdout}"
    );
    assert!(
        !stdout.contains("generated.rs"),
        "target/ file must not be a node:\n{stdout}"
    );
    for node in ["src/lib.rs", "src/a.rs", "src/b.rs"] {
        assert!(
            stdout.contains(node),
            "node count must include real source {node}:\n{stdout}"
        );
    }
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// With excluded paths present, output stays deterministic across runs.
#[test]
fn inspect_with_excluded_paths_is_deterministic_across_runs() {
    let fixture = common::Fixture::new();
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write("src/a.rs", "pub struct A;\n");
    fixture.write("src/b.rs", "use crate::a::A;\n");
    fixture.write("target/debug/build/a/out/generated.rs", "pub struct G;\n");
    fixture.write("vendor/dep.rs", "pub struct V;\n");
    fixture.write(".git/hooks/x.rs", "pub struct H;\n");

    let first = fixture.run(&["inspect"]);
    let second = fixture.run(&["inspect"]);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "output with excluded paths must be byte-identical across runs"
    );
    assert!(
        !stdout(&first).contains("generated.rs")
            && !stdout(&first).contains("vendor")
            && !stdout(&first).contains(".git"),
        "excluded content must not leak into output:\n{}",
        stdout(&first)
    );
}

/// Go parity: the go file inspector and the go scan driver share one
/// exclusion predicate (`vendor`, `testdata`, dot-directories — the go
/// member list), so inspect's node set obeys exactly the driver's rules.
/// `vendorx` is NOT a go member (unlike the rust/c# vendor rules) and the
/// go driver scans it today — parity means keeping it, not "fixing" it.
#[test]
fn inspect_go_node_set_obeys_the_go_driver_predicate() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write("app/app.go", "package app\n\nfunc App() {}\n");
    fixture.write("vendor/dep/dep.go", "package dep\n\nfunc Dep() {}\n");
    fixture.write("testdata/fixture.go", "package testdata\n\nfunc F() {}\n");
    fixture.write(".hidden/x.go", "package x\n\nfunc X() {}\n");
    fixture.write("vendorx/keep.go", "package keep\n\nfunc K() {}\n");

    let output = fixture.run(&["inspect"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    let diagram = stdout(&output);
    assert!(
        diagram.contains("app/app.go") && diagram.contains("vendorx/keep.go"),
        "driver-visible files must remain nodes:\n{diagram}"
    );
    for excluded in ["dep.go", "fixture.go", "x.go", "testdata", ".hidden"] {
        assert!(
            !diagram.contains(excluded),
            "go-excluded content leaked ({excluded}):\n{diagram}"
        );
    }
    assert!(stderr(&output).is_empty(), "stderr should be empty");
}

/// A go tree whose only `.go` files sit under excluded paths is rejected
/// with the "no Go sources found" wording, mirroring the rust rule above.
#[test]
fn inspect_rejects_go_tree_with_only_excluded_sources() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/gotree\ngo 1.21\n");
    fixture.write("vendor/dep/dep.go", "package dep\n\nfunc Dep() {}\n");
    let output = fixture.run(&["inspect"]);

    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    // The language driver still finds a package (scan parity), so either
    // surface wording names Go and the empty source set honestly.
    assert!(
        err.contains("no Go sources found under:") || err.contains("no Go packages found under:"),
        "stderr must say no Go sources found:\n{err}"
    );
    assert!(stdout(&output).is_empty(), "no partial diagram on error");
}
