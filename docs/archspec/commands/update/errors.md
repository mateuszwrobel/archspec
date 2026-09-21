# `update` — Failure Contract (Errors)

Every failure produces a clear human message naming the cause and a **non-zero exit**. Messages go to stderr; no spec file is written. A pre-existing `architecture.spec.toml` is left byte-identical to its prior state.

A parse failure **aborts the run** — the spec is not produced for a partially parsed tree. A silent gap in the snapshot would seed a spec that celebrates broken input.

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| path does not exist | the invalid path | non-zero |
| path is a regular file | the path is invalid and why | non-zero |
| no supported-language sources found (recursively) | that no sources were found, and where | non-zero |
| a source file fails to parse | the name of the failing file | non-zero |
| `architecture.spec.toml` already exists and no `--force` | the spec file path and that `--force` is required to overwrite | non-zero |
| language driver disabled via the `ARCHSPEC_DISABLE_DRIVERS` seam | the missing driver/toolchain and a suggestion to run `archspec doctor` | non-zero |
| more than one positional argument | that only one path is accepted | non-zero |

## Examples

```bash
# message identifies the missing path
$ archspec update /no/such/dir
error: path does not exist: /no/such/dir

# message identifies the file and why it is rejected
$ archspec update Cargo.toml
error: path is not a directory: Cargo.toml

# message says no sources and where
$ archspec update ./docs
error: no supported-language sources found under: ./docs

# message names the failing file, no partial spec
$ archspec update ./bad-crate
error: failed to parse source: ./bad-crate/src/broken.rs

# message names the existing spec and the required flag
$ archspec update
error: spec already exists: ./architecture.spec.toml (use --force to overwrite)

# drivers ship in the binary; the seam is ARCHSPEC_DISABLE_DRIVERS=go
# message names the missing driver/toolchain and points at doctor
$ archspec update ./go-service
error: go toolchain not found (run 'archspec doctor' to diagnose drivers)

# message says only one path is accepted
$ archspec update ./src ./tests
error: expected at most one path argument
```

## Output on error

- stderr carries the message; stdout carries nothing.
- The pre-existing `architecture.spec.toml`, if any, is left byte-identical to its prior state.
- Exit code is non-zero (distinct failure codes are not required in phase 1).
