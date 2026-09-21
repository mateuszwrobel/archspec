# `spec` — Acceptance Tests

CLI-level, behavioral only. Each scenario runs the real command against a temp directory and asserts on observable output (exit code, stdout/stderr). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `archspec spec --schema` prints valid JSON (a draft-07 schema for `architecture.spec.toml`) to stdout and exits `0`; `archspec spec` prints an annotated reference TOML covering every section and exits `0`; the schema's per-type required arrays exactly match what `verify`'s runtime validation enforces (a shared table, guarded by tests so they cannot drift); output is byte-identical across runs; an unknown flag or a positional argument produces a clear error and a non-zero exit; and `archspec spec --help` documents the command and carries the same reference material.

## Schema output

| # | Given | When | Then |
|---|---|---|---|
| 1 | a terminal | run `archspec spec --schema` | exit 0; stdout is valid JSON; stderr empty |
| 2 | — | parse the S1 stdout | object with `$schema` = `http://json-schema.org/draft-07/schema#`, a `title`, `type: "object"`, `properties` containing `project`, `module`, `constraint`, `stereotype` |
| 3 | — | inspect schema | `project.language` is required and limited to enum `["rust","csharp","go"]` |
| 4 | — | inspect schema | `module`: `name` required; `matches` accepts `units` and `modules` (arrays of string); `contract` accepts the enforced `forbid` and rejects presence of the reserved `expose` (`not.required = ["expose"]`, empty list included); `allowed` accepts `depend_on`/`forbidden`; `submodules` recurses |
| 5 | — | inspect schema | `constraint.type` enum covers all 7 types; per-type `required` arrays match the shared table; `severity` optional enum `["error","warning"]`; `modules` optional array of string |
| 6 | a terminal | run `archspec spec --schema` twice, from fresh directories | both exit 0; byte-identical stdout |

## Reference output

| # | Given | When | Then |
|---|---|---|---|
| 7 | a terminal | run `archspec spec` | exit 0; prints annotated example TOML (all sections: `[project]`, `[[module]]`, `[[constraint]]`, `[[stereotype]]`), not JSON; stderr empty |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 8 | a terminal | run `archspec spec --bogus` | exit 1; stderr names the unknown flag |

## Help

| # | Given | When | Then |
|---|---|---|---|
| 9 | a terminal | run `archspec spec --help` | exit 0; output shows the usage line, documents `--schema`, and carries the same reference material as an unpaginated `archspec spec`; stderr empty |

## Consistency

| # | Given | When | Then |
|---|---|---|---|
| 10 | a terminal | run `archspec spec --schema` | the emitted per-type required arrays do not diverge from what the runtime validator enforces — guarded by a unit test that builds the schema from the shared table and asserts the runtime validator accepts/rejects identically |