//! Prose-vs-behavior guard (audit-workflow contract): the audit recipe printed
//! by `archspec help workflow` names real commands, routes warning triage to the
//! diagnostics topic (the single home for warning meanings), states the
//! guard/discovery split, and pins `--strict` to "green means zero findings".
//! Every `[audit ...]` citation in the workflow prose names a command, a
//! conformance-corpus tree, and the exit code / finding category / warning the
//! run must actually produce — read through real command behavior on the same
//! trees the shared scenario corpus exercises (the clean / violating /
//! warning-only counterparts), never through a second list. A behavior change
//! that contradicts the recipe fails this suite.
//!
//! Fan-out half (fan-out plan US 02): the numbered recipe of the workflow topic
//! is the ONLY statement of the audit ordering. No arrow chain readable as a
//! step ordering appears in the topic or in any rendered help surface, no
//! payload doc restates the sequence in either the old arrow spelling or the
//! new numbered spelling (docs cite `archspec help workflow` instead), and the
//! onboarding flows row runs verbatim — `init`, `update --force`, `verify`,
//! every step exit 0.

mod common;
#[allow(dead_code)]
mod shared;

use shared::driver::{Driver, LogicalTree};
use shared::scenarios::boundary_spec;

/// One parsed `[audit ...]` citation: command + tree case + observables the
/// recipe claims the run produces.
struct AuditClaim {
    case: String,
    command: String,
    exit: i32,
    pass: Option<String>,
    category: Option<String>,
    warning: Option<String>,
}

/// Extract every `[audit name="value" ...]` citation from `text`. Same
/// deliberately dumb shape as the capability-citation parser: body up to the
/// first `]`, then `name="value"` pairs; values are quoted because two
/// observables contain separators.
fn parse_audit_citations(text: &str) -> Vec<AuditClaim> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[audit ") {
        let after = &rest[start + "[audit ".len()..];
        let Some(end) = after.find(']') else { break };
        let body = &after[..end];
        let mut fields: Vec<(String, String)> = Vec::new();
        let mut remainder = body;
        while let Some(eq) = remainder.find('=') {
            let name = remainder[..eq].trim().to_string();
            let Some(value_rest) = remainder[eq + 1..].trim_start().strip_prefix('"') else {
                break;
            };
            let Some(close) = value_rest.find('"') else {
                break;
            };
            fields.push((name, value_rest[..close].to_string()));
            remainder = &value_rest[close + 1..];
        }
        let get = |key: &str| {
            fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        };
        let Some(case) = get("case") else { break };
        let Some(command) = get("command") else { break };
        let Ok(exit) = get("exit").unwrap_or_default().parse::<i32>() else {
            break;
        };
        out.push(AuditClaim {
            case,
            command,
            exit,
            pass: get("pass"),
            category: get("category"),
            warning: get("warning"),
        });
        rest = &after[end..];
    }
    out
}

/// The workflow topic exactly as an agent reads it.
fn workflow_help() -> String {
    let fixture = common::Fixture::new();
    let output = fixture.run(&["help", "workflow"]);
    assert!(
        output.status.success(),
        "`archspec help workflow` must exit 0: {}",
        common::stderr(&output)
    );
    assert!(
        common::stderr(&output).is_empty(),
        "help workflow must not write stderr: {}",
        common::stderr(&output)
    );
    common::stdout(&output)
}

/// Whitespace-collapsed workflow text: needle checks match across line wraps
/// and column alignment, so the guard pins wording, not line breaks.
fn workflow_flat() -> String {
    workflow_help()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Materialize the corpus tree a citation names in a fresh fixture. `clean`
/// mirrors `unit_boundary_matching_tree_verifies_clean`, `ceiling` mirrors the
/// undeclared-edge half of `undeclared_dependency_exceeds_allowed_ceiling`, and
/// `warning` mirrors `dead_reference_to_source_module_classifies_per_language`.
fn build_case_tree(driver: &Driver, case: &str) -> common::Fixture {
    let fx = common::Fixture::new();
    match case {
        "clean" => {
            driver.materialize(&fx, &driver.probe_tree());
            fx.write("architecture.spec.toml", &boundary_spec(driver));
        }
        "ceiling" => {
            let mut tree = LogicalTree::new();
            tree.units = vec!["a".into(), "b".into()];
            tree.hard_edges.push(("a".into(), "b".into()));
            driver.materialize(&fx, &tree);
            let a = driver.unit_name("a");
            let b = driver.unit_name("b");
            fx.write(
                "architecture.spec.toml",
                &format!(
                    "[project]\nlanguage = \"{}\"\n\n\
                     [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\n\
                     [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n",
                    driver.language.as_str()
                ),
            );
        }
        "warning" => {
            let mut tree = LogicalTree::new();
            tree.units = vec!["a".into(), "b".into()];
            tree.hard_edges.push(("a".into(), "b".into()));
            tree.modules.insert("a".into(), vec!["core".into()]);
            driver.materialize(&fx, &tree);
            let a = driver.unit_name("a");
            let b = driver.unit_name("b");
            // Boundary `a` also claims core's module path: on go the nested
            // package is a unit addressed by its full import path, and an
            // unclaimed one would add an unassigned-unit error beside the dead
            // reference under test (mirrors the scenario in verify.rs). The
            // reference target stays the bare `core` — no declared module
            // carries that name, so the dead-reference claim holds.
            let a_core = driver.module_path("a", "core");
            fx.write(
                "architecture.spec.toml",
                &format!(
                    "[project]\nlanguage = \"{}\"\n\n\
                     [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"], modules = [\"{a_core}\"] }}\n\
                     [module.allowed]\ndepend_on = [\"b\", \"core\"]\n\n\
                     [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n",
                    driver.language.as_str()
                ),
            );
        }
        other => panic!("citation names unknown tree case {other:?}"),
    }
    fx
}

#[test]
fn audit_citations_match_real_command_behavior() {
    let citations = parse_audit_citations(&workflow_help());
    assert!(
        !citations.is_empty(),
        "the workflow topic must carry at least one [audit ...] citation — a \
         recipe step whose behavior is not pinned cannot be guarded"
    );
    for driver in Driver::all() {
        for citation in &citations {
            let fx = build_case_tree(&driver, &citation.case);
            let args: Vec<&str> = citation.command.split_whitespace().collect();
            let output = driver.run(&fx, &args);
            let stdout = common::stdout(&output);
            let stderr = common::stderr(&output);
            assert_eq!(
                output.status.code(),
                Some(citation.exit),
                "citation {:?} on {} must exit {}: stdout {stdout} stderr {stderr}",
                citation.command,
                driver.language.as_str(),
                citation.exit,
            );
            if let Some(pass) = &citation.pass {
                assert!(
                    stdout.contains(pass.as_str()),
                    "citation {:?} on {} must confirm {pass:?}:\n{stdout}",
                    citation.command,
                    driver.language.as_str(),
                );
            }
            if let Some(category) = &citation.category {
                assert!(
                    stdout.contains(category.as_str()),
                    "citation {:?} on {} must name the category {category:?}:\n{stdout}",
                    citation.command,
                    driver.language.as_str(),
                );
            }
            if let Some(warning) = &citation.warning {
                assert!(
                    stdout.contains(&format!("warning: {warning}")),
                    "citation {:?} on {} must carry the warning {warning:?}:\n{stdout}",
                    citation.command,
                    driver.language.as_str(),
                );
            }
        }
    }
}

/// The recipe must name commands that exist. Every `archspec <verb>` in the
/// workflow text is asserted to resolve: `archspec help <verb>` exits 0 (a real
/// command prints its help, a real topic prints the topic; a ghost command
/// errors). A step that sends an agent to a command the tool does not have
/// fails here.
#[test]
fn recipe_names_only_real_commands() {
    let out = workflow_help();
    let mut verbs: Vec<String> = Vec::new();
    let mut rest = out.as_str();
    while let Some(pos) = rest.find("archspec ") {
        let after = &rest[pos + "archspec ".len()..];
        let verb: String = after
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == '-')
            .collect();
        // "archspec" only ever precedes a command verb here; skip the empty run
        // (e.g. the literal usage phrase) and the `help` verb itself, whose
        // operands are topics/commands checked by other guards.
        if !verb.is_empty() && verb != "help" && !verbs.contains(&verb) {
            verbs.push(verb);
        }
        rest = after;
    }
    assert!(
        verbs.len() >= 5,
        "the audit recipe must reference several real commands, saw {verbs:?}"
    );
    let fixture = common::Fixture::new();
    for verb in &verbs {
        let output = fixture.run(&["help", verb]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "workflow references `archspec {verb}` but it is not a real command/topic"
        );
    }
}

#[test]
fn recipe_routes_warning_triage_to_diagnostics() {
    // Scenario "warnings are taught as findings, not noise": the workflow text
    // routes each warning to the diagnostics topic (the single authoritative
    // home) and labels them signal, without restating their definitions.
    let flat = workflow_flat();
    let lower = flat.to_lowercase();
    assert!(
        flat.contains("archspec help diagnostics"),
        "workflow must route warning triage to the diagnostics topic:\n{flat}"
    );
    for warning in ["vacuous", "dead reference", "unowned module edge endpoint"] {
        assert!(
            lower.contains(warning),
            "workflow must name the warning {warning:?} it routes to diagnostics:\n{flat}"
        );
    }
    assert!(
        lower.contains("signal") && lower.contains("noise"),
        "workflow must label warnings as audit signal, not noise:\n{flat}"
    );
    // The one engine-level fact that spans the warnings lives here; the
    // per-warning mechanics stay in diagnostics (no restatement).
    assert!(
        lower.contains("--strict") && lower.contains("warning"),
        "workflow must state that --strict turns warnings into failures:\n{flat}"
    );
    // No restatement of the diagnostics definitions: the workflow must not
    // re-print the diagnostics-only remedy prose for a warning it routes out.
    assert!(
        !lower.contains("checks nothing while appearing to pass"),
        "workflow must not restate the vacuous definition owned by diagnostics:\n{flat}"
    );
}

#[test]
fn recipe_states_guard_discovery_split() {
    // Scenario "the guard/discovery split is explicit": inspect surfaces
    // file-level hypotheses, verify enforces declared facts, and a validated
    // hypothesis with no expressing rule becomes a driver coverage gap.
    let flat = workflow_flat();
    let lower = flat.to_lowercase();
    assert!(
        lower.contains("inspect") && lower.contains("hypothes"),
        "workflow must explain inspect surfaces hypotheses:\n{flat}"
    );
    assert!(
        lower.contains("declared") && lower.contains("verify"),
        "workflow must state verify enforces declared facts:\n{flat}"
    );
    assert!(
        lower.contains("driver coverage gap"),
        "workflow must direct an unexpressible hypothesis to a driver coverage gap:\n{flat}"
    );
}

#[test]
fn recipe_is_numbered_and_ordered() {
    // Scenario "audit recipe is reachable and ordered": a numbered sequence
    // naming scan, depgraph modules, verify, the tightening step, the CI gate,
    // and the inspect-mining step, each present in the printed topic.
    let flat = workflow_flat();
    let lower = flat.to_lowercase();
    for step in ["1.", "2.", "3.", "4.", "5.", "6."] {
        assert!(
            flat.contains(step),
            "workflow recipe must be numbered (missing step {step}):\n{flat}"
        );
    }
    for needle in [
        "scan",
        "depgraph modules",
        "verify",
        "tighten",
        "verify --strict",
        "inspect",
    ] {
        assert!(
            lower.contains(needle),
            "workflow recipe must name the step {needle:?}:\n{flat}"
        );
    }
    assert!(
        lower.contains("--strict") && lower.contains("zero findings of any kind"),
        "workflow must state --strict green means zero findings of any kind:\n{flat}"
    );
}

// ---- fan-out plan US 02: the numbered recipe is the sole ordering owner ----

/// Recipe orderings as they could be restated: the old arrow chain (the bytes
/// of the workflow topic's retired "Command families" header) and the new
/// numbered sequence as a comma chain. The workflow topic's numbered steps are
/// the only place any of these may appear — in a rendered surface or a doc.
const RECIPE_CHAINS: &[&str] = &[
    "scan -> report",
    "scan → report",
    "report -> diagram",
    "report → diagram",
    "diagram -> verify",
    "diagram → verify",
    "scan, depgraph modules, verify",
    "depgraph modules, verify, tighten",
];

/// Whitespace-collapsed lowercase text: chain checks match across line wraps,
/// arrow spacing, and indentation, so the guard pins sequences, not layout.
fn flatten(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Every rendered surface an agent reads: the topic index, all topics, every
/// command's --help (which `help <command>` prints byte-identically).
fn rendered_surfaces() -> Vec<(String, String)> {
    let fixture = common::Fixture::new();
    let mut surfaces = vec![("help".to_string(), common::stdout(&fixture.run(&["help"])))];
    for topic in ["commands", "glob", "spec", "constraints", "languages", "roles", "workflow", "diagnostics"] {
        surfaces.push((
            format!("help {topic}"),
            common::stdout(&fixture.run(&["help", topic])),
        ));
    }
    for command in ["init", "scan", "diagram", "verify", "update", "report", "spec", "doctor", "inspect", "depgraph", "skill"] {
        surfaces.push((
            format!("{command} --help"),
            common::stdout(&fixture.run(&[command, "--help"])),
        ));
    }
    surfaces.push(("skill".to_string(), common::stdout(&fixture.run(&["skill"]))));
    surfaces
}

#[test]
fn workflow_topic_states_one_ordering() {
    // Scenario "the binary states one ordering": the numbered recipe keeps its
    // bytes as the only ordering statement, and the retired families header
    // reads as a taxonomy pointing at the numbered list below it.
    let out = workflow_help();
    for step in [
        "1. archspec scan",
        "2. archspec depgraph modules",
        "3. archspec verify",
        "4. tighten allowed.depend_on",
        "5. archspec verify --strict",
        "6. archspec inspect",
    ] {
        assert!(
            out.contains(step),
            "numbered recipe step {step:?} must keep its current bytes:\n{out}"
        );
    }
    assert!(
        !out.contains(" -> ") && !out.contains('→'),
        "the workflow topic must state no arrow chain at all:\n{out}"
    );
    let header_start = out.find("Command families:").expect("the families header must survive");
    let header = out[header_start..]
        .lines()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    let header = flatten(&header);
    for verb in ["scan", "report", "diagram", "verify"] {
        assert!(
            !header.contains(verb),
            "the families header is a taxonomy, not a step list; it names {verb:?}:\n{header}"
        );
    }
    assert!(
        header.contains("taxonomy") && header.contains("the numbered recipe below"),
        "the families header must read as a taxonomy pointing at the numbered \
         recipe below it:\n{header}"
    );
    for chain in RECIPE_CHAINS {
        assert!(
            !flatten(&out).contains(&flatten(chain)),
            "the workflow topic restates a second ordering {chain:?}"
        );
    }
}

#[test]
fn no_rendered_surface_and_no_doc_restates_the_recipe() {
    // Scenario "docs cite and never restate" — both halves. No rendered help
    // surface carries a chain outside the numbered steps (D09: "sole owner"
    // includes the binary's own surfaces), and no payload doc restates the
    // sequence in either spelling; the agents guide cites the owner topic.
    for (name, body) in rendered_surfaces() {
        let flat = flatten(&body);
        for chain in RECIPE_CHAINS {
            assert!(
                !flat.contains(&flatten(chain)),
                "{name} restates the recipe ordering {chain:?}"
            );
        }
    }
    let manifest = env!("CARGO_MANIFEST_DIR");
    fn sweep(dir: &std::path::Path, chains: &[&str], failures: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read docs dir") {
            let path = entry.expect("doc entry").path();
            if path.is_dir() {
                sweep(&path, chains, failures);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let flat = flatten(&std::fs::read_to_string(&path).expect("read doc"));
            for chain in chains {
                if flat.contains(&flatten(chain)) {
                    failures.push(format!("{}: restates {chain:?}", path.display()));
                }
            }
        }
    }
    let mut failures = Vec::new();
    sweep(
        &std::path::Path::new(manifest).join("docs/archspec"),
        RECIPE_CHAINS,
        &mut failures,
    );
    assert!(
        failures.is_empty(),
        "a payload doc restates an audit-step ordering (cite `archspec help \
         workflow` instead):\n{}",
        failures.join("\n")
    );

    let agents =
        std::fs::read_to_string(format!("{manifest}/docs/archspec/agents.md")).expect("read agents.md");
    assert!(
        flatten(&agents).contains("archspec help workflow"),
        "the agents guide must cite the recipe's owning topic"
    );
}

#[test]
fn onboarding_row_runs_verbatim() {
    // Scenario "the onboarding row runs verbatim": the flows row carries the
    // --force flag on its update step (init scaffolds a spec that plain
    // `update` refuses to overwrite), and running the row's commands in row
    // order on a fresh tree exits 0 for every step, on every driver.
    let manifest = env!("CARGO_MANIFEST_DIR");
    let flows = std::fs::read_to_string(format!(
        "{manifest}/docs/archspec/commands/verify/flows.md"
    ))
    .expect("read flows.md");
    let row = flows
        .lines()
        .find(|line| line.starts_with("| onboarding"))
        .expect("the flows doc keeps its onboarding row");
    let flat = flatten(row);
    let mut positions = Vec::new();
    for step in ["archspec init", "archspec update --force", "archspec verify"] {
        let at = flat
            .find(step)
            .unwrap_or_else(|| panic!("the onboarding row lost the step {step:?}:\n{row}"));
        positions.push(at);
    }
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "the onboarding row must name init, update --force, verify in row order:\n{row}"
    );

    for driver in Driver::all() {
        let fx = common::Fixture::new();
        driver.materialize(&fx, &driver.probe_tree());
        for (step, args) in [
            ("archspec init", ["init"].to_vec()),
            ("archspec update --force", vec!["update", "--force"]),
            ("archspec verify", vec!["verify"]),
        ] {
            let output = fx.run(&args);
            assert_eq!(
                output.status.code(),
                Some(0),
                "onboarding step {step:?} on {} must exit 0 (row runs verbatim): stderr {}",
                driver.language.as_str(),
                common::stderr(&output)
            );
        }
    }
}
