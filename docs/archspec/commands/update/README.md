# archspec `update` — Design

Seed-spec snapshot of the current model for existing-code onboarding. Part of the archspec design (see `../../../archspec-design.md`).

The command extracts the architecture model from a source tree — the same model `scan` extracts — and writes it as `architecture.spec.toml` in the declared spec format (see `../../spec.md`). The result captures what the code does **today**, as the starting point of the "existing code → spec → guard" flow. It is a seed, not a verdict: the generated spec is meant to be reviewed, trimmed, and strengthened into the desired architecture before `verify` enforces it.

`update` is the primary on-ramp for real code. Unlike `init` (which writes a minimal `[project] language` header plus the global `no_cycles` guard, and an `archspec.toml` output config), `update` seeds boundaries from the actual tree — units and/or internal modules — so the user starts from what exists and edits toward what they want. It is shape-agnostic: the same command seeds a single-crate, multi-crate, or hybrid tree without any selection.

## Scope

- One project directory at a time (default: the current directory).
- Model extraction per the detected language, by that language's driver — every driver ships inside the binary, so no external toolchain on `PATH` is required.
- Writes exactly one file: `architecture.spec.toml`, in the scanned project directory.
- No merging, no editing of an existing spec: a reviewed or edited spec is not silently clobbered (see `cli.md`, `errors.md`).
- Deterministic: the same source tree always produces byte-identical `architecture.spec.toml`.

## Quick usage

```bash
# from a project root — uses the current directory
archspec update

# explicit path
archspec update ./crates/auth

# overwrite an existing spec (discards any edits to it)
archspec update --force
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: path semantics, accepted and rejected inputs |
| `output.md` | Output contract: destination, structure, determinism |
| `errors.md` | Failure contract: error conditions, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |
