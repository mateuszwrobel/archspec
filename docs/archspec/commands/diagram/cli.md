# `diagram` — Command Surface (CLI Contract)

## Invocation

```
archspec diagram [path] [--source <spec|scan> <scan-path>] [--format <mermaid|plantuml>] [--output <path>] [--check]
```

`path` is optional and positional. Every flag is optional; `scan-path` is required only when `--source scan` is given.

| Part | Meaning | Default |
|---|---|---|
| `path` | project directory | current working directory |
| `--source` | model source: `spec` or `scan` | `spec` |
| `--source scan <path>` | scan snapshot artefact to render | — (required with `--source scan`) |
| `--format` | diagram format | `mermaid` |
| `--output <path>` | write the diagram to that file | stdout unless `[output]` set |
| `--check` | compare the diagram to its destination without writing | off |

## Accepted flag values

- `--source`: `spec`, `scan`. Any other value is an error (see `errors.md`).
- `--format`: `mermaid`, `plantuml`. Any other value is an error (see `errors.md`).
- `--output`: any writable file path.
- `--source scan` takes exactly one following argument: the path to the scan artefact. If the next token is another flag, or nothing follows, it is an error (see `errors.md`).

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
- `--check` compares the rendered diagram to the destination without writing it: identical bytes exit `0`; a differing or missing destination exits non-zero; with no destination at all it is an error (`errors.md`).

## Help

`archspec diagram --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
