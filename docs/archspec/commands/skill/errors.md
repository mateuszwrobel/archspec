# `archspec skill` — errors

| Situation | Exit | Message shape |
|---|---|---|
| unknown name or subcommand | 1 | `error: unknown skill: <value> (expected: print, install, audit, analysis)` |
| path given to `print` or a skill name | 1 | `error: skill <print\|audit\|analysis> takes no path arguments` |
| more than one path after `install` | 1 | `error: expected at most one path argument` |
| unknown flag | 64 | `error: unknown flag: --<name>` |
| modified install, no `--force` | 1 | `error: <path> differs from the shipped skill; pass --force to overwrite` |
| filesystem failure | 1 | `error: cannot <read\|write\|create ...>: <os error>` |

Selection is fail-loud: an unknown name never emits, never falls back to the
audit skill. The unknown-name refusal keeps the historical exit 1 of the
verb's argument refusals (a bare error, like the former unknown-subcommand
case); only the tool-wide usage paths carry 64.
