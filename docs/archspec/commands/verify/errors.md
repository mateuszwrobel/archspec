# `verify` — Failure Contract (Errors)

Two distinct kinds of failure:

1. **Operational errors** — the run cannot happen: missing path, missing spec, invalid spec, no sources, parse failure, missing driver. A clear human message naming the cause goes to stderr; stdout carries nothing; exit is non-zero.
2. **Rule violations** — the run completes but the model differs from the spec. The full diff report goes to stdout and exit is non-zero. This is not an error *message*; it is the product (see `output.md`).

A parse failure **aborts the run** — no diff is produced for a partially extracted tree. Silent gaps would fake a verdict, so a hard fail is intentional and honest.

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| path does not exist | the invalid path | non-zero |
| path is a regular file | the path is invalid and why | non-zero |
| no `architecture.spec.toml` found | the expected file name and where it was expected | non-zero |
| spec is not valid TOML | the file and the TOML parse error | non-zero |
| spec violates the schema | the file, the offending field/section, and why | non-zero |
| module declares an empty `contract.expose` entry | the file, the module name, and the empty entry position | non-zero |
| constraint references an undeclared module | the file, the 1-indexed constraint number, and the undeclared module name | non-zero |
| no supported-language sources found (recursively) | that no sources were found, and where | non-zero |
| the declared language has no root marker where other markers stand (`language mismatch: <declared> — no root marker <marker> for the declared language, found: <markers>`) | the declared language, the marker file it expects, and the marker files actually found | non-zero |
| the declared language disagrees with the counted tree (`language mismatch`, the D14 agreement check) | the declared and found languages with the counts of each, and the `excluded_sources` remedy | non-zero |
| a ts specifier resolves to no file (`unresolvable import`) — or only into an excluded tree, where the line names the exclusion | the importing file, the specifier verbatim, and the exclusion remedy | non-zero |
| a source file fails to parse | the name of the failing file | non-zero |
| language driver/toolchain missing | the language, and the suggestion to run `archspec doctor` | non-zero |
| unknown flag | the flag name | non-zero |
| more than one positional argument | that only one path is accepted | non-zero |
| a scanned ts source file carries no model fact (`orphan file: <path>`, closed by one `orphan file census: <n> of <m> scanned files — the model lists no fact for any of them` summary, never at zero) | that the import-graph fact sets list no fact for the file — report-only stderr on the shipped report surface, printed under green AND red verdicts alike, never itself a verdict | 0 (green) / non-zero only from real findings |

## Examples

```bash
# message identifies the missing path
$ archspec verify /no/such/dir
error: path does not exist: /no/such/dir

# message identifies the file and why it is rejected
$ archspec verify Cargo.toml
error: path is not a directory: Cargo.toml

# message names the missing spec and where it was expected
$ archspec verify ./docs
error: no architecture.spec.toml found in: ./docs

# message names the file and the TOML parse error
$ archspec verify ./bad-spec
error: invalid spec: ./architecture.spec.toml: expected `=` after key at line 3

# message names the file, the offending field, and why
$ archspec verify ./bad-schema
error: invalid spec: architecture.spec.toml: unknown constraint type "no_loops" in [constraint] #2

# message names the module and the empty expose entry
$ archspec verify ./empty-expose
error: invalid spec: architecture.spec.toml: module "auth" has contract.expose entry #1: empty entries are not allowed

# message names the constraint number and the undeclared module
$ archspec verify ./undeclared-ref
error: invalid spec: architecture.spec.toml: constraint #1 references undeclared module "ghost"

# message says no sources and where
$ archspec verify ./empty-project
error: no supported-language sources found under: ./empty-project

# message names the failing file, no partial diff
$ archspec verify ./bad-crate
error: failed to parse source: ./src/broken.rs

# message names the language and points to doctor
$ archspec verify ./go-module
error: no Go toolchain found (driver missing); run `archspec doctor` to diagnose

# message names the unknown flag
$ archspec verify --format mermaid
error: unknown flag: --format

# message says only one path is accepted
$ archspec verify ./src ./tests
error: expected at most one path argument
```

## Output on error

- stderr carries the message; stdout carries nothing.
- Exit code is non-zero. Rule violations also exit non-zero but print the full diff report to stdout first.
