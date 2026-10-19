# Changelog

All notable changes to archspec are documented here. The format follows
Keep a Changelog; versions are release tags. Entries describe the tool
only.

## [Unreleased]

## [0.6.30] - 2026-10-19

### Fixed

- Windows release builds compile again. The Windows job compiles the `cc`
  build-dependency for a Windows host; the locked `find-msvc-tools` 0.1.13
  exposed its `windows_sys::FILE_ATTRIBUTE_TEMPORARY` constant as `i32`,
  and the pinned `cc` passes that constant directly to the standard
  library's `OpenOptionsExt::custom_flags`, which takes `u32` — a type
  error that failed the Windows build while every other platform stayed
  green. The lockfile now resolves `find-msvc-tools` 0.1.15, which exposes
  the constant as `u32`. No behaviour of the tool itself moved.

## [0.6.29] - 2026-10-18

### Added

- Self-dogfooding e2e coverage harness: `tests/e2e_matrix.rs` registers every
  documented verb×mode cell (contract × hermetic corpus), executes the real
  binary over each cell, and witnesses the exit class plus stdout/stderr
  line-class facts; the generated `e2e-matrix.json` is freshness-checked like
  `feature-matrix.json`, and the registry is welded to the live help bytes in
  both directions. `tests/python_driver.rs` witnesses the python driver at
  binary level; `report --pretty` now runs in a cell with the byte-identity
  witness; `tests/goldens/derivation.json` + `tests/golden_derivation.rs`
  make golden freshness derived (content fingerprint + named derivation
  event) instead of asserted by prose. No engine behaviour strings moved.

### Notes

- Release-prep sync: the Cargo.toml version-line release marker now points
  at this section (`under [0.6.29]`) — the tag number, which is the heading
  the release-notes extractor keys on; the blind5 version-line weld pins
  the same bytes.

## [0.6.28] - 2026-10-18

### Changed

- Rust trait references now appear as ordinary dependency edges: trait paths
  in `impl ... for ...` headers, trait paths of `dyn Trait` bounds, and type
  paths nested inside generic arguments are extracted like value imports.
  A trait path that starts with a reserved segment (`crate`, `self`,
  `super`) resolves at the module tier and adds a file-level `path` edge;
  a trait path with any other first segment resolves at the unit tier and
  adds a unit edge only when the trait lives in another unit. External-crate
  traits, self-impls, generic and blanket impls, and stdlib targets emit no
  edge — trait positions never add external rows. The previously invisible
  set of trait edges can newly complete cycles or cross declared boundaries
  in `verify`, so a spec that was green can turn red on this change.

## [0.6.27] - 2026-10-18

### Changed

- ~1,184 specifiers' worth of previously invisible member edges can newly
  cross declared boundaries or complete cycles in `verify`: a member-subpath
  import whose mapped target is ABSENT and gitignored — the exact state
  that printed `generated source not built` — is now attributed to the one
  existing SOURCE file the winning exports entry declares under its
  `types` arm (`*` captures expanded from the same single key match). The
  import replaces that line with the existing `workspace target not built`
  disclosure, whose first slot now spells the member-qualified specifier
  (`workspace target not built: @org/ui/gen/map ->
  packages/ui/dist/gen/map.js`). Zero candidates, two or more candidates
  or a `types` arm spelling only `.d.ts` declarations keep today's line
  VERBATIM — archspec never position-picks and never infers paths.

## [0.6.26] - 2026-10-18

### Changed

- Internal structure: the three bare `unwrap()` sites the self-audit named —
  the mermaid tree endpoints in `inspect::mermaid` and the SCC pop in the
  `verify::compare::cycles` Tarjan walk — now carry named invariant
  messages (self-audit F5). Each invariant is enforced by construction:
  `build_tree` creates a directory only as an ancestor of a file, and the
  Tarjan root is on the stack at its own close, so no failure path changed
  and none is reachable from input. Zero bare unwraps remain outside the
  test modules. No behavior, output or exit-code change.

## [0.6.25] - 2026-10-18

### Changed

- Internal structure: the module-wide `#![allow(dead_code)]` in
  `archspec::parse` is gone. The allowance now rides the single documented
  item it covered — `grammar::first_error_range`, the error-tolerance probe
  exercised by the grammar tests and deliberately not called from a command
  path — as an item-level `allow(dead_code, reason = ...)` (self-audit F4).
  The dead-code lint watches every other item in the module again, and the
  F7 dead-signal census now covers the `parse` surface at zero candidates.
  No behavior, output or exit-code change.

## [0.6.24] - 2026-10-18

### Changed

- Internal structure: the eight scan-driver `Model` literals and the model
  tests' empty-model helper now build on one scaffold constructor
  `Model::new(language)` — schema version 1, the scanned language, every
  collection empty and every optional absent, stated once in `model.rs`
  (self-audit F3). Each driver site lists only the fields it populates
  (`..Model::new(language)` update syntax); the find-replace field-order
  fossils the rust, go and csharp literals carried are gone, and a new model
  fact lands empty in the constructor instead of at nine call sites. The
  constructor states exactly the field values the collapsed literals stated:
  every scan output stays byte-identical. No behavior, output or exit-code
  change.

## [0.6.23] - 2026-10-18

### Changed

- Internal structure: `verify/compare.rs` now lives as the module directory
  `archspec::verify::compare` — a thin `mod.rs` holding `compare` and its
  tests, plus twelve cohesive submodules (`diff`, `mapping`, `claims`,
  `pattern`, `edges`, `adjudicate`, `contracts`, `cycles`, `surfaces`,
  `external`, `boundaries`, `engagement`), every one under 1k lines. The move
  is verbatim: the `verify` public surface re-exports the same names, every
  scan/verify/report/diagram output and exit code is byte-identical on all
  corpora, and the crate's own `no_cycles` gate stays green — the file-tier
  import ring the single file carried is gone. No behavior, output or
  exit-code change.

## [0.6.22] - 2026-10-18

### Changed

- Internal structure: the parallel fact-leg machinery now lives in the leaf
  module `archspec::scan::leg` and the ts exclusion/resolution vocabulary in
  the leaf module `archspec::scan::ts::common`. The scan host and its
  language drivers call down into these leaves and nothing calls back, so the
  import ring the scan module graph carried is gone — no behavior, output or
  exit-code change. The crate's own spec now gates the area as five boundaries
  (`scan-core`, `scan-leg`, `scan-ts`, `scan-python`, `scan-kotlin`) under a
  dedicated `no_cycles` row and exact `depend_on` tables, so a reintroduced
  back-call fails `archspec verify` on the spot.

## [0.6.21] - 2026-10-18

### Added

- **npm channel**: the release now also ships an npm installer package.
  `npm install -g @scope/archspec` (the scope is set per repository) installs
  the same binary as the GitHub release assets — one archive per platform
  pair, downloaded by the platform table in
  `npm/lib/matrix.mjs` and checked against the release's SHA256 sidecars
  before it is cached or run. The package carries no binaries; the archive
  for the installing machine's platform is fetched at install time, from the
  cache (`~/.cache/archspec/`) when already present. Windows installation
  needs PowerShell 5.1 or newer. `archspec upgrade` (cargo install) remains
  the default documented path.
- The release workflow gained a `publish-gate` job: a tag whose version has
  a published npm version with different content fails before any build
  starts (the republish freeze), and the same check re-runs before the npm
  publish step.

## [0.6.20] - 2026-10-18

### Notes

- Round-2 fan-out (decision-check round 2, verdicts ADOPTED): the
  `help diagnostics` catalog, the claim-registry welds, the capability
  docs enumeration and the cross-driver matrix harness were aligned with
  the census in the same release; the matrix silence-law census strip is
  driver-scoped to ts. Version lands at 0.6.20 under the auto-bump law
  (the census commit itself carried 0.6.19).

## [0.6.19] - 2026-10-18

### Added

- **ts driver orphan-file census**: a scanned ts source file the model
  lists NO fact for now states itself on the scan/verify report surface.
  A fact is membership in exactly three sets — an endpoint (either end)
  of an internal edge, an external-attribution key, or a dynamic-import
  wildcard key. The observable outcome: one report-only
  `orphan file: <path>` line per such file (sorted, deduped) and one
  closing `orphan file census: <n> of <m> scanned files — the model
  lists no fact for any of them` summary that never fires at zero. The
  lines are stderr-only, never the model JSON, stdout or any artefact,
  and never a verdict (exit 0). The capability table gains an
  `orphan-census` row: ts `granular`, every driver without the arm
  `not-emitted`. The census computes strictly after the named extraction
  tripwire and never feeds its arms; the `help diagnostics` catalog
  entry names the reading rules and the blind spots (generated-
  attributed member imports; self-contained entry points that import
  nothing are listed; `require()` sites now state edges).


## [0.6.18] - 2026-10-18

### Changed

- A literal `require("...")` whose target resolves to nothing now aborts
  the scan like any literal import — the `unresolvable import:` line names
  file and specifier, exit 1. Before this release the site was not
  traced at all, so such trees finished green.

### Added

- **ts driver `require()` tracing**: a literal `require("lit")` in any
  ts-family file (every scanned extension and every Svelte `<script>`
  block) states an ordinary value edge through the ordinary resolution
  law — the same file edges an `import` mints, with no symbols and never
  a republication. A computed `require(expr)` states the existing
  `dynamic import wildcard` report line and rides the model's
  `dynamic_import_wildcards` census: report-only, never an edge, never
  red. `require.resolve`, `require.cache`, `jest.requireActual` and
  `jest.requireMock` are not require sites. The pilot corpus of one
  multi-package monorepo gains the 94 require edges the previous scan
  left blind; the second corpus (SvelteKit) is byte-identical.

## [0.6.17] - 2026-10-18

- The kotlin driver's pilot config landed; the version ride continues — no release cut.

## [0.6.16] - 2026-10-18

### Added

- **The kotlin driver**: `language = "kotlin"` scans gradle projects.
  `settings.gradle.kts` / `settings.gradle` is the module list — every
  `include(...)` names one unit, a nested project path `:a:Tests` folds
  to the unit `a::Tests`, and a tree carrying both settings files fails
  closed before anything is scanned. Each unit's `build.gradle.kts`
  states its declared dependencies: `project(":shared")` is a hard edge,
  a coordinate `group:artifact` — or a `libs.alias` entry resolved
  through the version catalog — is an external. Every `.kt` file's
  `package` and `import` headers state the source facts; `.kts` scripts
  are fact-legs, never vertices, and `.java` files are counted, not
  indexed. The module tier is unit-identified (the kotlin driver record
  K-P2): a gradle project IS the module a spec addresses, so modules,
  boundaries and the graph views read the gradle project graph directly.
- Kotlin was added to the language-detection chain at the LOWEST rank
  (the python precedent): gradle files or `.kt` sources answer kotlin
  only when no higher-ranked language claims the tree, so a corpus that
  predates the driver cannot shift language under it.
- The `verify` surfaces reach kotlin: unit and module boundaries, cycle
  checks, externals (coordinates from the build scripts; the top segment
  of an import names a plain external), the vacuity law (its carrier is
  a MISSING build script — a script stating an empty `dependencies { }`
  block states a fact, the ts/python C5 shape verbatim), and the
  test-tier law (a `src/test` or `src/androidTest` source set carrying
  test-engine evidence states a test file). `update seed` and
  `spec init` emit the gradle shape.
- The modules and api-usage graphs render a model whose module edges
  name their endpoints directly — when neither prefix fold separates
  the endpoints, distinct endpoints keep distinct nodes instead of
  colliding into `root`.

## [0.6.15] - 2026-10-17

### Changed

- The ts and python drivers' **per-file fact leg runs across worker
  threads** (scoped std threads, no new dependency): a file's read and
  parse is pure, so it parallelizes over the walk-ordered file list,
  while resolution and the fold stay strictly serial. The fold replays
  the per-file results in walk order and returns on the FIRST
  per-file error — a later file's findings can never exist when an
  earlier file errored — so stdout, stderr and exit codes are
  byte-identical to the previous serial loop on every scanned tree
  (measured 8.1 s → 1.2 s on a 13k-file ts monorepo; trees below 128
  files keep the serial loop unchanged).
- The ts resolver **memoizes the manifest-unreadable verdict per
  probed manifest path**: guard G's upward walk re-probed the same
  paths across (dir, package) cache keys (measured 253,226 `stat`s on
  a large monorepo resolve); the verdict is a function of the path
  alone over the committed tree, so the predicate is probed once per
  path per scan. The existing (dir, package) walk-answer cache stays —
  a per-path predicate cache and a walk-answer cache are different
  caches.

## [0.6.14] - 2026-10-17

### Breaking

- **The `svelte.config.js` presence probe is deleted.** The static
  implicit `$lib` alias no longer engages when the file merely exists
  at the scan root — file presence was auto-discovery, and framework
  auto-discovery is out of scope by ruling. The signal is now the
  declared key: `[project] framework = "sveltekit"` registers the `$lib`
  alias and the paired tsconfig framework behavior. Migration (the
  measured case is open-webui): add `framework = "sveltekit"` next to
  `language = "ts"` in `architecture.spec.toml`; without the key the
  tree stays green but `$lib` imports return to the external tier and
  their edges leave the model.

### Added

- **`[project] framework`** — an optional single name declaring which
  framework's path conventions the tool models. The registry is
  driver-side and computed (`archspec spec` renders it): the ts driver
  registers `sveltekit`, every other driver registers nothing. An
  unregistered name is an `invalid spec` error at spec load on every
  verb, naming the driver's registered alternatives (`none` where the
  driver registers nothing).
- **One report-only line for `$`-shaped specifiers without the key**:
  `specifier not a package name: <verbatim> — framework conventions
  not configured?` — a `$`-prefixed bare specifier cannot be an npm
  package name, so the generic external delegation asks the wrong
  question. The line states the unmodelled shape; attribution stays
  exactly as before (no laundering, verdict unchanged), deduplicated
  per importing dir and spelled specifier like the sibling lines.
- **Generated-class exclusion additions** `.next`, `.nuxt`, `.astro`
  join the mechanical exclusion list (exclusions only — exclusions
  never become auto-discovery; the pytest ruling is unchanged).

### Changed

- Docs fan-out: the capability errors surface no longer enumerates the
  languages (`language not a driver name`); the list follows the driver
  registry (schema enum drift fix, F11).

## [0.6.13] - 2026-10-17

### Changed

- A **bare-resolution memo per (importing dir, specifier)**: the edge
  loop re-entered the bare-classification seam for repeated pairs
  (measured 819 repeat calls of 43,652 on a large monorepo resolve,
  ~235 µs each); one classification per pair per scan now. The verdict
  is a function of the pair over the committed tree, and every
  side-effect channel the seam writes (report-only lines, fault and
  manifest findings, resolver entries) is a deduped set — a memo hit
  replays the identical verdict and owes nothing further. The memo key
  keeps the importing dir, so the per-file config law (the same alias
  spelled from two dirs resolving through two configs) stands
  byte-identical; BARE-MEMO-KEY trap fixture.

## [0.6.12] - 2026-10-17

### Changed

- A **member-manifest memo per member dir**: every member-classify pass
  re-read and re-parsed the member's `package.json` (measured 24,293
  manifest reads on a large monorepo resolve); one parse per member dir
  per scan now. The memo is keyed by the member dir, so two members with
  different manifests never answer from each other's parse; a manifest
  that exists but fails to parse is never memoized — the re-read keeps
  the exact fail-open shape of the pre-fix read.

## [0.6.11] - 2026-10-17

### Changed

- A **generated-source verdict cache per base path**: the candidate loops
  re-probed the same absent bases through the ignore walk; each base now
  walks once per scan. The verdict is a function of the path alone over
  the committed tree, so the memo cannot drift from the walk it replaces.

## [0.6.10] - 2026-10-17

### Changed

- A **`.gitignore` parse cache per scan**: the generated-source test walks
  the ancestor chain of every absent candidate; every walk step used to
  re-read and re-parse every ancestor `.gitignore` (measured ~6 opens per
  query on a large tree). One parse per file per scan now; a missing or
  unreadable file is cached as a no-file entry, keeping the fail-closed
  law. Verdicts are byte-identical: deepest-file-wins, last-match-in-file,
  negation, and the excluded-parent no-re-entry law are unchanged.

## [0.6.9] - 2026-10-17

### Added

- A **python driver**: `archspec scan` now detects and extracts python
  trees. Parsing rides the `tree-sitter-python` grammar; resolution runs
  with NO interpreter, NO venv and no version modeling — tiers resolve
  OWN PACKAGE → STDLIB → EXTERNAL from the filesystem alone. OWN keys on
  directory existence under the package root (a package-root directory
  named like a PyPI import name claims first-party — a named deviation
  from `sys.path`, zero such sites measured on the pilot); the stdlib
  tier is a static union of the CPython 3.9–3.13 top-level names, so a
  list miss can only drift the attribution label, never a verdict — a
  missed stdlib import stays an EXTERNAL fact. `from a.b import c` is
  submodule-or-attribute ambiguous: when `a/b/c.py` or `a/b/c/__init__.py`
  exists the edge also lands on that module. A bare namespace-directory
  import (PEP 420) is a first-party module-grain edge, never red.
  Dynamic imports (`importlib.import_module`, `__import__`) print
  report-only `dynamic import not modelled:` lines — no edge, never a
  finding. The package root is its own resolution root when it holds
  sources; otherwise a single all-sources child is the root, or every
  top-level source-bearing directory is one.
- **Python manifests**: a root `pyproject.toml` states the unit
  (`[project]` name, dependencies plus dependency-groups/extras);
  `requirements*.txt` ADD to the dependency facts. Manifests are read at
  the scan root only — no walk-up. One pyproject is one package unit.
- **The python test tier**: `test_*.py` / `*_test.py` / `conftest.py`
  files are nodes and import sources, marked test-gated only when the
  root manifest ALSO states a pytest-family dependency — co-occurrence
  or nothing.
- `pyproject.toml` joins ROOT_MARKERS. A root carrying both
  `package.json` and `pyproject.toml` now answers `ambiguous root
  markers` to a no-pin scan — the detection record's landmine, realized;
  the remedy is `[project] language` pinning or a per-root scan. A
  multi-marker set naming ONE language resolves to that language.
- `help languages`, the capability matrix, `doctor`, `depgraph`,
  `diagram` and the feature matrix gained the python rows; module-tier
  file-identity rules ts pioneered (the D10 shape: file vertices,
  `::`-restated submodule identity, per-file seeds) now read python
  trees through the same arms. New fixtures PY-*.

## [0.6.8] - 2026-10-17

### Changed

- Language detection now consults ROOT MARKERS first: a `Cargo.toml`,
  `package.json` or `go.mod` AT the scan root names the driver outright.
  Several root markers with no pin answer the new error
  `ambiguous root markers: <path> (<language>), … — pin [project] language
  or scan a single-language root`; a spec pin selects the driver exactly
  as `verify` always pinned, and a pin whose root marker is absent while
  foreign markers stand reddens with the `language mismatch:` byte prefix
  naming the markers found. With no root marker the shipped fallback chain
  runs byte-for-byte unchanged; the tree-wide agreement check is unchanged.
  A foreign root-marker file BELOW the scan root now emits one report-only
  line per marker file, `nested foreign marker: <path> (<language>) — not
  modelled`, exit 0 — behind the mechanical exclusion set, never a finding.
  New fixtures DET-ROOT-PIN-*, DET-ROOT-AMBIGUOUS*, DET-PIN-NO-ROOT-MARKER,
  DET-NESTED-FOREIGN-REPORT, DET-NESTED-EXCLUDED-SILENT, DET-STRAY-FLIP,
  SRC-MJS-CJS-INDEX, SRC-MJS-NEXTMAP, SRC-MTS-CTS-INDEX.
- The ts driver's source family gained the Node resolution flavours:
  `.mjs`/`.cjs` (JavaScript grammar) and `.mts`/`.cts` (TypeScript
  grammar) are counted, parsed and linked like the shipped extensions —
  no new grammar crate; the flavour is a resolution property, not a
  language one.

### Added

- `help diagnostics` gained the `ambiguous root markers` and
  `nested foreign marker` categories; the claim registry and the
  every-emitted-category guard welded to both.

## [0.6.7] - 2026-10-17

### Changed

- TS resolution adopted the generated-tsconfig pairing law, implemented
  together. An ABSENT `extends` target now answers three honest facts: a
  target absent under a generated-output directory (the generated class of
  the mechanical exclusion list — `.svelte-kit`, `dist`, `target`, `bin`,
  `obj`) emits one report-only line `generated tsconfig absent: <path>`,
  exit 0; a target that resolves through node_modules style with no
  installed copy emits one report-only line
  `tsconfig extends not installed: <specifier>`, exit 0; every other
  absence keeps the `tsconfig not found` red byte-identical. A tree with
  `svelte.config.js` at its scan root (file presence is the signal — the
  file's content and alias override are never read) gains the static
  implicit alias `$lib → <root>/src/lib`, consulted where nothing else
  answered: an unbuilt generated tsconfig can no longer silent-mistier the
  `$lib` surface. The line and the alias are PAIRED — either half alone
  launders the external tier behind a quiet report. New fixtures
  TSCGEN-SVELTEKIT-ABSENT with its red twin (non-generated absence),
  TSCGEN-EXTENDS-NOT-INSTALLED and SVELTE-ALIAS-VACUITY.
- The mechanical `vendor` class narrowed: a `vendor` segment under the
  scan root excludes nothing — first-party glue spelled in ts lands its
  edges and asset leaf edges (`.wasm` and the rest) land again. One
  predicate law answers the scan walk and resolution alike.
  `node_modules`, `dist`, `target`, `.svelte-kit`, `.git`, `bin` and
  `obj` stay absolute exclusions, and a declared `excluded_sources`
  entry still wins everywhere with the shipped red text unchanged. New
  fixture pair VENDOR-SOURCE-EXT-UNDER-ROOT and VENDOR-ABSOLUTE-CLASSES.

## [0.6.5] - 2026-10-17

### Changed

- TS resolution program adopted three laws, implemented together.
  P-CTX (context-free member resolution): a bare specifier or subpath that
  names a workspace member resolves to that member through the registry
  MEMBERSHIP, with no importer tsconfig gate — the old context-first order
  let a missing `paths` entry on the importing side demote a member import
  to the external tier. The importer's tsconfig no longer decides the tier;
  a member import that the importing tsconfig cannot spell keeps a
  report-only line `undeclared member import: <from> -> <member>` (bare
  family only, hoisting-family gates keep their verdict). New fixture pair
  CTX-MEMBER-BARE-FROM-UNDECLARED and CTX-EXPORTS-WILDCARD-SUBPATH.
  P-DIST (source-always-for-members): a member target that is absent or
  sits under build output (`dist`, `build`, `out`) resolves to the member's
  SOURCE entry — the types-condition target first, else the main/module
  spellings with the build-output prefix stripped through the candidates
  table — and the edge lands on the source file with a report-only line
  `workspace target not built: <member> -> <declared-target>` once per
  pair. A literal target that exists outside build output still wins
  (P-LITERAL). New fixture pair DIST-TYPES-CONDITION-WINS and DIST-STRIP-MAIN.
  P-GENSRC (gitignore-evidenced generated-source class): an otherwise-red
  absent target whose path is ignored by the repository's committed
  `.gitignore` files is a repo-declared generated artifact: the scan exits
  0 with a report-only line `generated source not built: <verbatim-target>
  — gitignored <pattern-file>:<line>`, no edge and no external-tier entry.
  Without a committed `.gitignore` covering the path the red stands (fail
  closed). The evaluator is static — git never runs at scan time — and its
  semantics (parent-exclusion, deepest-file precedence, last-match-wins,
  negation, anchors, dir-only and `**` forms) are pinned against captured
  `git check-ignore -v` oracle data. New fixtures GEN-NOTBUILT-IMPORTS-MAP,
  GEN-NOTBUILT-ALIAS, GEN-PARENT-DIR-IGNORE, GEN-NEGATION,
  GEN-NESTED-PRECEDENCE, GEN-ANCHORED and GEN-RELATIVE, each with its
  vacuity carrier.

## [0.6.4] - 2026-10-17

### Breaking

- **Breaking: the rust `no_cycles` default scope flips from `claims` to
  `both`.** Rust now matches the ts posture: a file cycle inside one claim
  reddens `verify` at error severity with no `scope` written in the spec —
  the file tier joins the module tier in the default SCC. The counted
  residual was cleaned before the flip — the pre-flip probe counted zero
  intra-claim file cycles across the rust sources — so the flip lands green,
  and the serialized `file_edges` facts carry the adjudication. The posture
  table (`archspec capabilities`), the `capability matrix` and verify help
  surfaces, spec.md and the output docs restate the row, and ADR-023 records
  the named change. A spec that wants the old verdict writes
  `scope = "claims"` explicitly.

- **Breaking: the dead-reference `does not exist` class is an error now**
  (warning-green→red). An `allowed.depend_on` or `allowed.forbidden` entry
  naming a target that exists nowhere — a removed claim, a forward-declared
  target never created — printed
  `warning: dead reference: … does not exist …` and exited `0` without
  `--strict`. It now prints `dead reference: … does not exist …` as an
  error-class finding: `verify` exits `1` without `--strict` and `report`
  counts it as a violation. The line bytes are unchanged apart from the
  dropped `warning: ` prefix, and the line is byte-identical under
  `--strict` — the severity is declared, not promoted. The other
  dead-reference classifications (`exists in source but undeclared`, `not
  verifiable from source`, the contract-target `names no stereotype`
  entries) keep warning severity. The promotion covers the whole class at
  once per the design record (R3-11), so one verdict can never carry two
  severities by field shape. The spec.md rule line, the verify and report
  output docs and the `help diagnostics` severity line fan out to this
  law in the same change. Phase B of the ts-driver design record.

### Added

- The five remaining ts gaps close (Lane C of the
  ts-resolution-and-completeness design record). (i) A module claim whose
  selectors name files or a module identity now accounts for its
  package's units through the claim-unit fallback, so a banned edge routed
  through a claimed conduit barrel launders under
  `verify.forbidden_laundering` on ts exactly as on the other drivers —
  an unclaimed conduit stays off the pair graph and surfaces only through
  `unowned module edge endpoint` warnings. (ii) The ts scan reads the
  test-tier convention (`*.test.*`, `*.spec.*`, `__tests__/` directories,
  gated on test-framework evidence in the manifest) and states each
  package manifest's dependencies in `unit_manifests`, so
  `scan.unit_manifests` engages instead of resting inert. (iii)
  `forbid_submodule_dependency` engages on ts: file edges project onto
  their claim owners at the restated `::` submodule identity, a parent
  constraint bans edges between sibling submodules with the
  submodule-dependency label, and contract stereotype patterns match the
  file path and its restated form. (iv) A package-root `index.ts` that is
  pure republication derives `Role::Facade` at scan, so the root facade
  rule engages on ts through the roles map. (v) `inspect --file` renders
  file-level nodes and edges for ts, reusing the scan's module edges — no
  second resolver. Each closed cell carries its named vacuity carrier for
  the empty-evidence case (`scan.unit_manifests`,
  `verify.forbidden_laundering`, `verify.root_facade`,
  `verify.submodule_contracts`, `inspect.file_level`), vacuous never
  gating without `--strict`. The capability table, the feature matrix,
  the claim registry, ADR-017 and the verify output docs fan out in the
  same change.

- Workspace resolution joins the ts resolution seam (Lane R2 of the
  ts-resolution-and-completeness design record). An npm `workspaces` field
  (array or object form) or a `pnpm-workspace.yaml` at the scan root
  registers a workspace registry of member directories — globs expand
  one segment deep, membership is realpath-based. A workspace tree is a
  multi-unit tree: the root unit plus one unit per member directory,
  files attributed to the deepest owning directory, manifests collected
  per unit; a single-package tree stays byte-identical (its registry is
  empty). The realpath-wins precedence law adjudicates member imports:
  (i) a resolution landing inside a member directory classifies
  first-party even when a link stands between the root and the importing
  file, and a deep `@org/pkg/sub` import lands first-party on the member
  entry file; (ii) a member-naming import the importing manifest declares
  that resolves to a copy outside the member directory classifies
  external and fires the report-only line `workspace link missing:
  <member name> — resolved to <path> instead of the member dir` — the
  verdict is unchanged and the run exits 0; an undeclared hoisted copy is
  a plain external fact with no line; (iii) a declared member name whose
  member directory is absent from disk stays first-party — resolved onto
  the member entry through its manifest — and fires the report-only line
  `workspace link missing: <member name> — absent from disk`; (iv) an
  undeclared member-name import states `unresolvable import` naming its
  target and exits 1. Package-tier edges join the model `edges`: declared
  member-naming dependencies — any version spelling — from the root and
  member manifests become package-level edges between the units owning
  the two manifests, and the `no_cycles` projection takes the union of
  resolved-ownership edges and package edges, so a package cycle reddens
  where no file-level SCC exists. The `help diagnostics` catalog, the
  help-suite category weld and the claim registry fan out in the same
  change.

- The `ts` language arm joins the driver languages as the fallback of the
  fixed detection chain rust > csharp > go > ts. Detection now runs behind a
  mechanical exclusion set (`node_modules`, `dist`, `target`, `.git`,
  `vendor`, `bin`, `obj`, `.svelte-kit`), so dependency and build trees can
  never select a driver. A TS/JS/Svelte tree is detected as `ts`; the
  extraction arm states its absence loudly until the following change of
  the same design delivers it. Phase A of the ts-driver design record.
- The `[project] excluded_sources` spec key registers with the ts-driver
  design record: an array of `{ path, reason }` entries with both keys
  mandatory — an entry missing one, or carrying an empty value, is an
  invalid-spec error naming the entry and the key. Every run of `scan` and
  `verify` prints every active entry on the report surface as
  `excluded sources: <path> — <reason>`, so an exclusion can never shrink
  the analyzed set in silence. `scan` now reads a spec at the scanned root
  by root-only discovery — an invalid spec fails `scan` with verify's
  message class. Phase A of the ts-driver design record — detection-side
  consumption of the entries lands with the agreement guard.
- The language agreement guard lands on both `scan` and `verify` as a run
  precondition: with a spec at the tree root, the declared `[project]`
  language is the truth, and any counted source file outside it — after
  the mechanical exclusion set and every `excluded_sources` entry — fails
  the run loud with `language mismatch: <declared> vs <found> — <lang>:
  <n> files, ...` before any verdict, model or artifact (exit 1). The
  counted walk counts source-extension files only; marker files state no
  counts. Without a spec, chain detection stays the whole truth and
  nothing is checked. The `help diagnostics` catalog names the new
  fail-loud class. Phase A of the ts-driver design record — the entries
  registered by the previous entry are now consumed.
- The ts extraction arm delivers v1 file-grained facts: every scanned
  `.ts`/`.tsx`/`.js`/`.jsx`/`.svelte` file registers as a module vertex
  identified by its repo-relative path, and every import becomes a
  file-grained `module_edges` row. A `type` import carries the row with
  the new `type_only: true` marker (a mixed clause contributes exactly
  one value and one type row); the marker is absent from the JSON of
  every row that is not type-only, so rust, go and csharp model bytes
  are untouched. `.svelte` files parse through their `<script>` blocks.
  Bare specifiers resolve to the npm package identity in the external
  tier; relative specifiers resolve through extension and index
  inference and fail loud as `unresolvable import: <file> -> "<spec>"`
  when nothing is found (naming the entry when the only candidate sits
  in an excluded tree). A non-literal `import()` specifier is stated as
  a `dynamic import wildcard: <file> -> <expression>` report line and a
  model `dynamic_import_wildcards` fact — report-only, never a verdict.
  A package whose scanned files import but that carries no resolvable
  structure fails loud through the content tripwire. `package.json`
  dependencies feed the manifest facts and exported names the public
  facts. The grammar layer registers the pinned TypeScript, TSX and
  JavaScript grammars (ADR-022). Phase A of the ts-driver design record.
  This change's first commit attempt fired the version stamp: the
  `## [0.6.1]` heading and the `0.6.1` manifest and lock lines it wrote
  carry no tag, so they are reverted and the delta is registered on the
  version line instead — the version stays 0.6.0 through phase A.
- The capability table states every fact for every language: the nine
  dependency-facts rows now carry a `ts` column beside rust, csharp and
  go. The ts rows read `granular` for the module tier, symbols, external
  packages, public-api surface and the new `file-grained-dependency-facts`
  fact; `not-emitted` for role-facade, role-composition and test-tier (the
  ADR-017 investigation found no derivable role or test idiom); and the
  conditional `declarations, no gating facts` for root-module-declarations.
  The `feature-matrix.{json,md}` artefacts regenerate at four drivers, and
  the matrix guard that pins the matrix languages to the table languages
  makes the fourth column mechanical. The ts driver runs every shared
  scenario: where a tree shape has no ts dialect — multi-unit hard-edge
  trees, role-derived cells, manifest-only and go-test-file trees, the
  top-level module fold and the unit-manifest tier — an applicability
  predicate names the skip, never a fake shape. Phase A of the ts-driver
  design record.
- `module.allowed` entries accept the tagged shape
  `{ target = "<pattern>", type_only = "count" | "exempt" }` beside the
  bare string. The `type_only` key adjudicates the dependency tier:
  `count` (which is also the default) counts a type-only edge like any
  other edge, and `exempt` admits the pair's type-only crossing while the
  same crossing written as a value import stays red. The exemption lives
  on the rule row, never on the language: the cycle graph counts every
  edge regardless of kind, and on rust, go and csharp trees — whose edges
  carry no kind fact — every crossing is a value crossing, so `exempt`
  reddens their crossings exactly like `count`. A tagged
  entry with an unknown key, a value outside the closed set, or a shape
  that is neither string nor table is a load-time spec error naming the
  entry index, the key and the supported values.
- The ts driver keys `module_external` by the bare repo-relative file
  path — the same identity its module edges and spec claims use (the D10
  file-identity law) — and merges `package.json` declared dependencies
  into the `external` tier after the content tripwire, so a declared-but-
  unused package appears in `external` and never in `module_external`:
  the family's external law now reads the same for all four drivers.
  A `from` pattern naming the unit attributes every file key of the
  single package. `update` seeds a ts tree per file: one `[[module]]`
  per file vertex, matched by its path, allowed to exactly the file's
  real internal targets, plus the global `no_cycles` guard. Verify
  engagement now counts ts declaration names as module vertices, so a
  claim or `from` pattern naming an export-only or external-only file
  engages instead of reading unmatched. Phase A of the ts-driver design
  record.
- The `no_cycles` constraint accepts `scope = "claims" | "both"` — the
  adjudication tier selector of the ts-driver design record (D5). The
  field is validated at spec load like `severity`: a value outside the
  two supported values is an invalid-spec error naming them, and the
  field on any other constraint row kind is an invalid-spec error naming
  the field and the row kind. The JSON Schema types the enum beside
  `severity`. Scope adjudication lands with the following change of the
  same record; until then every scope value adjudicates at the shipped
  claim tier. Phase B of the ts-driver design record.
- The per-language posture table lands as declared data in the binary (D7):
  `ts → both/error`, `rust, go, csharp → claims/error` — the answer to an
  absent `scope` until the scope adjudication change engages it. The
  `capability matrix` surface gains one `posture <language> <scope>
  <severity>` row per language arm beside the capability rows, so the
  table is read by value, never restated, and the weld test binds every
  `both` posture to the granular `file-grained-dependency-facts` row. A
  new `capabilities` help topic makes the shipped vacuity pointer
  `archspec help capabilities` resolve; the topic list fans out to the
  help docs and the exit-taxonomy byte pins in the same change. The ADR
  agreement guard pins the binary's rows, the posture table and ADR-023's
  wording together. Phase B of the ts-driver design record.
- The `scope = "both"` adjudication engages the file tier (D5/D6): the run
  adjudicates the claim tier as today and then the strongly connected
  components of the file-grained adjacency — a type-only import is an edge
  in the cycle graph, an asset import never closes one. A file ring prints
  as `cycle: <claim>: <file> -> <file>` attributed to the nearest common
  claim ancestor of its members over the claims union the synthetic tree
  root; members under disjoint top-level claims carry no claiming segment
  and name every member. Each ring rotates deterministically to its
  smallest path. A file SCC whose claim projection is a cycle the claim
  tier already printed — every projected edge an edge of a reported claim
  cycle — is suppressed: one violation one finding, coarsest tier first.
  The `help diagnostics` cycle entry, the verify and report output docs
  and the `spec.md` constraint table fan out to these bytes in the same
  change. rust, go and csharp postures stay `claims` — their file tier
  rides the named flip change. Phase B of the ts-driver design record.
- The `no_cycles` vacuity carrier covers the file tier (D7): with the file
  tier in scope — posture-derived or explicit `scope = "both"` — a run
  whose file-grained dependency facts the capability table marks
  `not-emitted` prints the capability-naming carrier on the shipped
  `vacuous constraint` channel, and a run whose facts are granular but
  whose file adjacency is empty among the constraint's files prints the
  empty-adjacency carrier. The carrier keys on the FACTS, never the row's
  label: an empty adjacency states itself even when the row reads
  `granular` — no silent green. The `help diagnostics` vacuous entry and
  the verify output docs fan out to these bytes in the same change.
  Phase B of the ts-driver design record.
- `diagram --tiers files` renders the file tier (D8): with the spec source
  and a territory flag (`--module` or `--modules-dir` — the size law: one
  claim's territory per render, never the whole tree), the claim's
  subgraph gains a `files` cluster — one vertex per territory file by its
  repo-relative path with extension, one arrow per first-party file edge.
  Cycle arrows are marked through the adjudication seam's cycle-membership
  query, never a diagram-side SCC re-derivation, so the marks agree
  verdict-for-verdict with `verify`: the error bucket renders dashed
  `violates spec`, the warning bucket renders solid. Asset vertices render
  with their incoming edges only and never carry the mark — cycles close
  between source files only. `--tiers files` with `--source scan` is a
  usage error (exit `64`): the scan tier is the module tier; the seam
  marks through facts a scan artefact does not carry. A language whose
  driver emits no file-grained-dependency-facts fact states the vacuity
  on stderr and renders the module-view bytes — never a fabricated empty
  cluster. `files` joins the `--tiers` vocabulary (`units, modules,
  files`); every default and `--tiers modules` render stays byte-exact.
  The diagram cli, errors and input docs, the acceptance tier tables, the
  help surface and the claim registry fan out in the same change.
  Phase C of the ts-driver design record.
- The rust driver serializes file-grained dependency facts (the D11-named
  precondition of the rust posture): the model gains the additive
  `file_edges` key — `(unit, from, to, kind)` rows over repo-relative file
  identities with rust's truthful lexical kind vocabulary (`mod`, `use`,
  `path`). Resolved file-backed `mod` declarations source the `mod` rows;
  resolved intra-unit imports and qualified path uses source the `use`
  and `path` rows. Rust fabricates nothing it does not carry: there is no
  type-only split, no dynamic import and no asset edge, and a file
  adjacency the scanner cannot derive — an inline `mod` body, an
  `include!`, a macro-built tree, an unresolved declaration — serializes
  no row. Rows stay unit-internal; a cross-unit reference stays a unit
  edge. The `file-grained-dependency-facts` capability row reads
  `granular` for rust — welded to the serialized facts, never to a guess
  — and a `no_cycles` with an explicit `scope = "both"` adjudicates the
  file tier on rust's real facts. Verdicts stay byte-identical: the rust
  posture row stays `claims`. go and csharp scan bytes are untouched (the
  field serializes nothing when empty). The capability matrix, the
  `capabilities` help topic, the diagram files-tier cells, the verify
  output docs and the ADR-023 restatement fan out in the same change.
- The ts resolution seam lands: the ts driver resolves every bare and `#`
  specifier through the pinned `oxc_resolver` (`resolve_file`) instead of
  stating a name-only fact. Per importing file the seam takes the owning
  tsconfig from the bounded chain of Lane R1 part A and delegates with
  `TsconfigDiscovery::Manual` (references disabled — the crate's `Auto`
  walk stays unused); with no owning config within the bound the
  delegation runs alias-free. The condition set is `node` + `import` +
  `require` + `svelte` (no `types` — a types-only export map rides the
  exports miss) and the extension list is `.ts`/`.tsx`/`.js`/`.jsx`/
  `.svelte`/`.json`. Node builtins — the canonical name list, `node:`-
  prefixed or bare — classify external: never delegated, never
  unresolvable. Every `ResolveError` variant has exactly one home: dead
  alias targets, `#imports` misses and export-map misses ride the
  `unresolvable import` line naming their target, manifest or condition
  set; `tsconfig not found`, `tsconfig load failed`, `tsconfig cycle
  (extends or reference)`, `package manifest invalid` — fired even beside
  a successful legacy fallback — and the `unclassified resolver error`
  catch-all are named findings that exit 1; a resolution landing in
  `node_modules` or outside the scan root states the package identity as
  an external fact. A declared-but-absent package adds the report-only
  line `external not installed: <package> — declared in <manifest>,
  absent from disk`: never red, never in the model artifact. The `help
  diagnostics` catalog, the help-suite category weld and the claim
  registry fan out in the same change. Part B of the
  ts-resolution-and-completeness design record (Lane R1).

### Changed

- Part A (Lane R1) of the ts-resolution-and-completeness design record —
  plumbing only, no behavior change: the pinned `oxc_resolver` dependency
  lands at an exact pin (`=11.24.3`, the release the Lane R1 spike measured
  end-to-end — the measured basis of ADR-024), confined to the scan layer by
  the `forbid_external_crates` constraint so a resolver import outside the
  scan arm reddens the crate's own verify — law, not convention. The scan
  ts arm gains the bounded owning-tsconfig discovery: an importing file's
  owning `tsconfig.json` is the first one met walking upward from the file's
  directory, the walk consulting the scan root and stopping there — a config
  above the bound is never read and never reddened, and no config within the
  bound is the inferred project: alias-free and never a fault. The tsconfig
  pre-read leg validates the full `extends` closure (own config plus every
  named file, at ANY file name, deduped by canonical path) before any
  resolution runs, with three named classes: a present-but-unreadable or
  unparsable config, a missing `extends` target (the as-declared specifier
  beside the attempted path), and an `extends` chain that loops — the cycle
  is its own class, never a load failure. The pinned crate's
  `TsconfigDiscovery::Auto` is not used: the spike measured its walk running
  unbounded past the scan root and silently skipping unreadable candidates,
  so the chain and the pre-read are computed on the crate's exposed pieces
  (the never-silent law). Nothing consumes the two legs yet; the resolution
  verdicts are byte-identical until the resolver-seam change lands (workplan
  archspec_ts_resolution_completeness, Lane R1).

- The ts-resolution decision-check round-3 closure: the design record gains the manifest-fallback honesty extension (an unreadable package manifest on a resolution path is a named `package manifest invalid` error even when a legacy fallback resolves the import — fixture variant UNREADABLE-MANIFEST-FALLBACK), the OUT-OF-BOUND-BROKEN-ANCESTOR fixture variant pinning the scan-root discovery bound (a broken `tsconfig.json` one level above the scan root stays green with alias-free resolution), and an accurate release-posture sentence (no version-VALUE bump; the CHANGELOG delta is the meta rollup bullet only) — a docs-only delta with no behavior change

- The ts-resolution-and-completeness design record, decision-check round 2 adoption: the workplan gains the package-cycle UNION projection (resolved-ownership edges plus declared package-level dependency edges — a declared member-to-member cycle reddens even across registry copies), the node-builtins law (`node:`-prefixed and bare builtins classify external, never unresolvable), the scan-root discovery bound (configs above the scan root are never read and never reddened), the exhaustive resolver-error taxonomy (one named class per variant, a new `package manifest invalid` class and an `unclassified resolver error` catch-all that fails loud), the bounded pre-read set equal to the delegation read set plus the extends closure at any file name, and the declaration-only firing of the `workspace link missing` line; ADR-024 restates the bound, the attempted-target naming, the widened cycle name, the exhaustive mapping and the builtins consequence; two internal-crate names in scan-layer code comments were reworded generically so the published payload carries no internal identifiers — a docs-and-comments-only delta with no behavior change

- The ts-resolution-and-completeness design record, decision-check round 1 adoption: the workplan gains the realpath-wins workspace precedence law (a resolution beats a name — resolved-outside member imports ride the external classification with a report-only `workspace link missing` line), the six-class unresolvable taxonomy (adds `tsconfig extends cycle` and the export-map miss riding the `unresolvable import` category), the tsconfig pre-read law (a present-but-unreadable config reddens — never a silent alias-free green), the vacuity mandate for every flipping ts row, and the widened `node` + `import` + `require` + `svelte` condition set (CJS-only packages land on implementation files); ADR-024 restates the taxonomy, the precedence and the corrected tsrs rejection rationale — a docs-only delta with no behavior change

- The ts-resolution-and-completeness design record: the workplan and ADR-024 state the ts-driver resolution-layer laws (the pinned `oxc_resolver` resolution layer) and the five ts completeness cells as a docs-only delta with no behavior change

- The rust posture-flip design-record closeout: the ADR-006 deferred-language line, the ADR-022 driver-status line and the ADR-023 posture line state the implemented posture — the rust `no_cycles` default scope is `both` since 2026-10-16 with the zero-count fleet re-probe measured at the change — a docs-only restatement with no behavior change

- The rust file-cycle adjudication laws at the scanner and the verifier file tier: a `mod` declaration row is composition and never rows at file grain, a path that lands on a CONTAINING (ancestor) module of the using module — spelled `super::`, `crate::`-absolute or a bare name — is containment, not dependency, and contributes no file row, test-gated `use`/path references contribute no file rows, and name-match resolution never targets a containing module; the law keeps sibling hops and descendant-target rows intact (the tier-comparability law of the D6 projection dedup, ADR-023)

- **File-tier territory spelling projects file identities to module
  spellings before claim adjudication.** A dotted-module claim (`domain::models`)
  now matches its backing file (`src/domain/models.rs`) at the file tier — the
  file identity is projected to the module spelling before the claim lookup,
  the mirror of the cycle-count probe's projection (`file_module_identity` in
  the verify comparison). A test-gated file never participates in file-tier
  adjacency: the `mod tests;` declaration plus the test file's imports is a
  declaration-and-import pair, not a production ring (the one-case-one-verdict
  law at file grain, mirroring the module-tier exclusion of test-gated
  modules and the go and ts test-file law). The file tier stays off the rust
  default until the posture flip of the same design record, so both rules
  bind today only at an explicit `scope = "both"`.

- A bare first segment of a `use` path resolves by the declaring-file law:
  the name is in scope only when the file carrying the `use` declares that
  module (`mod name;` in the same file, inline or file-backed). The scan
  previously walked the module's ancestors and the crate root, so a
  dependency import inside a module named like the dependency (a module
  `tauri` with dependency `tauri`) resolved to the crate's own module — a
  false parent back-edge in the model and a phantom file cycle. Such an
  import now registers the dependency as external, exactly as Rust resolves
  it. The same law governs bare glob prefixes on the public-surface walk.
- Intra-claim file cycles removed in the archspec payload: `CommandInfo` now
  lives in the new leaf module `src/archspec/commands/registry.rs` and the
  `help` surface receives the command rows as data from the dispatch, while
  `FileGraph` now lives in the new leaf module
  `src/archspec/inspect/file_graph.rs` and the language file scanners import
  it directly. The command tree and the inspect tree import one way, so the
  file dependency graph has no strongly connected component inside a claim.
  No behavior change: the rendered help text, exit codes and diagrams are
  byte-identical; only the file layout of the imports moved.
- `archspec report` reads the language mapping from the canonical parser: a
  spec declaring `language = "ts"` now reports the way it verifies. The
  command's private table had lagged the phase A ts arm and answered
  `invalid spec: unknown language "ts"` while verify, scan and diagram
  accepted the same spec. The report binary carries a ts-acceptance test.
  Phase B of the ts-driver design record.
- both macOS release builds moved from the `macos-15` / `macos-15-intel`
  runner labels to `macos-26` / `macos-26-intel` — GitHub's macOS-14
  deprecation notice keeps moving the images forward, and the macOS 26
  images are the newest supported runner images per current GitHub
  runner docs.

### Docs

- ADR-022 records the TS/JS/Svelte native driver — the fourth `Language`
  arm, pinned grammar crates and `<script>`-block extraction behind the
  shipped model — and its revision line closes the TS/JS deferral clause
  of ADR-006. ADR-023 records the doctrine beside the planned `scope`
  field of `no_cycles`: the per-language posture table, cycle attribution
  to the nearest common ancestor, and the deep-cycle posture in which
  `warning` stays an explicit exception under the checked-in baseline
  guard only. Design record only — no tool behavior moves with this entry.
- The ts-driver and file-cycle design record carries its round-1
  decision-check: 20 findings adopted — the `type_only` dependency fact,
  the dynamic-specifier wildcard and asset-import edges, the posture-row
  and cross-check agreement law, the baseline guard, the facts-file
  identity and content-tripwire contract, and the blind-spot and coverage
  amendments to ADR-022 and ADR-023. The workplan gains the same
  amendments. Design record only — no tool behavior moves with this
  entry.
- The ts-driver design record carries its round-2 decision-check: 9 findings
  adopted — the declared-language truth with a divergence guard, the
  `[project] excluded_sources` key with mandatory reasons, the fallback
  detection position, the per-entry `type_only` law (default `count`), the
  capability/posture/docs agreement weld, the exclusion resolution law, the
  synthetic-root attribution, the asset vertices, and the honest
  before/after pair for the dead-reference promotion. The ADRs and this
  entry's round-1 wording move with the amendments. Design record only — no
  tool behavior moves with this entry.
- The ts-driver design record carries its round-3 decision-check: 11 findings
  adopted — the one-verdict resolution for excluded-path imports (an
  `unresolvable import` naming the exclusion, exit 1; the wildcard class is
  non-literal specifiers only), the projection-keyed cross-tier cycle dedup,
  the three-key schema registration (`scope`, `[project] excluded_sources`,
  entry-level `type_only`), the scan-side spec-reading mechanics behind the
  graph-is-artefact law, the excluded-tree report line with its owner surface,
  the spec-source files tier with the extended usage refusal, the
  `(from, to, kind)` edge rows, the site-keyed weld oracle with its two named
  law-change cells, and the whole does-not-exist dead-reference class. This
  entry's section moves back under `[Unreleased]`: the `## [0.6.1]` heading the
  version-stamp hook stamped during the round-2 landing carried no tag and no
  version line, and is deleted. Design record only — no tool behavior moves
  with this entry.
- The ts-driver design record carries its round-4 decision-check: two MINOR
  coherence findings adopted — the D12 schema-key count corrected to the
  three registered keys with the Breaking classes itemized, and the D14
  verify-side seam sentence stating the detection walk the divergence guard
  re-runs. Design record only — no tool behavior moves with this entry.

## [0.6.3] - 2026-10-17

### Added

- The ts driver maps NodeNext extension spellings archspec-side, in the
  relative-specifier candidate loop: `.js` also tries `.ts` and `.tsx`,
  `.mjs` tries `.mts`, `.cjs` tries `.cts` — the LITERAL spelling first,
  always. When both `x.js` and `x.ts` exist the edge lands on `x.js`:
  the runtime loads what it loads (the mixed-tree law). The mapping
  fires only after the literal miss, carries no `.svelte` row, and the
  delegated resolver's own alias table stays disabled — it would fire
  before the literal and error on twin-less trees.
- The ts resolution seam gained the virtual-specifier and query-suffix
  laws. A scheme-shaped specifier (`^[A-Za-z][A-Za-z0-9+.-]*:`) or a
  NUL-leading one is a virtual module id: never delegated, never
  stripped — the run records NO edge and NO external-tier entry and
  states one report-only `virtual specifier not modelled: <verbatim>`
  line, exit 0 (the measured fix: `https://esm.sh/lodash` no longer
  pollutes the external tier with the identity `https:`). A `?`-query
  on a relative, aliased or bare specifier resolves on the BASE: found,
  an ordinary edge with the external identity losing the query
  (`canvas-confetti?optional` names `canvas-confetti`); base absent,
  the ordinary miss class naming the FULL ORIGINAL specifier verbatim.
  The builtins law runs before the virtual test, so `node:` keeps its
  shipped external fact. A `help diagnostics` catalog entry, the claim
  registry row and the every-emitted-category guard carry the new
  line; no capability row flips with these laws.

### Changed

- The ts scanner decodes escape sequences in specifier strings: fragments
  and escapes join in order, so a virtual id spelled `"\0virtual:module"`
  arrives at the seam as the runtime NUL-leading string the law tests by
  name (previously the first fragment alone was taken and the leading
  escape was dropped). The silent-parser tripwire now also counts
  report-only lines as stated outcomes: a file whose only import is a
  virtual specifier exits 0 with its line, not tripwire red.

## [0.6.2] - 2026-10-17

### Added

- The ts resolution seam narrows its case-(iv) verdict for BARE
  undeclared member-name imports through a linker-family gate read from
  the lockfile at the workspace registry root (the lockfile is read as a
  family signal — file names only, never content, never a registry).
  A bare specifier naming a registered workspace member with no on-disk
  resolution, imported by a manifest that does not declare it, is now a
  first-party membership edge onto the first registered member for that
  name (nearest scan root first) when the root carries a hoisting-family
  lockfile (`yarn.lock` or `npm-shrinkwrap.json`) — the hoisting linker
  makes the import a real runtime edge, so the run honors the linker
  with one report-only `undeclared member import: <from> -> <member>`
  line and exits 0. Under `pnpm-lock.yaml` or no lockfile the same miss
  keeps the honest `unresolvable import` red, exit 1 — pnpm-strict
  refuses the link and red is the runtime truth there. Subpath misses
  (member name + path) keep the red under every family; non-member bare
  misses and the declared-member cases are unchanged. A `help
  diagnostics` catalog entry, the claim registry row and the
  every-emitted-category guard carry the new line; ADR-024's precedence
  sentence states the gate.

## [0.6.1] - 2026-10-17

### Documentation

- The ts-resolution design record carries its round-4 amendments (a
  post-landing regressions audit): the R5 case-(iv) narrowing with the
  hoisting-linker membership gate (a bare undeclared member import under
  a `yarn.lock`/`npm-shrinkwrap.json` family becomes a first-party edge
  with a report-only line; `pnpm-lock.yaml` or no lockfile keeps the
  honest red; subpath misses keep the red), the archspec-side NodeNext
  extension mapping (R10 — literal-first substitution in the ts candidate
  loop; the resolver-side alias table rejected on two measurements), the
  query-suffix and virtual-specifier law (scheme-shaped specifiers are
  never delegated and never pollute the external tier), and the
  mechanical corrections — ADR-024 flips to Accepted, the fan-out owner
  of the retired npm-workspace blind-spot entry corrected to the ADR-022
  record, the `tsconfig cycle` finding string narrowed to name the
  unread project references, and three behavior fixtures required for
  the alias/imports-map/export-map arms. Design record only — no tool
  behavior moves with this entry.

## [0.6.0] - 2026-10-15

### Added

- `contract.expose` is accepted at every spec depth (the reserved-key refusal
  retires): a module-tier claim's expose list is a complete public-surface
  allowlist for the symbols the scan shows in its territory, adjudicated by
  the deepest-claim chain law — one uncovered public symbol is the red
  finding `public api leak: <claim> exposes <symbol> (not exposed)`, and an
  entry that names nothing across its territory is the warning
  `stale contract export`; an entry naming both a declared stereotype and
  live symbols is the warning `ambiguous expose entry`. A claim whose expose
  arm has no facts to check (a Rust module tier carries no module-tier
  public-API facts) prints the vacuity carrier naming the claim instead of
  a green `ok:`.
- `archspec diagram --module <claim>` renders one claim's territory as a
  module graph, and `--modules-dir <dir>` writes one stable-named file per
  claim (dotted filenames through an injective escape, `%3a` for `::`).
  Public-API marks render through the same adjudication `verify` runs: an
  arrow is dashed exactly where a `verify` run reds the source's crossing.
  `--check` reports `out of date diagram` and `orphan diagram` findings
  against the batch directory and never deletes or names foreign files; a
  blocked write fails its own file, lands every other file complete, and
  exits operational (70).

### Changed

- Every failing `archspec verify` exit ends stderr with the footer
  `note: archspec <version>`; usage errors (exit 64) stay footer-free and
  byte-frozen.
- A `contract.expose` over a Go module tier with no public-API facts states
  its vacuity per claim instead of the silent pass; C# root-territory files
  contribute module rows to the expose adjudication.

### Breaking

- **Breaking: `archspec diagram --source scan` marks more edges dashed**
  (green→red-adjacent, D12). The marking law moved to the same adjudication
  the verdict engine uses: an edge that violates a `depend_on` ceiling —
  source outside the governed set, target not listed — is now dashed
  (it is the one Breaking class a scan diagram can show). Edges carried by
  a `depend_on` never gain a dash, so a fully governed crate's diagram is
  unchanged; a crate with a ceiling gap sees new dashes — the dashed set
  becomes a subset of the `archspec verify` failure set (the two can still
  differ: a diagram reports no violations, `verify` reports verdicts).
  The fix: list the real target in `depend_on` — or accept the dash as
  the documented gap. Module tiers (`--tiers`) render module nodes under
  their governing claim's name with the same marking law (added surface,
  no existing bytes change); the default scan bytes are byte-identical.

- **Breaking: a nested `allowed` table now adjudicates** (green→red).
  A `[[module.submodules]]` claim carrying `[module.allowed]` gates the
  dependency edges of the files it claims with the same jurisdiction a
  top-level table has — most-specific-claim ownership at every depth,
  verdicts identical to the equivalent flat spec. A spec whose nested
  table forbids an edge the source really takes verified green before
  this release and verifies red now. The fix: complete the nested policy
  — list the real targets in `depend_on` — or delete the table; a nested
  claim without one inherits the nearest ancestor's policy, so deleting
  an unintended table changes no verdict.
- **Breaking: a cross-depth tie is ambiguous now** (green→red). An
  equal-specificity conflict over one territory between a nested claim
  and a flat claim — the finding `ambiguous module match` — was resolved
  silently by declaration order before this release and fails now. The
  fix: list the ambiguous pair once — the finding names both claims; let
  one claim carry the territory so ownership resolves by specificity.
- **Breaking: the new depth findings bite under `--strict`** (green→red
  for strict runs). The engagement warnings introduced on this line —
  `unmatched claim:`, `shadowed claim:`, and the `dead reference` and
  missing-edge findings now emitted at submodule depth — exit 0 without
  `--strict` and exit 1 with it; non-strict runs keep passing. The fix:
  add the claimed path to the parent claim or fix the pattern, and
  delete or re-point the claims the warnings name.
- **Breaking: containment edges are never adjudicated now** (red→green).
  A dependency edge between a claim and a claim inside its own subtree
  (parent files reaching own-submodule files, either direction) is
  composition — never a violation, and never an allowance's proof — so a
  spec that failed only on such parent↔own-submodule edges verifies
  green now. Contract-only nesting is unchanged: a nested claim without
  an `allowed` table keeps the verdicts it produced before, through the
  nearest ancestor's policy.
- **Breaking: a dead module-tier claim warns instead of erroring**
  (red→green). A top-level claim declaring only `matches.modules` that
  matches no model element reported the `missing component:` error
  (exit 1) before this release; it now reports the `unmatched claim:`
  warning — exit 0, red under `--strict`. Unit-backed claims keep the
  `missing component:` error. The fix: re-point the dead module-tier
  claim at paths that exist, or delete it.

### Added

- Nested `[[module.submodules]]` claims now carry dependency jurisdiction.
  A submodule with an `[module.allowed]` table adjudicates dependencies
  inside its territory; a submodule without one inherits the nearest
  ancestor's policy. Most-specific-claim ownership decides every edge at
  every depth, with verdicts identical to the equivalent flat spec.
  Ancestors' `contract.forbid` binds descendants downward. Direct
  ancestor-descendant containment edges are never adjudicated. New
  `unmatched claim:` and `shadowed claim:` warnings (`--strict` red);
  `missing component:` now checks claim subtrees; the ok line counts all
  claims. A `matches.units` key on a submodule is rejected at parse.
  `diagram --source spec` renders submodule claims and allowance arrows
  at depth.

### Changed

- **Refactor: edge adjudication now runs through one seam** (no behavior
  change). The pair-verdict rule and the module-edge placement rule each
  live in one place in the verify comparison now, consumed by verify's
  findings. Finding texts and exit verdicts are byte-identical.

### Changed

- A top-level claim that declares only `matches.modules` and matches
  nothing now warns (`unmatched claim:`) instead of erroring (red→green —
  see the `Breaking:` section); unit-backed claims keep the
  `missing component:` error.
- Claim-name uniqueness now addresses nested claims by their full dotted
  path, so a nested name colliding with a top-level name is refused.

### Added

- Agent relation-analysis playbook as its own skill on the binary's skill
  surface: `archspec skill analysis` emits the procedure byte-for-byte
  (`docs/archspec/skill-analysis.md`, embedded) — the dump commands
  (`archspec scan --output`, `archspec depgraph modules|submodules
  --parent|api-usage`, the file-level `inspect` map, the module-tier
  refusal path), five heuristics over the dumps — cycles beyond the
  `no_cycles` guard, fan-in/fan-out outliers as ratios over the graph
  mean, orphan modules, layering direction versus declared boundaries,
  god-crate candidates — and a recommendation-not-a-verdict output
  contract with dated-report naming and file:line citation rules. Every
  falsifiable statement in it cites its binary owner surface; verdicts
  stay in exit codes. `skill` gains name selection (`audit` is the
  default emit, byte-unchanged; an unknown name fails loud — no silent
  fallback); `skill install` and the printed default are untouched.
- The published tree ships `.github/FUNDING.yml` — GitHub's sponsorship
  configuration file — so the GitHub repository page can offer a sponsor
  link. Sponsorship metadata only; no binary surface reads it.

### Changed

- The relation-analysis playbook no longer ships as a payload markdown
  doc (`docs/archspec/analysis.md`, added earlier on this unreleased
  line): its content moved into the emitted analysis skill, so the
  payload shrinks per the fan-out law; the crate README, the agent
  guide, the docs index and the audit skill now point at
  `archspec skill analysis`. Documentation and procedure only — no
  verdict verb, no behavior change to the audit surface.

### Fixed

- `report` no longer exits non-zero while listing no findings (the silent-red
  class). The constraint-tier categories — `public api leak`, `unverifiable
  glob export`, `empty glob export`, `forbidden external crate`, `not external
  free`, `manifest integrity`, `feature boundary`, `forbidden submodule
  dependency` — were counted by the exit gate but skipped by the report
  renderers, so a run failing on one of them printed `no violations found` /
  `Result: clean` and `"findings": []` while still exiting 1. The exit gate
  and the report renderers now read one shared field list
  (`Diff::error_fields`): every category that can drive the non-zero exit is
  printed in text and markdown and serialized in the JSON `findings` array,
  counted in the `contracts` metrics bucket and the summary line. A guard
  test fails if a gated field ever lacks a render path again.
- The `feature_matrix` test regenerates `feature-matrix.{json,md}` under the
  target directory instead of rewriting them in the source tree, and asserts
  the committed copies equal the regenerated reports byte for byte, naming
  the regenerated files on drift. The freshness guarantee is not weakened: a
  committed report the current code disagrees with still fails the suite.
  The test now passes on a read-only checkout, which the containerized gate
  legs mount.
- The self-dogfood gate resolves the binary it just built under
  `$CARGO_TARGET_DIR/debug/archspec` when the caller presets a target dir
  externally (containerized legs run with one); the manifest-relative
  fallback is unchanged. The leg failed closed naming the stale path before
  this fix.

### Docs

- ADR-020 records the module-claim jurisdiction model for nested
  `[[module.submodules]]` claims: the most-specific claim carrying an
  `allowed` table owns dependency adjudication at any depth; a nested claim
  without one inherits the nearest ancestor's policy; ancestor↔descendant
  edges inside one claim subtree are composition, not adjudicated
  dependencies; ancestor prohibitions bind descendants cumulatively; claim
  names address rules at any depth. Design record only — no behavior ships
  with this entry; the submodule and target-addressing doc strings move with
  the implementation per the fan-out law.

### Added

- **`contract.expose` is schema and enforced — the expose chain law.**
  Specs that failed to parse on the reserved `contract.expose` key now parse
  and adjudicate at every depth. A declaring claim states the exact public
  surface of its territory: a crossing symbol it does not permit is a leak
  (`public api leak: <owner> exposes <export> (not exposed)`), attributed
  once to the deepest declaring claim, and an ancestor's declared set binds
  every descendant export — nesting may narrow the surface, never widen it,
  so splitting a module grants nothing. An entry that globs zero across the
  declaring claim's whole territory reports a `stale contract export`
  warning; an entry naming a declared stereotype and matching symbols
  adjudicates the stereotype arm and warns `ambiguous expose entry`; the
  shipped `dead reference` classification now covers expose entries. The
  `vacuous constraint` category gains the named-subject carrier
  `[contract.expose of <claim>]` for symbol-arm vacuity on drivers without
  module-tier public-API facts.
- **`archspec diagram --module <claim>`** renders one claim's module
  subgraph with public-API marks read through the same adjudication
  `verify` runs; an unknown claim exits 64 naming it. **`--modules-dir <dir>`**
  writes one file per claim (injective, traversal-proof names; a failed
  run leaves previous-or-complete bytes only), and `--check` flags stale
  files and orphans.
- **Every non-zero `archspec verify` exit ends stderr with the footer
  line `note: archspec <version>`** (issue #3 provenance answer). This
  is the committed-artifact class for consumers that parse empty stderr on
  red runs: red-run stderr is no longer empty. Stdout and every committed
  report byte stay version-free — report artifacts stay byte-identical
  across tool upgrades; the go arm of the `public_api_allowlist` vacuity
  sentence moves to the language-neutral wording, so committed artifacts
  carrying the old rust-vocabulary sentence need regeneration. A spec
  without expose declarations verifies byte-identically — the law is
  verdict-neutral where nothing declares.

### Changed

- **Breaking: the exit surface now names the failure class.** A usage
  mistake — no command, an unknown command, an unknown flag, or a `help`
  typo — exits 64; a verb this tool does not declare is an unknown
  command and exits 64; a missing `architecture.spec.toml` where a verb
  needs one (`verify`, `diagram`, `report`) exits 70, the
  operational-precondition class — never a verdict code. Verdict codes
  are unchanged: a rule violation stays exit 1, a clean run stays exit 0.
  Error messages are byte-identical on every changed path; only the
  codes moved. The `help diagnostics` legend states the full contract.
  Released binaries at 0.5.x and older predate this taxonomy: they age
  out through this deprecation note and carry no compatibility shim —
  the 0.6.0 binary answers with the new codes, and nothing translates
  them back.
- `Cargo.toml` declares the repo-wide lint policy (`[lints.clippy] all =
  "deny"`, `[lints.rust] unused_imports = "deny"`); clippy findings are now
  denied by the manifest, so a plain `cargo clippy` run matches the gate.

## [0.5.3] - 2026-10-05

### Added

- Misuse deflection on the vacuous `public_api_allowlist` line: an `allowed`
  entry without `::` that case-sensitively equals the last segment of a
  module path carrying public types, or names a declared module addressing
  such a path, now welds one prescriptive instruction onto the vacuity —
  `; allowed entries are module paths — did you mean "Sample::Contracts"?`
  — naming up to three sorted candidates, the rest counted as
  `...and N more module path(s)`; without an exact structural candidate the
  sentence stays byte-identical, and rust trees (whose driver emits no
  module-tier public-type facts) keep their sentence byte-unchanged. The
  agent guide gains a `Reporting a gap` section: probe the release binary
  you cite, quote `archspec --version`, the command and the output
  verbatim, check the known-boundaries table for the honest silences, and
  the which-command-proves-which-claim table (`scan`, `verify --strict`,
  `inspect tree`, `capability matrix`, each run green in the suite).
- GitHub issue #2 regression coverage (semicolon-bodied record
  declarations): the investigation against the released 0.5.2 binary showed
  the reported gap does NOT reproduce — positional `record X(...);`
  declarations already emit `module_public_types` facts and `verify --strict`
  reports them as leaks (`module exposes Type (not allowlisted)`, exit 1) —
  and the requested regression is welded so it can never regress: scan facts
  for every record body shape (positional with and without the trailing
  semicolon, braced, `record struct`, `record class`, partial across both
  shapes, generic, `where`-claused, attributed, nested), the allowlist leak
  line for the semicolon record, and the corporate corpus pin now carrying a
  brace-bodied record (allowlisted, silent) beside a semicolon-bodied one
  (leaked).
- The honest grammar boundary that investigation found: `;`-bodied
  `class X;`/`class X(int A);`/`struct X(int A);`/`interface I;`/`enum E;`
  spellings parse to ERROR fragments in the pinned grammar (no declaration
  node exists) and emit no facts — welded as a bounded-gap test with its
  rationale, extending the bodiless-class gap to the full non-record family
  (ADR-019, the scan output contract and the spec wording updated to match).

### Changed

- the crate now builds under Rust edition 2024; one extern declaration moved
  into an `unsafe extern` block, as that edition requires.
- the macOS arm64 release build moved from the `macos-14` runner label to
  `macos-15` — the macOS 14 images retire on 2026-11-02 (brownouts starting
  2026-10-05); the x86_64 macOS job stays on `macos-15-intel`, unchanged.
- the release workflow pins the Rust toolchain to `1.99.0` instead of the
  rolling `stable` channel, so release builds use the same toolchain as the
  repository's pinned builds.

## [0.5.2] - 2026-09-29

### Changed

- the c# public-api suite's unit-root comment now reads through the root
  identity ladder (csproj RootNamespace/AssemblyName, common namespace prefix
  truncated to the unit-name segment count, full unit name) instead of the
  retired first-two-segments era - comment wording only, no behavior change
- the roles matrix now derives its C# conflicting-fold honesty at realistic
  naming: the `csharp-conflicting-fold` fixture materializes as a corporate
  three-segment pair (`Acorp.Platform.Api` wiring registrations,
  `Bcorp.Platform.Api` a using-only root), the corporate roles equality pins
  state the full `Acorp::Platform::Api` / `Bcorp::Platform::Api` keys, the
  row's modules-graph cell gained a weld that fails if the recorded render
  loses its stated fold note, and a modules-graph scenario added at the same
  corporate naming pins the folded node, the untouched edge and the note
  bytes — the honest `none` cell and its stated drop cannot silently re-pin
  onto two-segment toys
- the roles matrix now carries its C# evidence at realistic naming: three
  corporate 3-segment fixtures — the DI-extension tree (registrations in an
  Extensions file), the sibling-claim tree (composition and facade units
  side by side) and the conflicting-fold pair (two unit names folding to one
  node with different roles, the stated-note cell) — joined the computed
  views × markers matrix, every cell re-pinned from recorded output, and
  equality pins on the corporate roles keys make a silent shallowing back to
  two-segment toys impossible; the composition predicate's docs (ADR-017
  derivation table, scan acceptance row 60) now state it as implemented —
  entrypoint gate AND registration-family calls anywhere in the unit
- view markers resolve the way the model states them: `inspect tree` and
  `scanner` unit headers — in both output formats — find their role through
  the `::`↔`.` identity the diagram already used, so a corporate dotted unit
  whose roles entry is spelled `ACorp::Api` is no longer the one declaration
  left unmarked; and the depgraph fold no longer silently drops what it
  cannot place: a fold that silences a node because different roles landed
  there appends one conditional note line stating how many role facts it
  could not place (views whose fold places every claim keep their exact
  bytes — rust and go goldens unchanged)
- C# facade evidence widened to the shapes real re-exports take: using-only
  root files (plain or `global` usings, sentinel or under the root
  namespace) and `[assembly: TypeForwardedTo]` forwarding files — the
  attribute is evidence on its own; wrapper-type roots stay unmarked BY
  DESIGN and an unwired entrypoint root keeps the sanctioned conduit reading
  (ADR-017 laundering semantics, pinned)
- C# composition reads .NET's own vocabulary instead of a filename: the unit's
  entrypoint gate — a Program/Startup file, a host `Sdk` (`.Web`/`.Worker`) or
  `OutputType=Exe` — AND a registration-family call anywhere in its production
  files, the family widened with `AddDbContext` and `AddMemoryCache`; a gate
  without wiring or wiring without a gate states nothing (ADR-017)
- C# role claims now merge per unit: composition beats facade only within one
  unit's own root, the same role claimed by several units states once, and a
  root key two units claim with different roles states nothing (the ambiguity
  guard) — a sibling unit's composition claim no longer swallows or silences
  another unit's role
- the C# root module is now derived through an identity ladder — the csproj
  `RootNamespace`, else `AssemblyName`, else the common prefix of the unit's
  declared namespaces truncated to the unit name's segment count, else the
  full unit name — replacing the first-two-segments rule that invented a
  phantom root on three-segment corporate trees (`Acme.Whatever.Service`
  attributed composition-root facts to `Acme::Whatever`, where no file
  lives); two-segment and smaller units resolve to byte-identical keys,
  ADR-017 carries the ladder, and the `scan_reality_*` guards pin it
- the shared scenario corpus gained `csharp_public_api_leak_pin`: a hand-built
  3-segment corporate tree (`Acmecorp.Inventory.*`) whose unallowlisted
  `Core.Internal` type leaks attributed to its module and whose scan is
  byte-deterministic — the public-API behavior is now pinned at realistic
  naming in every matrix run, not only on two-segment fixtures; the
  regenerated `feature-matrix.{json,md}` carry the row
- the manual now states the C# public-API facts: the constraint table and the
  glob re-export rule name both fact sources of `public_api_allowlist`
  (rust crate-root exports, C# module-attributed public types with no
  ancestor territory), `scan` output documents `module_public_types`, the
  `verify` finding vocabulary gained rows 96–99 and 100, and ADR-019 records
  the per-driver decision
- the unit runner pins its nextest config via --config-file (hook context
  runs from the repo root, where .config discovery finds nothing — without
  the pin the hooks ran with no per-test timeouts at all); the same pin
  landed across every migrated runner in the parent repo's lanes

### Added

- the capability matrix states a `public-api-surface` fact: rust enumerates
  it from crate-root exports, C# from module-attributed public types, and go
  emits none (its allowlist stays a fact-emptiness vacuity); a C# tree with
  no public-API facts now says `no public API facts to check` instead of the
  rust crate-root sentence
- `public_api_allowlist` now enforces C# trees: `allowed` patterns glob-match
  the modules carrying public-API facts (an entry claims no ancestor
  territory), every public type of an unlisted module is a `public api leak`
  naming module and type, and an `allowed` pattern addressing no fact-carrying
  module is stated as vacuous instead of passing silently; rust trees keep
  their crate-root semantics byte-unchanged
- the C# scanner records public-API surface facts: every explicitly `public`
  type (class, interface, struct, enum, record — partials deduped, nested
  types named through their parent) is attributed to its namespace-derived
  module in the model's `module_public_types`, production tier only; rust and
  go scans serialize no such key

## [0.5.1] - 2026-09-27

### Changed

- the self-dogfood spec now claims the depgraph engine module in its engines
  group, and declares three further constraint types on the crate — a
  `public_api_allowlist` over the library root, a `forbid_external_crates`
  rule keeping the grammar machinery (tree-sitter, its grammars and syn) out
  of the foundations layer, and a `manifest_integrity` publish requirement on
  top of the existing `no_cycles` checks; the spec comments now state the
  unit-tier truth about cross-crate imports and enumerate the constraint
  surface and the test-gating exemption the crate lives by.
- the diagnostics-help catalog constant now lives in a foundations-layer
  module (`src/archspec/helptext.rs`), so the engine-side consumers of the
  text — the report renderers' welds — reach it down-layer instead of
  importing the commands layer from `cfg(test)` code; the `help diagnostics`
  bytes are unchanged and the crate no longer leans on the test-gating
  exemption for any live import.
- a source guard (`tests/lib_imports_guard.rs`) now owns the file-level fact
  of which modules import the library crate — cross-crate imports carry no
  module endpoints in the model, so the boundary rules could never name them;
  the README self-dogfood section states the full group list, the four
  constraint types the crate declares on itself, and the test-gating
  exemption it deliberately does not lean on.
- a `cargo test`–reachable self-verify leg (`tests/spec_self_verify.rs`): the
  crate now checks itself against its own spec strictly from the test suite —
  zero violations, zero vacuous warnings, the checked-constraint count welded
  to the declarations in `architecture.spec.toml` — not only from the
  `run-unit-tests.sh` gate behind the git hook, and the leg writes no files.

## [0.5.0] - 2026-09-22

Version 0.5.0 — released from this entry. The era that landed under this one
version line, in one view; every previously written bullet survives in the
subsections below, unchanged.

- roles plan and contracts — role derivations, the capability role rows, the
  derived inert notes and verify consumption across languages with the
  report and inspect role surfaces; the composition laundering-hole hop
  exemption; the consolidated cross-language roles contracts (goldens, shared
  scenarios, acceptance rows) and the roles teaching sweep through the skill,
  model and rule docs (ADR-017).
- audit-defects — the view-layer fixes through the api-usage emptiness
  routing and the allowance-ownership decision and fix.
- relocation and roles-views surfaces — the module-relocation scan-surface
  pins and acceptance-row alignment to declaration-order resolution, the
  diagram scan-mode role marks, the depgraph folded-role marks with their
  fold-silence, the roles-in-views goldens settle and the cross-surface
  scenarios.
- cvd-precision lanes and CLI doc contracts — the verify-lane definitions
  (feature_boundary split, external_free tiers, roles matrix with the
  module-tier registry leg), the seed test-tier registry legs for rust and
  go, the api-usage state matrix and the c# submodules fold-shape statement,
  the refusal-family verbatim quotes with doc-quote guards, and the tool-wide
  CLI doc contracts (closed-pipe silence, the `--version` invocation row).
- the go-facade honest-absence decision — the ADR-017 amendment with its
  probe evidence, the decision-worded inert note and the no-facade guard
  extension over the probe-corpus trees.
- the stdout destination contract — the `wrote <path>` echo on every
  path-writing mode, the destination-not-written statement around a
  `--format` bypass, the machine-format purity of json and plantuml bytes and
  the `--output -` stdout convention.
- the views × markers roles contract — the computed matrix, the `help roles`
  topic and the skill pointer replacing the falsified per-view enumeration,
  with the api-usage self-stated role-less note welded to its matrix cell.
- the doc-surface naming sweep — the rust unit-naming sentence, the go
  naming-legality sentence and the report diagram-granularity line, each in
  one owner surface welded to the behavior that proves it; the workflow topic
  owning the audit recipe ordering; the go module-tier prose coherence sweep
  with its agreement guards.
- the machine-readable claim registry — every registered canonical welded to
  the surface that renders or emits it and to the dedicated guard pinning its
  bytes, with a universal fan-out scan failing any paraphrase of a registered
  claim across the rendered help surfaces and the payload docs, naming
  surface, line and owner.
- the blind-2 and blind-4 follow-ups — the payload leak purge, the friction
  prose fixes and the `--version` identity flag; the universal `wrote` echo
  with the committed-destination clobber on `--source scan` renders removed,
  the `--check` green ok-line across the five checking commands, the
  `--pretty` serialization switch, the `--source scan` diagram rule sentence,
  the csharp duality sentence and the depgraph and help one-liners,
  registered and welded with the wave-sweep planted-drift scenario.
- this batch, the blind-5 follow-up — the check error names the rendering it
  compared and hints that rendering's exact command form; the
  destination-canonical rule, the recipe's first-spec bootstrap, the earlier
  fold-sentence placement and the completed capability summary bring four
  true sentences to the surfaces an agent reads first; the roles matrix
  covers every inspect mode — scanner marks, bare inspect reads none — with
  the weld at seven fragments and a docs-tree pointer; the artefact dirt map
  is stated once at `help diagnostics`, welded to probe-shaped freshness
  scenarios; and the four new canonicals enter the registry with their
  guards, pins and planted-drift families so the fan-out guard owns the
  drift.

### Changed
- The claim registry now registers this plan and every wave-1..5 sentence it
  missed: twenty canonical claims (twelve before), each welded to the surface
  that renders it and the dedicated guard pinning its bytes (blind4-followup
  US 06). New entries: the `--check` green ok-line `ok: <path> up to date`,
  the exploratory-source rule — its sentence now lives in
  `archspec diagram --help`, the lane's only surface addition — the `--pretty`
  existence rows, the csharp dual-spelling sentence owned by `help languages`,
  the depgraph fold-pointer, test-gated-edge visibility and `mod` label
  statements, and the init/update summary pair; `wrote-echo` is superseded in
  place with its universal owner (machine destinations and explicit
  `--output` writes echo alike; no forked id). The fan-out guard scans the
  updated surfaces (the positional `help roles` paragraph, the COMMANDS
  summary split, `depgraph --help`), welds each new claim to rendered help and
  live command stdout, and a new wave-sweep scenario plants a stale echo of
  every registered family and must fail it loudly. Payload docs that still
  restated the pre-plan stdout contract — machine-format write silence, the
  silent `--output`, the green-check silence for the five checking commands —
  now echo the owner's canonical bytes.
- The crate version returns to the `0.5.0` unreleased tag. Two commits
  (`e2276bc9c`, `f5cd3a291`) let the pre-commit auto-bump move it to `0.5.1`
  and `0.5.2` because their diffs carried no explicit `Cargo.toml` version
  change; the tag carries no release and no behavior moved with this
  restoration — `Cargo.toml` and `Cargo.lock` are back on `0.5.0`, and the
  do-not-bump-until-the-release-commit invariant stands (blind-4 follow-up).
- `archspec depgraph --help` now states where unit ownership lives behind the
  folded node labels — the `root_module_declarations` map that `archspec scan`
  emits in its scan JSON — instead of growing a grouping flag (blind-4-followup
  US 05, D05/D08); the flag set and exit codes are unchanged and a new guard
  pins that a `--show-units`-style flag is rejected as unknown and cannot creep
  into the advertised surface.
- The nine one-liner facts the blind runs stumbled over now sit at exactly one
  binary owner each (blind-4-followup US 04, D06/D07): every render command's
  `--output` clause ships the literal example `--output -` beside its meaning
  (`-` is a flag value, never a path positional); `archspec diagram --help`
  shows the literal artefact form `archspec diagram --source scan
  docs/archspec/scan.json` and states that with `--source scan` the argument
  is a scan-JSON artefact file written by `archspec scan`, not a project
  directory; the `help roles` topic shows its views in runnable positional
  form (`archspec inspect tree`) and says `tree` and `modules` are positional
  subcommands, never flags; the role-less api-usage note keeps its canonical
  sentence byte-intact and gains the suffix `— explained in 'archspec help
  roles'`; the csharp section of `archspec help languages` now owns the C#
  dual-spelling sentence, moved verbatim out of the skill and spec docs, which
  keep a pointer instead of a restatement; `depgraph --help` states that
  depgraph does not read `archspec.toml [output]` — an artefact path comes
  only from an explicit `--output` — explains the `mod` label as a unit's
  root-module bucket (the namespace-less root where the composition/wiring
  code lives), and states in one sentence that test-gated/soft edges (test
  namespaces and their consumers) can appear in depgraph projections while
  being excluded from `depend_on` comparison; and the `--help` summary lines
  now split init (`scaffold config + starter spec (no real-tree capture)`)
  from update (`seed a spec by snapshotting the real tree`).
  `tests/blind4_oneliners.rs` welds each sentence to its owner surface and
  fails on any restatement in another rendered surface or payload doc; the
  dash clause pins (`tests/output_dash.rs`), the api-usage note-line stdout
  pins (`tests/depgraph.rs`, `tests/depgraph_reasons.rs`), the summary pins
  and the identity-duality weld (`tests/help.rs`) shifted byte-minimally in
  this change.
- Go module-tier prose now agrees with the model everywhere an agent reads
  it: the go section of `archspec help languages` owns the one sentence that
  the module tier is derived from the tree's own packages (a single
  `go.mod` tree whose packages reference each other carries it — `scan`
  emits those `module_edges` and `depgraph` renders them), and every other
  surface points at it or at `archspec capability matrix` instead of
  rephrasing it. `help constraints` dropped its stale "single-module go"
  example of a tier-less driver for that capability pointer; the
  `help diagnostics` emitted-by notes now point at their capability rows;
  and `depgraph --help` frames its refusal sentence as quoted error copy —
  the error emitted when the model has no module tier ("when the model has
  no module tier the command refuses with: …") — not a standing status of a
  healthy tree. The agents guide, spec, and the verify/update/report output
  docs dropped their stale claims that a flat go tree records no module
  tier, and every command-shaped capability mention now states the
  `archspec capability matrix` form. `tests/go_tier_agreement.rs` welds the
  owner sentence, the doc sweep, the live flat-tree scan edges, and the
  live tier-less refusal bytes to the registered claims `go-module-tier`
  and `depgraph-refusal` (fanout-output US 03).
- A machine-readable claim registry `claim-registry.toml` at the crate root
  now names the owner surface of every registered claim — the audit recipe
  of `help workflow`, the go module-tier derivation, the depgraph and inspect
  refusal bytes, the go facade inert note, the views × markers roles
  teaching, the rust and go naming sentences, the report diagram-granularity
  line and the stdout `wrote <path>` / `note: destination <path> not
  written` formats — with canonical bytes, signatures and the dedicated
  guard that pins them. `tests/claim_fanout.rs` welds every entry to what
  its owner surface renders or emits and to the guard's pinned constants,
  and scans every rendered help topic and command surface, the capability
  matrix, the skill output and every payload doc: an occurrence of a claim
  is legal only as a byte-identical echo of its canonical text or as a
  citation of the owner carrying none of the signature bytes, so a
  paraphrase anywhere fails naming surface, line, owner and the two allowed
  fixes. Planted-paraphrase, registry-drift and dead-family scenarios prove
  the guard fails loudly; five doc paraphrases of the go derivation rule
  became owner pointers (fanout-output US 07).
- The stdout destination contract became universal on the write axis and
  safe on the exploratory axis (blind4-followup US 01, D01/D02, D08). Every
  command mode that writes a body to a path — an `[output]` destination of
  any command and any format (the model JSON wrote silently under the wave-3
  machine split until now) or an explicit `--output` on any command
  (`depgraph --output` was the silent mode the go run named) — echoes
  exactly one line `wrote <path>` on stdout, spelled as configured or as
  given; the echo rides after the bytes land, so a failed write echoes
  nothing and keeps today's error and exit code. Machine bodies stay pure:
  no status text ever inside a written body, and a machine body on stdout
  stays byte-clean. The exploratory render path lost a destructive trap —
  `diagram --source scan <artefact>` with no `--output` used to write the
  configured destination, clobbering a committed spec-mode diagram (one
  confirmed incident, reproduced live at plan time); it now renders to
  stdout and never touches the destination, a human body stating the skip
  with the existing `note: destination <path> not written` line — the rule
  `report` already follows on its format axis, applied to the source axis.
  The old capability stays explicit: `--output <path>` writes wherever told.
  The canonical spec-mode destination contract is unchanged, exit codes are
  unchanged, no-destination runs are byte-identical (no echo), and the pins
  that exercised the formerly silent modes moved by exactly the echo line
  and nothing else.
- A green `--check` verdict speaks: a fresh check now prints exactly one line
  `ok: <path> up to date` on stdout and still exits 0, for every command with
  a destination (`scan`, `diagram`, `report`, `inspect`, `depgraph`) and for
  paths supplied by `--output` or by the `[output]` configuration alike
  (blind4-followup US 02, D02). Red paths are byte-unchanged: a stale,
  missing or destination-less check keeps its finding and non-zero exit and
  prints no ok-line, so a failing check never looks like a passing one. A
  fresh artefact of violating code gets both verdicts honestly apart — the
  ok-line states freshness on stdout while the exit code still states the
  violations. Silence no longer hides "checked and fresh"; the pins that
  asserted green-check silence moved by exactly the ok-line and nothing
  else.
- A stale `report --check` now names the rendering it compared (blind5
  followup US 01, D01). The report family's stale finding carries a naming
  clause — `compared the text rendering` vs `compared the markdown
  rendering` — and the regenerate hint carries that rendering's exact
  command form: `regenerate with: archspec report --format markdown` when
  a markdown body was compared, `archspec report` when the canonical text
  rendering was, so following the hint verbatim regenerates the compared
  body byte-exactly instead of rewriting the artefact in another format —
  the trap two blind5 runs named, one of them "a CI footgun that punishes
  the honest reader of its error message". When the canonical rendering
  was compared the error also carries the advice clause `pass the same
  --format to --check to verify a --format-generated artefact`; on a
  format-named check the exact-form hint already is the advice and the
  clause is absent. The guidance sentence itself is stated at two welded
  surfaces: the `--check` line of `report --help` and the `[output]`
  section of `docs/archspec/config.md`. Naming applies to the report
  family only — scan, diagram, inspect and depgraph emit one rendering
  each and their stale bytes, hints included, did not move; exit codes and
  the `ok:` green family are unchanged.
- Three help-surface relocations and one summary correction, all wording
  where the reader already is (blind5-followup US 02). The destination-
  canonical behaviour — a configured destination serves the default text
  rendering and a format-named run writes stdout only — is now stated in
  the `report` help text next to the `report_destination` granularity line,
  where the destination flag itself is explained; the recipe's Supporting
  commands rows keep their bytes. `help workflow` gained one bootstrap
  sentence: steps 1–2 create no spec, and a first capture of an untracked
  repo seeds one with `archspec update` before step 3 — the `update` row
  keeps ownership of the reconciliation. `depgraph --help` now states the
  fold-pointer sentence before the `mod`-label paragraph, so a reader
  hitting the folded-label cloud learns the disambiguation route earlier;
  the sentence's bytes are unchanged. The COMMANDS capability row's
  summary now names both accepted modes, `matrix | granular <language>
  <fact>` — it listed only `matrix` while its usage line offered both;
  bare `capability` still prints usage on stderr with exit 1, as the
  discovery run observed. The three sentences are registered canonical
  bytes awaiting their US 05 registry entries.
- The release workflow builds the aarch64 musl binary again. On the arm64
  runner the gnu step passed while the musl step died at cargo exit 101:
  the tree-sitter grammar build scripts drive cc-rs, cc-rs compares full
  target and host triples, and a native arm64 host building
  `aarch64-unknown-linux-musl` is not the host triple — so cc-rs demanded
  `aarch64-linux-musl-gcc`, a compiler no Ubuntu noble package ships. The
  job now installs `musl-tools` and points cc-rs at `musl-gcc` through
  `CC_aarch64_unknown_linux_musl`, mirroring the compiler the x86_64 job's
  musl build auto-discovers; the linker stays rustc's default, which links
  the musl target's self-contained crt objects and libc.a.

### Added
- Licensing and credits — the tool is licensed under the Apache License 2.0:
  the payload root carries the license text (`LICENSE`) and a generated
  third-party credits listing (`CREDITS.md`, regenerated by
  `scripts/gen-credits.sh` and freshness-gated by `run-unit-tests.sh`
  alongside the artefact checks); the crate metadata and the Arch package
  metadata both state the `Apache-2.0` SPDX identifier; the `licensing`
  payload guard pins the license file against the canonical text, the
  metadata field against the SPDX identifier, and the credits listing
  against the lockfile-resolved dependency closure — and fails on any
  remnant of the earlier permissive-only license declaration.
- Machine JSON learned to be readable (blind4-followup US 03, D03).
  `archspec scan --pretty` serializes the model indented — stdout, `-` and
  `--output` destinations alike, the `wrote <path>` echo unchanged — a
  different layout of one model: the pretty body decodes to facts exactly
  equal to the default line and stays byte-stable across runs. `archspec
  report --pretty` is accepted on every format and changes no byte, because
  report json renders indented already and other formats have no json
  layout, so one flag set scripts across commands. The refused surfaces
  stay refused: `--pretty` on `diagram`, `verify` and friends keeps failing
  as an unknown flag. No default byte of any command changed — the flag
  only adds a serialization switch, and the help surfaces of `scan` and
  `report` teach it.
- Three facts that cost the blind runs a probe cycle each are now stated in
  the surface an agent reads before hand-writing a spec or trusting a
  report: the rust section of `archspec help languages` states that a crate
  with `src/lib.rs` and `src/main.rs` and no explicit `[[bin]]` or `[lib]`
  section yields units `<pkg>` (kind=crate) and `<pkg>-bin` (kind=bin) and
  that `matches` must name them exactly (the rust run hand-wrote wrong unit
  names because scan's `-bin` suffix was discoverable nowhere); the go
  section states the naming legality decided from code (D10) — `matches.units`
  targets are full import paths (a short package name verifies as
  `unexpected component` / `missing component`), while module `name` values
  and `allowed.depend_on` references may be short, resolving to declared
  module names (the go run kept 38-char paths out of superstition); and the
  report help surface states that the diagram embedded in a markdown report
  is file-granular and the module-granular graph is
  `archspec depgraph modules` (the go run delivered a ~120-node hairball
  before finding `depgraph modules`). Each sentence has exactly one owner
  surface: the guards run the binary, weld the rust sentence to a real
  scan/verify of a lib+bin tree and the go sentence to real verify runs on a
  go fixture, and fail on an unregistered echo in any other rendered help
  surface or payload doc (`tests/doc_surface_nits.rs`; fanout-output US 08).
- `depgraph api-usage` now states its designed silence: every human-form
  body (table or emptiness statement) trails with the one line
  `note: this view shows no roles by decision (a role is not a usage fact)`
  — unconditional, table and statement bytes untouched, machine forms
  never carry it. The blind runs read the role-less output as possible
  breakage; the note removes the guesswork, and the roles matrix welds the
  line's meaning to its `none by decision` cell: the derivation panics
  naming registry and emitting module when either surface changes without
  the other, and the `help roles` topic quotes the note verbatim
  (roles-contract US 06; `tests/roles_matrix.rs`).
- The roles-in-views contract now lives in one computed place: the
  views × markers matrix (`tests/shared/roles_matrix.rs`) runs every view
  (`inspect tree`, `depgraph modules`, `depgraph api-usage`, `diagram`,
  `diagram --source scan`) over four fixtures (rust probe, rust lib+bin,
  c# probe, go probe) with the built binary and derives each cell from the
  recorded output — a role marker present is `marks`, a registry-decided
  silence is `none by decision`, otherwise `none`; the pinned table is the
  reviewed derivation, re-pinned whenever a run shifts an observation.
  `archspec help roles` (eighth topic, wired into the index, the
  unknown-topic error and the tool-help footer) states the contract, its
  per-view lines byte-welded to the matrix registry and the two decision
  reasons quoted verbatim (a role is not a usage fact; declared components
  are not model paths). Scenario guards: `tests/roles_matrix.rs`
  (roles-contract US 01).
- `archspec --version` (and the `version` subcommand spelling) prints
  `archspec <crate version>` and exits 0 — the binary can now answer "am I
  holding a stale build?", the muscle-memory first command of the second
  blind run, which hit the unknown-command refusal and left the question
  unanswerable. The dispatch early return names exactly the two spellings:
  the unknown-command refusal path is untouched and stays guarded, pinning
  a sibling unimplemented flag (`--verbose`) as still refused.
  Discoverability is wired through the top-level help footer, the skill
  workflow table's "Which binary am I holding" row, and the doctor docs
  (sanity = `doctor`, identity = `--version`) (blind2 friction #1;
  `tests/help.rs`).
- `archspec skill` — the agent-facing audit skill ships in-binary as a
  deliverable: `archspec skill` prints it, `archspec skill install [path]`
  writes it to `<path>/.agent/skills/archspec.md` (default: current directory),
  creating parents; an identical file is never rewritten (idempotent), a
  locally modified file is kept and requires `--force` to replace. The content
  is the payload document `docs/archspec/skill.md`, embedded at compile time
  and byte-pinned against emission (drift guard): front matter for
  skill-loading harnesses, the `--backend syntax` extraction rule, the tiers
  of trust, the four finding classes (named crossings, absence-of-facts,
  type-position boundaries, spec-vs-model gaps), the report-format pointer,
  and the honest blind spots. Discovery is automatic — the top-level command
  list names the command, `skill --help` prints its contract, and the workflow
  manual topic points at `archspec skill install` outside the numbered recipe
  sequence. Motivation is the measured discoverability gap of unguided agent
  runs (workplan archspec_agent_skill; ADR-015).
- `capability` reports an extraction back-end matrix: after the fact rows and
  rule rows, `capability matrix` prints a dedicated section with one
  `backend <language> <backend> <fact> <emission>` row per language, `--backend`
  value and back-end-dependent fact (`module-tier`, `symbols`) — rust stated
  as `same-as-classic`, csharp symbol emission stated for `syntax` only, the
  go module tier stated go.work-only under `classic` and as a full module tier
  under `syntax`; the existing fact rows keep their values verbatim. The
  feature matrix gains the `scan.backend_syntax` feature id.
- Depgraph wording and the module-tier refusal are back-end-qualified: the
  api-usage empty reason for csharp under `syntax` states that the selected
  back-end emitted no symbol facts for the tree (the classic driver sentence
  stays byte-frozen for `classic`), and a tier-less model refusing under
  `syntax` names the syntax tier rule (package references) instead of the
  classic-only go.work remedy — in `depgraph` and in the `inspect tree` refusal.
  Trees whose syntax model carries the module tier render instead of refusing.
  `capability` acceptance rows 3–7, `depgraph` rows 29–31, `inspect` row 31.
- `depgraph api-usage` symbol rendering is pinned per back-end: on trees whose
  `syntax` model carries module-edge symbols (c# type-targeted usings and type
  positions, go.work exported selectors), the `syntax` run prints those symbols
  grouped per (target, using) top-level module in the rust shape — backticked,
  comma-separated, sorted — and equal to the grouping of the same-value `scan`
  model, while the `classic` run of the same tree stays byte-identical to the
  pre-feature rendering and rust symbol rows are byte-identical across no flag,
  `classic` and `syntax`. `depgraph` acceptance rows 27–28.
- Inspect model view agreement with scan pinned per back-end: `inspect tree`
  and `inspect scanner` with a selected `--backend` value draw exactly the
  module boundaries the same-value `scan` model records — no boundary from the
  other value's model, none dropped, none invented — proven per value on
  csharp trees whose back-ends differ (type-targeted using plus type
  position), on single-module and `go.work` go trees, while the file-level
  map stays byte-identical across the values. `inspect` acceptance rows
  28–30.
- Go `syntax` back-end extends go.work edges with selector symbols and adds
  within-member package edges: cross-member module edges keep their classic
  `unit` and import-path endpoints and additionally carry the exported
  selector symbols of row 57's casing and merge rules, within-member package
  imports gain module edges addressed exactly like the single-module path
  (`unit` = the member module), and the unit tier, units, external tier and
  member grouping stay back-end-independent — `classic` keeps the frozen
  workspace shape. Canonical `scan` acceptance row 58.
- Go `syntax` back-end records exported selector symbols on package edges:
  the selector identifier following a package qualifier (import alias or the
  package's own name) joins the symbols of the edge that import created when
  its first character is uppercase — casing is the visibility rule — merged
  deduplicated and sorted across files while the edge itself stays one;
  unexported selectors, method calls on local values, chained-call tails,
  struct-literal field keys, qualified type references and dot-imported bare
  calls contribute no symbols (blank and dot imports keep their edges
  symbolless), and stdlib/external selectors add nothing module-side.
  Canonical `scan` acceptance row 57.
- Go `syntax` back-end emits module edges between packages of one module: an
  import of a sibling package records `from`/`to` as the packages' import
  paths with `/` -> `::` (the go.work-edge vocabulary), `unit` = the importing
  module and empty symbols (selectors land with the next Go scenario);
  `classic` keeps an empty module tier on such trees and go.work trees keep
  their classic workspace output under both values (interim rule). Canonical
  `scan` acceptance row 56.
- C# `syntax` back-end attributes compilation-unit `using` directives of
  multi-namespace files (placed above the first block namespace) to EVERY
  namespace the file declares instead of dropping them the way the frozen
  classic tokenizer does — the drop is demonstrated on the same fixture by a
  `classic` run. Canonical `scan` acceptance row 55.
- C# `syntax` back-end injected-field edges proven from the field TYPE
  POSITION: a field (or constructor parameter) of a declared-namespace
  interface type records the owner-addressed edge with the interface as the
  only symbol, once regardless of how many calls traverse the field;
  member-access chains rooted at the field name are anchored noise and
  contribute nothing — no identifier-link machinery was needed. Canonical
  `scan` acceptance row 54.
- C# `syntax` back-end emits module edges from type positions (base list,
  `new`, cast, attribute, parameter/return, generic arguments, field types,
  anchored dotted chains) with the referenced type names as symbols; bare
  names resolve through the file's visibility anchored on declared types,
  while `classic` and the external tier stay byte-identical.
- C# external attribution proven back-end-independent: a tree whose
  cross-namespace usings target only NuGet and BCL namespaces records no
  module edges and byte-identical `module_external`/`external` facts under
  `classic` and `syntax`; alias right-hand targets attribute identically;
  `global::`-anchored targets never become module edges under either
  back-end and attribute under `syntax` only, where the frozen classic
  tokenizer reads no target at all. Canonical `scan` acceptance row 49.
- C# `syntax` back-end emits module edges from using directives addressed at
  the deepest declared namespace, with the remaining dotted segments as
  referenced type symbols.
- syntax-tree grammar layer compiled into the binary (tree-sitter; csharp and
  go grammars).
- `syntax` proven a fact-identical alias of `classic` for Rust: byte-equality
  tests across `scan`, `verify`, `update`, `report`, `depgraph modules` and
  `inspect tree` on a rich multi-crate workspace, plus equality of both flag
  values with the frozen classic golden model; acceptance rows registered per
  command (canonical `scan` output row 46).
- `--backend` value rejection pinned at the CLI level: on `scan`, `verify`,
  `update`, `report`, `inspect` and `depgraph` an unsupported value (or an
  empty one) exits 1 with `unsupported backend: <value> (supported: classic,
  syntax)` before any path or spec work — the back-end message wins over a
  path error — and `--backend` without its value fails in the parser with
  `flag --backend requires a value`; acceptance rows registered per command
  (canonical `scan` errors row 45).
- Commands accept `--backend classic|syntax` (default `classic`); syntax
  back-end lands per language in later scenarios. `scan`, `verify`, `update`,
  `report`, `inspect` and `depgraph` parse and validate the flag and thread
  the selected back-end to the extraction entry points; until a language
  gains its syntax scanner both values emit byte-identical output (for Rust
  they always will — `syntax` is a fact-identical alias of `classic`).
- Golden regression fixtures freezing the classic extraction path
  byte-for-byte: `scan`, `update`, `report --format json`, `depgraph modules`
  and `inspect tree` runs on per-language fixtures (Rust workspace, C#
  multi-project, Go single module, Go workspace) are compared against
  committed artefacts in `tests/goldens/`, recorded from the pre-`--backend`
  binary and re-recordable with `ARCHSPEC_GOLDEN=1 cargo test --test
  backend_freeze`. The goldens are the contract the planned syntax back-ends
  must not disturb on the classic path.
- Documentation: ADR 014 accepts syntax-tree extraction back-ends selected by
  a planned `--backend classic|syntax` flag, amending the ADR 012
  implementation-status line; the flag itself is not implemented yet.
- The scan model carries structural roles as a serialized fact: `roles`
  maps model paths to a closed two-role vocabulary (`facade`, `composition`).
  The rust driver derives both from its own facts — `facade` at the root
  module path of a unit whose crate root defines nothing but declarations and
  re-exports, `composition` at `<unit>::main` of a bin unit whose main root
  wires modules — and `verify`'s `facade dependency` rule now reads the map
  (rule name, findings and exit codes unchanged; rust findings are
  byte-identical to the pre-change record). A tree or module whose driver
  cannot derive a role carries no entry, and an empty map serializes to no
  key at all: absence states "no role", never "role denied". The serde-skipped
  in-house `facade_roots` field is retired with this commit; csharp and go
  state no roles yet and their models serialize unchanged.
- The go driver derives `composition` from its own source facts: a unit whose
  `package_clause` is `main` carries `composition` at its unit's import path
  in the roles map — a module-root main, a nested `cmd/` main, and the main
  packages of each member of a `go.work` tree alike; the packages the main
  binary links gain nothing beyond that, and a library-only tree states no
  roles at all. The go driver states no facade anywhere: the fact has no
  derivable go predicate, so the roles map's silence is the honest truth the
  capability row will carry — pinned by a guard test across every go shape
  (single-module main, cmd-rooted main, go.work workspace, library-only),
  never an oversight (workplan archspec_roles US 03).
- The csharp driver derives roles from its own facts: a root module whose only
  facts are `using`-facts — namespace-less or root-namespace files contributing
  usings while declaring no types — carries `facade`; an entrypoint project
  whose Program/Startup files carry DI-registration calls of the
  AddScoped/AddTransient/AddSingleton/AddHostedService families carries
  `composition` at that root module key, and a unit with registration facts
  never gains facade in the same scan (the two predicates address disjoint
  shapes of one key, and composition claims win when distinct units resolve to
  a shared root key). Registration calls are a fact source only — no new
  edges, no rule engagement yet; family matching reads code with comments and
  literals stripped, so a registration named in prose states nothing and
  framework `Add…` calls outside the family state no role. A live C# API
  wiring cross-layer services reads as a composition root; a csharp tree with
  neither shape carries no roles key (workplan archspec_roles, US 02).
- The capability table states the two role facts the derivations shipped:
  `role-facade` and `role-composition`, one row per language, written from
  live probes after the drivers landed — rust and csharp `granular` for both
  roles, go `granular` for `role-composition` and `not-emitted` for
  `role-facade` (the go driver derives no facade role, and the table says
  so). The `facade dependency` rule pairs onto `role-facade` (rule name,
  findings and exit codes unchanged); the `root-facade` fact retires with no
  alias row, so `capability granular <language> root-facade` fails closed as
  an unknown fact through the unchanged `<language> <fact> <granularity>`
  grammar. `verify`'s inert notes derive from the rows: a go run states one
  role-scoped line — `driver emits no role-facade fact` — a csharp run is
  silent because its facade derivation emits the role, and rust runs stay
  byte-identically silent; the repeated facade-inert note noise retires as
  a consequence, not a side quest (workplan archspec_roles, US 04).
- Payload document `docs/adr/018-identity-ownership-of-allowances.md`: the
  identity-ownership decision for allowances — when an edge reaches a boundary
  graph under a namespace/module identity, its allow/deny resolves against the
  spec module claiming that identity's source unit, and a module-tier
  attribution never re-adjudicates a dependency the unit-tier attribution
  already allows (audit defect D5).
- `diagram --source scan` names its role carriers: a node whose identity
  equals a key of the model's `roles` map (modulo the `.`/`::` separator
  equivalence the depgraph views already use) renders the role marker roles
  US 06 established — the ` [facade]` / ` [composition]` label suffix in
  mermaid, the `<<facade>>` / `<<composition>>` stereotype riding the bare
  declaration identifier in PlantUML (new additive `plantuml_component_marked`
  primitive in `render`). A pure lookup with no derived facts: nodes the map
  does not address, the whole external cluster, and a model with an empty
  `roles` map render byte-identical pre-plan bytes, and spec mode reads no
  roles map at all; a rust bin composition (`<unit>::main`) sits below this
  view's unit vocabulary and stays honestly unmarked — `inspect tree` marks
  it. Acceptance rows 28–30, `tests/diagram.rs`, `tests/mermaid_syntax.rs`
  (workplan archspec_roles_views, US 01).
- `depgraph modules` and `depgraph submodules` mark what their own
  projection can honestly name: every `roles` key is projected through the
  view's OWN node function (`top_node` / `deep_node` / `child_node`, go
  import-path spellings lifted to the model's `::` path grammar), and a
  rendered node carries the roles US 06 marker — the label suffix in
  mermaid, the stereotype on the bare declaration in PlantUML — only when
  the paths folded into it state exactly one distinct role. A conflicting
  fold stays silent (the fold is too coarse to state one role; no node
  ever grows two markers), a key that projects to an unrendered node
  contributes nothing (no node is invented to carry a role), and an empty
  `roles` map reproduces the pre-plan bytes: the marker data crosses to
  `render_mermaid` / `render_plantuml` on a default-empty `ModuleGraph`
  field, and the primitives already roles US 06 landed do the rest. The
  trivial fold marks at the granularity it renders (go main packages),
  unit-tier rust/c# keys fold onto a `root` node these views do not render
  and stay honestly unmarked (`inspect tree` marks them);
  `depgraph api-usage` keeps its three fixed columns untouched by roles —
  a role is not usage, and the roles answer lives in the graph and the
  report (rows 34–38). `depgraph-modules` goldens: go and go-workspace
  re-baseline with the composition marker as their only delta.
  `tests/depgraph.rs`, `src/archspec/depgraph.rs` unit tests (workplan
  archspec_roles_views, US 02).
- The roles-in-views contracts settle as bytes and cross-language pins:
  the `depgraph-modules` goldens are re-baselined through the documented
  `ARCHSPEC_GOLDEN=1` flow with zero delta on top of US 02's two marker
  lines (go and go-workspace carry one marked node each, rust and c# stay
  byte-identical — their unit-tier keys fold onto the unrendered `root`,
  the honest-absence arm), two consecutive regenerations are byte-stable,
  and the `scan`, `report`, `update` and `inspect-tree` goldens are proven
  byte-unchanged by a green compare-mode run of the whole freeze suite —
  verified, not assumed (this supersedes the roles US 07 sentence
  "depgraph prints no roles"). Two shared scenarios carry the bidirectional
  rule on every driver: `scan_mode_marks_role_carriers` (`diagram --source
  scan` marks exactly the nodes the roles map addresses under the `.`/`::`
  equivalence) and `modules_mark_folded_role_carriers` (`depgraph modules`
  marks exactly where the roles keys fold onto a rendered node with one
  honest role) — both written from the model, so a key below a view's
  vocabulary reads as documented absence, never a skip; the feature matrix
  grows the two scenario rows (additive only), passing on all three
  drivers. `tests/shared/scenarios/{diagram,depgraph}.rs` (workplan
  archspec_roles_views, US 03).
- The views × markers matrix now covers every inspect mode: `help roles`
  gains the `inspect scanner` row (marks — the same structural model as
  `inspect tree`, same role labels — blind5 probe) and the bare
  `inspect (file level)` row (none — roles attach to model nodes, not
  files), the matrix derives both rows' cells from recorded command
  output over all four fixtures, the `roles-in-views` weld grows from five
  fragments to seven so a planted drift — scanner flipped to none or bare
  inspect claiming marks — fails by naming row and surface, and
  `docs/archspec/README.md` gains one pointer line to the topic: a
  pointer, no cells restated (workplan archspec_blind5_followup, US 03).
- The artefact dirt map is stated as one owner sentence at
  `archspec help diagnostics` and welded to probe-shaped freshness
  scenarios: a comment-only edit leaves every check and verify green;
  structure-changing edits (files, modules, edges) dirty the `scan` model
  JSON, the `inspect` file map and the `depgraph` projection while the
  spec-mode `diagram`, `report` and `verify` stay green; a component-layer
  undeclared edge additionally dirties `report` and fails `verify` with
  the `disallowed cross-component dependency` finding while the spec-mode
  diagram still tracks only the spec — every scenario a fixture replay of
  the sources.md probe table, all four destinations wired plus a
  committed projection (workplan archspec_blind5_followup, US 04).
- The blind-5 canonicals are registered: the claim registry grows from
  twenty entries to twenty-four — `check-render-guidance` (the guidance
  sentence, its error advice clause and the naming pattern over the three
  report renderings, owner the report help `--check` line, welded to the
  config doc's `[output]` section and to live stale-check probes of all
  three renderings), `first-spec-bootstrap` (owner the `help workflow`
  intro), `destination-canonical` (owner the report help surface,
  registered beside — not inside — `destination-statement`, whose emitted
  note template it does not touch) and `artefact-dirt-map` (owner
  `help diagnostics`), each pointing at its dedicated guard with a guard
  pin, so a stale registry entry fails the weld against the rendered or
  emitted bytes (workplan archspec_blind5_followup, US 05, D06). The
  `fold-pointer` weld gains the position pin — the fold sentence ordered
  before the `mod`-label paragraph in both depgraph help surfaces — beside
  its byte pin; the `roles-in-views` weld stands at seven fragments through
  `TOPIC_LINES`; the COMMANDS `capability` summary line gains an agreement
  leg against the bare-`capability` usage error, both naming the
  `matrix | granular <language> <fact>` grammar; and the wave-sweep
  planted-drift scenario grows one family per new claim — a planted stale
  echo of any registered family fails naming surface, line and owner, and
  the same scan returns green on the clean tree.

### Docs

- The three verify-lane language divergences are now stated as one contract
  with their why, beside the rows a verify user reads: the
  `feature_boundary` unknown-pattern split (rust fails the unknown gate
  pattern, csharp/go report one capability-reason vacuous constraint) is
  explained as tracking the `root-module-declarations` capability fact —
  inventing per-pattern errors on a driver that emits no declaration map
  would fabricate model content (acceptance prose under the
  `feature_boundary` table); the `external_free` engagement tiers (module
  subtree on rust/csharp, whole-unit on a tier-less go tree) are explained
  as monitoring the tier the driver serializes, and `spec.md`'s
  `external_free` row gained that cross-reference; and the roles rules get
  one engagement matrix per driver in `verify/output.md` — facade granular
  on rust/csharp and row-inert on go (the inert note quoted verbatim),
  composition granular everywhere — with values read from the capability
  rows and the terminal treatment of go's facade explicitly left to the
  go-driver workplan (workplan archspec_cvd_precision, US 01).
- The stated contracts gained their cross-driver pins: a new shared-scenario
  leg `external_free_engages_the_serialized_module_tier`, keyed on the
  `module-tier` capability fact, runs the purity guard's module-subtree
  contract (pure match passes non-vacuously, contamination names module and
  packages) on every driver the table marks granular — the leg the pin
  inventory named as missing; the feature_boundary split and the roles
  engagement matrix were already pinned ×3 by the capability-keyed registry
  pairs and are recorded, not re-keyed (workplan archspec_cvd_precision,
  US 02; `feature-matrix.{json,md}` regenerate with the leg).
- The seed's test-tier provenance is now pinned ×3 in the shared registry,
  not only per language: the csharp runner-package leg is joined by
  `seed_test_tier_follows_cfg_gating_not_module_name` (rust — the gated arm
  seeds no boundary while a module merely named `tests` seeds one, keyed on
  the `root-module-declarations` row the cfg mark rides) and
  `seed_omits_go_test_file_imports_and_marks_no_module` (go — a `*_test.go`
  import seeds nothing and marks no module, keyed inert on `test-tier` where
  the row reads `file tier only`); the `LogicalTree` harness gained one
  `test_gated` field so the two materializers write each language's own test
  shape, and every leg passed against unchanged behavior — the paragraph
  stated the truth, the pins were what was missing (workplan
  archspec_cvd_precision, US 05; `feature-matrix.{json,md}` regenerate with
  the legs).
- The two tool-wide CLI contracts that lived only in code and tests are now
  stated in `docs/archspec/help.md`: a closed pipe on any render command is
  silence, not panic — the next write dies quietly (SIGPIPE, shell status
  141) or the process exits 0 when the output had flushed first, one
  contract for the whole tool that `tests/cli_pipe.rs` pinned but no doc
  surface named (guarded verbatim in `tests/help.rs`) — and the invocation
  table gained the `--version` / `version` identity row plus the tool-help
  footer line the binary already prints, closing the last CLI-2 gap around
  the already-landed identity flag (workplan archspec_cvd_precision,
  US 06).
- The api-usage empty-vs-table decision is now an explicit **state matrix**
  in `commands/depgraph/acceptance.md`, mirroring `api_usage_empty_reason`'s
  arm order cell for cell (csharp tree fact, rust unit-tier placement,
  fold-placement, bare statement, plus the two rendering states, the
  never-bare invariant and a refusal cross-reference) with a named pin per
  state; the arms are stated as decision rules, not per-state guarantees —
  the reachability probe found the csharp arm firing on a symbol-bearing c#
  tree whose symbol-carrying edges land inside one rendered node (its
  absence wording misstates that tree: a reported defect, behavior frozen),
  and `output.md`'s routing sentence was aligned to arm order (its "any
  model" generalization and "structural, not language-matched" framing were
  wrong for the csharp-first arm). The c# `submodules` fold shape (row 20)
  now states the rendered node set the way rust row 4 does — children `A`,
  `B` plus the lone `mod` node the parent's own namespace folds into and the
  child edge, the fold cited as the projection choice shared across drivers
  — and row 9's c# cell gained that sentence verbatim; two new pins lock the
  shape bytes and the probe state verbatim (`tests/depgraph_reasons.rs`:
  `csharp_submodules_fold_prints_child_nodes_mod_and_edge_verbatim`,
  `csharp_arm_prints_the_tree_fact_sentence_even_when_edges_carry_symbols`
  — both green against the unchanged behavior; workplan
  archspec_cvd_precision, US 03).
- The module-tier refusal family is now quoted verbatim on both acceptance
  surfaces, wording untouched: `commands/inspect/acceptance.md` row 31 carries
  the exact bytes of `TREE_MODULE_TIER_REQUIREMENT` (the paraphrase replaced,
  the `inspect scanner` remedy tail included, exit and stream stated),
  matching the pin `tests/inspect_structural.rs` already held, and both
  acceptance files gained one prose line under their refusal rows stating the
  family relationship — one clause shared ("needs the module tier…derived
  from package references…records none of"), the only drift the "the tree's"
  addressing in the inspect sentence (same fact, recorded as quote-lock drift,
  no stderr change), each view keeping its own remedy and its own decision
  (`worklog/done/workplan_depgraph_message_real_remedies.md`). Guards follow
  the help doc-quote precedent: `tests/help.rs::
  inspect_docs_quote_tree_module_tier_sentence_verbatim` locks the inspect
  sentence into its acceptance and errors docs,
  `depgraph_help_and_docs_quote_module_tier_sentence` gained
  `commands/depgraph/acceptance.md`, and `tests/inspect_structural.rs::
  inspect_scanner_renders_where_tree_refuses_on_tier_less_go` pins the row's
  second half — the scanner view renders (exit 0) the very tree the tree view
  refuses (exit 1, stderr verbatim), so the refusal is a model-content verdict
  (workplan archspec_cvd_precision, US 04).
- The two graph views now teach their role surfaces (roles-views US 04; the
  marker behavior itself landed with US 01/US 02 untouched).
  `commands/diagram/output.md` gained *Diagram structure* item 5 — scan mode
  marks a node iff the model's `roles` map addresses that node's identity under
  the `.`/`::` separator equivalence, the marker bytes per format, the honest
  sentence that a role keyed below the unit tier (a rust bin's `<unit>::main`)
  marks nothing in either diagram mode while `inspect tree` marks that entry one
  tier down, and the note that the PlantUML stereotype rides the declaration
  while edge lines stay raw. `commands/depgraph/output.md` gained the *Role
  markers* paragraph — each rendered node carries at most one marker, the roles
  keys projected through the view's own fold decide it, two different roles
  folding onto one node stay silent, a key projecting to an unrendered node
  invents nothing — plus the same rule as a bullet under *submodules* and one
  sentence under *api-usage* writing down that roles are absent there by
  decision, pointing at the modules graph, the report's Roles section and the
  scan map. The skill's "Which nodes are special" row now names every surface a
  role shows up on (scan `roles` map, report Roles section, `inspect tree`
  labels, `diagram --source scan`, `depgraph modules` fold marks) and teaches
  both silences — spec-mode diagram and api-usage — as decisions rather than
  gaps; the two command READMEs cross-reference the new paragraphs, and the
  acceptance tables close their numbering with a per-row pin list for the
  roles-views rows (no number collides across the plans that appended beside
  this one). Guards follow the help doc-quote precedent:
  `tests/help.rs::diagram_docs_teach_scan_mode_role_marks_verbatim` and
  `tests/help.rs::depgraph_docs_teach_fold_markers_and_the_api_usage_silence_verbatim`
  lock the legend sentences and assert the renderer emits exactly the marker
  bytes those legends quote, and
  `tests/help.rs::skill_row_names_role_surfaces_and_no_doc_reclaims_silence`
  locks the skill's teaching and sweeps the payload docs so no sentence claiming
  a graph view prints no roles survives (the roles-era sentence this plan's
  record supersedes) (workplan archspec_roles_views, US 04).


- Blind-run friction fixes (second blind run, roles era): the unit-vs-
  namespace identity duality is now stated plainly in skill.md and spec.md
  (every C# project appears twice — `.`-name build unit, `::`-name
  namespace identity; an `update`-seeded spec carrying both tiers is not
  duplication, ADR-018 assigns allowance ownership), pinned verbatim by a
  doc-guard test in both files. The skill workflow table's seed row names
  the target (`architecture.spec.toml` in the scanned directory) and the
  required review-and-trim step (the seed mirrors the graph, not the
  intent) — previously that hint lived only in the `update` stdout line;
  the `update` command docs already carried the full contract and are
  unchanged. The inspect docs teach label-grep over node-id grep in model
  modes (ids carry hash suffixes, markers and names live in the quoted
  label) with a concrete one-liner. The report docs explain the
  `edges (internal)` pair-union semantics — a single-crate tree
  legitimately prints 0 while the module graph is rich (display semantics
  unchanged; acceptance rows #22/#25 already pinned them).

### Removed
- Breaking: the classic byte-tokenizer extraction back-end. Tree-sitter
  syntax extraction is the sole path for every non-rust driver; the rust
  driver is untouched. There is no migration path — the flag, the
  classic/syntax split and the back-end-qualified wording existed only in
  unreleased payload and no release carried the surface being removed
  (ADR-016, supersedes ADR-014, amends ADR-015).
- Breaking: the `--backend` flag on every command, removed rather than
  deprecated: any use produces the standard `unknown flag: --backend`
  error, exit 1 (ADR-016).
- The back-end capability matrix from `capability`: the `backend ...`
  rows retire and the per-language fact rows state the values the sole
  path emits (ADR-016).

### Changed
- `archspec help workflow` is the sole owner of the audit recipe ordering
  (fan-out plan US 02, decision D09): the workflow topic's "Command families"
  header lost its arrow chain (`scan -> report -> diagram -> verify`) and now
  reads as a taxonomy pointing at the numbered recipe below, whose step bytes
  are unchanged. Every doc surface that had restated an ordering — the agents
  guide's four-row recipe table, its manual-list line, the help docs' topic
  rows — now cites the topic and names no sequence; the topic index dropped its
  comma-chain gloss the same way. The onboarding flows row runs verbatim:
  `archspec init` → `archspec update --force` → `archspec verify`, every step
  exit 0 (plain `update` refuses the spec `init` just created). Fan-out guards:
  `tests/help_audit_recipe.rs` pins the numbered steps canonical, sweeps every
  rendered surface and payload doc for recipe chains in either spelling, and
  executes the onboarding row in row order on every driver.
- `--output -` is stdout, for every command that accepts the flag (`scan`,
  `report`, `diagram`, `inspect`, `depgraph`): the body is printed and no file
  is created — the rust blind run's `inspect tree --output -` had instead left a
  36 KB file named `./-` in the project root (owner decision C, 2026-09-23).
  Under the dash stdout carries the body alone: not even US 04's destination
  statement, because an explicit dash is pipe intent. The configured destination
  keeps its bytes, the exit code stays 0, and the convention lands on the flag
  only — an `[output]` value of `-` is still a literal path, as today. Every
  surface documenting `--output` states it: each command's `--help`, its
  `archspec help <command>` page, and the flag rows of the command docs
  (`tests/output_dash.rs`, `docs/archspec/config.md`).
- Where a render command's body went is now stated on stdout, in one line — the
  `[output]` silence the blind runs read as breakage (`report | head` → empty
  stdout, exit 0) is gone (owner decisions A + B, 2026-09-23). A destination
  write of a human body (`text`, `markdown`, `mermaid`) echoes `wrote <path>`;
  a `--format` run that sends a human body to stdout while the configured
  destination stays untouched opens with `note: destination <path> not written`.
  Machine bodies (`json`, `plantuml`) stay byte-clean on every path — written to
  a destination they echo nothing, printed to stdout they carry no status line
  inside — and a run with no configured destination, an `--output` destination or
  `--check` keeps today's bytes byte-for-byte. Exit codes are untouched
  (`tests/config.rs`, `docs/archspec/config.md`).
- Refusal and diagnostic wordings that were qualified per back-end
- The skill's roles passage and the decision table's "Which nodes are
  special" row carry no per-view marking enumeration of their own: the
  falsified sentence ("a rust bin's `<unit>::main` entry marks in `inspect
  tree` alone" — while `depgraph modules` demonstrably marks it) is gone,
  replaced by one pointer sentence naming `archspec help roles` as the
  source, because the computed matrix outranks the prose (roles-contract
  US 01).
  collapse to single constants describing the sole path: the module-tier
  requirement sentence, the tree module-tier refusal and the
  no-symbol-facts empty reason (ADR-016).
- Extraction goldens re-baselined to sole-path bytes — checked to
  byte-match the previously recorded `--backend syntax` outputs on the
  same fixtures before the old goldens die: the go `scan` and `report`
  goldens record the syntax-tree model where classic emitted none, and
  `depgraph` and `inspect tree` on go trees render instead of refusing
  at a missing module tier (ADR-016).
- Test suites renamed out of the `backend_*` era — `backend_freeze`
  becomes `extraction_freeze` and the siblings likewise — and the
  cross-back-end parity suites retire with the second path they guarded
  (ADR-016).
- `report` `edges (internal)` now counts **distinct internal dependency pairs
  at unit tier** instead of summing both model tiers: the production unit→unit
  edges are set-unioned with module edges projected onto their endpoint units
  (self-pairs and test-gated pairs dropped), so a dependency stated in both
  tiers counts once. A single-module Go tree reporting 50 for its 25 real
  dependencies now reports 25; module-tier pairs with no unit-tier twin
  (cross-project C# usage reached without a direct reference) still count —
  exactly once; unit-internal module edges add nothing. `edges (external)` and
  every hard model fact are unchanged (`scan` output stays byte-identical).
- The `facade dependency` rule engages on the model's roles map instead of
  the rust unit set: every module path carrying the `facade` role is a
  publication root, whatever driver derived it. csharp umbrella roots now
  report what the scan already stated — a referencing project consuming the
  umbrella through the root namespace (`using App;`) is a facade dependency,
  while wiring through the product namespaces stays clean; go derives no
  facade role and gains no findings (the inert note is the capability row's).
  The main-root exemption is now the `composition` role, not the `main`
  name: rust behavior is unchanged for every tree whose root facts are real
  (scenario-98 family green, rust trees verify byte-identical), and a lib
  module merely named `main` importing through the root re-export is now
  reported like any other internal consumer.
- The laundering check consults composition ownership beside the fallback
  heuristic: a hop owned by a boundary carrying a `composition`-role path
  (the DI-wired csharp entrypoint, the go `package main`) is sanctioned
  wiring — never a ban endpoint, never conduit territory. `verify --strict`
  passes on DI and main-package bridges whose route rides the composition
  root's legal hop; the same edges without the role warn exactly as before,
  and fallback conduits keep warning. Exit codes and the diagnostics
  catalog are unchanged.
- The report and the model views surface the roles map as stated fact: the
  text and markdown reports gain a `Roles` section between metrics and the
  diff (`facades:` then `composition roots:` rows, paths sorted; `## Roles`
  table in markdown), the report JSON carries the model's `roles` map between
  `metrics` and `findings`, and `inspect tree|scanner` markers name role
  carriers on their declaration lines — label suffix `[facade]`/`[composition]`
  in Mermaid, stereotype `<<facade>>`/`<<composition>>` in PlantUML, with ids
  and edge lines untouched. Absence renders as absence: an empty roles map
  prints no section, serializes no key, marks no node — no `roles: none`
  theater anywhere (workplan archspec_roles, US 06).
- The role contracts now read the same across drivers: the canonical probe
  trees gained the shapes the derivations actually consume (c# `Program.cs`
  with DI-registration-family calls, go `package main` roots, workspace
  member included), the scan/report/inspect-tree goldens re-baselined over
  them with the delta limited to the roles key and marker lines (rust,
  update and depgraph goldens byte-unchanged), the cross-language scenario
  suites state the roles map, the honest go facade absence, the verbatim
  report restatement and the tree markers once and drive them per language,
  and the acceptance docs register the behaviors (scan 60–61, report 42,
  inspect 32) as the feature matrix regenerates (workplan archspec_roles,
  US 07).
- The role facts are now taught where agents meet them: the installable
  skill retires the "composition roots flag" blind spot for roles teaching
  (the scan JSON's `roles` map names each tree's facades and composition
  roots, `inspect tree` marks them by legend, DI-wiring edges are judged by
  stated role, and the honest go facade absence stays as the one real blind
  spot), the model-shape and rule docs state the roles map at every consumer
  surface (scan output contract, spec facade and laundering rules, agent
  guide scan captures), the capability failure contract sheds its stale
  conditional example (the go module-tier row is granular since the syntax
  path became the sole path), and ADR-017 records the closed vocabulary
  {facade, composition}, the serialized roles map replacing the
  serde-skipped field, the per-driver honest-absent derivations, and the
  hop-level composition exemption of the laundering check
  (workplan archspec_roles, US 08).
- The go facade inert note reads as a recorded decision, not a gap
  (workplan archspec_go_facade, US 02b; branch (b) of the go-facade plan —
  the US 01 probe verdict "not derivable"). ADR-017 gains an amendment
  section with the evidence the bare assertion lacked: the three investigated
  patterns (alias umbrella, public-package marker, root delegation), the
  byte-identical-decoy probes proving alias declarations sit outside the go
  fact surface, the measured 14-of-14 public-package false-positive sweep, the
  falsifier conditions, and the findings table restated payload-neutral with
  the probe record cited by worklog path; the go table cell and Status line
  point at the amendment. The note sentence is reworded once, generically, at
  the constructor — `note: facade dependency rule inert for go: no derivable
  role-facade idiom — alias umbrella, public-package and root delegation forms
  investigated, recorded in ADR-017` — sound because role facts are the only
  note-bearing rules and `role-facade` their sole `not-emitted` cell, so the
  sentence's only live output surface is this note. The row-driven note
  machinery, the capability table, all verdicts and exit codes are untouched:
  rust runs stay silent, a go run prints exactly one note with byte-comparable
  verdict and exit, and rust and csharp verify output is byte-identical. The
  shared-scenario needle
  (`facade_rule_note_announces_facade_tierless_driver`), the quoted sample
  line in the verify docs (output contract and the facade-rule table row) and
  the skill's teaching prose (roles paragraph, blind-spot entry) re-key to the
  new bytes in the same commit.
- The go facade honesty guard patrols the decided shapes now
  (workplan archspec_go_facade, US 03; branch (b), guard kept):
  `guard_go_scans_never_state_the_facade_role` keeps its four-tree absence
  loop and gains the four trees the US 01 probe planted — alias umbrella and
  its byte-identical wrapper decoy, delegating root and its defining decoy —
  so a future derivation cannot mark a facade through those shapes silently.
  The new `guard_go_facade_note_and_adr_amendment_agree` pins the decision
  surface: the single note a go run prints must cite ADR-017 and enumerate
  the investigated patterns, and every pattern label it enumerates must
  appear in the ADR's §Go facade investigation — wording drift on either
  side fails there, not in production. The guard's comment cites the
  amendment; no production code and no capability row is involved.

### Fixed
- `verify` no longer launders forbidden edges through composition roots
  (roles US 05b). Boundary-level bearer whitelisting — skipping an entire
  boundary because it owned some composition path — is deleted together with
  its stage-1/2/3 attribution ladder: a planted bin main plus a catch-all
  boundary claiming the bin's unit used to whitelist the whole boundary and
  hide the ban. The exemption is now hop-level, keyed on the roles map:
  edges whose source path carries the composition role are never attributed
  as laundering conduits, a misattributed sanctioned hop (a multi-segment
  unit root landing on the neighbour boundary that claims its first
  segment) is dropped from adjudication entirely, and the remaining check
  fires when the banned module's own direct exit hop is attributed only by
  fallback into an allowed target that reaches the ban. Banned pairs that
  reach the ban directly are still reported by the ordinary forbidden-edge
  check. Regression tests `verify_scenario_99` (planted bin main behind a
  catch-all composition boundary) and the multi-segment
  `verify_scenario_98j` fail on the pre-fix tool and pass on the fixed one;
  `verify_scenario_98f` gained its honest `use` edge, and `98h`/`98i`/`100`
  pin the sanctioned hop, the roleless-root report and the multi-root roles
  map.
- `inspect tree|scanner --format plantuml` honours the flag: the model
  views validated `--format` and then ignored it, always emitting a
  Mermaid model diagram. The requested format now reaches the model
  renderers, so both views render the PlantUML diagram of the same model
  content (`@startuml` header, one package per unit, quoted raw names on
  every node and edge) and Mermaid stays the default in every mode.
- Skill workflow commands re-keyed to the flagless surface: the embedded
  skill no longer instructs the removed `--backend syntax` flag and its
  back-end-era wording is rephrased to driver facts, so a skill-following
  agent never hits `unknown flag: --backend`.
- `update` under `--backend syntax` on a single-module Go tree seeds its
  packages instead of an empty workspace-shaped spec: the seed routing now
  keys on the model's member facts and package-level module facts rather than
  on the bare presence of a module tier, so a syntax scan (which projects
  package references onto the module tier without any go.work) yields one
  `[[module]]` per package — byte-equal to the classic seed of the same tree —
  and the `update` -> `report --backend syntax` round-trip exits 0 with zero
  findings (US 17 own-application audit, deviation D-1).
- `archspec help glob` describes subtree matching the way it actually works:
  a named module pattern like `app` selects its entire subtree without a
  trailing star (what the verifier's `module_path_matches` implements), and a
  trailing `*` is only the explicit prefix form. The same correction applies
  to `archspec spec` guidance, whose header now points at `archspec help glob`
  and `archspec help constraints` instead of a page that described a removed
  flag. `help constraints` gained a key-namespaces note — `depend_on` lists
  declared module names, while `from` and `forbid_external_crates` match
  model-path globs, case-sensitive. The `depgraph` and `update`
  supporting-command seeds now say their diagrams show top-level modules
  (unit-level diagrams belong to `depgraph --symbols` and `inspect flows`).
  The audit skill's workflow table gained a row pointing at
  `archspec help glob` before authoring any constraint.
- Docs and help no longer reference the removed `--backend` flag: command
  pages drop it from usage lines, tables and prose, the `capability`
  back-end matrix section is gone (the capability table reports the current
  granular facts directly), and acceptance rows that described
  classic-versus-syntax gated behaviour were rewritten to the single current
  extraction path or removed where the behaviour is already pinned elsewhere
  (the `update --force` rows superseded by Migrate 11-13).
- `depgraph modules` renders a graph where the top-level projection was a single
  folded node: when that projection collapses the whole module tier onto one
  label while the tier holds at least two distinct paths beneath it, the view now
  renders one level deeper — the paths below the fold become the nodes, labelled
  by their own final segment, and the edges reproject at that level. A
  single-`go.mod` tree whose packages address themselves below the module path
  therefore shows its **package graph** instead of `graph TD` with one node and no
  arrows, and a nested-parent tree of any driver shows its children (the picture
  `submodules --parent` already gave). The rule keys on the shape of the
  projection, not on a language, so trees whose projection has more than one node
  render byte-for-byte unchanged: the rust, csharp and go-workspace `depgraph
  modules` goldens are frozen evidence of that, and the single-module go golden
  moves from one node to its three-node package graph by exactly that axis.
- `inspect tree|scanner` no longer references mermaid nodes it never declared:
  every id an arrow uses now appears on a labelled declaration line above the
  arrows — `id["raw model name"]`, the same labelling `diagram --source scan`
  emits — unless a unit subgraph header or a soft-tier module line already
  declared it. A Go tree whose units address themselves with `/` while its
  module edges address the same packages with `::` used to draw arrows to
  FNV-suffixed ids declared nowhere, so mermaid rendered them detached,
  unlabelled and disconnected from their subgraphs; those edges now attach to
  labelled nodes while the graph's geometry — which pairs connect — is
  unchanged, and ids keep the FNV-suffixed sanitized form that keeps
  hyphen/dot variants collision-distinct. The go `inspect-tree` golden gains
  the declaration lines; rust, csharp and go-workspace renders are
  byte-frozen evidence that models carrying soft-tier declarations do not
  move (audit D3).
- `depgraph api-usage` never prints the bare "No internal API usage details
  found." on a model that carries symbols: grouping now reads the same
  trivial-fold granularity as the modules graph (D1's rule, one shared
  decision), so a single-`go.mod` tree's or a nested rust crate's once-folded
  edge symbols render as package- or child-module rows, and an empty grouping
  on a symbol-bearing model — every symbol-carrying edge landing on one
  rendered node, one level too deep for the fold — states the placement:
  `Module edges carry symbols, but every symbol-carrying edge stays inside
  one rendered module; the usage sits between modules below the granularity
  this view renders.` The bare sentence survives exactly where it is honest,
  for models whose module edges hold no symbol facts, in any language; the
  go hole that printed no reason at all is gone and the csharp and rust
  reason arms are unchanged (audit D4). On the single-module Go tree of the
  re-audit the model's 24 selector-symbol facts across 14 package-crossing
  edges printed a bare sentence before this fix; they now print their table.
- No internal product names remain in payload test comments: the Go update-seed
  fixture is now documented by its shape — a single-module tree with layered
  packages — not by an application name. No behaviour change.
- `verify`'s allowance lookup owns identity correctly (ADR-018): a dependency
  reaching the boundary graph both as a unit edge and under its namespace
  identity is no longer re-adjudicated against the allowance-less module
  boundary the namespace resolved to — the boundary claiming the edge's
  source unit grants or denies, so a unit-tier `depend_on` survives namespace
  attribution and the false `disallowed cross-component dependency` findings
  of a dual-identity C# spec (audit defect D5) are gone. Module-only specs
  adjudicate by boundary-name equality as before, a same-unit dependency
  stays module-attributed, and an owning boundary's default-deny still
  reports an undeclared target with its attributed text: the lookup is not
  an exemption.
- No internal product names remain in the payload: the C# fixtures and the
  prose around them are now documented as a generic layered C# service tree
  under a neutral `Shop.*` namespace family — mirroring the neutral example
  the scan-output doc already uses — with the Mermaid id expectations
  recomputed for the renamed fixture names. No behaviour change.
- Module relocation no longer rides only through `verify_constraints`
  plumbing: five inline fixtures pin the §Module relocation rows directly at
  the scan surface — the `#[path]` tree joining `soft_structure` and
  `module_edges` with children in the target's own directory, `cfg_attr`
  candidates resolving first-existing in declaration order with the
  conventional variants behind the attributed ones, and the declared-but-
  unresolved bare node (node-without-children in the JSON, no
  `unresolved_module_files` key, its `unresolved module file` warning
  spoken at `verify`). The serialized bytes are unchanged — the suite now
  states them outright (workplan archspec_rust_relocation, US 01).
- The §Module relocation acceptance rows now state the rule the resolution code
  implements rather than rustc's: candidates are tried in declaration order,
  the first existing one wins, the conventional variants sit behind the
  attributed ones, and cfg predicates are never evaluated — row #40's
  "when none exists", readable as "no predicate holds", is now the
  predicate-free "when no attributed candidate file exists", and the section
  preamble says outright that the scanner does not evaluate predicates.
  Row #39 was checked against the walk and the five US 01 pins line by line
  and stands verbatim. No behaviour change
  (workplan archspec_rust_relocation, US 02).

## [0.4.159] - 2026-09-21

### Fixed
- Publishing a tag that already has a release page edits that page instead of
  failing. The release job runs again whenever a tag is re-pointed or a run is
  retried, and there creating the page answers 422 ("Release.tag_name already
  exists"); because the asset upload follows it, a failed create left a release
  that no later run could repair. The step is now a script in this tree
  (`release-assets.sh`) so the same logic runs in CI and in local checks.

## [0.4.158] - 2026-09-21

### Fixed
- The release page now always carries the version's own notes. The release job
  read them from the tag through `git`, and on a shallow checkout a tag ref can
  resolve to the commit it peels to, where `git` reports that commit's message as
  the tag contents — so a release could publish whatever the last commit happened
  to say. The job reads the tag object's message through the API instead and falls
  back to a one-line title rather than to a commit message.

## [0.4.157] - 2026-09-21

### Fixed
- The previous entry described the aarch64 Arch package as carrying the same
  payload as the aarch64 tarball and deb. It does not: the tarball and deb are
  the glibc build (`aarch64-unknown-linux-gnu`), the Arch package is the static
  musl build (`aarch64-unknown-linux-musl`), which is exactly why the package
  may declare no dependencies. The entry now says which build each asset
  carries.

## [0.4.156] - 2026-09-21

### Added
- Release artifacts for every published version: Linux (tarball, deb, Arch
  package; x86_64 and aarch64), macOS (x86_64 and aarch64) and Windows
  (x86_64) binaries with checksums, attached to the release page by the
  repository's own release workflow.
- Arch Linux packages that install. `pacman -U` accepts them, `pacman -Qkk`
  verifies every installed file against the package manifest, and the x86_64
  asset is installed by a stock Arch container as part of the build. Both
  packages carry the static musl build rather than the glibc one, so their
  empty dependency list is true rather than optimistic.
- `package-arch.sh` builds a package from a finished binary,
  `check-arch-package.sh` reads a package back the way `pacman` does (archive
  members, `.PKGINFO` fields, a manifest that actually hashes the binary,
  payload architecture, staticness against the dependency list) and
  `install-check-arch-package.sh` proves the install in a stock Arch
  environment. Packaging is therefore inspectable without a release run.
- This changelog: every change to the published source now records an entry
  here, and release notes are generated from it plus the commit record.

### Changed
- Publication appends one release commit with aggregated notes to the
  repository history instead of rewriting it.
- Release packaging is produced by the scripts above rather than by a
  containerized `makepkg` run, so the built asset can be checked before it is
  uploaded instead of after.
- The published tags before this version (every one up to and including the
  release-pipeline experiments) produced no release page at all: they carried
  runner labels, container images, shell options and quoting fixes rather than
  binaries. Their entries are collapsed into this one, so the gap in version
  numbers is documented instead of hidden.

## [0.4.146]

### Added
- File-level import map for Go in `inspect`: package files become nodes and
  own-module imports become edges, so Go trees get the same discovery surface
  as rust and c# trees.
- `external_free` constraint: purity guards that report on presence instead
  of passing vacuously.
- Behavioral language-conformance matrix: each guard rule is exercised per
  language with clean counterparts and non-vacuity checks.
- Model-facing audit workflow in `help workflow`, including inspect-edge
  mining as the discovery step before writing rules.

### Changed
- Cross-unit `using` edges in the c# driver: reference-reachable namespace
  usings now produce module-tier edges instead of staying invisible.
- Production imports never target test-tier files in `inspect` edge maps, in
  any language; test files stay visible as nodes and import sources. The rule
  is pinned by a shared cross-language corpus scenario.
- Structural inspect modes render what the model carries; tree-mode refusal
  now names the missing module tier instead of the previous blanket message.

Earlier releases predate this changelog; their notes lived only in release
annotations.
