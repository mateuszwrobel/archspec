# `verify` — Command Surface (CLI Contract)

## Invocation

```
archspec verify [path] [--strict]
```

`path` is optional and positional. `--strict` is optional.

| Part | Meaning | Default |
|---|---|---|
| `path` | project directory to extract and compare | current working directory |
| `--strict` | promote warnings to errors (CI gate) | off |

## Accepted flag values

- `--strict`: boolean, takes no value. Any other flag is an error (see `errors.md`).

## Usage examples

### Default run

```bash
archspec verify
```

Extract the model from the current directory, compare it against `architecture.spec.toml` in the same directory, print the result.

### Explicit path

```bash
archspec verify ./crates/auth
```

Extract `./crates/auth`, compare it against `./crates/auth/architecture.spec.toml`, print the result.

### Strict gate

```bash
archspec verify --strict
```

Promote every warning to an error so any divergence fails the run — the CI gate.

### Combine anywhere

```bash
archspec verify --strict ./crates/auth
```

Flags and path can be given in any order.

## Behavior notes

- Exactly one positional argument. Extra positional arguments are an error.
- Exit `0` when no error-level failure; non-zero otherwise (see `errors.md`).
- A pass prints a short confirmation to stdout; a fail prints the full diff report to stdout. Warning findings are listed inside that stdout diff report (as `warning:` lines unless `--strict`); **stderr carries operational errors only** (see `errors.md`).
- `verify` is single-shot: it extracts, compares, reports, and exits. No daemon, no persistent state.

## Help

`archspec verify --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
