# `inspect` — Import Resolution (What the Diagram Means)

The diagram shows **resolved** internal imports between files. This file defines what counts as an internal import and how it resolves to a concrete target file.

## What produces an edge

An edge `F → T` appears when file `F` contains an import that resolves to file `T`, **or** declares a file-backed module whose child file is `T` (a `mod x;` in `F` that resolves to `x.rs` / `x/mod.rs`). Edge direction is importing or declaring file → imported or declared file. A declaration edge is drawn per declaring→declared pair (collapsed, so a duplicate `mod` never doubles the edge).

An edge whose source is a production file **never targets a test-tier file** — in every language, because no production code can import a test file: go test files (`*_test.go`), C# files of a test project (classified by the `scan` driver's manifest predicate), and Rust test targets are nodes and import **sources** only. Their own imports of production code are real hypotheses and render; an import that would resolve a production file into a test file is a phantom — unreachable in Rust by ownership-key shape, and filtered from the target list in Go and C#.

Internal imports are `crate::`, `super::`, and `self::` paths (including `use` statements and inline paths), **plus paths written through one of the package's own cargo target names** read from `Cargo.toml` — e.g. `archspec::model::x` resolves in-crate exactly like `crate::model::x`, preferring the library target on a name collision. **Imports that name neither a local module nor an own cargo target — external crates — produce no edge and no node.**

## Resolution rules

| Import in file `F` | Resolves to |
|---|---|
| `use crate::a::b::X` | the file that owns module `a::b` |
| `use super::x::Y` | the file that owns sibling module `x` (same parent as `F`) |
| `use self::x::Z` | the file that owns submodule `x` of `F`'s module |
| `use archspec::a::b::X` where `archspec` is an own cargo target name | the file that owns module `a::b` (resolved through that target's module root, lib first) |
| references within `F`'s own module | `F` itself — no self-edge is drawn |
| a module path that ends in an inline `mod` | the file containing that inline `mod` |
| a file-backed `mod x;` declaration in `F` | the declared child module's file (`x.rs` / `x/mod.rs`) — a declaration edge |

## How module ownership is determined

A module is owned by exactly one file:

- A file at `src/a.rs` owns module `a`.
- A file at `src/a/mod.rs` owns module `a` (and its submodules are declared there).
- A file at `src/a/b.rs` owns module `a::b`.
- An inline `mod b { ... }` declared inside `src/a.rs` is owned by `src/a.rs` (module `a::b` lives in `a.rs`).

So `use crate::a::inner::Type` targets `src/a.rs` when `inner` is an inline module of `a`, and targets `src/a/inner.rs` (or `src/a/inner/mod.rs`) when it is a separate file.

## Module relocation

The file graph keys module ownership purely by file location — it never reads `#[path]` or `#[cfg_attr(..., path = "...")]`. A module relocated to an unusual file through one of those attributes is still looked up under its conventional `x.rs` / `x/mod.rs` path here, so such a `mod` may resolve to a different (or no) file than the `scan` driver reports. `scan`'s module walk honours `#[path]` and evaluates `cfg_attr` path candidates; `inspect`'s file graph does not. This is a deliberate divergence: `inspect` maps the on-disk folder layout, not rustc's module resolution.

## Folder grouping

Nodes are grouped into subgraphs that mirror the directory structure, recursively. A folder contributes a subgraph only if it contains files (directly or in nested folders). This is the "based on current folder structure" view: at a glance you can see whether cross-folder edges cross a boundary and where the tangles concentrate.

## Example

Source tree:

```
src/
├── lib.rs
├── commands.rs
└── orchestration/
    ├── mod.rs        (declares: mod common; mod pipeline;)
    ├── common.rs
    ├── pipeline.rs
    └── common/
        └── util.rs   (module common::util)
```

Imports:

```rust
// src/commands.rs
use crate::orchestration::pipeline::run;

// src/orchestration/pipeline.rs
use super::common::Node;            // sibling module common
use crate::orchestration::common::util::Id;  // module common::util

// src/orchestration/mod.rs
use self::common::util::Id;
```

Resulting edges:

```
src/commands.rs -> src/orchestration/pipeline.rs            (via crate::orchestration::pipeline)
src/orchestration/mod.rs -> src/orchestration/common.rs     (declaration: mod common;)
src/orchestration/mod.rs -> src/orchestration/common/util.rs (via self::common::util)
src/orchestration/mod.rs -> src/orchestration/pipeline.rs   (declaration: mod pipeline;)
src/orchestration/pipeline.rs -> src/orchestration/common.rs      (via super::common)
src/orchestration/pipeline.rs -> src/orchestration/common/util.rs (via crate::...::common::util)
```

No external-crate edges and no self-edges. `mod` declarations themselves **do** produce edges (declaring file → declared child file), which is why `src/orchestration/mod.rs` links to `common.rs` and `pipeline.rs` even though neither import names them.

## Go resolution

Go trees resolve imports by path rather than by module syntax, so the rules above apply to Rust; the C# and Go scanners follow the same shape with their own ownership sources. For Go, an `import` path is **internal** when it equals a module path of the tree or sits under one of its path prefixes — the module paths are exactly the ones the `scan` driver resolves for the same tree (`go.mod` module for a single module; every member `go.mod` of the root `go.work` otherwise), so `inspect` and `scan` never disagree on what "internal" means. An internal import resolves to the **production files declaring** the imported package: the package clause — not the directory layout — declares ownership, and the target list applies the scan driver's file-level test predicate, so an import of `example.com/gotree/store` fans out to the non-test files of the `store` package and never to a `*_test.go` file — in either package form (`package foo` or `package foo_test`), a test file owns no import path. Test files are nodes and import **sources** like any other file — their own imports of internal packages render, discovery deliberately includes what the guard model drops at scan, and a directory holding only test files simply owns no import path. Stdlib imports, third-party paths and the cgo marker (`import "C"`) sit under no own module path and render no edge and no node, and `//go:embed` directives are comments the tokenizer skips — neither is an internal dependency fact.
