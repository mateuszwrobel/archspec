# ADR 020: Module-claim jurisdiction over nested submodules

## Status
Accepted (2026-10-14). Records the jurisdiction model that lets a nested
`[[module.submodules]]` claim engage in dependency adjudication at the same
strength as a top-level claim.

## Context
A spec declares boundaries at the top level and nests structure below them
with `[[module.submodules]]`. Contract enforcement already recurses: a
submodule's `contract.forbid` is enforced over the surface crossing the
submodule's boundary path. The dependency half never followed. A nested
`[submodule.allowed]` table was parsed, displayed by `depgraph submodules`
and `diagram`, and consulted by no check: an edge of the submodule's own files
resolved to the nested claim as the most specific owner, found no enforced
rule there, and passed — while the flat equivalent of the same `matches` and
policy failed. Nesting a layer was therefore a silent downgrade of its rules:
splitting one top-level claim into parent-plus-submodules could both void the
ceiling (an ignored nested `depend_on`) and void the prohibitions (files no
longer covered by the parent's `forbidden` once a more specific claim owned
them). Two questions had no answer: which claim adjudicates an edge whose
territory is claimed at two depths, and what a nested claim without an
`allowed` table means.

## Decision
**A claim that carries an `allowed` table has full dependency jurisdiction
over the files it claims, at any depth; the most-specific claiming claim owns
an edge's adjudication exactly as it does for flat top-level claims.** The
model is four laws plus their addressing rules:

1. **Most-specific ownership.** A nested claim's `matches` participate in
   edge ownership with the same specificity semantics as flat claims; its
   `allowed.depend_on` and `allowed.forbidden` gate the edges of the files it
   claims. A nested claim and its flat equivalent produce identical verdicts
   and identical finding text. The ambiguity check spans all depths: an
   equal-specificity tie across nested and flat claims fires the existing
   `ambiguous module match` finding, and declaration order never changes a
   verdict.
2. **Inheritance for policy-free nesting.** A nested claim without an
   `allowed` table is structure and contract only: its files' dependency
   edges are governed by the nearest ancestor claim that has an `allowed`
   table. A top-level claim without an `allowed` table keeps its current
   meaning, unchanged.
3. **Containment exemption.** A dependency edge whose endpoints both resolve
   within one parent's claim subtree, and where one endpoint's claim is a
   direct-or-transitive ancestor of the other, is composition: never
   adjudicated, never a violation, rendered as structural nesting. Sibling
   claims under one parent adjudicate normally against the source claim's
   rules, so intra-layer boundaries ("billing must not touch legacy") stay
   expressible.
4. **One-way prohibition ratchet.** An ancestor's `allowed.forbidden` lists
   and `contract.forbid` stereotypes bind every descendant claim's files
   cumulatively. A descendant may narrow `depend_on` allowances but can never
   cancel an ancestor's prohibitions; an empty descendant `forbidden` list
   relaxes nothing.

Addressing and tiers: claim names are globally unique across all depths and
are valid `depend_on`/`forbidden` targets at any depth. `matches.units` on a
submodule is rejected at parse — submodules claim the module tier only; the
unit tier keeps its existing semantics at top level. Engagement is mandatory
at every depth: a claim matching no model element, a claim fully shadowed by
an equal-or-more-specific claim, a dead reference inside a submodule allowed
list, and a declared-but-unexercised `depend_on` at depth each surface as a
finding (the last two reuse the existing `dead reference` and missing-edge
findings), so engagement opens no new silent-green locations. The pass
confirmation counts every claim at every depth, and the spec-source diagram
renders submodule claims and their declared edges at full depth — nesting for
containment, arrows for allowances.

## Consequences
- Nested and flat spellings of one boundary behave alike: same verdict, same
  finding text, same counts line. The equivalence is observable: the pair of
  specs verifies with byte-identical output.
- Verdicts change on upgrade in named classes. Green→red: nested `allowed`
  tables that never enforced now enforce; equal-specificity collisions across
  the nested/flat seam now report `ambiguous module match`; claims matching
  nothing or shadowed by others now warn (and fail under `--strict`).
  Red→green: parent↔own-submodule containment edges are exempt from
  adjudication. Contract-only nesting (no `allowed` table) is verdict-stable
  by the inheritance law.
- Splitting a layer into submodules can no longer weaken the layer's rules:
  prohibitions ratchet downward, and narrowing stays a deliberate act.
- Nesting becomes safe to recommend in guidance: it is either equal
  (with `allowed`), inheritive (without), or composition (containment) —
  never silent.
- The diagram and the verdict share one claim-resolution path, so the diagram
  cannot display allowances `verify` does not enforce.

## Alternatives considered
- **Reject nested dependency rules at parse.** Rejected: the docs promise
  recursion into submodules and `depgraph`/`diagram` build module structure
  from nested claims; rejection fights the rest of the tool and strands the
  nested `allowed` table as an error instead of a feature.
- **Flat-parity deny-all for nested claims without an `allowed` table.**
  Rejected: it flips every existing contract-only nested spec green→red on
  upgrade while closing no silent hole — those specs state structure and
  contracts, not dependency policy.
- **Require an `allowed` table on every nested claim.** Rejected: it forces
  authors to duplicate the parent's policy at every child, and duplicated
  policy drifts out of agreement with the parent it copies.
- **Treat every intra-parent edge as composition.** Rejected: it makes
  intra-layer boundaries unexpressible — the sibling case (billing reaching
  legacy inside one layer) is exactly what authors nest to control.
- **Most-specific ownership at the unit tier as well.** Rejected: the unit
  tier is first-declared-wins today; changing it is a second breaking class
  unrelated to module nesting, so nested `matches.units` is rejected at
  parse instead.
