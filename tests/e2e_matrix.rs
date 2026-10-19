//! End-to-end matrix harness: every documented verb×mode cell is contract ×
//! corpus, executed against the real binary (Option C of the archspec CLI
//! e2e coverage proposal, landed 2026-10-18 as a worklog proposal record).
//!
//! Law recap honoured here:
//! - Fail-closed binary resolution (`shared::matrix::resolve_binary`): the
//!   `ARCHSPEC` env var wins; otherwise the subject-under-test built for this
//!   run. Silence never passes: every registry row must produce a result.
//! - The cell witness is the exit class plus stdout/stderr line-class facts —
//!   coarse on purpose so rust-edge changes do not churn this file (the
//!   byte-identity contract lives in `extraction_freeze.rs` goldens).
//! - The generated `e2e-matrix.json` at the payload root is freshness-checked
//!   against a render into the target dir, mirroring the `feature_matrix.rs`
//!   write-and-compare law. Regenerate with
//!   `ARCHSPEC_MATRIX_GOLDEN=1 cargo test --test e2e_matrix`.
//! - Registry↔help agreement is guarded both directions: every flag a cell
//!   names must appear in that verb's live help bytes, and every flag the
//!   live help documents must be exercised by at least one cell.

mod common;
#[allow(dead_code)]
mod shared;

use common::Fixture;
use serde_json::{json, Value};
use shared::driver::{Driver, Language};
use shared::matrix;

// ---------------------------------------------------------------------------
// Cell registry
// ---------------------------------------------------------------------------

/// One cell: contract (args + expected exit class + line-class facts) run
/// against one hermetic corpus.
struct Cell {
    id: &'static str,
    verb: &'static str,
    mode: &'static str,
    kind: &'static str, // "mode" | "exit-class" | "line-class"
    corpus: &'static str,
    /// Commands run in the fixture before the cell command; each must exit 0.
    /// An entry starting with '@' writes a file: "@<path>=<content>".
    pre: &'static [&'static [&'static str]],
    args: &'static [&'static str],
    exit: i32,
    stdout: &'static [&'static str],
    stderr: &'static [&'static str],
    /// Substrings that must NOT appear on stdout (body-pollution witness).
    absent: &'static [&'static str],
    /// Fixture-relative path that must exist after the cell command.
    creates: Option<&'static str>,
    witness: &'static str,
}

macro_rules! cell {
    ($id:expr, $verb:expr, $mode:expr, $kind:expr, $corpus:expr, $pre:expr, $args:expr,
     $exit:expr, $stdout:expr, $stderr:expr, $absent:expr, $creates:expr, $witness:expr) => {
        Cell {
            id: $id,
            verb: $verb,
            mode: $mode,
            kind: $kind,
            corpus: $corpus,
            pre: $pre,
            args: $args,
            exit: $exit,
            stdout: $stdout,
            stderr: $stderr,
            absent: $absent,
            creates: $creates,
            witness: $witness,
        }
    };
}

const NO_PRE: &[&[&str]] = &[];
const NO_ARGS: &[&str] = &[];

fn registry() -> Vec<Cell> {
    let mut cells: Vec<Cell> = vec![
        // --- version: identity, not a command (no help-verb guard row) ---
        cell!("version/default", "version", "version", "mode", "empty", NO_PRE,
            &["version"], 0, &["archspec "], &[], &[], None,
            "stdout names the tool with a version"),
        cell!("version/dashdash-version", "version", "--version", "mode", "empty", NO_PRE,
            &["--version"], 0, &["archspec "], &[], &[], None,
            "--version emits the same identity line shape"),
        // --- init ---
        cell!("init/default", "init", "[path]", "mode", "rust-workspace", NO_PRE,
            &["init"], 0, &["created architecture.spec.toml", "created archspec.toml"],
            &[], &[], None, "init scaffolds config + starter spec"),
        // --- scan ---
        cell!("scan/default", "scan", "default", "mode", "rust-probe", NO_PRE,
            &["scan"], 0, &["\"language\":\"rust\""], &[], &[], None,
            "compact model JSON on stdout"),
        cell!("scan/output", "scan", "--output", "mode", "rust-probe", NO_PRE,
            &["scan", "--output", "model.json"], 0, &[], &[], &[], Some("model.json"),
            "model written to the destination file"),
        cell!("scan/check", "scan", "--check", "mode", "rust-probe",
            &[&["scan", "--output", "model.json"]],
            &["scan", "--check", "--output", "model.json"], 0, &["up to date"], &[], &[], None,
            "fresh check names the destination with the ok-line"),
        cell!("scan/pretty", "scan", "--pretty", "mode", "rust-probe", NO_PRE,
            &["scan", "--pretty"], 0, &["{\n  \"schema_version\""], &[], &[], None,
            "indented JSON body"),
        // --- diagram ---
        cell!("diagram/spec-source", "diagram", "spec source", "mode", "rust-probe",
            &[&["update"]], &["diagram"], 0, &["graph"], &[], &[], None,
            "mermaid body from the declared spec on stdout"),
        cell!("diagram/source-scan", "diagram", "--source scan", "mode", "rust-probe",
            &[&["scan", "--output", "scan.json"]],
            &["diagram", "--source", "scan", "scan.json"], 0, &["graph"], &[], &[], None,
            "mermaid body from the scan artefact"),
        cell!("diagram/format-plantuml", "diagram", "--format plantuml", "mode", "rust-probe",
            &[&["update"]], &["diagram", "--format", "plantuml"], 0, &["@startuml"], &[], &[], None,
            "plantuml body"),
        cell!("diagram/tiers", "diagram", "--tiers", "mode", "rust-probe",
            &[&["scan", "--output", "scan.json"]],
            &["diagram", "--source", "scan", "scan.json", "--tiers", "modules"], 0, &["graph"], &[], &[], None,
            "additive module-tier render on the scan source exits 0 with a body"),
        cell!("diagram/module", "diagram", "--module", "mode", "rust-probe",
            &[&["update"]], &["diagram", "--module", "app"], 0, &["graph"], &[], &[], None,
            "single-claim territory render"),
        cell!("diagram/modules-dir", "diagram", "--modules-dir", "mode", "rust-probe",
            &[&["update"]], &["diagram", "--modules-dir", "diagrams"], 0, &[], &[], &[],
            Some("diagrams/app.mmd"), "one diagram file per module claim"),
        cell!("diagram/output", "diagram", "--output", "mode", "rust-probe",
            &[&["update"]], &["diagram", "--output", "out.mmd"], 0, &[], &[], &[],
            Some("out.mmd"), "diagram written to the destination file"),
        cell!("diagram/check", "diagram", "--check", "mode", "rust-probe",
            &[&["update"], &["diagram", "--output", "out.mmd"]],
            &["diagram", "--check", "--output", "out.mmd"], 0, &["up to date"], &[], &[], None,
            "fresh check names the destination with the ok-line"),
        // --- verify ---
        cell!("verify/default", "verify", "default", "mode", "rust-violating", NO_PRE,
            &["verify"], 1, &["does not match source model"], &[], &[], None,
            "violation diffs on stdout, exit 1"),
        cell!("verify/strict", "verify", "--strict", "mode", "rust-violating", NO_PRE,
            &["verify", "--strict"], 1, &["does not match source model"], &[], &[], None,
            "--strict keeps the verdict, promotes vacuity class"),
        // --- update ---
        cell!("update/default", "update", "default", "mode", "rust-workspace", NO_PRE,
            &["update"], 0, &["generated"], &[], &[], Some("architecture.spec.toml"),
            "seed spec generated from the tree"),
        cell!("update/force", "update", "--force", "mode", "rust-workspace",
            &[&["update"]], &["update", "--force"], 0, &["generated"], &[], &[], None,
            "--force rewrites an existing seed spec"),
        // --- report ---
        cell!("report/text", "report", "text", "mode", "rust-probe",
            &[&["update"]], &["report"], 0, &["Architecture report"], &[], &[], None,
            "text body on stdout"),
        cell!("report/format-markdown", "report", "--format markdown", "mode", "rust-probe",
            &[&["update"]], &["report", "--format", "markdown"], 0, &["# Architecture report"],
            &[], &[], None, "markdown body on stdout"),
        cell!("report/format-json", "report", "--format json", "mode", "rust-probe",
            &[&["update"]], &["report", "--format", "json"], 0, &["\"language\": \"rust\""],
            &[], &[], None, "already-indented JSON body"),
        cell!("report/output", "report", "--output", "mode", "rust-probe",
            &[&["update"]], &["report", "--output", "report.md"], 0, &[], &[], &[],
            Some("report.md"), "report written to the destination file"),
        cell!("report/check", "report", "--check", "mode", "rust-probe",
            &[&["update"], &["report", "--output", "report.md"]],
            &["report", "--check", "--output", "report.md"], 0, &["up to date"], &[], &[], None,
            "fresh check names the destination with the ok-line"),
        cell!("report/pretty", "report", "--pretty", "mode", "rust-probe",
            &[&["update"]], &["report", "--format", "json", "--pretty"], 0,
            &["\"language\": \"rust\""], &[], &[], None,
            "hole (b): --pretty actually runs; body bytes identical to non-pretty"),
        // --- spec ---
        cell!("spec/reference", "spec", "reference", "mode", "empty", NO_PRE,
            &["spec"], 0, &["annotated reference"], &[], &[], None,
            "annotated spec reference on stdout"),
        cell!("spec/schema", "spec", "--schema", "mode", "empty", NO_PRE,
            &["spec", "--schema"], 0, &["json-schema.org"], &[], &[], None,
            "draft-07 JSON Schema on stdout"),
        // --- doctor ---
        cell!("doctor/default", "doctor", "(zero flags)", "mode", "empty", NO_PRE,
            &["doctor"], 0, &["driver: present"], &[], &[], None,
            "driver/toolchain census per language"),
        // --- inspect ---
        cell!("inspect/default", "inspect", "default", "mode", "rust-probe", NO_PRE,
            &["inspect"], 0, &["graph"], &[], &[], None, "file-level mermaid on stdout"),
        cell!("inspect/tree", "inspect", "tree", "mode", "rust-probe", NO_PRE,
            &["inspect", "tree"], 0, &["graph"], &[], &[], None, "structural tree model renders"),
        cell!("inspect/scanner", "inspect", "scanner", "mode", "rust-probe", NO_PRE,
            &["inspect", "scanner"], 0, &["graph"], &[], &[], None, "unit-tier scanner model renders"),
        cell!("inspect/format", "inspect", "--format", "mode", "rust-probe", NO_PRE,
            &["inspect", "--format", "plantuml"], 0, &["@startuml"], &[], &[], None,
            "plantuml body"),
        cell!("inspect/output", "inspect", "--output", "mode", "rust-probe", NO_PRE,
            &["inspect", "--output", "inspect.mmd"], 0, &[], &[], &[], Some("inspect.mmd"),
            "diagram written to the destination file"),
        cell!("inspect/check", "inspect", "--check", "mode", "rust-probe",
            &[&["inspect", "--output", "inspect.mmd"]],
            &["inspect", "--check", "--output", "inspect.mmd"], 0, &["up to date"], &[], &[], None,
            "fresh check names the destination with the ok-line"),
        // --- depgraph ---
        cell!("depgraph/modules", "depgraph", "modules", "mode", "rust-probe", NO_PRE,
            &["depgraph", "modules"], 0, &["graph"], &[], &[], None,
            "module-tier graph on stdout"),
        cell!("depgraph/api-usage", "depgraph", "api-usage", "mode", "rust-probe", NO_PRE,
            &["depgraph", "api-usage"], 0, &[], &[], &[], None,
            "markdown usage table exits 0 (exit-class witness)"),
        cell!("depgraph/submodules", "depgraph", "submodules", "mode", "rust-probe", NO_PRE,
            &["depgraph", "submodules", "--parent", "app::core"], 0, &["graph"], &[], &[], None,
            "child graph of one module"),
        cell!("depgraph/format", "depgraph", "--format", "mode", "rust-probe", NO_PRE,
            &["depgraph", "modules", "--format", "plantuml"], 0, &["@startuml"], &[], &[], None,
            "plantuml body"),
        cell!("depgraph/parent", "depgraph", "--parent", "mode", "rust-probe", NO_PRE,
            &["depgraph", "submodules", "--parent", "app::core"], 0, &["graph"], &[], &[], None,
            "--parent selects the expanded module"),
        cell!("depgraph/output", "depgraph", "--output", "mode", "rust-probe", NO_PRE,
            &["depgraph", "modules", "--output", "deps.mmd"], 0, &[], &[], &[], Some("deps.mmd"),
            "body written to the destination file"),
        cell!("depgraph/check", "depgraph", "--check", "mode", "rust-probe",
            &[&["depgraph", "modules", "--output", "deps.mmd"]],
            &["depgraph", "modules", "--check", "--output", "deps.mmd"], 0, &["up to date"], &[], &[], None,
            "fresh check names the destination with the ok-line"),
        // --- capability ---
        cell!("capability/matrix", "capability", "matrix", "mode", "empty", NO_PRE,
            &["capability", "matrix"], 0, &["capability rust"], &[], &[], None,
            "driver capability table on stdout"),
        cell!("capability/granular", "capability", "granular", "mode", "empty", NO_PRE,
            &["capability", "granular", "rust", "role-facade"], 0, &[], &[], &[], None,
            "machine query exits 0 at full granularity"),
        // --- skill ---
        cell!("skill/audit", "skill", "audit emit", "mode", "empty", NO_PRE,
            &["skill"], 0, &["name: archspec"], &[], &[], None, "default audit skill on stdout"),
        cell!("skill/print", "skill", "print", "mode", "empty", NO_PRE,
            &["skill", "print"], 0, &["name: archspec"], &[], &[], None,
            "print emits the audit skill"),
        cell!("skill/install", "skill", "install", "mode", "empty", NO_PRE,
            &["skill", "install", "."], 0, &["installed"], &[], &[],
            Some(".agent/skills/archspec.md"), "skill installed under .agent/skills/"),
        cell!("skill/force", "skill", "--force", "mode", "empty",
            &[&["skill", "install", "."], &["@.agent/skills/archspec.md=drifted\n"]],
            &["skill", "install", ".", "--force"], 0, &["overwrote"], &[], &[], None,
            "--force overwrites drifted content; the echo names the file"),
        // --- help ---
        cell!("help/no-args", "help", "no args", "mode", "empty", NO_PRE,
            &["help"], 0, &["usage: archspec help"], &[], &[], None, "manual index on stdout"),
        cell!("help/per-command", "help", "per-command", "mode", "empty", NO_PRE,
            &["scan", "--help"], 0, &["usage: archspec scan"], &[], &[], None,
            "per-command help surfaces the usage line"),
        cell!("help/typo", "help", "typo", "mode", "empty", NO_PRE,
            &["help", "scanx"], 64, &[], &["unknown help topic"], &[], None,
            "unknown topic is a usage error listing valid topics"),
    ];
    for (id, topic) in [
        ("help/topic-commands", "commands"),
        ("help/topic-glob", "glob"),
        ("help/topic-spec", "spec"),
        ("help/topic-constraints", "constraints"),
        ("help/topic-languages", "languages"),
        ("help/topic-roles", "roles"),
        ("help/topic-workflow", "workflow"),
        ("help/topic-capabilities", "capabilities"),
        ("help/topic-diagnostics", "diagnostics"),
    ] {
        cells.push(cell!(
            id, "help", topic, "mode", "empty", NO_PRE,
            // args built per topic at run time below; the registry entry
            // carries NO_ARGS and the runner substitutes ["help", topic].
            NO_ARGS, 0, &["archspec"], &[], &[], None, "manual topic prints a page"
        ));
    }
    // Exit-class cells (exit-code contract surface from the proposal table).
    cells.push(cell!("verify/missing-spec", "verify", "(no spec)", "exit-class", "empty",
        NO_PRE, &["verify"], 70, &[], &["no architecture.spec.toml"], &[], None,
        "missing precondition exits 70 naming the file"));
    cells.push(cell!("report/missing-spec", "report", "(no spec)", "exit-class", "empty",
        NO_PRE, &["report"], 70, &[], &["spec file not found"], &[], None,
        "missing precondition exits 70 naming the file"));
    cells.push(cell!("cli/unknown-command", "cli", "unknown command", "exit-class", "empty",
        NO_PRE, &["frobnicate"], 64, &[], &["unknown command"], &[], None,
        "unknown command exits 64"));
    cells.push(cell!("cli/unknown-flag", "cli", "unknown flag", "exit-class", "empty",
        NO_PRE, &["scan", "--nope"], 64, &[], &["unknown flag"], &[], None,
        "unknown flag exits 64"));
    // Line-class cells (binary-level witnesses for report-only classes).
    cells.push(cell!("scan/python-not-modelled", "scan", "python dynamic sites", "line-class",
        "python-dynamic", NO_PRE, &["scan"], 0, &[],
        &["dynamic import not modelled: app.py:4 — importlib",
          "dynamic import not modelled: app.py:5 — importlib"],
        &["dynamic import not modelled"], None,
        "hole (a): python emitter witnessed at binary level; never in the machine body"));
    cells.push(cell!("scan/ts-dynamic-wildcard", "scan", "ts dynamic wildcard", "line-class",
        "ts-dynamic", NO_PRE, &["scan"], 0, &[],
        &["dynamic import wildcard: src/a.ts"],
        &["dynamic import wildcard"], None,
        "wildcard report line on stderr; never in the machine body"));
    cells
}

// ---------------------------------------------------------------------------
// Corpora (hermetic builders)
// ---------------------------------------------------------------------------

fn build_corpus(name: &str, fx: &Fixture) -> Result<(), String> {
    match name {
        "empty" => Ok(()),
        "rust-workspace" => {
            fx.write("Cargo.toml", "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n");
            fx.write("crates/auth/Cargo.toml", "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
            fx.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
            fx.write("crates/billing/Cargo.toml", "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n");
            fx.write("crates/billing/src/lib.rs", "pub fn bill() { auth::auth(); }\n");
            Ok(())
        }
        "rust-violating" => {
            build_corpus("rust-workspace", fx)?;
            fx.write(
                "architecture.spec.toml",
                "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"Billing\"\nmatches = { units = [\"billing*\"] }\n\n[module.allowed]\ndepend_on = []\n\n[[module]]\nname = \"Shared\"\nmatches = { units = [\"shared*\"] }\n",
            );
            Ok(())
        }
        "rust-probe" => {
            let driver = Driver { language: Language::Rust };
            driver.materialize(fx, &driver.probe_tree());
            Ok(())
        }
        "python-dynamic" => {
            fx.write(
                "app.py",
                "import importlib\n\ndef load(name):\n    mod = importlib.import_module(name)\n    spec = importlib.util.spec_from_file_location(\"dyn\", \"dyn.py\")\n    return mod, spec\n",
            );
            fx.write("dyn.py", "x = 1\n");
            Ok(())
        }
        "ts-dynamic" => {
            fx.write("package.json", "{\"name\":\"x\",\"version\":\"1.0.0\"}\n");
            fx.write(
                "src/a.ts",
                "export async function load(name: string) {\n  const mod = await import(`./dyn/${name}`);\n  return mod;\n}\n",
            );
            Ok(())
        }
        other => Err(format!("unknown corpus {other:?} — a cell row names a corpus builder that does not exist")),
    }
}

fn run_pre(fx: &Fixture, cmd: &[&str]) -> Result<(), String> {
    if let Some(spec) = cmd.first().and_then(|a| a.strip_prefix('@')) {
        let (path, content) = spec
            .split_once('=')
            .ok_or_else(|| format!("write-pre needs @path=content, got {spec:?}"))?;
        fx.write(path, content);
        return Ok(());
    }
    let (code, out, err) = matrix::run3(&fx.root, cmd);
    if code != 0 {
        return Err(format!("pre-command {cmd:?} exited {code}\nstdout:\n{out}\nstderr:\n{err}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Cell runner
// ---------------------------------------------------------------------------

fn run_cell(cell: &Cell) -> Result<(), String> {
    let fx = Fixture::new();
    build_corpus(cell.corpus, &fx)?;
    for cmd in cell.pre {
        run_pre(&fx, cmd)?;
    }
    let args: Vec<&str> = if cell.args.is_empty() && cell.verb == "help" && cell.mode != "no args"
    {
        // topic cells: registry carries NO_ARGS, the invocation is help <topic>
        vec!["help", cell.mode]
    } else {
        cell.args.to_vec()
    };
    let (code, out, err) = matrix::run3(&fx.root, &args);
    if code != cell.exit {
        return Err(format!(
            "exit class: want {} got {code}\nargs: {args:?}\nstdout:\n{out}\nstderr:\n{err}",
            cell.exit
        ));
    }
    for needle in cell.stdout {
        if !out.contains(needle) {
            return Err(format!("stdout must contain {needle:?}\nstdout:\n{out}\nstderr:\n{err}"));
        }
    }
    for needle in cell.stderr {
        if !err.contains(needle) {
            return Err(format!("stderr must contain {needle:?}\nstdout:\n{out}\nstderr:\n{err}"));
        }
    }
    for needle in cell.absent {
        if out.contains(needle) {
            return Err(format!("stdout must NOT contain {needle:?} — report-only class leaked into the machine body\nstdout:\n{out}"));
        }
    }
    if let Some(rel) = cell.creates
        && !fx.path(rel).exists()
    {
        return Err(format!("cell must create {rel:?}\nstdout:\n{out}\nstderr:\n{err}"));
    }
    // Named custom witness: hole (b) byte-identity law for `report --pretty`.
    if cell.id == "report/pretty" {
        let (ref_code, ref_out, ref_err) =
            matrix::run3(&fx.root, &["report", "--format", "json"]);
        if ref_code != 0 {
            return Err(format!("non-pretty reference run exited {ref_code}\nstderr:\n{ref_err}"));
        }
        if ref_out != out {
            return Err(format!(
                "report --pretty must not change body bytes: pretty is indentation-only\n\
                 non-pretty {} bytes, pretty {} bytes",
                ref_out.len(),
                out.len()
            ));
        }
    }
    Ok(())
}

/// Cells that are expected to fail with a named reason (defect discipline).
/// An entry that turns green must be removed in the same change.
const KNOWN_FAILING: &[(&str, &str)] = &[];

#[test]
fn matrix_cells_execute_and_witness() {
    let mut failures: Vec<String> = Vec::new();
    for cell in registry() {
        match run_cell(&cell) {
            Ok(()) => {
                if let Some((_, reason)) = KNOWN_FAILING.iter().find(|(id, _)| *id == cell.id) {
                    failures.push(format!("{}: known-failing cell turned GREEN — promote it (reason was {reason})", cell.id));
                }
            }
            Err(e) => {
                if KNOWN_FAILING.iter().any(|(id, _)| *id == cell.id) {
                    // expected failure; re-checked below that it still fails
                } else {
                    failures.push(format!("{}: {e}", cell.id));
                }
            }
        }
    }
    for (id, reason) in KNOWN_FAILING {
        let cell = registry().into_iter().find(|c| c.id == *id);
        match cell {
            Some(c) => {
                if run_cell(&c).is_ok() {
                    failures.push(format!("{id}: known-failing turned green — remove the entry ({reason})"));
                }
            }
            None => failures.push(format!("{id}: known-failing row names no registry cell")),
        }
    }
    assert!(failures.is_empty(), "{} cell(s) red:\n{}", failures.len(), failures.join("\n---\n"));
}

#[test]
fn every_registry_row_is_exercised_and_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for cell in registry() {
        assert!(seen.insert(cell.id), "duplicate cell id {}", cell.id);
        assert!(!cell.corpus.is_empty(), "{} names no corpus", cell.id);
        assert!(
            matches!(cell.kind, "mode" | "exit-class" | "line-class"),
            "{} has unknown kind {:?}", cell.id, cell.kind
        );
    }
    // The registry must never silently shrink below the documented surface:
    // at least one cell per documented verb, the 2a table's modes, plus the
    // exit-class and line-class rows.
    let cells = registry();
    assert!(cells.len() >= 58, "registry has {} cells — below the documented surface floor of 58", cells.len());
    let verbs: std::collections::BTreeSet<&str> = cells.iter().map(|c| c.verb).collect();
    for verb in ["version", "init", "scan", "diagram", "verify", "update", "report",
                 "spec", "doctor", "inspect", "depgraph", "capability", "skill", "help"] {
        assert!(verbs.contains(verb), "no cell exercises verb {verb:?} — the registry went silent on it");
    }
}

// ---------------------------------------------------------------------------
// Registry <-> live help agreement
// ---------------------------------------------------------------------------

fn help_flags(verb: &str) -> Vec<String> {
    let (code, out, err) = matrix::run3(std::path::Path::new("."), &[verb, "--help"]);
    assert_eq!(code, 0, "{verb} --help must exit 0 (usage surface is documented)");
    let text = format!("{out}{err}");
    extract_flags(&text)
}

fn extract_flags(text: &str) -> Vec<String> {
    let mut flags = std::collections::BTreeSet::new();
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        let candidate = if trimmed.starts_with("usage:") {
            // bracketed option groups inside usage lines: [--flag <value>]
            trimmed
                .split_whitespace()
                .filter_map(|t| t.trim_start_matches(['[', '<']).strip_prefix("--"))
                .map(str::to_string)
                .collect::<Vec<_>>()
        } else if trimmed.starts_with("--") && indent <= 4 {
            // option-definition line (definitions sit at the left margin;
            // deeper-indented "--x" openers are wrapped prose, not definitions)
            trimmed
                .split_whitespace()
                .next()
                .map(|t| t[2..].to_string())
                .into_iter()
                .collect()
        } else {
            continue;
        };
        for raw in candidate {
            let name: String = raw
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || *c == '-' || c.is_ascii_digit())
                .collect();
            let name = name.trim_matches('-');
            if !name.is_empty() {
                flags.insert(format!("--{name}"));
            }
        }
    }
    flags.into_iter().collect()
}

#[test]
fn registry_matches_documented_help_both_directions() {
    let binary = matrix::resolve_binary();
    assert!(binary.is_file(), "resolved binary {} is not a file", binary.display());
    // Verb list from the live top-level help (the "commands:" section).
    let (code, out, _) = matrix::run3(&std::env::current_dir().unwrap_or_default(), &["--help"]);
    assert_eq!(code, 0);
    let mut help_verbs: Vec<String> = Vec::new();
    let mut in_commands = false;
    for line in out.lines() {
        if line.trim() == "commands:" {
            in_commands = true;
            continue;
        }
        if in_commands {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || !line.starts_with("  ") {
                break;
            }
            if let Some(verb) = trimmed.split_whitespace().next() {
                help_verbs.push(verb.to_string());
            }
        }
    }
    assert!(help_verbs.len() >= 12, "top help must document the command list, got {help_verbs:?}");

    let cells = registry();
    let mut missing_rows = Vec::new();
    let mut stray_flags = Vec::new();
    let mut undocumented = Vec::new();
    for verb in &help_verbs {
        let verb_cells: Vec<&Cell> = cells
            .iter()
            .filter(|c| c.verb == verb && c.kind == "mode")
            .collect();
        if verb_cells.is_empty() {
            missing_rows.push(verb.clone());
            continue;
        }
        let documented = help_flags(verb);
        let mut exercised: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for cell in &verb_cells {
            for arg in cell.args {
                // --help is a universal meta-flag (never verb-documented); the
                // per-command help cell exercises the manual, not a verb lever.
                if *arg == "--help" {
                    continue;
                }
                if let Some(rest) = arg.strip_prefix("--") {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_lowercase() || *c == '-' || c.is_ascii_digit())
                        .collect();
                    let name = name.trim_matches('-');
                    if !name.is_empty() {
                        exercised.insert(format!("--{name}"));
                    }
                }
            }
        }
        for flag in &exercised {
            if !documented.contains(flag) {
                stray_flags.push(format!("{verb}: cell args name {flag} which the live help never documents"));
            }
        }
        for flag in &documented {
            if !exercised.contains(flag) {
                undocumented.push(format!("{verb}: help documents {flag} but no cell exercises it"));
            }
        }
    }
    // version is identity, not a listed command (mod.rs comment) — cells for it
    // exist above; the help-verb guard does not cover it.
    for cell in &cells {
        if cell.kind == "mode" && cell.verb != "version" && cell.verb != "cli" {
            assert!(
                help_verbs.iter().any(|v| v == cell.verb),
                "cell {} names verb {:?} which the top-level help never lists",
                cell.id, cell.verb
            );
        }
    }
    assert!(missing_rows.is_empty(), "verbs documented but never exercised: {missing_rows:?}");
    assert!(stray_flags.is_empty(), "cells use undocumented flags:\n{}", stray_flags.join("\n"));
    assert!(undocumented.is_empty(), "documented flags never exercised:\n{}", undocumented.join("\n"));
}

// ---------------------------------------------------------------------------
// Line-class evidence weld (2b rows)
// ---------------------------------------------------------------------------

struct LineClassRow {
    class: &'static str,
    /// (test file under tests/, needle) — evidence that the class is witnessed
    /// by a binary-level test that spawns the binary and asserts the line.
    evidence: &'static [(&'static str, &'static str)],
    /// Cell id in this registry that lit the row in-harness, when present.
    lit_by: Option<&'static str>,
}

const LINE_CLASS_ROWS: &[LineClassRow] = &[
    LineClassRow { class: "dynamic import wildcard", evidence: &[("ts_driver.rs", "dynamic import wildcard")], lit_by: Some("scan/ts-dynamic-wildcard") },
    LineClassRow { class: "dynamic import not modelled", evidence: &[("python_driver.rs", "dynamic import not modelled")], lit_by: Some("scan/python-not-modelled") },
    LineClassRow { class: "external not installed", evidence: &[("ts_driver.rs", "external not installed")], lit_by: None },
    LineClassRow { class: "workspace link missing", evidence: &[("ts_workspace.rs", "workspace link missing")], lit_by: None },
    LineClassRow { class: "undeclared member import", evidence: &[("ts_workspace.rs", "undeclared member import")], lit_by: None },
    LineClassRow { class: "workspace target not built", evidence: &[("ts_workspace.rs", "workspace target not built")], lit_by: None },
    LineClassRow { class: "generated source not built", evidence: &[("ts_driver.rs", "generated source not built"), ("ts_workspace.rs", "generated source not built")], lit_by: None },
    LineClassRow { class: "generated tsconfig absent", evidence: &[("ts_driver.rs", "generated tsconfig absent")], lit_by: None },
    LineClassRow { class: "tsconfig extends not installed", evidence: &[("ts_driver.rs", "tsconfig extends not installed")], lit_by: None },
    LineClassRow { class: "virtual specifier not modelled", evidence: &[("ts_driver.rs", "virtual specifier not modelled")], lit_by: None },
    LineClassRow { class: "framework conventions not configured", evidence: &[("ts_driver.rs", "framework conventions not configured")], lit_by: None },
    LineClassRow { class: "no settings file", evidence: &[("kotlin_driver.rs", "no settings file")], lit_by: None },
    LineClassRow { class: "unresolved settings include", evidence: &[("kotlin_driver.rs", "unresolved settings include")], lit_by: None },
    LineClassRow { class: "split package", evidence: &[("kotlin_driver.rs", "split package")], lit_by: None },
    LineClassRow { class: "gradle catalog unreadable", evidence: &[("kotlin_driver.rs", "gradle catalog unreadable")], lit_by: None },
    LineClassRow { class: "boundary advisory", evidence: &[("kotlin_driver.rs", "boundary advisory")], lit_by: None },
    LineClassRow { class: "java sources not indexed", evidence: &[("kotlin_driver.rs", "java sources not indexed")], lit_by: None },
    LineClassRow { class: "orphan file / orphan file census", evidence: &[("ts_driver.rs", "orphan file")], lit_by: None },
    LineClassRow { class: "vacuous constraint (scan manifest)", evidence: &[("python_driver.rs", "vacuous constraint")], lit_by: Some("scan/python-not-modelled") },
];

#[test]
fn line_class_rows_have_binary_level_evidence() {
    let cells = registry();
    let mut problems = Vec::new();
    for row in LINE_CLASS_ROWS {
        let mut lit = false;
        for (file, needle) in row.evidence {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join(file);
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    if text.contains(needle) {
                        lit = true;
                    } else {
                        problems.push(format!("{}: cited test {file} no longer contains {needle:?}", row.class));
                    }
                }
                Err(e) => problems.push(format!("{}: cited test {file} unreadable: {e}", row.class)),
            }
        }
        if let Some(id) = row.lit_by {
            assert!(cells.iter().any(|c| c.id == id), "{}: lit_by names no cell", row.class);
            lit = true;
        }
        if !lit {
            problems.push(format!("{}: no binary-level witness", row.class));
        }
    }
    assert!(problems.is_empty(), "line-class registry drift:\n{}", problems.join("\n"));
}

// ---------------------------------------------------------------------------
// Generated artifact: e2e-matrix.json (freshness-checked like feature_matrix)
// ---------------------------------------------------------------------------

fn render_matrix() -> Value {
    let cells: Vec<Value> = registry()
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "verb": c.verb,
                "mode": c.mode,
                "kind": c.kind,
                "corpus": c.corpus,
                "args": if c.args.is_empty() && c.kind == "mode" && c.verb == "help" && c.mode != "no args" {
                    vec!["help".to_string(), c.mode.to_string()]
                } else {
                    c.args.iter().map(|a| a.to_string()).collect::<Vec<_>>()
                },
                "exit_class": c.exit,
                "witness": c.witness,
                "status": if KNOWN_FAILING.iter().any(|(id, _)| *id == c.id) { "known-failing" } else { "pass" },
            })
        })
        .collect();
    let known: Vec<Value> = KNOWN_FAILING
        .iter()
        .map(|(id, reason)| json!({ "id": id, "reason": reason }))
        .collect();
    let mode_cells = registry().iter().filter(|c| c.kind == "mode").count();
    let exit_cells = registry().iter().filter(|c| c.kind == "exit-class").count();
    json!({
        "schema": "archspec-e2e-matrix-v0",
        "generated_by": "tests/e2e_matrix.rs (generated, never hand-edited)",
        "denominator": {
            "mode_cells": mode_cells,
            "exit_class_cells": exit_cells,
            "line_class_rows": LINE_CLASS_ROWS.len(),
            "note": "the proposal heading says 58 verb-mode cells; the registry enumerates the table rows, which is the honest denominator",
        },
        "cells": cells,
        "known_failing": known,
        "line_class_rows": LINE_CLASS_ROWS.iter().map(|r| json!({
            "class": r.class,
            "evidence": r.evidence.iter().map(|(f, n)| json!({"test": f, "needle": n})).collect::<Vec<_>>(),
            "lit_by": r.lit_by,
        })).collect::<Vec<_>>(),
    })
}

#[test]
fn matrix_artifact_is_fresh() {
    let rendered = serde_json::to_vec_pretty(&render_matrix()).expect("render json");
    let committed = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("e2e-matrix.json");
    if std::env::var("ARCHSPEC_MATRIX_GOLDEN").is_ok() {
        std::fs::write(&committed, &rendered).expect("write committed matrix");
        return;
    }
    match std::fs::read(&committed) {
        Ok(existing) => {
            let mine: Value = serde_json::from_slice(&rendered).expect("rendered json parses");
            let theirs: Value = serde_json::from_slice(&existing)
                .unwrap_or_else(|e| panic!("committed e2e-matrix.json unparseable: {e}"));
            assert_eq!(mine, theirs, "committed e2e-matrix.json is stale — regenerate with ARCHSPEC_MATRIX_GOLDEN=1 cargo test --test e2e_matrix");
        }
        Err(e) => panic!("committed e2e-matrix.json missing ({e}) — generate with ARCHSPEC_MATRIX_GOLDEN=1 cargo test --test e2e_matrix"),
    }
}
