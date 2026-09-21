# ADR 011: Contract model and phase scoping

## Status
Accepted

**Implementation Status:** Implemented as of September 2026 — declared stereotypes, nested sub-components, `no_cycles`, and symbol-level contract checks (`public_api_allowlist`) shipped; composition-root placement checks remain unshipped.

**Revised August 2026:** declared stereotypes (user-defined vocabulary), internal architecture dissolved into declared nested sub-components (ADR-009), composition root declared in spec, `no_cycles` severity, phase 2 shrunk to symbol-level contract checks.

## Context
The phrase "nothing bleeds out of its boundaries" is the heart of the tool (docs/archspec-design.md §3, §9). A component must expose only what its contract declares, and depend only on declared neighbors. But contract checks need symbol-level analysis, which is expensive and only sensible after the module-level graph is solid. The scope must be split into phases so phase 1 delivers real value without symbol resolution, and phase 2 closes the leak-detection gap over API symbols.

> Note on scope of ("API-symbol leak checks"): a *leak* means "a symbol used by a consumer beyond the component's declared contract" — e.g. a C# `internal` type reached via `InternalsVisibleTo`, or a Go exported identifier outside `internal/` not in the declared contract. It is **not** a git/GitHub concept. (Clarified during design; recorded to prevent re-introduction of confusion.)

### Requirements
1. Phase 1 (core value): module-level verification — component presence, allow/forbid dependency edges, exposed-surface *stereotypes* (what *things* may cross: service vs entity vs DbContext), unassigned units, cycle constraints. Full diff (ADR-010).
2. Phase 2: symbol-level contract enforcement — symbols crossing a boundary must be a subset of the declared contract symbols; composition-root wiring checks.
3. Designates to-be-maintained design seam between phases so phase 2 does not break the IR.

### Constraints
- **The core has no built-in conventions.** Stereotypes, framework conventions, and declared internal structure are *user-declared in the spec* (match rules over extracted structure). The core only evaluates declared rules against extracted structure.
- Symbol resolution is a back-end/plugin concern (Roslyn for C#, syn for Rust, go/packages for Go) and belongs to phase 2, not phase 1.
- The IR must already carry a contract slot per component (declared surface + allowed/forbidden neighbors) so phase 1 = stereotype-level, phase 2 = symbol-level, without model change.

## Decision
**Two contracting phases, unioned in one IR.**

*Phase 1 contract checks:*
- declared component set matches extraction (missing/extra/unassigned reported);
- extracted edges ⊆ declared allowed; forbidden edges reported (full diff);
- **declared stereotypes:** the user declares a stereotype vocabulary (`[[stereotype]]` tables: name + match rules over unit/component names, paths, namespaces). A component's exposed items are classified by these declared stereotypes; exposure must fall within the declared contract stereotypes and never within the forbidden stereotypes. No naming conventions are hardcoded in the core — the user declares the vocabulary directly in the spec;
- **cycle constraints:** `no_cycles` over declared component groups, with `severity = "warning" | "error"` per constraint. Warnings for existing codebases (fix progressively), errors for greenfield. `--strict` promotes all warnings to errors (CI gate);
- unassigned units (units matched by no component) surfaced for review.

*Phase 2 contract checks:*
- symbol-level subset check per directed boundary edge (exported symbols crossing a boundary ⊆ declared contract symbols);
- composition-root wiring checks — the composition root is **declared in the spec** (user marks the component where wiring is allowed), never recognized by convention.

*What phase 2 is NOT:*
- No separate internal-architecture engine. Per-component internal structure is expressed as declared nested sub-components over the recursive model (ADR-009). Verification is the same module-level check as everything else, on a deeper IR level. The recursive model exists for this now; only symbol-level analysis is deferred to phase 2.

This is the best solution because:
1. Phase 1 ships the whole module-level value prop with cheap extraction and zero built-in conventions.
2. Phase 2 slots into the same contract pipeline without an IR break — its only new machinery is symbol resolution in the plugins.
3. Symbols are the eventual input for coding-agent contract reviews — the seam is designed now, filled later.

## Alternatives Considered

| Solution | Module-level value | Symbol-level | Conventions in core | Verdict |
|---|---|---|---|---|
| **Declared stereotypes phase 1 → symbol phase 2** (chosen) | Phase 1 | Phase 2 | None | Best fit |
| Symbol-level from day one | Delayed | Day one | n/a | Too slow to first value; 3 back-ends × symbol tables |
| Stereotype-level only, forever | Yes | Never | n/a | Leaks via specific symbols undetectable |
| Core with built-in naming conventions | Yes | Phase 2 | Yes | Conventions baked in, not user-defined — rejected |

### Alternative 1: Symbol-level from day one
**Pros:** complete contract picture immediately.
**Cons:** symbol resolution × 3 languages before any diagram exists; blocks the core value.
**Use Case:** research prototype of leak detection, not v1.

### Alternative 2: Stereotype-level only
**Pros:** cheapest.
**Cons:** misses the actual drift mechanism (a specific type leaking), which is the coding-agent use case from design §2.
**Use Case:** folder hygiene decks.

### Alternative 3: Conventions baked into core
**Pros:** stereotype detection works out of the box for common stacks.
**Cons:** couples core to ecosystem fads; convention drift forces core releases; contradicts the "spec is code" principle (ADR-008) where intent lives in the spec.
**Use Case:** rejected — conventions belong to the user-declared spec.

## Consequences

### Positive
- Fast, verifiable phase 1 with the full module/diagram workflow.
- Phase 2 extends without model redesign; its only new work is plugin symbol resolution.
- No core convention vocabulary to maintain or version.
- Symbol-check use case gets an explicit seat (agent contract review).

### Negative
- Full leak detection is deferred; phase 1 contract checks are coarse by nature.
- User must declare a stereotype vocabulary before stereotype checks work — the user declares it directly in the spec (no shipped catalog).

### Neutral
- Contract slot semantics (stereotype taxonomy) is a small, phase-1-defined vocabulary to extend in phase 2.
