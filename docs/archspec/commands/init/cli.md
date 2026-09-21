# `init` — Command Surface (CLI Contract)

## Invocation

```
archspec init [path]
```

`path` is optional and positional.

| Part | Meaning | Default |
|---|---|---|
| `path` | project directory to scaffold into | current working directory |

There is **no `--profile` flag**. `init` writes a minimal base spec for the detected language — `[project] language` plus the global `no_cycles` guard (see `../../spec.md`) and an `archspec.toml` output config with the standard `[output]` defaults (see `../../config.md`); boundaries are filled in later by the user or by `update`.

## Language detection

`init` detects the project language from its manifests or sources (see `input.md`):

| Detected language | Signal |
|---|---|
| Rust | `Cargo.toml` or any `*.rs` source |
| C#/.NET | any `*.csproj` / `*.sln` or any `*.cs` source |
| Go | `go.mod` or any `*.go` source |

The detected language is written to the spec's `[project] language`.

## Usage examples

### Default run

```bash
archspec init
```

Detect the language of the current directory, write `architecture.spec.toml` with a minimal `[project] language` header and the global `no_cycles` constraint, and write `archspec.toml` with the standard `[output]` defaults.

### Explicit path

```bash
archspec init ./crates/my-app
```

Detect the language of `./crates/my-app` and write both files into that directory.

## Behavior notes

- Exactly one positional argument. Extra positional arguments are an error.
- `init` **refuses to overwrite** an existing `architecture.spec.toml` or `archspec.toml`; there is no force flag (see `errors.md`). A future `force` flag will allow overwriting.
- A valid run exits `0` and prints a short summary on stdout (see `output.md`). Any error condition exits non-zero with a message on stderr (see `errors.md`).
- `init` is single-shot: it writes two files and exits. No daemon, no persistent state.

## Help

`archspec init --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
