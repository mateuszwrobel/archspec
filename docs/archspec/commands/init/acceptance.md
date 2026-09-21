# `init` — Acceptance Tests (Phase 1)

CLI-level, behavioral only. Each scenario runs the real command against a fixture project and asserts on observable output (exit code, stdout/stderr, files created/changed). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `init` on a fixture project creates `architecture.spec.toml` with a `[project] language` header for the detected language plus the global `no_cycles` constraint, creates `archspec.toml` with the standard `[output]` defaults, prints a deterministic summary on stdout — and each error condition produces a clear message and a non-zero exit without creating or modifying any file.

## Happy path

| # | Given | When | Then |
|---|---|---|---|
| 1 | directory with `Cargo.toml` | run `init <dir>` | exit 0; `<dir>/architecture.spec.toml` and `<dir>/archspec.toml` created; stdout names the created files and language `rust` |
| 2 | directory with `go.mod` | run `init <dir>` | exit 0; spec created with `[project] language = "go"`; `archspec.toml` created |
| 3 | directory with a `*.csproj` file | run `init <dir>` | exit 0; spec created with `[project] language = "csharp"`; `archspec.toml` created |
| 4 | current directory is a Rust project | run `init` (no args) | exit 0; `architecture.spec.toml` and `archspec.toml` created in the current directory |
| 5 | directory with `Cargo.toml` | run `init <dir>` | exit 0; spec contains `[project]` and `language = "rust"`, **no** `[[module]]` or `[[stereotype]]` sections, and exactly one global `[[constraint]]` (`type = "no_cycles"`, no `modules` list) |
| 14 | directory with `Cargo.toml` | run `init <dir>` | exit 0; `archspec.toml` contains `[output]` with the standard `inspect`, `diagram`, `report`, and `scan` defaults under `docs/archspec/` |
| 16 | project whose crates form a dependency cycle | run `init <dir>`, add module declarations for the cyclic crates, run `verify <dir>` | non-zero exit; report names the cycle — the scaffolded guard is global, so new projects start cycle-clean |

## Determinism

| # | Given | When | Then |
|---|---|---|---|
| 6 | two identical Rust fixture projects | run `init` on each | both spec files byte-identical; both config files byte-identical; both stdout summaries byte-identical |
| 7 | fresh copy of the same project, unchanged | run `init <dir>` twice | both runs byte-identical spec file, config file, and summary |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 8 | `<dir>/architecture.spec.toml` already exists | run `init <dir>` | non-zero exit; message identifies the existing file; neither file is modified |
| 15 | `<dir>/archspec.toml` already exists | run `init <dir>` | non-zero exit; message identifies the existing file; neither file is modified |
| 9 | directory with no detectable language | run `init <dir>` | non-zero exit; message says the language was not detected; no file created |
| 10 | path does not exist | run `init <missing>` | non-zero exit; message identifies the invalid path |
| 11 | path is a regular file | run `init <file>` | non-zero exit; message says the path is not a directory |
| 12 | two positional paths given | run `init <a> <b>` | non-zero exit; message says at most one path accepted |
| 13 | target directory not writable | run `init <dir>` | non-zero exit; message says the spec could not be written; no file created |
