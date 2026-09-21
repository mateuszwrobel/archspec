# `spec` — Input Contract

## Accepted input

No positional input. The command takes zero or one flag (`--schema`).

```bash
# prints the annotated reference
archspec spec

# prints the JSON Schema
archspec spec --schema
```

The command does not read the filesystem — it prints static, embedded content. It neither requires nor accepts a project directory, a spec file, or any toolchain.

## Rejected input

| Input | Why rejected |
|---|---|
| any positional argument | only static output is printed; a path is meaningless |
| any unknown flag (e.g. `--bogus`) | only `--schema` is a recognized flag |

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: unknown flag
archspec spec --bogus

# rejected: positional argument
archspec spec ./project
```

## Notes

- The flag may appear anywhere in the arguments; `archspec --schema spec` is disallowed by dispatch (the first argument must be the command name).
- Output is independent of the current working directory.