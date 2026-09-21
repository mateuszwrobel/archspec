# `doctor` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec doctor| D[doctor command]
  D --> R[probe each supported language driver]
  R --> REP[report per-driver availability and version detail]
  REP --> CALL[call out absent drivers with guidance]
  CALL --> SUM[summarize scannable languages]
  SUM --> OUT[stdout report]
  OUT --> U
  U -->|reads report| A{is the toolchain I need on PATH?}
  A -- yes --> B[run scan / verify / update / report]
  A -- no --> I[install the toolchain per guidance - scanning still runs on the in-binary driver]
  I --> U
```

Two intents drive `doctor`:

1. **Onboarding** — before first use of archspec, a developer runs `doctor` to see which toolchains for the languages they work with are present on `PATH`. The report describes the environment; scanning runs on the in-binary drivers either way. If a toolchain is missing, the report says what to install.
2. **Debugging a driver error** — `scan`, `verify`, `update`, or `report` fails with a "toolchain not found" error for some language. That error comes from the driver-availability check, which in phase 1 can only be tripped through the `ARCHSPEC_DISABLE_DRIVERS` test seam — every driver ships in the binary. The developer runs `doctor` to see what the environment actually exposes and what to install.

## Application flow

```
flowchart TD
  START[archspec doctor] --> CHK{can inspect own environment?}
  CHK -- no --> ERR[error message on stderr, non-zero exit]
  CHK -- yes --> PROBE[probe each supported driver: rust, csharp, go]
  PROBE --> DET[record availability and version/status detail per driver]
  DET --> CALL[call out absent drivers with guidance]
  CALL --> SUM[aggregate scannable languages]
  SUM --> REN[render canonical text report]
  REN --> OUT[stdout]
  OUT --> OK[exit 0]
```

The command enumerates the supported language drivers (`rust`, `csharp`, `go`), probes the environment for each, records presence and version detail, marks absent drivers with guidance, aggregates the scannable set, and writes a deterministic report to stdout. A run that cannot inspect its own environment fails with a message on stderr and a non-zero exit; every successful report exits `0` regardless of how many drivers are missing.
