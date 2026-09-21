# `scan` — Command Surface (CLI Contract)

## Invocation

```
archspec scan [path] [--output <path>] [--check]
```

`path` is optional and positional. `--output` is optional.

| Part | Meaning | Default |
|---|---|---|
| `path` | project directory to scan | current working directory |
| `--output <path>` | write the canonical model to that file | stdout unless `[output]` set |
| `--check` | compare the model to its destination without writing | off |

## Accepted flag values

- `--output`: any writable file path. The file is created or overwritten with the model.
- `--check`: freshness switch; compares the model to the resolved destination without writing (see below and `errors.md`).

## Usage examples

### Default run

```bash
archspec scan
```

Detect the language in the current directory, extract the model, print JSON to stdout.

### Explicit path

```bash
archspec scan ./crates/auth
```

Scan `./crates/auth`, print the model as JSON to stdout.

### Write the model to a file

```bash
archspec scan ./crates/auth --output docs/model.json
```

Write the model to `docs/model.json`. Stdout stays empty.

### Combine anywhere

Flags and path can be given in any order:

```bash
archspec scan --output out.json ./src
```

### Freshness check (CI gate)

```bash
archspec scan --output docs/model.json --check
```

Extract the model in memory and compare it byte-for-byte with `docs/model.json`. Nothing is written; a fresh destination exits `0`, a stale or missing one exits non-zero with the regeneration command.

## Behavior notes

- Exactly one positional argument. Extra positional arguments are an error.
- A valid run exits `0`. Any error condition exits non-zero with a message on stderr (see `errors.md`).
- `scan` is single-shot: it detects the language, extracts the model, and exits. No daemon, no persistent state.
- `scan` has no `--format` flag: the model is always emitted as the canonical JSON IR (see `output.md`).
- Destination: `--output` wins, then `[output] scan` in `archspec.toml` (relative to project root), then stdout (`../../config.md`).
- `--check` compares the model to the destination without writing: identical bytes exit `0`; a differing or missing destination exits non-zero; with no destination at all it is an error (`errors.md`).

## Help

`archspec scan --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
