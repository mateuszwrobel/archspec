# ADR 014: Syntax-tree extraction back-ends behind a `--backend` flag

## Status
Accepted → **Superseded by ADR-016** (2026-09-22): syntax extraction became the
sole path; the `--backend` flag, the classic back-end and the back-end parity
contract are removed. The body below stands as the record of the interim
design.

## Context
The C# and Go scan drivers are hand-rolled byte tokenizers that recognize only
`namespace`/`using` (C#) and `import` blocks (Go). This caps what those drivers
can report at the module tier: C# emits `module_edges` only from `using`
directives with empty `symbols`; Go emits no intra-module edges at all and
never emits `symbols`. The rust driver, built on `syn`, reports both — edges
from qualified references and symbols from import tails. The asymmetry blocks
the goal of auditing module boundaries as public-API surfaces: an agent reading
scan output for a C# or Go tree cannot see *which* names cross a boundary, or
that a boundary crossed through a type position (base type, cast, `new`,
attribute, injected field) rather than a `using`.

ADR-012 revised 2026-08 considered tree-sitter for C#/Go and blocked it
("no symbols, fails phase 2"), then shipped in-binary tokenizers instead of the
plugin-driver design it accepted. Grammar-based extraction with
`tree-sitter-c-sharp` has since been proven workable inside this repository
family's tooling at tree-sitter 0.24 + grammar 0.23, and its blocked reason —
grammar-only parsing lacks semantic symbol resolution — applies equally to the
rust driver, which resolves names by module-prefix heuristics over `syn` ASTs,
not by compilation. Parity therefore needs syntax trees, not compilers.

## Decision
Add a selectable extraction **back-end** to every command that extracts a
model: `--backend classic|syntax`, default `classic`.

- `classic` — today's driver behaviour, byte-for-byte unchanged. Tokenizers for
  C#/Go, `syn` for Rust.
- `syntax` — syntax-tree substrate back-end. Rust: identical facts to `classic`
  (`syn` *is* its syntax substrate; no tree-sitter Rust path exists). C#:
  `tree-sitter-c-sharp` — edges from `using` *and* type positions (base types,
  `new`, casts, attributes, generics, parameter/return types) plus field-type
  linking for DI-style usage, with non-empty `symbols`. Go: `tree-sitter-go` —
  intra-module package edges plus selector symbols (capitalized selectors).

Grammars `tree-sitter`, `tree-sitter-c-sharp`, `tree-sitter-go` are compiled
in-binary, pinned to tree-sitter 0.24 with grammar crates 0.23 (the combination
already proven by in-repo grammar tooling; the Go grammar itself is new to the
family and its build is proven by the packaging gate before merge). The shared
grammar-registry/parse/error-tolerance layer is a thin in-tree module,
deliberately duplicated from the repository family's other grammar tool rather
than extracted into a shared crate: archspec's payload is leak-guarded and
publishable, so any shared crate would have to go to crates.io — machinery cost
far above the ~200-line overlap. No external toolchain process (Roslyn,
`go/packages`) — that rejection from ADR-012 stands.

This **amends ADR-012**: its Implementation Status paragraph gains the
syntax-back-end sentence in the same change, and it remains true otherwise —
C#/Go substrate is in-binary syntax trees behind a flag (not tokenizers, not
plugin-driver processes), unit-level C# ground truth stays with project
references, and the syntax back-end adds *module-tier* facts only. The flag
default flips to `syntax` per language by a later decision, recorded against
the own-application audit artefacts the companion workplan produces.

## Consequences
- Binary grows: tree-sitter C runtime + two grammars compiled statically. The
  Arch packaging gate must prove x86_64 and aarch64 payload staticness with the
  new C sources present.
- `capability` and the built-in help gain a per-language back-end matrix; the
  feature matrix gains a `scan.backend_syntax` capability row.
- C# `symbols not-emitted` and the Go module tier's `WORKTIER_ONLY` status
  become back-end-qualified, not driver-wide statements; depgraph acceptance
  text and help prose about symbolless csharp edges and go.work-gated module
  tiers is scoped to `classic`.
- Two extraction paths per language until default flip + removal — inspect↔scan
  agreement invariants must hold for each back-end separately.
- Nothing published: no tags, no release, payload stays ahead of the last
  release until the audit decision lands.

## Alternatives considered
- **Roslyn / `go/packages` subprocesses** — true symbol resolution, but adds
  managed-runtime hosting or toolchain dependence to a zero-toolchain binary;
  rejected by ADR-012, still rejected.
- **tree-sitter-rust replacing `syn`** — parity theatre: `syn` already gives
  typed path positions; swapping loses a maintained full parser for a C grammar
  with no fact gain.
- **Shared in-repo crate with the family's other grammar tool** — blocked by
  the public-payload leak
  guard unless published to crates.io; the shareable surface is thin (registry +
  parse helpers), duplication cost is lower than a fifth publish pipeline.
- **Silent replacement (no flag)** — no rollback lever for an agent's model
  diff while the syntax facts are unaudited; flag keeps `classic` reproducible.
