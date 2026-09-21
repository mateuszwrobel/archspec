# ADR 006: New multi-language architecture tool replaces single-language tools

## Status
Accepted

**Implementation Status:** Implemented as of September 2026 — `archspec` ships as the binary of this repository (`rust-arch-test-kit` crate converted to the archspec tool); the Rust TOML rule engine named in the Context has been removed, so the "existing tools" landscape described here is historical.

## Context
Architecture enforcement today is fragmented: `rust-arch-test-kit` covers Rust with a TOML rule engine; NetArchTest / ArchUnitNET cover C# with fluent assertions over compiled assemblies; dependency-cruiser covers JS/TS. Each has its own model, config, and reporting. Modern products span these languages (a .NET modular-monolith back-end plus Rust/Golang services) yet get no shared architectural view. Existing tools force drawing diagrams by hand, which drift.

### Requirements
1. One tool for architecture description, verification, and diagram generation across Rust, C#, and Go.
2. Deterministic relation between code and generated diagrams.
3. Full design upfront; lifecycle-friendly architecture (can be modified during development).

### Constraints
- Do not build on top of / wrap existing arch libraries (no NetArchTest, no dependency-cruiser, no ArchUnit fork).
- TS/JS is out of scope for now (dependency-cruiser already serves it); revisit later.
- Must keep the good parts of the existing `rust-arch-test-kit` idea (config-driven, module-graph diffing, rendering).

## Decision
Build a **new, standalone, multi-language tool** (working name `archspec`) that replaces `rust-arch-test-kit` and the other single-language tools. For Rust + C#/.NET now, Go later (not before phase 1). TS/JS deferred.

This is the best solution because:
1. A single canonical model (ADR-007) yields one language for describing architecture and one verification engine.
2. A new tool avoids contorting an existing single-language code base into polyglot shape and avoids inheritance of its rule-DSL.
3. Modular architecture allows adding back-ends cleanly (ADR-012).

## Alternatives Considered

| Solution | Polyglot model | Single source of truth | Unification of rules | Verdict |
|---|---|---|---|---|
| **New tool** (chosen) | High | Yes | Yes | Best fit |
| Extend `rust-arch-test-kit` | Low | Partial | No | Rust-tied, rule DSL baked |
| Wrap NetArchTest/dependency-cruiser | Medium | No | No | Config fragmentation remains |
| Roslyn analyzers only | Low | No | No | Statement-level, not boundary-level |

### Alternative 1: Extend `rust-arch-test-kit`
**Pros:** existing syn-based Rust parser, tests, publishing pipeline.
**Cons:** Rust-shaped IR and TOML rules baked in; C#/Go would be second-class; name and behavior mislead.
**Use Case:** team only ever writes Rust.

### Alternative 2: Wrap existing libraries
**Pros:** fast to stand up per language.
**Cons:** three rule models, three reports — only shallow unification; violates requirement for one model.
**Use Case:** throwaway validation, not a product.

## Consequences

### Positive
- One model and one verification engine across three languages.
- Deterministic diagrams (ADR-007, ADR-010).
- Modern, from-scratch design; can absorb per-language conventions cleanly.

### Negative
- Bootstrapping engineering for 3 back-ends up-front.
- No migration story (ADR-013): legacy `rust-arch-test-kit` keeps publishing for external consumers; the new tool is adopted by testing it on this repo's own projects.

### Neutral
- Product name and packaging to be decided (see ADR-013).
