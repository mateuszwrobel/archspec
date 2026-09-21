mod common;

use common::{stderr, stdout, Fixture};

const COMMANDS: &[(&str, &str)] = &[
    ("init", "archspec init"),
    ("scan", "archspec scan"),
    ("diagram", "archspec diagram"),
    ("verify", "archspec verify"),
    ("update", "archspec update"),
    ("report", "archspec report"),
    ("spec", "archspec spec"),
    ("doctor", "archspec doctor"),
    ("inspect", "archspec inspect"),
    ("depgraph", "archspec depgraph"),
    ("help", "archspec help"),
];

const SUMMARIES: &[&str] = &[
    "scaffold a minimal base spec",
    "extract the architecture model only",
    "render model (extracted or declared) to an artefact",
    "extract + compare vs spec, full diff, exit code",
    "snapshot current model as a seed spec",
    "text/markdown/json diff + metric output",
    "print the spec JSON schema or annotated reference",
    "diagnose which language drivers/toolchains are present",
    "zero-config file-level import map (discovery)",
    "current-state module/submodule/API-usage views of a project",
    "print the archspec built-in manual (topics + per-command help)",
];

const TOPICS: &[&str] = &[
    "commands",
    "glob",
    "spec",
    "constraints",
    "languages",
    "workflow",
    "diagnostics",
];

// Every finding category the `diagnostics` catalog must name. Maintained
// alongside `DIAGNOSTICS_HELP` in src/archspec/commands/help.rs: adding a
// finding category to verify/compare.rs requires a new entry here and a new
// block in the catalog (mirrors how help_constraints lists its seven types).
// The set below is the full emitted-category contract of both renderers —
// compare::render_report and report::diff_items — enforced structurally by
// `every_emitted_category_is_in_the_diagnostics_catalog` (src/archspec/report).
const DIAGNOSTIC_CATEGORIES: &[&str] = &[
    "ambiguous module match",
    "contract leak",
    "cycle",
    "cycle detected in",
    "dead reference",
    "disallowed cross-component dependency",
    "empty glob export",
    "facade dependency",
    "feature boundary",
    "forbidden edge",
    "forbidden edge present",
    "forbidden external crate",
    "forbidden submodule dependency",
    "laundered forbidden edge",
    "manifest integrity",
    "missing component",
    "missing edge",
    "no_cycles",
    "public api leak",
    "unassigned unit",
    "unowned module edge endpoint",
    "unexpected component",
    "unresolved module file",
    "unverifiable glob export",
    "vacuous constraint",
];

#[test]
fn no_args_prints_command_list() {
    let fixture = Fixture::new();
    let output = fixture.run(&[]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    for (name, _) in COMMANDS {
        assert!(
            out.contains(name),
            "command list must contain {name}:\n{out}"
        );
    }
    assert!(
        stderr(&output).is_empty(),
        "no stderr on help output"
    );
}

#[test]
fn help_flag_prints_command_list() {
    let fixture = Fixture::new();
    let output = fixture.run(&["--help"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(out.contains("usage:"), "must show usage line:\n{out}");
    for (name, _) in COMMANDS {
        assert!(out.contains(name), "command list must contain {name}:\n{out}");
    }
    for summary in SUMMARIES {
        assert!(
            out.contains(summary),
            "command list must contain purpose \"{summary}\":\n{out}"
        );
    }
    assert!(stderr(&output).is_empty(), "no stderr on help output");
}

#[test]
fn each_command_help_prints_usage() {
    let fixture = Fixture::new();
    for (name, usage) in COMMANDS {
        let output = fixture.run(&[name, "--help"]);

        assert_eq!(
            output.status.code(),
            Some(0),
            "{name} --help must exit 0"
        );
        let out = stdout(&output);
        assert!(
            out.contains(usage),
            "{name} --help must contain usage line \"{usage}\":\n{out}"
        );
        assert!(
            stderr(&output).is_empty(),
            "{name} --help must not write stderr"
        );
    }
}

#[test]
fn help_anywhere_in_args_wins() {
    let fixture = Fixture::new();
    let output = fixture.run(&["diagram", "--format", "mermaid", "--help"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).contains("archspec diagram"),
        "must print diagram help:\n{}",
        stdout(&output)
    );
}

#[test]
fn help_overrides_flag_validation() {
    let fixture = Fixture::new();
    let output = fixture.run(&["doctor", "--help"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).contains("archspec doctor"),
        "doctor --help must print help despite doctor rejecting flags:\n{}",
        stdout(&output)
    );
}

#[test]
fn unknown_command_errors_with_hint() {
    let fixture = Fixture::new();
    let output = fixture.run(&["bogus"]);

    assert_eq!(output.status.code(), Some(1));
    let err = stderr(&output);
    assert!(
        err.contains("error: unknown command: bogus"),
        "must name the command:\n{err}"
    );
    assert!(
        err.contains("run 'archspec --help' for the command list"),
        "must hint at --help:\n{err}"
    );
}

#[test]
fn unknown_command_with_help_errors() {
    let fixture = Fixture::new();
    let output = fixture.run(&["bogus", "--help"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("error: unknown command: bogus"),
        "unknown command must still error:\n{}",
        stderr(&output)
    );
}

// ---- Feature: `archspec help` (G1–G13) --------------------------------------

#[test]
fn help_lists_topics_and_commands() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("usage: archspec help"),
        "must show the usage line:\n{out}"
    );
    for topic in TOPICS {
        assert!(
            out.contains(topic),
            "help must list topic {topic}:\n{out}"
        );
    }
    for (name, _) in COMMANDS {
        assert!(
            out.contains(name),
            "help must list command {name}:\n{out}"
        );
    }
    assert!(
        stderr(&output).is_empty(),
        "no stderr on help output:\n{}",
        stderr(&output)
    );
}

#[test]
fn help_topics_is_byte_identical_to_help() {
    let fixture = Fixture::new();
    let help_out = stdout(&fixture.run(&["help"]));
    let topics_out = stdout(&fixture.run(&["help", "topics"]));

    assert_eq!(help_out, topics_out, "help topics must equal help");
}

#[test]
fn help_commands_names_commands_with_purpose_and_flags() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "commands"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    for (name, _) in COMMANDS {
        assert!(
            out.contains(name),
            "help commands must name {name}:\n{out}"
        );
    }
    for summary in SUMMARIES {
        assert!(
            out.contains(summary),
            "help commands must state purpose \"{summary}\":\n{out}"
        );
    }
    for flag in ["--strict", "--output", "--format", "--schema"] {
        assert!(
            out.contains(flag),
            "help commands must mention {flag}:\n{out}"
        );
    }
    assert!(
        stderr(&output).is_empty(),
        "no stderr on help commands:\n{}",
        stderr(&output)
    );
}

#[test]
fn help_glob_documents_semantics() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "glob"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("INCLUDING separators") && out.contains("'.'")
            && out.contains("'/'") && out.contains("'::'") && out.contains("'-'"),
        "glob help must state separators match:\n{out}"
    );
    assert!(
        out.contains("case-sensitive") && out.contains("no regex"),
        "glob help must state case-sensitive, no regex:\n{out}"
    );
    assert!(
        out.contains("full unit names"),
        "unit globs match full unit names:\n{out}"
    );
    assert!(
        out.contains("full dotted path")
            && out.contains("bare last segment")
            && out.contains("unit prefix stripped"),
        "module globs match full path, bare last segment, or unit-stripped path:\n{out}"
    );
    assert!(
        out.contains("subtree") && out.contains("descendants"),
        "matches.modules must be a subtree match:\n{out}"
    );
    for key in ["'from'", "'forbid'", "'gated_modules'", "'allowed_from'", "'parent'"] {
        assert!(
            out.contains(key),
            "glob help must cover key {key}:\n{out}"
        );
    }
    assert!(
        out.contains("EXACT declared-module-name matches"),
        "allowed.depend_on/forbidden and no_cycles.modules are exact names:\n{out}"
    );
    assert!(
        stderr(&output).is_empty(),
        "no stderr on help glob:\n{}",
        stderr(&output)
    );
}

#[test]
fn help_spec_prints_share_reference_material() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "spec"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    for marker in [
        "architecture.spec.toml",
        "[project]",
        "[[module]]",
        "[module.allowed]",
        "[[constraint]]",
    ] {
        assert!(
            out.contains(marker),
            "help spec must contain {marker}:\n{out}"
        );
    }
    for language in ["rust", "csharp", "go"] {
        assert!(
            out.contains(language),
            "help spec must annotate language {language}:\n{out}"
        );
    }
}

#[test]
fn help_constraints_names_all_seven_types_with_keys() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "constraints"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    let expected = [
        ("no_cycles", "modules, severity"),
        ("public_api_allowlist", "allowed"),
        ("forbid_external_crates", "from, forbid"),
        ("manifest_integrity", "require_publish, forbidden_dependencies, required_features"),
        ("feature_boundary", "feature, gated_modules, allowed_from"),
        ("forbid_submodule_dependency", "parent, from, forbid"),
        ("external_free", "from"),
    ];
    for (kind, keys) in expected {
        assert!(
            out.contains(kind),
            "help constraints must name {kind}:\n{out}"
        );
        assert!(
            out.contains(keys),
            "help constraints must list keys \"{keys}\" for {kind}:\n{out}"
        );
    }
    assert!(
        out.contains("error") && out.contains("warning") && out.contains("--strict"),
        "severity default, warning, and --strict promotion must be documented:\n{out}"
    );
}

#[test]
fn help_languages_one_block_per_language() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "languages"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    for language in ["rust", "csharp", "go"] {
        assert!(
            out.contains(&format!("{language}:")),
            "help languages must have a block for {language}:\n{out}"
        );
    }
    for tier in [
        "units",
        "soft_structure",
        "module_edges",
        "external",
        "module_external",
        "unit_manifests",
        "root_public_exports",
        "root_glob_exports",
        "root_module_declarations",
    ] {
        assert!(
            out.contains(tier),
            "help languages must mention tier {tier}:\n{out}"
        );
    }
    assert!(
        stderr(&output).is_empty(),
        "no stderr on help languages:\n{}",
        stderr(&output)
    );
}

#[test]
fn help_workflow_mentions_recipe_and_support_commands() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "workflow"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("scan -> report -> diagram -> verify"),
        "workflow must state the recipe:\n{out}"
    );
    for command in ["scan", "report", "diagram", "verify", "doctor", "init", "update"] {
        assert!(
            out.contains(command),
            "workflow must cover {command}:\n{out}"
        );
    }
    assert!(
        out.contains("scan") && out.contains("report") && out.contains("diagram") && out.contains("verify"),
        "workflow must state each recipe command's purpose:\n{out}"
    );
    assert!(
        out.contains("--strict") && out.contains("CI gate"),
        "workflow must present --strict as the CI gate:\n{out}"
    );
}

#[test]
fn help_command_matches_command_help() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "verify"]);
    let direct = fixture.run(&["verify", "--help"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        stdout(&direct),
        "help verify must be byte-identical to verify --help"
    );
}

#[test]
fn help_spec_is_byte_identical_to_spec_output() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "spec"]);
    let direct = fixture.run(&["spec"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        stdout(&direct),
        "help spec must be byte-identical to spec output"
    );
}

#[test]
fn help_dash_dash_help_prints_topic_index() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "--help"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("usage: archspec help"),
        "help --help must show the usage line:\n{out}"
    );
    for topic in TOPICS {
        assert!(
            out.contains(topic),
            "help --help must print the topic index ({topic}):\n{out}"
        );
    }
    assert!(
        stderr(&output).is_empty(),
        "no stderr on help --help:\n{}",
        stderr(&output)
    );
}

#[test]
fn help_bogus_errors_naming_topic_and_valid_topics() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "bogus"]);

    assert_eq!(output.status.code(), Some(1));
    let err = stderr(&output);
    assert!(
        err.contains("bogus"),
        "must name the unknown topic:\n{err}"
    );
    for topic in TOPICS {
        assert!(
            err.contains(topic),
            "must list valid topic {topic}:\n{err}"
        );
    }
    assert!(
        err.contains("run 'archspec help'"),
        "must hint at run 'archspec help':\n{err}"
    );
}

#[test]
fn help_spec_is_deterministic() {
    let a = Fixture::new();
    let b = Fixture::new();
    assert_eq!(
        stdout(&a.run(&["help", "spec"])),
        stdout(&b.run(&["help", "spec"])),
        "help spec must be byte-identical across runs"
    );
}

#[test]
fn help_glob_is_deterministic() {
    let a = Fixture::new();
    let b = Fixture::new();
    assert_eq!(
        stdout(&a.run(&["help", "glob"])),
        stdout(&b.run(&["help", "glob"])),
        "help glob must be byte-identical across runs"
    );
}

// ---- Feature: `archspec help diagnostics` (f22) ------------------------------

#[test]
fn help_diagnostics_lists_every_category() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "diagnostics"]);

    assert_eq!(output.status.code(), Some(0), "help diagnostics must exit 0");
    let out = stdout(&output);
    assert!(!out.trim().is_empty(), "catalog must not be empty:\n{out}");
    for category in DIAGNOSTIC_CATEGORIES {
        assert!(
            out.contains(category),
            "diagnostics catalog must name category \"{category}\":\n{out}"
        );
    }
    assert!(stderr(&output).is_empty(), "no stderr on help diagnostics:\n{}", stderr(&output));
}

#[test]
fn help_diagnostics_carries_verbatim_wordings() {
    let fixture = Fixture::new();
    let out = stdout(&fixture.run(&["help", "diagnostics"]));
    // Exact message fragments emitted by verify/compare.rs — the catalog must
    // quote them, not paraphrase (guards against drift from the f21 sync).
    for fragment in [
        "facade dependency",
        "laundered forbidden edge",
        "contract leak",
        "public api leak",
        "(not allowlisted)",
        "(forbidden)",
        "resolves to no public items",
        "exists in source but undeclared",
        "does not exist",
        "not verifiable from source",
        "names no stereotype",
        "unresolved module file",
        "unowned module edge endpoint",
        "vacuous constraint",
        "disallowed cross-component dependency",
        "forbidden external crate",
    ] {
        assert!(
            out.contains(fragment),
            "catalog must quote the verbatim fragment \"{fragment}\":\n{out}"
        );
    }
}

#[test]
fn help_diagnostics_documents_exit_code_contract() {
    let fixture = Fixture::new();
    let out = stdout(&fixture.run(&["help", "diagnostics"]));
    assert!(
        out.contains("exit 0") && out.contains("exit 1"),
        "catalog must state the exit-code contract (exit 0 / exit 1):\n{out}"
    );
    assert!(
        out.contains("--strict") && out.to_lowercase().contains("promot")
            && out.contains("warning"),
        "catalog must document the --strict warning promotion:\n{out}"
    );
}

#[test]
fn help_diagnostics_explains_resolution_classes() {
    let fixture = Fixture::new();
    let out = stdout(&fixture.run(&["help", "diagnostics"]));
    for class in ["code-fix", "spec-fix", "architecture-rework"] {
        assert!(
            out.contains(class),
            "catalog must define resolution class \"{class}\":\n{out}"
        );
    }
    assert!(
        out.contains("REPORT FORMAT") || out.contains("report format"),
        "catalog must specify the required report format:\n{out}"
    );
}

#[test]
fn help_diagnostics_is_deterministic() {
    let a = Fixture::new();
    let b = Fixture::new();
    assert_eq!(
        stdout(&a.run(&["help", "diagnostics"])),
        stdout(&b.run(&["help", "diagnostics"])),
        "help diagnostics must be byte-identical across runs"
    );
}

#[test]
fn depgraph_help_and_docs_quote_module_tier_sentence() {
    // The module-tier capability sentence (the exact words the depgraph guard
    // prints on a tier-less tree) must appear verbatim in the printed `depgraph`
    // help and in the depgraph docs, so a Go user can read the requirement and
    // its remedies BEFORE running the command.
    const SENTENCE: &str =
        "depgraph needs the module tier, which this model has none of; a Go tree acquires one through go.work members (2+)";

    let fixture = Fixture::new();
    let out = stdout(&fixture.run(&["depgraph", "--help"]));
    assert!(
        out.contains(SENTENCE),
        "depgraph --help must carry the module-tier sentence:\n{out}"
    );

    let manifest = env!("CARGO_MANIFEST_DIR");
    for doc in [
        "docs/archspec/commands/depgraph/errors.md",
        "docs/archspec/commands/depgraph/README.md",
    ] {
        let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
            .unwrap_or_else(|err| panic!("read {doc}: {err}"));
        assert!(
            body.contains(SENTENCE),
            "{doc} must carry the module-tier sentence verbatim"
        );
    }
}

#[test]
fn help_diagnostics_unknown_topic_still_errors_listing_diagnostics() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "bogus"]);

    assert_eq!(output.status.code(), Some(1), "unknown topic must exit 1");
    let err = stderr(&output);
    assert!(err.contains("bogus"), "must name the unknown topic:\n{err}");
    assert!(err.contains("diagnostics"), "must list diagnostics as valid:\n{err}");
    assert!(err.contains("run 'archspec help'"), "must hint at help:\n{err}");
}