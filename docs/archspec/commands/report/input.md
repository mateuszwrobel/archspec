# `report` — Input Contract

## Accepted input

Exactly one optional positional argument: a directory.

- If omitted, the **current working directory** is used.
- The directory must exist and contain an `architecture.spec.toml` (see `../../spec.md`). The spec is the declared side of the diff and the source of truth.
- The spec's `[project] language` selects the extractor; the directory must contain at least one source file in that language, discovered recursively.

```bash
# reports on the current directory
archspec report

# reports on the given directory tree
archspec report ./crates/auth

# reports on a project nested in the tree
archspec report ./repos/billing
```

## Rejected input

| Input | Why rejected |
|---|---|
| path that does not exist | nothing to report on |
| path that is a regular file | only directories are accepted |
| directory without `architecture.spec.toml` | the diff has no declared side to compare against |
| invalid `architecture.spec.toml` | the declared model is not readable |
| directory with no sources in the spec's declared language | nothing to extract |

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: no such directory
archspec report /no/such/dir

# rejected: it is a file, not a directory
archspec report Cargo.toml

# rejected: no spec file
archspec report ./docs

# rejected: no sources in the declared language
archspec report ./rustless

# rejected: two paths given
archspec report ./src ./tests
```

## Notes

- The spec is required and is the source of truth (`../../spec.md`): its `[project] language` selects the extractor, and its declared model is the comparison side. Unlike `inspect` (zero-config discovery), `report` is spec-driven — no spec, no report.
- Only sources in the spec's declared language are considered. Other languages in the tree are ignored.
