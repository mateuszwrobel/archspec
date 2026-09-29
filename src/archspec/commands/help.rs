use crate::archspec::cli;
use crate::archspec::commands::spec;
use crate::archspec::helptext::DIAGNOSTICS_HELP;

/// help's own help: the topic index. Printed by `archspec help`, `archspec help
/// topics`, and `archspec help --help` (dispatch intercepts the global `--help`
/// and prints this constant), so the three stay byte-identical.
pub const HELP: &str = "\
usage: archspec help [topic]
archspec built-in manual: topics, glob rules, the spec reference, constraint
types, the language-tier matrix, the roles-in-views contract, the CI workflow,
and the diagnostics catalog.
'help <command>' prints that command's --help.

topics:
  commands      every command with its purpose and flags
  glob          glob matching rules for units and module paths
  spec          the architecture.spec.toml annotated reference
  constraints   the seven constraint types with their keys and severity
  languages     which model tiers each scanner populates
  roles         which views mark roles and which stay silent by decision
  workflow      the numbered audit recipe (this topic owns the ordering), the
                --strict CI gate, and inspect-edge mining
  diagnostics   every verify/report finding category: meaning, origin, severity,
                the code-fix/spec-fix/architecture-rework decision, and report format

commands:
  init       scaffold config + starter spec (no real-tree capture)
  scan       extract the architecture model only (no spec needed)
  diagram    render model (extracted or declared) to an artefact
  verify     extract + compare vs spec, full diff, exit code
  update     seed a spec by snapshotting the real tree
  report     text/markdown/json diff + metric output
  spec       print the spec JSON schema or annotated reference
  doctor     diagnose which language drivers/toolchains are present
  inspect    zero-config file-level import map (discovery)
  depgraph   current-state module/submodule/API-usage views
  help       print this built-in manual
  skill      print or install the agent-facing audit skill

run 'archspec help <topic>' for a topic; 'archspec help <command>' for a command
";

const TOPICS: [&str; 8] = [
    "commands",
    "glob",
    "spec",
    "constraints",
    "languages",
    "roles",
    "workflow",
    "diagnostics",
];

/// The nine model tiers a scanner can populate (model.rs). `LANGUAGE_TIERS`
/// lists, per language, which of these the real scanner fills — guarded by a
/// consistency test that extracts a tiny fixture and asserts the matrix matches
/// actual scan output.
pub const ALL_TIERS: [&str; 9] = [
    "units",
    "soft_structure",
    "module_edges",
    "external",
    "module_external",
    "unit_manifests",
    "root_public_exports",
    "root_glob_exports",
    "root_module_declarations",
];

/// Per-language model tiers the scanner populates, derived from the real scan
/// behavior (`scan/rust.rs`, `scan/csharp.rs`, `scan/go.rs`).
pub const LANGUAGE_TIERS: &[(&str, &[&str])] = &[
    (
        "rust",
        &[
            "units",
            "soft_structure",
            "module_edges",
            "external",
            "module_external",
            "unit_manifests",
            "root_public_exports",
            "root_glob_exports",
            "root_module_declarations",
        ],
    ),
    (
        "csharp",
        &[
            "units",
            "soft_structure",
            "module_edges",
            "external",
            "module_external",
            "unit_manifests",
        ],
    ),
    (
        "go",
        &["units", "module_edges", "external", "module_external"],
    ),
];

/// Per-language naming facts the tier matrix cannot carry, stated in the
/// section an agent reads before hand-writing a spec. Each sentence lives in
/// exactly this surface; every other mention is a link (fanout-output US 08,
/// registry relation rules).
pub const LANGUAGE_NOTES: &[(&str, &str)] = &[
    (
        "rust",
        "  a crate with `src/lib.rs` and `src/main.rs` and no explicit `[[bin]]` or\n  `[lib]` section yields units `<pkg>` (kind=crate) and `<pkg>-bin`\n  (kind=bin); `matches` must name those units exactly.\n",
    ),
    (
        "csharp",
        "  Two names for one project. In a C# tree (and any seeded spec with both\n  tiers), every project appears twice: its `.`-separated name is the build\n  unit (named by `units` and unit edges), its `::`-separated name is the\n  namespace identity (the key space of `module_edges` and `roles`); an\n  `update`-seeded spec therefore carries both tiers and that is not\n  duplication — ADR-018 assigns allowance ownership across them.\n",
    ),
    (
        "go",
        "  `matches.units` targets must be full import paths — a short package\n  name does not resolve there (verify then reports `unexpected component` /\n  `missing component`); module `name` values and `allowed.depend_on`\n  references may be short — references resolve to declared module names.\n  The module tier is derived from the tree's own packages, so a single\n  `go.mod` tree whose packages reference each other has one; `archspec\n  capability matrix` reports the fact.\n",
    ),
];

pub const GLOB_HELP: &str = "\
# archspec help glob — glob matching rules
Patterns in architecture.spec.toml match against unit names and dotted module paths.

- '*' and '**' match any run of characters, INCLUDING separators such as '.', '/',
  '::', and '-'. '**' behaves identically to '*'.
- Matching is case-sensitive and literal: only '*' is special. There is no regex,
  no character classes, no anchors.
- Unit globs (matches.units) match against full unit names (crates, projects,
  packages, go packages), e.g. 'Billing.*' matches 'Billing.core'.
- Module globs (matches.modules) match a module's full dotted path (e.g.
  'Billing::domain'), its bare last segment (e.g. 'domain'), or the path with its
  unit prefix stripped (e.g. 'app::auth::config' also matches as 'auth::config').
- A 'matches.modules' pattern matches the named module AND every module beneath
  it: its entire subtree of descendants, no wildcard needed. 'commands' covers
  'commands', 'commands::start', 'commands::init', and so on. Subtree matching
  never matches less than the module it names.
- A 'matches.modules' entry ending in '*' is a PATH PREFIX claim — the explicit
  form of that same subtree: 'commands::*' covers 'commands::start',
  'commands::init', and every deeper descendant in one entry ('commands*' is a
  raw character prefix — it also claims 'commands_old::*'). Ownership is by
  specificity, not declaration order: the entry pinning the most of the path
  wins, an exact entry always wins; when two boundaries claim the same module
  with equal specificity, 'verify' reports an 'ambiguous module match' instead
  of guessing.
- 'from' / 'forbid' / 'gated_modules' / 'allowed_from' / 'parent' keys resolve with
  the same module-path glob semantics as matches.modules (full path, bare last
  segment, and subtree ancestors).
- 'allowed.depend_on' and 'allowed.forbidden' targets, and 'no_cycles.modules',
  are EXACT declared-module-name matches — they are not globs.
";

pub const CONSTRAINTS_HELP: &str = "\
# archspec help constraints — the seven constraint types
Each [[constraint]] declares a 'type' and may set 'severity'.

severity:
  error       (default) a violation fails the run
  warning     listed in the report and tolerated; promoted to a failing error
              under --strict
  --strict    promote every warning-level divergence to an error (the CI gate)

key namespaces:
  allowed.depend_on / allowed.forbidden / no_cycles.modules address DECLARED
  MODULE NAMES — exact names, never globs. Constraint 'from' / 'forbid' /
  'parent' / 'gated_modules' / 'allowed_from' address MODEL-PATH GLOBS over the
  extracted tree; bare last-segment matching there is case-sensitive. Full
  rules: 'archspec help glob'.

types and their keys:
  no_cycles                     keys: modules, severity
    detect cycles over the declared module groups (unit and/or module edges);
    no modules list = every declared module. 'modules' is an exact
    declared-module-name list, not globs.
  public_api_allowlist          keys: allowed
    the unit's crate-root public exports must sit in 'allowed'; a root glob
    re-export (pub use path::*) is enumerated when it names same-crate modules
    and checked name by name; one that cannot resolve (external, unknown,
    cfg-gated) or resolves to nothing is reported, never skipped.
  forbid_external_crates        keys: from, forbid
    a module matching a 'from' pattern must not import a forbidden external
    crate. 'from' is module-path glob semantics; 'forbid' names packages in
    the driver's vocabulary: crate names (rust), NuGet package ids (c# — a
    package is attributed when referenced and namespace-visible via usings,
    longest-prefix match), module paths (go). Pattern comparison is
    case-sensitive while c# attribution is not — a c# 'forbid' entry must
    match the referenced package id casing exactly.
  manifest_integrity            keys: require_publish, forbidden_dependencies, required_features
    assert root + per-member manifest facts; each manifest checked individually.
  feature_boundary              keys: feature, gated_modules, allowed_from
    cfg(feature = \"...\")-gated module declarations; 'gated_modules' and
    'allowed_from' match modules as subtrees.
  forbid_submodule_dependency   keys: parent, from, forbid
    within a 'parent' module, a listed 'from' submodule must not depend on a
    forbidden sibling submodule. 'parent' is module-path glob semantics.
  external_free                 keys: from
    the modules or units matching 'from' must import NOTHING: zero attributed
    external packages. Engagement is by presence, not by externals — a matched
    element that already has no externals PASSES as a real check (no vacuous
    warning), a contamination fails naming the module and its packages, and a
    'from' pattern matching no element in the model at all is vacuous. A
    module-tier pattern monitors the module subtree's externals; a pattern
    naming a unit monitors every external attributed anywhere to that unit;
    run `archspec capability matrix` for the module-tier fact per language. Put the guard on
    leaf layers (domain/core) — on a composition root that wires everything it
    fails immediately, which is correct: the guard is misplaced there.
";

pub const WORKFLOW_HELP: &str = "\
# archspec help workflow — the audit recipe
Command families: extraction, current-state views, comparison, rendering,
discovery — a taxonomy of what the commands are for, not an ordering; the
numbered recipe below owns it. Steps 1–2 create no spec: on a first capture of
an untracked repo, seed one with `archspec update` before step 3 runs and
reconcile the seed per step 4 — the `update` row under Supporting commands owns
that reconciliation. The full audit of an unfamiliar repo, in order:

  1. archspec scan            extract the model (units, tiers, external) with no
                              spec — declare boundaries over what actually exists.
  2. archspec depgraph modules read the real module dependency graph before you
                              write a single rule. Node labels are projections
                              onto top-level modules, not addresses — the
                              addresses live in `scan` module_edges.
  3. archspec verify           compare the model against architecture.spec.toml.
                              An undeclared edge fails it (exit 1); every warning
                              — vacuous, dead reference, unowned module edge
                              endpoint — is audit signal, not noise. Each meaning
                              and remedy lives in `archspec help diagnostics`.
                              [audit case=\"clean\" command=\"verify\" exit=\"0\" pass=\"matches source model\"]
                              [audit case=\"ceiling\" command=\"verify\" exit=\"1\" category=\"disallowed cross-component dependency\"]
  4. tighten allowed.depend_on from the real graph — the declared set is an exact
                              ceiling AND floor, so list the edges depgraph
                              showed and nothing more.
  5. archspec verify --strict  the CI gate: promotes every warning-severity
                              finding to a failing error, so a green run means
                              zero findings of any kind. A warning-only tree
                              passes bare but fails --strict.
                              [audit case=\"clean\" command=\"verify --strict\" exit=\"0\" pass=\"matches source model\"]
                              [audit case=\"warning\" command=\"verify\" exit=\"0\" warning=\"dead reference\"]
                              [audit case=\"warning\" command=\"verify --strict\" exit=\"1\" category=\"dead reference\"]
  6. archspec inspect          mine file-level import edges as hypotheses the
                              guard cannot express yet. Discovery (inspect) reads
                              files; the guard (verify) enforces only declared
                              facts — convert each validated hypothesis into a
                              spec rule, and when no rule expresses it, report
                              the driver coverage gap.

Supporting commands:
  init       scaffold a minimal base spec to start from.
  update     snapshot the current model as a seed spec once the architecture is
             healthy. The seed may emit parallel unit-tier and namespace-tier
             boundaries for one project with differing depend_on sets — treat
             it as a starting point and reconcile it against depgraph modules.
  doctor     report driver capability and toolchain availability (rust, csharp, go)
             as separate facts — drivers parse in-binary, toolchains never gate.
  skill      install the agent-facing audit skill: `archspec skill install`
             drops it at .agent/skills/archspec.md so future agent sessions in
             the project load the extraction rules, the finding classes, and
             the blind spots automatically.

Warning meanings and the exit-code / `--strict` severity contract live in
`archspec help diagnostics`; constraint keys and severity defaults live in
`archspec help constraints`. This topic sequences them — it never restates them.

Piping convention:
  render commands (scan/diagram/report/inspect) write with the system default
  SIGPIPE disposition; piping stdout to a consumer that exits first
  (`archspec diagram | head -1`) ends the process quietly by signal — status
  141 (128+13) through the shell, or 0 when the output completed before the
  pipe closed — never with panic text on stderr.
";

/// The roles-in-views contract (roles plan, US 01): which views mark roles and
/// which are designed silent. The views × markers matrix in the test suite
/// (`tests/shared/roles_matrix.rs`) derives these cells from recorded command
/// output, so this prose follows the runs, never the other way around.
pub const ROLES_HELP: &str = "\
# archspec help roles — where roles appear in the views

Roles attach to model nodes (facade, composition root, and the rest); each
driver derives them from its own facts. A view either marks roles or it does
not — the contract, per view:

  The names below are commands run as written — `archspec inspect tree`,
  `archspec depgraph modules` — where `tree` and `modules` are positional
  subcommands, never flags.

  inspect tree — marks: entrypoint and facade nodes carry a role label.
  inspect scanner — marks: the same structural model as inspect tree, same role labels.
  inspect (file level) — none: roles attach to model nodes, not files — this view renders files.
  depgraph modules — marks: module nodes carry a role label.
  depgraph api-usage — none by decision: a role is not a usage fact; the view itself prints 'note: this view shows no roles by decision (a role is not a usage fact)'.
  diagram (spec mode) — none by decision: declared components are not model paths.
  diagram --source scan — marks: the scan model carries its roles.

A cell reading `none by decision` is designed silence, not oversight. The
fixture × view matrix the test suite computes — every cell derived from
recorded command output — outranks any document; a per-view sentence anywhere
else in the manual or the skill is a pointer to this topic, not a claim of
its own.
";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(args, &[])?;
    cli::exactly_one_positional(&parsed)?;
    let Some(topic) = parsed.positionals.first().map(String::as_str) else {
        print!("{HELP}");
        return Ok(());
    };
    match topic {
        "topics" => print!("{HELP}"),
        "commands" => print!("{}", commands_block()),
        "glob" => print!("{GLOB_HELP}"),
        "spec" => print!("{}", spec::HELP),
        "constraints" => print!("{CONSTRAINTS_HELP}"),
        "languages" => print!("{}", languages_block()),
        "roles" => print!("{ROLES_HELP}"),
        "workflow" => print!("{WORKFLOW_HELP}"),
        "diagnostics" => print!("{DIAGNOSTICS_HELP}"),
        other => {
            if let Some(info) = crate::archspec::COMMANDS
                .iter()
                .find(|command| command.name == other)
            {
                print!("{}", info.help);
            } else {
                return Err(format!(
                    "unknown help topic: {topic}\nvalid topics: {}\nrun 'archspec help'",
                    TOPICS.join(", ")
                ));
            }
        }
    }
    Ok(())
}

/// One block per command: name + purpose, then its full --help text, so the
/// flag listing is the same single source the commands themselves print.
fn commands_block() -> String {
    let mut out = String::from("commands:\n");
    for command in crate::archspec::COMMANDS {
        out.push_str(&format!("\n{}  {}\n", command.name, command.summary));
        for line in command.help.lines() {
            out.push_str(&format!("  {line}\n"));
        }
    }
    out
}

/// Render the language-tier matrix from `LANGUAGE_TIERS` so the printed manual
/// and the consistency-tested data cannot drift.
fn languages_block() -> String {
    let mut out = String::from("archspec help languages — the model tiers each scanner populates\n");
    out.push_str(&format!("\nA language scanner may populate any of: {}\n", ALL_TIERS.join(", ")));
    for &(language, tiers) in LANGUAGE_TIERS {
        out.push_str(&format!("\n{language}:\n"));
        if let Some((_, note)) = LANGUAGE_NOTES.iter().find(|(name, _)| *name == language) {
            out.push_str(note);
        }
        for tier in tiers {
            out.push_str(&format!("  {tier}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archspec::language::Language;
    use crate::archspec::model::Model;
    use crate::archspec::scan;
    use std::fs;
    use std::path::Path;

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir");
        }
        fs::write(path, content).expect("write fixture file");
    }

    fn tier_filled(model: &Model, tier: &str) -> bool {
        match tier {
            "units" => !model.units.is_empty(),
            "soft_structure" => !model.soft_structure.is_empty(),
            "module_edges" => !model.module_edges.is_empty(),
            "external" => !model.external.is_empty(),
            "module_external" => !model.module_external.is_empty(),
            "unit_manifests" => !model.unit_manifests.is_empty(),
            "root_public_exports" => !model.root_public_exports.is_empty(),
            "root_glob_exports" => !model.root_glob_exports.is_empty(),
            "root_module_declarations" => !model.root_module_declarations.is_empty(),
            other => panic!("unknown model tier: {other}"),
        }
    }

    fn write_rust_fixture(root: &Path) {
        write(
            root,
            "Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
        );
        write(
            root,
            "src/lib.rs",
            "pub mod core;\npub mod ui;\npub use core::types::Widget;\npub use core::types::*;\npub use serde::*;\n",
        );
        write(root, "src/core.rs", "pub mod types;\n");
        write(root, "src/core/types.rs", "pub struct Widget;\n");
        write(
            root,
            "src/ui.rs",
            "use crate::core;\nuse serde::Serialize;\npub fn paint() {}\n",
        );
    }

    fn write_csharp_fixture(root: &Path) {
        write(
            root,
            "app/app.csproj",
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Newtonsoft.Json\" />\n  </ItemGroup>\n</Project>\n",
        );
        write(root, "app/Core.cs", "namespace app.Core;\npublic class Core { }\n");
        write(
            root,
            "app/Ui.cs",
            "using app.Core;\nusing Newtonsoft.Json;\nnamespace app.Ui;\npublic class Ui { }\n",
        );
    }

    fn write_go_fixture(root: &Path) {
        write(root, "go.mod", "module example.com/demo\ngo 1.21\n");
        write(
            root,
            "app/app.go",
            "package app\n\nimport (\n\t\"example.com/demo/shared\"\n\t\"modernc.org/sqlite\"\n)\n\nfunc Run() {}\n",
        );
        write(root, "shared/shared.go", "package shared\n\nfunc X() {}\n");
    }

    fn language_of(name: &str) -> Language {
        match name {
            "rust" => Language::Rust,
            "csharp" => Language::Csharp,
            "go" => Language::Go,
            _ => panic!("unknown language: {name}"),
        }
    }

    fn extract_fixture(language: &str) -> Model {
        let temp = tempfile::TempDir::new().expect("temp dir");
        match language {
            "rust" => write_rust_fixture(temp.path()),
            "csharp" => write_csharp_fixture(temp.path()),
            "go" => write_go_fixture(temp.path()),
            _ => unreachable!(),
        }
        scan::extract(language_of(language), temp.path())
            .expect("extract must succeed")
    }

    #[test]
    fn language_tiers_match_real_scan_output() {
        for &(language, listed) in LANGUAGE_TIERS {
            let model = extract_fixture(language);
            for tier in ALL_TIERS {
                let populated = tier_filled(&model, tier);
                if listed.contains(&tier) {
                    assert!(
                        populated,
                        "the '{language}' scanner must populate tier '{tier}' but left it empty"
                    );
                } else {
                    assert!(
                        !populated,
                        "matrix omits '{tier}' for '{language}' but the scanner populated it"
                    );
                }
            }
        }
    }
}