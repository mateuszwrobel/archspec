# ADR 019: Per-driver public-API facts under one constraint

## Status
Accepted (2026-09-29). Records the decision behind the C# public-API gap:
`public_api_allowlist` was defined over the rust crate-root export family
only, so on a C# tree it compared `allowed` patterns against facts no C#
source ever populates — the rule reported violations of nothing while real
public surface went unchecked.

## Context
The rust fact source is the unit's root file: named `pub` items plus names
enumerated from resolvable root glob re-exports (`root_public_exports`,
`root_glob_exports`, `root_empty_glob_exports`). C# has no root-file analog —
its surface is the set of types each namespace declares `public`. A single
`[[constraint]] type = "public_api_allowlist"` was already documented for all
languages, and the capability table's job is to state exactly this kind of
asymmetry rather than paper over it.

Two shapes were possible. A C#-specific constraint type would have kept the
TOML honest but forced users to learn which spelling their language uses,
and would have duplicated the vacuity, severity and reporting machinery.
A shared type keyed on whichever fact source the driver populates keeps one
user contract and lets the capability matrix state per-driver granularity —
including `not-emitted` for a driver that can never engage the rule for real.

## Decision
`public_api_allowlist` stays one constraint type and gains a second,
module-tier fact source alongside the rust root family:

- The C# scan records `module_public_types`: module path → the explicitly
  `public` types of its production sources (`class`/`interface`/`struct`/
  `enum`/`record` with an explicit `public` modifier; partials deduped;
  nested types qualified through their parent; a namespace-less file
  attributes to the unit's root module key; a nested block namespace records
  under its own innermost name, the spelling the soft tier already uses).
  The test tier contributes nothing, matching the other project-tier facts.
- Enforcement attributes every public type to its module. A module counts as
  allowlisted when an `allowed` pattern glob-matches its module path. An
  entry claims **no ancestor territory**: gating `A::B` does not expose
  `A::B::C`, mirroring the rust root shape where only listed exports are
  allowed. A leak names module and type: `module exposes Type (not
  allowlisted)`.
- Engagement honesty: a tree with module facts whose `allowed` patterns match
  no fact-carrying module states the vacuity naming the patterns; a C# tree
  with no public-API facts at all states `no public API facts to check`. The
  rust sentence (`no crate-root public exports to check`) stays byte-verbatim
  for rust trees, pinned by the rust vacuity guards.
- The capability matrix states a `public-api-surface` fact — `granular` for
  rust (root family) and C# (module map), `not-emitted` for go, whose
  allowlist can therefore never engage beyond the fact-emptiness vacuity.

## Consequences
- One TOML contract, one finding vocabulary (`public api leak`), two fact
  sources; the model field is `serde`-skipped when empty, so rust and go
  scan output stays byte-identical.
- The rule's answer is now computable from the model alone for both fact
  families; neither can pass silently on an empty surface.
- Go keeps no public-API fact source. That is stated in the table, not
  discovered by a silently-passing check.
