# Help — Tool-wide CLI Contract

`archspec` has a built-in help surface. All help text is generated from the
command registry in code: each command's usage and description live in its own
module, so the runtime text has a single source of truth. These contracts pin
what users see; the runtime text is produced from code (`src/archspec/`), not
from these files. `cli.md` per command stays the authoritative behavioral
reference for invocation and flags.

## Invocation forms

| Form | Behavior | Exit |
|---|---|---|
| `archspec` | print tool help: usage + full command list | 0 |
| `archspec --help` | print tool help: usage + full command list | 0 |
| `archspec --version` / `archspec version` | print the tool version (`archspec <crate version>`) — identity, not a command | 0 |
| `archspec <command> --help` | print that command's help: usage + flags | 0 |
| `archspec help [topic]` | print the built-in manual: topic index or one topic | 0 |
| `archspec help <command>` | print that command's help (aliases `<command> --help`) | 0 |

`--help` is recognized in any position after the command name and always wins:
`archspec scan --output out.json --help` prints `scan`'s help and runs nothing.
Identity resolves before command dispatch: the two spellings `--version` and
`version` print the version, and any other unknown name still meets the
unknown-command refusal (see Errors). The tool help (`archspec`, `archspec
--help`) and the topic index (`archspec help`, `help topics`, `help --help`)
all carry the full command list. Help output goes to stdout; stderr stays
empty.

## Tool help (no args or `--help`)

- A usage line: `archspec <command> [args]`.
- One line per command: the command name, padded, then a one-line purpose.
- Three trailing hint lines, in this order: `run 'archspec <command> --help'
  for details on a command`, `run 'archspec --version' for the tool version`,
  and a pointer at the topics: `run 'archspec help' for topics: commands, glob,
  spec, constraints, languages, roles, workflow, diagnostics`.

The command list covers every command the tool dispatches. The ordering and
purposes match the command table in `README.md`.

## Command help (`<command> --help`)

- A usage line matching that command's `cli.md` invocation.
- A one-line description of what the command does.
- One line per positional/flag part: the part name, whether it takes a value,
  and its meaning (with default where applicable).
- The command runs nothing else: parsing and validation of other flags/args is
  skipped.

## The `help` command

`archspec help [topic]` is the built-in manual. `help <command>` aliases that
command's `--help` (`archspec help verify` prints exactly what
`archspec verify --help` prints). The topics:

| Topic | Content |
|---|---|
| `commands` | one block per command: name, purpose, flags |
| `glob` | glob matching rules for units and module paths (incl. which keys are exact declared-name matches) |
| `spec` | the `architecture.spec.toml` annotated reference (shared with `archspec spec`) |
| `constraints` | the seven constraint types, their keys, and the severity contract |
| `languages` | the language-tier matrix — which model tiers each scanner populates |
| `roles` | which views mark roles and which stay silent by decision — the roles-in-views contract |
| `workflow` | the numbered audit recipe (this topic owns the ordering), `--strict` as the CI gate |
| `diagnostics` | the catalog of every `verify`/`report` finding category — message pattern, meaning, origin, severity + `--strict`, follow-up, decision, and the required agent report format |

`archspec help`, `archspec help topics`, and `archspec help --help` all print
the same topic index (usage line + all 8 topics + every command). All topic
content is embedded in the binary (`src/archspec/commands/help.rs`), so output
is deterministic and byte-identical across runs.

## Output destinations

The help surface never touches a project tree. Output destinations for
`inspect`, `diagram`, `report`, and `scan` default to
the `[output]` table of `archspec.toml` when present, and to stdout otherwise
(`--output` overrides both; see `config.md`).

### Closed pipes

A render command whose consumer exits first is silence, not panic: the next write to a closed pipe dies quietly (SIGPIPE — shell status 141) or the process exits 0 when the output had already flushed before the pipe closed; no panic text is ever printed. The `archspec ... | head -1` shape is therefore always safe. One contract for the whole tool — no per-command duplicates — pinned by `tests/cli_pipe.rs`.

## Errors

- Unknown command: `error: unknown command: <name>` followed by
  `run 'archspec --help' for the command list` on stderr, exit `1`. This applies
  even when `--help` follows the unknown name.
- `--help` is never a valid positional or flag value for any command; it is
  always intercepted before command parsing.