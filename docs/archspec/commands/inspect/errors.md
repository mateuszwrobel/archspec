# `inspect` — Failure Contract (Errors)

Every failure produces a clear human message naming the cause and a **non-zero exit**. Messages go to stderr; no partial diagram is written.

A parse failure **aborts the run** — the diagram is not produced for a partially parsed tree. Silent gaps in the map would hide tangles, so a hard fail is intentional and honest.

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| path does not exist | the invalid path | non-zero |
| path is a regular file | the path, the inspect modes that exist (file-level import maps for rust/csharp/go and structural `tree`/`scanner` model views) and the directory form to pass instead | non-zero |
| no supported-language sources found (recursively) | that no supported-language sources were found, and where | non-zero |
| `inspect tree` on a model without a module tier (single-`go.mod` Go trees) | that the tree view needs the module tier, that this model has none, and how a Go tree acquires one (go.work members) — a model-fact refusal, not a language verdict; `scanner` renders such a model | non-zero |
| detected language whose `scan` driver is unavailable (`tree`/`scanner` modes, or an env-disabled driver) | the detected language and a pointer to run `archspec doctor` | non-zero |
| a source file fails to parse | the name of the failing file | non-zero |
| unsupported `--format` value | the value and the supported formats | non-zero |
| more than one positional argument | that only one path is accepted | non-zero |
| `--check` and the destination differs from the generated diagram | that it is stale, the file, and how to regenerate | non-zero |
| `--check` and the destination is missing | that it is missing, the file, and how to generate it | non-zero |
| `--check` with no destination | that `--check` needs an output destination | non-zero |

## Examples

```bash
# message identifies the missing path
$ archspec inspect /no/such/dir
error: path does not exist: /no/such/dir

# message identifies the file, names the directory-taking modes, and shows the directory form
$ archspec inspect Cargo.toml
error: path is not a directory: Cargo.toml; every inspect mode takes a directory — file-level import map modes (rust, csharp, go; default mermaid/plantuml output) and structural modes 'inspect tree|scanner' (model views; guard surfaces live in scan/verify/report/update/diagram); pass a directory, e.g. archspec inspect .

# message says no sources and where
$ archspec inspect ./docs
error: no supported-language sources found under: ./docs

# every source sits under an excluded dir: no file-level sources
$ archspec inspect ./only-target-left
error: no Rust sources found under: ./only-target-left

# the tree view projects the module tier; a single-module Go model has none
$ archspec inspect tree ./go-service
error: inspect tree needs the module tier, which this model has none of; a Go tree acquires one through go.work members (2+), 'inspect scanner' renders the unit-tier model

# driver for the detected language is unavailable (tree/scanner modes)
$ archspec inspect tree ./service
error: detected language 'go' but the go driver is not available (run 'archspec doctor')

# message names the failing file, no partial diagram
$ archspec inspect ./bad-crate
error: failed to parse source: ./bad-crate/src/broken.rs

# message names the unsupported value and the supported set
$ archspec inspect --format bmp .
error: unsupported format: bmp (supported: mermaid, plantuml)

# message says only one path is accepted
$ archspec inspect ./src ./tests
error: expected at most one path argument

# --check reports a stale destination, nothing is written
$ archspec inspect --output docs/imports.md --check
error: out of date diagram: docs/imports.md differs from generated output (regenerate with: archspec inspect)
```

## Output on error

- stderr carries the message; stdout carries nothing.
- Exit code is non-zero (distinct failure codes are not required in phase 1).
