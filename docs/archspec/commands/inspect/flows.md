# `inspect` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec inspect [tree|scanner] [path]| C[inspect command]
  C --> S[scans source tree]
  S --> R[resolves imports + mod declarations between files]
  R --> G[groups nodes into folder subgraphs + layout links]
  G --> O[renders diagram]
  O --> M[stdout or --output file]
  M --> U
  U -->|reads diagram| A[spots tangles, hubs, existing boundaries]
```

Developer runs `inspect` on a crate (or lets it default to the current directory), optionally in `tree`/`scanner` mode to see the extracted model rather than the raw file tree, receives a diagram grouped by folders, opens it, and reads off where the tangles are, whether folder boundaries already hold, and which files import from everywhere. The tool offers the graph; the developer draws the conclusions.

## Application flow

```
flowchart TD
  START[arg path, default cwd] --> MODE{mode keyword?}
  MODE -- "tree / scanner" --> SDET[locate source root]
  SDET --> SDRIVER{scan driver available?}
  SDRIVER -- no --> ERRD[error naming language + doctor, non-zero exit]
  SDRIVER -- yes --> SEXP[scan::extract → model]
  SEXP --> SMODEL[render model by unit + module; scanner adds cross-unit edges]
  SMODEL --> OUT[stdout or --output file]
  MODE -- "none (file-level)" --> D[locate source root]
  D --> E{has Rust/C#/Go sources?}
  E -- no --> ERR1[error message, non-zero exit]
  E -- yes --> W[collect all source files under root]
  W --> P[parse each file]
  P --> F{all files parse?}
  F -- no --> ERR2[error naming failing file, non-zero exit]
  F -- yes --> T[build module-to-file ownership table + own-target index]
  T --> X[resolve each import and mod declaration to target file]
  X --> GR[build file-level graph]
  GR --> SUB[group nodes into folder subgraphs + layout links]
  SUB --> REN[render canonical Mermaid or PlantUML]
  REN --> OUT
  OUT --> OK[exit 0]
```

The command branches on its mode keyword. The default mode resolves the directory, walks it for Rust/C#/Go sources, parses every file, builds a table mapping each module to its owning file (plus an own-target index read from `Cargo.toml`), resolves every import and `mod` declaration to its concrete target file (including submodules declared inline and `mod.rs` layouts), assembles the file graph, groups nodes by folder, lays them out with invisible `~~~` links, renders a deterministic diagram in the requested format, and writes it. The Go path reuses the scan driver's tokenizer and module-path resolution (`go.mod`, `go.work` members), resolves an import that falls under an own module path to the production files declaring that package — test files are nodes and import sources, never targets — and drops external/stdlib/cgo targets. The `tree`/`scanner` modes instead detect the language, gate on the `scan` driver, run the shared `scan::extract`, and branch on the model's content, not its language: `scanner` renders whatever units, edges and soft structure the model carries, while `tree` projects the module tier and is refused with the module-tier sentence when the model has none — a Go tree whose packages record no reference to each other, mirroring the `depgraph` refusal; Go `go.work` trees render containment here, in the requested format like every other mode.
