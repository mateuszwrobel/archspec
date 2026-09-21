# `inspect` — Input Contract

## Accepted input

Exactly one optional positional argument (a `tree`/`scanner` mode keyword may precede it): a directory.

- If omitted, the **current working directory** is scanned.
- The directory must exist and must contain Rust, C# or Go sources, discovered recursively. The tree's language is detected from its files (`Cargo.toml`/`.rs` → Rust, `.csproj`/`.sln`/`.cs` → C#, `go.mod`/`.go` → Go, in that precedence). Every language's file-level import map renders; the structural views branch on model content, not language — `scanner` renders whatever the model carries, `tree` needs a module tier (Go trees reach it through `go.work` members; a single-module Go tree is refused by that missing fact, see `errors.md`).
- The scan covers the whole directory tree beneath the given path.
- Generated, vendored, and hidden directories are excluded from the scan (see `output.md` / `acceptance.md`), mirroring the matching `scan` driver's rule for that language through one shared predicate: for Rust any directory named `target` or `vendor`, or starting with `.`, is skipped; for C# any directory named `bin`, `obj`, or `node_modules`, or starting with `.`, is skipped; for Go any directory named `vendor` or `testdata`, or starting with `.`, is skipped (the go driver's member list — `node_modules` and `target` are **not** Go exclusions). Everything beneath a skipped directory is skipped too.

```bash
# scans the current directory
archspec inspect

# scans the given directory tree
archspec inspect ./crates/auth

# scans the src/ tree of a crate
archspec inspect ./src
```

## Rejected input

| Input | Why rejected |
|---|---|
| path that does not exist | nothing to scan |
| path that is a regular file | only directories are scannable |
| directory with no Rust, C# or Go sources (recursively) | nothing to map |
| more than one positional argument | ambiguous target |

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: no such directory
archspec inspect /no/such/dir

# rejected: it is a file, not a directory
archspec inspect Cargo.toml

# rejected: no supported-language sources
archspec inspect ./docs

# rejected: two paths given
archspec inspect ./src ./tests
```

## Notes

- `Cargo.toml` is **not required**, but it is optionally read to resolve imports written through an own cargo target name (its `package`/`lib`/`[[bin]]` target names map those paths back into the crate). When it is absent — or names no matching target — such imports simply resolve to nothing; no error is raised, and `crate::`/`super::`/`self::` resolution is unaffected.
- No spec file, rules, or diagram configuration are read for the file-level graph. The mode keyword selects a view, not a configuration.
- The node set is the set of non-excluded source files. A directory whose only source files live under excluded paths has no scannable sources and is rejected (see `errors.md`).
