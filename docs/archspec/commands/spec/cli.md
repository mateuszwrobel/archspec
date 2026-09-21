# `spec` — Command Surface (CLI Contract)

## Invocation

```
archspec spec [--schema]
```

`--schema` is optional. No positional argument is accepted.

| Part | Meaning | Default |
|---|---|---|
| `--schema` | print the JSON Schema (draft-07) for `architecture.spec.toml` instead of the reference | off |

## Accepted flags

- `--schema`: boolean, takes no value. Any other flag is an error (see `errors.md`).

## Usage examples

### Annotated reference

```bash
archspec spec
```

Print the annotated example TOML covering every section (`[project]`, `[[module]]`, `[module.allowed]`, `[[constraint]]` for all 7 types, `[[stereotype]]`) to stdout.

### JSON Schema

```bash
archspec spec --schema
```

Print a JSON Schema (draft-07) for `architecture.spec.toml` to stdout. The schema's per-type `required` arrays match exactly what the runtime validator enforces at load time.

## Behavior notes

- No positional arguments. Any positional argument is an error.
- Exit `0` on success; non-zero on an invalid invocation (see `errors.md`).
- `spec --help` prints this command's usage plus the same reference material; it runs nothing else.
- `spec` is single-shot and pure: no project reads, no state, no side effects beyond stdout.

## Help

`archspec spec --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.