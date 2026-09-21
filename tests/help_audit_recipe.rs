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
            let Some(close) = value_rest.find('"') else { break };
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
            fx.write(
                "architecture.spec.toml",
                &format!(
                    "[project]\nlanguage = \"{}\"\n\n\
                     [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\
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
