# Roadmap

Plans archspec features **from behaviors**. A feature enters the roadmap as a user story with observable acceptance; when its behaviors are pinned down, they become contract files in the target command's folder (`../commands/<command>/`).

## Method

1. **Story** — user-facing outcome, no implementation detail.
2. **Acceptance** — observable behaviors (Given/When/Then at e2e level). This is the test contract.
3. **Contract files** — once behaviors are stable, write them into the command's folder (`cli.md`, `input.md`, `output.md`, `errors.md`, `flows.md`, `acceptance.md`), following the `inspect` pattern (`../commands/inspect/`).
4. **Implement** — later; each tool ships its own test tooling (no shared harness).

All phase-1 commands now have behavior contracts. `inspect` is the reference example of a fully behavior-planned command.

## Status by feature

| Feature | Command | Behaviors | Contracts |
|---|---|---|---|
| File import map (Rust), excluded paths (15–19) | `inspect` | implemented | `../commands/inspect/` |
| C# file import map | `inspect` | implemented | `../commands/inspect/` |
| Model extraction (units + module-level tiers) | `scan` | implemented | `../commands/scan/` |
| Minimal base spec scaffold | `init` | implemented | `../commands/init/` |
| Diagram render | `diagram` | implemented | `../commands/diagram/` |
| Spec verify (7 constraint types, shape-agnostic) | `verify` | implemented | `../commands/verify/` |
| Seed spec snapshot (boundaries from tree) | `update` | implemented | `../commands/update/` |
| Diff + metrics report | `report` | implemented | `../commands/report/` |
| Driver diagnostics | `doctor` | implemented | `../commands/doctor/` |
| Tool-wide and per-command help | all | implemented | `../help.md` |
| Configurable output destinations (`archspec.toml`) | inspect, diagram, report, scan | implemented | `../config.md` |
| Report with embedded diagram + marked violations | `report` | implemented | `../commands/report/` |
| Model-faithful dependency views (modules, api-usage, submodules) | `depgraph` | implemented | `../commands/depgraph/` |

## Planning notes

- Behavior-first: acceptance tests are designed before implementation; flows are represented as diagrams *and* tests (see `../commands/inspect/flows.md`, `../commands/inspect/acceptance.md`).
- The spec is **shape-agnostic** (see `../spec.md`): the same declaration works for a single-crate, multi-crate, or hybrid tree. `matches.units` targets the hard tier; `matches.modules` targets module paths inside units. Module-level extraction (`scan` soft tier) and the extended constraint types (`verify`) enable module-granularity boundaries inside a crate — which is what lets the CLI verify a single-crate modular repository, archspec's own included.
- Profiles were removed from the design (no runtime recognition); `examples.md` holds illustrative starter specs only.
- Phase-2 scope (Go package-level `inspect`; more languages' unit/module mapping) is tracked separately.
