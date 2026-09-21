# `scan` — Acceptance Tests

CLI-level, behavioral only. Each scenario runs the real command against a fixture project and asserts on observable output (exit code, stdout/stderr, written files). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `scan` on a fixture project emits the canonical model as byte-stable JSON — units, hard edges, usage, and soft structure for the detected language — on stdout by default, or to the `--output` file with empty stdout — and every error condition produces a clear message and a non-zero exit.

## Happy path

| # | Given | When | Then |
|---|---|---|---|
| 1 | Rust workspace with crates `auth` and `billing`; `billing` depends on `auth` | run `scan` on the workspace root | exit 0; JSON on stdout; `language` is `rust`; lists both crates as units and the hard edge `billing -> auth` |
| 2 | C# solution with `Orders` and `Orders.Abstractions` projects; `Orders` references `Orders.Abstractions` | run `scan` on the solution root | exit 0; JSON lists both projects as units and the project reference as an edge |
| 3 | Go module with `cmd/` entry and an `internal/` privacy tree | run `scan` on the module root | exit 0; JSON reflects the packages and the `internal/` structure |
| 39 | Go workspace: root `go.work` with `use` entries naming two or more member modules | run `scan` on the workspace root | exit 0; each member module appears in the module tier (`soft_structure`) grouped with its packages, and cross-member imports are recorded as `module_edges` |

## Language detection

| # | Given | When | Then |
|---|---|---|---|
| 4 | Go module (`go.mod`) | run `scan` on the module root | exit 0; JSON `language` is `go` |
| 5 | tree with no Rust, C#, or Go sources or manifests | run `scan` | non-zero exit; message says no supported-language sources found |

## Module-level structure (soft boundaries)

Module-level structure is the namespace/folder/visibility groupings above and below the hard units. For Rust it is the `src/` module tree plus the `use crate::X` / `super::X` edges between modules; for C# the namespace/folder tree and `using` edges; for Go the `internal/` tree and import edges. Behaviors below are language-agnostic in wording but each is exercised on the relevant language fixture.

| # | Given | When | Then |
|---|---|---|---|
| 13 | Rust crate with `src/lib.rs` declaring `mod auth; mod billing;` | run `scan` on the crate root | exit 0; JSON `soft_structure` lists `auth` and `billing` as module nodes of the crate |
| 14 | Rust crate where `billing.rs` does `use crate::auth::Token;` | run `scan` on the crate root | exit 0; JSON records the module-level edge `billing -> auth` distinct from any hard unit edge |
| 15 | Rust crate with nested modules `auth/storage.rs` under `mod auth;` | run `scan` | exit 0; JSON `soft_structure` nests `auth::storage` under `auth` |
| 16 | Rust module file referencing a sibling via `super::` | run `scan` | exit 0; JSON records the module-level edge to the sibling module |
| 17 | Rust crate importing external crates (`use serde;`) | run `scan` | exit 0; JSON `external` lists `serde` (and other imported external crates) |
| 18 | Rust crate using an item from another module (`use crate::auth::Token;`) | run `scan` | exit 0; JSON `usage` records the symbol `Token` on the `billing -> auth` module edge |
| 19 | same module-structured project, unchanged | run `scan` twice | both runs byte-identical output including `soft_structure`, module edges, `external`, and `usage` |
| 22 | Rust crate with BOTH `lib.rs` and `main.rs`, where `main.rs` declares `mod archspec;` and structure lives under it | run `scan` | exit 0; the model captures module structure from BOTH the lib tree and the main tree (not just lib) |
| 23 | Rust crate where a module calls a sibling via a qualified path `audio::list_devices()` with no `use` | run `scan` | exit 0; JSON records a module edge for the qualified-path call (not only `use` statements) |
| 24 | Rust crate where `main.rs` references the crate's own lib via a qualified path `voice_app_lib::run()` | run `scan` | exit 0; JSON records the main→lib edge; the crate's own name is NOT listed in `external` |
| 25 | Rust crate with BOTH `lib.rs` and `main.rs` as separate compilation targets | run `scan` | exit 0; JSON lists TWO units (lib unit and bin unit), not one merged crate unit |
| 26 | Rust crate with `src/bin/tool.rs` (a separate binary target) | run `scan` | exit 0; JSON lists the binary as its own unit distinct from the crate |

## Module relocation (`#[path]` / `cfg_attr`)

The module walk resolves a file-backed `mod` through the same candidates rustc uses: an unconditional `#[path = "..."]` names the one required file, a `#[cfg_attr(pred, path = "...")]` contributes each attributed target as a candidate behind which the conventional `<dir>/name.rs` / `<dir>/name/mod.rs` variants remain. The scan's file graph keys modules by location; the *model* walk honours these attributes.

| # | Given | When | Then |
|---|---|---|---|
| 39 | Rust crate whose root declares `#[path = "misc/thing.rs"] pub mod relocated;` (target file declares `pub mod inner;`) **and** a file-backed `mod missing;` resolving to no file | run `scan` | exit 0; `relocated::inner`, read from the `#[path]` target, appears in `soft_structure` and its references produce `module_edges`; `missing` resolves to no candidate and is recorded as an unresolved declaration (surfaced by `verify` as a warning) rather than silently dropped |
| 40 | Rust crate whose root declares `#[cfg_attr(unix, path = "os/unix.rs")] #[cfg_attr(windows, path = "os/windows.rs")] pub mod os;` with one of those files present | run `scan` | exit 0; the module resolves through the first existing `cfg_attr` path candidate (falling back to the conventional file when none exists), so `os` and its contents join `soft_structure`/`module_edges` and it is not reported unresolved |

## Root glob public-API exports

Root `pub use module::*;` re-exports are classified three ways at scan time: a resolvable same-crate glob is enumerated into `root_public_exports` (checked like named exports), an unresolvable one is a token in `root_glob_exports` (unverifiable), and a chain is only as resolvable as its weakest link.

| # | Given | When | Then |
|---|---|---|---|
| 41 | Rust crate whose root re-exports through a glob chain, e.g. `pub use facade::*;` where `facade` re-exports `pub use self::sub::*;` (or hops a `#[path] mod` / a `super::` link) reaching a same-crate module with public items | run `scan` | exit 0; `root_public_exports` lists the union of names enumerated by following the chain — each link located through `module_file_candidates` (honouring `#[path]`) with `super::` resolved from the current module and a visited set terminating cycles; a chain with a cfg-blocked, unlocatable, or non-local link poisons the whole chain, landing its glob token in `root_glob_exports` instead |
| 42 | Rust crate whose root re-exports a same-crate module's public items via `pub use module::*;`, plus a `pub use serde::*;` glob of an external crate | run `scan` | exit 0; `root_public_exports` enumerates `module`'s public item names (checked like named exports); the external glob's token appears in `root_glob_exports` as unverifiable, never guessed |

## External dependencies

| # | Given | When | Then |
|---|---|---|---|
| 20 | Rust crate depending on third-party crates declared in `Cargo.toml` but not part of the workspace | run `scan` | exit 0; JSON `external` lists the third-party crates |
| 21 | workspace crate whose dependency is another workspace member | run `scan` | exit 0; the member is a hard edge, NOT listed in `external` |
| 27 | Rust crate importing the standard library (`use std::path::Path`) | run `scan` | exit 0; `std` is classified as stdlib, NOT listed as an external crate |
| 28 | Rust crate referencing its own modules via bare relative paths (`use asr_client::AsrTranscriber;` inside the crate) | run `scan` | exit 0; own module names are NOT listed in `external` |
| 29 | Rust crate that references a crate by its underscore name in source while the manifest declares it with a dash (`tower-http` in Cargo.toml, `tower_http` in `use`) | run `scan` | exit 0; the crate appears ONCE in `external`, dash/underscore normalized |
| 30 | Rust crate with dev-dependencies declared in `Cargo.toml` | run `scan` | exit 0; dev-dependencies are NOT mixed into the runtime `manifest.dependencies` / `external` lists |
| 31 | Rust crate whose `lib.rs` declares `mod tauri` under `#[cfg(feature = "tauri")]` | run `scan` | exit 0; the module declaration captures WHICH feature gates it (`tauri`), not just a `gated: true` flag |
| 32 | Rust crate whose manifest sets `publish = ["gitea"]` (a specific registry) | run `scan` | exit 0; `manifest.publish` reflects the actual declaration; `publish = false` is distinct from an absent `publish` field |
| 33 | Rust crate where `main.rs` calls `audio::list_devices()` (qualified path) and `use crate::tmux::send_keys` | run `scan` | exit 0; JSON records edges for BOTH the qualified-path call and the `use` import (module_edges not empty) |
| 34 | Rust crate referencing an external crate only via a fully-qualified path (`toml::from_str(...)` with no `use toml`) | run `scan` | exit 0; JSON lists the crate in `external` (fully-qualified path references index external crates) |
| 35 | Rust crate whose lib target uses a crate name different from the package name (`[lib] name = "voice_app_lib"`), and the bin references the lib via that crate name (`voice_app_lib::run()`) | run `scan` | exit 0; JSON records the bin→lib edge; the lib's crate name resolves to its unit |

## Output and determinism

| # | Given | When | Then |
|---|---|---|---|
| 6 | project with a model | run `scan --output model.json` | exit 0; `model.json` contains the model; stdout empty |
| 7 | same project, unchanged | run `scan` twice | both runs byte-identical output |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 8 | path does not exist | run `scan <missing>` | non-zero exit; message identifies the invalid path |
| 9 | path is a regular file | run `scan <file>` | non-zero exit; message identifies the invalid path |
| 10 | project containing a file with invalid syntax | run `scan` | non-zero exit; message names the failing file; no partial model on stdout or `--output` file |
| 11 | Go project but the Go driver/toolchain is unavailable | run `scan` | non-zero exit; message names the detected language and says to run `archspec doctor` |
| 12 | two positional paths given | run `scan <a> <b>` | non-zero exit; message says at most one path accepted |

## Freshness (`--check`)

| # | Given | When | Then |
|---|---|---|---|
| 36 | `--output` destination already holds the generated model | run `scan --output <f> --check` | exit 0; file untouched; stdout empty |
| 37 | `--output` destination differs from the generated model | run `scan --output <f> --check` | non-zero exit; message says it differs and names the regenerate command; file untouched |
| 38 | no `[output] scan` configured and no `--output` | run `scan --check` | non-zero exit; message says `--check` requires an output destination |
