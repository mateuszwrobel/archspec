# `archspec skill` — errors

| Situation | Exit | Message shape |
|---|---|---|
| unknown subcommand | 1 | `error: unknown skill subcommand: <value> (expected: print, install)` |
| path given to `print` | 1 | `error: skill print takes no path arguments` |
| more than one path after `install` | 1 | `error: expected at most one path argument` |
| unknown flag | 1 | `error: unknown flag: --<name>` |
| modified install, no `--force` | 1 | `error: <path> differs from the shipped skill; pass --force to overwrite` |
| filesystem failure | 1 | `error: cannot <read|write|create ...>: <os error>` |
