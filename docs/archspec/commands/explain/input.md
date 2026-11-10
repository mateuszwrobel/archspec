# `explain` — Inputs

## Positionals

- `source`, `target` — each resolves in a fixed fact order: a declared module name first, then a model file path (claim-attributed), then the bare identity space. A spelling that matches none is stated as unresolved; explain guesses nothing.
- `path` — the project directory that carries `architecture.spec.toml` and the sources. Same loader as verify: the spec is found by the spec-file search (this directory, then ancestors), so an invocation from a subdirectory reads the same spec verify would.

## Flags

- `--constraint <pattern>` — a glob matched against each row's `type` and each of its declared `modules` names; a row enters the census when either matches. The filter limits only the constraints section — endpoints, model rows and findings are rendered regardless.

## Files read

- `architecture.spec.toml` — the governing spec: `[project]`, `[[module]]`, `[[constraint]]`. Loaded exactly as verify loads it, including `[[excluded]]` handling.
- the source tree under `path` — extracted exactly as verify extracts it (same driver, same exclusions).

## Preconditions (checked in this order)

1. `path` exists and is a directory — otherwise operational error.
2. the spec file is found — otherwise operational error, byte-identical to verify's message.
3. the declared language resolves and a driver is available — otherwise operational error.
