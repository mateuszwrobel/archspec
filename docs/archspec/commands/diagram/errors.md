# `diagram` — Failure Contract (Errors)

Every failure produces a clear human message naming the cause and a **non-zero exit**. Messages go to stderr; no partial diagram is written.

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| project path does not exist | the invalid path | non-zero |
| project path is a regular file | the path is invalid and why | non-zero |
| no `architecture.spec.toml` in the project directory | that no spec was found, and where | non-zero |
| the spec file fails to parse | the name of the failing file | non-zero |
| scan artefact does not exist | the missing artefact path | non-zero |
| scan artefact is not a valid model snapshot | the artefact path and that it is invalid | non-zero |
| model contains nothing to render | that there is no model content | non-zero |
| unsupported `--format` value | the value and the supported formats | non-zero |
| unsupported `--source` value | the value and the supported sources | non-zero |
| `--source scan` without an artefact path | that an artefact path is required | non-zero |
| more than one positional argument | that only one path is accepted | non-zero |
| `--check` and the destination differs from the generated diagram | that it is stale, the file, and how to regenerate | non-zero |
| `--check` and the destination is missing | that it is missing, the file, and how to generate it | non-zero |
| `--check` with no destination | that `--check` needs an output destination | non-zero |

## Examples

```bash
# message identifies the missing path
$ archspec diagram /no/such/dir
error: path does not exist: /no/such/dir

# message identifies the file and why it is rejected
$ archspec diagram Cargo.toml
error: path is not a directory: Cargo.toml

# message says no spec and where
$ archspec diagram ./docs
error: no spec found under: ./docs

# message names the failing spec file plus the parse detail, no partial diagram
$ archspec diagram ./bad-spec
error: failed to parse spec: ./bad-spec/architecture.spec.toml: TOML parse error at line 2, column 1

# message identifies the missing artefact
$ archspec diagram --source scan ./artifacts/missing.json
error: scan artefact not found: ./artifacts/missing.json

# message names the invalid artefact plus the parse detail
$ archspec diagram --source scan README.md
error: invalid scan artefact: README.md (expected value at line 1 column 1)

# message says nothing to render
$ archspec diagram --source scan ./artifacts/empty.json
error: no model content to render

# message names the unsupported value and the supported set
$ archspec diagram --format bmp .
error: unsupported format: bmp (supported: mermaid, plantuml)

# message names the unsupported source and the supported set
$ archspec diagram --source gif .
error: unsupported source: gif (supported: spec, scan)

# message says the artefact path is missing
$ archspec diagram --source scan
error: --source scan requires an artefact path

# message says only one path is accepted
$ archspec diagram ./src ./tests
error: expected at most one path argument

# --check reports a stale destination, nothing is written
$ archspec diagram --output docs/architecture.mmd --check
error: out of date diagram: docs/architecture.mmd differs from generated output (regenerate with: archspec diagram)

# --check reports a missing destination
$ archspec diagram --output docs/architecture.mmd --check
error: out of date diagram: docs/architecture.mmd is missing (generate with: archspec diagram)

# --check with no destination configured or passed
$ archspec diagram --check
error: --check requires an output destination (no default configured for diagram; use --output <path>)
```

## Output on error

- stderr carries the message; stdout carries nothing.
- Exit code is non-zero (distinct failure codes are not required in phase 1).
