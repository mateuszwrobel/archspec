# `init` — Failure Contract (Errors)

Every failure produces a clear human message naming the cause and a **non-zero exit**. Messages go to stderr; stdout carries nothing. Both target files are existence-checked before any write, so every rejection creates nothing and modifies nothing — the only case that can leave a file behind is a config write that fails after the spec was created (see below).

`init` refuses to overwrite an existing spec or output config: clobbering a guarded spec or a user's config is destructive, so a hard fail is intentional and honest. There is no force flag.

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| path does not exist | the invalid path | non-zero |
| path is a regular file | the path is invalid and why | non-zero |
| `architecture.spec.toml` already exists in the target | the existing file's full path | non-zero |
| `archspec.toml` already exists in the target | the existing file's full path | non-zero |
| project language not detectable | that no language was detected, and where | non-zero |
| more than one positional argument | that only one path is accepted | non-zero |
| spec file cannot be written | the path and why | non-zero |
| config file cannot be written | the path and why | non-zero |

## Examples

```bash
# message identifies the missing path
$ archspec init /no/such/dir
error: path does not exist: /no/such/dir

# message identifies the file and why it is rejected
$ archspec init Cargo.toml
error: path is not a directory: Cargo.toml

# message identifies the existing spec; nothing is overwritten
$ archspec init ./guarded
error: architecture.spec.toml already exists: ./guarded/architecture.spec.toml

# message identifies the existing config; nothing is overwritten
$ archspec init ./configured
error: archspec.toml already exists: ./configured/archspec.toml

# message says no language was detected and where
$ archspec init ./empty-project
error: could not detect project language: ./empty-project (expected Cargo.toml, go.mod, or *.csproj/*.sln)

# message says only one path is accepted
$ archspec init ./src ./tests
error: expected at most one path argument

# message says the spec could not be written
$ archspec init /readonly/guard
error: cannot write spec: /readonly/guard/architecture.spec.toml

# message says the config could not be written (the spec is already on disk)
$ archspec init /half-writable/guard
error: cannot write config: /half-writable/guard/archspec.toml
```

## Output on error

- stderr carries the message; stdout carries nothing.
- No file is created or modified: every rejection (path, existing spec, existing config, undetectable language) fails before the first write. A config write that fails after the spec was created leaves that newly created spec on disk; neither the spec nor the config that already existed is ever touched.
- Exit code is non-zero (distinct failure codes are not required in phase 1).
