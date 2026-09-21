# How Modules & Boundaries Work in Each Language

> Reference research backing the archspec design (ADR-009, ADR-012). Captures how "module" is actually expressed at development-time in Rust, C#, and Go — because in no language is a module a single primitive. Everywhere there are two levels: **hard** (compiler/build enforced) and **soft** (convention/enforced-by-nothing) boundaries. A boundary-literate tool must model both.

---

## Rust

### Boundary primitives

| Level | Primitive | Enforcement |
|---|---|---|
| Hard | **Crate** (cargo workspace member) | Compiler — dependency declared in `Cargo.toml`. Strongest lock in the language. |
| Soft | **Module** (`mod` / folder under `src/`) | Convention + `pub` / `pub(crate)` / `pub(super)` visibility only. |

### How it's laid out

- **Cargo workspace** = collection of crates sharing `Cargo.lock` + `target/`. Root manifest lists `members`, shared deps via `[workspace.dependencies]`.
- Single crate: `src/lib.rs` / `src/main.rs` + module tree. `src/bin/*` for extra executables.
- Crates *can't* import crates not listed in `Cargo.toml` → workspace crate graph = exact dependency truth.

### Common architectures seen

- **Hexagonal, workspace style**: coarse domain crates → composition root. Typical:
  ```
  crates/
    domain/        # types + ports (traits), deps: none
    application/   # use-cases, deps: domain
    adapters/      # http/db, deps: application, domain
  server/          # binary composition root — only place that wires concrete adapters
  ```
  "Dependencies point inward. Nothing else matters as much as this."
- **Hexagonal, single-crate**: `domain/`, `ports/`, `adapters/` as modules; `#[cfg(feature = "...")]` gates adapters; ports/domain never feature-gated.
- **Layered single crate**: `api/`, `domain/`, `infra/` modules; dependency direction inward.
- **Feature gating**: cargo features only ever switch adapters in/out. Capability features on the crate that owns adapter (`vision_v4l2`), profile features only on composition root. Ports & domain always compiled.

### Contract signal

- What crosses a crate boundary = **public API re-exported through crate root** (`pub mod`, `pub use`), only what's accessible via the crate name. Everything else is `pub(crate)`/private = hidden.
- Cargo feature set is part of the published contract surface.

---

## Go

### Boundary primitives

| Level | Primitive | Enforcement |
|---|---|---|
| Hard | **Package** (import unit) | Compiler — a package can only import what's exported (capitalized) and allowed. Compiler also rejects import cycles outright. |
| Hard/soft | **`internal/` subtree** | Compiler-enforced privacy: packages under `internal/` importable only by code rooted at the parent of that `internal/`. *The* real Go boundary tool. |
| Hard | **Module** (`go.mod`) | Versioning + distribution unit; import path prefix. |
| Soft | folder layout | Convention (golang-standards layout is *not* endorsed by Go team). |

### How it's laid out

- Official guidance: single module, packages map to directories, `internal/` for private, `cmd/` for executable entry points.
- `pkg/`: optional, contested; explicit "reusable outside" signal. Go team recommends *not* adding by default.
- Go tooling (`go list`, `go/packages`) gives the package graph + dependencies for free.

### Common architectures seen

- **Hexagonal**:
  ```
  cmd/                       # entry points
  internal/
    core/ domain/ ports/ services/
    adapters/ handler/ repository/
  ```
- **DDD**:
  ```
  internal/domains/{domain}/
    domain/            # entities, value objects
    application/       # use-cases/services
    infrastructure/    # db, external
    interfaces/        # http/grpc adapters
  ```
- **Feature-oriented packages**: `internal/orders`, `internal/payments`, `internal/postgres` + `cmd/myapp`. Greek the feature; technical packages only when they're a real technical boundary.
- **Dependency inversion**: the *consumer* package defines the interface (`orders.Repository`), `postgres` implements it. `orders` never imports `postgres`.

### Contract signal

- Exported = capitalized identifier. Public contract = exported identifiers of packages outside `internal/`.
- `internal/` = "may change freely, not part of public API". Moving a package up/down across `internal/` changes its contract class.

---

## C# / .NET

### Boundary primitives

| Level | Primitive | Enforcement |
|---|---|---|
| Hard | **Project** (`.csproj`) + **project references** | Compiler at build. Exact project graph from solution/csproj. |
| Soft | **Namespace + folder** | Convention only — unless an analyzer (NsDepCop) enforces namespace-level rules at compile time |
| Soft | **visibility modifiers** (`public`, `internal`, etc.) | Compiler for the modifier itself; convention for the boundary policy |
| Hard-ish | **DOOR: `InternalsVisibleTo`** | `internal` types escape only where explicitly granted |

Framework (.NET/ASP.NET) is the *most* convention-rich of the three — the interesting structures live at project level, not source level.

### How it's laid out (dominant 2025+ conventions)

**Modular monolith** (the flagship pattern):
```
src/
  Core/                                 # shared CQRS interfaces, primitives
  {Module}/                             # short module name: Orders, Billing
    Company.Product.Orders/             # domain + application + features
    Company.Product.Orders.Abstractions/# PUBLIC CONTRACT: DTOs, service interfaces, events
    Company.Product.Orders.Api/         # minimal-API host (one per web module)
    Company.Product.Orders.Data.Postgres/# optional persistence adapter
  Shared/
    Company.Product.IntegrationEvents/
tests/
  Company.Product.ArchitectureTests/
```
Boundary rules (the ones worth verifying):
- Module → other module **only via the other's Abstractions project**; never the implementation project.
- `Abstractions` → nothing owns module impl (no circular reference); minimal deps only.
- Commands/queries are **internal** to the module; DTOs live only in Abstractions.
- `internal` by default; expose only registration ext `Add{Module}Module(this IServiceCollection)`.
- Each module owns its own schema/DbContext/migrations. Cross-module: Abstractions interfaces or integration events, never shared tables, never direct handler calls.

**Clean Architecture**: Domain / Application / Infrastructure / (Web) projects; dependency rule points strictly inward.

**Hexagonal / ports & adapters**: core (domain + application) with port interfaces; adapters reference core, never reverse.

**Vertical slices within module**: `Features/{FeatureName}/` namespaces; per-op files `{Op}Command|Query.cs` + `{Op}Handler.cs` + `{Op}Response.cs`; sealed records.

**Single-project clean**: one `.csproj`, layers as namespaces `Domain.*`, `Infrastructure.*`, `*Features.*`; enforced via **NsDepCop** source analyzer (error on violation).

### Contract signal

- Public contract = **public types in the Abstractions/Contracts project**.
- `internal` types = hidden. Leak vectors: (a) bare `public` types in module impl, (b) `InternalsVisibleTo` grants, (c) project reference reaching into a module's implementation project.
- Package/NuGet reference lines are the "external dependency" surface per project.

---

## Cross-language synthesis — module model for the tool

1. **Two boundary kinds, both first-class**: hard units (crate / project / package) with exact edges, and soft structure (namespace/folder/module/visibility) above and below them.
2. **Module is declared, not discovered**: a module = declared grouping over units + namespace/folder patterns. .NET module = a *set of projects* (Abstractions is a sub-unit, not a friend module). Rust boundary module in a workspace = one crate + its feature flags. Go module = package group under `internal/`.
3. **Contract is per-module and different per language**:
   - Rust: crate-root `pub` surface + feature set.
   - Go: capitalized exports; `internal/` marks private class.
   - C#: `.Abstractions` project surface; `internal` by default; NuGet refs.
4. **Composition/wiring point is recognizable in all three**: Rust `main.rs`/`server` crate; Go `cmd/*`; C# host `Program.cs`. Together with DI registration extension in .NET.
5. **Framework/architecture conventions** ship in no preset: there is no preset/profile catalog and no recognition engine (see archspec design §8, ADR-013). The spec format is uniform across shapes; conventions live only as user-declared stereotypes in the spec, with illustrative starter specs (no runtime meaning).
