# `help` — Failure Contract (Errors)

The command is static: the only failures are invalid invocations. There are no
operational failures (no filesystem access, no toolchain probing).

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| unknown topic | the unknown topic name, the list of valid topics, a hint at `archspec help` | non-zero |
| more than one positional | that at most one topic is accepted | non-zero |
| unknown flag | the flag name | non-zero |

## Examples

```bash
# message names the unknown topic, lists valid topics, hints at the index
$ archspec help bogus
error: unknown help topic: bogus
valid topics: commands, glob, spec, constraints, languages, workflow, diagnostics
run 'archspec help'

# message says more than one topic is rejected
$ archspec help glob spec
error: expected at most one path argument

# message names the unknown flag
$ archspec help --bogus
error: unknown flag: --bogus
```

## Output on error

- stderr carries the message; stdout carries nothing.
- Exit code is non-zero.