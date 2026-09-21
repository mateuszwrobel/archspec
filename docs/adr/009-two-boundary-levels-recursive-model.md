# ADR 009: Two boundary levels — hard units plus soft structure — in a recursive model

## Status
Accepted

**Implementation Status:** Implemented as of September 2026 — unit and module boundary levels enforced by `verify`; nested submodules enforced by contract checks (submodule contracts landed September 2026).

## Context
In every supported language, "module" is not one primitive (see docs/architecture-references/language-module-boundaries.md). There are always two kinds of boundaries:

- **Hard boundaries** — compilation/import units with exact edges: Rust crate (workspace member), C# project (csproj + project references), Go package (plus the compiler-enforced `internal/` privacy subtree).
- **Soft boundaries** — namespace/folder/visibility groupings *above and below* the hard units: Rust `src/` module tree, C# `Features/{Feature}` namespaces, Go `internal/domains/{d}/...` folders. These carry the layer/feature/hexagonal structure, are enforced by nothing, and drift first.

The tool must see and verify **both**, at lower levels of structure too — not only "top-level directory" granularity (the limitation of the old `rust-arch-test-kit`).

### Requirements
1. Model both boundary kinds, with hard-unit edges as ground truth and soft structure as first-class verified content.
2. Recursive hierarchy: a component contains sub-components (e.g. .NET module = a set of projects, where `.Abstractions` is a sub-unit with its own contract role).
3. Governance reverse: support declaring module boundaries at the granularity the architecture actually needs — crate/package/project when that's the boundary, namespace/folder when those are.

### Constraints
- Composition path must remain detectable in all languages (Rust `main.rs`/`server` crate, Go `cmd/*`, C# host `Program.cs`) — a core reuse for phase-2 composition-root checks.
- Keep per-component internal architecture (nested sub-components) representable in the IR even though phase-2 enforcement comes later.

## Decision
Model boundaries at **two levels, recursively**, where each component:

- resolves to a set of **hard units** (unit globs / project patterns / package patterns);
- may hold **soft sub-structure** (namespace/folder matches) as nested sub-components;
- may itself be nested inside a larger component.

Verification reports drift at the level it happens — a crate boundary guard and a `Features/{Feature}` namespace guard are the same mechanism on different IR levels. Composition-root identification exists in the IR from day one (used by rendering and phase-2 checks).

This is the best solution because:
1. Faithfully matches how all three ecosystems actually express boundaries.
2. Hard units keep verify honest (compiler-true edges); soft structure is where the value-add is (nothing enforces it today).
3. The recursion is exactly what "each module can have its own internal architecture" (modular-architecture-rules.md #5) needs.

## Alternatives Considered

| Solution | Hard truth | Soft structure | Granularity freedom | Per-module arch | Verdict |
|---|---|---|---|---|---|
| **Two-level recursive model** (chosen) | Yes | Yes | Any | Yes | Best fit |
| Hard-units only | Yes | No | unit-level only | No | Blind to layers/features |
| Soft-structure only (folders/namespaces) | No | Yes | fine | Partial | Drift undetected at compile level |
| Fix top-level-only (old kit model) | Partial | Partial | coarse | No | Known limitation |

### Alternative 1: Hard units only
**Pros:** pure, simple IR.
**Cons:** misses the entire world of namespace/folder conventions where .NET/Go/Rust actually live their architecture; contract checks impossible on feature slices.
**Use Case:** build-graph tooling, not architecture enforcement.

### Alternative 2: Soft structure only
**Pros:** fine granularity.
**Cons:** treats conventions as truth — drift between soft and hard boundaries (e.g. .NET module projects vs namespaces) invisible.
**Use Case:** folder hygiene checkers; insufficient for boundaries.

## Consequences

### Positive
- Faithful per-language boundary semantics in one IR.
- Same diff/verify engine serves both levels.
- Recursive design ready for phase-2 per-component internal architecture without a model break.

### Negative
- IR and nested-sub-structure semantics are more complex than a flat unit graph; needs careful spec wording so internal architecture doesn't become a second hidden language.

### Neutral
- Matching expressions (globs, namespace/folder patterns) are a phase-1 API to define precisely and re-use across boundary declarations.
