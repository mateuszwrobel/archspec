# `spec` — Failure Contract (Errors)

The command is static: the only failures are invalid invocations. There are no operational failures (no filesystem access, no toolchain probing).

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| unknown flag | the flag name | non-zero |
| positional argument | that the command takes no path | non-zero |

## Examples

```bash
# message names the unknown flag
$ archspec spec --bogus
error: unknown flag: --bogus

# message says no path is accepted
$ archspec spec ./project
error: spec takes no path argument
```

## Output on error

- stderr carries the message; stdout carries nothing.
- Exit code is non-zero.