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

The module walk resolves a file-backed `mod` through the same candidates rustc uses: an unconditional `#[path = "..."]` names the one required file, a `#[cfg_attr(pred, path = "...")]` contributes each attributed target as a candidate behind which the conventional `<dir>/name.rs` / `<dir>/name/mod.rs` variants remain. The scanner never evaluates cfg predicates: resolution is candidate existence in declaration order, the conventional variants last. The scan's file graph keys modules by location; the *model* walk honours these attributes.

| # | Given | When | Then |
|---|---|---|---|
| 39 | Rust crate whose root declares `#[path = "misc/thing.rs"] pub mod relocated;` (target file declares `pub mod inner;`) **and** a file-backed `mod missing;` resolving to no file | run `scan` | exit 0; `relocated::inner`, read from the `#[path]` target, appears in `soft_structure` and its references produce `module_edges`; `missing` resolves to no candidate and is recorded as an unresolved declaration (surfaced by `verify` as a warning) rather than silently dropped |
| 40 | Rust crate whose root declares `#[cfg_attr(unix, path = "os/unix.rs")] #[cfg_attr(windows, path = "os/windows.rs")] pub mod os;` with one of those files present | run `scan` | exit 0; the module resolves through the first existing `cfg_attr` path candidate in declaration order (falling back to the conventional file when no attributed candidate file exists), so `os` and its contents join `soft_structure`/`module_edges` and it is not reported unresolved |

## Root glob public-API exports

Root `pub use module::*;` re-exports are classified three ways at scan time: a resolvable same-crate glob is enumerated into `root_public_exports` (checked like named exports), an unresolvable one is a token in `root_glob_exports` (unverifiable), and a chain is only as resolvable as its weakest link.

| # | Given | When | Then |
|---|---|---|---|
| 41 | Rust crate whose root re-exports through a glob chain, e.g. `pub use facade::*;` where `facade` re-exports `pub use self::sub::*;` (or hops a `#[path] mod` / a `super::` link) reaching a same-crate module with public items | run `scan` | exit 0; `root_public_exports` lists the union of names enumerated by following the chain — each link located through `module_file_candidates` (honouring `#[path]`) with `super::` resolved from the current module and a visited set terminating cycles; a chain with a cfg-blocked, unlocatable, or non-local link poisons the whole chain, landing its glob token in `root_glob_exports` instead |
| 42 | Rust crate whose root re-exports a same-crate module's public items via `pub use module::*;`, plus a `pub use serde::*;` glob of an external crate | run `scan` | exit 0; `root_public_exports` enumerates `module`'s public item names (checked like named exports); the external glob's token appears in `root_glob_exports` as unverifiable, never guessed |

## Structural roles

`roles` is the serialized roles map of the model: model path -> role, drawn from the closed vocabulary {`facade`, `composition`}. Each driver derives entries from its own facts and every consumer reads the map instead of re-deriving it. A module or tree whose driver cannot derive a role carries no entry — and an empty map serializes to no `roles` key at all — so absence states "no role stated", never "role denied".

| # | Given | When | Then |
|---|---|---|---|
| 59 | Rust crate whose `lib.rs` defines nothing (only `mod` declarations and re-exports) plus a bin target whose `main.rs` wires modules (`use crate::…` into the modules it glues) | run `scan` | exit 0; JSON `roles` maps the crate-root path (`app`) to `facade` and the bin main root path (`<bin-unit>::main`) to `composition`; modules with no derivable role carry no entry, and a driver that states no roles emits no `roles` key |
| 60 | The canonical probe tree per language (rust: mod-declaration-only roots; c#: entrypoint project with DI-registration-family calls in `Program.cs`; go: unit whose root file declares `package main` with `func main`) | run `scan` | exit 0; JSON `roles` states exactly the entries that driver derives from its own facts — `facade` on the rust unit roots, `composition` on the c# entrypoint root and the go main-package unit path — and every value drawn from the closed vocabulary {`facade`, `composition`} |
| 61 | Go tree (driver whose capability row marks `role-facade` not-emitted) with a `package main` wiring unit | run `scan` | exit 0; no `facade` value appears anywhere in `roles` while the composition entry for the main package stays stated — absence per path, never a stand-in for the whole tree |

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
| 6 | project with a model | run `scan --output model.json` | exit 0; `model.json` contains the model; stdout is `wrote model.json` |
| 7 | same project, unchanged | run `scan` twice | both runs byte-identical output |
| 43 | fixture tree of any supported language and committed golden artefacts under the test suite (`tests/goldens/<command>/`) | run `scan`, `update`, `report --format json`, `depgraph modules` and `inspect tree` | exit codes unchanged; each command's stdout — or the seed spec `update` writes, or the refusal sentence where the module tier is absent — is byte-identical to its golden; the goldens pin the sole extraction path |
| 47 | C# file with a type-targeted using (`using App.Core.Engine;`, `Engine` a TYPE inside the declared namespace `App.Core`) | run `scan` | the module edge is addressed at the deepest declared namespace (`App::Core`) carrying the type tail `Engine` as a referenced symbol | pass |
| 48 | C# files with namespace-less, alias, `static` and `global::` usings | run `scan` | all forms share one addressing rule — endpoint is the deepest declared namespace (the same source module the reference is attributed to), remaining dotted segments are symbols; `var` resource acquisitions and lambdas add no edges | pass |
| 49 | C# tree whose cross-namespace usings target only NuGet and BCL namespaces (plain, alias and `global::` forms; no declared namespace owns a target) | run `scan` | `module_edges` is empty (the external tier is reached through the declared-namespace ownership miss — no target addressed as a module endpoint); `global::`-anchored targets are normalized and attributed like any other target | pass |
| 52 | C# tree referencing declared types purely in TYPE POSITIONS without any using (`: App.Core.IBase`, `new App.Core.Engine()`, `(App.Core.Widget)o`, `[App.Core.Attr]`, method parameter and return types, `App.Core.Wrapper<App.Core.Inner>`, field and property types) | run `scan` | every referenced type records the module edge to the owner of the deepest declared prefix (`App::Core`) carrying the referenced type name as a symbol (one deduped edge per from/to pair, symbols sorted) | pass |
| 53 | C# tree mixing a bare name under a namespace using (`using App.Core;` + `new Engine()`), locals and instance chains (`var timer`, `timer.Tick()`, `Console.WriteLine`), using-directive text inside a string literal, and BCL types referenced without a using | run `scan` | the bare name resolves through exactly its file's visibility (using'd namespaces + enclosing namespace) iff that prefix DECLARES the identifier as a TYPE — same edge and symbol as the qualified form, deduped; locals, instance chains, `var` and string content add nothing; type-position references never add `module_external` entries | pass |
| 54 | C# class declaring an injected field of a declared-namespace interface type — qualified (`private readonly App.Core.IEngine _engine;`), bare under `using App.Core;`, or a constructor parameter assigned to the field — with any number of calls through the field (`_engine.Run()`) | run `scan` | the field TYPE POSITION records the module edge to the owner of the deepest declared prefix (`App::Core`) carrying the interface type as the only symbol; chains reconstructed through the field root at a non-declared identifier (`_engine.Run`) contribute no edge and no symbol (the anchored-prefix rule), so the edge appears ONCE regardless of call count and method names never enter the symbol set | pass |
| 55 | ONE C# file declaring TWO block-scoped namespaces with `using` directives placed ABOVE the first namespace | run `scan` | each compilation-unit using is attributed to EVERY namespace the file declares (visible to all of them), one edge per (namespace, target) pair addressed like any other using (deepest declared prefix, tail segments as symbols), and the distributed using is visible to bare names in the later namespaces; a file with exactly ONE namespace folds its root usings into that namespace | pass |
| 56 | Go module (`example.com/demo`) whose package `store` imports sibling package `format` (plain, aliased, blank and dot specs alike) | run `scan` | the module edge `example.com::demo::store -> example.com::demo::format` (full import paths with `/` -> `::`, the go.work-edge vocabulary) with `unit` = the importing module `example.com/demo`, duplicates across import blocks and files recorded once and nested packages addressed at their full path depth; stdlib and external imports contribute no module edge and attribute through the SAME shared rules, `*_test.go` imports stay excluded, and a `go.work` tree with 2+ members takes the workspace rules of row 58 | pass |
| 57 | Go package `store` importing sibling `format` whose code calls `format.Title()` and `format.helper()` — plus aliased, blank and dot imports, chained calls, method calls on local values, struct literals with qualified types, and stdlib/external selectors | run `scan` | the exported selector symbols sit on the edge THAT import created: the selector identifier following a package qualifier (the package's own name or an import alias) joins the edge's symbol set when its first character is uppercase (`Title` yes, `helper` no — casing is the visibility rule, no config), sets dedup and sort across files and call sites while the edge itself exists exactly once; noise guards: a selector whose operand identifier is a local variable attributes nothing, a chained call's trailing selector (operand is a `call_expression`) attributes nothing — only the first matched selector counts — struct-literal field keys and qualified TYPE references are grammar-disjoint from selectors (`keyed_element` keys, `qualified_type` nodes) and contribute no symbols, a dot-imported package's calls are bare identifiers indistinguishable from local ones so its edge stays symbolless (the anchored-prefix philosophy of the csharp bare-name rule), and a blank import binds no referenceable name at all; selectors qualified by stdlib or external packages add nothing module-side — the external tier stays import-driven | pass |
| 58 | `go.work` workspace whose members `example.com/api` and `example.com/store` import each other's packages (cross-member import with an exported and an unexported selector call site, plus a WITHIN-member import of the member's own root package) | run `scan` | every cross-member module edge keeps `unit` = importing member and `from`/`to` import-path endpoints (`/` -> `::`), carrying the exported selector symbols through the row 57 casing and merge rules (cross-member AND within-member edges alike; selectors against stdlib or external qualifiers add nothing); within-member package imports gain module edges addressed exactly like the single-module path of row 56 (same `dotted_module` vocabulary, `unit` = the member module), while the unit-tier edges, units, external tier and `soft_structure` keep their unit-level rules; go.work discovery/validation errors are extraction facts, and error tolerance follows the grammar policy of the single-module path | pass |

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
| 36 | `--output` destination already holds the generated model | run `scan --output <f> --check` | exit 0; file untouched; stdout is `ok: <f> up to date` |
| 37 | `--output` destination differs from the generated model | run `scan --output <f> --check` | non-zero exit; message says it differs and names the regenerate command; file untouched |
| 38 | no `[output] scan` configured and no `--output` | run `scan --check` | non-zero exit; message says `--check` requires an output destination |
