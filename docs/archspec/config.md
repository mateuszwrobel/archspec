# `archspec.toml` — Output Config Contract

Optional per-project config controlling where generated artefacts land. When
absent, every command behaves exactly as before: stdout unless `--output` is
given.

## Location & discovery

File `archspec.toml` at the project root — the directory a command operates on
(the positional `path`, or the current working directory). Commands that read
the config: `inspect`, `diagram`, `report`, `scan`. The config is optional; a
missing file is not an error and does not change any command's behavior.
`init` scaffolds `archspec.toml` with the standard `[output]` defaults below
(see `commands/init/output.md`).

## Format

```toml
[output]
inspect = "docs/archspec/inspect.mmd"
diagram = "docs/archspec/diagram.mmd"
report  = "docs/archspec/report.md"
scan    = "docs/archspec/scan.json"
```

Each key names the artefact type:

| Key | Command | Artefact |
|---|---|---|
| `inspect` | `inspect` | file-level import-map diagram |
| `diagram` | `diagram` | architecture diagram |
| `report` | `report` | diff + metrics report |
| `scan` | `scan` | canonical model JSON |

A value is a destination file path, resolved **relative to the config file's
directory** (the project root). Any key may be absent; that artefact then falls
back to stdout.

A `--check` compares the canonical rendering unless `--format` names another; to verify a `--format`-generated artefact, pass the same `--format` to `--check`.

## Resolution order

For each command:

1. `--output <path>` wins. The path is relative to the current working
   directory, as today. `--output -` is the exception: the body goes to stdout,
   no file is created, and no destination is named at all — so rule 2's status
   line is not printed either.
2. Else, config `[output].<key>` for the artefact, resolved relative to the
   project root. Every body written there — human or machine, of any format —
   echoes `wrote <path>` on stdout.
3. Else, stdout.

## Errors

Malformed TOML in `archspec.toml` is an operational error: non-zero exit, a
message naming the file, and nothing written. Unknown keys and sections are
ignored.

## Acceptance tests

CLI-level, behavioral only. Each scenario runs the real command against a
fixture project and asserts on observable output.

| # | Given | When | Then |
|---|---|---|---|
| 1 | project with `archspec.toml` `[output] report = "docs/archspec/report.md"` and a valid spec | run `report` | exit 0; `docs/archspec/report.md` contains the report; stdout is `wrote docs/archspec/report.md` |
| 2 | project with `archspec.toml` `[output] inspect = "docs/archspec/inspect.mmd"` and supported sources | run `inspect` | exit 0; `docs/archspec/inspect.mmd` contains the diagram; stdout is `wrote docs/archspec/inspect.mmd` |
| 3 | project with `archspec.toml` `[output] scan = "docs/archspec/scan.json"` and supported sources | run `scan` | exit 0; `docs/archspec/scan.json` parses as the model JSON; stdout is `wrote docs/archspec/scan.json` |
| 4 | project with `archspec.toml` `[output] diagram = "docs/archspec/diagram.mmd"` and a valid spec | run `diagram` | exit 0; `docs/archspec/diagram.mmd` contains the diagram; stdout is `wrote docs/archspec/diagram.mmd` |
| 5 | project with a config default for an artefact | run `<cmd> --output custom.ext` | exit 0; the artefact is written to `custom.ext`; the configured path is not created; stdout is `wrote custom.ext` |
| 6 | project without `archspec.toml` | run `report` | exit 0; report on stdout |
| 7 | project with `archspec.toml` missing the `[output] report` key | run `report` | exit 0; report on stdout |
| 8 | project with malformed `archspec.toml` | run `report` | non-zero exit; message names the config file; nothing written |
| 9 | project with a config destination for the report | run `report`, then `report --output copy.md` | both runs write byte-identical bodies — the `wrote` line is the only stdout move |
| 10 | project with a config destination provisioned for the text report | run `report --format markdown` | exit 0; stdout opens with `note: destination docs/archspec/report.md not written` and carries the body; the destination keeps its bytes |
| 11 | project with a config destination provisioned for the text report | run `report --format json` | exit 0; stdout is parseable JSON with no status line inside; the destination keeps its bytes |
| 12 | project with a config destination for `diagram` | run `depgraph modules --format plantuml` or `inspect --format plantuml` | exit 0; stdout opens with the plantuml body and carries no status line |
| 13 | project with `[output]` destinations configured | run `report --output -` (or `scan`/`diagram`/`inspect`/`depgraph <view>` with it) | exit 0; stdout is the body alone, with no status line; no file named `-` is created; the configured destination keeps its bytes |
