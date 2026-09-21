# ADR 013: Spec format, implementation language, and distribution

## Status
Proposed → **Accepted**

**Revised August 2026:** drivers ship as separate artifacts; thin wrappers per ecosystem; output formats = Mermaid + PlantUML only (no SVG/images); no migration story; no profile catalog — the spec is shape-agnostic and `init` writes a minimal base spec.

**Implementation Status:** Implemented as of September 2026 — TOML spec (`architecture.spec.toml`), Rust binary, GitHub release publishing shipped (pipeline wiring is internal, not documented here); `init` writes the minimal base spec, no profile catalog shipped. Note: the "drivers ship as separate artifacts" deviation is recorded in ADR-012 — drivers shipped in-binary instead.

## Context
Three product-level decisions are still open: the spec/declaration file format, the tool's implementation language, and how it ships. These set the developer experience and the repo conventions.

### Requirements
1. Spec is declarative, git-diffable, reviewable by humans and coding agents (ADR-008).
2. Implementation must host 3 parser stacks (syn, Roslyn, go — ADR-012) and a shared core.
3. Distribution must not exclude C#/Go teams.

### Constraints
- Repo convention is TOML for configs.
- The project's AI/agent tooling is Rust-based; synergy matters for the agent-contract-review use case (ADR-011, phase 2).
- **No migration story required.** The new tool is adopted by introducing it to this repo's own projects and testing it on them. Legacy `rust-arch-test-kit` keeps publishing for external consumers; no parallel-publish migration plan.
- **Output formats: Mermaid + PlantUML only.** Both are deterministic text formats — no SVG, no images, no embedded renderers. Screenshot/PDF via Playwright rendering of Mermaid/PlantUML in HTML is possible later as an *external reporting* step, never a core renderer.

## Decision
- **Spec format: TOML** (`architecture.spec.toml`), versioned schema. Matches repo conventions, native serde support, git-friendly, human-written.
- **Implementation language: Rust** (single CLI binary + shared `archspec-core` lib). Language back-ends are **plugin drivers** (ADR-012): separate processes emitting canonical JSON IR over stdout. The Rust binary orchestrates, extracts Rust itself, and shells to the C#/Go drivers.
- **No profile catalog.** The spec is shape-agnostic (single crate, multi-crate workspace, or hybrid — see design §8). `init` writes a minimal `[project] language` base spec; `update` seeds boundaries from the actual tree. There is no profile-resolution engine and no shipped template vocabulary.
- **Distribution:**
  - Rust core/CLI published as a cargo library/binary (consistent with `rust-arch-test-kit`'s existing cargo publishing).
  - **Static binaries per platform published to releases** so C#/Go teams without cargo can adopt.
  - **Driver artifacts ship separately:** C# driver as a .NET tool, Go driver as a Go binary/release artifact.
  - **Thin wrappers per ecosystem** (cargo crate / npm package / NuGet package) wrap the same binaries where a native package is expected. Wrappers contain no logic.
  - `archspec doctor` diagnoses which drivers/toolchains are present and which languages can be scanned.

### Why this approach?
1. TOML: zero new tooling, right-sized for declarative specs, serde-native, human-written.
2. Rust: single binary, existing parser base (`syn`), agent-tooling synergy, strong CLI ergonomics.
3. Drivers decouple distribution: each ecosystem gets its native toolchain without coupling the Rust release to Roslyn/Go versions.
4. Mermaid/PlantUML text keeps the byte-stable determinism promise (ADR-007/010) without pinning a layout engine.

## Alternatives Considered

| Solution | Spec format | Implementation | Distribution | Outputs | Verdict |
|---|---|---|---|---|---|
| **TOML + Rust + drivers + wrappers** (proposed) | TOML | Rust | cargo + binaries + thin wrappers | Mermaid, PlantUML | Best fit |
| Roslyn bundled single binary | TOML | Rust | cargo only | Mermaid, PlantUML | Rejected (ADR-012) — cargo-only excludes C#/Go teams |
| YAML spec + Node CLI | YAML | Node | npm | any | Wrong strength (Node vs Roslyn paring), foreign to repo |
| C# CLI + Roslyn-native C# backend | JSON | C# | NuGet | any | One backend native, two foreign |
| Multi-artifact (npm/nuget/cargo) | TOML | mixed | 3 registries | any | Wrappers already cover this without 3 logic-bearing codebases |

### Alternative 1: Node CLI
**Pros:** JS/TS ecosystem familiarity, fast startup.
**Cons:** no native Roslyn; would shell out for C# parsing; foreign to repo conventions; requires Node runtime on CI.
**Use Case:** teams exclusively in JS tooling.

### Alternative 2: C# as primary implementation
**Pros:** Roslyn is already C# — zero FFI; top-notch C# parsing.
**Cons:** Rust and Go parsing from C# are painful; repo/agent tooling is Rust-first; NuGet-only distribution excludes Rust consumers of the shared core.
**Use Case:** .NET-centric product shop.

### Alternative 3: Multi-artifact distribution with logic in each
**Pros:** native package in every ecosystem.
**Cons:** triple CI, triple versioning, triple support. Wrappers (thin, no logic) get the native-package benefit without the triple codebase.
**Use Case:** unnecessary — wrappers cover it.

## Expected Consequences

### Positive
- Matches repo depths: TOML, Rust, cargo.
- Deterministic text artefacts (Mermaid/PlantUML) — byte-stable promise holds (ADR-007/010).
- Shared core reusable as a library (`archspec-core`) by agent tooling for contract review.
- C#/Go teams adopt via static binaries/wrappers, no cargo required.

### Negative
- Multiple artifacts to version (core, drivers) — versioned as a set.
- Driver runtime presence required per language (accepted, ADR-012: users already have their toolchain).
- No inline images — architecture pictures require the external Playwright step later.

### Risks
- Driver/IR contract stability is load-bearing — version the IR schema (ADR-008 consequence).
- TOML spec versioning discipline needed to avoid breaking changes.

## Open Questions
- Final product name (working name `archspec`; note collisions: Spack's `archspec` Python lib, ASDF "ArchSpec").
- Static binaries on releases: build all three platforms from phase 1, or Linux-only initially? (lean: Linux + macOS, Windows later)

## References
- ADR-006, ADR-008, ADR-012
- docs/archspec-design.md §11 (open questions)
