use crate::archspec::cli;
use crate::archspec::commands::spec;

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

/// Catalog of every `verify`/`report` finding category: the exact message
/// pattern it prints (quoted verbatim from `verify::compare::render_report` and
/// its `format!` bodies plus the `report::diff_items` labels), what it means,
/// its origin (code / spec / intent), severity and `--strict` behaviour, the
/// first follow-up command, and the decision an agent must make. The exit-code
/// contract and the required REPORT FORMAT close the topic. Byte-identical
/// across runs (plain constant).
pub const DIAGNOSTICS_HELP: &str = r#"# archspec help diagnostics — interpreting verify/report findings

Legend — origin (what to change):
  code      source violates what the spec declares                -> code-fix
  spec      spec is mis-declared for what the code already is     -> spec-fix
  intent    code and spec each describe real structure the other
            cannot express without a redesign                      -> architecture-rework

Legend — severity and exit-code contract:
  error      always fails the run: verify exits 1; report lists the line
             (report has no --strict gate; verify is the gate).
  warning    listed after every error prefixed 'warning: ', run exits 0.
             Under --strict each warning is promoted to an error line (the
             'warning: ' prefix is dropped) and the run exits 1.
  exit 0     clean pass, or warnings-only without --strict.
  exit 1     any error-level finding, or any warning under --strict.
  The verify/report text goes to stdout; a rule violation exits 1 WITHOUT
  echoing the report to stderr (stderr carries operational errors only).

ARTEFACT FRESHNESS — the dirt map, what dirties which artefact:

Artefact freshness keys on structure, not content: a comment-only edit dirties no artefact; edits that change structure (files, modules, or edges) dirty the `scan` model JSON, the `inspect` file map, and `depgraph` projections; `report` follows only structure that reaches the component layer (component edges, metrics, violations), the same layer at which `verify` engages; the spec-mode `diagram` renders declared components and never dirties on source edits, only on spec changes.

REPORT FORMAT an agent must emit for every finding:
  finding:    the exact '<category>: <detail>' line printed by verify/report
  evidence:   the command run plus the output excerpt that produced it
              (archspec verify / report / scan / depgraph / inspect)
  resolution: one of   code-fix | spec-fix | architecture-rework
  follow-up:  the single next command to re-run to confirm the fix

===== error-level categories (must clear before verify exits 0) =====

forbidden edge
  pattern:    forbidden edge: <from> -> <to>   (report label 'forbidden edge present')
  meaning:    <from> imports <to>, which <from>'s allowed.forbidden bans.
  origin:     code
  severity:   error (exit 1)
  follow-up:  archspec inspect   (the concrete import crossing the edge)
  decision:   code-fix — remove the import or move the dependency. Re-declaring
              the ban as a spec-fix only re-asserts a rule the code ignores.

disallowed cross-component dependency
  pattern:    disallowed cross-component dependency: <from> -> <to>
  meaning:    <from> depends on <to> across a boundary; <to> is not in <from>'s
              allowed.depend_on.
  origin:     spec (dependency is real but undeclared) or code
  severity:   error (exit 1)
  follow-up:  archspec depgraph  (is the edge intended?)
  decision:   spec-fix — add allowed.depend_on if intended; else code-fix to cut
              the dependency.

missing edge
  pattern:    missing edge: <from> -> <to>
  meaning:    <from> declares allowed.depend_on <to> but no extracted edge exists.
  origin:     spec (a dependency the code dropped)
  severity:   error (exit 1)
  follow-up:  archspec scan      (check module_edges for the pair)
  decision:   spec-fix — delete the stale depend_on, or code-fix to restore it.

facade dependency
  pattern:    facade dependency: <from> -> <root>
  meaning:    an internal module depends on a publication-only crate-root facade
              (a root whose file only declares modules + re-exports). Listing the
              root in allowed.depend_on does NOT legalize it (f21, ac6349500).
   emitted by: rust and csharp (the role-facade fact is the model's facade
               roles, each driver deriving them from its own facts; the go
               driver derives no facade role, so the rule is inert there)
               [capability role-facade rust="granular" csharp="granular" go="not-emitted"]
  origin:     code
  severity:   error (exit 1)
  follow-up:  archspec inspect   (the import routed through a root re-export)
  decision:   code-fix — canonicalize the import to its real module path; never
              declare the root as a dependency target.

contract leak
  pattern:    contract leak: <module> exposes <stereotype> (forbidden)
  meaning:    a module or submodule surfaces a unit matching a stereotype its
              contract.forbid forbids.
   emitted by: rust, csharp, and go for the submodule form (submodule
               enforcement needs the module tier — the capability row below —
               so top-level and submodule contracts both engage there)
               [capability module-tier rust="granular" csharp="granular" go="granular"]
  origin:     code
  severity:   error (exit 1)
  follow-up:  archspec depgraph  (which unit carries the forbidden stereotype)
  decision:   code-fix — stop exposing it; or spec-fix if the contract forbids a
              stereotype the module was always meant to publish.

cycle / no_cycles violation
  pattern:    cycle: <A -> B -> A>   (report label 'cycle detected in'; the
              check is the no_cycles constraint)
  meaning:    a no_cycles constraint found a strongly connected component.
  origin:     code, or intent when the cycle is the honest shape of the design
  severity:   error (exit 1); a no_cycles constraint with severity = "warning"
              makes it a warning (exit 0 without --strict, exit 1 under it).
  follow-up:  archspec depgraph  (see the ring), archspec scan for raw edges
  decision:   code-fix — invert or extract one edge; architecture-rework when no
              single cut breaks the ring without contradicting the model.

public api leak
  pattern:    public api leak: <owner> exposes <export> (not allowlisted)
  meaning:    a crate-root public export is not in the public_api_allowlist
              allowed set.
  origin:     code (grew surface) or spec (the surface is intended)
  severity:   error (exit 1)
  follow-up:  archspec scan      (root_public_exports lists the real surface)
  decision:   code-fix to hide the export, or spec-fix to allowlist an intended
              public API.

unverifiable glob export
  pattern:    unverifiable glob export: <owner> exposes <glob>
  meaning:    a root 'pub use path::*' cannot be resolved from source (external
              crate, unknown path, cfg-blocked or poisoned chain) — reported,
              never silently skipped.
  origin:     spec / intent (the model cannot enumerate the surface)
  severity:   error (exit 1)
  follow-up:  archspec scan      (root_glob_exports shows the glob)
  decision:   spec-fix — expand the glob to the explicit names it should resolve
              to; architecture-rework if the glob must stay external.

empty glob export
  pattern:    empty glob export: <owner> exposes <glob> (resolves to no public items)
  meaning:    a root glob resolves to a same-crate module exporting nothing; an
              empty derived set would silently drop a checked surface.
  origin:     code (surface vanished) or spec (stale glob)
  severity:   error (exit 1) — fails closed
  follow-up:  archspec scan
  decision:   spec-fix — drop the glob, or code-fix to restore the re-exported
              items.

forbidden external crate
  pattern:    forbidden external crate: <module> -> <crate>
  meaning:    a module matched by a forbid_external_crates 'from' imports a crate
              matched by 'forbid'.
  origin:     code
  severity:   error (exit 1)
  follow-up:  archspec inspect   (which file imports the crate)
  decision:   code-fix — remove/replace the dependency; architecture-rework if the
              boundary genuinely needs that crate.

not external free
  pattern:    not external free: <module> imports <packages>
  meaning:    an external_free constraint matched this module/unit and the model
              attributes external packages to it — the purity guard fired. Its
              vacuous counterpart reads 'matches no module or unit present in
              the model': a pure module that merely has no matching element is
              NOT the passing case, it is a dead pattern.
  origin:     code
  severity:   error (exit 1)
  follow-up:  archspec inspect   (which file imports the package)
  decision:   code-fix — remove the dependency; architecture-rework when the
              layer is meant to own externals (then the guard is misplaced —
              it belongs on leaf layers, not composition roots).

manifest integrity
  pattern:    manifest integrity: <manifest> has forbidden dependency <dep>
              (also 'publish=...' / 'missing required feature <feature>')
  meaning:    a package manifest diverges from a manifest_integrity constraint.
  origin:     code
  severity:   error (exit 1)
  follow-up:  archspec scan      (manifest / unit_manifests facts)
  decision:   code-fix the manifest, or spec-fix a constraint that no longer
              holds.

feature boundary
  pattern:    feature boundary: ...   (cfg(feature) gated-module checks)
  emitted by: rust (cfg feature gates are rust-driver facts; on drivers that
              emit no root-module-declarations fact the constraint cannot
              engage and surfaces as a vacuous constraint with that capability
              reason instead of per-pattern findings)
              [capability root-module-declarations rust="granular" csharp="not-emitted" go="not-emitted"]
  origin:     code or spec
  severity:   error (exit 1); the capability vacuity warning is exit 0
              (exit 1 under --strict)
  follow-up:  archspec scan      (root_module_declarations gated flags)
  decision:   code-fix or spec-fix, whichever side drifted.

forbidden submodule dependency
  pattern:    forbidden submodule dependency: ...   (forbid_submodule_dependency)
   emitted by: rust, csharp (needs module content below the unit; go packages
               are units, so no tree shape engages it there)
               [capability module-tier rust="granular" csharp="granular" go="granular"]
  origin:     code
  severity:   error (exit 1)
  follow-up:  archspec depgraph  (submodule views)
  decision:   code-fix to cut the intra-parent sibling dependency.

structural components (boundary declaration, always error-level):
  missing component: <name>         spec declares a component absent from source
                                    -> spec-fix (declare or drop it)
  unexpected component: <name>      source has a component the spec omits
                                    -> spec-fix (declare a [[module]] for it)
  unassigned unit: <name>           a sub-unit no matches.units glob covers
                                    -> spec-fix (widen the glob)
  ambiguous module match: <entry>   two boundaries claim one path with equal
                                    specificity -> spec-fix (disambiguate)

===== warning-level categories (exit 0 now, exit 1 under --strict) =====

cycle (warning severity)
  pattern:    cycle: <A -> B -> A>   (rendered 'warning: cycle: ...')
  meaning:    a no_cycles constraint with severity = "warning" found a ring.
  decision:   as the error cycle above, tolerated without --strict; promoted to
              an error line (exit 1) under --strict.

unresolved module file
  pattern:    unresolved module file: <path>
  meaning:    a 'mod' declaration has no backing file the scanner could find.
   emitted by: rust (mod-file resolution is a rust-driver fact; the csharp/go
               drivers never produce this finding)
               [capability root-module-declarations rust="granular" csharp="not-emitted" go="not-emitted"]
  origin:     code
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec inspect / archspec scan
  decision:   code-fix — restore the file or remove the mod declaration.

unowned module edge endpoint
  pattern:    unowned module edge endpoint: <path>
  meaning:    a module-edge endpoint is owned by no declared boundary, so the
              pair cannot be placed on the boundary graph.
   emitted by: rust, csharp, and go once the tree carries a module tier
               (see the capability row below); a tier-less tree
               has no soft edges to report here
               [capability module-tier rust="granular" csharp="granular" go="granular"]
  origin:     spec
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec depgraph
  decision:   spec-fix — declare a boundary (matches.modules) that claims the path.

laundered forbidden edge
  pattern:    laundered forbidden edge: <from> -> <target> via <intermediates>
  meaning:    a banned pair is not directly imported, but a route from <from> to
              <target> rides at least one hop whose ownership resolved through
              the unit fallback (undeclared territory) — the ban is routed around.
   emitted by: rust, csharp, and go (hop ownership resolves through the module
               tier — the capability row below — so laundering is checked
               there too)
               [capability module-tier rust="granular" csharp="granular" go="granular"]
  origin:     code (the conduit exists in source)
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec depgraph (trace the path), archspec inspect the hops
  decision:   spec-fix if the intermediate is a real boundary you forgot to
              declare; else code-fix to remove the conduit hop.

dead reference (boundary / stereotype reference)
  pattern:    dead reference: module '<name>' <field> target "<target>"
              exists in source but undeclared — references resolve to declared
              top-level module names only
  pattern:    dead reference: module '<name>' <field> target "<target>"
              does not exist — create it or fix the reference (did you mean: ...)
  pattern:    dead reference: module '<name>' <field> target "<target>"
              not verifiable from source — driver emits no module-tier fact
              for this tree (did you mean: ...)
  emitted by: rust, csharp, and go; on runs whose capability row carries no
              module-tier fact the absent target is called "not verifiable
              from source" instead of claiming "does not exist"
              [capability module-tier rust="granular" csharp="granular" go="granular"]
  meaning:    an allowed.depend_on / allowed.forbidden target can never engage a
              declared boundary, so the rule silently verifies nothing.
  origin:     spec
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec scan / archspec depgraph (confirm the real module name)
  decision:   spec-fix — correct the reference, or declare the module it names.

dead contract target
  pattern:    dead reference: module '<name>' contract.forbid target "<target>"
              names no stereotype — declare a stereotype named "<target>" or fix
              the reference (did you mean: ...)
  meaning:    a contract.forbid names a stereotype that does not exist (top-level
              or submodule), so the contract can never fire.
  origin:     spec
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec scan (stereotypes)
  decision:   spec-fix — declare the stereotype, or fix the contract target.

vacuous constraint
  pattern:    vacuous constraint: [constraint #N] <type>: <detail>
              (rendered 'warning: vacuous constraint: ...')
  meaning:    a constraint matches no module, edge, manifest or export — it
              checks nothing while appearing to pass.
  origin:     spec
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec scan  (ground the constraint in a real module/edge)
  decision:   spec-fix — fix the 'from'/'modules'/targets so it engages, or
              delete the constraint.

note: the model tiers root_public_exports and root_glob_exports are not reported
as standalone lines; they surface through public api leak, unverifiable glob
export and empty glob export above.
"#;

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