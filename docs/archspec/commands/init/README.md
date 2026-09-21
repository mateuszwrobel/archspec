# archspec `init` — Design (Phase 1)

Scaffolds a new architecture spec for a project. Part of the archspec design (see `../../../archspec-design.md`).

`init` writes a **minimal base spec** — `architecture.spec.toml` with a `[project] language` header plus a global `no_cycles` constraint (all declared modules, no `modules` list) — into a project directory. It is the on-ramp to the archspec workflow: without a spec there is nothing to verify. Its purpose is **adoption** — a place to start, filled in by the user or seeded from the real tree by `update` (see `../../commands/update/`). The spec format itself is shape-agnostic (see `../../spec.md`): `init` never picks a profile or asserts a project shape.

## Phase 1 scope

- Targets an existing project tree; the current working directory by default.
- Supports the three phase-1 languages: Rust, C#/.NET, Go.
- Writes a minimal base spec for the detected language: `[project] language` plus the global `no_cycles` guard, so new projects start cycle-clean. No profile catalog, no selection.
- Two output artefacts: `architecture.spec.toml` (the minimal base spec) and `archspec.toml` (the standard `[output]` defaults). Nothing else is created or modified.
- `init` refuses to overwrite an existing spec — there is no force flag. Clobbering a guarded spec is destructive.

## Quick usage

```bash
# from a project root — detects the language and writes a minimal base spec
archspec init

# explicit project directory
archspec init ./crates/my-app
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: path semantics, language detection, accepted and rejected inputs |
| `output.md` | Output contract: destination, spec structure, determinism |
| `errors.md` | Failure contract: error conditions, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |
