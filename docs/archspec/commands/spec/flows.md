# `spec` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec spec [--schema]| S[spec command]
  S -->|--schema given?| SCHEMA[print JSON Schema to stdout]
  S -->|no flags| REF[print annotated reference to stdout]
  SCHEMA --> U
  REF --> U
```

The developer asks for either the machine-readable schema (to validate specs in an editor, CI, or as an agent prompt) or the annotated reference (to author a spec by hand). Both land on stdout for piping, `>`-redirect to a file, or feeding to a validator.

## Application flow

```
flowchart TD
  START[parse args] --> FLAGS{known flags?}
  FLAGS -- no --> ERR1[error naming the flag, non-zero exit]
  FLAGS -- yes --> POS{positional args?}
  POS -- yes --> ERR2[error: takes no path, non-zero exit]
  POS -- no --> SCHEMA{--schema?}
  SCHEMA -- yes --> OUT1[serialize schema from shared required-field table, print to stdout]
  SCHEMA -- no --> OUT2[print embedded reference TOML to stdout]
  OUT1 --> EXIT0[exit 0]
  OUT2 --> EXIT0
```

The command parses and validates its flags, then prints either the schema built from the shared `CONSTRAINT_REQUIRED_FIELDS` table or the embedded reference constant. No project, spec file, or toolchain is touched. Launched through `ARCHSPEC_DOCTOR_FAIL`-style seams there are none — the output is fully deterministic.