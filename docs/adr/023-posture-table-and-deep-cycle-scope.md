# ADR 023: A per-language posture table sets the cycle scope; deep cycles adjudicate red where the facts exist

## Status
Accepted (2026-10-15). Implemented (2026-10-16): the `scope` field, the
declared posture table and the red posture shipped across phases B and C of
the design record, and the rust row's named flip landed the same day —
rust → both with the portfolio's intra-claim file cycles cleaned and the
zero-count re-probe measured at the flip change itself. Records the doctrine
change behind the `scope` field of `no_cycles`, the per-language posture
table that defaults it, and the severity posture that keeps deep-cycle
enforcement red.

## Context
`no_cycles` adjudicates between the groups the spec claims: a cycle whose
files all live inside one claim's territory is invisible — the check's graph
never reaches inside a claim. The ts driver (ADR-022) makes that gap
concrete: a frontend's file graph is dense with mutual imports, and a
granularity the tool cannot see is a granularity nobody enforces. Adding the
adjudication raises three doctrine questions this ADR answers: which schema
carries the choice, what each language's default is, and what severity the
default carries.

The severity question is a feedback-loop question. This fleet's codebases are
written substantially by generation loops, and a generation loop respects
exactly one signal: a red verdict. A deep-cycle check that defaults to warning
is a soft silent-green — the issue #3 class this crate has repeatedly
outlawed: the check runs, prints softly, and the loop proceeds as if nothing
spoke. The vacuity law closes the other end: a rule that cannot engage must
say so in the report (capability, tier, pointer), never pass in silence —
which fixes what a run whose file adjacency is empty must print.

## Decision
1. **One schema field.** `no_cycles` gains an optional `scope` string —
   `claims` (today's adjudication, byte-unchanged) or `both` (the claim tier
   plus cycles over the file-grained adjacency inside the constraint's
   territory, in one run). A files-only value does not exist: every cycle
   the claim graph reddens is red under both values, and the claims scope's
   verdicts stay byte-identical. A `both` run reports the claim tier first
   and suppresses a files-tier SCC iff the nearest claiming ancestors of its
   members — the common-ancestor function applied member-wise, over the claims
   union a synthetic tree root — form a cycle already reported at the claim
   tier; a files-tier cycle whose claim projection is not itself a reported
   claim cycle prints beside the claim-tier findings. One violation, one
   finding, coarsest tier first; suppression keys on the claim PROJECTION,
   never on member-set equality — claim-tier cycles are claim sets, files-tier
   cycles are file sets. The
   field is accepted on `no_cycles` rows only; on any other constraint row
   it is an invalid-spec error naming the field and the row kind. This arm
   adds one schema field; the design as a whole registers THREE schema keys —
   `scope` here, the `[project] excluded_sources` entries and the entry-level
   `type_only` key on allowed/forbidden entries (ADR-022 states both) — each
   validated at spec load in the spec grammar, each invalid value an
   invalid-spec error at the historical spec-validation exit; the severity ratchet reuses the shipped
   `constraint.severity` (`error` default, `warning` explicit). The finding
   stays the shipped cycle category: at the file tier one cycle is one
   finding attributed to the **nearest common ancestor of all its members,
   computed over the claims union a synthetic tree root** — the root
   always exists, so the ancestor is total; members under disjoint claims
   attribute to the synthetic root, where the finding names every member
   file and states that the members share no claiming ancestor — the
   same common-claim-ancestor attribution law the contract checks ship, so
   nested and flat specs stay byte-identical (the pair law holds at the new
   scope);
   cycle adjacency is the union over FILE identities, so mirrored unit
   identities dedup to file pairs.
2. **A declared per-language posture table defaults the scope.** An absent
   `scope` derives from declared data beside the capability table, not from
   a silent per-engine guess. The capability matrix gains the row the posture
   reads — fact `file-grained-dependency-facts`: granular for the ts driver
   (its edges are file-grained by construction) and granular for the rust
   driver (its `file_edges` rows serialize file→file adjacency — the flip
   named here landed 2026-10-16); not-emitted for go and csharp. A weld
   test binds the two tables:
   every posture row whose scope is `both` has the capability granular for
   that language. At this decision's landing the posture was: **ts → both,
   error**; **rust, go, csharp → claims, error** (rust's behavior then
   unchanged). The rust row flipped to **both, error** as the separate
   named change this decision required, carrying its three preconditions:
   the one-time cleanup of the existing rust-portfolio intra-claim file
   cycles — the resolved inventory of 16 SCCs over 54 distinct files is
   retired, the fleet measures zero intra-claim file cycles — file facts
   serialized in the model, and a fresh zero-count re-probe measured at the
   flip change itself. The posture now reads: **rust → both** and
   **ts → both**, both at error; **go, csharp → claims**, at error.
   The python driver landing (0.6.9) entered at **python → both**, at
   error, and the kotlin driver landing (0.6.16) entered at
   **kotlin → both**, at error: both drivers' file facts are granular
   from their first release, so the posture's precondition holds at the
   landing itself.
   Flipping blind would have reddened the portfolio without current
   evidence; the evidence is on record.
3. **Red is the posture; warning is an exception under a guard.** No
   language's default severity is `warning`. `warning` remains available
   exactly as shipped — an explicit per-spec `severity = "warning"` row used
   while pre-existing violations are cleaned up — and the exception is
   falsifiable, not prose: a checked-in warning-severity baseline lists
   every warning row the fleet accepts, and any `severity = "warning"` row
   absent from the baseline fails the portfolio guard unless the same change
   updates the baseline. `--strict` stays red over it; it is never a shipped
   default and never a way of living.
4. **Vacuity keys on facts, not on the row label.** Any run whose file
   adjacency is empty or absent says so through the shipped
   `vacuous constraint` category with the `[constraint #N]` carrier, the
   capability named, and the capability help as the pointer — whether the
   scope came from an explicit `both` or from a posture derivation, and even
   when the capability row reads granular. Not silence, not a bespoke
   warning class — the vacuity law's exact shape.

## Consequences
- Cycle findings gain a granularity without gaining a category, a severity
  vocabulary, or an attribution rule: one field selects the graph the shipped
  engine walks.
- The posture table is grep-visible declared data; a reviewer can read every
  language's default in one place, and the rust flip is a one-row change
  with named preconditions.
- On the ts pilot the single counted file cycle is fixed inside the pilot
  change at error severity — no warning-severity row appears in the pilot
  spec; the warning-severity baseline guard keeps any future warning row
  visible and reviewable, and strict runs are red from day one.
- A type-only edge counts in both adjudications by default — dependency
  and cycle graphs agree — and one dependency rule may demote it to a
  graph member that satisfies no rule and violates none through the
  per-entry `type_only` key on that rule; the exemption lives on the rule
  row, never on the language: the two adjudications differ only where a
  spec says so, by law, not by accident.
- Go and C# trees gain nothing and lose nothing: their default stays
  byte-identical, and an author asking for the file tier there is answered
  by the carrier rather than by a false green.

## Alternatives considered
| Solution | Verdict |
|---|---|
| **`scope` field on `no_cycles` + posture table (chosen)** | One vocabulary, one severity, one attribution; defaults declared, not guessed. |
| A second `no_file_cycles` constraint type | Rejected: a second cycle vocabulary for one concept — the drift class the marking law of ADR-021 already outlawed; more schema surface, not less. |
| Warning-by-default for deep cycles | Rejected: a warning is a green a generation loop does not read — the issue #3 class; the baseline-guarded per-spec exception carries real migrations honestly. |
| Files-only or always-both scope on every language | Rejected: a files-only value drops the claim tier a spec relies on today, and an everywhere-both default runs the file tier vacuously on go and csharp — the vacuity law demands the carrier, not an everywhere-default. |
| Required `scope` (no default) | Rejected: a breaking schema edit for zero behavioral gain; every existing spec would churn to restate its current tier. |
