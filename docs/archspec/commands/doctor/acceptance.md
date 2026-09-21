# `doctor` — Acceptance Tests (Phase 1)

CLI-level, behavioral only. Each scenario runs the real command against a controlled environment (a machine or container with a known set of toolchains) and asserts on observable output (exit code, stdout/stderr). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `doctor` reports, per supported language driver (rust, csharp, go), driver capability and toolchain availability as separate facts (driver present — the in-binary parse-only driver; toolchain present/absent with version detail), calls out only a truly disabled driver with guidance, and prints a summary of which languages are scannable by DRIVER — byte-stable for the same environment, and exits `0` whenever a report is produced. Only an invalid invocation or a failure to inspect the environment exits non-zero.

## Happy path

| # | Given | When | Then |
|---|---|---|---|
| 1 | environment with rust, csharp, and go toolchains present | run `doctor` | exit 0; report has a block for each of rust, csharp, go; driver present and toolchain present in each; summary lists rust, csharp, go |
| 2 | environment with only the rust toolchain present | run `doctor` | exit 0; rust toolchain present; csharp and go report driver present with toolchain absent (parse-only note) — the two facts stay independent; summary still lists rust, csharp, go |

## Missing-toolchain callout

| # | Given | When | Then |
|---|---|---|---|
| 3 | environment without a go toolchain | run `doctor` | exit 0; go block reports driver present, toolchain absent with the parse-only note, no install pressure; go stays in the summary |
| 4 | environment without any supported toolchain | run `doctor` | exit 0; every block reports driver present / toolchain absent with the note — scanning works from the in-binary drivers; summary lists rust, csharp, go |

## Determinism

| # | Given | When | Then |
|---|---|---|---|
| 5 | a fixed environment | run `doctor` twice | both runs byte-identical output |
| 6 | a fixed environment | run `doctor` from two different directories | both runs byte-identical output |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 7 | a positional path argument given | run `doctor <path>` | non-zero exit; message says a path is not accepted |
| 8 | an unknown flag given | run `doctor --bogus` | non-zero exit; message names the flag |
| 9 | environment that cannot be inspected — triggered deterministically via the `ARCHSPEC_DOCTOR_FAIL` test seam, since a probe failure alone degrades to an `absent` finding | run `doctor` | non-zero exit; message names the cause; no report on stdout |
