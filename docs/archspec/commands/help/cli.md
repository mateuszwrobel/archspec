# `help` — Command Surface (CLI Contract)

## Invocation

```
archspec help [topic]
```

`topic` is optional. At most one topic is accepted.

| Topic | Prints | Shared with |
|---|---|---|
| *(none)* | the topic index (usage line + all 7 topics + every command) | `help topics`, `help --help` |
| `topics` | the topic index, byte-identical to bare `archspec help` | — |
| `commands` | one block per command: name, purpose, flags | every command's own `--help` |
| `glob` | the glob matching rules for units and module paths | — |
| `spec` | the `architecture.spec.toml` annotated reference | `archspec spec` (shared constant) |
| `constraints` | the seven constraint types with their keys and severity | the JSON Schema in `archspec spec --schema` |
| `languages` | the language-tier matrix (which model tiers each scanner populates) | `LANGUAGE_TIERS` (consistency-tested) |
| `workflow` | the numbered audit recipe (this topic owns the ordering) | — |
| `diagnostics` | the catalog of every `verify`/`report` finding category: message pattern, meaning, origin, severity + `--strict`, follow-up, decision, and the required report format | the finding strings in `verify/compare.rs` |
| `<command>` | that command's `--help`, byte-identical | `archspec <command> --help` |

## Accepted input

- Zero or one positional: a topic name or a command name.
- `--help` anywhere after the command: intercepted by dispatch before parsing, prints the index (see `help.md`).

## Usage examples

```bash
# the topic index
archspec help

# the spec reference — same output as `archspec spec`
archspec help spec

# a command's help — same output as `archspec verify --help`
archspec help verify

# the glob rules
archspec help glob
```

## Behavior notes

- All output is pure and deterministic: every topic is an embedded constant (`src/archspec/commands/help.rs`) or derived from the command registry / `LANGUAGE_TIERS`, so repeated runs are byte-identical.
- `archspec` and `archspec --help` print the tool help — a different layout (usage line, padded command list, trailing hints at `<command> --help` and at the topics), not this topic index. Both carry the same command list (see `../../help.md`).
- Exit `0` on success; non-zero on an unknown topic (see `errors.md`).
- `help --help` prints the topic index and runs nothing else.

## Help

`archspec help --help` prints the topic index and exits `0`. See `../../help.md` for the tool-wide contract.