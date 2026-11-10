# `explain` — Errors and Exit Codes

## Exit codes

| Code | Class | Meaning |
|---|---|---|
| 0 | green | the trace was produced — never a statement that the pair is clean |
| 64 | usage | no command, unknown flag, fewer than two positionals, more than one path positional |
| 70 | operational | a precondition the trace needs is missing or broken |

A finding never moves the exit code. A tree whose pair fails every constraint still exits 0 from explain — the verdict surface is `archspec verify` (exit 1).

## Operational preconditions (exit 70)

| Message | Cause |
|---|---|
| `path does not exist: <path>` | the third positional names no existing directory |
| `path is not a directory: <path>` | the third positional names a file |
| `no architecture.spec.toml found in: <path>` | no spec in the directory or its ancestors — the same bytes verify prints |
| `no supported-language sources found under: <path>` | nothing the declared driver can extract |
| driver/toolchain findings | marker and agreement findings ride verbatim, as on the verify surface |

## Usage mistakes (exit 64)

| Message | Cause |
|---|---|
| `usage: archspec explain <source> <target> [path] [--constraint <pattern>]` | fewer than two positional endpoints |
| `expected at most one path argument` | more than one positional path given |
| `unknown flag: --<name>` | any flag other than `--constraint` |

## Not errors

- an unresolved endpoint — a rendered fact, exit 0, stated in the endpoint section;
- findings naming the pair — rendered facts, exit 0;
- an empty filtered census — the named statement in the constraints section, exit 0.
