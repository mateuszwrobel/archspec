# `inspect` — Command Surface (CLI Contract)

## Invocation

```
archspec inspect [tree|scanner] [path] [--format <mermaid|plantuml>] [--output <path>] [--check]
```

`tree`/`scanner` is an optional mode keyword (see Modes); `path` is optional and positional. Every flag is optional.

| Part | Meaning | Default |
|---|---|---|
| `tree` / `scanner` | structural model mode (render the scan model, not the file tree) | file-level import graph |
| `path` | directory to scan | current working directory |
| `--format` | diagram format (default mode only) | `mermaid` |
| `--output <path>` | write the diagram to that file | stdout unless `[output]` set |
| `--check` | compare the diagram to its destination without writing | off |

## Modes

| Invocation | What it renders |
|---|---|
| `inspect [path]` | zero-config **file-level import graph** (nodes = source files, edges = imports + `mod` declarations) |
| `inspect tree [path]` | the extracted **model** rendered by unit and module boundary (module-level edges only) |
| `inspect scanner [path]` | the extracted model **plus** cross-unit hard edges (units + modules + edges) |

The mode keyword, when present, is the first positional argument; `path` then follows it. The model modes detect the language, run the same `scan` extraction, and render the model via `structural::render_model` — so they work for any language with a driver, not only Rust. `--format` is validated in every mode but **only the default mode honours it**: `tree` and `scanner` always emit a Mermaid model diagram, ignoring `--format plantuml`. `--output` and `--check` behave identically in all modes.

## Accepted flag values

- `--format`: `mermaid`, `plantuml`. Any other value is an error (see `errors.md`).
- `--output`: any writable file path.
- `--check`: freshness switch; compares the diagram to the resolved destination without writing (see `errors.md`).

## Usage examples

### Default run

```bash
archspec inspect
```

Scan the current directory, print a Mermaid diagram to stdout.

### Explicit path

```bash
archspec inspect ./crates/auth
```

Scan `./crates/auth`, print a Mermaid diagram to stdout.

### Structural model

```bash
archspec inspect tree ./crates/auth
```

Render the extracted model grouped by unit and module boundary (no cross-unit edges). `archspec inspect scanner ./crates/auth` adds the cross-unit hard edges. Both ignore `--format`.

### Mermaid to a file

```bash
archspec inspect ./crates/auth --output docs/imports.md
```

Write a Mermaid diagram to `docs/imports.md`. Stdout stays empty.

### PlantUML to a file

```bash
archspec inspect ./crates/auth --format plantuml --output docs/imports.puml
```

Write a PlantUML diagram to `docs/imports.puml`. Stdout stays empty.

### Combine anywhere

Flags and path can be given in any order:

```bash
archspec inspect --output out.mmd --format mermaid ./src
```

### Freshness check (CI gate)

```bash
archspec inspect --output docs/imports.md --check
```

Render the diagram in memory and compare it byte-for-byte with `docs/imports.md`. Nothing is written; a fresh destination exits `0`, a stale or missing one exits non-zero with the regeneration command.

## Behavior notes

- Positional arguments: in the default mode at most one `path` (omitted means the current directory); in `tree`/`scanner` the mode keyword may be followed by at most one `path`. More positional arguments are an error (`expected at most one path argument`).
- A valid run exits `0`. Any error condition exits non-zero with a message on stderr (see `errors.md`).
- `inspect` is single-shot: it scans, renders, and exits. No daemon, no persistent state.
- Destination: `--output` wins, then `[output] inspect` in `archspec.toml` (relative to project root), then stdout (`../../config.md`).
- `--check` compares the diagram to the destination without writing: identical bytes exit `0`; a differing or missing destination exits non-zero; with no destination at all it is an error (`errors.md`).

## Help

`archspec inspect --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
