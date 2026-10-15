# ADR 021: Scan diagram opt-in tiers and claim-named module identity

## Status
Accepted (2026-10-15). Records the tier model behind
`archspec diagram --source scan --tiers` and the node-identity law it brings
to the module tier.

## Context
The scan surface rendered units and externals only. The snapshot's
`module_edges` tier — the granularity at which `verify` adjudicates
dependencies (ADR-018, ADR-020) — was parsed into the model and never drawn,
so a reviewer choosing the `scan` source got reality without modules, and
the `spec` source gave policy without reality. Two further facts shaped the
change. First, marking was a diagram-side re-derivation: the scan builder
consulted `allowed.forbidden` (plus contract stereotypes) and nothing else,
so an extracted edge that fitted neither the owning claim's `depend_on`
ceiling nor its prohibitions drew solid while `verify` reds exactly that
pair as a `disallowed cross-component dependency` — a diagram claiming calm
where the gate says stop, the docs-versus-binary drift family this crate has
outlawed. Second, committed diagram artefacts are gated by `diagram --check`
in CI: any always-on change to the module tier would silently stale every
committed scan diagram, spending the byte-change budget on a view rather
than on the one correctness fix.

## Decision
**The module tier of a scan snapshot renders only under an explicit opt-in,
and where it renders, a node's identity is the claim that adjudicates it.**

1. **Opt-in tiers.** `--tiers <list>` selects render tiers additively;
   supported tiers are `units` and `modules`, default `units`. The unit and
   external renders never change inside a flagged run — the module render is
   a `modules` wrapper cluster added beside them, one node per module-edge
   endpoint and one arrow per distinct resolved endpoint pair, canonically
   ordered. The flag is accepted only on the scan source; invalid names,
   duplicates, empty lists, and spec-source use are usage errors (exit 64)
   naming the rule broken.
2. **Claim-named identity.** Under a governing spec a module node's identity
   is its owning claim — the claim `verify`'s findings name (the
   most-specific `matches.modules` owner, or the unit fallback, per ADR-018
   and ADR-020) — so a finding greps into the diagram. The label carries
   both vocabularies (`api-billing (Sample::Api::Billing)`). Unowned
   endpoints keep the path name; a specless artefact renders paths as
   before. Identity resolution runs through the same adjudication seam
   `verify` consumes — one resolution path, no diagram-side ownership model.
3. **One mark vocabulary.** Every edge `verify` reds — `forbidden edge` or
   `disallowed cross-component dependency`, at either tier — renders dashed
   and labelled `violates spec`; an edge `verify` allows renders solid; a
   declared rule with no extracted edge draws nothing; containment and
   same-claim pairs draw no arrow. The always-on half of this change is
   exactly the ceiling-gap class: previously solid, now dashed — the one
   Breaking class, and the only default-byte movement.

## Consequences
+ `verify` and the scan diagram agree edge for edge in both formats and both
  flag states — the pair law is testable against one adjudication source.
+ Committed artefacts stay fresh by default; the flagged render is a pure
  addition to the default bytes.
+ A reviewer holding a red finding finds the node by grepping the claim
  name.
− A committed scan diagram that drew a ceiling-gap unit edge solid goes
  stale on upgrade: regenerate it, or complete the ceiling policy — the dash
  is the finding `verify` already raised.
− An artefact module path that no claim owns renders path-named: honest by
  construction, no implied ownership `verify` does not see.
