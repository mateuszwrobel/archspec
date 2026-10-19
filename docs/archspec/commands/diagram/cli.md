# `diagram` — Command Surface (CLI Contract)

## Invocation

```
archspec diagram [path] [--source <spec|scan> <scan-path>] [--format <mermaid|plantuml>] [--output <path>] [--tiers <units,modules,files>] [--module <claim> | --modules-dir <dir>] [--check]
```

`path` is optional and positional. Every flag is optional; `scan-path` is required only when `--source scan` is given.

| Part | Meaning | Default |
|---|---|---|
| `path` | project directory | current working directory |
| `--source` | model source: `spec` or `scan` | `spec` |
| `--source scan <path>` | scan snapshot artefact to render | — (required with `--source scan`) |
| `--format` | diagram format | `mermaid` |
| `--output <path>` | write the diagram to that file | stdout unless `[output]` set |
| `--tiers <list>` | additive comma-separated tiers to render, from `units`, `modules` (scan source) and `files` (spec source with `--module`/`--modules-dir`) | `units` |
| `--module <claim>` | spec source only: render that one module claim's territory (members, edges, contract marks) | — |
| `--modules-dir <dir>` | spec source only: write one stable claim-named diagram per module claim into `<dir>` | — |
| `--check` | compare the diagram to its destination without writing | off |

## Accepted flag values

- `--source`: `spec`, `scan`. Any other value is an error (see `errors.md`).
- `--format`: `mermaid`, `plantuml`. Any other value is an error (see `errors.md`).
- `--output`: any writable file path.
- `--source scan` takes exactly one following argument: the path to the scan artefact. If the next token is another flag, or nothing follows, it is an error (see `errors.md`).
- `--tiers`: a non-empty comma-separated list of tier names from `units`, `modules`, `files` — additive. `modules` renders on top of the unchanged unit render, so `--tiers modules` equals `--tiers units,modules`; `units` and `modules` keep the shipped scan-source law (the flag with either value on `--source spec` is a usage error, exit `64`). `files` is the file-granular tier and is accepted on `--source spec` only, always under `--module` or `--modules-dir` (the size law: no whole-tree file render); `files` with `--source scan` or without a territory flag is a usage error (exit `64`) naming the rule broken. An invalid name, a duplicate entry, or an empty list is a usage error (exit `64`) naming the offending value and the supported tiers (see `errors.md`). The default `units` renders the bytes `--tiers` never changed.
- `--module <claim>`: the exact name of one module claim of the governing spec, at any depth. The render shows the claim's territory — its members, the edges that touch it, and its contract marks — with the same adjudication `verify` uses, so a dashed crossing in the render is exactly a red crossing in the report. A name no claim of the spec carries is a usage error (exit `64`, see `errors.md`). The flag is accepted only with `--source spec`; combined with `--modules-dir` it is a usage error (one claim or the batch, never both).
- `--modules-dir <dir>`: a directory to hold one diagram file per declared module claim. File names are computed from claim names by an escaping that is total and collision-free (the render-charset characters stay verbatim, every other byte percent-escapes, and dots that could fake a path escape too), so no claim name — including `a::b` or a name containing `..` — can escape the directory, and `.` and `..` are unrepresentable by construction. Every batch write is atomic per file (temp-plus-rename in the destination): a failed run leaves every file at previous-or-complete bytes and no temp residue. `--check` compares bytes without writing and flags stale, missing, and orphan files — entries with the render extension whose decoded name no claim declares; files outside the render extension belong to nobody but the user and are never orphans (orphans are flagged, never deleted). Accepted only with `--source spec`; mutually exclusive with `--module`.

## Usage examples

### Default run

```bash
archspec diagram
```

Render the declared spec (`architecture.spec.toml`) from the current directory as a Mermaid diagram to stdout.

### Explicit project directory

```bash
archspec diagram ./crates/auth
```

Render the declared spec from `./crates/auth`.

### Render a scan snapshot

```bash
archspec diagram --source scan ./artifacts/model.json
```

Render the extracted current-state model snapshot as a Mermaid diagram to stdout.

### Module tier on a scan snapshot (opt-in)

```bash
archspec diagram --source scan ./artifacts/model.json --tiers modules
```

Render the unchanged unit-tier diagram plus a `modules` cluster carrying the snapshot's module edges — module nodes render under the claim names that govern them, dual-labelled with their module paths (see `output.md`). Without `--tiers`, module edges draw nothing.

### File tier for a claim (opt-in)

```bash
archspec diagram --module ui --tiers files
```

Render the claim's unchanged module view plus a nested `files` cluster: one vertex per file of the claim's territory the model shows (repo-relative path with extension), one arrow per first-party file edge touching it. A file arrow that `verify` reddens as part of a cycle renders dashed `violates spec` — the marks read the shared adjudication seam, never a diagram-side re-derivation — and asset files carry incoming arrows only and never join a cycle. On a language whose driver emits no file-grained facts the render states the vacuity and draws no file vertices.

### Mermaid to a file

```bash
archspec diagram --output docs/architecture.mmd
```

Write a Mermaid diagram to `docs/architecture.mmd`. Stdout stays empty.

### PlantUML to a file

```bash
archspec diagram --format plantuml --output docs/architecture.puml
```

Write a PlantUML diagram to `docs/architecture.puml`. Stdout stays empty.

### Combine anywhere

Flags and path can be given in any order:

```bash
archspec diagram --output out.mmd --format mermaid --source scan ./artifacts/model.json
```

### Freshness check (CI gate)

```bash
archspec diagram --output docs/architecture.mmd --check
```

Render the diagram in memory and compare it byte-for-byte with `docs/architecture.mmd`. Nothing is written; a fresh file exits `0`, a stale or missing file exits non-zero with the regeneration command.

## Behavior notes

- Exactly one positional argument. Extra positional arguments are an error.
- `--source spec` is accepted explicitly and is identical to the default.
- `--source scan` without an artefact path is an error.
- A valid run exits `0`. Any error condition exits non-zero with a message on stderr (see `errors.md`).
- `diagram` is single-shot: it loads the model, renders, and exits. No daemon, no persistent state.
- Destination: `--output` wins, then `[output] diagram` in `archspec.toml` (relative to project root), then stdout (`../../config.md`).
- `--check` compares the rendered diagram to the destination without writing it: identical bytes exit `0`; a differing or missing destination exits non-zero; with no destination at all it is an error (`errors.md`). With `--tiers`, the compared bytes are the ones the selected tiers render produces.

## Help

`archspec diagram --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
