//! Foundations home for the help surfaces' data constants. `DIAGNOSTICS_HELP`
//! is the owner-source of the `help diagnostics` topic — the catalog of every
//! verify/report finding category — kept in the foundations layer so both
//! `commands::help` (which renders it) and the report renderers' tests (which
//! weld their emitted categories to it) reach it down-layer, never upward.
//! The bytes are a contract: the claim registry welds them and
//! `archspec help diagnostics` prints them verbatim — relocating this constant
//! must move bytes, never edit prose.

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
  exit 64    usage class, universal across the tool: no command, unknown
             command, unknown flag, or a `help` typo — the arguments are
             wrong; no verdict was computed.
  exit 70    operational precondition: the spec file a verb needs is
             missing. A missing precondition is never answered as a verdict.
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
   emitted by: rust, csharp and ts (the role-facade fact is the model's facade
               roles, each driver deriving them from its own facts; the go
               driver derives no facade role, so the rule is inert there)
               [capability role-facade rust="granular" csharp="granular" go="not-emitted" ts="granular" python="not-emitted" kotlin="not-emitted"]
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
               [capability module-tier rust="granular" csharp="granular" go="granular" ts="granular" python="granular" kotlin="granular"]
  origin:     code
  severity:   error (exit 1)
  follow-up:  archspec depgraph  (which unit carries the forbidden stereotype)
  decision:   code-fix — stop exposing it; or spec-fix if the contract forbids a
              stereotype the module was always meant to publish.

cycle / no_cycles violation
  pattern:    cycle: <A -> B -> A>   (report label 'cycle detected in'; the
              check is the no_cycles constraint)
              file tier (constraint scope = "both", the ts and rust posture):
              cycle: <claim>: <file> -> <file> -> <file> — the ring of
              source files, attributed to the nearest common claim ancestor
              of its members;
              cycle: <file> -> <file> -> <file> — members under disjoint
              top-level claims: no claiming segment, every member named.
              A file ring whose claim projection is a cycle the claim tier
              already printed is not repeated (dedup by projection).
   emitted by: file tier by ts (dependency facts are file-grained by
              construction) and rust (the driver serializes file-grained
              facts and the rust posture derives both); go and csharp
              emit none
              [capability file-grained-dependency-facts ts="granular" rust="granular" go="not-emitted" csharp="not-emitted" python="granular" kotlin="granular"]
  meaning:    a no_cycles constraint found a strongly connected component —
              over the claim graph, and at the file tier over file edges
              (a type-only import is an edge; an asset import never is).
  origin:     code, or intent when the cycle is the honest shape of the design
  severity:   error (exit 1); a no_cycles constraint with severity = "warning"
              makes it a warning (exit 0 without --strict, exit 1 under it).
  follow-up:  archspec depgraph  (see the ring), archspec scan for raw edges
  decision:   code-fix — invert or extract one edge; architecture-rework when no
              single cut breaks the ring without contradicting the model.

public api leak
  pattern:    public api leak: <owner> exposes <export> (not allowlisted)
              public api leak: <owner> exposes <export> (not exposed)
  meaning:    a crate-root public export is not in the public_api_allowlist
              allowed set; or — the `(not exposed)` shape — a public symbol
              crosses a contract.expose boundary whose deepest claiming
              module does not list it (the expose chain law, spec.md).
  origin:     code (grew surface) or spec (the surface is intended)
  severity:   error (exit 1)
  follow-up:  archspec scan      (root_public_exports lists the real surface)
  decision:   code-fix to hide the export, or spec-fix to allowlist or expose
              an intended public API.

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
              [capability root-module-declarations rust="granular" csharp="not-emitted" go="not-emitted" ts="declarations, no gating facts" python="declarations, no gating facts" kotlin="not-emitted"]
  origin:     code or spec
  severity:   error (exit 1); the capability vacuity warning is exit 0
              (exit 1 under --strict)
  follow-up:  archspec scan      (root_module_declarations gated flags)
  decision:   code-fix or spec-fix, whichever side drifted.

forbidden submodule dependency
  pattern:    forbidden submodule dependency: ...   (forbid_submodule_dependency)
   emitted by: rust, csharp (needs module content below the unit; go packages
               are units, so no tree shape engages it there)
               [capability module-tier rust="granular" csharp="granular" go="granular" ts="granular" python="granular" kotlin="granular"]
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

language mismatch (run precondition, always fail-loud — exit 1):
  pattern:    language mismatch: <declared> vs <found> — <lang>: <n> files, ...
  pattern:    language mismatch: <declared> — no root marker <marker> for the declared language, found: <markers>
   meaning:    the spec's [project] language disagrees with the counted source
               files under the tree: sources outside the declared language
               survive the mechanical exclusion set and every
               excluded_sources entry, so the run would analyze the wrong tree.
               The check runs on scan and verify BEFORE any verdict or
               artifact, after the declared exclusions were applied.
   origin:     spec / tree shape (mixed-language tree declared as one language)
   severity:   error (exit 1) — no model, no verdict, no artifact
   follow-up:  archspec scan      (excluded sources lines name the active set)
   decision:   spec-fix — add [project] excluded_sources entries with reasons
               for the foreign subtree, or restructure the tree; fix the
               declared language only when the declaration itself is wrong.
               A declared language whose root marker is absent answers the
               second pattern: the scan root carries root markers of other
               languages only — pin the language the root actually spells,
               or scan a single-language root.

ambiguous root markers (run precondition, always fail-loud — exit 1):
  pattern:    ambiguous root markers: <path> (<language>), … — pin [project] language or scan a single-language root
   emitted by: every detect()-consuming verb (scan, update, depgraph,
               inspect, init) — the scan root carries more than one root
               marker (Cargo.toml, package.json, go.mod) and no pin decides
  meaning:    a polyglot root: the root markers name more than one driver
               and nothing decides between them. archspec models ONE
               driver per root, so the run states the collision and names
               EVERY marker file instead of guessing a winner.
   origin:     tree shape (a polyglot root scanned without a pin)
   severity:   error (exit 1) — no model, no verdict, no artifact
   follow-up:  archspec scan      (write the [project] language pin into
               architecture.spec.toml at the root, or scan one root per
               language)
   decision:   spec-fix — pin [project] language; the pin selects the
               driver and resolves the collision. Per-root scanning is
               the polyglot story meanwhile: `archspec scan backend/`
               names the tree you mean.

ambiguous settings file (run precondition, always fail-loud — exit 1):
  pattern:    ambiguous settings file: settings.gradle + settings.gradle.kts — keep one
   emitted by: kotlin (the module-list parse reads the settings file at the
               scan root; BOTH settings files present at one root is the
               collision — detection still answers kotlin, the family is
               one family, so it is the PARSE that refuses)
  meaning:    two files name the module list and nothing decides between
               them. archspec executes no gradle, so it cannot know which
               file gradle itself would read; guessing a winner could
               state a module list the build never has. The run names the
               collision and states no model.
   origin:     tree shape (a gradle root carrying both settings files)
   severity:   error (exit 1) — no model, no verdict, no artifact
   follow-up:  archspec scan      (keep ONE settings file at the root)
   decision:   one owner decides: keep one file. Gradle itself reads
               settings.gradle.kts when both exist; deleting the stale
               file restores the exact build the scan models.

 nested foreign marker (report-only, never a verdict — exit 0):
  pattern:    nested foreign marker: <path> (<language>) — not modelled
   emitted by: scan and verify (a rooted scan; one line per FOREIGN
               root-marker file below the scan root, behind the mechanical
               exclusion set, sorted by path)
  meaning:    the tree below that marker belongs to another language and
               this run models only the root's driver — a stated blind
               spot. The line states the marker; the sources under it keep
               the existing loud agreement law, so nothing goes silently
               unmodelled in a declared tree.
   origin:     tree shape (a polyglot tree below a rooted scan)
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (scan the sub-root per language, or
               declare the subtree with excluded_sources)
   decision:   none required; scan the sub-root when its facts matter, or
               declare it with an excluded_sources entry and a reason.

unresolvable import (run precondition, always fail-loud — exit 1):
  pattern:    unresolvable import: <file> -> "<specifier>"
   emitted by: ts (the ts driver's resolution law) and python (the relative
               depth clause below; rust, go and csharp
               resolve through their own module systems, never here)
  meaning:    a relative or exact specifier names no file the scan can find —
               extension and index inference located nothing, so an edge
               would silently vanish; the model can never state a dropped
               edge. The python driver's variant clause names the relative
               form whose `depth = dots − 1` walk-up leaves the package root
               ("— relative depth beyond package root"); a python relative
               import that lands on no referent rides the bare shape
               (python driver record P3). When the only candidate sits inside an excluded tree the
               line names it: "... — resolves only into excluded path <entry>".
               The resolution seam classes its bare and `#` misses onto this
               line too: a dead alias target appends the missed path
               ("— alias target not found: <path>"; a matched alias key whose
               map entry is gone spells the key: "— alias target not found
               for matched alias key \"<key>\""), an `#imports` miss appends
               the declaring manifest ("— imports map miss: <manifest>"), and
               an export-map miss appends the subpath, the package path and
               the active condition set ("— <subpath> is not exported under
               <conditions> from package <path> (see exports field in
               <manifest>)"). A bare miss that is neither a built-in nor a
               workspace-registry case stays an external fact. Linker
               honesty (R7d as narrowed): a workspace-member SUBPATH miss
               ("member name + path") rides this finding under every
               linker family; a BARE miss naming a workspace member is
               decided by the lockfile family at the registry root — under
               `pnpm-lock.yaml` or no lockfile it rides this finding, and
               under a hoisting family it is a first-party membership edge
               carrying the report-only `undeclared member import` line.
   origin:     code
   severity:   error (exit 1) — no model, no verdict
   follow-up:  archspec scan      (excluded sources lines name the active set)
   decision:   code-fix — restore or correct the import; spec-fix only by
               removing an exclusion that wrongly swallowed first-party code.

tsconfig not found (run precondition, always fail-loud — exit 1):
  pattern:    tsconfig not found: <path>
   emitted by: ts (the owning chain and its extends closure, pre-read
               before every delegation; the other drivers read no tsconfig)
  meaning:    a tsconfig the owning chain reaches resolves to no file. A
               relative `extends` target is spelled as the joined absolute
               path; an npm-specifier `extends` target is spelled verbatim
               as declared. The pre-read names the fault before any
               delegation, so a broken chain never resolves alias-free in
               silence.
   origin:     code
   severity:   error (exit 1) — no model, no verdict
   follow-up:  archspec scan      (rerun: the fault names itself before any verdict)
   decision:   code-fix — restore or correct the extends target the line
               names.

tsconfig load failed (run precondition, always fail-loud — exit 1):
  pattern:    tsconfig load failed: <path>
   emitted by: ts (the pre-read of the owning chain and its extends
               closure at ANY file name, and the resolver's own config
               load; the other drivers read no tsconfig)
  meaning:    a config on the chain EXISTS yet cannot be opened, read or
               parsed, so its aliases and paths cannot be trusted — the run
               never resolves alias-free over an unreadable config (the
               never-silent law). A JSON fault on a tsconfig-named file
               carries this class too. The line names the config path; the
               config's semantics stay the compiler's, never archspec's.
   origin:     code
   severity:   error (exit 1) — no model, no verdict
   follow-up:  archspec scan      (rerun: the fault names itself before any verdict)
   decision:   code-fix — repair or restore the config the line names.

tsconfig cycle (extends or reference):
  pattern:    tsconfig cycle (extends or reference): <a -> b -> a>
               tsconfig cycle (extends or reference): <path> — the project
               reference path points to this tsconfig
   emitted by: ts (the extends-closure walk and the resolver's own cycle
               detection; the other drivers read no tsconfig)
  meaning:    the extends chain loops — the walk names every config path
               verbatim, joined by ' -> ' — or a project reference points
               at its own tsconfig. The cycle is its own class, never a
               load failure: the loop is the fault.
   origin:     code
   severity:   error (exit 1) — no model, no verdict
   follow-up:  archspec scan      (rerun: the fault names itself before any verdict)
   decision:   code-fix — cut the loop the line names.

package manifest invalid:
  pattern:    package manifest invalid: <path>
   emitted by: ts (package manifests on a resolution path; other drivers
               state manifest faults through manifest integrity, not here)
  meaning:    a package.json that exists cannot be opened, read or parsed.
               The finding fires even when a legacy fallback resolves the
               import beside it — an installed-but-broken manifest never
               masquerades as `external not installed`.
   origin:     code
   severity:   error (exit 1) — no model, no verdict
   follow-up:  archspec scan      (rerun: the fault names itself before any verdict)
   decision:   code-fix — repair or restore the manifest the line names.

unclassified resolver error:
  pattern:    unclassified resolver error: <variant debug form>
   emitted by: ts (the resolution seam's catch-all; the other drivers
               resolve through their own module systems, never here)
  meaning:    a resolver error that no taxonomy class owns — the line
               names the variant and its payload verbatim. The seam matches
               every resolver error variant; a variant an upstream update
               adds surfaces in this line: a new upstream case fails loud,
               never silence.
   origin:     code or intent (the tree fault the payload names, or the
               mapping needing the new variant a home)
   severity:   error (exit 1) — no model, no verdict
   follow-up:  archspec scan      (rerun: the fault names itself before any verdict)
   decision:   code-fix when the named payload states a fixable tree fault;
               architecture-rework upstream (record the variant a home) when
               the line names an enum addition the mapping has not met.

dynamic import wildcard (report-only, never a verdict — exit 0):
  pattern:    dynamic import wildcard: <file> -> <expression>
   emitted by: ts (non-literal import() specifiers and computed require()
               calls; the other drivers have no dynamic-import fact to state)
  meaning:    the file computes an import() or require() specifier at
               runtime, so no scan can name its target — a stated blind
               spot, not a violation.
               The fact rides the model as dynamic_import_wildcards beside
               this report line; it never reddens a run and never counts
               as an edge.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (model dynamic_import_wildcards)
   decision:   none required; replace the computed specifier with a literal
               — or a map of literals — when the blind spot matters.

dynamic import not modelled (report-only, never a verdict — exit 0):
  pattern:    dynamic import not modelled: <file>:<line> — importlib
   emitted by: python (importlib.import_module and spec_from_file_location
               sites; the ts arm states its own `dynamic import wildcard`
               line, and the other drivers have no dynamic-import fact to
               state)
  meaning:    the file computes an import at runtime through the importlib
               machinery, so no scan can name its target — a stated blind
               spot, not a violation. A LITERAL string argument rides this
               same line (the named choice, python driver record P2):
               literal-to-external extraction is parked with the wildcard
               precedent. The site is NAMED and nothing is gated on it; the
               fact rides the transient report channel, never the machine
               body, never an edge.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (report surface only)
   decision:   none required; replace the computed specifier with a literal
               import statement when the blind spot matters.

external not installed (report-only, never a verdict — exit 0):
  pattern:    external not installed: <package> — declared in <manifest>, absent from disk
   emitted by: ts (a bare miss the importing package.json declares but
               that has no installed copy; the other drivers never print
               this line)
  meaning:    the package is declared in the nearest package.json yet no
               installed copy resolves — the import stays an external
               fact and the run passes on the statement. The absence of
               an installed copy is not a violation: a source tree whose
               dependencies were never installed is the normal clone
               state. The line never stands for a broken manifest — that
               fault owns the `package manifest invalid` error class.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   none required; install the dependency when the tree needs
               its facts resolved.

workspace link missing (report-only, never a verdict — exit 0):
  pattern:    workspace link missing: <member name> — resolved to <path> instead of the member dir
              workspace link missing: <member name> — absent from disk
   emitted by: ts (an npm or pnpm workspace whose importing manifest
               declares the member name: the first line fires when the
               resolution lands outside the member directory, the second
               when the member directory is absent from disk; the other
               drivers never print this line)
  meaning:    the member source lives in the workspace but the installed
               link does not own the resolution — a collision shadow or a
               missing install. The verdict follows resolution: a healthy
               link resolves the member first-party through the member
               directory, and this line states the link fault without
               changing any verdict.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   none required; restore the workspace link so the member
               directory owns the resolution.

undeclared member import (report-only, never a verdict — exit 0):
  pattern:    undeclared member import: <from> -> <member>
   emitted by: ts (a bare specifier naming a workspace member that the
               context-free member law leaves WITHOUT an answer — the
               target member declares no resolvable entry — imported by a
               manifest that does not declare it, where the workspace
               registry root carries a hoisting-family lockfile — yarn.lock
               or npm-shrinkwrap.json; a target member that carries its own
               answer resolves first-party under EVERY family with no line;
               under pnpm-lock.yaml or no lockfile an answer-less import is
               an `unresolvable import` error instead; the other drivers
               never print this line)
  meaning:    under a hoisting linker an undeclared member-name import is
               a real runtime edge, so the run honors the linker: the
               import is a first-party membership edge onto the first
               registered member for that name (nearest scan root first)
               and this line states the missing declaration without
               changing any verdict. The lockfile is read as a linker-
               family signal only — file names, never content, never a
               membership registry.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   none required; declare the member in the importing
               package.json when the dependency is intended.

workspace target not built (report-only, never a verdict — exit 0):
  pattern:    workspace target not built: <specifier> -> <declared-target>
   emitted by: ts (a specifier naming a workspace member: a BARE name
               whose declared main/module/default target sits under build
               output or is ABSENT — the member edge lands on the SOURCE
               answer, the types-condition TS target first, else the
               declared spelling with the dist|build prefix stripped plus
               the extension and index table — or a member-qualified
               SUBPATH whose mapped target is absent and gitignored while
               the winning exports entry's `types` arm declares exactly
               one existing SOURCE file: that source carries the edge.
               This line names the absent declared target once per pair;
               the other drivers never print this line)
  meaning:    the library is not built; the source is the truth for a
               member edge, so the edge is honest and the line states the
               unbuilt declared target without changing any verdict. For
               an attributed subpath import this line REPLACES the
               generated-source-not-built line — never doubled.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   none required; build the member when the built output is
               the artifact under test.

generated source not built (report-only, never a verdict — exit 0):
  pattern:    generated source not built: <verbatim-target> — gitignored <pattern-file>:<line>
   emitted by: ts (an import that fails resolution ONLY because its target
               is absent from disk AND the absent path is ignored by the
               committed `.gitignore` files — the verdict is a STATIC
               evaluation of those files; git is NEVER spawned; the
               evidence names the deciding `.gitignore` and its line)
  meaning:    the tree declares the target as generated output — the
               generator has not run, or its output was cleaned. The class
               never invents an edge and never adds an external-tier entry;
               with no committed ignore evidence the miss stays an
               `unresolvable import` error instead (fail closed).
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   none required; run the generator that owns the target.

generated tsconfig absent (report-only, never a verdict — exit 0):
  pattern:    generated tsconfig absent: <path>
   emitted by: ts (an `extends` target ABSENT from disk whose path runs
               under a generated-class directory of the mechanical
               exclusion list — the `.svelte-kit` precedent; when the spec declares
               `framework = "sveltekit"` the paired law also gives `$lib`
               a static alias to
               `<root>/src/lib`, so a fresh clone resolves honestly
               pre-build)
  meaning:    the tree's own generated tsconfig has not been built yet —
               the normal fresh-clone state of a sveltekit project. The
               line states the absence; it never invents a resolution
               and never reddens the run. Where the tree is genuinely
               mis-authored the paired `tsconfig not found` red stays.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   none required; run the project's build when the generated
               tsconfig surface matters.

tsconfig extends not installed (report-only, never a verdict — exit 0):
  pattern:    tsconfig extends not installed: <specifier>
   emitted by: ts (an `extends` target that resolves through the
               node_modules style and finds no installed copy — the
               specifier is spelled verbatim; the other drivers never
               print this line)
  meaning:    the base tsconfig package is not installed — the normal
               clone state, the `external not installed` precedent for
               the extends surface (`@tsconfig/*` bases carry no path
               maps, so the resolution delta is nil). The run passes on
               the statement; a path-form extends miss under a
               non-generated location keeps the `tsconfig not found` red.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   none required; install the base package when the extends
               semantics matter.

virtual specifier not modelled (report-only, never a verdict — exit 0):
  pattern:    virtual specifier not modelled: <verbatim>
   emitted by: ts (a bare specifier shaped like a URI scheme — letters
               then alphanumerics, `+`, `.` or `-`, then a colon — or one
               starting with a NUL byte: a virtual module id handed out by
               a bundler or runtime hook; the other drivers never print
               this line)
  meaning:    the specifier names code no file in the tree owns (an
               `https://` import, a `\0virtual:` id), so the run neither
               delegates nor strips it: no edge and no external-tier
               entry is recorded, and this line states what stayed
               unmodelled without changing any verdict. A `?` suffix is
               NOT this class — query suffixes resolve on their base.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   none required; the import names a build-time realm
               archspec does not model by decision.

framework conventions not configured (report-only, never a verdict — exit 0):
   pattern:    specifier not a package name: <verbatim> — framework
               conventions not configured?
   emitted by: ts (a bare specifier starting with `$` — the SvelteKit
               module-alias shape; a `$`-prefixed name is not a legal npm
               package id, so the generic external delegation asks the
               wrong question — while the spec declares no
               `[project] framework` key; deduplicated per importing
               directory and spelled specifier, like the sibling lines)
   meaning:    the specifier is shaped like a framework module alias the
               run is not configured to model. The tool ships NO
               framework auto-discovery: `svelte.config.js` is never
               read. Declaring `framework = "sveltekit"` registers the
               `$lib` alias (its imports become module edges) and pairs
               the tsconfig framework-shaped hint; a spec WITHOUT the
               key stays green exactly as before — the specifier keeps
               its generic external-tier attribution and this line only
               states the unmodelled shape. It never replaces
               attribution and never reddens the run.
   origin:     code
   severity:   report-only (exit 0)
   follow-up:  archspec scan      (stderr carries the line; the model
               artifact stays pure and carries no report field)
   decision:   declare `framework = "sveltekit"` in `[project]` when the
               tree is a SvelteKit project whose `$lib` imports should
               enter the model; otherwise none required.

no settings file (report-only, never a verdict — exit 0):
  pattern:    no settings file: gradle module list unavailable — one unit at the scan root
   emitted by: kotlin (a kotlin-pinned root with neither settings file —
               the driver knows no module list; rust, csharp, go, ts and
               python never print this line)
  meaning:    gradle declares its modules in the settings file; without
               one archspec executes no gradle and cannot invent the
               module list. The model answers with ONE unit at the scan
               root — every source file belongs to it. The claim backstop
               stays honest: a `[[module]]` claim over a unit the list
               skipped reports through the `vacuous constraint` category,
               so an absent module list can never green a constraint
               silently.
   origin:     tree shape (a kotlin root without a settings file)
  severity:   report-only (exit 0)
  follow-up:  archspec scan      (add the settings file with its include
               rows, or accept the one-unit model at this root)
  decision:   none required; declare the modules in the settings file
               when the module tier matters at this root.

unresolved settings include (report-only, never a verdict — exit 0):
  pattern:    unresolved settings include: <file>:<line> — module list may be incomplete
   emitted by: kotlin (an include argument that is not a statically
               visible string literal — a property, a loop, or computed
               logic; the site is NAMED)
  meaning:    the include row exists but archspec executes no gradle, so
               it cannot read the module name the build will compute.
               The site is named; the module stays out of the unit list.
               The named backstop: a `[[module]]` claim over a unit the
               list skipped reddens through the `vacuous constraint`
               warning — an incomplete module list can never green a
               constraint silently.
   origin:     tree shape (logic-driven include in a settings file)
  severity:   report-only (exit 0)
  follow-up:  archspec scan      (a literal `include(":name")` row is
               statically visible and resolves exactly)
  decision:   none required; prefer literal include rows when the module
               tier matters.

split package (report-only, never a verdict — exit 0):
  pattern:    split package: <package> — declared by <m> modules: <list>
   emitted by: kotlin (one package declaration carried by several gradle
               modules; rust, csharp, go, ts and python never print this
               line)
  meaning:    Kotlin permits one package declared across modules — a
               legal split, not a violation. The census merges the
               modules' files and imports resolve across the split; the
               line states WHICH modules share the package so a reader
               sees the shape without running gradle. Edges still land;
               nothing is gated on the line.
   origin:     tree shape (one package declaration in several modules)
  severity:   report-only (exit 0)
  follow-up:  archspec scan      (the line names the modules; the split
               stays legal)
  decision:   none required; consolidate the package under one module
               when the split is not intended.

gradle catalog unreadable (report-only, never a verdict — exit 0):
  pattern:    gradle catalog unreadable: <path> — test-tier evidence unavailable
   emitted by: kotlin (the version catalog at gradle/libs.versions.toml
               does not parse; the junit-family evidence leg reads
               catalog aliases, so the test tier degrades to
               no-evidence until the catalog is readable)
  meaning:    the dependency catalog the build scripts reference cannot
               be read, so `testImplementation(libs.junit)` style
               evidence resolves to nothing and no module can prove the
               test-tier fact through the catalog. Test-tier attribution
               falls back to the inline-coordinate evidence only; a
               `*Test.kt` name alone states no tier (the shipped
               framework-evidence-not-name law). The scan still answers
               every other fact.
   origin:     tree shape (a malformed catalog file)
  severity:   report-only (exit 0)
  follow-up:  archspec scan      (repair the catalog TOML; the line
               disappears when the catalog parses)
  decision:   none required; repair the catalog when test-tier facts
               matter at this root.

boundary advisory (report-only, never a verdict — exit 0):
  pattern:    boundary advisory: zone <zone> splits the package <package> — <n> member files outside
   emitted by: kotlin, at scan time, when the tree carries a spec (the
               driver's own module map plus the spec's declared module
               grouping decide zone membership; the other drivers never
               print this line)
  meaning:    a declared zone claims a package whose member files sit in
               modules outside the zone. Kotlin permits the split, and
               the member census depends on build state for generated
               sources, so this is an ADVISORY, never a finding: the
               generic verify-time constraint over this predicate is
               deferred until a customer tree proves the census firing
               outside build-state noise. The line states the shape.
   origin:     spec × tree shape (a declared zone and the package census)
  severity:   report-only (exit 0)
  follow-up:  archspec scan      (align the zone's units with the
               package's declaring modules, or accept the advisory)
  decision:   none required; move the zone's unit list or the package
               when the split is not intended.

java sources not indexed (report-only, never a verdict — exit 0):
  pattern:    java sources not indexed: <n> files — the kotlin driver indexes .kt only
   emitted by: kotlin (a SHAPE CENSUS — the count of `.java` files under
               the scan root, stated without indexing any of them; the
               line never fires at zero)
  meaning:    the gradle kotlin plugin ships java interop: a Kotlin tree
               can carry real java sources whose imports and code the
               model does not carry. The count makes that absence
               visible — no java driver is implied and no detection
               changes. A mixed tree needs its java story told
               explicitly, not silently mis-modelled as Kotlin.
   origin:     tree shape (`.java` files below a kotlin root)
  severity:   report-only (exit 0)
  follow-up:  archspec scan      (the java files keep living where they
               are; the line states their count only)
  decision:   none required; the count is a census, not a verdict.

orphan file (report-only, never a verdict — exit 0):
  pattern:    orphan file: <path>
              orphan file census: <n> of <m> scanned files — the model lists no fact for any of them
   emitted by: ts (the walk set minus the three fact sets; the summary
               line never fires at zero; disjoint from `orphan diagram:`,
               the render-artefact class of `diagram --modules-dir
               --check` — different noun, different domain)
  meaning:    the model lists NO IMPORT-GRAPH FACT for this file: it is
               absent from the internal-edge endpoints (both ends), from
               the external-attribution keys and from the wildcard keys.
               Read it precisely: NOT "the model does not list the file"
               and NOT "the file has no edges" — the export-symbol tier
               (`module_public_types`) is not one of the three fact sets,
               so half of this class DOES appear there. Two attribution
               blind spots can suppress edges and must never be read as
               "this file depends on nothing": a generated-attributed
               member import (subpath resolution landing on gitignored
               build output states no edge — tracked separately), and a
               self-contained entry point — entry points that
               import NOTHING are listed. (require() sites now state
               edges: the require-tracing round closed that blind spot.)
   origin:     tree shape × model facts (the walk set minus the fact sets)
  severity:   report-only (exit 0)
  follow-up:  archspec scan      (wire the file, delete it, or exclude
               its tree — the census states the class, it gates nothing)
  decision:   none required; the census is a census, not a verdict.

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
               [capability root-module-declarations rust="granular" csharp="not-emitted" go="not-emitted" ts="declarations, no gating facts" python="declarations, no gating facts" kotlin="not-emitted"]
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
               [capability module-tier rust="granular" csharp="granular" go="granular" ts="granular" python="granular" kotlin="granular"]
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
               [capability module-tier rust="granular" csharp="granular" go="granular" ts="granular" python="granular" kotlin="granular"]
  origin:     code (the conduit exists in source)
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec depgraph (trace the path), archspec inspect the hops
  decision:   spec-fix if the intermediate is a real boundary you forgot to
              declare; else code-fix to remove the conduit hop.

dead reference (boundary / stereotype reference)
  pattern:    dead reference: module '<name>' <field> target "<target>"
              exists in source but undeclared — references resolve to declared
              claim names at any depth
  pattern:    dead reference: module '<name>' <field> target "<target>"
              does not exist — create it or fix the reference (did you mean: ...)
  pattern:    dead reference: module '<name>' <field> target "<target>"
              not verifiable from source — driver emits no module-tier fact
              for this tree (did you mean: ...)
  emitted by: rust, csharp, and go; on runs whose capability row carries no
              module-tier fact the absent target is called "not verifiable
              from source" instead of claiming "does not exist"
              [capability module-tier rust="granular" csharp="granular" go="granular" ts="granular" python="granular" kotlin="granular"]
  meaning:    an allowed.depend_on / allowed.forbidden target can never engage a
              declared boundary, so the rule silently verifies nothing.
  origin:     spec
  severity:   the `does not exist` class: error (exit 1 without --strict, the
              line identical under --strict); the two other classifications:
              warning (exit 0; exit 1 under --strict)
  follow-up:  archspec scan / archspec depgraph (confirm the real module name)
  decision:   spec-fix — correct the reference, or declare the module it names.

dead contract target
  pattern:    dead reference: module '<name>' contract.forbid target "<target>"
              names no stereotype — declare a stereotype named "<target>" or fix
              the reference (did you mean: ...)
  pattern:    dead reference: module '<name>' contract.expose target "<target>"
              names no stereotype — declare a stereotype named "<target>" or fix
              the reference (did you mean: ...)
  meaning:    a contract.forbid names a stereotype that does not exist (top-level
              or submodule), so the contract can never fire; or a contract.expose
              entry names no stereotype and globs no symbol anywhere, so the entry
              engages nothing (the forbid-parity classification).
  origin:     spec
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec scan (stereotypes)
  decision:   spec-fix — declare the stereotype, or fix the contract target.

stale contract export
  pattern:    stale contract export: <claim> declares <entry> (never exported
              across its territory)
  meaning:    a contract.expose entry globs zero across the declaring claim's
              whole territory — own files union descendant exports — judged per
              entry, so a live entry never masks a stale one. An expose entry is
              a permission, not a requirement: staleness is warning-class.
  origin:     spec (entry deleted, renamed, or misspelled against the facts —
              the entry is spelled in the fact vocabulary, see spec.md)
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec scan   (the module public-API facts list the real surface)
  decision:   spec-fix — drop or fix the entry; code-fix when the export was
              meant to exist.

ambiguous expose entry
  pattern:    ambiguous expose entry: <claim> entry <entry> names stereotype
              <name> and matches symbols
  meaning:    a contract.expose entry both names a declared stereotype and
              matches real symbols by spelling; the stereotype arm adjudicates
              and the entry is flagged — never a silent symbol pass.
  origin:     spec (an entry that means two things at once)
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec help spec (the expose chain law's classification order)
  decision:   spec-fix — rename the entry or the stereotype so the entry means
              exactly one thing.

vacuous constraint
  pattern:    vacuous constraint: [constraint #N] <type>: <detail>
              vacuous constraint: [contract.expose of <claim>]: <detail>
              (rendered 'warning: vacuous constraint: ...')
              no_cycles file tier (scope = "both"): the detail states the
              file adjacency is empty among the constraint's files, or the
              driver emits no file-grained-dependency-facts fact and the
              carrier names that capability (capability not-emitted).
  meaning:    a constraint matches no module, edge, manifest or export — it
              checks nothing while appearing to pass; or a claim's
              contract.expose symbol arm runs on a driver with no module-tier
              public-API facts, which the carrier states instead of a bare
              vacuity (the named-subject carrier beside the indexed one); or
              a no_cycles file tier whose file facts are absent or whose
              file adjacency is empty — a green that would lie, stated
              instead (the carrier keys on the facts, the capability row
              only names the reason).
  origin:     spec
  severity:   warning (exit 0; exit 1 under --strict)
  follow-up:  archspec scan  (ground the constraint in a real module/edge);
              archspec help capabilities for the fact source a driver states
  decision:   spec-fix — fix the 'from'/'modules'/targets so it engages, or
              delete the constraint.

note: the model tiers root_public_exports and root_glob_exports are not reported
as standalone lines; they surface through public api leak, unverifiable glob
export and empty glob export above.
"#;
