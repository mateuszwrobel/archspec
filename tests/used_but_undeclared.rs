//! GitHub issue #19 — a unit pair with file-tier code import evidence that
//! declares no edge reports the `used but undeclared` error with the
//! importing file as its loc, and never renders the zero-evidence
//! `(no import evidence for this pair)` reason: the reason asked the
//! exercised pair graph, which is built from manifest declarations only, so
//! a pair the code scan demonstrably crossed was answered "no evidence".
//! The classic line stays reserved for genuinely evidence-free pairs. The
//! fixtures use the ts workspace shape (hoisted symlinks) the issue report
//! ran: the bare specifier resolves through the workspace symlink while no
//! manifest declares the pair.

mod common;

use common::{stderr, stdout};
use std::path::Path;

fn link(from: &Path, to_dir: &Path) {
    std::fs::create_dir_all(from.parent().expect("link parent")).expect("link parent dir");
    std::os::unix::fs::symlink(to_dir, from).expect("symlink");
}

/// Two workspace packages, both hoisted into the root `node_modules`; `a`'s
/// code imports `b` while `a` declares no dependency on `b`. The report must
/// name the error class with the importing file as loc, and must not claim
/// the pair lacks evidence.
#[test]
fn used_but_undeclared_import_reports_error_with_evidence_loc() {
    let fixture = common::Fixture::new();
    fixture.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    fixture.write(
        "packages/a/package.json",
        "{\n  \"name\": \"a\",\n  \"version\": \"1.0.0\",\n  \"main\": \"src/index.ts\"\n}\n",
    );
    fixture.write(
        "packages/a/src/index.ts",
        "import { b } from 'b';\nexport const a = b;\n",
    );
    fixture.write(
        "packages/b/package.json",
        "{\n  \"name\": \"b\",\n  \"version\": \"1.0.0\",\n  \"main\": \"src/index.ts\"\n}\n",
    );
    fixture.write("packages/b/src/index.ts", "export const b = 1;\n");
    let packages = fixture.root.join("packages");
    link(&fixture.root.join("node_modules/a"), &packages.join("a"));
    link(&fixture.root.join("node_modules/b"), &packages.join("b"));
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\n",
    );

    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an undeclared used edge must fail (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("used but undeclared: a -> b (packages/a/src/index.ts)"),
        "code evidence with no declared edge must report the error class with the importing file:\n{out}"
    );
    assert!(
        !out.contains("no import evidence"),
        "a pair with code import evidence must not claim there is no evidence:\n{out}"
    );
}

/// The same evidenced pair under a `depend_on` declaration, beside a second
/// declared pair with no evidence at all: the evidenced pair loses the
/// zero-evidence reason (it is named by the error class); the genuinely
/// empty pair keeps the classic line.
#[test]
fn used_but_undeclared_suppresses_no_import_evidence_reason() {
    let fixture = common::Fixture::new();
    fixture.write(
        "package.json",
        "{\n  \"name\": \"mono\",\n  \"workspaces\": [\"packages/*\"]\n}\n",
    );
    fixture.write(
        "packages/a/package.json",
        "{\n  \"name\": \"a\",\n  \"version\": \"1.0.0\",\n  \"main\": \"src/index.ts\"\n}\n",
    );
    fixture.write(
        "packages/a/src/index.ts",
        "import { b } from 'b';\nexport const a = b;\n",
    );
    fixture.write(
        "packages/b/package.json",
        "{\n  \"name\": \"b\",\n  \"version\": \"1.0.0\",\n  \"main\": \"src/index.ts\"\n}\n",
    );
    fixture.write("packages/b/src/index.ts", "export const b = 1;\n");
    fixture.write(
        "packages/c/package.json",
        "{\n  \"name\": \"c\",\n  \"version\": \"1.0.0\",\n  \"main\": \"src/index.ts\"\n}\n",
    );
    fixture.write("packages/c/src/index.ts", "export const c = 1;\n");
    let packages = fixture.root.join("packages");
    for name in ["a", "b", "c"] {
        link(
            &fixture.root.join(format!("node_modules/{name}")),
            &packages.join(name),
        );
    }
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[module.allowed]\ndepend_on = [\"b\", \"c\"]\n\n[[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\n\n[[module]]\nname = \"c\"\nmatches = { units = [\"c\"] }\n",
    );

    let output = fixture.run(&["verify"]);
    let out = stdout(&output);
    assert!(
        out.contains("used but undeclared: a -> b (packages/a/src/index.ts)"),
        "the evidenced undeclared pair must be named by the error class:\n{out}"
    );
    assert!(
        !out.contains("missing edge: a -> b"),
        "an evidenced pair must never render the missing-edge zero-evidence line:\n{out}"
    );
    assert!(
        out.contains("missing edge: a -> c (no import evidence for this pair)"),
        "the genuinely evidence-free pair must keep the classic reason line:\n{out}"
    );
}
