# `verify` — Input Contract

## Accepted input

Exactly one optional positional argument: a directory.

- If omitted, the **current working directory** is used.
- The directory must exist, must contain `architecture.spec.toml`, and must contain at least one source file in a supported language, discovered recursively.
- The extract covers the whole directory tree beneath the given path, as scoped by the spec's matching rules.

```bash
# uses the current directory
archspec verify

# uses the given project directory
archspec verify ./crates/auth
```

## Spec discovery

- The spec file is `architecture.spec.toml` (see `../../spec.md`), located directly in the given directory.
- The spec's `[project].language` selects the language whose sources are extracted and compared.

## Rejected input

| Input | Why rejected |
|---|---|
| path that does not exist | nothing to verify |
| path that is a regular file | only directories are verifiable |
| directory with no `architecture.spec.toml` | nothing to compare against |
| directory with no supported-language sources | nothing to extract |
| more than one positional argument | ambiguous target |

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: no such directory
archspec verify /no/such/dir

# rejected: it is a file, not a directory
archspec verify Cargo.toml

# rejected: no spec file
archspec verify ./docs

# rejected: no supported-language sources
archspec verify ./empty-project

# rejected: two paths given
archspec verify ./src ./tests
```

## Notes

- The spec is the comparison target — verification is impossible without it (`archspec init` scaffolds one).
- An incomplete spec is a failure, not a silent gap: an extracted unit matched by no declared component is reported as unassigned, never dropped (see `../../spec.md`).
