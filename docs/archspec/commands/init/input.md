# `init` — Input Contract

## Accepted input

Exactly one optional positional argument: an existing project directory.

- If omitted, the **current working directory** is used.
- The directory must exist and be a directory.
- The project must be in a supported language, detectable from its manifests or sources.

```bash
# scaffolds into the current directory
archspec init

# scaffolds into the given project directory
archspec init ./crates/my-app
```

## Language detection

`init` detects the project language from the project tree. A language is detected when the tree contains its manifest or a source file in that language:

| Language | Detection signal |
|---|---|
| Rust | `Cargo.toml` or any `*.rs` present |
| C#/.NET | any `*.csproj` / `*.sln` or any `*.cs` present |
| Go | `go.mod` or any `*.go` present |

Signals are searched recursively under the project directory. If more than one language is signaled, a fixed precedence decides so behavior stays deterministic (Rust first, then C#/.NET, then Go). A project with no signal has an undetectable language.

## No profile selection

`init` has **no `--profile` flag and no profile catalog**. It writes a minimal base spec (`[project] language` plus the global `no_cycles` constraint) for the detected language. There is nothing to select — boundaries come later, from the user editing the spec or running `update` (see `../../commands/update/`).

```bash
# detection writes a minimal base spec
archspec init ./rust-workspace
```

## Rejected input

| Input | Why rejected |
|---|---|
| path that does not exist | nothing to scaffold into |
| path that is a regular file | only directories are scaffoldable |
| existing `architecture.spec.toml` in the project directory | overwriting a guarded spec is destructive |
| existing `archspec.toml` in the project directory | an existing output config must not be silently clobbered |
| project language not detectable | cannot choose a language for the spec |
| more than one positional argument | ambiguous target |

Both target files are checked before anything is written, so a rejection creates nothing.

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: no such directory
archspec init /no/such/dir

# rejected: it is a file, not a directory
archspec init Cargo.toml

# rejected: a spec already exists in the target directory
archspec init ./already-guarded

# rejected: no manifests or supported sources
archspec init ./empty-project

# rejected: two paths given
archspec init ./src ./tests
```

## Notes

- `init` reads the project tree only to detect the language and to refuse clobbering. It does not analyze source content beyond that.
- The spec is written into the target directory itself — not into a subfolder.
