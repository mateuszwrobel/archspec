# `report` — Command Surface (CLI Contract)

## Invocation

```
archspec report [path] [--format <text|markdown|json>] [--output <path>] [--check]
```

`path` is optional and positional. Every flag is optional.

| Part | Meaning | Default |
|---|---|---|
| `path` | project directory to report on | current working directory |
| `--format` | report format | `text` |
| `--output <path>` | write the report to that file | stdout unless `[output]` set |
| `--check` | compare the report to its destination without writing | off |

## Accepted flag values

- `--format`: `text`, `markdown`, `json`. Any other value is an error (see `errors.md`).
- `--output`: any writable file path.
- `--check`: freshness switch; compares to the resolved destination without writing (see below and `errors.md`).

## Usage examples

### Default run

```bash
archspec report
```

Report on the current directory in plain text, printed to stdout.

### Explicit path

```bash
archspec report ./crates/auth
```

Report on `./crates/auth` in plain text, printed to stdout.

### Markdown to a file

```bash
archspec report ./crates/auth --format markdown --output docs/architecture-report.md
```

Write a Markdown report to `docs/architecture-report.md`. Stdout stays empty.

### Text to a file

```bash
archspec report ./crates/auth --output report.txt
```

Write a plain-text report to `report.txt`. Stdout stays empty.

### Combine anywhere

Flags and path can be given in any order:

```bash
archspec report --output out.md --format markdown ./src
```

## Destination

| Run | Destination |
|---|---|
| `report --output <path>` | the given file, stdout `wrote <path>` |
| `report` with `[output] report` in `archspec.toml` | the configured path (relative to project root) |
| `report` otherwise | stdout |

Resolution order: `--output` wins, then the config default (`../../config.md`), then stdout.

A configured `[output] report` destination receives the text artefact it is
provisioned for (what `report --check` compares). An explicit `--format` whose
format differs from it (`markdown`, `json`) writes to stdout and leaves the
configured path untouched; `--format text` behaves as without the flag, and
`--output` always wins. See `output.md`.

## Markdown report

`--format markdown` produces a self-contained Markdown artefact that adds, on
top of the diff + metrics:

- a `## Diagram` section embedding the current file-level import-map diagram as
  a Mermaid code fence (renders in Gitea/GitHub);
- a `## Forbidden imports` section listing every forbidden edge present,
  disallowed cross-component dependency, and facade dependency;
- a `## Cycles` section listing every cycle detected.

These sections appear only in the Markdown format; the plain-text format stays a
diff + metrics report. Every supported language (rust, csharp, go) has a
file-level import-map scanner, so the `## Diagram` section carries a diagram on
any tree.

## JSON report

`--format json` emits the machine-readable verdict: the same metrics summary and
the same finding set the text renderers print, as `{language, metrics,
findings}` with one `{category, severity, message}` entry per diff line. The
findings array is the contract (category ids match `help diagnostics`); metrics
are informational. The canonical model JSON stays `scan`'s output — the report
JSON never embeds a second full model encoding. The shape is identical for
every driver. See `output.md` for the schema and an example.

## Behavior notes

- Exactly one positional argument. Extra positional arguments are an error.
- A successful run exits `0` when the code satisfies its spec; **error-level rule violations exit non-zero** after the full report is emitted. Warning-level findings never gate. Non-zero exits for operational failures are described in `errors.md`.
- Warning-severity findings (dead references, laundered forbidden edges, unresolved module files, unowned module-edge endpoints) and vacuous constraints are ordinary diff lines under their own category label in all formats — never a separate stream, never a gate; in the JSON format they carry `severity: "warning"`.
- `report` is single-shot: it extracts, compares, renders, and exits. No daemon, no persistent state.

## Help

`archspec report --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
