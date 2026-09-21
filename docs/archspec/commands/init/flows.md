# `init` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec init [path]| I[init command]
  I --> D[detect project language from manifests or sources]
  D --> S[write architecture.spec.toml and archspec.toml]
  S --> M[stdout summary]
  M --> U
  U -->|run archspec update| S2[seed boundaries from the tree]
  S2 -->|edits spec| E[adds module boundaries and relations]
  E -->|run archspec verify| VF[verify guards code against the spec]
```

The developer adopts archspec on an existing codebase: runs `init` from the project root or with an explicit path, receives a minimal `architecture.spec.toml` (`[project] language` plus the global cycle guard) and an `archspec.toml` output config plus a short summary, then seeds and shapes the spec. `update` (see `../../commands/update/`) snapshots the actual tree as a starting point; the user trims and edits it to express their real boundaries and relations, then enters the guard loop with `verify`. `init` is the entry point; the spec it writes is the source of truth for everything that follows.

## Application flow

```
flowchart TD
  START[arg path, default cwd] --> VAL{path is an existing directory?}
  VAL -- no --> ERR1[error message, non-zero exit]
  VAL -- yes --> EX{architecture.spec.toml present?}
  EX -- yes --> ERR2[already-exists error, non-zero exit, no write]
  EX -- no --> CFG{archspec.toml present?}
  CFG -- yes --> ERR2
  CFG -- no --> DET[detect language from manifests or sources]
  DET --> F{language detectable?}
  F -- no --> ERR3[not-detectable error, non-zero exit]
  F -- yes --> WR[write minimal base spec architecture.spec.toml]
  WR --> CFGW[write output config archspec.toml]
  CFGW --> SUM[print summary: files, language, next step]
  SUM --> OK[exit 0]
```

The command resolves the directory, refuses to clobber an existing spec or output config, detects the project language, writes the minimal base spec (`[project] language` plus the global `no_cycles` constraint) to `architecture.spec.toml`, writes the `[output]` config to `archspec.toml`, prints the summary, and exits. Every failure before the first write stops the run with nothing created or modified; only a config write that fails after the spec was created leaves that newly created spec on disk.
