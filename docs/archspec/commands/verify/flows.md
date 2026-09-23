# `verify` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec verify [path] [--strict]| C[verify command]
  C --> E[extracts model from source]
  E --> D[reads declared spec]
  D --> V[compares structurally]
  V --> F{differs?}
  F -- yes --> R[full diff report, exit non-zero]
  F -- no --> P[short confirmation, exit 0]
  P --> U
  R --> U
  U -->|fixes code or spec| U
```

The developer runs `verify` after a change (or as a CI gate with `--strict`) and reads the verdict: a confirmation when code still satisfies the declared spec, or the complete diff report when it does not. On failure the developer fixes the code or the spec — everything that differs is in front of them, so the fix is one pass, not one iteration per error.

### Developer intent by flow

| Flow | Intent | Invocation |
|---|---|---|
| CI gate | fail the build on any divergence, warnings included | `archspec verify --strict` |
| dev loop | quick check after local changes, warnings tolerated | `archspec verify` |
| onboarding | first-time grounding of existing code | `archspec init` (scaffold) → `archspec update --force` (snapshot — plain `update` refuses the spec `init` just created) → `archspec verify` |

## Application flow

```
flowchart TD
  START[arg path, default cwd] --> SPEC{architecture.spec.toml?}
  SPEC -- no --> ERR1[error message, non-zero exit]
  SPEC -- yes --> VALID{spec valid TOML + schema?}
  VALID -- no --> ERR2[error naming file and field, non-zero exit]
  VALID -- yes --> SRC{supported-language sources?}
  SRC -- no --> ERR3[error message, non-zero exit]
  SRC -- yes --> DRV{driver/toolchain present?}
  DRV -- no --> ERR4[error, suggest archspec doctor, non-zero exit]
  DRV -- yes --> X[extract model from source]
  X --> PARSE{all sources parse?}
  PARSE -- no --> ERR5[error naming failing file, non-zero exit]
  PARSE -- yes --> M[map units onto declared components]
  M --> EX[extracted component model]
  DEC[declared component model from spec] --> CMP{set-based structural compare}
  EX --> CMP
  CMP -- equal --> PASS[short confirmation, exit 0]
  CMP -- differs --> DIFF[compute full diff report]
  DIFF --> R[classify each item by severity]
  R --> STR{--strict?}
  STR -- yes --> E2{any error or warning?}
  STR -- no --> N2{any error?}
  E2 -- yes --> FAIL[full diff report, exit non-zero]
  E2 -- no --> PASS2[short confirmation, exit 0]
  N2 -- yes --> FAIL
  N2 -- no --> PASS2
```

The command resolves the directory, requires and validates `architecture.spec.toml`, confirms sources and driver presence, extracts the model through the language driver for the declared language, parses every source file (a single failure aborts), maps each extracted unit onto the declared components, and compares the extracted and declared models as sets. It computes the **full** diff, classifies each item by severity, promotes warnings to errors when `--strict` is given, and prints either a short confirmation (exit `0`) or the complete diff report (exit non-zero).
