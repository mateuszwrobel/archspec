# archspec `explain` — Design

The per-pair adjudication trace: given two endpoints, print every fact the verify path computes about the pair between them. Part of the archspec design (see `../../../archspec-design.md`).

Where `verify` answers "is this tree compliant" and `verify --coverage` answers "which constraints run where", `explain` answers GitHub issue #26's question: "why is this edge invisible to this guard?" It states the endpoint resolution, the claim chain, the model rows, the territory membership and the run's findings for that one pair — and nothing else.

## Design decision: renderer, not adjudicator

`explain` is a renderer over the real verify path (the binding bound of the issue thread):

- **Same calls, in order.** The verb runs the same spec load, driver selection, extraction and `compare` calls `verify` runs. Every printed fact is a value that path computes; nothing is re-derived, so explain and verify cannot disagree.
- **Findings from the exit-gate table.** The findings section reads `Diff::error_fields()` — the same field table that gates `verify`'s exit code and that `help diagnostics` catalogs — plus the warning bucket verbatim. No hand-listed channel array can drift here.
- **No verdict, no advice.** A finding never moves explain's exit code; verdicts stay in `verify`. explain states which constraint rows adjudicate the pair and never why the answer is good, what to edit, or how to fix anything.

## What explain adds

The per-pair trace is the only new content. The per-constraint census stays in `verify --coverage`; finding categories mean what `help diagnostics` says. explain composes those surfaces' facts along one axis — a single source→target pair — that neither surface expresses.

| Section | Fact | Source |
|---|---|---|
| endpoints | declared module or model file, with its claim chain or match patterns | spec + model |
| model rows | module-graph row present/absent, value tier, file-tier rows | the graph `compare` walks |
| constraints | per-row territory and whether the row adjudicates the pair | `[[constraint]]` rows |
| findings | this run's diff lines naming both endpoints | `Diff::error_fields()` + warnings |

## Exit codes

explain carries the usage and operational classes only: 64 for a usage mistake, 70 for a missing precondition (missing spec, missing path, missing toolchain). Exit 0 means "the trace was produced", never "the pair is clean". For verdicts run `archspec verify`.
