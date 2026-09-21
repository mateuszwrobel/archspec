# archspec `inspect` — Design (Phase 1)

Zero-configuration file-level import map for source trees. Part of the archspec design (see `../../../archspec-design.md`).

The tool scans a Rust, C# or Go tree, resolves internal imports between files, and renders a diagram grouped by the real folder structure. Its purpose is **discovery**: to see the current relations between files — complexity, existing boundaries, and files that import from everywhere. It renders relations and nothing else.

## Scope

- Rust, C# and Go file-level import maps (one tree's source root at a time). A C# file scanner mirrors the Rust one; `using` resolves to the file declaring the namespace. The Go scanner resolves `import` paths that fall under the tree's own module path(s) — the same resolution the `scan` driver uses (`go.mod` module, `go.work` members) — to the production files declaring the imported package, so cross-package and cross-member imports render as edges; stdlib, third-party and cgo (`import \"C\"`) targets render nothing. Go test files (`*_test.go`, both package forms) are nodes with their source-side edges — discovery includes what the guard model drops, while no production import ever targets them. The structural modes branch on model content, not language: `scanner` renders whatever units, edges and soft structure the model carries; `tree` renders containment where a module tier exists (`go.work` workspaces among them) and is refused where it does not (single-module Go — the tree view projects only the module tier; `scan`/`verify`/`report`/`update`/`diagram` carry Go's guard surfaces).
- Zero configuration: no spec file, no rules. `Cargo.toml` is optionally read only to resolve own-crate-name imports; nothing is required.
- Pure discovery: no ranking, no metrics, no boundary suggestions. A human or another tool draws conclusions from the graph.
- External-crate imports are not drawn; a workspace-wide merge across many roots is not this command's job.

## Quick usage

```bash
# from a crate root (or src/ parent) — uses the current directory
archspec inspect

# explicit path
archspec inspect ./crates/auth

# write the diagram to a file instead of stdout
archspec inspect ./crates/auth --output docs/imports.md

# PlantUML instead of Mermaid
archspec inspect --format plantuml --output docs/imports.puml

# structural model views (units + modules; `scanner` adds cross-unit edges)
archspec inspect tree ./crates/auth
archspec inspect scanner ./crates/auth

# CI freshness gate: fail if the committed diagram is stale
archspec inspect --output docs/imports.md --check
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: path semantics, accepted and rejected inputs |
| `output.md` | Output contract: destination, formats, diagram structure, determinism |
| `errors.md` | Failure contract: error conditions, messages, exit behavior |
| `resolution.md` | Import resolution and folder grouping — what the diagram means |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |
