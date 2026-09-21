# `depgraph` — Failure Contract (Errors)

Every failure produces a clear human message naming the cause and exit code `1`. Messages go to stderr; stdout carries nothing and no partial body is written.

## Error conditions

| Condition | Message |
|---|---|
| unknown flag | `unknown flag: --<name>` |
| flag given without its value | `flag --<name> requires a value` |
| no view given | usage line naming the views |
| unknown view | `unknown depgraph view: <x> (supported: modules, api-usage, submodules)` |
| path does not exist | `path does not exist: <path>` |
| path is a regular file | `path is not a directory: <path>` |
| no supported-language sources | `no supported-language sources found under: <path>` |
| driver for detected language missing | `detected language '<lang>' but the <lang> driver is not available (run 'archspec doctor')` |
| model has no module tier to render (the go driver emits none) | `depgraph needs the module tier, which this model has none of; a Go tree acquires one through go.work members (2+)` |
| unsupported `--format` (graph views) | `unsupported format: <x> (supported: mermaid, plantuml)` |
| `--format` on `api-usage` other than markdown | `unsupported format for api-usage: <x> (supported: markdown)` |
| `submodules` without `--parent` | `submodules view requires --parent <module>` |
| `--parent` with a non-submodules view | `--parent is only valid for the submodules view` |
| `--parent` names no module in the model | `parent module not found in model: <x> (known top-level modules: <names>)` |
| more than one positional path | `expected at most one path argument` |
| `--check` and the destination differs from the generated body | `out of date dependency graph: <path> differs from generated output (regenerate with: archspec depgraph)` |
| `--check` and the destination is missing | `out of date dependency graph: <path> is missing (generate with: archspec depgraph)` |
| `--check` without `--output` (no `[output]` default exists) | `--check requires an output destination (no default configured for dependency graph; use --output <path>)` |

## Examples

```bash
$ archspec depgraph modules . --x
error: unknown flag: --x

$ archspec depgraph modules --format
error: flag --format requires a value

$ archspec depgraph bogus .
error: unknown depgraph view: bogus (supported: modules, api-usage, submodules)

$ archspec depgraph submodules .
error: submodules view requires --parent <module>

$ archspec depgraph modules --format bmp .
error: unsupported format: bmp (supported: mermaid, plantuml)

$ archspec depgraph api-usage --format mermaid .
error: unsupported format for api-usage: mermaid (supported: markdown)

$ archspec depgraph modules --parent core .
error: --parent is only valid for the submodules view

$ archspec depgraph submodules --parent ghost .
error: parent module not found in model: ghost (known top-level modules: app::auth, app::billing)

$ archspec depgraph modules ./go-tree
error: depgraph needs the module tier, which this model has none of; a Go tree acquires one through go.work members (2+)

$ archspec depgraph api-usage ./go-tree
error: depgraph needs the module tier, which this model has none of; a Go tree acquires one through go.work members (2+)

$ archspec depgraph submodules --parent config ./go-tree
error: depgraph needs the module tier, which this model has none of; a Go tree acquires one through go.work members (2+)

$ archspec depgraph modules --output docs/modules.mmd --check
error: out of date dependency graph: docs/modules.mmd differs from generated output (regenerate with: archspec depgraph)

$ archspec depgraph modules --check
error: --check requires an output destination (no default configured for dependency graph; use --output <path>)
```

> The module-tier guard runs before any view renders, so a Go tree gets this
> capability sentence on **every** view (including `submodules`, where it
> replaces the per-parent `parent module not found` complaint). The guard keys on
> the model's module tier, not the detected language: a rust/csharp model always
> carries the tier, and a Go tree whose scanned model carries it — a multi-member
> `go.work` workspace — clears the guard and renders with no change here. A
> single-`go.mod` tree stays refused, because `depgraph` scans only and the
> spec-declared derivation exists solely inside the compare (`verify`/`update`).

## Output on error

- stderr carries the message; stdout carries nothing.
- Exit code is `1`.
