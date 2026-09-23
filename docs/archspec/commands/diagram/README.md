# archspec `diagram` — Design (Phase 1)

Renders the architecture model as a diagram artefact. Part of the archspec design (see `../../../archspec-design.md`).

The diagram is a **pure function of the model** — the code ↔ diagram relation is deterministic (design §2, ADR-007). `diagram` takes a model — either the declared spec (the governed architecture) or a scan snapshot (the extracted current-state model) — and renders it as a Mermaid or PlantUML diagram. Its purpose is **communication**: make the architecture visible and reviewable as an artefact derived from the model, never hand-maintained (ADR-008).

## Scope

- Two model sources: declared spec (`--source spec`, default) and scan snapshot artefact (`--source scan <path>`).
- Model only — `diagram` never reads the source tree; rendering is a pure function of the model.
- Output formats: **Mermaid + PlantUML only** (ADR-013), deterministic text, byte-stable canonical ordering.
- Violating/forbidden edges visibly marked (dashed/red); external crates in a separate cluster.
- Role marks in scan mode only: a node the model's `roles` map addresses carries that role's marker (`output.md`, *Diagram structure* item 5); spec mode reads no roles map, and a role keyed below the unit tier (a rust bin's `<unit>::main`) marks in `inspect tree`, not here.
- Single-shot: load model, render, exit. No daemon, no persistent state.

## Quick usage

```bash
# render the declared spec as Mermaid (governed architecture)
archspec diagram

# explicit project directory
archspec diagram ./crates/auth

# render a scan snapshot (extracted current-state model)
archspec diagram --source scan ./artifacts/model.json

# write the diagram to a file instead of stdout
archspec diagram --output docs/architecture.mmd

# PlantUML instead of Mermaid
archspec diagram --format plantuml --output docs/architecture.puml
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: model sources, path semantics, accepted and rejected inputs |
| `output.md` | Output contract: destination, formats, diagram structure, determinism |
| `errors.md` | Failure contract: error conditions, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |

## Relationship to `inspect`

`inspect` (`../inspect/README.md`) is the zero-config, file-level discovery variant: it maps the current relations between files with no spec and no rules. `diagram` renders the **governed model** — the declared architecture from the spec, or the extracted model from a scan snapshot. `inspect` shows the raw source structure; `diagram` shows the architecture as the model declares it. The declared model is `architecture.spec.toml` (see `../../spec.md`).
