# archspec `spec` — Design

Print the machine-readable contract for `architecture.spec.toml` — either a JSON Schema (draft-07) usable by editors, CI validators, and coding agents, or an annotated reference of every section. Part of the archspec design (see `../../../archspec-design.md`).

The command never reads a project tree. It is a pure output command: `archspec spec --schema` prints the schema, `archspec spec` prints the annotated reference, both to stdout.

## Scope

- `--schema` — JSON Schema (draft-07) for `architecture.spec.toml`: `[project]`, `[[module]]` (with recursive `submodules`), `[[constraint]]` (all 7 types), `[[stereotype]]`.
- Reference — annotated example TOML covering every section, shared with the command's `--help` text.
- The schema's per-type `required` arrays come from the same table that drives the runtime validator (`CONSTRAINT_REQUIRED_FIELDS` in `src/archspec/spec.rs`), so the two cannot drift.

## Quick usage

```bash
# JSON Schema for architecture.spec.toml
archspec spec --schema

# annotated reference TOML
archspec spec
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: what the command accepts and rejects |
| `output.md` | Output contract: schema and reference output, determinism |
| `errors.md` | Failure contract: invalid invocations, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |