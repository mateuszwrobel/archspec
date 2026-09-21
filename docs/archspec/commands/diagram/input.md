# `diagram` — Input Contract

## Model sources

`diagram` renders one of two model sources:

| Source | Input | Where it comes from |
|---|---|---|
| `spec` (default) | `architecture.spec.toml` | declared model, consumed by `verify` and `diagram` (`../../spec.md`) |
| `scan` | a model snapshot artefact file | produced by `archspec scan` (extracted current-state model) |

`diagram` reads the model only — never the source tree. Rendering is a pure function of the model.

## Accepted input

### Declared spec (`--source spec`, default)

- The spec file is located as `architecture.spec.toml` in the project directory.
- If the project directory is omitted, the **current working directory** is used.
- The project directory must exist and must be a directory.
- The spec is the single declaration of the governed model (`../../spec.md`).

```bash
# spec from the current directory
archspec diagram

# spec from the given project directory
archspec diagram ./crates/auth
```

### Scan snapshot (`--source scan`)

- The scan artefact path is given as the argument to `--source scan`.
- The artefact must exist and must be a valid model snapshot produced by `archspec scan` (the snapshot format is defined by the `scan` command's contract).
- When a governing spec (`architecture.spec.toml`) is present in the project directory, extracted edges are checked against its constraints and violations are marked in the diagram (see `output.md`). When no spec is present, nothing is forbidden and no edges are marked.

```bash
# the project directory defaults to the current directory
archspec diagram --source scan ./artifacts/model.json

# project directory given explicitly, for the governing-spec lookup
archspec diagram ./crates/auth --source scan ./artifacts/model.json
```

## Rejected input

| Input | Why rejected |
|---|---|
| path that does not exist | nothing to render from |
| path that is a regular file | only directories are valid project paths |
| project directory without `architecture.spec.toml` | no declared model to render |
| `--source scan` artefact that does not exist | nothing to render |
| `--source scan` artefact that is not a valid model snapshot | the model is unreadable |
| model with no components/units to render | nothing to draw |
| more than one positional argument | ambiguous target |
| `--source scan` without an artefact path | the snapshot is not named |

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: no such project directory
archspec diagram /no/such/dir

# rejected: it is a file, not a directory
archspec diagram Cargo.toml

# rejected: no spec in the project directory
archspec diagram ./docs

# rejected: no such scan artefact
archspec diagram --source scan ./artifacts/missing.json

# rejected: artefact is not a valid model snapshot
archspec diagram --source scan README.md

# rejected: empty model
archspec diagram --source scan ./artifacts/empty.json

# rejected: two paths given
archspec diagram ./src ./tests

# rejected: scan mode without an artefact path
archspec diagram --source scan
```

## Notes

- The spec format is defined in `../../spec.md`; the scan snapshot format is defined by the `scan` command's contract.
- `diagram` never modifies the project: no spec is written, no files are touched except the `--output` destination (see `output.md`).
