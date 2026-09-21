# `report` — Flows

## User flow

### Reviewing architecture health in a PR or review discussion

```
flowchart LR
  U[Developer / reviewer] -->|archspec report [path]| C[report command]
  C --> X[extracts model + reads spec]
  X --> D[structural diff + metrics]
  D --> R[renders readable report]
  R --> M[stdout or --output file]
  M --> U
  U -->|reads diff + metrics| A[judges health, discusses PR]
```

The developer runs `report` on the project (or lets it default to the current directory), reads the diff and metrics, and argues from evidence: which edge is forbidden, which component went missing, how many cycles crept in. The tool offers the facts; the reviewer draws the verdict.

### Reading a committed report artefact

```
flowchart LR
  A[architecture-report.md in repo] -->|opened in review / CI output| R[Reviewer or agent]
  R -->|reads metrics + diff sections| D[judges architecture health offline]
```

A report written with `--output` and committed becomes a review artefact: any reviewer or coding agent opens the file later and reads the same byte-stable facts the reporter observed, without re-running the tool.

## Application flow

```
flowchart TD
  START[arg path, default cwd] --> D[locate project directory]
  D --> SP{spec file present?}
  SP -- no --> ERR1[error message, non-zero exit]
  SP -- yes --> SV[validate spec]
  SV --> SVP{spec valid?}
  SVP -- no --> ERR2[error naming file and reason, non-zero exit]
  SVP -- yes --> L[read declared language]
  L --> DRV{driver / toolchain available?}
  DRV -- no --> ERR3[error suggesting archspec doctor, non-zero exit]
  DRV -- yes --> X[extract model from source]
  X --> PF{all sources parse?}
  PF -- no --> ERR4[error naming failing file, non-zero exit]
  PF -- yes --> MAP[map units into declared components]
  MAP --> CMP[structural compare extracted vs declared]
  CMP --> MET[compute model metrics]
  MET --> REN[render diff + metrics in requested format]
  REN --> OUT[stdout or --output file]
  OUT --> OK[exit 0]
```

The command resolves the directory, reads and validates the spec, selects the extractor for the spec's declared language, extracts the model from the source tree, maps units onto the declared components, performs the full structural comparison, computes the model metrics, renders a deterministic report in the requested format, and writes it. The report is always emitted in full; afterwards an error-level violation exits non-zero, while a clean or warnings-only diff exits `0` (`report` is informational about warnings and has no `--strict` gate). Only operational failures skip the report and exit non-zero.
