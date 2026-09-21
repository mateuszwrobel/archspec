# ADR 010: Structural comparison and full-diff verification

## Status
Accepted

**Implementation Status:** Implemented as of September 2026 — `verify`/`report` render the full structural diff (violations, warnings, vacuous-constraint notes) between the code projection and the spec projection.

## Context
Verification must be *semantic*: the extracted component graph (from code) vs the declared component graph (from spec) must match in content, not in byte layout or ordering. And when it fails, the report must show everything that's wrong — not stop at the first violation — so engineers can fix a whole pass rather than iterate one error at a time.

### Requirements
1. Deterministic comparison independent of file/edge order.
2. Failure produces a **full diff**: added components, missing components, forbidden edges present, allowed edges absent, unassigned units, contract leaks.
3. Byte-stable rendering for the same model (so CI can diff artefacts).

### Constraints
- No type/property schema for the data beyond the model — comparison is over structured sets (components, edges), not serialized text.
- `verify` must exit non-zero on any violation (CI gate) but still complete the whole diff.

## Decision
Verification is **set-based structural equality** over the model:

- Component sets compare by identity (name/resolution); edge sets compare by (from, to) membership, order-insensitive.
- Canonical serialization of the model (stable ordering, hashable) enables quick equality probes and artefact diffing.
- On failure, the engine continues and reports **all** deltas across every check (component presence, edges, contracts, unassigned units) as a structured report, then exits non-zero.
- Rendering sorts canonically; identical model ⇒ identical output bytes.

This is the best solution because:
1. Order-insensitivity matches how engineers reason about boundaries (a cyclone `A→B→A` is the same structure regardless of edge ordering).
2. Full-diff makes fixing a passing test suite a single interaction per check category.
3. Deterministic byte output underpins the code↔diagram promise (ADR-007) and CI-friendly artefacts.

## Alternatives Considered

| Solution | Semantics | Failure behavior | Determinism | Verdict |
|---|---|---|---|---|
| **Set-based structural + full diff** (chosen) | Yes | full | Strong | Best fit |
| Textual diff of serialized models | Fragile | noisy | moderate | Diff of dump, not intent |
| Fail-fast single violation | Yes | first-only | n/a | Poor DX, no overview |

### Alternative 1: textual diff of serialized snapshots
**Pros:** trivial implementation.
**Cons:** ordering noise, no structured exit/report; intent lost in text.
**Use Case:** human eyeball only.

### Alternative 2: fail-fast
**Pros:** simple loop.
**Cons:** hides the map; a reviewer fixes one thing, re-runs, repeats. Contradicts full-diff requirement.
**Use Case:** smallest possible tool.

## Consequences

### Positive
- Deterministic CI artefact diffing.
- One fix-per-category workflow; report is structured and scriptable (JSON + readable).

### Negative
- Structural comparison engine must know canonical forms per element (name normalization, edge identity rules) — some design surface in phase 1.

### Neutral
- `verify` output doubles as report generation (`archspec report`) — same pipeline, two consumers.
