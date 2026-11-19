# `archspec.toml` — Output Config Contract

Optional per-project config controlling where generated artefacts land and
which diagram a markdown report embeds. When absent, every command behaves
exactly as before: stdout unless `--output` is given, and the `files` diagram.

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

[report]
diagram = "modules"
```

Each key names the artefact type:

| Key | Section | Command | Artefact |
|---|---|---|---|
| `inspect` | `[output]` | `inspect` | file-level import-map diagram |
| `diagram` | `[output]` | `diagram` | architecture diagram |
| `report` | `[output]` | `report` | diff + metrics report |
| `scan` | `[output]` | `scan` | canonical model JSON |
| `diagram` | `[report]` | `report` | which diagram the markdown report embeds |

An `[output]` value is a destination file path, resolved **relative to the
config file's directory** (the project root). Any key may be absent; that
artefact then falls back to stdout.

### `[report] diagram`

The `diagram` key of the `[report]` section selects the diagram a markdown
`report` embeds: `files` (the default — the file-level import map), `modules`
(the graph `archspec depgraph modules` draws), or `none` (omit the section).
The key is the committable surface: `report --check` reads it, so a run that
wrote with `--diagram` and a later check without the flag resolve the same
diagram from the same file. `report --diagram <mode>` overrides the key for
one-offs. An absent key means `files`.

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
message naming the file, and nothing written. A well-formed file carrying an
unknown key or section is not an error: each unknown key prints one `warning:`
line on stderr naming the file and the key (`section.key`, or the section name
for a key outside every known section) and stays ignored — a typo never
changes what renders, and now never stays silent.

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
| 14 | project with `archspec.toml` carrying a typo'd key (`[report] diagrams`) or a typo'd section (`[repot]`) | run `report --format markdown` | exit 0; one `warning:` line on stderr names the file and the key; the report is byte-identical to the config-less report |
| 15 | project with `[report] diagram = "modules"` | run `report --format markdown --diagram modules --output r.md`, then `report --format markdown --check --output r.md` (no flag) | the write echoes `wrote r.md`; the check exits 0 with stdout `ok: r.md up to date` — the key, not the flag, carries the diagram; the same write with no key goes stale-red on the flag-less check |
| 16 | project whose spec declares `[[async_depends]]`, with and without `[report] diagram = "files"` | run `report --format markdown` both ways | both runs write stderr nothing and emit byte-identical bodies with an identical verdict — the default arm never moves |
| 17 | project with `[report] diagram = "modules"` | run `report --format markdown --output r.md` (no flag), then `report --check`, then move the key to `files` and check again | the embedded diagram is the `depgraph modules` body byte-for-byte; the check honors the key; a moved key makes the check exit non-zero |
