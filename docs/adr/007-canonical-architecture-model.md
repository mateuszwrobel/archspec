# ADR 007: Single canonical architecture model as the source of truth

## Status
Accepted

**Implementation Status:** Implemented as of September 2026 — the canonical model lives in `src/archspec/model.rs`; source and spec both project into it and compare/report diff the projections.

## Context
The core value of archspec is a deterministic relation: **when source code satisfies the spec, a diagram generated from the code equals a diagram generated from the spec.** To guarantee this, there must be exactly one representation of architecture; both source and spec are different views of it. The previous generation of tools mixed the concerns: rules lived in config, graphs were recomputed from source only, and diagrams were drawn by hand and inevitably drifted.

### Requirements
1. Deterministic, byte-stable diagrams given the same input model.
2. One model that both *extraction* (from source) and *declaration* (in spec) produce.
3. Language-agnostic shared core (mapping, comparison, rendering) that back-ends feed.

### Constraints
- Diagram is an **artefact**, derived from the model — never hand-maintained, never the spec itself.
- Model must be hierarchical: hard boundary units + soft namespace/folder structure above/below units (see ADR-009).

## Decision
Maintain **one canonical architecture model (intermediate representation)**:

- **Unit** — hard boundary (Rust crate, C# project, Go package), with exact dependency edges.
- **Component** — declared module over one-or-more units or namespace/folder patterns; recursive (components contain sub-components).
- **Edge** — dependency between units (extracted) and between components (declared).
- **Contract** — declared exposed surface + allowed/forbidden neighbors per component.
- **Gating** — conditional presence (features, `#if`, build tags).

**Diagram = pure function of the model.** Extraction and declaration are two independent producers of the model; verification compares them structurally (ADR-010).

This is the best solution because:
1. Determinism is by construction: same model → same rendering, no matter the producer.
2. One shared core serves every language back-end.
3. Hierarchical IR keeps the door open for per-component internal architecture (ADR-009, phase 2).

## Alternatives Considered

| Solution | Determinism | Single source of truth | Extensible to per-module arch | Verdict |
|---|---|---|---|---|
| **Canonical model, dual producers** (chosen) | Strong | Yes | Yes | Best fit |
| Rule engine over raw source (rust-arch-test-kit model) | Weak | No | No | Rules ≠ model |
| Golden snapshot of the diagram file | Medium | No | No | Snapshot drifts, not a model |

### Alternative 1: Rule engine over raw source
**Pros:** matches existing kits; quick to validate single constraints.
**Cons:** no unified model, no shared rendering, pristine determinism not possible; per-module architecture impossible.
**Use Case:** single-language linter, not a polyglot product.

### Alternative 2: Golden snapshot of rendered diagram
**Pros:** trivial compare.
**Cons:** text-diff noise, no abstraction, cannot emit spec or other formats from the same source, ties artifact to a specific renderer.
**Use Case:** testing renderers only, not architecture.

## Consequences

### Positive
- Determinism guaranteed structurally (ADR-010) — the user-visible promise.
- One rendering stack for all formats and languages.
- Extensible: new back-ends and boundary declarations plug into the same model.

### Negative
- IR design is load-bearing; changes to it ripple to all back-ends.
- Extraction complexity lands mostly in back-ends (ADC 012), but IR constraints must stay stable.

### Neutral
- Model serialization format (stable canonical text, hashable) is an implementation detail but a phase-1 deliverable.
