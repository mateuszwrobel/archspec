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
    ("skill", "archspec skill"),
];

const SUMMARIES: &[&str] = &[
    "scaffold config + starter spec (no real-tree capture)",
    "extract the architecture model only",
    "render model (extracted or declared) to an artefact",
    "extract + compare vs spec, full diff, exit code",
    "seed a spec by snapshotting the real tree",
    "text/markdown/json diff + metric output",
    "print the spec JSON schema or annotated reference",
    "diagnose which language drivers/toolchains are present",
    "zero-config file-level import map (discovery)",
    "current-state module/submodule/API-usage views of a project",
    "print the archspec built-in manual (topics + per-command help)",
    "print or install the agent-facing audit skill",
];

const TOPICS: &[&str] = &[
    "commands",
    "glob",
    "spec",
    "constraints",
    "languages",
    "roles",
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
    assert!(stderr(&output).is_empty(), "no stderr on help output");
}

#[test]
fn help_flag_prints_command_list() {
    let fixture = Fixture::new();
    let output = fixture.run(&["--help"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(out.contains("usage:"), "must show usage line:\n{out}");
    for (name, _) in COMMANDS {
        assert!(
            out.contains(name),
            "command list must contain {name}:\n{out}"
        );
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

        assert_eq!(output.status.code(), Some(0), "{name} --help must exit 0");
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

// ---- Feature: `archspec --version` (blind-run friction: identity) ----------
// Muscle memory asks the binary which build it is. `--version` (and the
// `version` subcommand) answer with the crate version and exit 0; the
// unknown-command refusal path must survive the new early return intact.

#[test]
fn version_flag_prints_identity_and_exits_zero() {
    let fixture = Fixture::new();
    let output = fixture.run(&["--version"]);

    assert_eq!(output.status.code(), Some(0), "--version must exit 0");
    let printed = stdout(&output);
    assert!(
        !printed.contains("unknown command"),
        "--version must not hit the unknown-command path:\n{printed}"
    );
    let version = printed.trim().strip_prefix("archspec ").unwrap_or(printed.trim());
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "--version must print the crate version:\n{printed}"
    );
    assert!(
        version.starts_with(|c: char| c.is_ascii_digit()) && version.split('.').count() >= 3,
        "printed version must look like a semver:\n{printed}"
    );
}

#[test]
fn version_subcommand_matches_the_flag() {
    let fixture = Fixture::new();
    let output = fixture.run(&["version"]);

    assert_eq!(output.status.code(), Some(0), "version must exit 0");
    assert_eq!(
        stdout(&output),
        format!("archspec {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn version_early_return_leaves_unknown_command_refusal_intact() {
    let fixture = Fixture::new();
    // A sibling of --version that is NOT implemented must still be refused:
    // the new early return may not swallow the unknown-command path.
    let output = fixture.run(&["--verbose"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("error: unknown command: --verbose"),
        "unknown flags outside --version must still be refused:\n{}",
        stderr(&output)
    );
}

#[test]
fn version_flag_is_discoverable_and_paired_with_doctor_identity_note() {
    let fixture = Fixture::new();
    let help = fixture.run(&["--help"]);
    assert!(help.status.success());
    assert!(
        stdout(&help).contains("archspec --version"),
        "top-level help must point at --version:\n{}",
        stdout(&help)
    );

    // The docs wire the split plainly: sanity = doctor, identity = --version.
    let manifest = env!("CARGO_MANIFEST_DIR");
    const SENTENCE: &str = "For identity — which build of archspec this is — use `archspec --version` (or `archspec version`); `doctor` answers sanity (toolchains and drivers), not identity.";
    let body = std::fs::read_to_string(format!("{manifest}/docs/archspec/commands/doctor/README.md"))
        .expect("read doctor README");
    assert!(
        body.contains(SENTENCE),
        "doctor README must pair sanity with --version identity:\n{body}"
    );
}

// ---- Feature: doc prose guards (blind-run friction fixes) ------------------
// Same mechanism as depgraph_help_and_docs_quote_module_tier_sentence: a
// verbatim sentence pinned in the payload docs, so a rewrite that drops the
// teaching fails the suite instead of silently re-introducing the friction.

#[test]
fn identity_duality_sentence_pinned_in_skill_and_spec_docs() {
    // The unit-vs-namespace duality (`Shop.Api` vs `Shop::Api`) is
    // stated plainly — not obliquely — in the csharp section of `archspec
    // help languages` (the single owner since blind-4 follow-up US 04); the
    // skill and spec-format docs keep an explicit pointer, never a restatement.
    const SENTENCE: &str = "every project appears twice: its `.`-separated name is the build unit (named by `units` and unit edges), its `::`-separated name is the namespace identity (the key space of `module_edges` and `roles`); an `update`-seeded spec therefore carries both tiers and that is not duplication — ADR-018 assigns allowance ownership across them.";
    const POINTER: &str = "csharp section of `archspec help languages`";

    let fixture = Fixture::new();
    let output = fixture.run(&["help", "languages"]);
    assert_eq!(output.status.code(), Some(0));
    let flat = stdout(&output)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        flat.contains(SENTENCE),
        "the csharp section of `archspec help languages` must state the identity duality verbatim:\n{flat}"
    );

    let manifest = env!("CARGO_MANIFEST_DIR");
    for doc in ["docs/archspec/skill.md", "docs/archspec/spec.md"] {
        let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
            .unwrap_or_else(|err| panic!("read {doc}: {err}"));
        assert!(
            body.contains(POINTER),
            "{doc} must keep an explicit pointer to the owner surface"
        );
        let flat_doc = body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace('*', "");
        assert!(
            !flat_doc.contains(SENTENCE),
            "{doc} must keep a pointer, not a restatement of the duality sentence"
        );
    }
}

#[test]
fn inspect_docs_teach_label_grep_over_node_id() {
    // Model-mode node ids carry hash suffixes; the teaching that markers and
    // names live in the quoted LABEL must be pinned in the inspect docs.
    const SENTENCE: &str = "node ids in model modes are generated identifiers (sanitized path plus a short hash suffix), while role markers and human-readable names live in the quoted label — grep the label, not the id";

    let manifest = env!("CARGO_MANIFEST_DIR");
    let doc = "docs/archspec/commands/inspect/output.md";
    let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
        .unwrap_or_else(|err| panic!("read {doc}: {err}"));
    assert!(
        body.contains(SENTENCE),
        "{doc} must teach label-grep over id-grep verbatim"
    );
    assert!(
        body.contains("grep -F"),
        "{doc} must carry a concrete grep one-liner"
    );
}

#[test]
fn report_docs_explain_unit_tier_pair_union() {
    // `edges (internal)` counts unit-tier pairs: a single-crate tree prints 0
    // while its module graph is rich. That semantics must be pinned.
    const SENTENCE: &str = "`edges (internal)` is a pair-union at unit tier: a single-crate tree legitimately prints `0` here while its module graph is rich, because unit-internal module edges add no unit-tier pair";

    let manifest = env!("CARGO_MANIFEST_DIR");
    let doc = "docs/archspec/commands/report/output.md";
    let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
        .unwrap_or_else(|err| panic!("read {doc}: {err}"));
    assert!(
        body.contains(SENTENCE),
        "{doc} must explain the pair-union semantics verbatim"
    );
}

#[test]
fn help_docs_state_the_closed_pipe_contract() {
    // The closed-pipe behavior every render command already shows is pinned by
    // tests/cli_pipe.rs but stated in exactly one doc surface — the tool-wide
    // help.md — so `| head -1` stays documented as silence, not panic, with no
    // per-command duplicates to drift apart.
    const SENTENCE: &str =
        "A render command whose consumer exits first is silence, not panic: the next write to a closed pipe dies quietly (SIGPIPE — shell status 141) or the process exits 0 when the output had already flushed before the pipe closed; no panic text is ever printed";

    let manifest = env!("CARGO_MANIFEST_DIR");
    let doc = "docs/archspec/help.md";
    let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
        .unwrap_or_else(|err| panic!("read {doc}: {err}"));
    assert!(
        body.contains(SENTENCE),
        "{doc} must state the closed-pipe contract verbatim"
    );
}

// ---- Feature: roles taught in the two graph views (roles-views US 04) ------
// The same quote-lock mechanism as the module-tier sentence: the legend that
// teaches where role markers appear (and which surfaces stay silent by
// decision) is pinned verbatim from the payload docs, and the emit legs prove
// the renderer produces exactly the marker bytes the legends quote — so a doc
// that drifts from the code, or code that drifts from the doc, goes red here.

/// The diagram legend: scan mode marks the nodes the roles map addresses,
/// exact identity with the separator equivalence, and spec mode is silent by
/// construction. The emit legs assert the quoted marker bytes themselves.
#[test]
fn diagram_docs_teach_scan_mode_role_marks_verbatim() {
    const LOOKUP: &str = "a node is marked iff the model's `roles` map holds a key equal to that node's name under the `.`/`::` separator equivalence (the key `Shop::Api` marks the node `Shop.Api`) — exact identity, no fold and no subtree containment";
    const MODE_SILENCE: &str = "Role marks exist in scan mode only: spec mode's nodes are declared components rather than model paths, so `diagram` in its default mode reads no roles map and its bytes never move.";

    let manifest = env!("CARGO_MANIFEST_DIR");
    let doc = "docs/archspec/commands/diagram/output.md";
    let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
        .unwrap_or_else(|err| panic!("read {doc}: {err}"));
    for sentence in [LOOKUP, MODE_SILENCE] {
        assert!(
            body.contains(sentence),
            "{doc} must teach the scan-mode marker rule verbatim: {sentence}"
        );
    }

    // Docs ↔ bytes: the marker forms the legend quotes are what the renderer
    // emits for a model addressing `Shop::Api` (a `::`-spelled key, a
    // `.`-spelled node — the equivalence the sentence names).
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{
  "schema_version": 1,
  "language": "csharp",
  "units": [
    { "name": "Shop.Api", "kind": "project", "path": "Shop.Api" },
    { "name": "Shop.Domain", "kind": "project", "path": "Shop.Domain" }
  ],
  "edges": [ { "from": "Shop.Api", "to": "Shop.Domain" } ],
  "roles": { "Shop::Api": "composition" }
}
"#,
    );
    let mermaid = fixture.run(&["diagram", "--source", "scan", "model.json"]);
    assert_eq!(mermaid.status.code(), Some(0), "{}", stderr(&mermaid));
    assert!(
        stdout(&mermaid).contains("[\"Shop.Api [composition]\"]"),
        "the mermaid label-suffix legend byte must be emitted:\n{}",
        stdout(&mermaid)
    );
    let plantuml = fixture.run(&["diagram", "--format", "plantuml", "--source", "scan", "model.json"]);
    assert_eq!(plantuml.status.code(), Some(0), "{}", stderr(&plantuml));
    assert!(
        stdout(&plantuml).contains("component Shop.Api <<composition>>"),
        "the plantuml stereotype legend byte must be emitted:\n{}",
        stdout(&plantuml)
    );
}

/// The depgraph legend: the projection-based rule, the fold-silence rule, and
/// the api-usage silence stated as a decision. Emit legs: one honest fold
/// marks, and the api-usage body grows no marker syntax on a role-carrying
/// tree (the sentence's "unchanged by roles" half).
#[test]
fn depgraph_docs_teach_fold_markers_and_the_api_usage_silence_verbatim() {
    const FOLD_RULE: &str = "every key of the model's `roles` map is projected through this view's own fold — the same separator lift and the same granularity decision that project the edges — and the node is marked iff the roles landing on it are exactly one distinct role";
    const FOLD_SILENCE: &str = "A node onto which two different roles fold carries no marker at all — the fold is too coarse to state one role, and silence beats an aggregate claim — and a key whose projection names a node the view does not render (a unit-root path folding to `root`) contributes nothing: the rule never invents a node to carry a role.";
    const USAGE_SILENCE: &str = "Roles are not usage facts, so this view marks nothing: the three fixed columns and the rows are unchanged by roles and the emptiness statements are untouched (acceptance row 38 records the same decision) — the roles answer for module-level questions is the `modules` graph, the report's Roles section and the `roles` map in the `scan` model.";

    let manifest = env!("CARGO_MANIFEST_DIR");
    let doc = "docs/archspec/commands/depgraph/output.md";
    let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
        .unwrap_or_else(|err| panic!("read {doc}: {err}"));
    for sentence in [FOLD_RULE, FOLD_SILENCE, USAGE_SILENCE] {
        assert!(
            body.contains(sentence),
            "{doc} must teach the marker rule verbatim: {sentence}"
        );
    }

    // Docs ↔ bytes: the `main` node a rust bin composition key folds onto
    // carries the quoted suffix and stereotype forms.
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/main.rs", "mod store;\nuse store::Db;\nfn main() { let _ = Db; }\n");
    fixture.write("src/store.rs", "pub struct Db;\n");
    let mermaid = fixture.run(&["depgraph", "modules"]);
    assert_eq!(mermaid.status.code(), Some(0), "{}", stderr(&mermaid));
    assert!(
        stdout(&mermaid).contains("main[\"main [composition]\"]"),
        "the folded node must carry the legend's label suffix:\n{}",
        stdout(&mermaid)
    );
    let plantuml = fixture.run(&["depgraph", "modules", "--format", "plantuml"]);
    assert_eq!(plantuml.status.code(), Some(0), "{}", stderr(&plantuml));
    assert!(
        stdout(&plantuml).contains("component main <<composition>>"),
        "the folded node must carry the legend's stereotype:\n{}",
        stdout(&plantuml)
    );
    // The api-usage half: the same role-carrying tree prints its empty-state
    // body with no marker syntax anywhere in it.
    let usage = fixture.run(&["depgraph", "api-usage"]);
    assert_eq!(usage.status.code(), Some(0), "{}", stderr(&usage));
    let out = stdout(&usage);
    assert!(
        !out.contains("[facade]") && !out.contains("[composition]") && !out.contains("<<"),
        "api-usage stays a usage table on a role-carrying tree:\n{out}"
    );
}

/// The skill's roles passage stops owning roles-in-views facts: it keeps no
/// per-view marking enumeration and points at `archspec help roles` (whose
/// cells derive from the computed matrix) as the source. The second half
/// sweeps the payload docs for the stale era claims this plan supersedes (a
/// graph view printing no roles, a per-view marking table).
#[test]
fn skill_row_points_at_the_roles_topic_and_no_doc_reclaims_silence() {
    const POINTER: &str = "`archspec help roles`";
    const BANNED: &[&str] = &[
        "marks in `inspect tree` alone",
        "marks only in `inspect tree`",
        "teach nothing about roles",
        "state no roles by decision",
        "marks the nodes its fold names",
    ];

    let manifest = env!("CARGO_MANIFEST_DIR");
    let skill = std::fs::read_to_string(format!("{manifest}/docs/archspec/skill.md"))
        .expect("read skill.md");
    assert!(
        skill.contains(POINTER),
        "skill.md's roles passage must carry the pointer sentence naming \
         `archspec help roles` as the source"
    );
    for phrase in BANNED {
        assert!(
            !skill.contains(phrase),
            "skill.md keeps no per-view marking enumeration; found {phrase:?}"
        );
    }
    let row = skill
        .lines()
        .find(|line| line.starts_with("| Which nodes are special"))
        .expect("the decision table keeps its Which-nodes-are-special row");
    for phrase in BANNED {
        assert!(
            !row.contains(phrase),
            "the decision-table row is a pointer, not an enumeration; found {phrase:?}"
        );
    }
    assert!(
        row.contains(POINTER),
        "the decision-table row must point at the roles topic:\n{row}"
    );

    fn sweep(dir: &std::path::Path, failures: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read docs dir") {
            let path = entry.expect("doc entry").path();
            if path.is_dir() {
                sweep(&path, failures);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let body = std::fs::read_to_string(&path).expect("read doc");
            for stale in [
                "prints no roles",
                "shows no roles",
                "prints nothing about roles",
            ] {
                if body.contains(stale) {
                    failures.push(format!("{}: {stale}", path.display()));
                }
            }
        }
    }
    let mut failures = Vec::new();
    sweep(&std::path::Path::new(manifest).join("docs/archspec"), &mut failures);
    assert!(
        failures.is_empty(),
        "a payload doc still claims a view prints no roles:\n{}",
        failures.join("\n")
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
        assert!(out.contains(topic), "help must list topic {topic}:\n{out}");
    }
    for (name, _) in COMMANDS {
        assert!(out.contains(name), "help must list command {name}:\n{out}");
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
        assert!(out.contains(name), "help commands must name {name}:\n{out}");
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
        out.contains("INCLUDING separators")
            && out.contains("'.'")
            && out.contains("'/'")
            && out.contains("'::'")
            && out.contains("'-'"),
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
    for key in [
        "'from'",
        "'forbid'",
        "'gated_modules'",
        "'allowed_from'",
        "'parent'",
    ] {
        assert!(out.contains(key), "glob help must cover key {key}:\n{out}");
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
        (
            "manifest_integrity",
            "require_publish, forbidden_dependencies, required_features",
        ),
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
    // Re-pointed at the numbered steps as canonical (fan-out plan US 02/D09):
    // the retired families header no longer carries an arrow chain, the
    // numbered recipe is the sole ordering statement of this topic.
    assert!(
        out.contains("1. archspec scan")
            && out.contains("2. archspec depgraph modules")
            && out.contains("3. archspec verify")
            && out.contains("5. archspec verify --strict"),
        "workflow must state the numbered recipe:\n{out}"
    );
    for command in [
        "scan", "depgraph", "diagram", "verify", "inspect", "doctor", "init", "update",
    ] {
        assert!(
            out.contains(command),
            "workflow must cover {command}:\n{out}"
        );
    }
    assert!(
        out.contains("scan")
            && out.contains("depgraph")
            && out.contains("verify")
            && out.contains("inspect"),
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
    assert!(err.contains("bogus"), "must name the unknown topic:\n{err}");
    for topic in TOPICS {
        assert!(err.contains(topic), "must list valid topic {topic}:\n{err}");
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

    assert_eq!(
        output.status.code(),
        Some(0),
        "help diagnostics must exit 0"
    );
    let out = stdout(&output);
    assert!(!out.trim().is_empty(), "catalog must not be empty:\n{out}");
    for category in DIAGNOSTIC_CATEGORIES {
        assert!(
            out.contains(category),
            "diagnostics catalog must name category \"{category}\":\n{out}"
        );
    }
    assert!(
        stderr(&output).is_empty(),
        "no stderr on help diagnostics:\n{}",
        stderr(&output)
    );
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
        out.contains("--strict")
            && out.to_lowercase().contains("promot")
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
    // help and in the depgraph docs, so a user can read the requirement and its
    // remedy BEFORE running the command.
    const SENTENCE: &str =
        "depgraph needs the module tier, which this model has none of; the module tier is derived from package references, which this tree records none of";

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
        "docs/archspec/commands/depgraph/acceptance.md",
    ] {
        let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
            .unwrap_or_else(|err| panic!("read {doc}: {err}"));
        assert!(
            body.contains(SENTENCE),
            "{doc} must carry the module-tier sentence verbatim"
        );
    }
}

/// The inspect half of the refusal-family doc quote-lock (cvd US 04): the
/// exact words the inspect tree-view guard prints must appear verbatim in the
/// inspect docs, so an acceptance row cannot drift into a paraphrase of the
/// stderr it quotes (the family note in both files sits beside the verbatim
/// rows; the runtime sentence itself stays per-view by decision).
#[test]
fn inspect_docs_quote_tree_module_tier_sentence_verbatim() {
    const SENTENCE: &str =
        "inspect tree needs the module tier, which this model has none of; the module tier is derived from the tree's package references, which this tree records none of, 'inspect scanner' renders the unit-tier model";

    let manifest = env!("CARGO_MANIFEST_DIR");
    for doc in [
        "docs/archspec/commands/inspect/acceptance.md",
        "docs/archspec/commands/inspect/errors.md",
    ] {
        let body = std::fs::read_to_string(format!("{manifest}/{doc}"))
            .unwrap_or_else(|err| panic!("read {doc}: {err}"));
        assert!(
            body.contains(SENTENCE),
            "{doc} must carry the inspect tree refusal sentence verbatim"
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
    assert!(
        err.contains("diagnostics"),
        "must list diagnostics as valid:\n{err}"
    );
    assert!(
        err.contains("run 'archspec help'"),
        "must hint at help:\n{err}"
    );
}
