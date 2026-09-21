# `doctor` — Input Contract

## Accepted input

None. `doctor` takes no positional arguments and no flags in phase 1.

```bash
# the only valid invocation
archspec doctor
```

`doctor` inspects the environment it runs in — the machine — not the current working directory and not a project tree. The report is the same regardless of where the command is invoked from.

## Rejected input

| Input | Why rejected |
|---|---|
| any positional argument (e.g. a path) | `doctor` diagnoses the environment, not a project |
| any flag | phase 1 has no flags |

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: path arguments are not accepted
archspec doctor ./my/project

# rejected: phase 1 has no flags
archspec doctor --verbose
```

## Notes

- No spec file, no manifest, and no configuration are read.
- The report does not depend on the current working directory or on any project on disk.
- Toolchain absence is not a rejected input — it is a finding the report describes (see `output.md`).
