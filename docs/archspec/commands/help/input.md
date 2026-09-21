# `help` — Input Contract

## Accepted input

Zero or one positional: a topic or a command name. No flags are defined by the
`help` command itself; the global `--help` is intercepted by dispatch.

```bash
# prints the topic index
archspec help

# prints one topic or a command's help
archspec help glob
archspec help spec
archspec help verify
```

The command does not read the filesystem. It prints static, embedded content;
it neither requires nor accepts a project directory, a spec file, or any
toolchain.

## Rejected input

| Input | Why rejected |
|---|---|
| more than one positional argument | only one topic is printed at a time |
| an unknown topic (e.g. `help bogus`) | names a topic or command that does not exist |
| any unknown flag (e.g. `help --bogus`) | only the global `--help` is recognized (by dispatch) |

Each rejection produces a clear message naming the cause and a non-zero exit
(see `errors.md`).

## Examples

```bash
# rejected: unknown topic
archspec help bogus

# rejected: more than one topic
archspec help glob spec
```

## Notes

- The `--help` flag may appear anywhere in the arguments; `help --help` prints
  the topic index via dispatch, before any topic lookup.
- Output is independent of the current working directory.