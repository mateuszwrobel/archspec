# ADR 008: Spec is code (declarative component definitions); graphs are artefacts

## Status
Accepted

**Implementation Status:** Implemented as of September 2026 — `scan` emits the model JSON, `diagram`/`report`/`depgraph` render projections of it, and `update` derives a spec from code.

## Context
For the "write tests first" flow, the desired architecture must be authorable before (or without) any code. Two candidate representations exist:

1. **Spec as expected-graph snapshot** — a compiled graph file that `verify` diffs against extraction. Simple, but the spec is then a dump, not a description: reviewing a diff of edge lists says nothing about *intent*, and per-module contracts/internal architecture can't be expressed.
2. **Spec as code (declarative component definitions)** — the spec declares components, their matching rules, dependency constraints, and contracts. It is the test definition; graphs and diagrams are derived artefacts.

The vision (docs/archspec-design.md §2-3) requires both planning (diagram from spec alone) and enforcement (verify code against spec), with deterministic equality of the two flows.

### Requirements
1. Spec is authorable first (planned-architecture flow) and from existing code (discovery flow) alike.
2. Verification compares *semantics* (which components exist, which edges are allowed) not a text dump.
3. Graphs/diagrams are never maintained by hand; they are always rendered from the model.

### Constraints
- Declarative, git-diffable, reviewable by humans and agents.
- A recovery path from existing code: `archspec update` snapshots the current model *as a seed spec* (edge-set realized via declarations) to be reviewed, not blindly trusted.

## Decision
**The spec is code** — a declarative TOML document that *defines* components to test source against:

- component declarations with matching rules (unit globs, namespace/folder patterns);
- dependency constraints (allow/forbid) between components;
- per-component contract (declared exposed surface; stereotype-level in phase 1, symbol-level in phase 2);
- per-component declared internal substructure (nested sub-components over the recursive model, ADR-009).

Graphs are a **derived artefact** rendered from the model — the model being the union of declarations and (checked) extraction. The snapshot/`update` flow generates a *candidate spec* from extraction; the developer reviews and commits it.

This is the best solution because:
1. Spec expresses intent and is reviewable as prose-with-structure, not a differential dump.
2. Diagram-from-spec and diagram-from-code are the same rendering function over the same model shape, preserving determinism (ADR-007/010).
3. Everything future (contracts, internal architecture) roots in the same declarative document.

## Alternatives Considered

| Solution | Expresses intent | Authorable first | Extensible to per-module arch | Verdict |
|---|---|---|---|---|
| **Spec = declarative component definitions** (chosen) | Yes | Yes | Yes | Best fit |
| Spec = expected-graph snapshot | No | Partially | No | Dumb, drifting diff |
| Spec = imperative rule DSL (like NetArchTest/ArchUnit) | Yes | Yes | Weak | Reintroduces rule-engine fragmentation (contrary to ADR-006) |

### Alternative 1: expected-graph snapshot
**Pros:** trivial to compare.
**Cons:** a diff of edge lists carries no intent; cannot express contracts/internal architecture; seeds ambiguity in reviews.
**Use Case:** low-grade golden tests of very small systems only.

### Alternative 2: imperative rule DSL
**Pros:** familiar to ArchUnit/NetArchTest users; expressive.
**Cons:** three languages have three idioms; the model is implicit, the rules are scattered; deterministic equivalence of flows is hard to prove.
**Use Case:** a catch-all for teams that insist on imperative constraints — explicitly out of scope.

## Consequences

### Positive
- Human- and agent-reviewable intent; clean git history of architecture changes.
- Discovery flow = candidate spec produced, then judged — same file format as planned flow.
- No hand-maintained artefacts.

### Negative
- Spec/IR semantics must be well-specified and versioned (breaking changes to declarations must be migrations).
- The seed-spec from `update` needs a review discipline, else it celebrates the current state instead of intent.

### Neutral
- Format is TOML to match repo conventions (ADR-013); the shape-agnostic spec format removes authoring burden (design §8).
