//! Doc-surface nits (fanout-output US 08): three facts that cost blind runs a
//! probe cycle each are stated once, in the help surface an agent reads before
//! hand-writing a spec or trusting a report — the rust `-bin` unit-naming
//! rule, the go naming legality decided from code (D10), and the report
//! diagram's granularity. Each scenario welds the sentence on the owner
//! surface to the binary's real behavior (scan/verify runs on fixtures), and
//! the uniqueness legs keep each registered sentence in exactly one surface —
//! a second echo anywhere fails as the unregistered drift the plan's registry
//! relation rules forbid.

mod common;
#[allow(dead_code)]
mod shared;

use common::{stderr, stdout, Fixture};
use shared::driver::{Driver, Language};
use std::fs;

/// Canonical rust naming sentence, whitespace-collapsed (the guard pins
/// wording, not line breaks).
const RUST_SENTENCE: &str = "a crate with `src/lib.rs` and `src/main.rs` and no explicit `[[bin]]` or `[lib]` section yields units `<pkg>` (kind=crate) and `<pkg>-bin` (kind=bin); `matches` must name those units exactly.";

/// Canonical go naming-legality sentence per the D10 probe transcript.
const GO_SENTENCE: &str = "`matches.units` targets must be full import paths — a short package name does not resolve there (verify then reports `unexpected component` / `missing component`); module `name` values and `allowed.depend_on` references may be short — references resolve to declared module names.";

/// Canonical report diagram-granularity sentence.
const REPORT_SENTENCE: &str = "The diagram embedded in a markdown report is file-granular; the module-granular graph is `archspec depgraph modules`.";

fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// stdout of `archspec help <topic-or-command>`, asserting the house contract
/// (exit 0, empty stderr).
fn help_surface(args: &[&str]) -> String {
    let fixture = Fixture::new();
    let output = fixture.run(args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`archspec {}` must exit 0: {}",
        args.join(" "),
        stderr(&output)
    );
    assert!(stderr(&output).is_empty(), "help must not write stderr");
    stdout(&output)
}

/// Every text surface an agent can read from the binary: the tool help, each
/// manual topic, each command's help (through `archspec help <cmd>` and the
/// raw `--help`), and the skill document.
fn rendered_surfaces() -> Vec<(String, String)> {
    let mut surfaces = vec![("help".to_string(), help_surface(&["help"]))];
    for topic in ["commands", "glob", "spec", "constraints", "languages", "roles", "workflow", "diagnostics"] {
        surfaces.push((format!("help {topic}"), help_surface(&["help", topic])));
    }
    for command in ["init", "scan", "diagram", "verify", "update", "report", "doctor", "inspect", "depgraph", "skill"] {
        surfaces.push((format!("help {command}"), help_surface(&["help", command])));
        surfaces.push((
            format!("{command} --help"),
            help_surface(&[command, "--help"]),
        ));
    }
    surfaces.push(("skill".to_string(), help_surface(&["skill"])));
    surfaces
}

/// Every markdown surface shipped in the payload (hand-written doc surfaces
/// the registry relation rules bind link-only).
fn payload_docs() -> Vec<(String, String)> {
    let mut files = vec![concat!(env!("CARGO_MANIFEST_DIR"), "/README.md").to_string()];
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/docs");
    let mut stack = vec![root.to_string()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read docs dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path.to_string_lossy().into_owned());
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                files.push(path.to_string_lossy().into_owned());
            }
        }
    }
    files
        .into_iter()
        .map(|path| {
            (
                path.replace(env!("CARGO_MANIFEST_DIR"), ""),
                fs::read_to_string(&path).expect("read payload doc"),
            )
        })
        .collect()
}

/// Scenario: rust bin units are named before `matches`.
#[test]
fn rust_bin_units_are_named_before_matches() {
    let out = help_surface(&["help", "languages"]);
    let rust_block = out
        .split("\ncsharp:")
        .next()
        .and_then(|head| head.split("\ngo:").next().map(str::to_owned))
        .expect("languages must have rust and csharp section boundaries");
    let flat = flatten(&rust_block);
    assert!(
        flat.contains(RUST_SENTENCE),
        "the rust section must state the <pkg>/<pkg>-bin unit-naming rule verbatim:\n{flat}"
    );
    // Slice-local D: naming precedes any scan-driven spec advice in the block.
    if let Some(advice) = flat.find("run `scan`") {
        assert!(
            flat.find(RUST_SENTENCE).expect("sentence present") < advice,
            "the naming sentence must precede scan-driven spec advice:\n{flat}"
        );
    }

    // Behavior leg: the sentence's exact names are what scan emits and what a
    // hand-written spec (from the sentence alone, no scan output read) must
    // name for verify to go green.
    let fx = Fixture::new();
    fx.write(
        "Cargo.toml",
        "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write("src/lib.rs", "pub mod store;\npub use store::Db;\n");
    fx.write("src/main.rs", "mod store;\nuse store::Db;\nfn main() { let _ = Db; }\n");
    fx.write("src/store.rs", "pub struct Db;\n");

    let scan = fx.run(&["scan"]);
    assert_eq!(scan.status.code(), Some(0), "scan failed: {}", stderr(&scan));
    let model: serde_json::Value = serde_json::from_str(&stdout(&scan)).expect("scan JSON");
    let units: Vec<(String, String)> = model["units"]
        .as_array()
        .expect("units array")
        .iter()
        .map(|u| {
            (
                u["name"].as_str().unwrap_or_default().to_string(),
                u["kind"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let mut sorted = units.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec![
            ("tool".to_string(), "crate".to_string()),
            ("tool-bin".to_string(), "bin".to_string()),
        ],
        "the lib+bin tree must yield exactly `<pkg>` (crate) and `<pkg>-bin` (bin)"
    );

    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"tool\"\nmatches = { units = [\"tool\", \"tool-bin\"] }\n",
    );
    let verify = fx.run(&["verify"]);
    let verify_out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "a spec naming the sentence's units from the help surface alone must verify green:\n{verify_out}{}",
        stderr(&verify)
    );
    assert!(verify_out.contains("ok:"), "verify must confirm: {verify_out}");
}

/// Scenario: go naming legality matches the binary (D10).
#[test]
fn go_naming_legality_matches_the_binary() {
    let out = help_surface(&["help", "languages"]);
    let go_block = flatten(out.split("\ngo:").nth(1).expect("languages must have a go section"));
    assert!(
        go_block.contains(GO_SENTENCE),
        "the go section must state the naming legality decided by the D10 probe:\n{go_block}"
    );

    let driver = Driver {
        language: Language::Go,
    };

    // Leg 1 — short names as `matches.units` targets do not resolve: verify
    // reports the two component categories the sentence names, per package.
    let fx = Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[module]]\nname = \"shared\"\nmatches = { units = [\"shared\"] }\n",
    );
    let verify = fx.run(&["verify"]);
    let verify_out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "a short-name spec must not verify green:\n{verify_out}"
    );
    for logical in ["app", "shared"] {
        let full = driver.unit_name(logical);
        assert!(
            verify_out.contains(&format!("missing component: {logical}")),
            "short target {logical:?} must surface as `missing component`:\n{verify_out}"
        );
        assert!(
            verify_out.contains(&format!("unexpected component: {full}")),
            "package {full:?} must surface as `unexpected component`:\n{verify_out}"
        );
    }

    // Leg 2 — full paths in `matches` + short module names + short
    // `depend_on` references: the green shape the sentence describes.
    let fx = Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    let app = driver.unit_name("app");
    let shared = driver.unit_name("shared");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"go\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{app}\"] }}\n[module.allowed]\ndepend_on = [\"shared\"]\n\n\
             [[module]]\nname = \"shared\"\nmatches = {{ units = [\"{shared}\"] }}\n"
        ),
    );
    let verify = fx.run(&["verify"]);
    let verify_out = stdout(&verify);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "full-path `matches` + short names + short `depend_on` must verify green:\n{verify_out}{}",
        stderr(&verify)
    );
    assert!(
        verify_out.contains("2 modules"),
        "the green line must count both short-named modules:\n{verify_out}"
    );
    let strict = fx.run(&["verify", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(0),
        "the same spec is green under --strict:\n{}",
        stdout(&strict)
    );
}

/// Scenario: report diagram granularity stated once.
#[test]
fn report_diagram_granularity_stated_once() {
    // The report's help surfaces own the line...
    for args in [
        vec!["report", "--help"],
        vec!["help", "report"],
    ] {
        let flat = flatten(&help_surface(&args));
        assert!(
            flat.contains(REPORT_SENTENCE),
            "`archspec {}` must state the diagram granularity:\n{flat}",
            args.join(" ")
        );
    }

    // ...and no surface adds a second granularity sentence. The pair check
    // (both granularity words in one surface) separates this claim from the
    // unrelated `file-granular` label of the inspect discovery map and the
    // `module-granularity` wording of the verify rules.
    for (name, text) in rendered_surfaces() {
        let flat = flatten(&text);
        if flat.contains("usage: archspec report") {
            // Surfaces embedding the report help (`help commands`) carry the
            // owner sentence with it — one owner, rendered in place.
            assert!(
                flat.contains(REPORT_SENTENCE),
                "{name} embeds the report help and must carry its granularity line"
            );
            continue;
        }
        assert!(
            !(flat.contains("file-granular") && flat.contains("module-granular")),
            "{name} must not carry its own diagram-granularity sentence:\n{flat}"
        );
        assert!(
            !flat.contains(REPORT_SENTENCE),
            "{name} must not echo the report granularity sentence:\n{flat}"
        );
    }
    for (path, text) in payload_docs() {
        let flat = flatten(&text);
        assert!(
            !flat.contains(REPORT_SENTENCE),
            "payload doc {path} must not restate the report granularity sentence"
        );
        assert!(
            !(flat.contains("file-granular") && flat.contains("module-granular")),
            "payload doc {path} must not add a second granularity sentence"
        );
    }

    // The go naming sentence lives only in `help languages` (link-only
    // elsewhere, per the registry relation rules).
    for (name, text) in rendered_surfaces() {
        if name == "help languages" {
            continue;
        }
        assert!(
            !flatten(&text).contains(GO_SENTENCE),
            "{name} must not echo the go naming sentence (link-only elsewhere)"
        );
    }
    for (path, text) in payload_docs() {
        assert!(
            !flatten(&text).contains(GO_SENTENCE),
            "payload doc {path} must not restate the go naming sentence"
        );
    }
}
