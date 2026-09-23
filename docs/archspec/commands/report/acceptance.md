# `report` — Acceptance Tests

CLI-level, behavioral only. Each scenario runs the real command against a fixture project and asserts on observable output (exit code, stdout/stderr, written files). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `report` on a fixture project with a valid `architecture.spec.toml` writes a deterministic diff + metrics report (text by default, Markdown via `--format markdown`, machine-readable JSON via `--format json`) covering the full structural difference and the model metrics, exits `0` when the code satisfies its spec and non-zero on error-level violations, writes to stdout by default or to a file via `--output`, and every operational failure produces a clear message and a non-zero exit.

## Happy path

| # | Given | When | Then |
|---|---|---|---|
| 1 | project with a valid spec; sources match the spec exactly | run `report` | exit 0; stdout report with metrics and a no-violations statement |
| 2 | project with a valid spec; sources match the spec exactly | run `report --format markdown` | exit 0; Markdown report on stdout with the same metrics and diff content as the text run |
| 3 | project with a forbidden edge present in the code | run `report` | exit non-zero; report lists the forbidden edge and counts it under violations by category |
| 4 | project with a forbidden edge present in the code | run `report --output report.md` | exit non-zero; `report.md` contains the report; stdout is `wrote report.md` |

## Exit code

| # | Given | When | Then |
|---|---|---|---|
| 5 | project whose code violates several spec rules | run `report` | non-zero exit — the full report is still emitted first; error-level violations gate the run |

## Determinism

| # | Given | When | Then |
|---|---|---|---|
| 6 | same project, unchanged | run `report` twice | both runs produce byte-identical stdout |
| 7 | same project, unchanged | run `report --format markdown --output r.md` twice | both runs produce byte-identical `r.md` |

## Metrics and diff content

| # | Given | When | Then |
|---|---|---|---|
| 8 | project with two components whose units have internal and external edges | run `report` | exit 0; report shows component count, unit count, and internal and external edge counts |
| 9 | project where a declared component has no matching units | run `report` | exit non-zero; report lists the missing component |
| 10 | project with a unit not covered by any declared component | run `report` | exit non-zero; report lists the unassigned unit |
| 11 | project with a cycle among components | run `report` | exit non-zero; report shows the cycle detected and counts it under violations by category |
| 22 | spec declares module boundaries over a single crate; the source satisfies it | run `report` | exit 0; `components` counts the declared module boundaries (not 1); `edges (internal)` counts each distinct dependency pair at unit tier |
| 23 | spec declares module boundaries; the source has an edge whose endpoint is the crate root | run `report` | exit 0; a module edge whose endpoints share one unit projects onto a unit self-pair and adds nothing to the internal pair count — never external, never dropped from the model or module views |
| 24 | single-crate project with two modules `a` and `b` each declared as its own boundary; `a -> b` edge exists | run `report` | exit 0; the unit-internal `a -> b` edge adds nothing to the internal pair count and must NOT surface as external — internal/external is keyed on the owning unit, not on boundary-declaration granularity |
| 25 | single-crate project with internal module edges and N true external crates | run `report` | exit 0; `edges_internal` is the unit-tier pair count (unit-internal module edges add nothing); `edges_external` reflects the true external-crate edges, and the two are distinguishable (not conflated) |
| 36 | project with a vacuous constraint (empty effective domain) | run `report` | exit 0; report lists the vacuous constraint line as content |
| 37 | project with a dead reference (`allowed.depend_on` target names no declared module) | run `report` | exit 0; report lists the dead reference line as content |

## Markdown report enrichment

| # | Given | When | Then |
|---|---|---|---|
| 26 | project with a valid spec and matching sources | run `report --format markdown` | exit 0; markdown contains a `## Diagram` section with a Mermaid code fence |
| 27 | project with a forbidden edge present in the code | run `report --format markdown` | exit non-zero; markdown contains a `## Forbidden imports` section listing the edge |
| 28 | project with a cycle among components | run `report --format markdown` | exit non-zero; markdown contains a `## Cycles` section listing the cycle |
| 29 | project with a valid spec and matching sources | run `report` (text) | exit 0; text output has no `## Diagram`, `## Forbidden imports`, or `## Cycles` sections |

## JSON format

| # | Given | When | Then |
|---|---|---|---|
| 38 | project with at least one error-level violation | run `report --format json` | exit non-zero; stdout parses as JSON; the `findings` array carries the same (category, message) set as the text run's diff section, with `severity: "error"` |
| 39 | project with a valid spec; sources match the spec exactly | run `report --format json` | exit 0; parses as JSON with an empty `findings` array; `metrics` mirror the scan model tiers |
| 40 | project with a warnings-only diff | run `report --format json` | exit 0; the findings carry `severity: "warning"` |
| 41 | rust, csharp, and go projects with a violation each | run `report --format json` on all three | each parses as JSON with populated findings and the identical schema — no driver-specific shape |

## Roles

| # | Given | When | Then |
|---|---|---|---|
| 42 | canonical probe tree per language whose scan derives roles (rust: `facade` roots; c#: `composition` entrypoint root; go: `composition` main-package unit) | run `report` (text) then `report --format json` | text output carries a `Roles` section with one row per populated closed-vocabulary group (`facades: <paths>` / `composition roots: <paths>`, no row for an empty group); the json output's `roles` object equals the scan model's roles map key for key — a restatement, not a second derivation |

## Config default destination

| # | Given | When | Then |
|---|---|---|---|
| 30 | project with valid spec and `archspec.toml` with `[output] report = "docs/archspec/report.md"` | run `report` | exit 0; report written to `docs/archspec/report.md`; stdout is `wrote docs/archspec/report.md` |
| 31 | project with valid spec and a config default set | run `report --output custom.md` | exit 0; report written to `custom.md`; the configured path is not created |
| 32 | project with a malformed `archspec.toml` | run `report` | non-zero exit; message names the config file; no report written |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 12 | path does not exist | run `report <missing>` | non-zero exit; message identifies the invalid path |
| 13 | path is a regular file | run `report <file>` | non-zero exit; message identifies the invalid path |
| 14 | directory without `architecture.spec.toml` | run `report <dir>` | non-zero exit; message names the expected spec file |
| 15 | invalid spec file | run `report` | non-zero exit; message names the spec file and the reason |
| 16 | no sources in the spec's declared language | run `report <dir>` | non-zero exit; message says no sources found, the language, and where |
| 17 | source file with invalid syntax | run `report` | non-zero exit; message names the failing file; no report written |
| 18 | declared language's driver/toolchain missing | run `report` | non-zero exit; message names the language and suggests `archspec doctor` |
| 19 | unsupported format value | run `report --format html` | non-zero exit; message names the unsupported value |
| 20 | two positional paths given | run `report <a> <b>` | non-zero exit; message says at most one path accepted |
| 21 | source file with invalid syntax, `--output out.md` given | run `report --output out.md` | non-zero exit; `out.md` not created |

## Freshness (`--check`)

| # | Given | When | Then |
|---|---|---|---|
| 33 | `--output` destination already holds the generated report | run `report --output <f> --check` | exit 0; file untouched; stdout is `ok: <f> up to date` |
| 34 | `--output` destination differs from the generated report | run `report --output <f> --check` | non-zero exit; message says it differs and names the regenerate command; file untouched |
| 35 | no `[output] report` configured and no `--output` | run `report --check` | non-zero exit; message says `--check` requires an output destination |
