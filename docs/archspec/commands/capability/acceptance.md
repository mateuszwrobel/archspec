# `capability` — Acceptance Tests

CLI-level, behavioral. Each row runs the real command against a fixture and
asserts on observable output. The table itself is the contract (see
[index.md](index.md)); these rows pin its machine projection.

## Fact rows (`capability matrix`)

| # | Given | When | Then |
|---|---|---|---|
| 1 | any build | `capability matrix` | exit 0; one `capability <language> <fact> <emission>` row per (language, fact) for the six stated facts and three languages, then `rule <rule> <fact>` rows; parsing the rows equals reading the table (guards read them by value) |
| 2 | same run | the fact-row values | keep their current strings verbatim per (language, fact) — the emission constants are matched by value by `verify` and the notes; never reworded |

## Machine query (`capability granular`)

| # | Given | When | Then |
|---|---|---|---|
| 8 | any build | `capability granular <language> <fact>` | exit 0 exactly when the FACT row states `granular`; the matrix rows are output, not input to the query |
