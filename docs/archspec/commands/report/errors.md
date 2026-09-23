# `report` — Failure Contract (Errors)

Every failure produces a clear human message naming the cause and a **non-zero exit**. Messages go to stderr; no partial report is written. When `--output` is given and the run fails, the target file is not created (an existing file is left untouched).

Operational failures are distinct from rule violations: an error-level violation is report content and exits non-zero after the report is emitted; a failure is a broken run that exits non-zero without a report.

A parse failure **aborts the run** — the report is not produced for a partially parsed tree. A silent gap would hide exactly what the report exists to expose.

An explicit `--format` that differs from the configured destination's text
format is **not** an error condition: the report goes to stdout and the
configured path is left untouched (`output.md`). No failure is reported and the
exit code follows the usual rules (violations exit non-zero after the report is
emitted).

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| path does not exist | the invalid path | non-zero |
| path is a regular file | the path is invalid and why | non-zero |
| no `architecture.spec.toml` | the expected file and the directory searched | non-zero |
| invalid spec | the spec file and the reason | non-zero |
| no sources in the spec's declared language | that no sources were found, the language, and where | non-zero |
| a source file fails to parse | the name of the failing file | non-zero |
| declared language's driver/toolchain missing | the language and that `archspec doctor` can diagnose | non-zero |
| unsupported `--format` value | the value and the supported formats | non-zero |
| more than one positional argument | that only one path is accepted | non-zero |
| `--check` and the destination differs from the generated report | that it is stale, the file, and how to regenerate | non-zero |
| `--check` and the destination is missing | that it is missing, the file, and how to generate it | non-zero |
| `--check` with no destination | that `--check` needs an output destination | non-zero |

## Examples

```bash
# message identifies the missing path
$ archspec report /no/such/dir
error: path does not exist: /no/such/dir

# message identifies the file and why it is rejected
$ archspec report Cargo.toml
error: path is not a directory: Cargo.toml

# message names the expected spec file and where it was looked for
$ archspec report ./docs
error: spec file not found: ./docs/architecture.spec.toml

# message names the spec file and the reason it is invalid
$ archspec report ./bad-spec
error: invalid spec: ./bad-spec/architecture.spec.toml (malformed TOML)

# message says no sources, the language, and where
$ archspec report ./rustless
error: no rust sources found under: ./rustless

# message names the failing file, no partial report
$ archspec report ./bad-crate
error: failed to parse source: ./bad-crate/src/broken.rs

# message names the language and points to doctor
$ archspec report ./net
error: csharp driver unavailable (toolchain not found); run 'archspec doctor' to diagnose

# message names the unsupported value and the supported set
$ archspec report --format html .
error: unsupported format: html (supported: text, markdown, json)

# message says only one path is accepted
$ archspec report ./src ./tests
error: expected at most one path argument

# --check reports a stale destination, nothing is written
$ archspec report --output docs/architecture-report.md --check
error: out of date report: docs/architecture-report.md differs from generated output (compared the text rendering; regenerate with: archspec report; pass the same --format to --check to verify a --format-generated artefact)

# --check on a markdown artefact names the compared rendering and hints its exact form
$ archspec report --output docs/architecture-report.md --format markdown --check
error: out of date report: docs/architecture-report.md differs from generated output (compared the markdown rendering; regenerate with: archspec report --format markdown)

# --check reports a missing destination
$ archspec report --output docs/architecture-report.md --check
error: out of date report: docs/architecture-report.md is missing (generate with: archspec report)
```

## Output on error

- stderr carries the message; stdout carries nothing.
- When `--output` is given, the target file is not created on failure; an existing file at that path is left unchanged.
- Exit code is non-zero (distinct failure codes are not required).
