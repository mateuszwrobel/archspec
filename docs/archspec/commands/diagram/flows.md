# `diagram` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec diagram [path]| C[diagram command]
  C --> S[load model: declared spec or scan snapshot]
  S --> R[render diagram]
  R --> O[stdout or --output file]
  O --> M[diagram artefact]
  M --> U
  U -->|reviews / commits| A[architecture is visible, diffable in the repo]
```

Developer runs `diagram` on the declared spec (default) to preview the governed architecture, or on a scan snapshot to see the current-state model. The rendered artefact is committed to the repository and reviewed — architecture changes show up as diffs in the diagram, the same way code changes show up as diffs in source.

Two driving intents:

- **Plan first** — write the spec, render the diagram, review the target architecture before writing code. Forbidden edges appear marked (dashed/red) as the explicit no-go set.
- **Review drift** — render the extracted model from a scan snapshot, spot the edges that violate the governing spec (marked dashed/red), and fix code or spec accordingly.

## Application flow

```
flowchart TD
  START[args: path default cwd, source, format, output] --> SRC{--source?}
  SRC -- spec --> D[locate project dir]
  D --> E{path is a directory?}
  E -- no --> ERR1[error naming path, non-zero exit]
  E -- yes --> F{architecture.spec.toml present?}
  F -- no --> ERR2[error: no spec found, non-zero exit]
  F -- yes --> P[parse spec]
  P --> G{parses?}
  G -- no --> ERR3[error naming spec file, non-zero exit]
  G -- yes --> DECL[build declared model]
  SRC -- scan --> A[resolve artefact path]
  A --> B{artefact exists?}
  B -- no --> ERR4[error naming artefact, non-zero exit]
  B -- yes --> V{valid snapshot?}
  V -- no --> ERR5[error naming artefact, non-zero exit]
  V -- yes --> EXT[build extracted model]
  EXT --> S{governing spec present?}
  S -- yes --> VIO[mark edges violating declared constraints]
  VIO --> C
  S -- no --> C
  DECL --> C{model has content?}
  C -- no --> ERR6[error: no model content, non-zero exit]
  C -- yes --> MARK[mark forbidden/violating edges dashed/red]
  MARK --> CL[group external crates into separate cluster]
  CL --> REN[render canonical Mermaid or PlantUML]
  REN --> OUT[stdout or --output file]
  OUT --> OK[exit 0]
```

The command resolves the model source, loads the model (parsing the declared spec, or validating the scan snapshot), rejects unreadable or empty models, marks edges that violate the declared constraints, groups external crates into their own cluster, renders a deterministic diagram in the requested format, and writes it. On any error nothing is written and the exit is non-zero — a valid run never leaves a partial diagram behind.
