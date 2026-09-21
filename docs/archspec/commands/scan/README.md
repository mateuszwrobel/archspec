# archspec `scan` — Design

Extracts the architecture model only — units, hard edges, usage, and soft structure — from a source tree and its manifests, via the matching language driver. Part of the archspec design (see `../../../archspec-design.md`).

The tool detects the project language (Rust, C#, Go), runs the matching language driver, and emits the canonical model — the shared IR that the spec pipeline compares against. Its purpose is **discovery at the model level**: the raw ground truth a developer reads before writing a spec, and the artefact that `update` snapshots into a seed spec and `diagram --source scan` renders before any spec exists. It extracts the model and nothing else.

## Scope

- Language detected from the tree — Rust, C#, or Go — and the matching language driver extracts the model.
- Model content only: units, hard edges, usage, and soft structure. No spec needed, no verification, no diagram rendering.
- Output is the canonical model as deterministic JSON on stdout (or an `--output` file), byte-stable for the same input, consumable by `update` and `diagram --source scan`.
- One project tree per run; mixed-language trees are out of scope.

## Quick usage

```bash
# from a project root — uses the current directory
archspec scan

# explicit path
archspec scan ./crates/auth

# write the model to a file instead of stdout
archspec scan ./crates/auth --output docs/model.json
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: path semantics, language detection, accepted and rejected inputs |
| `output.md` | Output contract: destination, canonical model IR, determinism |
| `errors.md` | Failure contract: error conditions, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |
