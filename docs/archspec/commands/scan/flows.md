# `scan` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec scan [path]| C[scan command]
  C --> D[detect project language]
  D --> E[extract model via language driver]
  E --> M[canonical model IR]
  M --> O[stdout or --output file]
  O --> U
  U -->|reads raw model| W[write architecture.spec.toml]
  U -->|feeds| DI[diagram --source scan / update]
```

Developer runs `scan` on a project (or lets it default to the current directory), receives the raw canonical model — units, hard edges, usage, soft structure — and reads off how the codebase is actually structured before writing a spec. The model is also the feed for the rest of the pipeline: `update` snapshots it into a seed spec, `diagram --source scan` renders it. The tool offers the ground truth; the developer decides what to declare or draw.

## Application flow

```
flowchart TD
  START[arg path, default cwd] --> L{detect supported language?}
  L -- none --> ERR1[error message, non-zero exit]
  L -- detected --> DRV{driver / toolchain present?}
  DRV -- no --> ERR2[error naming language + run archspec doctor, non-zero exit]
  DRV -- yes --> EXT[extract units, hard edges, usage, soft structure]
  EXT --> P{all sources parse?}
  P -- no --> ERR3[error naming failing file, non-zero exit]
  P -- yes --> IR[canonical model IR]
  IR --> OUT[stdout or --output file]
  OUT --> OK[exit 0]
```

The command resolves the directory, detects the project language from manifests and sources, dispatches to the matching language driver (aborting with a `doctor` hint when that driver's toolchain is absent), extracts the model — units, hard edges, usage, and soft structure — parsing every source file, and fails the whole run if any file fails to parse. On success it serializes the canonical model to deterministic JSON and writes it to stdout or the `--output` file.
