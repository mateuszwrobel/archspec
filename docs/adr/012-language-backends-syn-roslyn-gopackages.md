# ADR 012: Language back-ends — plugin drivers feeding the unified IR

## Status
Accepted

**Implementation Status:** Implemented with deviations as of September 2026 — Rust driver uses syn; C# and Go extraction shipped as in-binary parsers (`src/archspec/scan/{csharp,go}.rs`), not Roslyn / `go/packages` external processes. The separate-artifact plugin-driver model has not shipped; the binary shells out to no toolchain (doctor only probes PATH).

**Revised August 2026:** back-ends are plugin drivers (separate processes emitting canonical JSON IR), not code bundled into the Rust binary. Extraction is spec-driven (gather only what the spec needs). C# edge ground truth is project references, not `using`. Rust bundling of Roslyn rejected.

## Context
archspec needs one IR (ADR-007) fed by multiple language extractors. The extractor choice determines how the "two boundary levels" (ADR-009) and later symbol-level contract checks (ADR-011) are realized per language. Two viable C# routes exist: source-based **Roslyn** (matches the "no build step" model and gives symbols free) vs compiled-assembly route (Mono.Cecil, the classic NetArchTest/ArchUnitNET approach).

### Requirements
1. Extraction requires no prior build step (scan-as-is) for deterministic relations.
2. Same IR emitted from every back-end; shared core unchanged.
3. Symbol-level contract checks (phase 2, ADR-011) should be natural for each back-end, not a rewrite.
4. Tool extensible to new languages/frameworks without touching the core (ADR-006 "add back-ends cleanly").
5. Back-ends gather **only** the data the current spec needs (spec-driven extraction) — no unconditional full dumps.

### Constraints
- Parser libraries must match env: Rust (`syn` already proven in `rust-arch-test-kit`), C# (Roslyn is the only serious source parser), Go (`go/packages` / `go/ast`).
- Bundling Roslyn inside the Rust binary (via .NET hostfxr/nethost FFI) is high-risk: FFI bridge to a managed runtime, version targeting, maintenance — rejected as the default.
- Runtime presence for a language is **not a cost**: a user who wants architecture checks on their .NET/Go code already has that toolchain installed.

## Decision
Proposed → **Accepted**: three source-based **plugin drivers**, each a separate process emitting canonical JSON IR over stdout, fed to the shared Rust core:

- **One plugin type:** *Extractor plugins* — language-level (Rust / C# / Go). Turn `source tree + collect-manifest` → IR fragment (units, hard edges, soft structure, gating). Conventions live in the user's spec, never in core; there is no shipped recognition/profile plugin.
- **Rust** — `syn` parse of workspace + `src/` module tree; units = crates, hard edges = manifest deps (proven in existing kit).
- **C#/.NET** — **Roslyn** source analysis + `csproj/solution` project graph; units = projects, soft structure = namespaces/folders, **hard edges = project references** (not `using` — modern implicit/global usings make `using` scanning noisy and incomplete), gating = `#if`/conditional, symbols available for phase 2. **No compilation step required.**
- **Go** — `go/packages` import graph + `internal/` tree + `go.mod`; units = packages, hard edges = imports, contract = exported idents.

The **IR schema is the plugin contract** — the stable seam between core and any back-end. A driver is a pure function `source tree + manifest → canonical IR`; determinism is preserved by construction.

Extraction is **spec-driven**: the spec declares what it needs (project graph only vs symbols for module X). A planner derives the minimal collect-manifest per driver, so symbol-resolution cost is paid only when the spec actually has symbol contracts.

### Why this approach?
1. All three back-ends parse source → identical pipeline, no build prerequisites, deterministic scan.
2. Roslyn gives symbol tables for phase-2 contract checks without a second toolchain.
3. Drivers decouple language toolchains from the Rust core: `go/packages` behavior tracks Go releases, Roslyn tracks .NET SDK — they version independently.
4. New language (e.g. TS) = new driver, zero core change.

## Alternatives Considered

| Solution | No build step | Symbols for phase 2 | Determinism | Uniform IR | Core coupling | Verdict |
|---|---|---|---|---|---|---|
| **Plugin drivers (syn / Roslyn / go-packages)** (chosen) | Yes | Roslyn yes, syn yes, go yes | Strong | Yes | Low (IR seam) | Best fit |
| Roslyn bundled in Rust binary (hostfxr/nethost FFI) | Yes | Yes | Strong | Yes | High | Rejected — FFI bridge to managed runtime, fragile |
| tree-sitter C#/Go grammars in Rust | Yes | No | Strong | Yes | Low | Blocked — no symbols, fails phase 2 |
| Compiled-assembly C# (Mono.Cecil) | No (build first) | Yes | Depends on build freshness | Partial | n/a | Classic but breaks scan-as-is |
| Wrap NetArchTest/ArchUnitNET for C# | No | No (rules not model) | Weak | No | n/a | Rejected by ADR-006 |

### Alternative 1: Roslyn bundled in Rust
**Pros:** single artifact, no external runtime.
**Cons:** FFI bridge to a managed runtime is the design's biggest technical risk; toolchain versioning coupled to Rust releases; high maintenance. The "single artifact" benefit it buys is not worth it.
**Use Case:** rejected — drivers get the same single-CLI UX without the risk.

### Alternative 2: C# via compiled assemblies (Mono.Cecil)
**Pros:** battle-tested, fast, ecosystem-standard.
**Cons:** requires compiled output; scan reflects stale binaries; wraps a *model-less* rule world — would smuggle a different abstraction back in. Determinism depends on build state, not just source.
**Use Case:** .NET-native teams who already compile first and only check layers.

### Alternative 3: tree-sitter grammars for C#/Go
**Pros:** single Rust binary, cheap structure extraction.
**Cons:** CST only, no symbol tables — cannot deliver phase-2 symbol contract checks, which are the product's differentiator for C#/Go.
**Use Case:** rejected for phase 2; not worth building a throwaway phase-1 path.

### Alternative 4: Wrap existing C# libraries
**Pros:** minimal C# engineering.
**Cons:** violates ADR-006 (no wrappers), two IRs, no single source of truth. Rejected.

## Expected Consequences

### Positive
- Pure source pipeline; `archspec` works on an unbuilt checkout.
- Roslyn yields symbols for the phase-2 contract/agent use case for free.
- One shared core, three thin-ish drivers, clean extensibility seam.
- Toolchains version independently; new languages drop in as drivers.

### Negative
- Roslyn is a heavy dependency; extraction must be careful with parse-time performance on large solutions.
- C#/Go users must have that language's toolchain present (accepted — they already do if they write that language).
- `archspec doctor` (driver/toolchain diagnosis) needed for good DX.

### Risks
- Driver process spawn + JSON IR serialization is an I/O cost per scan — acceptable at repo scale, but large workspaces need parallel driver runs.
- `go/packages` requires `go` toolchain present at scan time; vendor-mode/offline repos need a fallback (`go/parser`). Record as open question.
- IR schema stability is load-bearing: it is the plugin contract. Version it (ADR-008 consequence).

## Open Questions
- Offline Go repos: accept `go`-toolchain requirement, or build a pure-parser fallback now? (lean: accept for phase 1, revisit if a real offline need appears)

## References
- docs/architecture-references/language-module-boundaries.md
- ADR-006, ADR-007, ADR-009, ADR-011
