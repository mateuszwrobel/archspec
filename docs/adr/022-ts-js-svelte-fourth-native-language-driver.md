# ADR 022: TS/JS/Svelte becomes a fourth native language driver

## Status
Accepted (2026-10-15). Records the native `ts` driver (JS/TS/Svelte) and the
external grammar crates it adds, and settles the TS/JS deferral clause of
ADR-006 by revision. Implemented (2026-10-16): the driver ships — the `ts`
arm, its grammar crates, the `scope` field and the posture table landed
across phases A, B and C of the design record, and the rust posture row
flipped to `both` as the named change of ADR-023. This ADR is the design
record; the implementation changes closed its designed-not-shipped state.

## Context
ADR-006 deferred TS/JS ("dependency-cruiser already serves it; revisit
later"), and the design record kept the line "TS/JS: revisit after core
solid". The core is now solid: one canonical model (ADR-007), one adjudication
seam shared by `verify` and the diagram (ADR-021), a declarative capability
table, a recursive claim model (ADR-020) and a phase-2 contract model
(ADR-011 revised). Meanwhile a Svelte 5 single-page frontend used as an
architecture pilot shows the cost of the deferral: its imports are invisible
to archspec, its layering laws are held by a hand-written textual guard
scanner outside the model, and in that codebase most source files are
`.svelte` components — a driver that treats components as opaque would model
the application's majority as nothing. The design-doc revisit has arrived;
the question is only its shape.

## Decision
**archspec gains a fourth native `Language` arm `ts`, extracting structure
with tree-sitter grammars and adjudicating on the canonical model like every
other driver. No wrapping, no second model, no new binary.**

1. **Sources.** `.ts`, `.js`, `.tsx` and `.jsx` parse through the
   `tree-sitter-typescript` grammar family (its JavaScript sibling for plain
   JS); `.svelte` files are extracted through EVERY `<script>` block — plain
   `<script>` and `<script context="module">` alike, each parsed with the
   TypeScript grammar — and a component file is a first-class unit: its
   script-block imports are its imports. Markup carries no architectural
   fact, and the driver says so.
2. **New external dependency.** `tree-sitter-typescript` joins the grammar
   crates, pinned to its grammar ABI the way the existing C#/Go grammar crates
   are (ADR-014's pinning law). This is the ADR-triggering fact of the change.
3. **Detection.** A `package.json` marker plus the source extensions select
   the driver, the same marker-plus-extension shape that `Cargo.toml`,
   `*.csproj`/`*.sln` and `go.mod` use. A declared exclusion set
   (`node_modules`, `dist`, `target`, `.git`, vendor-style directories, plus
   the spec's `[project] excluded_sources` entries, each carrying a
   mandatory reason printed in every report) is removed BEFORE marker
   matching, so a dependency or build tree can never select a driver, and
   the precedence chain position is fixed and documented: rust > csharp >
   go, with `ts` as the fallback arm — the arm that selects TypeScript only
   when no rust, csharp, or go marker was found, so no shipped tree changes
   driver. The spec's declared language is the truth of a declared tree;
   scan detection must AGREE with it: any source file outside the declared
   language (after exclusions) fails the run loud, on `scan` and on
   `verify`, with the counts of both languages — the `verify`-side
   divergence guard flags the disagreement, it does not re-key extraction,
   which stays with the declared driver. `scan` reads the spec file at the
   scanned tree root only when the file is present — root-only discovery,
   never an upward walk; an invalid spec fails `scan` with the same
   invalid-spec message class and exit as `verify`, and the agreement check
   prints on the stderr/report surface only, leaving the scan model's artifact
   bytes unchanged — the graph is the artefact. The `verify`-side guard runs
   the same detection walk — marker collection after the exclusion sets —
   independently of the model. Every active `excluded_sources` entry prints as
   path + reason on every run of BOTH verbs, so an exclusion never shrinks the
   analyzed set in silence.
4. **Edges.** Import, export-from and dynamic-`import()` specifiers with a
   literal specifier, and asset imports (`.css` and friends — ordinary edges
   to the asset file, so runtime edges a textual guard used to count stay
   visible) are the edge sources. A dynamic `import()` with a non-literal
   specifier is a named dynamic-wildcard fact: report-only, never silently
   dropped, never red. A relative specifier resolves through extension
   inference (the four extensions) and index inference (`dir/x` also tries
   `dir/x/index.*`). A bare specifier is an **external** fact — npm packages
   live in the same external tier external crates do, and the shipped
   `forbid_external_crates` arm adjudicates them; resolution through linked
   npm-workspace packages is out of scope for the design and documented as
   such — no verdict is claimed for it. A specifier that resolves to nothing
   is a loud finding, never a silent drop; a specifier that resolves ONLY into
    excluded paths joins this class — an `unresolvable import` finding naming
    the exclusion, error class, exit 1 — an exclusion never turns a dropped
    edge green, and the dynamic-wildcard fact names non-literal specifiers
    only: it never carries an excluded-path resolution. Path aliases are not
    resolved: a
   bare-but-aliased specifier joins the unresolvable class honestly, and
   alias support is a named future change with its own resolution law.
5. **Exclusions.** `node_modules`, `dist` and `.svelte-kit` contribute
   neither files nor edges — build and dependency output is not structure.
6. **Facts.** The driver emits the model's module-tier facts at the file
   granularity its edges are born in — ts module identity is the
   repo-relative file path WITH its extension (so `a.ts` and `a.js` are
   distinct vertices) and the unit root is the package root — and
   contributes real named/default export facts to the public-API family, so
   the shipped contract machinery (allowlist, `contract.expose`) engages on
   a ts tree without a vacuity carrier. A ts scan with more than zero
   scannable files and zero edges despite parseable import statements fails
   loud — the vacuous-parser tripwire: a parser that silently edges nothing
   cannot pass green. The capability table states the rows; the posture
   consequences of the file granularity are ADR-023's.
7. **ADR-006 revision.** A revision line in ADR-011's append style records
   that the deferral is decided by this design — not yet shipped — and that
   the no-wrap constraint stands: the driver is designed native, not a
   wrapper.
8. **Type-only facts.** Edges carry a `type_only` fact (ts-only; absent on
   the other drivers, whose serialized bytes are unchanged where it is
   absent). Edge facts are rows keyed `(from, to, kind)`: a pair carrying both
   a value import and a type import serializes two rows in deterministic key
   order; the other drivers' rows carry no kind key, their bytes unchanged
   under the model's serde default-and-skip-when-empty stability law. The cycle graph counts every edge regardless of kind. Dependency
   adjudication counts type-only edges by default too; one allowed or
   forbidden entry may demote them for its own rule through the entry-level
   `type_only = "count" | "exempt"` key (default `"count"`) — the exemption
   lives on the rule row, never on the language (ADR-023). A type-only
   cycle is still a cycle.

## Consequences
- One more language answers every verb (`scan`, `verify`, `diagram`, `report`,
  `doctor`) with no verb-specific code: the driver plugs into the model, the
  model into the engines.
- The native build surface grows by the TypeScript grammar crates; packaging
  and toolchain gates carry them like the existing grammar crates.
- Layering laws that lived in a hand-written scanner can now live in a spec;
  the pilot frontend's absorption is a repo-side follow-up outside this
  ADR, welded by a per-rule agreement proof before its scanner is deleted.
- `unresolvable import` can redden trees whose resolver needs exceed the
  inferred law (exports maps, conditional exports). The red is honest and
  names file plus specifier; the resolution law has a named extension point.

## Alternatives considered
| Solution | Verdict |
|---|---|
| **Native driver (chosen)** | Best fit — one model, one engine, one report. |
| Wrap dependency-cruiser | Rejected: ADR-006's constraint forbids building on arch-wrapper tools; its config is a second rule vocabulary the canonical model would contort around; findings would not grep into claims. |
| Keep TS/JS deferred | Rejected: the deferral's own condition ("revisit after core solid") is met; an invisible majority language in an active pilot keeps layering law outside the model. |
| `.svelte` treated as opaque (script blocks ignored) | Rejected: components would be vertices without edges — the pilot's majority modeled as nothing, a green that lies. |
