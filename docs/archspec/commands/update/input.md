# `update` — Input Contract

## Accepted input

Exactly one optional positional argument: a directory.

- If omitted, the **current working directory** is scanned.
- The directory must exist and must contain at least one supported-language source, discovered recursively.
- The scan covers the whole directory tree beneath the given path.

```bash
# snapshots the current directory
archspec update

# snapshots the given project directory
archspec update ./crates/auth

# snapshots a sub-tree of a project
archspec update ./src
```

## Rejected input

| Input | Why rejected |
|---|---|
| path that does not exist | nothing to scan |
| path that is a regular file | only directories are scannable |
| directory with no supported-language sources (recursively) | no model to snapshot |
| `architecture.spec.toml` already present and no `--force` | a reviewed/edited spec must not be silently clobbered |
| more than one positional argument | ambiguous target |

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: no such directory
archspec update /no/such/dir

# rejected: it is a file, not a directory
archspec update Cargo.toml

# rejected: no supported-language sources
archspec update ./docs

# rejected: spec already exists, no --force
archspec update

# rejected: two paths given
archspec update ./src ./tests
```

## Notes

- No spec is read as input. An existing `architecture.spec.toml` is only ever treated as a conflict, never as a base to merge with.
- Extraction is driver-driven: the scanned tree's language must be detected, and its driver ships inside the binary — no toolchain on `PATH` gates it. The "toolchain not found" message is reachable only through the `ARCHSPEC_DISABLE_DRIVERS` test seam (see `errors.md`).
- The snapshot is written in the declared spec format (`../../spec.md`).
