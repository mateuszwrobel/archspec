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

## Resolution order

For each command:

1. `--output <path>` wins. The path is relative to the current working
   directory, as today.
2. Else, config `[output].<key>` for the artefact, resolved relative to the
   project root.
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
| 1 | project with `archspec.toml` `[output] report = "docs/archspec/report.md"` and a valid spec | run `report` | exit 0; `docs/archspec/report.md` contains the report; stdout empty |
| 2 | project with `archspec.toml` `[output] inspect = "docs/archspec/inspect.mmd"` and supported sources | run `inspect` | exit 0; `docs/archspec/inspect.mmd` contains the diagram; stdout empty |
| 3 | project with `archspec.toml` `[output] scan = "docs/archspec/scan.json"` and supported sources | run `scan` | exit 0; `docs/archspec/scan.json` contains the model; stdout empty |
| 4 | project with `archspec.toml` `[output] diagram = "docs/archspec/diagram.mmd"` and a valid spec | run `diagram` | exit 0; `docs/archspec/diagram.mmd` contains the diagram; stdout empty |
| 5 | project with a config default for an artefact | run `<cmd> --output custom.ext` | exit 0; the artefact is written to `custom.ext`; the configured path is not created |
| 6 | project without `archspec.toml` | run `report` | exit 0; report on stdout |
| 7 | project with `archspec.toml` missing the `[output] report` key | run `report` | exit 0; report on stdout |
| 8 | project with malformed `archspec.toml` | run `report` | non-zero exit; message names the config file; nothing written |