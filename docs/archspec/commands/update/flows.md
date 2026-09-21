# `update` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec update [path] [--force]| C[update command]
  C --> S[extracts model from source tree]
  S --> W[writes architecture.spec.toml]
  W --> P[prints summary to stdout]
  P --> U
  U -->|reviews, trims, strengthens seed| E[guarded spec committed]
  E --> V[archspec verify enforces spec going forward]
```

Developer runs `update` on a project (or lets it default to the current directory), receives a seed `architecture.spec.toml` and a summary telling them what to do next. They review the seed, trim it to what the architecture should actually allow, strengthen constraints, and commit it — then `verify` enforces that spec against all future changes. The tool snapshots today's model; the developer decides tomorrow's boundaries.

## Application flow

```
flowchart TD
  START[arg path, default cwd] --> V{path is an existing directory?}
  V -- no --> ERR1[error naming the invalid path, non-zero exit]
  V -- yes --> E{spec already exists?}
  E -- yes, no --force --> ERR2[error naming spec file + --force, non-zero exit]
  E -- no or --force --> L{has supported-language sources?}
  L -- no --> ERR3[error: no sources found, non-zero exit]
  L -- yes --> T{driver available?}
  T -- no --> ERR4[error suggesting archspec doctor, non-zero exit]
  T -- yes --> P[parse every source file]
  P --> F{all files parse?}
  F -- no --> ERR5[error naming failing file, non-zero exit]
  F -- yes --> M[extract model: units and edges]
  M --> W[write architecture.spec.toml, overwriting when --force was given]
  W --> SUM[print summary: file path + review guidance]
  SUM --> OK[exit 0]
```

The command validates the path, refuses an existing spec unless `--force` is given, detects the language from its sources, confirms the language driver is available (every driver ships inside the binary — the driver-unavailable branch is reachable only through the `ARCHSPEC_DISABLE_DRIVERS` test seam), collects the supported-language sources, and parses every file — a parse failure aborts the run so no partial snapshot is produced. It extracts the model, units and their dependency edges, and writes it as a deterministic `architecture.spec.toml`, overwriting the existing one when `--force` was given. On success stdout carries a summary pointing the developer at review and `archspec verify`.
