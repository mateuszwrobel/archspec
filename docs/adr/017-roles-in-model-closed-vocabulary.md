# ADR 017: Role facts in the model — a closed two-role vocabulary

## Status
Accepted (2026-09-23). Records the decisions of the roles workplan
(model refactor, per-driver derivations, consumer re-keying, and the
laundering hardening that followed the re-audit). Amended (2026-09-23):
the go `facade` cell's bare non-derivability assertion is replaced by an
evidenced investigation record (§Go facade investigation) — the closed
vocabulary, the derivations, the capability rows and the rules are
unchanged. Cross-references
ADR-016 (syntax extraction as the sole path — the drivers this ADR extends)
and ADR-018 (identity ownership of allowances — consulted so role facts and
allowance lookups stay separate concerns).

## Context
The model already implied two structural roles — publication facade and
composition root — but never stated them. The facade fact lived in an
in-house model field skipped by serde: visible only to the rust verify path,
invisible to every consumer that reads the serialized model, and absent for
the other languages entirely. The consequences were observable in the
plan's audit record: an installable skill had to document the gap as a
standing blind spot ("legitimate DI wiring in an entrypoint file still shows
as entrypoint-to-layer edges"), the laundering check flagged sanctioned
composition wiring in entrypoint files as suspected conduits because every
fallback-resolved hop looked alike, and every consumer re-derived the roles
by eye, in its own dialect, from edge shapes.

The schema question forced itself on the refactor: the previous JSON shape
had been treated as frozen mid-release, which is why the field was skipped
rather than serialized. That constraint is gone — the version at decision
time is unreleased — and every "why is this node special" mystery class in
the audits traces back to a stated fact nobody outside one code path could
see.

## Decision
**The model carries a serialized `roles` map — model path → role — drawn
from a closed vocabulary of exactly two roles: `facade` and `composition`.
The in-house field is removed; the map is the single source every consumer
reads.**

- **Closed vocabulary.** Exactly two roles. These are the only role facts
  any rule or audit consumes; an open vocabulary invites per-driver dialects
  no consumer can match against.
- **Serialized, not in-house.** A roles map addressed by model path rides
  the scan JSON (`scan`), is restated verbatim by `report` (text section and
  JSON object), is marked on role-carrying nodes by `inspect tree`
  (`[facade]`/`[composition]` label suffixes; `<<facade>>`/`<<composition>>`
  stereotypes), and is the fact source of the `verify` rules below. The
  schema cost is paid now, pre-release, because it is free.
- **Derivations are per-driver and honest-absent.** Each driver derives
  entries only from facts its own extraction states; absence is a stated
  truth, never a default deny, and no driver guesses a role from foreign
  evidence:

  | driver | `facade` derived from | `composition` derived from |
  |---|---|---|
  | rust | a unit root file that defines no items — only `mod` declarations and re-exports | the bin unit whose `main` root wires modules |
  | csharp | re-export evidence at the resolved root — a using-only root file (plain or `global` usings, sentinel or under the root namespace) or an `[assembly: TypeForwardedTo]` forwarding file, either alone or together; a root file that declares a type is not a facade BY DESIGN (rust parity) | a unit whose entrypoint gate is open — a Program/Startup file, a host `Sdk` (`.Web`/`.Worker`) or `OutputType=Exe` — whose production files call the DI-registration-family members (lifetime, hosted, EF and caching vocabulary) anywhere, not only in the entrypoint file |
  | go | not derivable — **not emitted** (alias umbrella, public-package and root delegation patterns investigated 2026-09, see §Go facade investigation); the capability row states `role-facade not-emitted` and `verify` prints one role-scoped inert note per run citing this record | the `package main` unit root |

- **The capability table grows two role facts** (`role-facade`,
  `role-composition`) per language, and the facade rule re-keys from the old
  rust-only fact onto `role-facade` — one fact per role keeps the
  `capability <language> <fact> <granularity>` grammar untouched. Rows are
  written from live probes after the drivers ship, never from intent before
  it.
- **Verify rules keep their names, change their fact source.** The
  `facade dependency` rule reports an internal module depending on any path
  the map carries as `facade` — the rust umbrella and the C# using-only
  root alike — and is inert where the role is not derivable (go), stated as
  one note derived from the capability row rather than repeated noise. The
  laundering check exempts sanctioned wiring at hop level: a hop owned by a
  boundary carrying a `composition`-role path is never counted as fallback
  territory. The exemption's source set is keyed on the roles map paths —
  not on the module tier of the hop, and not on spec boundaries: deriving
  roles from declared boundary shapes was rejected (specs exist only where
  written; the scan must state roles with no spec present), and ADR-018's
  unit-vs-namespace identity semantics govern allowance lookups, not role
  derivation, so the two must not re-key each other.

## Consequences
- An agent that greps the scan JSON for the `roles` key can name a tree's
  facades and composition roots with no spec, no docs, and no language
  knowledge beyond the two words; the skill's blind-spot entry retires to
  the honest residue (go facade absence).
- Rust consumers see a one-key JSON delta (`roles`) on otherwise byte-stable
  models, and rust findings stay byte-identical: the re-keyed rule reads the
  map instead of the field, same findings, same exit codes.
- Pre-release schema churn is the price and it is paid once: no alias, no
  compatibility shim, no serde-skipped field. The map is empty-safe — a
  tree with no derivable role serializes no key at all, so old-shape readers
  that ignore unknown keys are unaffected either way.
- Honest absence is now load-bearing: `not-emitted` rows and inert notes
  are the contract that go facade roots go unchecked, not verified-clean.
  A future go facade fact must arrive as a capability-row change first.
- Sanctioned DI wiring stops reading as a laundering conduit without
  weakening the ban check: conduits through genuinely unclaimed territory
  still warn, and the exemption can never silence a direct violation.
- Trees or drivers that cannot derive a role produce no entries and no
  findings; nothing is guessed, so no false `facade` can launder a ban
  through a wrong role claim.

## Alternatives considered
- **Keep the field in-house, add per-language facts beside it** — every
  consumer re-derives roles and the blind spot survives verbatim; rejected
  as the status quo that motivated the plan.
- **Derive roles from spec boundary shapes** (a boundary granting
  everything is "composition") — rejected: specless scans would state
  nothing, an empty spec would silently void the fact, and it would couple
  model facts to declaration syntax (cross-reference ADR-018: spec-side
  identity semantics decide allowances, not roles).
- **Open-string or richer role vocabularies** (`layer`, `adapter`, per-driver
  strings) — no rule consumes them, and dialect drift makes the map
  unmatchable; rejected in favor of two closed values.
- **Fold roles into `soft_structure` or module-edge attributes** — roles are
  per-path facts, not per-edge or per-grouping ones; folding them would
  re-embed the fact in a consumer-specific shape, the exact failure mode
  being removed.

## Go facade investigation (2026-09 amendment)

The go `facade` cell above was first written from the roles plan's working
assumption — an unexamined assertion — not from a probe of real trees. It
is now a recorded decision with evidence. Three candidate derivation
patterns were probed against live go trees (the 14-package go application
in scope, scanned read-only, and planted fixtures) with a binary built
from `main`; the full probe record lives in this repository's development
worklog (`worklog/workplan_archspec_go_facade/findings.md`). The table
below is the findings table as landed, restated payload-neutral.

| candidate pattern | derivable from the go driver's own facts? | probe finding | false-positive risk |
|---|---|---|---|
| alias umbrella — a publication package whose whole surface re-exports another package via `type X = pkg.X` aliases and forwarding vars | **no** | the go fact surface is imports, exported selector names and the package clause; a type alias surfaces in **no** fact at all, and a forwarding var surfaces only as an ordinary selector fact — a planted alias umbrella and a planted thin wrapper package scan to a **byte-identical** model, so no predicate over existing facts fires on the umbrella and only on the umbrella | unmeasurable on the corpus (zero alias declarations across its 14 packages); an edge-shaped approximation would mark every ordinary thin-consumer or wrapper package |
| public-package marker — "public means not under an `internal/` path" | path-derivable but vacuous | `internal` exists in the scan model only as path text inside module addresses — no key, flag or fact distinguishes internal from public; on a tree without an `internal` directory every package gains the role (measured: 14 of 14 packages of the application in scope, composition root included) | near-total — the rule marks non-facades wholesale |
| root delegation — a root package that defines nothing and delegates everything | **no** | "defines nothing" is invisible to the facts (a delegating root and a wrapper-defining root scan byte-identical), and where a go root package exists it is usually `package main`, already claimed by the composition derivation — vacuous where the root is absent, colliding where present | vacuous on module trees without a root library package; re-flags composition roots |

**Closed statement.** No go source fact distinguishes a publication
umbrella from an ordinary package: forwarding and wrapping are
model-identical, and publicness is a path property every package shares.
The `facade dependency` rule therefore stays honestly inert for go — a
recorded decision, not an unexamined silence. The capability row stays
`role-facade not-emitted`; the `verify` inert note names this record, and
the go test suite's decision-surface guard keeps note and amendment in
agreement (wording drift on either side fails there, not in production).

**What would reopen this decision** — each is a change of facts, not of
wording:

1. a go syntax-layer declaration-shape fact (a `type_spec` whose right side
   is a qualified type after `=`, or a top-level `var` initialized from a
   qualified selector) would make the alias predicate derivable from the
   driver's own facts; per Consequences, the capability row flips first.
2. an alias-heavy go tree entering the corpus in scope (API-version
   publication roots) would give the false-positive clause real data in
   either direction — today the local corpus cannot measure it.
3. an owner-stated umbrella definition that ignores the forward-vs-wrapper
   distinction ("a package whose outgoing symbols mirror its single
   internal import's surface, whatever the declarations are") would make an
   edge-shaped predicate a definition rather than a false positive — a
   narrower re-argument no probe of current facts can settle alone.

## C# root identity ladder (2026-09 amendment)

Role and external attribution on C# key the unit's *root module*, and until
now that root was derived as the first two dotted segments of the unit name
when the unit declared no matching namespace. On realistic corporate trees
(three-segment naming like `Acme.Whatever.Service`) that rule invented a root
module no source file lives in: composition-root usings and sentinel roles
keyed to a phantom `Acme::Whatever`, and a sibling `Acme.Whatever` unit
silently swallowed the attribution. The rule is replaced by an identity
ladder, evaluated per unit:

1. the csproj's `RootNamespace` property, when stated (MSBuild's own root);
2. the csproj's `AssemblyName` property, when stated and `RootNamespace`
   is not;
3. the longest common dotted prefix of the namespaces the unit declares,
   truncated to the unit name's segment count (so a unit named
   `Acme.Whatever` whose files all declare `Acme.Whatever.*` roots at
   `Acme.Whatever`, not deeper);
4. the full unit name (a unit declaring no namespaces roots at its own
   name; its namespace-less composition files attribute there).

Two-segment and one-segment units resolve through the ladder to exactly the
keys the old rule produced, so their models are byte-stable; the ladder only
changes trees where the old rule truncated. Shared-root-key unions across
units (scenario 22's flattened package sets) now require the units to *state*
the shared identity (`RootNamespace`) — an implicit collision of truncations
was never a real identity claim. The decision surface is the `scan_reality_*`
guards in the C# test suite.

The same amendment scope-gates the merge. Composition beating facade is an
exclusivity WITHIN one unit: a unit claims at most one role, at its own
resolved root. Across units the merge states a role claimed by every
claimant once, and a key two units claim with different roles states
nothing — the ambiguity guard, the same honesty the driver applies where
one module would satisfy conflicting rules. One unit's composition claim
therefore never swallows, silences or outvotes a sibling unit's facade
claim; where the old global sets overwrote across units, both roots now
state their own role (or, on a genuinely shared root, neither does).

The facade evidence at those roots was next widened to the shapes real C#
re-exports actually take: a using-only root file (plain or `global` usings —
today's case), an `[assembly: TypeForwardedTo]` forwarding file (evidence on
its own — an assembly that forwards declares nothing and re-exports
everything), or both together. Wrapper-type roots stay unmarked BY DESIGN:
a root that declares a type is a wrapper, not a shell — the rust parity
rule, stated not accidental. The entrypoint gate opens no facade exception:
a Program-shaped root that wires nothing keeps the conduit reading, because
the laundering check's sanctioned shape (a using-only root passing
references through without owning them) is exactly that file — dropping
composition from it re-exposes the conduit rather than excusing it.
