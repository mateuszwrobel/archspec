# archspec — Multi-language Architecture Test & Diagram Tool (Design)

> **Status:** Design draft (pre-workplan). Working name only — final name pending.
> **Scope:** Rust + C#/.NET (+ Go later). TS/JS explicitly out of scope for now.
> This is the full upfront design. It will be modified during development; ADRs record frozen decisions, this doc tracks the living picture.
>
> **Locked (August 2026):** no conventions in core (stereotypes/arch are user-declared in spec, ADR-011); back-ends are plugin drivers emitting JSON IR (ADR-012); outputs = Mermaid + PlantUML only, no SVG (ADR-013); no profile catalog — the spec is shape-agnostic and `init` writes a minimal base spec; distribution = cargo + static binaries + thin wrappers; no migration story (ADR-013).

---

## 1. Problem

Architecture — the set of components, their boundaries, and their dependency rules — is the most expensive thing in a codebase to get wrong and the least guarded.

Today that guarding is fragmented and single-language:

| Tool | Language | What it does |
|---|---|---|
| `rust-arch-test-kit` (this repo) | Rust | TOML rule config → module-graph diff + DOT/Mermaid render |
| NetArchTest / ArchUnitNET | C# | Fluent arch rule assertions over compiled assemblies (Mono.Cecil) |
| dependency-cruiser | JS/TS | Rule validation + graph render, own config/engine |
| Roslyn analyzers | C# | Statement/symbol-level compile-time rules |

Each has its own config language, its own model, its own reporting. No shared mental model. No deterministic relation between code and the diagram you draw to explain it. And none handle the reality reads in `docs/architecture-references/modular-architecture-rules.md`:

- modules differ and don't need the same internal architecture;
- boundaries exist at development-time, deployment-time, and runtime;
- a module is defined by **what it hides**, and its contract is what may cross the boundary.

## 2. Vision

Two complementary flows built on **one** idea:

1. **Existing code** → scan → extract model → render diagrams → review → commit spec.
2. **New code** → write spec first → render diagrams *from the spec* (planning) → implement → verify code matches spec.

And the guarantee: **when code satisfies the spec, the diagram generated from the code is identical to the diagram generated from the spec.** Code ↔ diagram relation is deterministic.

This replaces the single-language tools above, including `rust-arch-test-kit`.

### Why deterministic-by-construction

- There is exactly **one canonical architecture model**.
- Source code is a *view* of it (extraction).
- The spec is a *view* of it (declaration).
- A **diagram is a pure function of the model**, never of the source directly.
- **Verification** = structural comparison of the two views. Equal views ⇒ equal diagrams. Always.

No rule engine, no proposition logic, no maintenance of two sources of truth. The comparison *is* the test.

## 3. Core concepts

### 3.1 The model (intermediate representation, IR)

A **hierarchical**, **recursive** graph.

| Concept | Meaning | Rust | C#/.NET | Go |
|---|---|---|---|---|
| **Unit** | hard boundary, compiler/build enforced | crate | project (csproj) | package (+ `internal/`) |
| **Component** | declared logical module (one-or-more units, or a namespace/folder) | module declared over crates/src-folders | module declared over projects/namespaces | module declared over packages |
| **Edge** | dependency between units (extracted) and between components (declared) | crate dep / `crate::` | project ref / `using` | package import |
| **Contract** | what a component may expose outward + allowed/forbidden neighbors | | | |
| **Gating** | conditional presence | `cfg(feature)` | `#if`, conditional | build tags |

Two boundary kinds are both first-class in the model:

- **Hard boundaries** — the compilation/import units. True edges, cheap to extract, compiler can't lie. Ground truth.
- **Soft boundaries** — namespace/folder/visibility groupings *above and below* hard units. This is where "layered", "feature", "hexagonal" structure actually lives, expressed by convention, enforced by nothing — exactly why we verify it.

### 3.2 Spec = code, diagrams = artefact

The **spec is the test definition, written in code** (declarative TOML), and it is the source of truth for verification. Diagrams are **derived artefacts** — rendered output, never the spec itself, never hand-maintained.

A spec declares:

- a **stereotype vocabulary** — user-defined named match-sets (what an "entity", "service", "DbContext" *is* for this repo). The core has **no built-in conventions**; all conventions live here, in the spec;
- which components exist, with matching rules over the model (unit globs, namespace patterns, folder patterns);
- dependency constraints between components (allow / forbid, cycle constraints with severity);
- each component's contract (declared exposed surface, referencing declared stereotypes); a per-component declared **internal substructure** (nested sub-components).

### 3.3 Per-component internal architecture = declared nested structure

A component declares its **internal architecture** as nested sub-components over the recursive model (ADR-009): hexagonal (ports/adapters), onion/clean layers, vertical feature slices, DDD aggregates — different per component. There is **no recognition logic**: an internal architecture is just a declared set of nested components + their dependency edges, verified by the same module-level machinery on a deeper IR level. This is the "each module can have a different architecture" requirement, aligned with `modular-architecture-rules.md` rule #5.

## 4. Pipeline

```
source tree + manifests
        │
        ▼  plugin driver (language extractor, subprocess → JSON IR)
┌───────────────────────────┐
│  IR: units, edges, usage  │   (hard truth + soft structure)
└───────────────────────────┘
        │
        ▼ map (match declared components over units)
┌───────────────────────────┐
│  component model (extracted)         │
└───────────────────────────┘
        │                ▲
        │                │ structural compare
        ▼                │
┌───────────────────────────┐
│  component model (declared)│  ← tested against source
└───────────────────────────┘
        │
        ├─ pass ──► render diagram (arte fact)  [deterministic]
        └─ fail ──► full diff report            [exit non-zero]
```

- Structural equality is **set-based** (edge membership), order-insensitive; rendering uses canonical ordering so output is byte-stable.
- Diff reports **everything** that differs, not the first hit: added/missing components, forbidden edges, disallowed cross-components deps, unassigned units, contract leaks.
- Extraction is **spec-driven**: the spec declares what it needs; a planner derives the minimal collect-manifest per driver, so symbol resolution is paid only when the spec has symbol contracts.

## 5. CLI sketch

```
archspec init          # write minimal base spec ([project] language + global no_cycles guard)
archspec scan          # extract model only (no spec needed)
archspec diagram       # render model (extracted or declared) to artefact
archspec verify        # extract + compare vs spec, full diff, exit code
archspec update        # snapshot current model as seed spec (existing-code flow)
archspec report        # text/markdown/json diff + metric output
archspec spec          # print the spec JSON schema or annotated reference
archspec doctor        # diagnose which language drivers/toolchains are present
archspec inspect       # zero-config file-level import map (discovery)
archspec depgraph      # current-state module/submodule/API-usage dependency views
archspec help          # built-in manual: topics + per-command help
```

Output formats (artefacts): **mermaid, plantuml** (deterministic text). No SVG/images. C4-style and metrics later. Screenshot/PDF via Playwright rendering of Mermaid/PlantUML in HTML is an *external reporting* step, not a core renderer.

`verify` flags: `--strict` promotes all warnings (e.g. `no_cycles` severity=warning) to errors for CI gates.

## 6. Spec example (illustrative TOML)

```toml
[project]
language = "csharp"            # rust | csharp | go

# user-declared stereotype vocabulary — core has no built-in conventions
[[stereotype]]
name = "entity"
match = { names = ["*Entity", "*AggregateRoot"], paths = ["**/domain/**"] }

[[stereotype]]
name = "dbcontext"
match = { names = ["*DbContext"] }

[[module]]
name = "Billing"
matches = { units = ["Billing*", "Billing.Abstractions", "Billing.Data.*"] }
# contract: what this module may expose outward (phase 1: stereotypes; phase 2: symbols)
contract = { expose = ["service", "event"], forbid = ["entity", "dbcontext"] }
# allowed neighbors and banned ones
[module.allowed]
depend_on = ["Shared", "Payments.Abstractions", "Catalog.Abstractions"]
forbidden = ["Billing.Data", "Portal"]

[[module]]
name = "Portal"
matches = { units = ["Portal*"] }

# cycle constraint: warning for existing code, error for greenfield; --strict promotes
[[constraint]]
type = "no_cycles"
modules = ["Billing", "Portal"]
severity = "error"
```

An example spec can encode the .NET modular-monolith convention (see `archspec/examples.md`): cross-module talk only via `.Abstractions` sub-units; implementation units never referenced cross-module. These are illustrative, not shipped defaults.

## 7. Language backends — plugin drivers

All backends emit the same IR; the shared core (mapping, compare, diff, render) is language-agnostic. **Each backend is a plugin driver**: a separate process emitting canonical JSON IR over stdout. The **IR schema is the plugin contract**. Runtime presence for a language is not a cost — a user who writes .NET/Go already has that toolchain. Backends are *extractors* (language-level, code → IR); there is no recognition/plugin type and no profile shipping.

| | Rust | C#/.NET | Go |
|---|---|---|---|
| Extractor | `syn` source parse + workspace `Cargo.toml` | Roslyn source parse + `csproj` project graph | `go/packages` |
| Units | workspace crates; `src/` top-level modules as soft structure | projects; namespaces/folders soft | packages; `internal/` scoping |
| Edges (hard) | crate deps from manifests | **project references** (not `using` — implicit/global usings are noisy) | package imports |
| Contract signal | `pub` surface at crate root | `public` types; `internal` by default; `.Abstractions` projects | exported (capitalized) idents; `internal/` |
| Gating | `#[cfg(feature)]`, cargo features | `#if`, conditional compilation | build tags |
| Notes | parse-only, mirrors modern `rust-arch-test-kit` | source-based, no build step; Roslyn gives symbol-level for phase 2 | cheapest of the three; import graph + `internal` tree |

Extraction is **spec-driven**: the spec declares what it needs; a planner derives a minimal collect-manifest per driver so symbol resolution is paid only when the spec has symbol contracts.

Go: `internal/` is compiler-enforced privacy — the single real soft/hard crossover primitive. C#: the ecosystem convention (modular monolith abstractions projects) is the richest to recognize.

## 8. Shape-agnostic spec; no profiles

The spec is **shape-agnostic**: it declares boundaries and relations, never the tree's physical shape. The same declaration works for a single crate, a multi-crate workspace, and any hybrid (see `archspec/spec.md`). Boundaries address two independent tiers via `matches.units` (hard tier: crates/packages) and `matches.modules` (soft tier: module paths inside units). `verify` resolves declared boundaries against whatever the tree has.

There is **no profile catalog and no recognition engine**. Earlier designs shipped "profiles" — ready-made spec templates for a framework convention — materialized by `init`. That concept was removed: the spec format is uniform across shapes, `init` writes only a minimal `[project] language` header plus a global `no_cycles` guard, and `update` seeds boundaries from the actual tree (same guard), so scaffolded projects start cycle-clean without any engine-level default. Illustrative starter specs for common shapes live in `archspec/examples.md` (no runtime meaning).

## 9. Verification semantics

| Check | Phase | Detail |
|---|---|---|
| Component set | 1 | declared components exist; unknown units reported (unassigned / unexpected) |
| Dependency edges | 1 | extracted edges ⊆ allowed; no forbidden edges; full edge-set diff |
| Exposed-surface stereotypes | 1 | nothing bleeds out of declared contract (stereotype-level, against user-declared stereotype vocabulary) |
| Cycles | 1 | `no_cycles` over declared groups; severity warning/error; `--strict` promotes |
| API-symbol contract leaks | 2 | symbols crossing a boundary ⊆ declared contract symbols |
| Composition-root placement | 2 | wiring only in the component **declared** as root (no convention recognition) |
| Internal architecture | 1+ | declared nested sub-components (ADR-009) — same verify machinery, deeper level |

Structural equality: **set-based edges**, stable canonical serialization, hashable model snapshot so `verify` can short-circuit. Full diff always computed on failure.

## 10. Phasing

### Phase 1 (core value)
- Driver extraction: rust, csharp (Roslyn), go (packages) — units + hard edges + soft structure
- Spec (TOML): components, matching, allow/forbid, stereotypes, no_cycles severity
- Mapping + set-based structural diff, full report
- `init` minimal base spec (§8: shape-agnostic, no profiles)
- Diagram renderers (mermaid, plantuml) from model, canonical deterministic output
- CLI + cargo publish + static binaries in releases

### Phase 2
- API-symbol contract leak checks
- Composition-root wiring checks (root declared in spec)
- Richer artefacts (C4-style, markdown tables, metrics); Playwright screenshot/PDF as external reporting

### Deferred (not before phase 2)
- TS/JS. Existing dependency-cruiser served that world; revisit after core solid.

## 11. Open questions

- Final product name (note collisions: Spack's `archspec` Python lib, ASDF "ArchSpec").
- Static binaries in releases: Linux-only initially, or Linux + macOS from day one?
- Offline Go repos: accept `go`-toolchain requirement (lean) vs pure-parser fallback.
- Confirm boundary granularity default (hard-unit-first; see ADR-009) and how a single unit's internal modules are declared via `matches.modules`.

## 12. References

- `docs/architecture-references/modular-architecture-rules.md` — philosophy foundation
- `docs/architecture-references/language-module-boundaries.md` — research per language
- ADR-006 … ADR-018 in `docs/adr/`
- Prior art: NetArchTest, ArchUnitNET, dependency-cruiser, cargo workspaces, golang-standards/project-layout, Microsoft/JSdotNet modular monolith ADRs, hexagonal-rust-template
