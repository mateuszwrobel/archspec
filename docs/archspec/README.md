# archspec — Design

Multi-language architecture test & diagram tool. The living design (model, IR, ADRs) lives in `../archspec-design.md`; this folder holds **user-facing contracts** — one folder per command, plus a roadmap for planning features from behaviors.

## Commands

| Command | Purpose |
|---|---|
| `init` | Scaffold a minimal base spec |
| `scan` | Extract the architecture model only (no spec needed) |
| `diagram` | Render model (extracted or declared) to an artefact |
| `verify` | Extract + compare vs spec, full diff, exit code |
| `update` | Snapshot current model as a seed spec |
| `report` | Text/markdown diff + metric output |
| `spec` | Print the spec JSON schema (`--schema`) or annotated reference |
| `doctor` | Diagnose which language drivers/toolchains are present |
| `inspect` | Zero-config file-level import map (discovery) |
| `depgraph` | Model-faithful dependency views: [commands/depgraph/README.md](commands/depgraph/README.md) |
| `help` | Print the built-in manual: topics (commands, glob, spec, constraints, languages, workflow, diagnostics) and per-command help |
| `skill` | Print or install the agent-facing audit skill: [commands/skill/index.md](commands/skill/index.md) |

Each `commands/<command>/` folder holds that command's contracts (invocation, input, output, errors, flows, acceptance). `inspect` is the first command with fully planned behaviors. Per-language capability and skip reasons: [feature-matrix.md](../../feature-matrix.md).

`archspec capability matrix` (diagnostic surface, not a stable API) prints the driver capability table that `verify` and `doctor` read: [commands/capability/index.md](commands/capability/index.md), [commands/capability/acceptance.md](commands/capability/acceptance.md).

`archspec help roles` owns the per-view roles contract (which views mark roles) — run the command; this tree points there.

## Shared contracts

| File | Contract |
|---|---|---|
| `spec.md` | `architecture.spec.toml` format — used by `init`, `verify`, `update` |
| `examples.md` | Illustrative example specs — no runtime meaning (replaces the old profiles catalog) |
| `config.md` | `archspec.toml` output destinations — `inspect`, `diagram`, `report`, `scan` defaults |
| `help.md` | Tool-wide help: no-args listing, `--help`, per-command `--help` |
| `agents.md` | Guide for LLM agents / model-driven users: recipe order, built-in manual, worked prompts, real output shapes |

## Roadmap

`roadmap/` plans features **from behaviors** (user stories + acceptance). A feature is planned there first; when its behaviors are pinned down, they become contract files in the target command's folder. See `roadmap/README.md`.

## Licensing

The tool is licensed under the Apache License 2.0 (`../../LICENSE`); third-party dependency licenses are listed in the generated `../../CREDITS.md`.
