# ADR 018: Identity ownership of allowances

## Status
Accepted (2026-09-22). Records the decision behind audit defect D5 (a C# audit
finding: four false `disallowed cross-component dependency` warnings on a real
solution).

## Context
A spec may declare a project's territory twice: once as a unit-tier boundary
(`matches.units`, the composition unit) and again as a module-tier boundary
(`matches.modules`, the namespace identity). The same dependency then reaches
the boundary graph twice — once through a hard unit edge (the project
reference) and once through module edges attributed under the namespace
identity: cross-project `using`, injected field types, and composition-root
files whose usings are attributed to the project's root module because the
file declares no namespace.

Until now each attribution resolved to its own declared boundary and the
allow-list was consulted on whichever boundary the endpoint resolved to.
Namespace boundaries typically grant nothing — they exist to address
territory, not to permit dependencies — so the namespace attribution
re-adjudicated a dependency the unit-tier attribution already allowed against
a boundary whose empty allow-list defaults to deny. The grant was voided in
silence: the dependency was reported as disallowed while no rule ever denied
it.

## Decision
**Allowances are owned by the boundary that matched the edge's source unit
identity: an edge attributed under a namespace/module identity resolves its
allow/deny against the spec module claiming that unit; a module-tier
attribution can never re-adjudicate a dependency the unit-tier attribution
already allows.**

When the source identity of a pair resolves to a unit claimed by a boundary's
`matches.units`, the allowance lookup consults that boundary — with the
target expressed under the same unit attribution — instead of the module-tier
boundary the namespace happened to resolve to. Where no unit attribution
exists (specs whose boundaries are claimed only by module patterns) the
existing name-equality lookup governs, unchanged. An attribution that resolves
both endpoints into one boundary stays module-attributed: a dependency inside
a single unit is adjudicated by the module boundaries that name it, exactly
as before.

## Consequences
- A unit-tier allowance survives namespace attribution; identity-split false
  warnings disappear without touching any model fact, the pair graph, or the
  cycle computation.
- The rule is an allowance lookup, not an exemption: a dependency an owning
  unit boundary does not name remains reported — the default-deny stance of
  the owning boundary is what decides, and a composition root reaching a
  transitive dependency no rule declares stays a genuine finding.
- Findings keep the attributed pair as their reported text; only the
  adjudication moves. Forbidden-edge and missing-edge checks are unchanged,
  so the fix cannot widen what is reported either.
