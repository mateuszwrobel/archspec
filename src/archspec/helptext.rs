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
