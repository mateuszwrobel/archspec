# `update` — Command Surface (CLI Contract)

## Invocation

```
archspec update [path] [--force]
```

`path` is optional and positional. Every flag is optional.

| Part | Meaning | Default |
|---|---|---|
| `path` | project directory to scan and snapshot | current working directory |
| `--force` | overwrite an existing `architecture.spec.toml` | off |

## Accepted flag values

- `--force`: no value; presence enables overwrite of an existing spec.
- Any other flag is an error (see `errors.md`).

## Usage examples

### Default run

```bash
archspec update
```

Extract the model from the current directory, write `architecture.spec.toml` there, print a summary to stdout.

### Explicit path

```bash
archspec update ./crates/auth
```

Extract the model from `./crates/auth`, write `architecture.spec.toml` there, print a summary to stdout.

### Overwrite an existing spec

```bash
archspec update --force
```

Regenerate `architecture.spec.toml` in the current directory even though one already exists. Only use when the existing spec is deliberately discarded.

### Combine anywhere

Path and flag can be given in any order:

```bash
archspec update --force ./crates/auth
```

## Behavior notes

- Exactly one positional argument. Extra positional arguments are an error.
- A valid run exits `0`. Any error condition exits non-zero with a message on stderr (see `errors.md`).
- `update` refuses to overwrite an existing `architecture.spec.toml` unless `--force` is given. The guard protects a reviewed or edited spec from silent clobbering.
- `update` is single-shot: it extracts, writes, and exits. No daemon, no persistent state.

## Help

`archspec update --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
