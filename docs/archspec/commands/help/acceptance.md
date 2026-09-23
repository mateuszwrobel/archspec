# `help` — Acceptance Tests

CLI-level, behavioral only. Each scenario runs the real command against a temp directory and asserts on observable output (exit code, stdout/stderr). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `archspec help` prints the topic index (usage line, all 7 topics, every command) and exits `0`; `help topics` is byte-identical to `help`; `help commands` names every command with its purpose and its flags; `help glob` documents the glob semantics; `help spec` prints the shared spec reference (byte-identical to `archspec spec`); `help constraints` names all 7 types with their keys and the severity contract; `help languages` prints one block per language with the tier matrix; `help workflow` states the recipe, `--strict` as the CI gate, and the supporting commands; `help diagnostics` prints the finding catalog naming every category with its verbatim wordings, the exit-code contract, and the resolution classes; `help <command>` is byte-identical to `<command> --help`; `help --help` prints the topic index; an unknown topic produces a clear error with the valid topics and a hint; and repeated help runs are byte-identical.

## Topic index

| # | Given | When | Then |
|---|---|---|---|
| 1 | a terminal | run `archspec help` | exit 0; stdout has a usage line, lists all 7 topics and every command; stderr empty |
| 2 | a terminal | run `archspec help topics` | exit 0; output equals `archspec help` output (byte-identical) |
| 3 | a terminal | run `archspec help --help` | exit 0; prints the topic index (help's own help); stderr empty |

## Topics

| # | Given | When | Then |
|---|---|---|---|
| 4 | a terminal | run `archspec help commands` | exit 0; names every command with its purpose and its flags (`--strict`, `--output`, `--format`, `--schema`, …); stderr empty |
| 5 | a terminal | run `archspec help glob` | exit 0; states: `*`/`**` match any run incl separators (`.`, `/`, `::`, `-`); case-sensitive, no regex; unit globs match full unit names; module globs match full path, bare last segment, or unit-stripped path; `matches.modules` is a subtree (module + descendants); `from`/`forbid`/`gated_modules`/`allowed_from`/`parent` share the module-path semantics; `allowed.depend_on`/`forbidden` and `no_cycles.modules` are exact declared names (NOT globs) |
| 6 | a terminal | run `archspec help spec` | exit 0; contains `architecture.spec.toml`, `[project]` (`language` = rust\|csharp\|go), `[[module]]` (`matches` units/modules, `contract`, `[module.allowed]` depend_on/forbidden), `[[constraint]]`, and the annotated example TOML |
| 7 | a terminal | run `archspec help constraints` | exit 0; names all 7 constraint types with their keys and documents default severity `error`, `warning`, and `--strict` promotion |
| 8 | a terminal | run `archspec help languages` | exit 0; one block per language (rust, csharp, go) listing which model tiers each scanner populates, matching real scan output; stderr empty |
| 9 | a terminal | run `archspec help workflow` | exit 0; states the numbered audit recipe (steps `1.`–`6.`, the ordering this topic owns) with each step's purpose; mentions `--strict` as the CI gate; mentions `doctor`, `init`, `update` |
| 17 | a terminal | run `archspec help diagnostics` | exit 0; catalog is non-empty and names every finding category, quotes the verbatim verify/report message fragments, states the exit-code contract (`exit 0` / `exit 1`) with `--strict` promoting warnings, names the resolution classes `code-fix` / `spec-fix` / `architecture-rework`, and specifies the required report format; stderr empty |

## Per-command help

| # | Given | When | Then |
|---|---|---|---|
| 10 | a terminal | run `archspec help verify` | exit 0; output byte-identical to `archspec verify --help` |
| 11 | a terminal | run `archspec help spec` | exit 0; output byte-identical to `archspec spec` (shared reference constant) |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 12 | a terminal | run `archspec help bogus` | exit 1; stderr names the unknown topic `bogus`, lists the valid topics, hints `run 'archspec help'` |

## Consistency

| # | Given | When | Then |
|---|---|---|---|
| 13 | a terminal | run `archspec help spec` twice | both exit 0; byte-identical stdout (determinism) |
| 14 | a terminal | run `archspec help glob` twice | byte-identical output (determinism) |
| 15 | the `help` source | — | the `languages` tier matrix does not drift from the real scanners — guarded by a unit test that extracts a tiny fixture per language and asserts every listed tier is populated and every unlisted tier is empty |

## Help

| # | Given | When | Then |
|---|---|---|---|
| 16 | a terminal | run `archspec help --help` | exit 0; output shows the usage line and the topic index; stderr empty |