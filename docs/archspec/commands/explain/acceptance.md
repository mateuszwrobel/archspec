# `explain` — Acceptance

Canonical rows for the `explain` verb. Test names are exact; the feature-matrix cell `explain/*` and the capability row `[verbs.explain]` point here.

## Exit-code classes

| Row | Command | Expectation | Proven by |
|---|---|---|---|
| missing spec is operational | `archspec explain a b` in a source tree without a spec | exit 70, stderr byte-identical to verify's missing-spec message, stdout empty | `tests/explain.rs::explain_missing_spec_answers_operational_class` |
| no pair is usage | `archspec explain` | exit 64, the usage line names the pair arguments | `tests/explain.rs::explain_without_pair_arguments_answers_usage` |
| unknown flag is usage | `archspec explain a b --verdict` | exit 64 | `tests/explain.rs::explain_unknown_flag_answers_usage` |
| a finding never moves the code | `archspec explain a b` on a tree whose cycle verify adjudicates red | exit 0 while verify exits 1 | `tests/explain.rs::explain_renders_the_cycle_verify_adjudicates_without_moving_the_exit_code` |

## Parity with the verify path (bound B1)

| Row | Expectation | Proven by |
|---|---|---|
| the cycle verify lists is the cycle explain renders | the findings section prints `cycle detected in: <body>` with the body verify printed, and the trace states `pair adjudicated: yes` | `tests/explain.rs::explain_renders_the_cycle_verify_adjudicates_without_moving_the_exit_code`; per-driver: `explain_names_the_cycle_findings_verify_reports` |
| a clean pair holds no findings | exit 0, model row present, findings section names nothing, the renderer note closes the trace | `explain_traces_a_clean_pair_with_no_findings` |

## Renderer facts

| Row | Expectation | Proven by |
|---|---|---|
| unresolved endpoint | stated as `unresolved: not a declared module name…`, module-graph row absent, exit 0 | `tests/explain.rs::explain_states_an_unresolved_endpoint` |
| filtered empty census | `none — the spec declares no constraint row under this filter`, unmatched rows hidden, exit 0 | `tests/explain.rs::explain_constraint_filter_states_an_empty_census` |

## Non-goals pinned

- no advice: the trace contains no suggested edit, opinion or verdict sentence; the closing note names the verdict surface instead (present in every trace test).
- renderer, not adjudicator: findings come from `Diff::error_fields()` and the warning bucket — the same table that gates the exit code (asserted in the parity rows above).
