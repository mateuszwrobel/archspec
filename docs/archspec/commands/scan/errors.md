# `scan` — Failure Contract (Errors)

Every failure produces a clear human message naming the cause and a **non-zero exit**. Messages go to stderr; no partial model is written (with `--output <path>`, the file is not created or written on failure).

A parse failure **aborts the run** — the model is not produced from a partially parsed tree. Silent gaps in the model would corrupt every downstream consumer (`update`, `diagram`), so a hard fail is intentional and honest.

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| path does not exist | the invalid path | non-zero |
| path is a regular file | the path is invalid and why | non-zero |
| no supported-language sources found (recursively) | that no Rust/C#/Go/ts/python sources were found, and where | non-zero |
| the spec pins a language whose root marker is absent while other root markers stand (`language mismatch: <declared> — no root marker <marker> for the declared language, found: <markers>`) | the declared language, the marker file it expects, and the marker files actually found | non-zero |
| more than one root marker at the scan root with no spec pinning a language (`ambiguous root markers: <path> (<language>), … — pin [project] language or scan a single-language root`) | that the root is polyglot, every marker file named, and the pin or per-root remedy | non-zero |
| the spec's declared language disagrees with the counted tree (`language mismatch`, the D14 agreement check — only when a spec sits at the scanned root) | the declared and found languages with the counts of each, and the `excluded_sources` remedy | non-zero |
| a foreign root-marker file sits below the scan root (`nested foreign marker: <path> (<language>) — not modelled`) | that the tree below that marker is not modelled — report-only, one line per marker file, exit 0 | 0 |
| a scanned ts source file carries no model fact (`orphan file: <path>`, closed by one `orphan file census: <n> of <m> scanned files — the model lists no fact for any of them` summary, never at zero) | that the import-graph fact sets list no fact for the file (the census names its blind spots in `help diagnostics`) — report-only, one line per file, exit 0 | 0 |
| a ts specifier resolves to no file (`unresolvable import`) — or only into an excluded tree, where the line names the exclusion | the importing file, the specifier verbatim, and the exclusion remedy | non-zero |
| a source file fails to parse | the name of the failing file | non-zero |
| language detected but no driver/toolchain present | the detected language, that its toolchain is missing, and to run `archspec doctor` | non-zero |
| more than one positional argument | that only one path is accepted | non-zero |
| `--check` and the destination differs from the generated model | that it is stale, the file, and how to regenerate | non-zero |
| `--check` and the destination is missing | that it is missing, the file, and how to generate it | non-zero |
| `--check` with no destination | that `--check` needs an output destination | non-zero |

## Examples

```bash
# message identifies the missing path
$ archspec scan /no/such/dir
error: path does not exist: /no/such/dir

# message identifies the file and why it is rejected
$ archspec scan Cargo.toml
error: path is not a directory: Cargo.toml

# message says no supported-language sources and where
$ archspec scan ./docs
error: no supported-language sources found under: ./docs

# message names the failing file, no partial model
$ archspec scan ./bad-crate
error: failed to parse source: ./bad-crate/src/broken.rs

# message names the detected language and points at doctor
$ archspec scan ./golang-service
error: detected language 'go' but the go driver is not available (run 'archspec doctor')

# message says only one path is accepted
$ archspec scan ./src ./tests
error: expected at most one path argument

# --check reports a stale destination, nothing is written
$ archspec scan --output docs/model.json --check
error: out of date model: docs/model.json differs from generated output (regenerate with: archspec scan)
```

## Output on error

- stderr carries the message; stdout carries nothing. With `--output <path>`, the file is not created or written.
- Exit code is non-zero (distinct failure codes are not required).
