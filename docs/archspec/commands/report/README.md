# archspec `report` — Design

Human-readable diff + metric output: the extracted architecture model compared against the declared spec, rendered as a readable report. Part of the archspec design (see `../../../archspec-design.md`).

`report` shares its extraction + comparison basis with `verify`: both extract the model from source and compare it structurally to the spec. The difference is the consumer. `verify` gates — it exits non-zero on any violation, for CI. `report` explains — it writes the full diff and the metrics so a human (or a CI script parsing `--format json`) can read why; error-level violations still exit non-zero, warnings never do.

## Scope

- Reads the spec (`architecture.spec.toml`, see `../../spec.md`) and the source tree it declares.
- Emits the full structural diff (ADR-010, see `../../../archspec-design.md` §9) plus model-derived metrics.
- Warning-severity findings (dead references, laundered forbidden edges, unresolved module files, unowned module-edge endpoints) and vacuous constraints are rendered as ordinary diff lines under their own category label; `report` never promotes or gates them.
- Output formats: text (default), markdown, and json. All deterministic and byte-stable. The JSON format serializes the same summary + diff (`findings` array is the machine contract; see `output.md`).
- Destination precedence: `--output` always wins; a configured `[output] report` destination receives only the default text artefact, so an explicit `--format markdown|json` prints to stdout instead of overwriting it (see `output.md`).
- Exit: `0` when the code satisfies its spec; non-zero on error-level rule violations (after the report is emitted) or operational failures. Warnings never gate.
- Diagram rendering, spec scaffolding, model snapshots, and driver diagnostics belong to `diagram`, `init`, `update`, and `doctor` respectively.

## Quick usage

```bash
# text diff + metrics to stdout, using the current directory
archspec report

# explicit path
archspec report ./crates/auth

# write a markdown report to a file
archspec report ./crates/auth --format markdown --output docs/architecture-report.md
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: path semantics, accepted and rejected inputs |
| `output.md` | Output contract: destination, formats, report structure, determinism |
| `errors.md` | Failure contract: error conditions, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |
